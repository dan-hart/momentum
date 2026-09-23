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
    SyncStarted { cycle_id: u64 },
    /// The cycle ended. No reachable peer is not a successful sync.
    SyncCompleted {
        /// Present only for an attributable outbound cycle.
        cycle_id: Option<u64>,
        error: Option<String>,
    },
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
    /// Stop/restore wait for the complete store-commit/journal-acknowledgement pair.
    receive: Mutex<()>,
    cycle: Mutex<Option<(u64, bool, Option<String>)>>,
    /// Save commands also emit TaskFinished; their completion must not enqueue another save.
    state_saves: Mutex<std::collections::BTreeSet<u64>>,
    #[cfg(test)]
    before_ack: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
}

impl Runtime {
    fn save_transport_state(&self) {
        // Register the ticket while holding the same lock used by the event pump:
        // a fast worker may finish before submit returns.
        let mut saves = self.state_saves.lock().unwrap_or_else(|e| e.into_inner());
        match self.p2p.save_state() {
            Ok(ticket) => {
                saves.insert(ticket.0);
            }
            Err(error) => log::warn!("p2p: could not queue state save: {error}"),
        }
    }
    pub(crate) fn linked_count(&self) -> u32 {
        self.p2p.devices().len() as u32
    }
    fn engine(&self) -> Option<Arc<Engine>> {
        self.engine.upgrade()
    }
    fn emit(&self, e: P2pEvent) {
        if !self.stopped.load(Ordering::SeqCst) {
            self.delegate.on_event(e);
        }
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
            if self.stopped.load(Ordering::SeqCst) {
                return;
            }
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
        let g = engine.lock();
        if self.stopped.load(Ordering::SeqCst) {
            return;
        }
        let json = match serde_json::to_vec(&g.store.state) {
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
        drop(g);
        self.schedule_sync(5);
    }
    fn sync_now(&self) -> Result<u64, CoreError> {
        if self.stopped.load(Ordering::SeqCst) {
            return Err(CoreError::NotConfigured);
        }
        let mut cycle = self.cycle.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((id, _, _)) = cycle.as_ref() {
            return Ok(*id);
        }
        if self.linked_count() == 0 {
            let message = "No linked devices. Link a device to test the connection.".to_string();
            self.emit(P2pEvent::SyncCompleted {
                cycle_id: None,
                error: Some(message.clone()),
            });
            return Err(CoreError::Actionable { message });
        }
        match self.p2p.sync_all() {
            Ok(ticket) => {
                let id = ticket.0;
                *cycle = Some((id, false, None));
                self.emit(P2pEvent::SyncStarted { cycle_id: id });
                Ok(id)
            }
            Err(error) => {
                let message = error.to_string();
                self.emit(P2pEvent::SyncCompleted {
                    cycle_id: None,
                    error: Some(message.clone()),
                });
                Err(CoreError::Transient { message })
            }
        }
    }
    /// Applies received ops (and a bootstrap snapshot on first contact) to the store.
    fn apply_inbox(&self) -> Result<(), CoreError> {
        let _receive = self.receive.lock().unwrap_or_else(|e| e.into_inner());
        let Some(engine) = self.engine() else { return Ok(()) };
        let (ops, snap) = self.p2p.inbox();
        let changed = apply_received(&engine, ops.clone(), snap.clone(), &self.stopped)?;
        // A failed commit leaves the transport journal intact for the next cycle.
        #[cfg(test)]
        if let Some(pause) = self.before_ack.lock().unwrap().clone() {
            pause();
        }
        self.p2p.acknowledge_inbox(&ops, snap.as_ref())?;
        drop(_receive); // Delegates may synchronously request stop; never call them under the barrier.
        self.emit(if changed {
            P2pEvent::StoreChanged
        } else {
            P2pEvent::StatusChanged
        });
        Ok(())
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
                    let applied = self.apply_inbox();
                    if let Some((_, succeeded, error)) = self.cycle.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
                        match applied {
                            Ok(()) => *succeeded = true,
                            Err(e) => *error = Some(e.to_string()),
                        }
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
                self.save_transport_state();
                if let Err(error) = self.apply_inbox() {
                    self.emit(P2pEvent::SyncCompleted {
                        cycle_id: None,
                        error: Some(error.to_string()),
                    });
                }
            }
            Event::TaskFinished { ticket, result } => {
                let saved = self
                    .state_saves
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .remove(&ticket);
                if saved {
                    if let SyncResult::Failed(message) = result {
                        self.emit(P2pEvent::SyncCompleted {
                            cycle_id: None,
                            error: Some(message),
                        });
                    }
                    return;
                }
                self.save_transport_state();
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
                    self.emit(P2pEvent::SyncCompleted {
                        cycle_id: Some(ticket),
                        error,
                    });
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
                        break Box::new(|r: &Runtime| {
                            let _ = r.sync_now();
                        });
                    }
                    if d.snapshot.is_some_and(|t| t <= now) {
                        d.snapshot = None;
                        break Box::new(|r: &Runtime| r.publish_snapshot());
                    }
                    if d.periodic.is_some_and(|t| t <= now) {
                        d.periodic = Some(now + Duration::from_secs(300));
                        break Box::new(|r: &Runtime| {
                            let _ = r.sync_now();
                        });
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
fn apply_received(
    engine: &Engine,
    ops: Vec<sp_p2p::OpRecord>,
    snap: Option<sp_p2p::Snapshot>,
    stopped: &AtomicBool,
) -> Result<bool, CoreError> {
    engine.try_edit_checked(|g| {
        // p2p_stop sets this flag under the same owner lock. A queued callback
        // cannot commit after stop returns or after a restore starts replacing data.
        if stopped.load(Ordering::SeqCst) {
            return Err(CoreError::Transient {
                message: "Nearby sync was stopped".into(),
            });
        }
        let mut changed = false;
        let mut skip_before = 0;
        if let Some(s) = snap.filter(|_| !g.store.meta.p2p_bootstrapped) {
            if let Some(data) = s.json().and_then(|j| serde_json::from_slice::<AppData>(&j).ok()) {
                let added = g.store.stage_snapshot(data);
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
        // Upload queues may be cleared by another provider after our durable commit
        // but before transport acknowledgement. Exact receipts survive that boundary.
        let inbox_ids: std::collections::BTreeSet<_> = ops.iter().map(|r| r.id.clone()).collect();
        g.store.meta.p2p_applied_ops.retain(|id| inbox_ids.contains(id));
        let mut n = 0;
        for r in ops {
            if let Some((op, action)) = r.decode() {
                if !g.store.meta.p2p_applied_ops.contains(&r.id) {
                    if r.t > skip_before {
                        n += g.store.apply_remote(op, action) as usize;
                    }
                    // Operations covered by an adopted snapshot need receipts too:
                    // a retry no longer has that snapshot's skip-before boundary.
                    g.store.meta.p2p_applied_ops.insert(r.id);
                }
            }
        }
        if n > 0 {
            log::info!("p2p: applied {n} ops from peers");
            changed = true;
        }
        g.store.meta.last_nearby_ms = now_ms();
        g.dirty = true;
        if changed {
            g.spawn_repeats();
        }
        Ok(changed)
    })
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
        let mut runtime = self.p2p.lock().unwrap_or_else(|e| e.into_inner());
        if self.syncing.load(Ordering::SeqCst) {
            return Err(CoreError::Busy);
        }
        if let Some(r) = runtime.as_ref() {
            return Ok(P2pInfo {
                device_name: r.p2p.identity().user_id.clone(),
                device_id: r.p2p.identity().device_id.clone(),
                port: r.p2p.listen_addr().port() as u32,
            });
        }
        let (dir, id, epoch, state) = {
            let g = self.lock();
            (
                g.store.dir().join("p2p"),
                g.store.meta.client_id.clone(),
                g.store.meta.p2p_restore_epoch,
                serde_json::to_vec(&g.store.state).map_err(|e| CoreError::Invalid { message: e.to_string() })?,
            )
        };
        let keys: Box<dyn sp_p2p::KeyStore> = match demo_key_dir {
            Some(d) => Box::new(
                sp_p2p::FileKeyStore::new(std::path::PathBuf::from(d))
                    .map_err(|e| CoreError::Nearby { message: e.to_string() })?,
            ),
            None => Box::new(Base64Keys(secrets)),
        };
        let p2p = P2p::start_with_store(dir, &id, &device_name, keys, sp_p2p::DEFAULT_PORT, epoch, &state)
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
            receive: Mutex::new(()),
            cycle: Mutex::new(None),
            state_saves: Mutex::new(Default::default()),
            #[cfg(test)]
            before_ack: Mutex::new(None),
        });
        // Every event wakes the pump thread, which drains the stream and handles each event.
        let pump = Arc::downgrade(&rt);
        let (tx, rx) = std::sync::mpsc::channel::<()>();
        let initial_wake = tx.clone();
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
        *runtime = Some(rt);
        // Listener startup completed before the waker was registered. Drain those
        // queued events too, after publishing the runtime to callback consumers.
        drop(runtime);
        initial_wake.send(()).ok();
        Ok(info)
    }
    pub fn p2p_stop(&self) {
        let mut runtime = self.p2p.lock().unwrap_or_else(|e| e.into_inner());
        self.stop_p2p_locked(&mut runtime);
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
    pub fn p2p_sync_now(&self) -> Result<u64, CoreError> {
        self.runtime().ok_or(CoreError::NotConfigured)?.sync_now()
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
    pub(crate) fn stop_p2p_locked(&self, runtime: &mut Option<Arc<Runtime>>) {
        if let Some(rt) = runtime.take() {
            let _receive = rt.receive.lock().unwrap_or_else(|e| e.into_inner());
            {
                let mut g = self.lock();
                rt.stopped.store(true, Ordering::SeqCst);
                g.change_signal = None;
            }
            rt.sched.update(|d| d.stop = true);
            rt.p2p.shutdown();
            log::info!("p2p: stopped");
        }
    }
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
        assert!(!super::apply_received(&engine, vec![], None, &super::AtomicBool::new(false)).unwrap());
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
        assert!(!super::apply_received(&engine, vec![], Some(invalid), &super::AtomicBool::new(false)).unwrap());
        assert!(!engine.lock().store.meta.p2p_bootstrapped);
        assert!(
            super::apply_received(&engine, vec![], Some(snapshot.clone()), &super::AtomicBool::new(false)).unwrap()
        );
        assert!(engine.lock().store.meta.p2p_bootstrapped);
        assert_eq!(engine.all_tasks().len(), source.all_tasks().len());
        // Later snapshots cannot resurrect deleted tasks.
        engine.lock().store.state.task = Default::default();
        assert!(!super::apply_received(&engine, vec![], Some(snapshot), &super::AtomicBool::new(false)).unwrap());
        assert!(engine.all_tasks().is_empty());
    }

    struct NoSecrets;
    impl super::SecretStore for NoSecrets {
        fn get(&self, _: String) -> Option<String> {
            None
        }
        fn set(&self, _: String, _: String) -> bool {
            false
        }
        fn delete(&self, _: String) {}
    }
    struct Events(std::sync::Mutex<Vec<super::P2pEvent>>);
    impl super::P2pDelegate for Events {
        fn on_event(&self, event: super::P2pEvent) {
            self.0.lock().unwrap().push(event);
        }
    }
    fn running() -> (std::sync::Arc<super::Engine>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let engine = super::Engine::open(dir.path().join("store").to_string_lossy().into_owned());
        engine
            .clone()
            .p2p_start(
                std::sync::Arc::new(Events(Default::default())),
                std::sync::Arc::new(NoSecrets),
                "Isolated fixture".into(),
                Some(dir.path().join("keys").to_string_lossy().into_owned()),
            )
            .unwrap();
        (engine, dir)
    }

    #[test]
    fn nearby_reports_started_without_waiting_for_a_sync_command() {
        struct Started(std::sync::mpsc::Sender<u32>);
        impl super::P2pDelegate for Started {
            fn on_event(&self, event: super::P2pEvent) {
                if let super::P2pEvent::Started { port } = event {
                    self.0.send(port).ok();
                }
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let engine = super::Engine::open(dir.path().join("store").to_string_lossy().into_owned());
        let (sender, receiver) = std::sync::mpsc::channel();
        let info = engine
            .clone()
            .p2p_start(
                std::sync::Arc::new(Started(sender)),
                std::sync::Arc::new(NoSecrets),
                "Startup fixture".into(),
                Some(dir.path().join("keys").to_string_lossy().into_owned()),
            )
            .unwrap();
        let started = receiver.recv_timeout(std::time::Duration::from_secs(1));
        engine.p2p_stop();
        assert_eq!(started.unwrap(), info.port);
    }

    #[test]
    fn manual_nearby_sync_reports_typed_admission_failures() {
        let dir = tempfile::tempdir().unwrap();
        let stopped = super::Engine::open(dir.path().join("stopped").to_string_lossy().into_owned());
        assert!(matches!(stopped.p2p_sync_now(), Err(super::CoreError::NotConfigured)));

        let (running, _dir) = running();
        assert!(matches!(
            running.p2p_sync_now(),
            Err(super::CoreError::Actionable { .. })
        ));
        running.p2p_stop();
    }

    #[test]
    fn nearby_transport_saves_settle_without_an_idle_event_loop() {
        struct Notifications(std::sync::mpsc::Sender<()>);
        impl super::P2pDelegate for Notifications {
            fn on_event(&self, _: super::P2pEvent) {
                self.0.send(()).ok();
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let engine = super::Engine::open(dir.path().join("store").to_string_lossy().into_owned());
        let (sender, receiver) = std::sync::mpsc::channel();
        engine
            .clone()
            .p2p_start(
                std::sync::Arc::new(Notifications(sender)),
                std::sync::Arc::new(NoSecrets),
                "Idle fixture".into(),
                Some(dir.path().join("keys").to_string_lossy().into_owned()),
            )
            .unwrap();
        engine.runtime().unwrap().p2p.save_state().unwrap();
        let first = receiver.recv_timeout(std::time::Duration::from_secs(2));
        let mut settled = false;
        for _ in 0..32 {
            if receiver.recv_timeout(std::time::Duration::from_millis(150)).is_err() {
                settled = true;
                break;
            }
        }
        engine.p2p_stop();
        assert!(first.is_ok(), "the real event pump must have started");
        assert!(
            dir.path().join("store/p2p/state.bin").is_file(),
            "transport state must still persist"
        );
        assert!(
            settled,
            "saving transport state must not queue another save indefinitely"
        );
    }

    #[test]
    fn failed_peer_save_preserves_state_timestamp_and_bootstrap_for_retry() {
        let dir = tempfile::tempdir().unwrap();
        let engine = super::Engine::open(dir.path().join("target").to_string_lossy().into_owned());
        let before = engine.with_store(|s| serde_json::json!([s.state, s.pending, s.meta]));
        let mut state = sp_model::AppData::fresh();
        let task = sp_model::Task::new("Peer task", sp_model::INBOX_PROJECT_ID);
        state.task.insert(&task.id.clone(), task);
        let snapshot = sp_p2p::Snapshot::new("peer", &serde_json::to_vec(&state).unwrap());
        std::fs::create_dir(dir.path().join("target/pending.json.tmp")).unwrap();
        assert!(
            super::apply_received(&engine, vec![], Some(snapshot.clone()), &super::AtomicBool::new(false)).is_err()
        );
        assert_eq!(
            engine.with_store(|s| serde_json::json!([s.state, s.pending, s.meta])),
            before
        );
        std::fs::remove_dir(dir.path().join("target/pending.json.tmp")).unwrap();
        assert!(super::apply_received(&engine, vec![], Some(snapshot), &super::AtomicBool::new(false)).unwrap());
        assert_eq!(engine.all_tasks().len(), 1);
        let reopened = super::Engine::open(dir.path().join("target").to_string_lossy().into_owned());
        assert_eq!(reopened.all_tasks().len(), 1);
        assert!(reopened.sync_status().last_nearby_ms > 0);
    }

    #[test]
    fn import_stops_nearby_runtime_before_replacement() {
        let (engine, dir) = running();
        let old_runtime = engine.runtime().unwrap();
        let backup = dir.path().join("backup.json");
        std::fs::write(
            &backup,
            serde_json::to_vec(&serde_json::json!({"data": sp_model::AppData::fresh()})).unwrap(),
        )
        .unwrap();
        let result = engine.import_backup(backup.to_string_lossy().into_owned());
        let stopped = !engine.p2p_running();
        engine.p2p_stop();
        assert!(result.is_ok());
        assert!(stopped, "restore must stop the old runtime before changing its store");
        assert!(old_runtime.stopped.load(super::Ordering::SeqCst));
        assert!(super::apply_received(&engine, vec![], None, &old_runtime.stopped).is_err());
        assert_eq!(engine.sync_status().last_nearby_ms, 0);
    }

    #[test]
    fn restarting_after_restore_does_not_replay_the_old_transport_inbox() {
        let (engine, dir) = running();
        engine.p2p_stop();
        let journal_path = dir.path().join("store/p2p/journal.json");
        let mut journal: serde_json::Value = serde_json::from_slice(&std::fs::read(&journal_path).unwrap()).unwrap();
        let action = sp_oplog::Action::AddTask {
            task: sp_model::Task::new("Stale peer task", sp_model::INBOX_PROJECT_ID),
            bottom: true,
        };
        let op = action.to_op("peer", &mut Default::default());
        let record = sp_p2p::OpRecord {
            id: op.id.clone(),
            t: op.t,
            origin: "peer".into(),
            op: serde_json::to_value(&op).unwrap(),
            action: serde_json::to_value(action).unwrap(),
        };
        journal["inbox"] = serde_json::json!([record]);
        journal["seen"] = serde_json::json!([op.id]);
        std::fs::write(&journal_path, serde_json::to_vec(&journal).unwrap()).unwrap();
        let backup = dir.path().join("backup.json");
        std::fs::write(
            &backup,
            serde_json::to_vec(&serde_json::json!({"data": sp_model::AppData::fresh()})).unwrap(),
        )
        .unwrap();
        engine.import_backup(backup.to_string_lossy().into_owned()).unwrap();
        let engine = super::Engine::open(dir.path().join("store").to_string_lossy().into_owned());
        engine
            .clone()
            .p2p_start(
                std::sync::Arc::new(Events(Default::default())),
                std::sync::Arc::new(NoSecrets),
                "Fixture".into(),
                Some(dir.path().join("keys").to_string_lossy().into_owned()),
            )
            .unwrap();
        let result = engine.runtime().unwrap().apply_inbox();
        engine.p2p_stop();
        assert!(result.is_ok());
        assert!(
            engine.all_tasks().is_empty(),
            "restored data must survive a transport restart"
        );
    }

    #[test]
    fn providers_cannot_run_simultaneously() {
        let (engine, dir) = running();
        let result = engine.sync_nextcloud(crate::NextcloudSettings {
            server_url: "http://127.0.0.1:0".into(),
            user_name: "fixture".into(),
            password: "fixture".into(),
            folder: "fixture".into(),
            compress: false,
            encryption_password: None,
        });
        engine.p2p_stop();
        assert!(matches!(result, Err(crate::CoreError::Busy)));
        engine.syncing.store(true, super::Ordering::SeqCst);
        let result = engine.clone().p2p_start(
            std::sync::Arc::new(Events(Default::default())),
            std::sync::Arc::new(NoSecrets),
            "Fixture".into(),
            Some(dir.path().join("keys").to_string_lossy().into_owned()),
        );
        engine.syncing.store(false, super::Ordering::SeqCst);
        assert!(matches!(result, Err(crate::CoreError::Busy)));
        assert!(!engine.p2p_running());
    }

    #[test]
    fn concurrent_starts_share_one_runtime() {
        let (engine, dir) = running();
        engine.p2p_stop();
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
        let threads: Vec<_> = (0..2)
            .map(|_| {
                let engine = engine.clone();
                let barrier = barrier.clone();
                let keys = dir.path().join("keys").to_string_lossy().into_owned();
                std::thread::spawn(move || {
                    barrier.wait();
                    engine
                        .p2p_start(
                            std::sync::Arc::new(Events(Default::default())),
                            std::sync::Arc::new(NoSecrets),
                            "Fixture".into(),
                            Some(keys),
                        )
                        .unwrap()
                })
            })
            .collect();
        barrier.wait();
        let infos: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
        engine.p2p_stop();
        assert_eq!(infos[0], infos[1]);
    }

    #[test]
    fn corrupt_transport_journal_is_preserved_and_startup_fails() {
        let dir = tempfile::tempdir().unwrap();
        let engine = super::Engine::open(dir.path().join("store").to_string_lossy().into_owned());
        let journal = dir.path().join("store/p2p/journal.json");
        std::fs::create_dir_all(journal.parent().unwrap()).unwrap();
        std::fs::write(&journal, b"{broken").unwrap();
        let result = engine.clone().p2p_start(
            std::sync::Arc::new(Events(Default::default())),
            std::sync::Arc::new(NoSecrets),
            "Fixture".into(),
            Some(dir.path().join("keys").to_string_lossy().into_owned()),
        );
        engine.p2p_stop();
        assert!(result.is_err(), "unknown receipts must not become an empty journal");
        assert_eq!(std::fs::read(journal).unwrap(), b"{broken");
    }

    #[test]
    fn runtime_acknowledges_received_ops_only_after_the_store_commits() {
        let (engine, dir) = running();
        engine.p2p_stop();
        let path = dir.path().join("store/p2p/journal.json");
        let mut journal: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let action = sp_oplog::Action::AddTask {
            task: sp_model::Task::new("Received once", sp_model::INBOX_PROJECT_ID),
            bottom: true,
        };
        let op = action.to_op("peer", &mut Default::default());
        journal["inbox"] = serde_json::json!([sp_p2p::OpRecord {
            id: op.id.clone(),
            t: op.t,
            origin: "peer".into(),
            op: serde_json::to_value(&op).unwrap(),
            action: serde_json::to_value(action).unwrap()
        }]);
        journal["seen"] = serde_json::json!([op.id]);
        std::fs::write(&path, serde_json::to_vec(&journal).unwrap()).unwrap();
        engine
            .clone()
            .p2p_start(
                std::sync::Arc::new(Events(Default::default())),
                std::sync::Arc::new(NoSecrets),
                "Fixture".into(),
                Some(dir.path().join("keys").to_string_lossy().into_owned()),
            )
            .unwrap();
        let runtime = engine.runtime().unwrap();
        let blocked = dir.path().join("store/pending.json.tmp");
        std::fs::create_dir(&blocked).unwrap();
        let failed = runtime.apply_inbox();
        let retained = runtime.p2p.inbox().0.len();
        let unsaved = engine.all_tasks().is_empty();
        std::fs::remove_dir(blocked).unwrap();
        let saved = runtime.apply_inbox();
        let acknowledged = runtime.p2p.inbox().0.is_empty();
        engine.p2p_stop();
        assert!(failed.is_err());
        assert!(unsaved);
        assert_eq!(retained, 1);
        assert!(saved.is_ok());
        assert!(acknowledged);
        let reopened = super::Engine::open(dir.path().join("store").to_string_lossy().into_owned());
        assert_eq!(reopened.all_tasks().len(), 1);
        let journal: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(journal["inbox"], serde_json::json!([]));
    }

    #[test]
    fn restore_waits_for_the_old_runtime_journal_acknowledgement() {
        let (engine, dir) = running();
        let runtime = engine.runtime().unwrap();
        let (entered_tx, entered) = std::sync::mpsc::channel();
        let (release, released) = std::sync::mpsc::channel();
        let released = std::sync::Mutex::new(released);
        *runtime.before_ack.lock().unwrap() = Some(std::sync::Arc::new(move || {
            entered_tx.send(()).unwrap();
            released
                .lock()
                .unwrap()
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
        }));
        let apply = std::thread::spawn(move || runtime.apply_inbox());
        entered.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
        let backup = dir.path().join("backup.json");
        std::fs::write(
            &backup,
            serde_json::to_vec(&serde_json::json!({"data": sp_model::AppData::fresh()})).unwrap(),
        )
        .unwrap();
        let (done_tx, done) = std::sync::mpsc::channel();
        let importer = engine.clone();
        let import = std::thread::spawn(move || {
            let result = importer.import_backup(backup.to_string_lossy().into_owned());
            done_tx.send(()).unwrap();
            result
        });
        let waited = done.recv_timeout(std::time::Duration::from_millis(150)).is_err();
        release.send(()).unwrap();
        assert!(apply.join().unwrap().is_ok());
        assert!(import.join().unwrap().is_ok());
        assert!(
            waited,
            "restore returned while the old runtime could still overwrite its journal"
        );
        assert!(!engine.p2p_running());
    }

    #[test]
    fn receive_delegate_can_synchronously_stop_without_holding_the_ack_barrier() {
        struct StopOnStatus(std::sync::Weak<super::Engine>);
        impl super::P2pDelegate for StopOnStatus {
            fn on_event(&self, event: super::P2pEvent) {
                if matches!(event, super::P2pEvent::StatusChanged) {
                    if let Some(engine) = self.0.upgrade() {
                        engine.p2p_stop();
                    }
                }
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let engine = super::Engine::open(dir.path().join("store").to_string_lossy().into_owned());
        engine
            .clone()
            .p2p_start(
                std::sync::Arc::new(StopOnStatus(std::sync::Arc::downgrade(&engine))),
                std::sync::Arc::new(NoSecrets),
                "Fixture".into(),
                Some(dir.path().join("keys").to_string_lossy().into_owned()),
            )
            .unwrap();
        let runtime = engine.runtime().unwrap();
        assert!(runtime.apply_inbox().is_ok());
        assert!(!engine.p2p_running());
    }

    #[test]
    fn failed_acknowledgement_cannot_replay_an_old_edit_after_nextcloud_clears_pending() {
        let (engine, dir) = running();
        engine.p2p_stop();
        let path = dir.path().join("store/p2p/journal.json");
        let mut journal: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let task = sp_model::Task::new("Keep completed", sp_model::INBOX_PROJECT_ID);
        let id = task.id.clone();
        let action = sp_oplog::Action::AddTask { task, bottom: true };
        let op = action.to_op("peer", &mut Default::default());
        journal["inbox"] = serde_json::json!([sp_p2p::OpRecord {
            id: op.id.clone(),
            t: op.t,
            origin: "peer".into(),
            op: serde_json::to_value(&op).unwrap(),
            action: serde_json::to_value(action).unwrap()
        }]);
        journal["seen"] = serde_json::json!([op.id]);
        std::fs::write(&path, serde_json::to_vec(&journal).unwrap()).unwrap();
        engine
            .clone()
            .p2p_start(
                std::sync::Arc::new(Events(Default::default())),
                std::sync::Arc::new(NoSecrets),
                "Fixture".into(),
                Some(dir.path().join("keys").to_string_lossy().into_owned()),
            )
            .unwrap();
        let runtime = engine.runtime().unwrap();
        std::fs::create_dir(path.with_extension("tmp")).unwrap();
        let result = runtime.apply_inbox();
        engine.p2p_stop();
        assert!(result.is_err(), "journal acknowledgement was blocked");
        assert_eq!(engine.all_tasks().len(), 1, "the task-store commit happened first");
        std::fs::remove_dir(path.with_extension("tmp")).unwrap();
        assert!(engine.set_done(id.clone(), true).changed);
        let mut server = crate::transport_tests::PausedDav::new();
        let settings = server.settings.clone();
        let syncing = engine.clone();
        let exchange = std::thread::spawn(move || syncing.sync_nextcloud(settings));
        server.arrived.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        server.release();
        exchange.join().unwrap().unwrap();
        assert_eq!(
            engine.pending_count(),
            0,
            "a real Nextcloud cycle consumed the upload queue"
        );
        let engine = super::Engine::open(dir.path().join("store").to_string_lossy().into_owned());
        engine
            .clone()
            .p2p_start(
                std::sync::Arc::new(Events(Default::default())),
                std::sync::Arc::new(NoSecrets),
                "Fixture".into(),
                Some(dir.path().join("keys").to_string_lossy().into_owned()),
            )
            .unwrap();
        let result = engine.runtime().unwrap().apply_inbox();
        engine.p2p_stop();
        assert!(result.is_ok());
        assert!(
            engine.task_detail(id).unwrap().is_done,
            "retry must not reapply the old AddTask over a newer completion"
        );
    }

    #[test]
    fn bootstrap_covered_ops_cannot_replay_when_acknowledgement_is_retried() {
        let dir = tempfile::tempdir().unwrap();
        let engine = super::Engine::open(dir.path().to_string_lossy().into_owned());
        let task = sp_model::Task::new("Snapshot task", sp_model::INBOX_PROJECT_ID);
        let id = task.id.clone();
        let action = sp_oplog::Action::AddTask {
            task: task.clone(),
            bottom: true,
        };
        let op = action.to_op("peer", &mut Default::default());
        let record = sp_p2p::OpRecord {
            id: op.id.clone(),
            t: 1,
            origin: "peer".into(),
            op: serde_json::to_value(op).unwrap(),
            action: serde_json::to_value(action).unwrap(),
        };
        let mut data = sp_model::AppData::fresh();
        data.task.insert(&id, task);
        let snapshot = sp_p2p::Snapshot::new("peer", &serde_json::to_vec(&data).unwrap());
        let active = super::AtomicBool::new(false);
        super::apply_received(&engine, vec![record.clone()], Some(snapshot), &active).unwrap();
        assert!(engine.set_done(id.clone(), true).changed);
        super::apply_received(&engine, vec![record], None, &active).unwrap();
        assert!(engine.task_detail(id).unwrap().is_done);
        assert_eq!(engine.with_store(|s| s.meta.p2p_applied_ops.len()), 1);
        // Once the transport no longer includes the acknowledged batch, its receipts
        // are pruned in the next durable receive instead of accumulating forever.
        super::apply_received(&engine, vec![], None, &active).unwrap();
        let reopened = super::Engine::open(dir.path().to_string_lossy().into_owned());
        assert!(reopened.with_store(|s| s.meta.p2p_applied_ops.is_empty()));
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
