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
        ..Default::default()
    }
}
fn fixture() -> (Store, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::load(dir.path().into());
    s.state.rest.insert("globalConfig".into(), serde_json::json!({}));
    (s, dir)
}
fn add(s: &mut Store, title: &str) {
    s.dispatch(Action::AddTask {
        task: Task::new(title, INBOX_PROJECT_ID),
        bottom: true,
    });
}
fn run(c: &NextcloudCfg, s: &mut Store) -> Result<Report, SyncError> {
    exchange_guarded(c, s, DEFAULT_EXCHANGE_TIMEOUT, || true)
}
#[test]
fn optimized_bootstrap_and_existing_join_ignore_stale_legacy() {
    let dav = MockDav::start();
    let c = cfg(&dav);
    let (mut a, _a) = fixture();
    add(&mut a, "legacy");
    super::super::exchange(&c, &mut a).unwrap();
    let legacy = dav.raw("/remote.php/dav/files/u/legacy/sync-data.json");
    add(&mut a, "v3");
    run(&c, &mut a).unwrap();
    assert_eq!(dav.raw("/remote.php/dav/files/u/legacy/sync-data.json"), legacy);
    let (mut b, _b) = fixture();
    add(&mut b, "offline");
    run(&c, &mut b).unwrap();
    run(&c, &mut a).unwrap();
    assert_eq!(a.state.task.entities.len(), 3);
    assert_eq!(b.state.task.entities.len(), 3);
    assert!(a.pending.is_empty());
}
#[test]
fn failure_and_cancellation_never_acknowledge_pending() {
    let dav = MockDav::start();
    let c = cfg(&dav);
    let (mut s, _d) = fixture();
    add(&mut s, "pending");
    assert!(exchange_guarded(&c, &mut s, DEFAULT_EXCHANGE_TIMEOUT, || false).is_err());
    assert_eq!(s.pending.len(), 1);
    dav.keep_only_next_put.store(1, Ordering::SeqCst);
    assert!(run(&c, &mut s).is_err());
    assert_eq!(s.pending.len(), 1);
}
#[test]
fn encrypted_objects_authenticate_identity_name_and_kind() {
    let (envelope, key) = secure::create("collection", Some("secret")).unwrap();
    assert!(secure::open(&envelope, "other", Some("secret")).is_err());
    assert!(secure::open(&envelope, "collection", Some("wrong")).is_err());
    let key2 = secure::open(&envelope, "collection", Some("secret")).unwrap();
    let body = key.seal("manifest", "manifest", b"private task").unwrap();
    assert_eq!(key2.unseal("manifest", "manifest", &body).unwrap(), b"private task");
    assert!(key2.unseal("batch", "manifest", &body).is_err());
    assert!(key2.unseal("manifest", "different", &body).is_err());
    assert_ne!(body, key.seal("manifest", "manifest", b"private task").unwrap());
}
fn manifest(dav: &MockDav, c: &NextcloudCfg) -> Manifest {
    serde_json::from_slice(
        &dav.raw(&format!(
            "/remote.php/dav/files/u/{}/momentum-v3/manifest.json",
            c.folder
        ))
        .unwrap(),
    )
    .unwrap()
}
#[test]
fn delta_batches_conflict_retry_restart_and_legacy_cursors_are_separate() {
    let dav = MockDav::start();
    let c = cfg(&dav);
    let (mut a, _a) = fixture();
    add(&mut a, "initial");
    a.meta.last_etag = Some("legacy-tag".into());
    a.meta.last_sync_version = 123;
    run(&c, &mut a).unwrap();
    let checkpoint = manifest(&dav, &c).checkpoint;
    add(&mut a, "delta");
    let pending = a.pending.clone();
    dav.fail_next_put.store(true, Ordering::SeqCst);
    run(&c, &mut a).unwrap();
    let m = manifest(&dav, &c);
    assert_eq!(m.checkpoint, checkpoint);
    assert_eq!(m.batches.len(), 1);
    assert_eq!(a.meta.last_etag.as_deref(), Some("legacy-tag"));
    assert_eq!(a.meta.last_sync_version, 123);
    a.save().unwrap();
    let mut restarted = Store::load(a.dir().into());
    restarted.pending = pending;
    run(&c, &mut restarted).unwrap();
    assert_eq!(manifest(&dav, &c).batches.len(), 1);
    assert_eq!(restarted.state.task.entities.len(), 2);
    assert!(restarted.pending.is_empty());
}
#[test]
fn damaged_unknown_and_missing_objects_fail_without_acknowledgment() {
    for damage in ["unknown", "missing", "corrupt", "coverage"] {
        let dav = MockDav::start();
        let c = cfg(&dav);
        let (mut a, _a) = fixture();
        add(&mut a, "initial");
        run(&c, &mut a).unwrap();
        let mut m = manifest(&dav, &c);
        match damage {
            "unknown" => m.version = 99,
            "missing" => m.checkpoint.id = "0".repeat(64),
            "corrupt" => {
                dav.set_raw(
                    &format!("/remote.php/dav/files/u/legacy/momentum-v3/{}", m.checkpoint.id),
                    b"damaged".to_vec(),
                );
            }
            "coverage" => {
                m.batches.push(Object {
                    id: "0".repeat(64),
                    hash: "0".repeat(64),
                    kind: "archive".into(),
                });
            }
            _ => unreachable!(),
        }
        dav.set_raw(
            "/remote.php/dav/files/u/legacy/momentum-v3/manifest.json",
            serde_json::to_vec(&m).unwrap(),
        );
        add(&mut a, "pending");
        let before = serde_json::to_value(&a.state).unwrap();
        assert!(run(&c, &mut a).is_err(), "{damage}");
        assert_eq!(a.pending.len(), 1);
        assert_eq!(serde_json::to_value(&a.state).unwrap(), before);
    }
}
#[test]
fn checkpoint_archives_are_separate_and_delete_restore_reorder_survive() {
    let dav = MockDav::start();
    let c = cfg(&dav);
    let (mut a, _a) = fixture();
    add(&mut a, "first");
    add(&mut a, "second");
    a.state
        .rest
        .insert("archiveOld".into(), serde_json::json!({"opaque":{"value":9}}));
    run(&c, &mut a).unwrap();
    let initial = manifest(&dav, &c);
    let cp: Checkpoint = serde_json::from_slice(
        &dav.raw(&format!(
            "/remote.php/dav/files/u/legacy/momentum-v3/{}",
            initial.checkpoint.id
        ))
        .unwrap(),
    )
    .unwrap();
    assert!(!cp.state.rest.contains_key("archiveOld"));
    assert_eq!(cp.archives.len(), 1);
    let first = a
        .state
        .task
        .entities
        .values()
        .find(|t| t.title == "first")
        .unwrap()
        .clone();
    a.dispatch(Action::DeleteTask {
        task: first.clone(),
        sub_tasks: vec![],
    });
    run(&c, &mut a).unwrap();
    let (mut b, _b) = fixture();
    run(&c, &mut b).unwrap();
    assert!(!b.state.task.entities.contains_key(&first.id));
    b.dispatch(Action::RestoreTask {
        task: first.clone(),
        sub_tasks: vec![],
    });
    run(&c, &mut b).unwrap();
    run(&c, &mut a).unwrap();
    assert!(a.state.task.entities.contains_key(&first.id));
    assert_eq!(
        a.state.rest.get("archiveOld"),
        Some(&serde_json::json!({"opaque":{"value":9}}))
    );
    for i in 0..CHECKPOINT_INTERVAL {
        add(&mut a, &format!("cp-{i}"));
        run(&c, &mut a).unwrap();
    }
    let m = manifest(&dav, &c);
    assert_ne!(m.checkpoint, initial.checkpoint);
    let cp: Checkpoint = serde_json::from_slice(
        &dav.raw(&format!(
            "/remote.php/dav/files/u/legacy/momentum-v3/{}",
            m.checkpoint.id
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(cp.covered.len(), CHECKPOINT_INTERVAL);
    assert_eq!(
        cp.covered,
        m.batches
            .iter()
            .take(CHECKPOINT_INTERVAL)
            .map(|b| b.id.clone())
            .collect::<Vec<_>>()
    );
    run(&c, &mut b).unwrap();
    assert_eq!(
        serde_json::to_value(&a.state).unwrap(),
        serde_json::to_value(&b.state).unwrap()
    );
}
#[test]
fn encrypted_exchange_never_falls_back_to_legacy_on_wrong_key() {
    let dav = MockDav::start();
    let mut c = cfg(&dav);
    c.encrypt_key = Some("secret".into());
    let (mut a, _a) = fixture();
    add(&mut a, "private");
    run(&c, &mut a).unwrap();
    let (mut b, _b) = fixture();
    run(&c, &mut b).unwrap();
    assert_eq!(b.state.task.entities.len(), 1);
    c.encrypt_key = Some("wrong".into());
    add(&mut b, "pending");
    let puts = dav.puts.load(Ordering::SeqCst);
    assert!(matches!(run(&c, &mut b), Err(SyncError::WrongEncryptionPassword)));
    assert_eq!(dav.puts.load(Ordering::SeqCst), puts);
    assert_eq!(b.pending.len(), 1);
}
#[test]
fn session_amortizes_kdf_and_invalidates_on_credential_change() {
    let dav = MockDav::start();
    let mut c = cfg(&dav);
    c.encrypt_key = Some("secret".into());
    let (mut a, _a) = fixture();
    add(&mut a, "initial");
    let mut session = Session::default();
    let before = secure::kdf_count();
    session
        .exchange_guarded(&c, &mut a, DEFAULT_EXCHANGE_TIMEOUT, || true)
        .unwrap();
    add(&mut a, "warm");
    session
        .exchange_guarded(&c, &mut a, DEFAULT_EXCHANGE_TIMEOUT, || true)
        .unwrap();
    assert_eq!(secure::kdf_count() - before, 1);
    session.invalidate();
    session
        .exchange_guarded(&c, &mut a, DEFAULT_EXCHANGE_TIMEOUT, || true)
        .unwrap();
    assert_eq!(secure::kdf_count() - before, 2);
    c.encrypt_key = Some("wrong".into());
    assert!(session
        .exchange_guarded(&c, &mut a, DEFAULT_EXCHANGE_TIMEOUT, || true)
        .is_err());
    assert!(session.key.is_none());
}
#[test]
fn encrypted_folder_copy_rebinds_context_without_changing_source() {
    let dav = MockDav::start();
    let mut c = cfg(&dav);
    c.encrypt_key = Some("secret".into());
    let (mut a, _a) = fixture();
    add(&mut a, "initial");
    run(&c, &mut a).unwrap();
    add(&mut a, "delta");
    run(&c, &mut a).unwrap();
    let before = dav
        .raw("/remote.php/dav/files/u/legacy/momentum-v3/manifest.json")
        .unwrap();
    let settings = b"opaque portable settings payload".to_vec();
    dav.set_raw(
        &format!("/remote.php/dav/files/u/legacy/{SETTINGS_FILE}"),
        settings.clone(),
    );
    let report = copy_collection_guarded(&c, "new-folder", DEFAULT_EXCHANGE_TIMEOUT, || true).unwrap();
    assert_eq!(
        dav.raw(&format!("/remote.php/dav/files/u/new-folder/{SETTINGS_FILE}")),
        Some(settings)
    );
    assert!(report.copied_files >= 3);
    assert_eq!(
        dav.raw("/remote.php/dav/files/u/legacy/momentum-v3/manifest.json")
            .unwrap(),
        before
    );
    c.folder = "new-folder".into();
    let (mut b, _b) = fixture();
    run(&c, &mut b).unwrap();
    assert_eq!(
        serde_json::to_value(&a.state).unwrap(),
        serde_json::to_value(&b.state).unwrap()
    );
    assert!(copy_collection_guarded(&c, "legacy", DEFAULT_EXCHANGE_TIMEOUT, || true).is_err());
    assert_eq!(
        dav.raw("/remote.php/dav/files/u/legacy/momentum-v3/manifest.json")
            .unwrap(),
        before
    );
}
#[test]
fn warm_session_reuses_verified_immutable_checkpoint_and_transfers_only_delta() {
    let dav = MockDav::start();
    let c = cfg(&dav);
    let (mut a, _a) = fixture();
    a.state
        .rest
        .insert("opaqueLarge".into(), serde_json::json!("x".repeat(100_000)));
    add(&mut a, "initial");
    let mut session = Session::default();
    session
        .exchange_guarded(&c, &mut a, DEFAULT_EXCHANGE_TIMEOUT, || true)
        .unwrap();
    dav.downloaded_bytes.store(0, Ordering::SeqCst);
    add(&mut a, "delta");
    session
        .exchange_guarded(&c, &mut a, DEFAULT_EXCHANGE_TIMEOUT, || true)
        .unwrap();
    assert!(dav.downloaded_bytes.load(Ordering::SeqCst) < 20_000);
}
#[test]
fn cache_expires_and_wipes_without_another_exchange() {
    let (_, key) = secure::create("collection", Some("secret")).unwrap();
    let cached = CachedKey::new("fingerprint".into(), "envelope".into(), key, Duration::from_millis(10)).unwrap();
    std::thread::sleep(Duration::from_millis(100));
    assert!(cached.get("fingerprint", "envelope").is_none());
    assert!(cached.key.lock().unwrap().is_none());
}
#[test]
fn durable_pin_survives_failed_exchange_and_restore_and_corruption_fails_closed() {
    let dav = MockDav::start();
    let c = cfg(&dav);
    let (mut s, _d) = fixture();
    assert!(!require_optimized(&c, s.dir()).unwrap());
    pin_optimized(&c, s.dir()).unwrap();
    assert!(require_optimized(&c, s.dir()).unwrap());
    assert!(exchange_guarded(&c, &mut s, DEFAULT_EXCHANGE_TIMEOUT, || false).is_err());
    s.replace_state(AppData::fresh()).unwrap();
    assert!(require_optimized(&c, s.dir()).unwrap());
    let mut other = c.clone();
    other.folder = "other".into();
    assert!(!require_optimized(&other, s.dir()).unwrap());
    std::fs::write(
        s.dir().join(".nextcloud-protocol").join(collection_identity(&c)),
        b"truncated",
    )
    .unwrap();
    assert!(require_optimized(&c, s.dir()).is_err());
    assert!(pin_optimized(&c, s.dir()).is_err());
}
#[test]
fn collection_identity_canonicalizes_wire_origin_but_preserves_account_case() {
    let c = NextcloudCfg {
        server_url: "https://EXAMPLE.com:443/nextcloud/".into(),
        user_name: " User ".into(),
        folder: "/folder/".into(),
        ..Default::default()
    };
    let mut same = c.clone();
    same.server_url = "https://example.com/nextcloud".into();
    same.user_name = "User".into();
    same.folder = "folder".into();
    assert_eq!(collection_identity(&c), collection_identity(&same));
    same.user_name = "user".into();
    assert_ne!(collection_identity(&c), collection_identity(&same));
}
#[test]
fn concurrent_initializers_and_writers_converge_without_legacy_replacement() {
    let dav = MockDav::start();
    let c = cfg(&dav);
    let (mut a, _a) = fixture();
    let (mut b, _b) = fixture();
    add(&mut a, "a");
    add(&mut b, "b");
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let (mut a, mut b) = std::thread::scope(|scope| {
        let ca = c.clone();
        let gate = barrier.clone();
        let left = scope.spawn(move || {
            gate.wait();
            for _ in 0..4 {
                if run(&ca, &mut a).is_ok() {
                    return a;
                }
            }
            panic!("initializer a did not converge")
        });
        let gate = barrier.clone();
        let cb = c.clone();
        let right = scope.spawn(move || {
            gate.wait();
            for _ in 0..4 {
                if run(&cb, &mut b).is_ok() {
                    return b;
                }
            }
            panic!("initializer b did not converge")
        });
        (left.join().unwrap(), right.join().unwrap())
    });
    run(&c, &mut a).unwrap();
    run(&c, &mut b).unwrap();
    assert_eq!(a.state.task.entities.len(), 2);
    assert_eq!(
        serde_json::to_value(&a.state).unwrap(),
        serde_json::to_value(&b.state).unwrap()
    );
    assert!(dav.raw("/remote.php/dav/files/u/legacy/sync-data.json").is_none());
}
#[test]
fn acknowledged_optimized_manifest_disappearance_never_reboots_from_stale_legacy() {
    let dav = MockDav::start();
    let c = cfg(&dav);
    let (mut a, _a) = fixture();
    add(&mut a, "legacy");
    super::super::exchange(&c, &mut a).unwrap();
    run(&c, &mut a).unwrap();
    add(&mut a, "v3-only");
    run(&c, &mut a).unwrap();
    a.save().unwrap();
    dav.remove("/remote.php/dav/files/u/legacy/momentum-v3/manifest.json");
    let mut restarted = Store::load(a.dir().into());
    add(&mut restarted, "pending");
    let puts = dav.puts.load(Ordering::SeqCst);
    assert!(run(&c, &mut restarted).is_err());
    assert_eq!(restarted.pending.len(), 1);
    assert_eq!(restarted.state.task.entities.len(), 3);
    assert_eq!(dav.puts.load(Ordering::SeqCst), puts);
}
#[test]
fn unchanged_manifest_preserves_restored_state_across_restart() {
    let dav = MockDav::start();
    let c = cfg(&dav);
    let (mut s, _dir) = fixture();
    add(&mut s, "server task");
    run(&c, &mut s).unwrap();
    s.save().unwrap();
    let mut restored = AppData::fresh();
    restored
        .rest
        .insert("globalConfig".into(), serde_json::json!({"opaque":42}));
    s.replace_state(restored.clone()).unwrap();
    let mut s = Store::load(s.dir().into());
    let report = run(&c, &mut s).unwrap();
    assert!(!report.downloaded && !report.uploaded);
    assert_eq!(
        serde_json::to_value(&s.state).unwrap(),
        serde_json::to_value(restored).unwrap()
    );
    add(&mut s, "pending after restore");
    run(&c, &mut s).unwrap();
    assert_eq!(
        s.state.task.entities.len(),
        2,
        "pending edits retain the established remote rebase policy"
    );
}
#[test]
fn verified_folder_copy_witness_prevents_rebootstrap_after_manifest_loss() {
    let dav = MockDav::start();
    let c = cfg(&dav);
    let (mut s, _dir) = fixture();
    add(&mut s, "source");
    run(&c, &mut s).unwrap();
    copy_collection_guarded(&c, "copied", DEFAULT_EXCHANGE_TIMEOUT, || true).unwrap();
    let mut target = c.clone();
    target.folder = "copied".into();
    mark_initialized(&target, s.dir()).unwrap();
    dav.remove("/remote.php/dav/files/u/copied/momentum-v3/manifest.json");
    let puts = dav.puts.load(Ordering::SeqCst);
    assert!(run(&target, &mut s).is_err());
    assert_eq!(dav.puts.load(Ordering::SeqCst), puts);
    assert_eq!(s.state.task.entities.len(), 1);
}
