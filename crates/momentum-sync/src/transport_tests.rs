// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::mock_dav::MockDav;
use momentum_model::{Task, INBOX_PROJECT_ID};
use momentum_oplog::Action;
use std::sync::atomic::Ordering;

fn cfg(dav: &MockDav) -> NextcloudCfg {
    NextcloudCfg {
        server_url: dav.url.clone(),
        user_name: "u".into(),
        password: "p".into(),
        folder: "legacy".into(),
        compress: true,
        encrypt_key: None,
    }
}
fn fixture() -> (Store, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::load(dir.path().to_path_buf());
    store.state.rest.insert("globalConfig".into(), serde_json::json!({}));
    (store, dir)
}
fn add(store: &mut Store, title: &str) {
    store.dispatch(Action::AddTask {
        task: Task::new(title, INBOX_PROJECT_ID),
        bottom: true,
    });
}
#[test]
fn pending_edits_validate_baseline_without_redownloading_and_still_publish() {
    let dav = MockDav::start();
    let cfg = cfg(&dav);
    let (mut store, _dir) = fixture();
    let mut session = SyncSession::default();
    add(&mut store, "first");
    session
        .exchange_guarded(&cfg, &mut store, DEFAULT_EXCHANGE_TIMEOUT, || true)
        .unwrap();
    dav.downloaded_bytes.store(0, Ordering::SeqCst);
    add(&mut store, "second");
    let report = session
        .exchange_guarded(&cfg, &mut store, DEFAULT_EXCHANGE_TIMEOUT, || true)
        .unwrap();
    assert!(report.uploaded);
    assert_eq!(dav.not_modified.load(Ordering::SeqCst), 1);
    assert_eq!(dav.downloaded_bytes.load(Ordering::SeqCst), 0);
    let (remote, _) = download(&cfg).unwrap().unwrap();
    assert_eq!(remote.state.task.entities.len(), 2);
    assert!(store.pending.is_empty());
}
#[test]
fn restart_reuses_an_exact_persisted_remote_baseline_with_pending_edits() {
    let dav = MockDav::start();
    let cfg = cfg(&dav);
    let (mut store, _dir) = fixture();
    add(&mut store, "first");
    SyncSession::default()
        .exchange_guarded(&cfg, &mut store, DEFAULT_EXCHANGE_TIMEOUT, || true)
        .unwrap();
    store.save().unwrap();
    let mut restarted = Store::load(store.dir().to_path_buf());
    add(&mut restarted, "after restart");
    dav.downloaded_bytes.store(0, Ordering::SeqCst);
    SyncSession::default()
        .exchange_guarded(&cfg, &mut restarted, DEFAULT_EXCHANGE_TIMEOUT, || true)
        .unwrap();
    assert_eq!(dav.not_modified.load(Ordering::SeqCst), 1);
    assert_eq!(dav.downloaded_bytes.load(Ordering::SeqCst), 0);
    let (remote, _) = download(&cfg).unwrap().unwrap();
    assert_eq!(remote.state.task.entities.len(), 2);
}
#[test]
fn a_conflict_after_conditional_read_refetches_before_retrying() {
    let dav = MockDav::start();
    let cfg = cfg(&dav);
    let (mut store, _dir) = fixture();
    let mut session = SyncSession::default();
    add(&mut store, "first");
    session
        .exchange_guarded(&cfg, &mut store, DEFAULT_EXCHANGE_TIMEOUT, || true)
        .unwrap();
    add(&mut store, "second");
    dav.fail_next_put.store(true, Ordering::SeqCst);
    dav.downloaded_bytes.store(0, Ordering::SeqCst);
    session
        .exchange_guarded(&cfg, &mut store, DEFAULT_EXCHANGE_TIMEOUT, || true)
        .unwrap();
    assert_eq!(dav.not_modified.load(Ordering::SeqCst), 1);
    assert!(dav.downloaded_bytes.load(Ordering::SeqCst) > 0);
    assert!(store.pending.is_empty());
}
#[test]
fn shared_pool_reuses_a_keepalive_connection() {
    use std::net::TcpListener;
    use std::sync::{atomic::AtomicUsize, Arc};
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let connections = Arc::new(AtomicUsize::new(0));
    let accepted = connections.clone();
    std::thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            accepted.fetch_add(1, Ordering::SeqCst);
            std::thread::spawn(move || {
                stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
                loop {
                    let mut head = Vec::new();
                    let mut byte = [0];
                    while !head.ends_with(b"\r\n\r\n") {
                        if stream.read(&mut byte).unwrap_or(0) == 0 {
                            return;
                        }
                        head.push(byte[0]);
                    }
                    if stream
                        .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok")
                        .is_err()
                    {
                        return;
                    }
                }
            });
        }
    });
    let guard = ExchangeGuard {
        current: &|| true,
        deadline: Instant::now() + Duration::from_secs(3),
    };
    for _ in 0..2 {
        assert_eq!(
            guard
                .agent()
                .unwrap()
                .get(&url)
                .call()
                .unwrap()
                .body_mut()
                .read_to_string()
                .unwrap(),
            "ok"
        );
    }
    assert_eq!(connections.load(Ordering::SeqCst), 1);
}

#[test]
fn portable_unchanged_validation_preserves_bootstrap_consent() {
    let dav = MockDav::start();
    let cfg = cfg(&dav);
    let mut session = PortableSession::default();
    session
        .exchange_guarded(&cfg, DEFAULT_EXCHANGE_TIMEOUT, || true, |_| Ok(Some("settings".into())))
        .unwrap();
    dav.downloaded_bytes.store(0, Ordering::SeqCst);
    let puts = dav.puts.load(Ordering::SeqCst);
    let report = session
        .exchange_guarded(
            &cfg,
            DEFAULT_EXCHANGE_TIMEOUT,
            || true,
            |remote| {
                assert_eq!(remote, Some("settings"));
                Ok(None)
            },
        )
        .unwrap();
    assert!(!report.uploaded);
    assert_eq!(report.remote.as_deref(), Some("settings"));
    assert_eq!(dav.not_modified.load(Ordering::SeqCst), 1);
    assert_eq!(dav.downloaded_bytes.load(Ordering::SeqCst), 0);
    assert_eq!(dav.puts.load(Ordering::SeqCst), puts);
}

#[cfg(unix)]
#[test]
fn idle_validation_does_not_rewrite_the_durable_baseline() {
    use std::os::unix::fs::MetadataExt;
    let dav = MockDav::start();
    let cfg = cfg(&dav);
    let (mut store, _dir) = fixture();
    add(&mut store, "initial");
    let mut session = SyncSession::default();
    session
        .exchange_guarded(&cfg, &mut store, DEFAULT_EXCHANGE_TIMEOUT, || true)
        .unwrap();
    let path = store.dir().join(BASELINE_FILE);
    let before = std::fs::metadata(&path).unwrap().ino();
    session
        .exchange_guarded(&cfg, &mut store, DEFAULT_EXCHANGE_TIMEOUT, || true)
        .unwrap();
    assert_eq!(std::fs::metadata(path).unwrap().ino(), before);
}

#[test]
fn corrupt_or_mismatched_restart_cache_falls_back_to_full_read() {
    for fault in 0..4 {
        let dav = MockDav::start();
        let mut cfg = cfg(&dav);
        let (mut store, _dir) = fixture();
        add(&mut store, "initial");
        SyncSession::default()
            .exchange_guarded(&cfg, &mut store, DEFAULT_EXCHANGE_TIMEOUT, || true)
            .unwrap();
        let path = store.dir().join(BASELINE_FILE);
        let mut cached: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        match fault {
            0 => cached["digest"] = "corrupt".into(),
            1 => cached["tag"] = "W/\"weak\"".into(),
            2 => cached["sync_version"] = 999.into(),
            _ => cfg.compress = !cfg.compress,
        }
        std::fs::write(path, serde_json::to_vec(&cached).unwrap()).unwrap();
        dav.downloaded_bytes.store(0, Ordering::SeqCst);
        SyncSession::default()
            .exchange_guarded(&cfg, &mut store, DEFAULT_EXCHANGE_TIMEOUT, || true)
            .unwrap();
        assert_eq!(dav.not_modified.load(Ordering::SeqCst), 0, "fault {fault}");
        assert!(dav.downloaded_bytes.load(Ordering::SeqCst) > 0, "fault {fault}");
    }
}
#[test]
fn failed_exchange_removes_persisted_validation_and_preserves_pending() {
    let dav = MockDav::start();
    let cfg = cfg(&dav);
    let (mut store, _dir) = fixture();
    let mut session = SyncSession::default();
    add(&mut store, "first");
    session
        .exchange_guarded(&cfg, &mut store, DEFAULT_EXCHANGE_TIMEOUT, || true)
        .unwrap();
    add(&mut store, "unconfirmed");
    dav.hide_after_put.store(true, Ordering::SeqCst);
    assert!(session
        .exchange_guarded(&cfg, &mut store, DEFAULT_EXCHANGE_TIMEOUT, || true)
        .is_err());
    assert_eq!(store.pending.len(), 1);
    assert!(!store.dir().join(BASELINE_FILE).exists());
    dav.hide_after_put.store(false, Ordering::SeqCst);
    let conditional_reads = dav.not_modified.load(Ordering::SeqCst);
    let gets = dav.gets.load(Ordering::SeqCst);
    SyncSession::default()
        .exchange_guarded(&cfg, &mut store, DEFAULT_EXCHANGE_TIMEOUT, || true)
        .unwrap();
    assert_eq!(dav.not_modified.load(Ordering::SeqCst), conditional_reads);
    assert_eq!(dav.gets.load(Ordering::SeqCst), gets + 1);
    let (remote, _) = download(&cfg).unwrap().unwrap();
    assert_eq!(remote.state.task.entities.len(), 2);
}
#[test]
fn remote_replacement_rebases_pending_on_the_new_snapshot() {
    let dav = MockDav::start();
    let cfg = cfg(&dav);
    let (mut store, _dir) = fixture();
    let mut session = SyncSession::default();
    add(&mut store, "original");
    session
        .exchange_guarded(&cfg, &mut store, DEFAULT_EXCHANGE_TIMEOUT, || true)
        .unwrap();
    let (mut remote, _) = download(&cfg).unwrap().unwrap();
    remote.state.task.entities.values_mut().next().unwrap().title = "replaced remotely".into();
    dav.set_raw(
        "/remote.php/dav/files/u/legacy/sync-data.json",
        encode(&remote, cfg.compress, None).unwrap().into_bytes(),
    );
    add(&mut store, "pending");
    session
        .exchange_guarded(&cfg, &mut store, DEFAULT_EXCHANGE_TIMEOUT, || true)
        .unwrap();
    let mut titles: Vec<_> = store.state.task.iter().map(|v| v.title.clone()).collect();
    titles.sort();
    assert_eq!(titles, ["pending", "replaced remotely"]);
}
#[test]
fn per_request_remaining_deadline_bounds_a_stalled_reused_connection() {
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        for index in 0..2 {
            let mut head = Vec::new();
            let mut byte = [0];
            while !head.ends_with(b"\r\n\r\n") {
                if stream.read(&mut byte).unwrap_or(0) == 0 {
                    return;
                }
                head.push(byte[0]);
            }
            if index == 1 {
                std::thread::sleep(Duration::from_millis(500));
            }
            if stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok")
                .is_err()
            {
                return;
            }
        }
    });
    let first = ExchangeGuard {
        current: &|| true,
        deadline: Instant::now() + Duration::from_secs(2),
    };
    first
        .agent()
        .unwrap()
        .get(&url)
        .call()
        .unwrap()
        .body_mut()
        .read_to_string()
        .unwrap();
    let second = ExchangeGuard {
        current: &|| true,
        deadline: Instant::now() + Duration::from_millis(80),
    };
    let start = Instant::now();
    assert!(second.agent().unwrap().get(&url).call().is_err());
    assert!(start.elapsed() < Duration::from_millis(350));
}
#[cfg(unix)]
#[test]
fn encrypted_baseline_is_private_and_contains_no_plaintext_credentials_or_tasks() {
    use std::os::unix::fs::PermissionsExt;
    let dav = MockDav::start();
    let mut cfg = cfg(&dav);
    cfg.encrypt_key = Some("encryption-fixture-secret".into());
    let (mut store, _dir) = fixture();
    add(&mut store, "private-task-fixture");
    SyncSession::default()
        .exchange_guarded(&cfg, &mut store, DEFAULT_EXCHANGE_TIMEOUT, || true)
        .unwrap();
    let path = store.dir().join(BASELINE_FILE);
    assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
    let cached = std::fs::read_to_string(path).unwrap();
    assert!(!cached.contains("private-task-fixture"));
    assert!(!cached.contains("encryption-fixture-secret"));
    assert!(!cached.contains("password"));
}

#[test]
fn portable_pending_changes_publish_then_validate_the_new_settings() {
    let dav = MockDav::start();
    let cfg = cfg(&dav);
    let mut session = PortableSession::default();
    session
        .exchange_guarded(&cfg, DEFAULT_EXCHANGE_TIMEOUT, || true, |_| Ok(Some("old".into())))
        .unwrap();
    let changed = session
        .exchange_guarded(
            &cfg,
            DEFAULT_EXCHANGE_TIMEOUT,
            || true,
            |remote| {
                assert_eq!(remote, Some("old"));
                Ok(Some("new".into()))
            },
        )
        .unwrap();
    assert!(changed.uploaded);
    dav.downloaded_bytes.store(0, Ordering::SeqCst);
    let unchanged = session
        .exchange_guarded(
            &cfg,
            DEFAULT_EXCHANGE_TIMEOUT,
            || true,
            |remote| {
                assert_eq!(remote, Some("new"));
                Ok(Some("new".into()))
            },
        )
        .unwrap();
    assert!(!unchanged.uploaded);
    assert_eq!(dav.downloaded_bytes.load(Ordering::SeqCst), 0);
}
#[test]
fn portable_merge_failure_and_configuration_change_discard_validation() {
    let dav = MockDav::start();
    let cfg = cfg(&dav);
    let mut session = PortableSession::default();
    session
        .exchange_guarded(&cfg, DEFAULT_EXCHANGE_TIMEOUT, || true, |_| Ok(Some("settings".into())))
        .unwrap();
    assert!(session
        .exchange_guarded(
            &cfg,
            DEFAULT_EXCHANGE_TIMEOUT,
            || true,
            |_| Err(SyncError::Parse("merge rejected".into()))
        )
        .is_err());
    for next in [
        &cfg,
        &NextcloudCfg {
            compress: !cfg.compress,
            ..cfg.clone()
        },
    ] {
        dav.downloaded_bytes.store(0, Ordering::SeqCst);
        session
            .exchange_guarded(next, DEFAULT_EXCHANGE_TIMEOUT, || true, |_| Ok(None))
            .unwrap();
        assert!(dav.downloaded_bytes.load(Ordering::SeqCst) > 0);
    }
}

#[test]
fn admission_check_precedes_loading_a_persisted_baseline() {
    let dav = MockDav::start();
    let cfg = cfg(&dav);
    let (mut store, _dir) = fixture();
    add(&mut store, "first");
    SyncSession::default()
        .exchange_guarded(&cfg, &mut store, DEFAULT_EXCHANGE_TIMEOUT, || true)
        .unwrap();
    let path = store.dir().join(BASELINE_FILE);
    let first_check = std::sync::atomic::AtomicBool::new(true);
    dav.downloaded_bytes.store(0, Ordering::SeqCst);
    SyncSession::default()
        .exchange_guarded(&cfg, &mut store, DEFAULT_EXCHANGE_TIMEOUT, || {
            if first_check.swap(false, Ordering::SeqCst) {
                // Simulate source admission invalidating a restored cache before work.
                std::fs::remove_file(&path).unwrap();
            }
            true
        })
        .unwrap();
    assert!(dav.downloaded_bytes.load(Ordering::SeqCst) > 0);
}

#[test]
fn lightweight_validation_checks_exact_remote_baseline_without_task_clone() {
    let dav = MockDav::start();
    let cfg = cfg(&dav);
    let (mut store, _dir) = fixture();
    add(&mut store, "initial");
    let mut session = SyncSession::default();
    session
        .exchange_guarded(&cfg, &mut store, DEFAULT_EXCHANGE_TIMEOUT, || true)
        .unwrap();
    dav.downloaded_bytes.store(0, Ordering::SeqCst);
    assert!(session
        .validate_unchanged_guarded(
            &cfg,
            store.meta.last_sync_version,
            store.meta.last_etag.as_deref(),
            DEFAULT_EXCHANGE_TIMEOUT,
            || true
        )
        .unwrap());
    assert_eq!(dav.not_modified.load(Ordering::SeqCst), 1);
    assert_eq!(dav.downloaded_bytes.load(Ordering::SeqCst), 0);
    let mut changed_cfg = cfg.clone();
    changed_cfg.compress = !cfg.compress;
    assert!(!session
        .validate_unchanged_guarded(
            &changed_cfg,
            store.meta.last_sync_version,
            store.meta.last_etag.as_deref(),
            DEFAULT_EXCHANGE_TIMEOUT,
            || true
        )
        .unwrap());
    assert!(!session
        .validate_unchanged_guarded(
            &cfg,
            store.meta.last_sync_version,
            store.meta.last_etag.as_deref(),
            DEFAULT_EXCHANGE_TIMEOUT,
            || true
        )
        .unwrap());
}

#[test]
fn lightweight_changed_remote_requires_exchange_and_drops_old_validation() {
    let dav = MockDav::start();
    let cfg = cfg(&dav);
    let (mut store, _dir) = fixture();
    add(&mut store, "original");
    let mut session = SyncSession::default();
    session
        .exchange_guarded(&cfg, &mut store, DEFAULT_EXCHANGE_TIMEOUT, || true)
        .unwrap();
    let (mut file, _) = download(&cfg).unwrap().unwrap();
    file.state.task.entities.values_mut().next().unwrap().title = "changed".into();
    dav.set_raw(
        "/remote.php/dav/files/u/legacy/sync-data.json",
        encode(&file, cfg.compress, None).unwrap().into_bytes(),
    );
    assert!(!session
        .validate_unchanged_guarded(
            &cfg,
            store.meta.last_sync_version,
            store.meta.last_etag.as_deref(),
            DEFAULT_EXCHANGE_TIMEOUT,
            || true
        )
        .unwrap());
    assert_eq!(store.state.task.iter().next().unwrap().title, "original");
    assert!(!store.dir().join(BASELINE_FILE).exists());
    let gets = dav.gets.load(Ordering::SeqCst);
    assert!(!session
        .validate_unchanged_guarded(
            &cfg,
            store.meta.last_sync_version,
            store.meta.last_etag.as_deref(),
            DEFAULT_EXCHANGE_TIMEOUT,
            || true
        )
        .unwrap());
    assert_eq!(dav.gets.load(Ordering::SeqCst), gets);
    assert!(
        session
            .exchange_guarded(&cfg, &mut store, DEFAULT_EXCHANGE_TIMEOUT, || true)
            .unwrap()
            .downloaded
    );
    assert_eq!(store.state.task.iter().next().unwrap().title, "changed");
}
#[test]
fn lightweight_validation_obeys_cancellation_and_deadline_without_requests() {
    for cancel in [true, false] {
        let dav = MockDav::start();
        let cfg = cfg(&dav);
        let (mut store, _dir) = fixture();
        add(&mut store, "original");
        let mut session = SyncSession::default();
        session
            .exchange_guarded(&cfg, &mut store, DEFAULT_EXCHANGE_TIMEOUT, || true)
            .unwrap();
        let gets = dav.gets.load(Ordering::SeqCst);
        let error = session
            .validate_unchanged_guarded(
                &cfg,
                store.meta.last_sync_version,
                store.meta.last_etag.as_deref(),
                if cancel {
                    DEFAULT_EXCHANGE_TIMEOUT
                } else {
                    Duration::ZERO
                },
                || !cancel,
            )
            .unwrap_err();
        assert!(if cancel {
            matches!(error, SyncError::Cancelled)
        } else {
            matches!(error, SyncError::Deadline)
        });
        assert_eq!(dav.gets.load(Ordering::SeqCst), gets);
        assert!(!store.dir().join(BASELINE_FILE).exists());
    }
}
#[test]
fn lightweight_validation_rejects_mismatched_local_metadata_without_requests() {
    let dav = MockDav::start();
    let cfg = cfg(&dav);
    let (mut store, _dir) = fixture();
    add(&mut store, "original");
    for wrong_version in [true, false] {
        let mut session = SyncSession::default();
        session
            .exchange_guarded(&cfg, &mut store, DEFAULT_EXCHANGE_TIMEOUT, || true)
            .unwrap();
        let gets = dav.gets.load(Ordering::SeqCst);
        assert!(!session
            .validate_unchanged_guarded(
                &cfg,
                if wrong_version {
                    store.meta.last_sync_version + 1
                } else {
                    store.meta.last_sync_version
                },
                if wrong_version {
                    store.meta.last_etag.as_deref()
                } else {
                    Some("\"wrong\"")
                },
                DEFAULT_EXCHANGE_TIMEOUT,
                || true
            )
            .unwrap());
        assert_eq!(dav.gets.load(Ordering::SeqCst), gets);
    }
}

#[test]
fn durable_configuration_binding_never_becomes_a_fast_secret_oracle() {
    let dav = MockDav::start();
    let cfg = NextcloudCfg {
        encrypt_key: Some("strong-key-fixture".into()),
        ..cfg(&dav)
    };
    let (mut store, _dir) = fixture();
    add(&mut store, "private");
    SyncSession::default()
        .exchange_guarded(&cfg, &mut store, DEFAULT_EXCHANGE_TIMEOUT, || true)
        .unwrap();
    let cached: serde_json::Value =
        serde_json::from_slice(&std::fs::read(store.dir().join(BASELINE_FILE)).unwrap()).unwrap();
    let changed_secrets = NextcloudCfg {
        password: "different-http-password".into(),
        encrypt_key: Some("different-encryption-password".into()),
        ..cfg.clone()
    };
    assert_eq!(
        cached["configuration"].as_str().unwrap(),
        configuration_fingerprint(&changed_secrets)
    );
    let wrong_auth = NextcloudCfg {
        password: "wrong-auth".into(),
        ..cfg.clone()
    };
    assert_eq!(
        SyncSession::default()
            .exchange_guarded(&wrong_auth, &mut store, DEFAULT_EXCHANGE_TIMEOUT, || true)
            .unwrap_err()
            .kind(),
        SyncFailureKind::Unauthorized
    );
    SyncSession::default()
        .exchange_guarded(&cfg, &mut store, DEFAULT_EXCHANGE_TIMEOUT, || true)
        .unwrap();
    let wrong_key = NextcloudCfg {
        encrypt_key: Some("wrong-key".into()),
        ..cfg.clone()
    };
    assert!(matches!(
        SyncSession::default().exchange_guarded(&wrong_key, &mut store, DEFAULT_EXCHANGE_TIMEOUT, || true),
        Err(SyncError::WrongEncryptionPassword)
    ));
}
