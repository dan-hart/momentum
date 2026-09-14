// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Peer-to-peer sync for Momentum on top of LibreSync.
//!
//! LibreSync is only the transport: every Momentum operation becomes one immutable
//! record (`Op/<op id>`), so plain last-writer-wins on distinct ids loses nothing, and
//! the receiving side applies the typed action through `sp_oplog::apply`, the same code
//! path the Nextcloud sync uses. A device also publishes a compressed snapshot of its
//! whole state (`Snapshot/latest`) so a freshly linked device can bootstrap, after which
//! only ops travel. Ops older than [`KEEP_DAYS`] are tombstoned by their origin, which
//! keeps the shared state bounded.
//!
//! The engine runs on its own thread ([`libresync::BackgroundEngine`]); results arrive
//! as [`Event`]s that the app drains on its main loop.
use base64::Engine as _;
use libresync::{
    AppKey, BackgroundEngine, DeviceHandler, DeviceKeys, Engine, EngineConfig, FieldValue, Identity, KeyStoreExt,
    LogicalAdapter, RecordState, RecordView, State, SyncRecord, SyncRequest,
};
pub use libresync::{DeviceInfo, Error, Event, EventStream, FileKeyStore, KeyStore, Result, SyncResult, Ticket};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sp_oplog::{Action, Op};
use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const ADAPTER_ID: &str = "ops";
/// Shared by Devel and release builds so they can link with each other.
pub const APP_ID: &str = "io.github.dan_hart.Momentum";
pub const DEFAULT_PORT: u16 = 52345;
const SCHEMA: &str = "momentum";
const OP_ENTITY: &str = "Op";
const SNAPSHOT_ENTITY: &str = "Snapshot";
const SNAPSHOT_ID: &str = "latest";
/// Ops older than this are tombstoned by their origin; a new device relies on the snapshot.
pub const KEEP_DAYS: u64 = 30;
const PAIRING_WINDOW: Duration = Duration::from_secs(300);

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(tmp, path)
}
fn io(e: std::io::Error) -> Error {
    Error::Io(e)
}

/// One operation as it travels between devices.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpRecord {
    pub id: String,
    /// Wall-clock time of the op in ms; inbound ops are applied in this order.
    pub t: u64,
    pub origin: String,
    pub op: Value,
    pub action: Value,
}
impl OpRecord {
    pub fn decode(&self) -> Option<(Op, Action)> {
        Some((
            serde_json::from_value(self.op.clone()).ok()?,
            serde_json::from_value(self.action.clone()).ok()?,
        ))
    }
}

/// A gzip-compressed JSON snapshot of the whole app state, for bootstrapping.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Snapshot {
    pub t: u64,
    pub origin: String,
    pub gz_b64: String,
}
impl Snapshot {
    pub fn new(origin: &str, json: &[u8]) -> Self {
        let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(json).ok();
        let gz = enc.finish().unwrap_or_default();
        Self {
            t: now_ms(),
            origin: origin.into(),
            gz_b64: base64::engine::general_purpose::STANDARD.encode(gz),
        }
    }
    pub fn json(&self) -> Option<Vec<u8>> {
        let gz = base64::engine::general_purpose::STANDARD.decode(&self.gz_b64).ok()?;
        let mut out = Vec::new();
        flate2::read::GzDecoder::new(&gz[..]).read_to_end(&mut out).ok()?;
        Some(out)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Journal {
    /// Ops produced here, still to be published or tombstoned.
    own: Vec<OpRecord>,
    /// Ids already handed to the engine (own) or received (inbound).
    #[serde(default)]
    seen: BTreeSet<String>,
    /// Own ops already tombstoned in the shared state.
    #[serde(default)]
    tombstoned: BTreeSet<String>,
    /// Received ops not yet applied by the app.
    #[serde(default)]
    inbox: Vec<OpRecord>,
    #[serde(default)]
    snapshot_out: Option<Snapshot>,
    #[serde(default)]
    snapshot_dirty: bool,
    #[serde(default)]
    snapshot_in: Option<Snapshot>,
}
impl Journal {
    fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }
    fn save(&self, path: &Path) -> std::io::Result<()> {
        write_atomic(path, &serde_json::to_vec(self)?)
    }
}

struct OpAdapter {
    journal: Arc<Mutex<Journal>>,
    path: PathBuf,
}
fn op_record(r: &OpRecord, tombstone: bool) -> SyncRecord {
    let mut fields = std::collections::BTreeMap::new();
    if !tombstone {
        fields.insert("t".into(), FieldValue::I64(r.t as i64));
        fields.insert("origin".into(), FieldValue::String(r.origin.clone()));
        fields.insert("op".into(), FieldValue::String(r.op.to_string()));
        fields.insert("action".into(), FieldValue::String(r.action.to_string()));
    }
    SyncRecord {
        schema: SCHEMA.into(),
        entity: OP_ENTITY.into(),
        id: r.id.clone(),
        fields,
        tombstone,
        updated_at: Some(r.t),
        ..SyncRecord::default()
    }
}
fn field_str<'a>(r: &'a SyncRecord, k: &str) -> Option<&'a str> {
    match r.fields.get(k) {
        Some(FieldValue::String(s)) => Some(s),
        _ => None,
    }
}
fn field_i64(r: &SyncRecord, k: &str) -> Option<i64> {
    match r.fields.get(k) {
        Some(FieldValue::I64(v)) => Some(*v),
        _ => None,
    }
}
impl LogicalAdapter for OpAdapter {
    fn id(&self) -> &str {
        ADAPTER_ID
    }
    fn namespace(&self) -> &str {
        APP_ID
    }
    /// Publishes new own ops, tombstones old ones, and refreshes the snapshot record.
    fn load_records(&self, records: &mut RecordState) -> Result<()> {
        let mut j = self
            .journal
            .lock()
            .map_err(|_| Error::Protocol("journal poisoned".into()))?;
        let cutoff = now_ms().saturating_sub(KEEP_DAYS * 86_400_000);
        let mut keep = Vec::new();
        for r in std::mem::take(&mut j.own) {
            if r.t < cutoff {
                records.set(op_record(&r, true))?;
                j.tombstoned.insert(r.id.clone());
            } else {
                if records.get(SCHEMA, OP_ENTITY, &r.id)?.is_none() {
                    records.set(op_record(&r, false))?;
                }
                keep.push(r);
            }
        }
        j.own = keep;
        if j.snapshot_dirty {
            if let Some(s) = &j.snapshot_out {
                let fields = std::collections::BTreeMap::from([
                    ("t".to_string(), FieldValue::I64(s.t as i64)),
                    ("origin".to_string(), FieldValue::String(s.origin.clone())),
                    ("gz_b64".to_string(), FieldValue::String(s.gz_b64.clone())),
                ]);
                records.set(SyncRecord {
                    schema: SCHEMA.into(),
                    entity: SNAPSHOT_ENTITY.into(),
                    id: SNAPSHOT_ID.into(),
                    fields,
                    updated_at: Some(s.t),
                    ..SyncRecord::default()
                })?;
            }
            j.snapshot_dirty = false;
        }
        j.save(&self.path).map_err(io)
    }
    /// Collects ops and snapshots from other devices into the inbox.
    fn apply_records(&self, view: &RecordView) -> Result<()> {
        let mut j = self
            .journal
            .lock()
            .map_err(|_| Error::Protocol("journal poisoned".into()))?;
        let mut changed = false;
        for r in view.snapshot()? {
            if r.tombstone || r.schema != SCHEMA {
                continue;
            }
            if r.entity == OP_ENTITY {
                if j.seen.contains(&r.id) {
                    continue;
                }
                let (Some(op), Some(action), Some(origin)) =
                    (field_str(&r, "op"), field_str(&r, "action"), field_str(&r, "origin"))
                else {
                    continue;
                };
                let (Ok(op), Ok(action)) = (serde_json::from_str(op), serde_json::from_str(action)) else {
                    continue;
                };
                j.inbox.push(OpRecord {
                    id: r.id.clone(),
                    t: field_i64(&r, "t").unwrap_or(0) as u64,
                    origin: origin.to_string(),
                    op,
                    action,
                });
                j.seen.insert(r.id);
                changed = true;
            } else if r.entity == SNAPSHOT_ENTITY {
                let t = field_i64(&r, "t").unwrap_or(0) as u64;
                let newer = j.snapshot_in.as_ref().is_none_or(|s| s.t < t);
                let ours = j
                    .snapshot_out
                    .as_ref()
                    .is_some_and(|s| s.t == t && field_str(&r, "origin") == Some(&s.origin));
                if newer && !ours {
                    if let (Some(origin), Some(gz)) = (field_str(&r, "origin"), field_str(&r, "gz_b64")) {
                        j.snapshot_in = Some(Snapshot {
                            t,
                            origin: origin.into(),
                            gz_b64: gz.into(),
                        });
                        changed = true;
                    }
                }
            }
        }
        if changed {
            j.save(&self.path).map_err(io)?;
        }
        Ok(())
    }
}

/// A device this one has linked with.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LinkedDevice {
    pub device_id: String,
    /// The peer's display name (its `Identity::user_id`, the host name).
    pub name: String,
    pub fingerprint: String,
    pub address: Option<SocketAddr>,
    pub last_seen: Option<u64>,
}
#[derive(Debug, Default, Serialize, Deserialize)]
struct Devices {
    linked: Vec<LinkedDevice>,
}

/// Trust and key material for the engine. Linking is trust-on-first-use guarded by a
/// six-digit pairing code that the user reads off one device and types on the other.
pub struct Handler {
    identity: Identity,
    keys: Box<dyn KeyStore>,
    devices: Mutex<Devices>,
    devices_path: PathBuf,
    /// Our code and its expiry while "Link a device" is open.
    pairing: Mutex<Option<(String, Instant)>>,
    /// The code typed for an outgoing link request.
    entered: Mutex<Option<String>>,
    app_key: Mutex<Option<AppKey>>,
    device_keys: Mutex<Option<DeviceKeys>>,
}
impl Handler {
    fn new(identity: Identity, keys: Box<dyn KeyStore>, dir: &Path) -> Self {
        let devices_path = dir.join("devices.json");
        let devices = std::fs::read(&devices_path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Self {
            identity,
            keys,
            devices: Mutex::new(devices),
            devices_path,
            pairing: Mutex::new(None),
            entered: Mutex::new(None),
            app_key: Mutex::new(None),
            device_keys: Mutex::new(None),
        }
    }
    fn save_devices(&self, d: &Devices) {
        if let Ok(b) = serde_json::to_vec(d) {
            write_atomic(&self.devices_path, &b).ok();
        }
    }
    fn pairing_active(&self) -> bool {
        self.pairing
            .lock()
            .ok()
            .and_then(|p| p.as_ref().map(|(_, until)| Instant::now() < *until))
            .unwrap_or(false)
    }
    /// Records (or updates) a linked device.
    fn remember(&self, identity: &Identity, fingerprint: &str, address: Option<SocketAddr>) {
        let Ok(mut d) = self.devices.lock() else { return };
        let now = Some(now_ms());
        match d.linked.iter_mut().find(|x| x.device_id == identity.device_id) {
            Some(x) => {
                x.name = identity.user_id.clone();
                x.fingerprint = fingerprint.into();
                x.address = address.or(x.address);
                x.last_seen = now;
            }
            None => d.linked.push(LinkedDevice {
                device_id: identity.device_id.clone(),
                name: identity.user_id.clone(),
                fingerprint: fingerprint.into(),
                address,
                last_seen: now,
            }),
        }
        self.save_devices(&d);
    }
    fn touch(&self, device_id: &str, address: Option<SocketAddr>) {
        let Ok(mut d) = self.devices.lock() else { return };
        if let Some(x) = d.linked.iter_mut().find(|x| x.device_id == device_id) {
            x.address = address.or(x.address);
            x.last_seen = Some(now_ms());
            self.save_devices(&d);
        }
    }
    fn linked(&self) -> Vec<LinkedDevice> {
        self.devices.lock().map(|d| d.linked.clone()).unwrap_or_default()
    }
}
impl DeviceHandler for Handler {
    fn app_id(&self) -> &str {
        &self.identity.app_id
    }
    fn is_linked(&self, identity: &Identity) -> bool {
        self.linked().iter().any(|d| d.device_id == identity.device_id)
    }
    fn is_linked_with_fingerprint(&self, identity: &Identity, fingerprint: &str) -> bool {
        self.linked()
            .iter()
            .any(|d| d.device_id == identity.device_id && d.fingerprint == fingerprint)
    }
    fn approve_link(&self, identity: &Identity) -> Result<bool> {
        self.approve_link_with_fingerprint(identity, "")
    }
    /// Accepts while our pairing window is open (the code was already checked by the engine).
    fn approve_link_with_fingerprint(&self, identity: &Identity, fingerprint: &str) -> Result<bool> {
        if !self.pairing_active() {
            return Ok(false);
        }
        self.remember(identity, fingerprint, None);
        Ok(true)
    }
    fn pairing_secret(&self) -> Option<String> {
        if let Some(code) = self.entered.lock().ok().and_then(|e| e.clone()) {
            return Some(code);
        }
        self.pairing.lock().ok().and_then(|p| {
            p.as_ref()
                .filter(|(_, until)| Instant::now() < *until)
                .map(|(c, _)| c.clone())
        })
    }
    fn app_key(&self) -> Result<AppKey> {
        if let Some(k) = self.app_key.lock().ok().and_then(|k| k.clone()) {
            return Ok(k);
        }
        let k = self.keys.load_or_create_app_key()?;
        *self
            .app_key
            .lock()
            .map_err(|_| Error::Protocol("key cache poisoned".into()))? = Some(k.clone());
        Ok(k)
    }
    fn set_app_key(&self, app_key: &AppKey) -> Result<()> {
        self.keys.set_app_key(app_key)?;
        *self
            .app_key
            .lock()
            .map_err(|_| Error::Protocol("key cache poisoned".into()))? = Some(app_key.clone());
        Ok(())
    }
    fn device_keys(&self) -> Result<DeviceKeys> {
        if let Some(k) = self.device_keys.lock().ok().and_then(|k| k.clone()) {
            return Ok(k);
        }
        let k = self.keys.load_or_create_device_keys(&self.identity)?;
        *self
            .device_keys
            .lock()
            .map_err(|_| Error::Protocol("key cache poisoned".into()))? = Some(k.clone());
        Ok(k)
    }
    fn set_device_keys(&self, device_keys: &DeviceKeys) -> Result<()> {
        self.keys.set_device_keys(device_keys)?;
        *self
            .device_keys
            .lock()
            .map_err(|_| Error::Protocol("key cache poisoned".into()))? = Some(device_keys.clone());
        Ok(())
    }
}

/// A running peer-to-peer node: listener, journal, and linked devices.
pub struct P2p {
    engine: BackgroundEngine,
    journal: Arc<Mutex<Journal>>,
    journal_path: PathBuf,
    handler: Arc<Handler>,
    state_path: PathBuf,
    identity: Identity,
    listen_addr: SocketAddr,
    _advertiser: libresync::MdnsAdvertiser,
}
impl P2p {
    /// Starts listening. `dir` holds the journal, device list and encrypted engine state;
    /// `device_id` must be stable (the store's client id); `device_name` is shown to peers.
    pub fn start(dir: PathBuf, device_id: &str, device_name: &str, keys: Box<dyn KeyStore>, port: u16) -> Result<P2p> {
        std::fs::create_dir_all(&dir).map_err(io)?;
        let identity = Identity::new(device_id, APP_ID, device_name);
        let handler = Arc::new(Handler::new(identity.clone(), keys, &dir));
        let app_key = handler.app_key()?;
        let state_path = dir.join("state.bin");
        let journal_path = dir.join("journal.json");
        let journal = Arc::new(Mutex::new(Journal::load(&journal_path)));
        // Prefer the documented port (firewall rules); if it is taken (a Devel build next to
        // the release, or a socket still closing) fall back to any free port. The listener is
        // started synchronously so the real port is known before advertising it.
        let build = |addr: SocketAddr| -> Result<Engine> {
            let mut engine = Engine::new(
                EngineConfig::new(identity.clone()).with_listen_addr(addr),
                State::load_maybe_encrypted(&app_key, &state_path)
                    .or_else(|_| Ok::<State, libresync::Error>(State::new(device_id)))?,
                handler.clone(),
            );
            engine.register_logical_adapter(Arc::new(OpAdapter {
                journal: journal.clone(),
                path: journal_path.clone(),
            }))?;
            Ok(engine)
        };
        let mut engine = build(([0, 0, 0, 0], port).into())?;
        let listen_addr = match engine.start_listening() {
            Ok(addr) => addr,
            Err(e) if port != 0 => {
                log::info!("libresync: port {port} unavailable ({e}); using a free port");
                engine = build(([0, 0, 0, 0], 0).into())?;
                engine.start_listening()?
            }
            Err(e) => return Err(e),
        };
        let engine = engine.spawn()?;
        // The engine listens but does not announce itself; without this nobody finds us.
        let advertiser = libresync::register_mdns(&identity, listen_addr)?;
        log::info!("libresync: advertising {} on {listen_addr}", identity.device_id);
        Ok(Self {
            engine,
            journal,
            journal_path,
            handler,
            state_path,
            identity,
            listen_addr,
            _advertiser: advertiser,
        })
    }
    pub fn identity(&self) -> &Identity {
        &self.identity
    }
    pub fn events(&self) -> EventStream {
        self.engine.events()
    }
    /// The bound address once the listener is up (`ListenerStarted` carries the real port).
    pub fn listen_addr(&self) -> SocketAddr {
        self.engine.listener_addr().unwrap_or(self.listen_addr)
    }
    fn journal(&self) -> std::sync::MutexGuard<'_, Journal> {
        self.journal.lock().unwrap_or_else(|e| e.into_inner())
    }
    /// Queues a local op for the next sync. Ops already seen (own or inbound) are skipped.
    pub fn publish(&self, op: &Op, action: &Action) -> bool {
        let mut j = self.journal();
        if j.seen.contains(&op.id) {
            return false;
        }
        let (Ok(o), Ok(a)) = (serde_json::to_value(op), serde_json::to_value(action)) else {
            return false;
        };
        j.seen.insert(op.id.clone());
        j.own.push(OpRecord {
            id: op.id.clone(),
            t: op.t,
            origin: op.c.clone(),
            op: o,
            action: a,
        });
        j.save(&self.journal_path).ok();
        true
    }
    pub fn has_seen(&self, op_id: &str) -> bool {
        self.journal().seen.contains(op_id)
    }
    /// Publishes a snapshot of the whole state for devices that join later.
    pub fn publish_snapshot(&self, state_json: &[u8]) {
        let mut j = self.journal();
        j.snapshot_out = Some(Snapshot::new(&self.identity.device_id, state_json));
        j.snapshot_dirty = true;
        j.save(&self.journal_path).ok();
    }
    /// Received ops in time order, plus the newest snapshot from a peer (returned once).
    pub fn take_inbox(&self) -> (Vec<OpRecord>, Option<Snapshot>) {
        let mut j = self.journal();
        let mut ops = std::mem::take(&mut j.inbox);
        ops.sort_by_key(|r| r.t);
        let snap = j.snapshot_in.take();
        if !ops.is_empty() || snap.is_some() {
            j.save(&self.journal_path).ok();
        }
        (ops, snap)
    }
    /// Discovers linked devices on the network and syncs with each (also trying the last
    /// known address of linked devices that did not answer discovery).
    pub fn sync_all(&self) -> Result<Ticket> {
        let handler = self.handler.clone();
        self.engine.run(move |engine| {
            let discovered = engine
                .discover_devices_with_timeout(Duration::from_secs(2))
                .unwrap_or_default();
            let mut done = BTreeSet::new();
            for d in discovered.iter().filter(|d| d.linked) {
                let known = handler
                    .linked()
                    .into_iter()
                    .find(|x| x.device_id == d.identity.device_id);
                let mut info = d.clone();
                info.fingerprint = known.as_ref().map(|k| k.fingerprint.clone());
                if engine.sync_with_device_info(&info, ADAPTER_ID).is_ok() {
                    handler.touch(&d.identity.device_id, d.address);
                }
                done.insert(d.identity.device_id.clone());
            }
            for d in handler.linked().into_iter().filter(|d| !done.contains(&d.device_id)) {
                let Some(addr) = d.address else { continue };
                let req = SyncRequest::new(addr, ADAPTER_ID)
                    .with_expected_fingerprint(d.fingerprint.clone())
                    .with_expected_device_id(d.device_id.clone());
                if engine.sync(&req).is_ok() {
                    handler.touch(&d.device_id, Some(addr));
                }
            }
            Ok(())
        })
    }
    /// Syncs with one linked device at a known address (tests, manual addresses).
    pub fn sync_with(&self, device: &LinkedDevice) -> Result<Ticket> {
        let addr = device
            .address
            .ok_or_else(|| Error::Protocol("device has no address".into()))?;
        self.engine.try_sync(
            SyncRequest::new(addr, ADAPTER_ID)
                .with_expected_fingerprint(device.fingerprint.clone())
                .with_expected_device_id(device.device_id.clone()),
        )
    }
    pub fn discover(&self) -> Result<Ticket> {
        self.engine.try_discover(Duration::from_secs(2))
    }
    /// Opens a five-minute window in which one device may link using the returned code.
    pub fn begin_pairing(&self) -> String {
        let mut bytes = [0u8; 4];
        std::fs::File::open("/dev/urandom")
            .and_then(|mut f| f.read_exact(&mut bytes))
            .ok();
        let n = u32::from_le_bytes(bytes) % 1_000_000;
        let code = format!("{n:06}");
        if let Ok(mut p) = self.handler.pairing.lock() {
            *p = Some((code.clone(), Instant::now() + PAIRING_WINDOW));
        }
        code
    }
    /// Opens the pairing window with a given code (scripted tests).
    pub fn begin_pairing_with(&self, code: &str) {
        if let Ok(mut p) = self.handler.pairing.lock() {
            *p = Some((code.trim().into(), Instant::now() + PAIRING_WINDOW));
        }
    }
    pub fn end_pairing(&self) {
        if let Ok(mut p) = self.handler.pairing.lock() {
            *p = None;
        }
    }
    /// Requests a link with the device at `address` using the code shown on it.
    /// Finish with [`P2p::link_finished`] when `Event::LinkFinished` arrives.
    pub fn link(&self, address: SocketAddr, code: &str) -> Result<Ticket> {
        if let Ok(mut e) = self.handler.entered.lock() {
            *e = Some(code.trim().replace(' ', ""));
        }
        self.engine.try_request_link(address)
    }
    pub fn link_finished(&self, device: Option<&DeviceInfo>) {
        if let Ok(mut e) = self.handler.entered.lock() {
            *e = None;
        }
        if let Some(d) = device {
            self.handler
                .remember(&d.identity, d.fingerprint.as_deref().unwrap_or(""), d.address);
        }
    }
    pub fn devices(&self) -> Vec<LinkedDevice> {
        self.handler.linked()
    }
    pub fn unlink(&self, device_id: &str) {
        if let Ok(mut d) = self.handler.devices.lock() {
            d.linked.retain(|x| x.device_id != device_id);
            self.handler.save_devices(&d);
        }
    }
    /// Persists the engine state (call after syncs; the engine keeps it in memory).
    pub fn save_state(&self) -> Result<Ticket> {
        self.engine.try_save_state(&self.state_path)
    }
    pub fn shutdown(&self) {
        let _ = self.engine.shutdown();
    }
}
