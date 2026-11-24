// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Host subprocess acceptance. The controller kills the writer without running destructors.
use momentum_core::{Engine, Message, NextcloudSettings, View};

#[allow(dead_code, deprecated)]
#[path = "../../momentum-sync/src/mock_dav.rs"]
mod mock_dav;
use std::{
    env, fs,
    io::{BufRead, BufReader, ErrorKind},
    net::TcpListener,
    path::Path,
    process::{Child, Command},
    thread,
    time::{Duration, Instant},
};

const FIXTURE_DIR: &str = "MOMENTUM_TERMINATION_FIXTURE_DIR";
const FIXTURE_CASE: &str = "MOMENTUM_TERMINATION_FIXTURE_CASE";
const FIXTURE_SYNC_URL: &str = "MOMENTUM_TERMINATION_FIXTURE_SYNC_URL";
const TIMEOUT: Duration = Duration::from_secs(15);

fn open(directory: &Path) -> std::sync::Arc<Engine> {
    Engine::open_checked(directory.to_str().unwrap().into()).unwrap()
}

fn fixture_titles(engine: &Engine) -> Vec<String> {
    let mut titles = engine
        .all_tasks()
        .into_iter()
        .map(|task| task.title)
        .collect::<Vec<_>>();
    titles.sort();
    titles
}

struct Writer(Child);

impl Writer {
    fn spawn(root: &Path, case: &str, sync_url: Option<&str>) -> Self {
        let mut command = Command::new(env::current_exe().unwrap());
        command
            .args(["--exact", "termination_writer_child", "--nocapture"])
            .env(FIXTURE_DIR, root)
            .env(FIXTURE_CASE, case);
        if let Some(url) = sync_url {
            command.env(FIXTURE_SYNC_URL, url);
        }
        Self(command.spawn().unwrap())
    }

    fn kill_at_acknowledgement(mut self, root: &Path) {
        let deadline = Instant::now() + TIMEOUT;
        while !root.join("acknowledged").is_file() {
            assert!(
                self.0.try_wait().unwrap().is_none(),
                "writer exited before acknowledging"
            );
            assert!(Instant::now() < deadline, "writer did not acknowledge before timeout");
            thread::sleep(Duration::from_millis(5));
        }
        self.kill();
    }

    fn kill(&mut self) {
        self.0.kill().unwrap();
        let status = self.0.wait().unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            assert_eq!(status.signal(), Some(9));
        }
        assert!(!status.success());
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

#[test]
fn termination_writer_child() {
    let Some(root) = env::var_os(FIXTURE_DIR) else { return };
    let root = Path::new(&root);
    let engine = open(&root.join("store"));
    match env::var(FIXTURE_CASE).unwrap().as_str() {
        "edit" => {
            let captured = engine.add_from_text("Captured one\nCaptured two".into(), View::Today);
            assert!(captured.changed);
            let id = engine
                .all_tasks()
                .into_iter()
                .find(|task| task.title == "Captured one")
                .unwrap()
                .id;
            assert!(engine.set_done(id, true).changed);
        }
        "export" => {
            let outcome = engine
                .export_backup(root.join("backup.json").to_str().unwrap().into())
                .unwrap();
            assert_eq!(outcome.message, Some(Message::BackupExported));
        }
        "restore" => {
            assert!(
                engine
                    .import_backup(root.join("backup.json").to_str().unwrap().into())
                    .unwrap()
                    .changed
            );
        }
        "recover" => {
            assert!(engine.restore_backup_recovery().unwrap().changed);
        }
        "sync" => {
            let result = engine.sync_nextcloud(sync_settings(env::var(FIXTURE_SYNC_URL).unwrap()));
            panic!("the controller must kill the writer mid-request: {result:?}");
        }
        _ => panic!("unknown interruption fixture"),
    }
    fs::write(root.join("acknowledged"), []).unwrap();
    loop {
        thread::park();
    }
}

#[test]
fn acknowledged_capture_and_completion_survive_process_kill() {
    let root = tempfile::tempdir().unwrap();
    Writer::spawn(root.path(), "edit", None).kill_at_acknowledgement(root.path());
    let engine = open(&root.path().join("store"));
    assert_eq!(fixture_titles(&engine), ["Captured one", "Captured two"]);
    let completed = engine
        .all_tasks()
        .into_iter()
        .find(|task| task.title == "Captured one")
        .unwrap();
    assert!(completed.is_done);
    assert_eq!(engine.pending_count(), 3);
    assert_eq!(engine.sync_status().last_nextcloud_ms, 0);
}

#[test]
fn acknowledged_backup_export_and_restore_survive_process_kill() {
    let root = tempfile::tempdir().unwrap();
    let store = root.path().join("store");
    assert!(open(&store).add_task_for_today("Backed up task".into()).changed);
    Writer::spawn(root.path(), "export", None).kill_at_acknowledgement(root.path());
    fs::remove_file(root.path().join("acknowledged")).unwrap();

    let before_restore = open(&store);
    assert!(before_restore.add_task_for_today("Unrestored task".into()).changed);
    assert_eq!(before_restore.pending_count(), 2);
    drop(before_restore);
    Writer::spawn(root.path(), "restore", None).kill_at_acknowledgement(root.path());

    let restored = open(&store);
    assert_eq!(fixture_titles(&restored), ["Backed up task"]);
    assert_eq!(restored.pending_count(), 0);
    assert_eq!(restored.sync_status().last_nextcloud_ms, 0);
    assert!(restored.has_backup_recovery());
    drop(restored);
    fs::remove_file(root.path().join("acknowledged")).unwrap();
    Writer::spawn(root.path(), "recover", None).kill_at_acknowledgement(root.path());

    let recovered = open(&store);
    assert_eq!(fixture_titles(&recovered), ["Backed up task", "Unrestored task"]);
    assert_eq!(recovered.pending_count(), 2);
}

#[test]
fn interrupted_nextcloud_request_keeps_pending_edit_retryable() {
    let root = tempfile::tempdir().unwrap();
    let store = root.path().join("store");
    assert!(open(&store).add_task_for_today("Pending sync task".into()).changed);

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let settings = sync_settings(format!("http://{}", listener.local_addr().unwrap()));
    let mut writer = Writer::spawn(root.path(), "sync", Some(&settings.server_url));
    let deadline = Instant::now() + TIMEOUT;
    let stream = loop {
        match listener.accept() {
            Ok((stream, _)) => break stream,
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                assert!(writer.0.try_wait().unwrap().is_none(), "writer exited before syncing");
                assert!(Instant::now() < deadline, "writer did not connect before timeout");
                thread::sleep(Duration::from_millis(5));
            }
            Err(error) => panic!("loopback accept failed: {error}"),
        }
    };
    stream.set_nonblocking(false).unwrap();
    stream.set_read_timeout(Some(TIMEOUT)).unwrap();
    let mut request = String::new();
    BufReader::new(&stream).read_line(&mut request).unwrap();
    assert!(request.starts_with("GET ") && request.contains("/fixture/sync-data.json"));
    writer.kill();
    drop(stream);
    let recovered = open(&store);
    assert_eq!(fixture_titles(&recovered), ["Pending sync task"]);
    assert_eq!(recovered.pending_count(), 1);
    assert_eq!(recovered.sync_status().last_nextcloud_ms, 0);

    listener.set_nonblocking(false).unwrap();
    let dav = mock_dav::MockDav::serve(listener);
    let retried = recovered.sync_nextcloud(settings.clone()).unwrap();
    assert!(retried.uploaded);
    assert_eq!(retried.ops_uploaded, 1);
    drop(recovered);

    let reopened = open(&store);
    assert_eq!(fixture_titles(&reopened), ["Pending sync task"]);
    assert_eq!(reopened.pending_count(), 0);
    assert!(reopened.sync_status().last_nextcloud_ms > 0);
    assert_eq!(dav.puts.load(std::sync::atomic::Ordering::SeqCst), 1);
    let peer = open(&root.path().join("peer"));
    assert!(peer.sync_nextcloud(settings).unwrap().downloaded);
    assert_eq!(fixture_titles(&peer), ["Pending sync task"]);
}

fn sync_settings(server_url: String) -> NextcloudSettings {
    NextcloudSettings {
        server_url,
        user_name: "u".into(),
        password: "p".into(),
        folder: "fixture".into(),
        compress: false,
        encryption_password: None,
        optimized: false,
    }
}
