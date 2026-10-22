// SPDX-License-Identifier: GPL-3.0-or-later
use crate::*;
use serde_json::{json, Value};
fn engine() -> (std::sync::Arc<Engine>, tempfile::TempDir) {
    let d = tempfile::tempdir().unwrap();
    (
        Engine::open_checked(d.path().to_string_lossy().into_owned()).unwrap(),
        d,
    )
}
fn values(e: &Engine) -> Value {
    serde_json::from_str::<Value>(&e.portable_settings_json().unwrap()).unwrap()["values"].clone()
}
#[test]
fn checked_startup_preserves_damaged_or_missing_task_snapshots_instead_of_bootstrapping_them() {
    for damage in ["malformed", "missing-task-collection", "missing", "unreadable"] {
        let (e, dir) = engine();
        assert!(e.add_task("Local work to preserve".into(), View::Today).changed);
        drop(e);
        let path = dir.path().join("state.json");
        match damage {
            "malformed" => std::fs::write(&path, b"{broken local snapshot").unwrap(),
            "missing-task-collection" => std::fs::write(&path, b"{}").unwrap(),
            "missing" => std::fs::remove_file(&path).unwrap(),
            _ => {
                std::fs::remove_file(&path).unwrap();
                std::fs::create_dir(&path).unwrap();
            }
        }
        let pending = std::fs::read(dir.path().join("pending.json")).unwrap();
        let meta = std::fs::read(dir.path().join("meta.json")).unwrap();
        assert!(
            Engine::open_checked(dir.path().to_string_lossy().into_owned()).is_err(),
            "{damage}"
        );
        assert_eq!(std::fs::read(dir.path().join("pending.json")).unwrap(), pending);
        assert_eq!(std::fs::read(dir.path().join("meta.json")).unwrap(), meta);
        if damage == "malformed" {
            assert_eq!(std::fs::read(&path).unwrap(), b"{broken local snapshot");
        }
        if damage == "missing-task-collection" {
            assert_eq!(std::fs::read(&path).unwrap(), b"{}");
        }
        if damage == "missing" {
            assert!(!path.exists());
        }
        if damage == "unreadable" {
            assert!(path.is_dir());
        }
    }
}
#[test]
fn portable_values_are_validated_durable_and_never_task_state() {
    let (e, d) = engine();
    e.stage_portable_settings(json!({"version":1,"values":{"grouping":"project","upcomingDays":30,"password":"private","settingsSyncEnabled":true}}).to_string()).unwrap();
    assert_eq!(values(&e), json!({"grouping":"project","upcomingDays":30}));
    assert!(!e.settings_sync_status().enabled);
    let reopened = Engine::open_checked(d.path().to_string_lossy().into_owned()).unwrap();
    assert_eq!(values(&e), values(&reopened));
    let backup = d.path().join("backup.json");
    e.export_backup(backup.to_string_lossy().into_owned()).unwrap();
    let body = std::fs::read_to_string(backup).unwrap();
    assert!(!body.contains("momentumSettings") && !body.contains("upcomingDays"));
}
#[test]
fn a_settings_only_store_keeps_one_causal_actor_across_reopens() {
    let (e, dir) = engine();
    let actor = e.client_id();
    e.stage_portable_settings(json!({"version":1,"values":{"sort":"title"}}).to_string())
        .unwrap();
    assert!(!dir.path().join("state.json").exists() && !dir.path().join("pending.json").exists());
    drop(e);
    let reopened = Engine::open_checked(dir.path().to_string_lossy().into_owned()).unwrap();
    assert_eq!(
        reopened.client_id(),
        actor,
        "a settings-only store regenerated its causal identity"
    );
    reopened
        .stage_portable_settings(json!({"version":1,"values":{"sort":"created"}}).to_string())
        .unwrap();
    let cell = reopened.lock().portable.payload.cells["sort"].clone();
    assert_eq!(cell.actor, actor);
    assert_eq!(cell.clock.len(), 1);
    assert_eq!(cell.clock[&actor], 2);
    assert!(cell.conflicts.is_empty());
}
#[test]
fn device_summary_updates_preserve_every_portable_preference_and_planner_semantics() {
    let (e, _dir) = engine();
    let document = json!({"version":1,"values":{"grouping":"project","sort":"due","direction":"descending","upcomingDays":30,"autoArchive":true,"rowDensity":"compact","colorfulLabels":false,"completionSoundEnabled":true,"completionSoundChoice":"warm-spark"}}).to_string();
    e.stage_portable_settings(document).unwrap();
    let portable = e.portable_settings_json().unwrap();
    let status = e.settings_sync_status();
    let time = ClockTime { hour: 9, minute: 30 };
    e.set_device_summary_preferences(true, time).unwrap();
    let prefs = e.preferences();
    assert!(prefs.morning_summary_enabled);
    assert_eq!(prefs.morning_summary_time, time);
    assert_eq!(prefs.group_by, GroupBy::Project);
    assert_eq!(prefs.sort, SortKey::Due);
    assert_eq!(prefs.direction, SortDirection::Descending);
    assert_eq!(prefs.upcoming_days, 30);
    assert!(prefs.auto_archive);
    assert_eq!(e.portable_settings_json().unwrap(), portable);
    assert_eq!(e.settings_sync_status(), status);
    for invalid in [ClockTime { hour: 24, minute: 0 }, ClockTime { hour: 23, minute: 60 }] {
        assert!(e.set_device_summary_preferences(false, invalid).is_err());
        assert!(e.preferences().morning_summary_enabled);
        assert_eq!(e.preferences().morning_summary_time, time);
    }
    e.add_task("Today fixture".into(), View::Today);
    let day = momentum_model::today_str();
    let now = momentum_model::local_ms(&day, 8, 0).unwrap();
    let at = momentum_model::local_ms(&day, 9, 30).unwrap();
    let summary = e
        .notification_plan(now)
        .unwrap()
        .requests
        .into_iter()
        .find(|r| matches!(r.content, NotificationContent::Summary { .. }))
        .unwrap();
    assert_eq!(summary.fire_at_ms, at);
    e.set_device_summary_preferences(false, time).unwrap();
    assert!(!e
        .notification_plan(now)
        .unwrap()
        .requests
        .iter()
        .any(|r| matches!(r.content, NotificationContent::Summary { .. })));
    assert_eq!(e.portable_settings_json().unwrap(), portable);
}
#[test]
fn failed_first_private_save_cannot_publish_an_identity_or_settings_image() {
    let (e, dir) = engine();
    let actor = e.client_id();
    let blocked = dir.path().join("portable-settings.json.tmp");
    std::fs::create_dir(&blocked).unwrap();
    assert!(e
        .stage_portable_settings(json!({"version":1,"values":{"sort":"title"}}).to_string())
        .is_err());
    assert!(e.set_settings_sync_enabled(true).is_err());
    assert!(!e.settings_sync_status().enabled && e.settings_sync_status().pending_keys == 0);
    assert_eq!(values(&e), json!({}));
    assert!(!dir.path().join("portable-settings.json").exists());
    assert!(!dir.path().join("meta.json").exists());
    std::fs::remove_dir(blocked).unwrap();
    e.stage_portable_settings(json!({"version":1,"values":{"sort":"title"}}).to_string())
        .unwrap();
    assert_eq!(
        Engine::open_checked(dir.path().to_string_lossy().into_owned())
            .unwrap()
            .client_id(),
        actor
    );
}
#[test]
fn invalid_document_or_failed_write_leaves_all_values_unchanged() {
    let (e, d) = engine();
    e.stage_portable_settings(json!({"version":1,"values":{"sort":"title"}}).to_string())
        .unwrap();
    for doc in [
        json!({"version":2,"values":{}}),
        json!({"values":{}}),
        json!({"version":1,"values":{"sort":"created","upcomingDays":"30"}}),
        json!({"version":1,"values":{"sort":"bogus"}}),
    ] {
        assert!(e.stage_portable_settings(doc.to_string()).is_err());
        assert_eq!(values(&e), json!({"sort":"title"}));
    }
    std::fs::create_dir(d.path().join("portable-settings.json.tmp")).unwrap();
    assert!(e
        .stage_portable_settings(json!({"version":1,"values":{"sort":"created"}}).to_string())
        .is_err());
    assert_eq!(values(&e), json!({"sort":"title"}));
    assert_eq!(
        values(&Engine::open_checked(d.path().to_string_lossy().into_owned()).unwrap()),
        values(&e)
    );
}
#[test]
fn settings_only_transfer_and_independent_backup_controls_preserve_tasks_and_consent() {
    let (a, ad) = engine();
    let (b, bd) = engine();
    a.stage_portable_settings(json!({"version":1,"values":{"sort":"title","autoArchive":true}}).to_string())
        .unwrap();
    b.stage_portable_settings(json!({"version":1,"values":{"sort":"manual","rowDensity":"compact"}}).to_string())
        .unwrap();
    b.add_task(
        "Keep me".into(),
        View::Project {
            id: momentum_model::INBOX_PROJECT_ID.into(),
        },
    );
    let tasks = b.with_store(|s| s.state.task.ids.clone());
    let settings = ad.path().join("settings.json").to_string_lossy().into_owned();
    a.export_settings(settings.clone()).unwrap();
    b.import_settings(settings).unwrap();
    assert_eq!(b.with_store(|s| s.state.task.ids.clone()), tasks);
    assert_eq!(
        values(&b),
        json!({"sort":"title","autoArchive":true,"rowDensity":"compact"})
    );
    assert!(!b.settings_sync_status().enabled);
    let backup = bd.path().join("backup.json").to_string_lossy().into_owned();
    a.export_backup_with_settings(backup.clone(), true).unwrap();
    b.stage_portable_settings(json!({"version":1,"values":{"sort":"created"}}).to_string())
        .unwrap();
    b.import_backup_with_settings(backup.clone(), false).unwrap();
    assert_eq!(values(&b)["sort"], "created");
    b.import_backup_with_settings(backup, true).unwrap();
    assert_eq!(values(&b)["sort"], "title");
}

#[test]
fn causal_merge_preserves_independent_edits_and_converges_conflicts() {
    use crate::portable::{Document, PortableState};
    let mut a = PortableState::default();
    let mut b = PortableState::default();
    a.stage(
        Document::parse(r#"{"version":1,"values":{"sort":"title","colorfulLabels":true}}"#).unwrap(),
        "a",
    );
    b.stage(
        Document::parse(r#"{"version":1,"values":{"sort":"due","rowDensity":"compact"}}"#).unwrap(),
        "b",
    );
    let left = a.payload.clone();
    let right = b.payload.clone();
    a.payload.merge(&right);
    b.payload.merge(&left);
    assert_eq!(a.payload, b.payload);
    assert_eq!(a.payload.document().values["sort"], "due");
    assert_eq!(a.payload.cells.len(), 3);
    a.stage(
        Document::parse(r#"{"version":1,"values":{"sort":"manual"}}"#).unwrap(),
        "a",
    );
    b.payload.merge(&a.payload);
    assert_eq!(b.payload.document().values["sort"], "manual");
}
#[test]
fn invalid_included_settings_never_replace_tasks_or_opaque_legacy_config() {
    let (e, d) = engine();
    e.add_task("Preserve".into(), View::Today);
    e.with_store_mut(|s| {
        s.state
            .rest
            .insert("globalConfig".into(), json!({"opaque":{"privateLegacy":true}}));
    });
    let before = e.with_store(|s| serde_json::to_value(&s.state).unwrap());
    let path = d.path().join("invalid.json");
    std::fs::write(
        &path,
        json!({"data":momentum_model::AppData::fresh(),"momentumSettings":{"version":1,"values":{"autoArchive":"true"}}})
            .to_string(),
    )
    .unwrap();
    assert!(e
        .import_backup_with_settings(path.to_string_lossy().into_owned(), true)
        .is_err());
    assert_eq!(before, e.with_store(|s| serde_json::to_value(&s.state).unwrap()));
    let path = d.path().join("valid.json");
    e.export_backup_with_settings(path.to_string_lossy().into_owned(), true)
        .unwrap();
    e.import_backup_with_settings(path.to_string_lossy().into_owned(), true)
        .unwrap();
    assert_eq!(before, e.with_store(|s| serde_json::to_value(&s.state).unwrap()));
}
#[allow(dead_code)]
#[path = "../../momentum-sync/src/mock_dav.rs"]
mod mock_dav;
fn nextcloud(d: &mock_dav::MockDav) -> NextcloudSettings {
    NextcloudSettings {
        server_url: d.url.clone(),
        user_name: "u".into(),
        password: "p".into(),
        folder: "legacy".into(),
        compress: false,
        encryption_password: None,
    }
}
#[test]
fn native_empty_task_and_backup_stores_can_seed_the_compatible_nextcloud_envelope() {
    for mode in ["empty", "task", "native-backup"] {
        let dav = mock_dav::MockDav::start();
        let (e, dir) = engine();
        if mode != "empty" {
            e.add_task("Native task".into(), View::Today);
        }
        let backup = dir.path().join("native.json").to_string_lossy().into_owned();
        e.export_backup(backup.clone()).unwrap();
        let before: Value = serde_json::from_slice(&std::fs::read(&backup).unwrap()).unwrap();
        assert!(before["data"].get("globalConfig").is_none());
        if mode == "native-backup" {
            e.import_backup(backup).unwrap();
        }
        let task_ids: Vec<_> = e.all_tasks().into_iter().map(|t| t.id).collect();
        let mut cfg = nextcloud(&dav);
        cfg.folder = "momentum".into();
        let report = e.sync_nextcloud(cfg).unwrap();
        assert!(report.uploaded && !report.downloaded, "{mode}");
        let remote = dav.raw("/remote.php/dav/files/u/momentum/sync-data.json").unwrap();
        let decoded = momentum_sync::decode(std::str::from_utf8(&remote).unwrap(), None).unwrap();
        assert_eq!(decoded.state.rest["globalConfig"], json!({}));
        assert_eq!(decoded.state.task.ids, task_ids);
        let reopened = Engine::open_checked(dir.path().to_string_lossy().into_owned()).unwrap();
        assert_eq!(reopened.with_store(|s| s.state.rest["globalConfig"].clone()), json!({}));
        assert_eq!(reopened.all_tasks().len(), task_ids.len());
        assert_eq!(reopened.pending_count(), 0);
    }
}
#[test]
fn native_bootstrap_preserves_opaque_configuration_on_upload_and_download() {
    let dav = mock_dav::MockDav::start();
    let (source, dir) = engine();
    source.add_task("Opaque fixture".into(), View::Today);
    let path = dir.path().join("opaque.json");
    source.export_backup(path.to_string_lossy().into_owned()).unwrap();
    let mut backup: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let opaque = json!({"legacyUnknown":{"nested":[true,42,"fixture"]}});
    backup["data"]["globalConfig"] = opaque.clone();
    std::fs::write(&path, backup.to_string()).unwrap();
    source.import_backup(path.to_string_lossy().into_owned()).unwrap();
    source.sync_nextcloud(nextcloud(&dav)).unwrap();
    assert_eq!(source.with_store(|s| s.state.rest["globalConfig"].clone()), opaque);
    let (destination, _dd) = engine();
    destination.sync_nextcloud(nextcloud(&dav)).unwrap();
    assert_eq!(destination.with_store(|s| s.state.rest["globalConfig"].clone()), opaque);
}
#[test]
fn failed_native_bootstrap_preserves_tasks_pending_and_absent_compatibility_config() {
    use std::sync::atomic::Ordering;
    for failure in [
        "damaged",
        "encrypted",
        "unsupported",
        "conflict",
        "short-upload",
        "cancelled",
        "persistence",
    ] {
        let dav = mock_dav::MockDav::start();
        let (e, dir) = engine();
        e.add_task("Keep local task".into(), View::Today);
        let before = e.with_store(|s| json!([s.state, s.pending, s.meta]));
        let path = "/remote.php/dav/files/u/legacy/sync-data.json";
        let expected = match failure {
            "damaged" => {
                dav.set_raw(path, b"damaged".to_vec());
                Some(SyncFailureKind::RemoteFileDamaged)
            }
            "encrypted" => {
                dav.set_raw(path, b"pf_E2__encrypted".to_vec());
                Some(SyncFailureKind::EncryptionPasswordMissing)
            }
            "unsupported" => {
                dav.set_raw(path, b"pf_3__{\"version\":3}".to_vec());
                Some(SyncFailureKind::Incompatible)
            }
            "conflict" => {
                dav.conflict_puts.store(3, Ordering::SeqCst);
                Some(SyncFailureKind::Conflict)
            }
            "short-upload" => {
                dav.keep_only_next_put.store(1, Ordering::SeqCst);
                Some(SyncFailureKind::UploadIncomplete)
            }
            "cancelled" => Some(SyncFailureKind::Cancelled),
            _ => {
                std::fs::create_dir(dir.path().join("state.json.tmp")).unwrap();
                None
            }
        };
        let cancellation = SyncCancellation::new();
        if failure == "cancelled" {
            assert!(cancellation.cancel());
        }
        let error = e.sync_nextcloud_cancellable(nextcloud(&dav), cancellation).unwrap_err();
        if let Some(expected) = expected {
            let kind = match error {
                CoreError::Actionable { kind, .. } | CoreError::Transient { kind, .. } => kind,
                other => panic!("{failure}: {other:?}"),
            };
            assert_eq!(kind, expected, "{failure}");
        } else {
            assert!(matches!(error, CoreError::Io { .. }));
        }
        assert_eq!(
            e.with_store(|s| json!([s.state, s.pending, s.meta])),
            before,
            "{failure}"
        );
        let reopened = Engine::open_checked(dir.path().to_string_lossy().into_owned()).unwrap();
        assert_eq!(
            reopened.with_store(|s| json!([s.state, s.pending, s.meta])),
            before,
            "{failure}"
        );
        if failure == "conflict" {
            assert_eq!(dav.puts.load(Ordering::SeqCst), 3);
            assert_eq!(dav.file_count(), 0);
        }
        if failure == "cancelled" {
            assert_eq!(dav.gets.load(Ordering::SeqCst), 0);
        }
    }
}
#[test]
fn nextcloud_two_clients_settings_opt_in_bootstrap_converge_and_reenable() {
    let d = mock_dav::MockDav::start();
    let cfg = nextcloud(&d);
    let (a, _ad) = engine();
    let (b, _bd) = engine();
    a.stage_portable_settings(json!({"version":1,"values":{"sort":"title"}}).to_string())
        .unwrap();
    a.with_store_mut(|s| {
        s.state.rest.insert("globalConfig".into(), json!({}));
    });
    a.sync_nextcloud(cfg.clone()).unwrap();
    assert!(d.raw("/remote.php/dav/files/u/legacy/momentum-settings.json").is_none());
    a.set_settings_sync_enabled(true).unwrap();
    a.sync_nextcloud(cfg.clone()).unwrap();
    b.stage_portable_settings(json!({"version":1,"values":{"sort":"due","rowDensity":"compact"}}).to_string())
        .unwrap();
    b.set_settings_sync_enabled(true).unwrap();
    b.sync_nextcloud(cfg.clone()).unwrap();
    assert!(b.settings_sync_status().bootstrap_required);
    assert_eq!(values(&b)["sort"], "due");
    b.set_settings_bootstrap_choice(
        crate::SettingsBootstrapChoice::UseRemote,
        b.settings_sync_status().review_id.unwrap(),
    )
    .unwrap();
    b.sync_nextcloud(cfg.clone()).unwrap();
    assert_eq!(values(&b)["sort"], "title");
    assert_eq!(values(&b)["rowDensity"], "compact");
    a.sync_nextcloud(cfg.clone()).unwrap();
    a.stage_portable_settings(json!({"version":1,"values":{"colorfulLabels":false,"sort":"manual"}}).to_string())
        .unwrap();
    b.stage_portable_settings(
        json!({"version":1,"values":{"completionSoundEnabled":true,"sort":"created"}}).to_string(),
    )
    .unwrap();
    a.sync_nextcloud(cfg.clone()).unwrap();
    b.sync_nextcloud(cfg.clone()).unwrap();
    a.sync_nextcloud(cfg.clone()).unwrap();
    assert_eq!(values(&a), values(&b));
    b.set_settings_sync_enabled(false).unwrap();
    a.stage_portable_settings(json!({"version":1,"values":{"rowDensity":"regular"}}).to_string())
        .unwrap();
    a.sync_nextcloud(cfg.clone()).unwrap();
    b.sync_nextcloud(cfg.clone()).unwrap();
    assert_eq!(values(&b)["rowDensity"], "compact");
    b.set_settings_sync_enabled(true).unwrap();
    b.sync_nextcloud(cfg).unwrap();
    assert_eq!(values(&b)["rowDensity"], "regular");
    assert_eq!(b.settings_sync_status().pending_keys, 0);
}
#[test]
fn causal_merge_is_order_independent_when_a_newer_value_has_not_seen_a_losing_edit() {
    use crate::portable::{Document, PortableState};
    let doc = |value: &str| Document::parse(&json!({"version":1,"values":{"sort":value}}).to_string()).unwrap();
    let mut a = PortableState::default();
    a.stage(doc("manual"), "a");
    let mut z = PortableState::default();
    z.stage(doc("due"), "z");
    let mut m = PortableState::default();
    m.payload.merge(&z.payload);
    m.stage(doc("title"), "m");
    let mut left = a.payload.clone();
    left.merge(&z.payload);
    left.merge(&m.payload);
    let mut right = a.payload.clone();
    right.merge(&m.payload);
    right.merge(&z.payload);
    assert_eq!(left, right);
    assert_eq!(left.document().values["sort"], "title");
}
#[test]
fn included_restore_updates_revision_and_rolls_back_both_sections_on_write_failure() {
    let (a, ad) = engine();
    let (b, bd) = engine();
    a.stage_portable_settings(json!({"version":1,"values":{"sort":"title"}}).to_string())
        .unwrap();
    let backup = ad.path().join("with-settings.json").to_string_lossy().into_owned();
    a.export_backup_with_settings(backup.clone(), true).unwrap();
    b.stage_portable_settings(json!({"version":1,"values":{"sort":"created"}}).to_string())
        .unwrap();
    b.add_task("keep".into(), View::Today);
    let before = b.with_store(|s| serde_json::json!([s.state, s.pending, s.meta]));
    let revision = b.settings_sync_status().revision;
    std::fs::create_dir(bd.path().join("pending.json.tmp")).unwrap();
    assert!(b.import_backup_with_settings(backup.clone(), true).is_err());
    assert_eq!(
        before,
        b.with_store(|s| serde_json::json!([s.state, s.pending, s.meta]))
    );
    assert_eq!(values(&b)["sort"], "created");
    std::fs::remove_dir(bd.path().join("pending.json.tmp")).unwrap();
    b.import_backup_with_settings(backup, true).unwrap();
    assert!(b.settings_sync_status().revision > revision);
}
#[test]
fn settings_failure_keeps_pending_and_task_edits_with_separate_recovery_status() {
    let d = mock_dav::MockDav::start();
    let cfg = nextcloud(&d);
    let (e, _ed) = engine();
    e.with_store_mut(|s| {
        s.state.rest.insert("globalConfig".into(), json!({}));
    });
    e.sync_nextcloud(cfg.clone()).unwrap();
    e.stage_portable_settings(json!({"version":1,"values":{"sort":"title"}}).to_string())
        .unwrap();
    e.set_settings_sync_enabled(true).unwrap();
    d.set_raw(
        "/remote.php/dav/files/u/legacy/momentum-settings.json",
        b"ms_1__{\"version\":9,\"cells\":{}}".to_vec(),
    );
    e.add_task("offline survives".into(), View::Today);
    e.sync_nextcloud(cfg).unwrap();
    assert_eq!(e.all_tasks().len(), 1);
    assert!(e.settings_sync_status().error.is_some());
    assert_eq!(e.settings_sync_status().pending_keys, 1);
    assert_eq!(values(&e)["sort"], "title");
}
#[test]
fn core_folder_copy_preserves_pending_local_edits_and_reports_exact_destination_files() {
    let d = mock_dav::MockDav::start();
    let (e, _ed) = engine();
    e.add_task("pending offline".into(), View::Today);
    let before = e.with_store(|s| serde_json::json!([s.state, s.pending, s.meta]));
    d.set_raw(
        "/remote.php/dav/files/u/legacy/sync-data.json",
        b"pf_2__original".to_vec(),
    );
    let report = e
        .migrate_nextcloud_folder(nextcloud(&d), "momentum".into(), SyncCancellation::new(), 10000)
        .unwrap();
    assert!(report.verified);
    assert_eq!(report.target_files, vec!["sync-data.json"]);
    assert_eq!(
        before,
        e.with_store(|s| serde_json::json!([s.state, s.pending, s.meta]))
    );
}
#[test]
fn a_settings_commit_failure_retains_pending_and_visible_error_until_recovery() {
    let d = mock_dav::MockDav::start();
    let cfg = nextcloud(&d);
    let (e, ed) = engine();
    e.with_store_mut(|s| {
        s.state.rest.insert("globalConfig".into(), json!({}));
    });
    e.sync_nextcloud(cfg.clone()).unwrap();
    e.stage_portable_settings(json!({"version":1,"values":{"sort":"title"}}).to_string())
        .unwrap();
    e.set_settings_sync_enabled(true).unwrap();
    std::fs::create_dir(ed.path().join("portable-settings.json.tmp")).unwrap();
    e.sync_nextcloud(cfg.clone()).unwrap();
    assert!(e.settings_sync_status().error.is_some());
    assert_eq!(e.settings_sync_status().pending_keys, 1);
    std::fs::remove_dir(ed.path().join("portable-settings.json.tmp")).unwrap();
    e.sync_nextcloud(cfg).unwrap();
    assert_eq!(e.settings_sync_status().pending_keys, 0);
    assert!(e.settings_sync_status().error.is_none());
}
#[test]
fn settings_exchange_keeps_edits_made_while_uploading_and_cancellation_keeps_pending() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let d = mock_dav::MockDav::start();
    let (e, _ed) = engine();
    e.stage_portable_settings(json!({"version":1,"values":{"sort":"title"}}).to_string())
        .unwrap();
    e.set_settings_sync_enabled(true).unwrap();
    let cfg = momentum_sync::NextcloudCfg {
        server_url: d.url.clone(),
        user_name: "u".into(),
        password: "p".into(),
        folder: "legacy".into(),
        ..Default::default()
    };
    let edited = AtomicBool::new(false);
    e.exchange_nextcloud_settings(&cfg, std::time::Duration::from_secs(10), &|| {
        if d.puts.load(Ordering::SeqCst) > 0 && !edited.swap(true, Ordering::SeqCst) {
            e.stage_portable_settings(json!({"version":1,"values":{"sort":"due"}}).to_string())
                .unwrap();
            e.add_task("during upload".into(), View::Today);
        }
        true
    })
    .unwrap();
    assert_eq!(values(&e)["sort"], "due");
    assert_eq!(e.settings_sync_status().pending_keys, 1);
    assert_eq!(e.sync_status().pending_ops, 1);
    e.exchange_nextcloud_settings(&cfg, std::time::Duration::from_secs(10), &|| true)
        .unwrap();
    assert_eq!(e.settings_sync_status().pending_keys, 0);
    assert_eq!(e.sync_status().pending_ops, 1);
    e.stage_portable_settings(json!({"version":1,"values":{"sort":"created"}}).to_string())
        .unwrap();
    let puts = d.puts.load(Ordering::SeqCst);
    assert!(e
        .exchange_nextcloud_settings(&cfg, std::time::Duration::from_secs(10), &|| d
            .puts
            .load(Ordering::SeqCst)
            == puts)
        .is_err());
    assert_eq!(e.settings_sync_status().pending_keys, 1);
    assert_eq!(e.sync_status().pending_ops, 1);
}
#[test]
fn settings_can_seed_independently_when_task_transport_cannot_download_its_snapshot() {
    let d = mock_dav::MockDav::start();
    let (e, _ed) = engine();
    e.stage_portable_settings(json!({"version":1,"values":{"sort":"title"}}).to_string())
        .unwrap();
    e.set_settings_sync_enabled(true).unwrap();
    // Exhaust the task GET's bounded reads; the separate settings exchange can
    // then proceed. Native task bootstrapping itself is now supported.
    d.fail_gets.store(3, std::sync::atomic::Ordering::SeqCst);
    assert!(e.sync_nextcloud(nextcloud(&d)).is_err());
    assert!(d.raw("/remote.php/dav/files/u/legacy/momentum-settings.json").is_some());
    assert_eq!(e.settings_sync_status().pending_keys, 0);
    assert!(e.settings_sync_status().last_success_ms > 0);
}
#[test]
fn bootstrap_choice_does_not_follow_a_nextcloud_provider_switch() {
    let (a, ad) = engine();
    let (b, _bd) = engine();
    let (local, _ld) = engine();
    let da = mock_dav::MockDav::start();
    let db = mock_dav::MockDav::start();
    for (e, sort, d) in [(&a, "title", &da), (&b, "created", &db)] {
        e.with_store_mut(|s| {
            s.state.rest.insert("globalConfig".into(), json!({}));
        });
        e.stage_portable_settings(json!({"version":1,"values":{"sort":sort}}).to_string())
            .unwrap();
        e.set_settings_sync_enabled(true).unwrap();
        e.sync_nextcloud(nextcloud(d)).unwrap();
    }
    local
        .stage_portable_settings(json!({"version":1,"values":{"sort":"manual"}}).to_string())
        .unwrap();
    local.set_settings_sync_enabled(true).unwrap();
    local.sync_nextcloud(nextcloud(&da)).unwrap();
    local
        .set_settings_bootstrap_choice(
            crate::SettingsBootstrapChoice::UseRemote,
            local.settings_sync_status().review_id.unwrap(),
        )
        .unwrap();
    local.sync_nextcloud(nextcloud(&db)).unwrap();
    assert!(local.settings_sync_status().bootstrap_required);
    assert_eq!(values(&local)["sort"], "manual");
    drop(ad);
}
#[cfg(unix)]
#[test]
fn settings_edits_and_idle_nextcloud_status_keep_task_snapshots_untouched() {
    use std::os::unix::fs::MetadataExt;
    let d = mock_dav::MockDav::start();
    let (e, dir) = engine();
    e.with_store_mut(|s| {
        s.state.rest.insert("globalConfig".into(), json!({}));
    });
    e.stage_portable_settings(json!({"version":1,"values":{"sort":"title"}}).to_string())
        .unwrap();
    e.set_settings_sync_enabled(true).unwrap();
    e.sync_nextcloud(nextcloud(&d)).unwrap();
    let files = ["state.json", "pending.json"];
    let before = files.map(|name| {
        (
            std::fs::metadata(dir.path().join(name)).unwrap().ino(),
            std::fs::read(dir.path().join(name)).unwrap(),
        )
    });
    std::thread::sleep(std::time::Duration::from_millis(2));
    let report = e.sync_nextcloud(nextcloud(&d)).unwrap();
    assert!(!report.downloaded && !report.uploaded);
    assert_eq!(
        before,
        files.map(|name| (
            std::fs::metadata(dir.path().join(name)).unwrap().ino(),
            std::fs::read(dir.path().join(name)).unwrap()
        )),
        "idle settings status rewrote unchanged task snapshots"
    );
    std::fs::create_dir(dir.path().join("state.json.tmp")).unwrap();
    e.stage_portable_settings(json!({"version":1,"values":{"rowDensity":"compact"}}).to_string())
        .unwrap();
    assert_eq!(
        before,
        files.map(|name| (
            std::fs::metadata(dir.path().join(name)).unwrap().ino(),
            std::fs::read(dir.path().join(name)).unwrap()
        ))
    );
    assert_eq!(
        values(&Engine::open_checked(dir.path().to_string_lossy().into_owned()).unwrap())["rowDensity"],
        "compact"
    );
}
#[test]
fn bootstrap_review_tokens_reject_changed_remote_buttons_and_survive_reopen() {
    let d = mock_dav::MockDav::start();
    let (remote, _rd) = engine();
    let (local, ld) = engine();
    remote.with_store_mut(|s| {
        s.state.rest.insert("globalConfig".into(), json!({}));
    });
    remote
        .stage_portable_settings(json!({"version":1,"values":{"sort":"title"}}).to_string())
        .unwrap();
    remote.set_settings_sync_enabled(true).unwrap();
    remote.sync_nextcloud(nextcloud(&d)).unwrap();
    local
        .stage_portable_settings(json!({"version":1,"values":{"sort":"manual"}}).to_string())
        .unwrap();
    local.set_settings_sync_enabled(true).unwrap();
    local.sync_nextcloud(nextcloud(&d)).unwrap();
    let old = local.settings_sync_status().review_id.unwrap();
    let local = Engine::open_checked(ld.path().to_string_lossy().into_owned()).unwrap();
    assert_eq!(local.settings_sync_status().review_id.as_deref(), Some(old.as_str()));
    remote
        .stage_portable_settings(json!({"version":1,"values":{"sort":"created"}}).to_string())
        .unwrap();
    remote.sync_nextcloud(nextcloud(&d)).unwrap();
    local.sync_nextcloud(nextcloud(&d)).unwrap();
    let current = local.settings_sync_status().review_id.unwrap();
    assert_ne!(old, current);
    assert!(local
        .set_settings_bootstrap_choice(SettingsBootstrapChoice::UseRemote, old)
        .is_err());
    assert_eq!(values(&local)["sort"], "manual");
    local
        .set_settings_bootstrap_choice(SettingsBootstrapChoice::UseRemote, current)
        .unwrap();
    remote
        .stage_portable_settings(json!({"version":1,"values":{"sort":"due"}}).to_string())
        .unwrap();
    remote.sync_nextcloud(nextcloud(&d)).unwrap();
    local.sync_nextcloud(nextcloud(&d)).unwrap();
    assert!(local.settings_sync_status().bootstrap_required);
    assert_eq!(values(&local)["sort"], "manual");
    let token = local.settings_sync_status().review_id.unwrap();
    local
        .set_settings_bootstrap_choice(SettingsBootstrapChoice::UseRemote, token.clone())
        .unwrap();
    drop(local);
    let reopened = Engine::open_checked(ld.path().to_string_lossy().into_owned()).unwrap();
    assert_eq!(
        reopened.settings_sync_status().review_id.as_deref(),
        Some(token.as_str())
    );
    reopened.sync_nextcloud(nextcloud(&d)).unwrap();
    assert_eq!(values(&reopened)["sort"], "due");
    assert!(!reopened.settings_sync_status().bootstrap_required);
}
#[cfg(feature = "p2p")]
#[test]
fn bootstrap_review_is_bound_across_nextcloud_to_nearby_and_peer_changes() {
    use crate::portable::{Document, NearbySettings, PortableState};
    use momentum_p2p::SettingsHandler;
    let d = mock_dav::MockDav::start();
    let (remote, _rd) = engine();
    let (local, _ld) = engine();
    remote.with_store_mut(|s| {
        s.state.rest.insert("globalConfig".into(), json!({}));
    });
    remote
        .stage_portable_settings(json!({"version":1,"values":{"sort":"title"}}).to_string())
        .unwrap();
    remote.set_settings_sync_enabled(true).unwrap();
    remote.sync_nextcloud(nextcloud(&d)).unwrap();
    local
        .stage_portable_settings(json!({"version":1,"values":{"sort":"manual"}}).to_string())
        .unwrap();
    local.set_settings_sync_enabled(true).unwrap();
    local.sync_nextcloud(nextcloud(&d)).unwrap();
    let cloud = local.settings_sync_status().review_id.unwrap();
    local
        .set_settings_bootstrap_choice(SettingsBootstrapChoice::UseRemote, cloud.clone())
        .unwrap();
    let callback = NearbySettings {
        engine: std::sync::Arc::downgrade(&local),
        stopped: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
    };
    let envelope = |peer: &str, sort: &str| {
        let mut s = PortableState::default();
        s.stage(
            Document::parse(&json!({"version":1,"values":{"sort":sort}}).to_string()).unwrap(),
            peer,
        );
        json!({"version":momentum_p2p::SETTINGS_CHANNEL_VERSION,"accepted":{},"payload":s.payload}).to_string()
    };
    callback.receive("peer-a", &envelope("peer-a", "created")).unwrap();
    let first = local.settings_sync_status().review_id.unwrap();
    assert_ne!(first, cloud);
    assert_eq!(values(&local)["sort"], "manual");
    assert!(local
        .set_settings_bootstrap_choice(SettingsBootstrapChoice::UseRemote, cloud)
        .is_err());
    local
        .set_settings_bootstrap_choice(SettingsBootstrapChoice::UseRemote, first.clone())
        .unwrap();
    callback.receive("peer-b", &envelope("peer-b", "due")).unwrap();
    assert_ne!(local.settings_sync_status().review_id.as_deref(), Some(first.as_str()));
    assert!(local
        .set_settings_bootstrap_choice(SettingsBootstrapChoice::UseRemote, first)
        .is_err());
    assert_eq!(values(&local)["sort"], "manual");
    let second = local.settings_sync_status().review_id.unwrap();
    local
        .set_settings_bootstrap_choice(SettingsBootstrapChoice::UseRemote, second)
        .unwrap();
    callback.receive("peer-b", &envelope("peer-b", "due")).unwrap();
    assert_eq!(values(&local)["sort"], "due");
}

#[test]
fn day_period_visibility_setter_is_local_and_portable_updates_preserve_it() {
    let (e, dir) = engine();
    let document = json!({"version":1,"values":{"grouping":"project","sort":"due","direction":"descending","upcomingDays":30,"autoArchive":true,"rowDensity":"compact","colorfulLabels":false,"completionSoundEnabled":true,"completionSoundChoice":"warm-spark"}}).to_string();
    e.stage_portable_settings(document).unwrap();
    e.add_task("Pending local work".into(), View::Today);
    e.set_device_summary_preferences(true, ClockTime { hour: 9, minute: 30 })
        .unwrap();
    let before = e.preferences();
    let portable = e.portable_settings_json().unwrap();
    let status = e.settings_sync_status();
    let pending = e.with_store(|s| serde_json::to_value(&s.pending).unwrap());
    let portable_bytes = std::fs::read(dir.path().join("portable-settings.json")).unwrap();
    for visible in [false, false, true, false] {
        e.set_day_period_tasks_in_today(visible);
        let mut expected = before.clone();
        expected.show_day_period_tasks_in_today = visible;
        assert_eq!(e.preferences(), expected);
        assert_eq!(e.portable_settings_json().unwrap(), portable);
        assert_eq!(e.settings_sync_status(), status);
        assert_eq!(e.with_store(|s| serde_json::to_value(&s.pending).unwrap()), pending);
        assert_eq!(
            std::fs::read(dir.path().join("portable-settings.json")).unwrap(),
            portable_bytes
        );
    }
    e.stage_portable_settings(json!({"version":1,"values":{"grouping":"estimate"}}).to_string())
        .unwrap();
    assert_eq!(e.preferences().group_by, GroupBy::Estimate);
    assert!(!e.preferences().show_day_period_tasks_in_today);
    let path = dir.path().join("import-settings.json");
    std::fs::write(
        &path,
        json!({"version":1,"values":{"grouping":"none","showDayPeriodTasksInToday":true}}).to_string(),
    )
    .unwrap();
    e.import_settings(path.to_string_lossy().into_owned()).unwrap();
    assert_eq!(e.preferences().group_by, GroupBy::None);
    assert!(!e.preferences().show_day_period_tasks_in_today);
    assert!(values(&e).get("showDayPeriodTasksInToday").is_none());
    assert_eq!(e.with_store(|s| serde_json::to_value(&s.pending).unwrap()), pending);
}

#[test]
fn quiet_sound_choices_round_trip_without_changing_local_only_preferences() {
    let (e, _dir) = engine();
    for slug in [
        "soft-knock",
        "little-lift",
        "felt-finish",
        "warm-thumb",
        "gentle-pair",
        "small-bloom",
        "sunny-tap",
        "pocket-harp",
        "easy-landing",
        "quiet-cheer",
        "unknown-future-sound",
    ] {
        let document = json!({"version":1,"values":{"completionSoundChoice":slug}}).to_string();
        e.stage_portable_settings(document).unwrap();
        assert_eq!(values(&e)["completionSoundChoice"], slug);
    }
    assert!(values(&e).get("completionSoundVolume").is_none());
    assert!(values(&e).get("dynamicEmptyStateMessages").is_none());
}
