// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Sync with nearby devices, hosted by the engine: starts the LibreSync node, hands every
//! own op to it, applies what peers send, keeps the bootstrap snapshot fresh, and runs the
//! schedule (five seconds after local changes, every five minutes, on demand). The UI only
//! supplies a secret store (keyring, Keychain) and a delegate that hears about changes.
use crate::engine::Engine;
use crate::types::*;
use sp_model::{now_ms, AppData};
use sp_p2p::{DeviceInfo, Event, P2p, SyncResult};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, Weak};
use std::time::{Duration, Instant};

/// Where the app key and device certificate live: the keyring on Linux, the Keychain on
/// macOS. Values are base64 so any store can hold them as text.
#[cfg_attr(feature = "ffi", uniffi::export(with_foreign))]
pub trait SecretStore: Send + Sync {
    fn get(&self, name: String) -> Option<String>;
    fn set(&self, name: String, value: String) -> bool;
    fn delete(&self, name: String);
}

/// What the UI hears from the nearby-sync runtime. Calls come from a background thread.
#[cfg_attr(feature = "ffi", uniffi::export(with_foreign))]
pub trait P2pDelegate: Send + Sync {
    fn on_event(&self, event: P2pEvent);
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum P2pEvent {
    /// The node listens; the port is what a peer on an overlay network would need.
    Started { port: u32 },
    /// Peers changed the store: rebuild the views.
    StoreChanged,
    /// Only the status caption changed (a sync ran, nothing new).
    StatusChanged,
    /// A full discovery/exchange cycle began, including automatic cycles.
    SyncStarted,
    /// The cycle ended. No reachable peer is not a successful sync.
    SyncCompleted { error: Option<String> },
    /// The linked device list changed.
    DevicesChanged,
    /// A discovery pass finished; `discovered()` has the result.
    DiscoveryUpdated,
    /// Something to tell the user.
    Notice { message: Message },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct NearbyDevice {
    pub device_id: String,
    pub name: String,
    pub address: Option<String>,
    pub last_seen_ms: Option<u64>,
    pub linked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct P2pInfo {
    pub device_name: String,
    pub device_id: String,
    pub port: u32,
}

struct Base64Keys(Arc<dyn SecretStore>);
impl sp_p2p::KeyStore for Base64Keys {
    fn get(&self, name: &str) -> sp_p2p::Result<Option<Vec<u8>>> {
        use base64_impl::decode;
        Ok(self.0.get(name.to_string()).and_then(|s| decode(&s)))
    }
    fn set(&self, name: &str, value: &[u8]) -> sp_p2p::Result<()> {
        if self.0.set(name.to_string(), base64_impl::encode(value)) {
            Ok(())
        } else {
            Err(sp_p2p::Error::Protocol("secret store refused the key".into()))
        }
    }
    fn delete(&self, name: &str) -> sp_p2p::Result<()> {
        self.0.delete(name.to_string());
        Ok(())
    }
}

/// A dependency-free base64 (standard alphabet, padded), enough for key material.
mod base64_impl {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    pub fn encode(bytes: &[u8]) -> String {
        let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
        for chunk in bytes.chunks(3) {
            let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
            let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
            out.push(ALPHABET[(n >> 18) as usize & 63] as char);
            out.push(ALPHABET[(n >> 12) as usize & 63] as char);
            out.push(if chunk.len() > 1 {
                ALPHABET[(n >> 6) as usize & 63] as char
            } else {
                '='
            });
            out.push(if chunk.len() > 2 {
                ALPHABET[n as usize & 63] as char
            } else {
                '='
            });
        }
        out
    }
    pub fn decode(s: &str) -> Option<Vec<u8>> {
        let val = |c: u8| ALPHABET.iter().position(|&a| a == c).map(|p| p as u32);
        let clean: Vec<u8> = s.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
        let mut out = Vec::with_capacity(clean.len() / 4 * 3);
        for chunk in clean.chunks(4) {
            let mut n = 0u32;
            let mut pad = 0;
            for (i, &c) in chunk.iter().enumerate() {
                n <<= 6;
                if c == b'=' {
                    pad += 1;
                } else {
                    n |= val(c)?;
                }
                if i == 3 && pad > 2 {
                    return None;
                }
            }
            n <<= 6 * (4 - chunk.len());
            let pad = pad + (4 - chunk.len());
            out.push((n >> 16) as u8);
            if pad < 2 {
                out.push((n >> 8) as u8);
            }
            if pad < 1 {
                out.push(n as u8);
            }
        }
        Some(out)
    }
}

#[derive(Default)]
struct Deadlines {
    sync: Option<Instant>,
    snapshot: Option<Instant>,
    periodic: Option<Instant>,
    /// While the devices dialog is open: discover every four seconds.
    discover_every: Option<Duration>,
    discover: Option<Instant>,
    publish: bool,
    stop: bool,
}

struct Scheduler {
    state: Mutex<Deadlines>,
    wake: Condvar,
}

impl Scheduler {
    fn update(&self, f: impl FnOnce(&mut Deadlines)) {
        let mut d = self.state.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut d);
        self.wake.notify_all();
    }
}

fn hash_bytes(b: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    b.hash(&mut h);
    h.finish()
}

/// The running node plus its threads. Owned by the engine while nearby sync is on.
pub struct Runtime {
    p2p: P2p,
    engine: Weak<Engine>,
    delegate: Arc<dyn P2pDelegate>,
    discovered: Mutex<Vec<DeviceInfo>>,
    sched: Scheduler,
    snapshot_hash: Mutex<u64>,
    stopped: AtomicBool,
    cycle: Mutex<Option<(u64, bool, Option<String>)>>,
}

impl Runtime {
    pub(crate) fn linked_count(&self) -> u32 {
        self.p2p.devices().len() as u32
    }
    fn engine(&self) -> Option<Arc<Engine>> {
        self.engine.upgrade()
    }
    fn emit(&self, e: P2pEvent) {
        self.delegate.on_event(e);
    }
    fn schedule_sync(&self, secs: u64) {
        self.sched
            .update(|d| d.sync = Some(Instant::now() + Duration::from_secs(secs)));
    }
    fn schedule_snapshot(&self) {
        self.sched.update(|d| {
            if d.snapshot.is_none() {
                d.snapshot = Some(Instant::now() + Duration::from_secs(60));
            }
        });
    }
    /// Hands every own pending op to the transport.
    fn publish_pending(&self) {
        let Some(engine) = self.engine() else { return };
        let mut any = false;
        {
            let g = engine.lock();
            for pd in g.store.pending.iter().filter(|pd| pd.op.c == g.store.meta.client_id) {
                if !self.p2p.has_seen(&pd.op.id) {
                    any |= self.p2p.publish(&pd.op, &pd.action);
                }
            }
        }
        if any {
            self.schedule_sync(5);
        }
        self.schedule_snapshot();
    }
    /// Republishes the bootstrap snapshot when the state changed.
    fn publish_snapshot(&self) {
        let Some(engine) = self.engine() else { return };
        let json = match serde_json::to_vec(&engine.lock().store.state) {
            Ok(j) => j,
            Err(_) => return,
        };
        let h = hash_bytes(&json);
        {
            let mut last = self.snapshot_hash.lock().unwrap_or_else(|e| e.into_inner());
            if *last == h {
                return;
            }
            *last = h;
        }
        self.p2p.publish_snapshot(&json);
        self.schedule_sync(5);
    }
    fn sync_now(&self) {
        let mut cycle = self.cycle.lock().unwrap_or_else(|e| e.into_inner());
        if cycle.is_some() {
            return;
        }
        self.emit(P2pEvent::SyncStarted);
        match self.p2p.sync_all() {
            Ok(ticket) => *cycle = Some((ticket.0, false, None)),
            Err(e) => self.emit(P2pEvent::SyncCompleted {
                error: Some(e.to_string()),
            }),
        }
    }
    /// Applies received ops (and a bootstrap snapshot on first contact) to the store.
    fn apply_inbox(&self) {
        let Some(engine) = self.engine() else { return };
        let (ops, snap) = self.p2p.take_inbox();
        let changed = apply_received(&engine, ops, snap);
        self.emit(if changed {
            P2pEvent::StoreChanged
        } else {
            P2pEvent::StatusChanged
        });
    }
    fn handle(&self, e: Event) {
        log::debug!("p2p: event {e:?}");
        match e {
            Event::ListenerStarted { address } => {
                log::info!("p2p: listening on {address}");
                self.emit(P2pEvent::Started {
                    port: address.port() as u32,
                });
            }
            Event::LinkFinished { device, error, .. } => {
                self.p2p.link_finished(device.as_ref());
                match (device, error) {
                    (Some(d), _) => {
                        self.emit(P2pEvent::Notice {
                            message: Message::LinkedWith {
                                name: d.identity.user_id,
                            },
                        });
                        self.schedule_sync(1);
                    }
                    (None, Some(e)) => {
                        log::warn!("p2p: link failed: {e}");
                        self.emit(P2pEvent::Notice {
                            message: Message::LinkFailed,
                        });
                    }
                    _ => self.emit(P2pEvent::Notice {
                        message: Message::LinkDeclined,
                    }),
                }
                self.emit(P2pEvent::DevicesChanged);
            }
            Event::LinkingRequested { request } => {
                // The handler approved it while our pairing window was open.
                if self
                    .p2p
                    .devices()
                    .iter()
                    .any(|d| d.device_id == request.device.identity.device_id)
                {
                    self.emit(P2pEvent::Notice {
                        message: Message::LinkedWith {
                            name: request.device.identity.user_id,
                        },
                    });
                    self.emit(P2pEvent::DevicesChanged);
                    self.schedule_sync(1);
                }
            }
            Event::DiscoveryFinished { devices } => {
                *self.discovered.lock().unwrap_or_else(|e| e.into_inner()) = devices;
                self.emit(P2pEvent::DiscoveryUpdated);
            }
            Event::SyncFinished { device, result, .. } => match result {
                SyncResult::Success => {
                    log::debug!("p2p: synced with {}", device.identity.device_id);
                    self.apply_inbox();
                    if let Some((_, succeeded, _)) = self.cycle.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
                        *succeeded = true;
                    }
                }
                SyncResult::Failed(msg) => {
                    if let Some((_, _, error)) = self.cycle.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
                        *error = Some(msg);
                    }
                }
            },
            Event::InboundSync { device, applied } => {
                log::debug!("p2p: {} pushed {applied} entries", device.identity.device_id);
                self.apply_inbox();
            }
            Event::TaskFinished { ticket, result } => {
                self.p2p.save_state().ok();
                let mut cycle = self.cycle.lock().unwrap_or_else(|e| e.into_inner());
                if cycle.as_ref().is_some_and(|c| c.0 == ticket) {
                    let (_, succeeded, mut error) = cycle.take().unwrap();
                    if let SyncResult::Failed(message) = result {
                        error = Some(message);
                    }
                    if !succeeded && error.is_none() {
                        error = Some(
                            if self.linked_count() == 0 {
                                "No linked devices. Link a device to start syncing."
                            } else {
                                "No devices reached. Open Momentum on your other device and check the connection."
                            }
                            .into(),
                        );
                    }
                    self.emit(P2pEvent::SyncCompleted { error });
                }
                self.emit(P2pEvent::StatusChanged);
            }
            Event::FingerprintChanged { device, .. } => {
                log::warn!("p2p: fingerprint changed for {}", device.identity.device_id);
                self.emit(P2pEvent::Notice {
                    message: Message::IdentityChanged {
                        name: device.identity.user_id,
                    },
                });
            }
            Event::Error { message } => log::warn!("p2p: {message}"),
            _ => {}
        }
    }
    fn drain(&self) {
        let events = self.p2p.events();
        loop {
            match events.try_recv() {
                Ok(Some(e)) => self.handle(e),
                Ok(None) => break,
                Err(err) => {
                    log::warn!("p2p: event stream: {err}");
                    break;
                }
            }
        }
    }
    /// The scheduler thread: waits for the earliest deadline, runs it, repeats.
    fn run_scheduler(self: Arc<Self>) {
        loop {
            let action: Box<dyn FnOnce(&Runtime)> = {
                let mut d = self.sched.state.lock().unwrap_or_else(|e| e.into_inner());
                loop {
                    if d.stop {
                        return;
                    }
                    if d.publish {
                        d.publish = false;
                        break Box::new(|r: &Runtime| r.publish_pending());
                    }
                    let now = Instant::now();
                    let mut next: Option<Instant> = None;
                    for t in [d.sync, d.snapshot, d.periodic, d.discover].into_iter().flatten() {
                        next = Some(next.map_or(t, |n: Instant| n.min(t)));
                    }
                    if d.sync.is_some_and(|t| t <= now) {
                        d.sync = None;
                        break Box::new(|r: &Runtime| r.sync_now());
                    }
                    if d.snapshot.is_some_and(|t| t <= now) {
                        d.snapshot = None;
                        break Box::new(|r: &Runtime| r.publish_snapshot());
                    }
                    if d.periodic.is_some_and(|t| t <= now) {
                        d.periodic = Some(now + Duration::from_secs(300));
                        break Box::new(|r: &Runtime| r.sync_now());
                    }
                    if d.discover.is_some_and(|t| t <= now) {
                        d.discover = d.discover_every.map(|every| now + every);
                        break Box::new(|r: &Runtime| {
                            r.p2p.discover().ok();
                        });
                    }
                    d = match next {
                        Some(t) => {
                            let (guard, _) = self
                                .sched
                                .wake
                                .wait_timeout(d, t.saturating_duration_since(now))
                                .unwrap_or_else(|e| e.into_inner());
                            guard
                        }
                        None => self.sched.wake.wait(d).unwrap_or_else(|e| e.into_inner()),
                    };
                }
            };
            action(&self);
        }
    }
}

/// Store application is separate from transport so late bootstrap and empty exchanges
/// can be checked without opening network sockets.
fn apply_received(engine: &Engine, ops: Vec<sp_p2p::OpRecord>, snap: Option<sp_p2p::Snapshot>) -> bool {
    let mut changed = false;
    let mut skip_before = 0;
    {
        let mut g = engine.lock();
        if let Some(s) = snap.filter(|_| !g.store.meta.p2p_bootstrapped) {
            if let Some(data) = s.json().and_then(|j| serde_json::from_slice::<AppData>(&j).ok()) {
                let added = g.store.adopt_snapshot(data);
                if added == usize::MAX {
                    skip_before = s.t; // whole snapshot adopted: ops it already contains are older
                    log::info!("p2p: adopted snapshot from {}", s.origin);
                } else {
                    log::info!("p2p: merged {added} entities from {}'s snapshot", s.origin);
                }
                changed = true;
                // A sync with only ops must not suppress the first real snapshot.
                g.store.meta.p2p_bootstrapped = true;
            }
        }
        let mut n = 0;
        for r in ops.into_iter().filter(|r| r.t > skip_before) {
            if let Some((op, action)) = r.decode() {
                n += g.store.apply_remote(op, action) as usize;
            }
        }
        if n > 0 {
            log::info!("p2p: applied {n} ops from peers");
            changed = true;
        }
        g.store.meta.last_nearby_ms = now_ms();
        g.store.save().ok();
        if changed {
            g.invalidate();
            g.spawn_repeats();
        }
    }
    changed
}

impl Drop for Runtime {
    fn drop(&mut self) {
        if !self.stopped.swap(true, Ordering::SeqCst) {
            self.p2p.shutdown();
        }
    }
}

#[cfg_attr(feature = "ffi", uniffi::export)]
impl Engine {
    /// Starts the nearby-sync node. `demo_key_dir` uses file-backed keys instead of the
    /// secret store (screenshots and tests). Idempotent while running.
    pub fn p2p_start(
        self: Arc<Self>,
        delegate: Arc<dyn P2pDelegate>,
        secrets: Arc<dyn SecretStore>,
        device_name: String,
        demo_key_dir: Option<String>,
    ) -> Result<P2pInfo, CoreError> {
        if let Some(r) = self.p2p.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            return Ok(P2pInfo {
                device_name: r.p2p.identity().user_id.clone(),
                device_id: r.p2p.identity().device_id.clone(),
                port: r.p2p.listen_addr().port() as u32,
            });
        }
        let (dir, id) = {
            let g = self.lock();
            (g.store.dir().join("p2p"), g.store.meta.client_id.clone())
        };
        let keys: Box<dyn sp_p2p::KeyStore> = match demo_key_dir {
            Some(d) => Box::new(
                sp_p2p::FileKeyStore::new(std::path::PathBuf::from(d))
                    .map_err(|e| CoreError::Nearby { message: e.to_string() })?,
            ),
            None => Box::new(Base64Keys(secrets)),
        };
        let p2p = P2p::start(dir, &id, &device_name, keys, sp_p2p::DEFAULT_PORT)
            .map_err(|e| CoreError::Nearby { message: e.to_string() })?;
        let info = P2pInfo {
            device_name: p2p.identity().user_id.clone(),
            device_id: p2p.identity().device_id.clone(),
            port: p2p.listen_addr().port() as u32,
        };
        log::info!(
            "p2p: started as {} ({device_name}), listening on {}",
            info.device_id,
            info.port
        );
        let rt = Arc::new(Runtime {
            p2p,
            engine: Arc::downgrade(&self),
            delegate,
            discovered: Mutex::new(vec![]),
            sched: Scheduler {
                state: Mutex::new(Deadlines {
                    periodic: Some(Instant::now() + Duration::from_secs(300)),
                    sync: Some(Instant::now() + Duration::from_secs(3)),
                    publish: true,
                    ..Default::default()
                }),
                wake: Condvar::new(),
            },
            snapshot_hash: Mutex::new(0),
            stopped: AtomicBool::new(false),
            cycle: Mutex::new(None),
        });
        // Every event wakes the pump thread, which drains the stream and handles each event.
        let pump = Arc::downgrade(&rt);
        let (tx, rx) = std::sync::mpsc::channel::<()>();
        rt.p2p.events().set_waker(move || {
            let _ = tx.send(());
        });
        std::thread::Builder::new()
            .name("momentum-p2p-events".into())
            .spawn(move || {
                while rx.recv().is_ok() {
                    let Some(rt) = pump.upgrade() else { break };
                    if rt.stopped.load(Ordering::SeqCst) {
                        break;
                    }
                    rt.drain();
                }
            })
            .ok();
        let sched = rt.clone();
        std::thread::Builder::new()
            .name("momentum-p2p-schedule".into())
            .spawn(move || sched.run_scheduler())
            .ok();
        // Local changes wake the scheduler so pending ops are handed over right away.
        let signal = Arc::downgrade(&rt);
        self.lock().change_signal = Some(Arc::new(move || {
            if let Some(rt) = signal.upgrade() {
                rt.sched.update(|d| d.publish = true);
            }
        }));
        rt.publish_snapshot();
        *self.p2p.lock().unwrap_or_else(|e| e.into_inner()) = Some(rt);
        Ok(info)
    }
    pub fn p2p_stop(&self) {
        let rt = self.p2p.lock().unwrap_or_else(|e| e.into_inner()).take();
        self.lock().change_signal = None;
        if let Some(rt) = rt {
            rt.stopped.store(true, Ordering::SeqCst);
            rt.sched.update(|d| d.stop = true);
            rt.p2p.shutdown();
            log::info!("p2p: stopped");
        }
    }
    pub fn p2p_running(&self) -> bool {
        self.p2p.lock().unwrap_or_else(|e| e.into_inner()).is_some()
    }
    pub fn p2p_info(&self) -> Option<P2pInfo> {
        let g = self.p2p.lock().unwrap_or_else(|e| e.into_inner());
        g.as_ref().map(|r| P2pInfo {
            device_name: r.p2p.identity().user_id.clone(),
            device_id: r.p2p.identity().device_id.clone(),
            port: r.p2p.listen_addr().port() as u32,
        })
    }
    /// Discover linked devices and exchange ops with each; results arrive as events.
    pub fn p2p_sync_now(&self) {
        if let Some(rt) = self.runtime() {
            rt.sync_now();
        }
    }
    /// Linked devices, newest first as stored.
    pub fn p2p_devices(&self) -> Vec<NearbyDevice> {
        let Some(rt) = self.runtime() else { return vec![] };
        rt.p2p
            .devices()
            .into_iter()
            .map(|d| NearbyDevice {
                device_id: d.device_id,
                name: d.name,
                address: d.address.map(|a| a.to_string()),
                last_seen_ms: d.last_seen,
                linked: true,
            })
            .collect()
    }
    /// Devices seen by the last discovery pass that are not linked yet.
    pub fn p2p_discovered(&self) -> Vec<NearbyDevice> {
        let Some(rt) = self.runtime() else { return vec![] };
        let linked = rt.p2p.devices();
        let discovered = rt.discovered.lock().unwrap_or_else(|e| e.into_inner()).clone();
        discovered
            .iter()
            .filter(|d| !linked.iter().any(|l| l.device_id == d.identity.device_id))
            .map(|d| NearbyDevice {
                device_id: d.identity.device_id.clone(),
                name: d.identity.user_id.clone(),
                address: d.address.map(|a| a.to_string()),
                last_seen_ms: None,
                linked: false,
            })
            .collect()
    }
    /// Opens the pairing window (five minutes) and returns our six-digit code; discovery
    /// runs every four seconds until `p2p_end_pairing`.
    pub fn p2p_begin_pairing(&self) -> Option<String> {
        let rt = self.runtime()?;
        let code = rt.p2p.begin_pairing();
        rt.p2p.discover().ok();
        rt.sched.update(|d| {
            d.discover_every = Some(Duration::from_secs(4));
            d.discover = Some(Instant::now() + Duration::from_secs(4));
        });
        Some(code)
    }
    /// Devel builds: a fixed code for scripted pairing.
    pub fn p2p_begin_pairing_with(&self, code: String) {
        if let Some(rt) = self.runtime() {
            rt.p2p.begin_pairing_with(&code);
        }
    }
    pub fn p2p_end_pairing(&self) {
        if let Some(rt) = self.runtime() {
            rt.p2p.end_pairing();
            rt.sched.update(|d| {
                d.discover_every = None;
                d.discover = None;
            });
        }
    }
    /// Link with a discovered device using the code shown on it.
    pub fn p2p_link(&self, address: String, code: String) -> Result<(), CoreError> {
        let rt = self.runtime().ok_or(CoreError::Nearby {
            message: "nearby sync is off".into(),
        })?;
        let addr: SocketAddr = address.parse().map_err(|_| CoreError::Invalid {
            message: format!("not an address: {address}"),
        })?;
        rt.p2p
            .link(addr, code.trim())
            .map(|_| ())
            .map_err(|e| CoreError::Nearby { message: e.to_string() })
    }
    pub fn p2p_unlink(&self, device_id: String) {
        if let Some(rt) = self.runtime() {
            rt.p2p.unlink(&device_id);
            rt.emit(P2pEvent::DevicesChanged);
        }
    }
}

impl Engine {
    fn runtime(&self) -> Option<Arc<Runtime>> {
        self.p2p.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

#[cfg(test)]
mod tests {
    use super::base64_impl::{decode, encode};
    #[test]
    fn empty_exchange_does_not_consume_bootstrap_and_success_updates_timestamp() {
        let dir = tempfile::tempdir().unwrap();
        let engine = super::Engine::open(dir.path().join("target").to_string_lossy().into_owned());
        assert!(!super::apply_received(&engine, vec![], None));
        assert!(!engine.lock().store.meta.p2p_bootstrapped);
        assert!(engine.lock().store.meta.last_nearby_ms > 0);
        let source = super::Engine::demo(dir.path().join("source").to_string_lossy().into_owned());
        source.spawn_repeats();
        let json = serde_json::to_vec(&source.lock().store.state).unwrap();
        let snapshot = sp_p2p::Snapshot::new("peer", &json);
        let invalid = sp_p2p::Snapshot {
            gz_b64: "invalid".into(),
            ..snapshot.clone()
        };
        assert!(!super::apply_received(&engine, vec![], Some(invalid)));
        assert!(!engine.lock().store.meta.p2p_bootstrapped);
        assert!(super::apply_received(&engine, vec![], Some(snapshot.clone())));
        assert!(engine.lock().store.meta.p2p_bootstrapped);
        assert_eq!(engine.all_tasks().len(), source.all_tasks().len());
        // Later snapshots cannot resurrect deleted tasks.
        engine.lock().store.state.task = Default::default();
        assert!(!super::apply_received(&engine, vec![], Some(snapshot)));
        assert!(engine.all_tasks().is_empty());
    }

    #[test]
    fn base64_round_trips() {
        for n in 0..40 {
            let bytes: Vec<u8> = (0..n).map(|i| (i * 37 % 256) as u8).collect();
            assert_eq!(decode(&encode(&bytes)).unwrap(), bytes, "{n} bytes");
        }
        assert_eq!(encode(b"hello"), "aGVsbG8=");
        assert_eq!(decode("aGVsbG8=").unwrap(), b"hello");
        assert!(decode("###").is_none());
    }
}
