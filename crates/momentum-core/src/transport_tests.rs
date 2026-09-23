// SPDX-License-Identifier: GPL-3.0-or-later
//! Deterministic exchanges pause at a real HTTP boundary; only disposable stores are used.
use crate::*;
use sp_model::{AppData, Task, INBOX_PROJECT_ID};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc, Arc,
};
use std::time::Duration;

pub(crate) struct PausedDav {
    pub(crate) settings: NextcloudSettings,
    pub(crate) arrived: mpsc::Receiver<()>,
    resume: Option<mpsc::Sender<()>>,
    stop: Arc<AtomicBool>,
    puts: Arc<AtomicUsize>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl PausedDav {
    pub(crate) fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let settings = NextcloudSettings {
            server_url: format!("http://{}", listener.local_addr().unwrap()),
            user_name: "fixture".into(),
            password: "fixture".into(),
            folder: "fixture".into(),
            compress: false,
            encryption_password: None,
        };
        let (arrival, arrived) = mpsc::channel();
        let (resume, released) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let puts = Arc::new(AtomicUsize::new(0));
        let uploads = puts.clone();
        let thread = std::thread::spawn(move || {
            let file = sp_sync::SyncFile {
                version: 2,
                sync_version: 1,
                schema_version: sp_model::SCHEMA_VERSION,
                vector_clock: Default::default(),
                last_modified: 1,
                client_id: "remote".into(),
                state: AppData::fresh(),
                archive_young: None,
                archive_old: None,
                recent_ops: vec![],
                oldest_op_sync_version: None,
            };
            let response = sp_sync::encode(&file, false, None).unwrap();
            let mut first = true;
            while !stopped.load(Ordering::SeqCst) {
                let (mut stream, _) = match listener.accept() {
                    Ok(pair) => pair,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(1));
                        continue;
                    }
                    Err(e) => panic!("fixture accept: {e}"),
                };
                stream.set_nonblocking(false).unwrap();
                stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
                let mut request = vec![];
                let mut byte = [0];
                while !request.ends_with(b"\r\n\r\n") {
                    if stream.read_exact(&mut byte).is_err() {
                        return;
                    }
                    request.push(byte[0]);
                }
                let head = String::from_utf8(request).unwrap();
                if head.starts_with("PUT ") {
                    uploads.fetch_add(1, Ordering::SeqCst);
                }
                let length = head
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                let mut body = vec![0; length];
                stream.read_exact(&mut body).unwrap();
                if first {
                    assert!(head.starts_with("GET "));
                    first = false;
                    arrival.send(()).unwrap();
                    if released.recv_timeout(Duration::from_secs(5)).is_err() {
                        return;
                    }
                }
                let (status, body) = if head.starts_with("GET ") {
                    ("200 OK", response.as_str())
                } else {
                    ("204 No Content", "")
                };
                let reply = format!(
                    "HTTP/1.1 {status}\r\nETag: \"fixture\"\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(reply.as_bytes());
            }
        });
        Self {
            settings,
            arrived,
            resume: Some(resume),
            stop,
            puts,
            thread: Some(thread),
        }
    }
    pub(crate) fn release(&mut self) {
        self.resume.take().unwrap().send(()).unwrap();
    }
}
impl Drop for PausedDav {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.resume.take();
        self.thread.take().unwrap().join().unwrap();
    }
}
fn titles(engine: &Engine) -> Vec<String> {
    let mut titles = engine.all_tasks().into_iter().map(|t| t.title).collect::<Vec<_>>();
    titles.sort();
    titles
}

#[test]
fn import_during_exchange_rejects_the_old_result_without_writing_over_replacement() {
    let dir = tempfile::tempdir().unwrap();
    let live = dir.path().join("live");
    let engine = Engine::open(live.to_string_lossy().into_owned());
    engine.add_task("Before restore".into(), View::Today);
    let mut server = PausedDav::new();
    let syncing = engine.clone();
    let settings = server.settings.clone();
    let exchange = std::thread::spawn(move || syncing.sync_nextcloud(settings));
    server.arrived.recv_timeout(Duration::from_secs(5)).unwrap();
    let mut replacement = AppData::fresh();
    let task = Task::new("Restored", INBOX_PROJECT_ID);
    replacement.task.insert(&task.id.clone(), task);
    let backup = dir.path().join("backup.json");
    std::fs::write(&backup, serde_json::to_vec(&replacement).unwrap()).unwrap();
    engine.import_backup(backup.to_string_lossy().into_owned()).unwrap();
    engine.add_task("After restore".into(), View::Today);
    server.release();
    assert!(
        exchange.join().unwrap().is_err(),
        "the old exchange must not report success"
    );
    assert_eq!(titles(&engine), ["After restore", "Restored"]);
    assert_eq!(
        titles(&Engine::open(live.to_string_lossy().into_owned())),
        ["After restore", "Restored"]
    );
    assert_eq!(engine.pending_count(), 1);
    assert!(!engine.is_syncing());
    assert_eq!(
        server.puts.load(Ordering::SeqCst),
        0,
        "import invalidates the exchange before its next upload"
    );
}

#[test]
fn edits_during_exchange_remain_pending_and_keep_their_vector_clock() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open(dir.path().to_string_lossy().into_owned());
    engine.add_task("Before exchange".into(), View::Today);
    let mut server = PausedDav::new();
    let syncing = engine.clone();
    let settings = server.settings.clone();
    let exchange = std::thread::spawn(move || syncing.sync_nextcloud(settings));
    server.arrived.recv_timeout(Duration::from_secs(5)).unwrap();
    engine.add_task("During exchange".into(), View::Today);
    let clock = engine.with_store(|s| s.meta.vector_clock.clone());
    server.release();
    exchange.join().unwrap().unwrap();
    assert_eq!(titles(&engine), ["Before exchange", "During exchange"]);
    assert_eq!(engine.pending_count(), 1);
    engine.with_store(|s| assert_eq!(s.meta.vector_clock, clock));
}

#[test]
fn invalid_backup_preserves_tasks_pending_undo_and_disk() {
    let dir = tempfile::tempdir().unwrap();
    let live = dir.path().join("live");
    let engine = Engine::open(live.to_string_lossy().into_owned());
    engine.add_task("Keep me".into(), View::Search);
    engine.plan_for_today(vec![engine.all_tasks()[0].id.clone()]);
    assert!(engine.can_undo(), "fixture has a real undo batch before invalid import");
    let backup = dir.path().join("not-a-backup.json");
    std::fs::write(&backup, b"{\"data\":{}}").unwrap();
    assert!(matches!(
        engine.import_backup(backup.to_string_lossy().into_owned()),
        Err(CoreError::Invalid { .. })
    ));
    assert_eq!(titles(&engine), ["Keep me"]);
    assert_eq!(engine.pending_count(), 2);
    assert!(engine.can_undo());
    assert_eq!(titles(&Engine::open(live.to_string_lossy().into_owned())), ["Keep me"]);
}

#[test]
fn stalled_http_request_obeys_the_whole_exchange_deadline() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = sp_store::Store::load(dir.path().to_path_buf());
    store.save().unwrap();
    let before = std::fs::read(dir.path().join("state.json")).unwrap();
    let mut server = PausedDav::new();
    let settings = &server.settings;
    let cfg = sp_sync::NextcloudCfg {
        server_url: settings.server_url.clone(),
        user_name: settings.user_name.clone(),
        password: settings.password.clone(),
        folder: settings.folder.clone(),
        ..Default::default()
    };
    let exchange =
        std::thread::spawn(move || sp_sync::exchange_guarded(&cfg, &mut store, Duration::from_millis(100), || true));
    server.arrived.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(matches!(exchange.join().unwrap(), Err(sp_sync::SyncError::Deadline)));
    server.release();
    assert_eq!(std::fs::read(dir.path().join("state.json")).unwrap(), before);
    assert_eq!(server.puts.load(Ordering::SeqCst), 0);
}

#[test]
fn persistence_failure_does_not_acknowledge_or_drop_live_pending_changes() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open(dir.path().to_string_lossy().into_owned());
    engine.add_task("Keep pending".into(), View::Today);
    let mut server = PausedDav::new();
    let syncing = engine.clone();
    let settings = server.settings.clone();
    let exchange = std::thread::spawn(move || syncing.sync_nextcloud(settings));
    server.arrived.recv_timeout(Duration::from_secs(5)).unwrap();
    std::fs::create_dir(dir.path().join("state.json.tmp")).unwrap();
    server.release();
    assert!(matches!(exchange.join().unwrap(), Err(CoreError::Io { .. })));
    assert_eq!(engine.pending_count(), 1);
    assert_eq!(titles(&engine), ["Keep pending"]);
    assert_eq!(engine.sync_status().last_nextcloud_ms, 0);
    assert!(!engine.is_syncing());
    assert_eq!(sp_store::Store::load(dir.path().to_path_buf()).pending.len(), 1);
}

#[test]
fn failed_import_preserves_live_tasks_pending_and_undo() {
    let dir = tempfile::tempdir().unwrap();
    let live = dir.path().join("live");
    let engine = Engine::open(live.to_string_lossy().into_owned());
    engine.add_task("Keep me".into(), View::Search);
    engine.plan_for_today(vec![engine.all_tasks()[0].id.clone()]);
    assert!(engine.can_undo());
    let before_pending = engine.pending_count();
    let backup = dir.path().join("valid-backup.json");
    std::fs::write(&backup, serde_json::to_vec(&AppData::fresh()).unwrap()).unwrap();
    std::fs::create_dir(live.join("state.json.tmp")).unwrap();
    assert!(matches!(
        engine.import_backup(backup.to_string_lossy().into_owned()),
        Err(CoreError::Io { .. })
    ));
    assert_eq!(titles(&engine), ["Keep me"]);
    assert_eq!(engine.pending_count(), before_pending);
    assert!(engine.can_undo());
    assert_eq!(titles(&Engine::open(live.to_string_lossy().into_owned())), ["Keep me"]);
}

#[test]
fn checked_open_reports_unrecoverable_store_without_replacing_it() {
    let dir = tempfile::tempdir().unwrap();
    let live = dir.path().join("live");
    let engine = Engine::open(live.to_string_lossy().into_owned());
    engine.add_task("Keep me".into(), View::Today);
    drop(engine);
    let before = std::fs::read(live.join("state.json")).unwrap();
    std::fs::create_dir(live.join(".momentum-transaction")).unwrap();
    std::fs::write(live.join(".momentum-transaction/manifest.json"), b"{}").unwrap();
    assert!(matches!(
        Engine::open_checked(live.to_string_lossy().into_owned()),
        Err(CoreError::Io { .. })
    ));
    assert_eq!(std::fs::read(live.join("state.json")).unwrap(), before);
}

#[test]
fn failed_monitor_reload_preserves_the_live_owner_and_recovery_record() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open(dir.path().to_string_lossy().into_owned());
    engine.add_task("Keep me".into(), View::Today);
    let before = std::fs::read(dir.path().join("state.json")).unwrap();
    let journal = dir.path().join(".momentum-transaction");
    std::fs::create_dir(&journal).unwrap();
    std::fs::write(journal.join("manifest.json"), "{}").unwrap();
    assert!(!engine.reload_from_disk());
    assert_eq!(engine.with_store(|s| s.state.task.ids.len()), 1);
    assert_eq!(std::fs::read(dir.path().join("state.json")).unwrap(), before);
    assert!(journal.exists());
}

#[test]
fn explicit_cancellation_preserves_edits_and_allows_a_fresh_exchange() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open(dir.path().to_string_lossy().into_owned());
    assert!(!engine.cancel_nextcloud());
    engine.add_task("Before cancellation".into(), View::Today);
    let mut server = PausedDav::new();
    let syncing = engine.clone();
    let settings = server.settings.clone();
    let exchange = std::thread::spawn(move || syncing.sync_nextcloud(settings));
    server.arrived.recv_timeout(Duration::from_secs(5)).unwrap();
    let requested = engine.cancel_nextcloud();
    assert!(
        engine.is_syncing(),
        "cancellation must not admit another provider before the request drains"
    );
    engine.add_task("After cancellation".into(), View::Today);
    server.release();
    let result = exchange.join().unwrap();
    assert!(requested);
    assert!(
        result.is_err(),
        "a cancelled exchange must not commit or report success"
    );
    assert_eq!(server.puts.load(Ordering::SeqCst), 0);
    assert_eq!(engine.pending_count(), 2);
    assert_eq!(engine.sync_status().last_nextcloud_ms, 0);
    assert!(!engine.is_syncing());
    assert_eq!(
        titles(&Engine::open(dir.path().to_string_lossy().into_owned())),
        ["After cancellation", "Before cancellation"]
    );
    // Cancelling a completed/idle lease must not poison the next cycle.
    assert!(!engine.cancel_nextcloud());
    let mut server = PausedDav::new();
    let syncing = engine.clone();
    let settings = server.settings.clone();
    let exchange = std::thread::spawn(move || syncing.sync_nextcloud(settings));
    server.arrived.recv_timeout(Duration::from_secs(5)).unwrap();
    server.release();
    assert!(exchange.join().unwrap().is_ok());
    assert_eq!(engine.pending_count(), 0);
    assert!(engine.sync_status().last_nextcloud_ms > 0);
}

#[test]
fn cancellation_before_executor_admission_never_starts_http() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open(dir.path().to_string_lossy().into_owned());
    engine.add_task("Keep queued edit".into(), View::Today);
    let mut server = PausedDav::new();
    let cancelled = SyncCancellation::new();
    assert!(cancelled.cancel());
    assert!(engine
        .sync_nextcloud_cancellable(server.settings.clone(), cancelled.clone())
        .is_err());
    assert!(server.arrived.try_recv().is_err());
    assert!(!engine.is_syncing());
    assert_eq!(engine.pending_count(), 1);
    assert_eq!(engine.sync_status().last_nextcloud_ms, 0);
    assert!(cancelled.is_cancelled());
    let syncing = engine.clone();
    let settings = server.settings.clone();
    let fresh = SyncCancellation::new();
    let completed = fresh.clone();
    let exchange = std::thread::spawn(move || syncing.sync_nextcloud_cancellable(settings, fresh));
    server.arrived.recv_timeout(Duration::from_secs(5)).unwrap();
    server.release();
    assert!(exchange.join().unwrap().is_ok());
    assert_eq!(engine.pending_count(), 0);
    assert!(
        !completed.cancel(),
        "a completed operation must not be reported as cancelled"
    );
    assert!(!completed.is_cancelled());
    assert!(
        engine
            .sync_nextcloud_cancellable(server.settings.clone(), completed)
            .is_err(),
        "an operation handle is single-use"
    );
}

#[test]
fn nextcloud_connection_probe_does_not_mutate_store_or_sync_status() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open(dir.path().to_string_lossy().into_owned());
    engine.add_task("Keep queued edit".into(), View::Today);
    let before_status = engine.sync_status();
    let before_files: Vec<_> = ["state.json", "pending.json", "meta.json"]
        .iter()
        .map(|name| std::fs::read(dir.path().join(name)).unwrap())
        .collect();
    let server = PausedDav::new();
    let cancellation = SyncCancellation::new();
    assert!(
        engine
            .test_nextcloud_connection_cancellable(server.settings.clone(), cancellation, 1)
            .is_err(),
        "the held fixture must hit the injected deadline"
    );
    assert_eq!(engine.sync_status(), before_status);
    assert_eq!(engine.pending_count(), 1);
    for (name, bytes) in ["state.json", "pending.json", "meta.json"].iter().zip(before_files) {
        assert_eq!(
            std::fs::read(dir.path().join(name)).unwrap(),
            bytes,
            "probe changed {name}"
        );
    }
}

#[test]
fn operation_cancellation_drains_the_admitted_exchange_without_upload_or_commit() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open(dir.path().to_string_lossy().into_owned());
    engine.add_task("Keep before cancellation".into(), View::Today);
    let mut server = PausedDav::new();
    let cancellation = SyncCancellation::new();
    let signal = cancellation.clone();
    let syncing = engine.clone();
    let settings = server.settings.clone();
    let exchange = std::thread::spawn(move || syncing.sync_nextcloud_cancellable(settings, cancellation));
    server.arrived.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(signal.cancel());
    assert!(
        !signal.cancel(),
        "a second cancellation does not accept another transition"
    );
    assert!(engine.is_syncing(), "the transport lease remains held until I/O drains");
    engine.add_task("Keep during cancellation".into(), View::Today);
    assert!(matches!(
        engine.sync_nextcloud(server.settings.clone()),
        Err(CoreError::Busy)
    ));
    server.release();
    assert!(exchange.join().unwrap().is_err());
    assert_eq!(server.puts.load(Ordering::SeqCst), 0);
    assert_eq!(engine.pending_count(), 2);
    assert_eq!(engine.sync_status().last_nextcloud_ms, 0);
    assert!(!engine.is_syncing());
    assert_eq!(
        titles(&Engine::open(dir.path().to_string_lossy().into_owned())),
        ["Keep before cancellation", "Keep during cancellation"]
    );
}
