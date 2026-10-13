// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
struct Settings {
    enabled: AtomicBool,
    out: String,
    received: Mutex<Vec<String>>,
    failed: Mutex<Vec<String>>,
}
impl Settings {
    fn new(value: &str) -> Arc<Self> {
        Arc::new(Self {
            enabled: AtomicBool::new(true),
            out: value.into(),
            received: Mutex::new(vec![]),
            failed: Mutex::new(vec![]),
        })
    }
}
impl SettingsHandler for Settings {
    fn enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }
    fn outgoing(&self) -> Result<String> {
        Ok(self.out.clone())
    }
    fn receive(&self, _peer: &str, json: &str) -> Result<()> {
        self.received.lock().unwrap().push(json.into());
        Ok(())
    }
    fn failed(&self, error: String) {
        self.failed.lock().unwrap().push(error);
    }
}
fn node(dir: &Path, id: &str) -> P2p {
    P2p::start(
        dir.join(id),
        id,
        id,
        Box::new(FileKeyStore::new(dir.join(format!("{id}-keys"))).unwrap()),
        0,
    )
    .unwrap()
}
fn trust(a: &P2p, b: &P2p) {
    b.handler.set_app_key(&a.handler.app_key().unwrap()).unwrap();
    a.handler.remember(
        &b.identity,
        &b.handler.device_keys().unwrap().fingerprint(),
        Some(([127, 0, 0, 1], b.listen_addr().port()).into()),
    );
    b.handler.remember(
        &a.identity,
        &a.handler.device_keys().unwrap().fingerprint(),
        Some(([127, 0, 0, 1], a.listen_addr().port()).into()),
    );
}
fn tasks(a: &P2p, b: &P2p) {
    let ticket = a
        .sync_with(
            &a.devices()
                .into_iter()
                .find(|p| p.device_id == b.identity.device_id)
                .unwrap(),
        )
        .unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(Event::TaskFinished { ticket: t, result }) = a.events().try_recv().unwrap() {
            if t == ticket.0 {
                assert!(matches!(result, SyncResult::Success), "{result:?}");
                break;
            }
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn optional_channel_negotiates_and_never_leaks_values_into_task_state() {
    let d = tempfile::tempdir().unwrap();
    let a = node(d.path(), "a");
    let b = node(d.path(), "b");
    trust(&a, &b);
    let ah = Settings::new("settings-a");
    let bh = Settings::new("settings-b");
    a.configure_settings(ah.clone()).unwrap();
    b.configure_settings(bh.clone()).unwrap();
    tasks(&a, &b);
    a.sync_settings_with(&a.devices()[0]).unwrap();
    assert!(ah.received.lock().unwrap().contains(&"settings-b".into()));
    assert!(bh.received.lock().unwrap().contains(&"settings-a".into()));
    let (tx, rx) = std::sync::mpsc::channel();
    a.engine
        .run(move |e| {
            tx.send(e.state().lock().unwrap().snapshot()).unwrap();
            Ok(())
        })
        .unwrap();
    let entries = rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(!format!("{entries:?}").contains("settings-a"));
    bh.enabled.store(false, Ordering::SeqCst);
    b.configure_settings(bh.clone()).unwrap();
    tasks(&a, &b);
    let count = bh.received.lock().unwrap().len();
    assert!(a.sync_settings_with(&a.devices()[0]).is_err());
    assert_eq!(bh.received.lock().unwrap().len(), count);
    bh.enabled.store(true, Ordering::SeqCst);
    b.configure_settings(bh).unwrap();
    tasks(&a, &b);
    assert!(a.sync_settings_with(&a.devices()[0]).is_ok());
    a.shutdown();
    b.shutdown();
}
#[test]
fn old_peer_reports_unsupported_without_breaking_task_sync() {
    let d = tempfile::tempdir().unwrap();
    let a = node(d.path(), "a");
    let old = node(d.path(), "old");
    trust(&a, &old);
    a.configure_settings(Settings::new("settings-a")).unwrap();
    tasks(&a, &old);
    let error = a.sync_settings_with(&a.devices()[0]).unwrap_err().to_string();
    assert!(error.contains("unsupported"), "{error}");
    // The former channel used transfer success as a global acknowledgement.
    // Its capability must be rejected before any v2 settings are published.
    old.journal().settings_own = Some(settings::SettingsCapability {
        device_id: old.identity.device_id.clone(),
        version: 1,
        port: old.listen_addr().port(),
    });
    tasks(&a, &old);
    let error = a.sync_settings_with(&a.devices()[0]).unwrap_err().to_string();
    assert!(error.contains("unsupported settings channel version"), "{error}");
    tasks(&a, &old);
    a.shutdown();
    old.shutdown();
}
