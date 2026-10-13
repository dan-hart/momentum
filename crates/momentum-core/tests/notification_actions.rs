// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
use momentum_core::momentum_model::{local_ms, AppData, Task};
use momentum_core::momentum_oplog::Action;
use momentum_core::{CoreError, Engine, MorningSummary, NotificationAction, NotificationContent, NotificationRequest};
use serde_json::json;
use std::sync::Arc;

fn setup() -> (Arc<Engine>, tempfile::TempDir, u64, NotificationRequest) {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open(dir.path().to_string_lossy().into_owned());
    let now = local_ms("2026-09-16", 7, 0).unwrap();
    engine.with_store_mut(|s| {
        s.state.task.insert(
            "task",
            Task {
                id: "task".into(),
                title: "Private title".into(),
                due_day: Some("2026-09-16".into()),
                remind_at: Some(now - 60_000),
                ..Default::default()
            },
        );
        s.save().unwrap();
    });
    let request = engine.notification_plan(now).unwrap().requests.remove(0);
    assert!(engine.accept_notification(request.clone(), now).unwrap().accepted);
    (engine, dir, now, request)
}
fn snapshot(engine: &Engine) -> serde_json::Value {
    engine.with_store(|s| json!([s.state, s.pending, s.meta]))
}

#[test]
fn notification_action_rejects_changed_deleted_completed_and_restored_sources() {
    for change in ["title", "delete", "complete", "restore"] {
        let (engine, dir, now, request) = setup();
        match change {
            "title" => engine.dispatch(Action::UpdateTask {
                id: "task".into(),
                changes: [("title".into(), json!("Edited"))].into_iter().collect(),
            }),
            "delete" => {
                engine.delete_task("task".into());
            }
            "complete" => {
                engine.set_done("task".into(), true);
            }
            _ => {
                let backup = dir.path().join("replacement.json");
                std::fs::write(&backup, serde_json::to_vec(&json!({"data": AppData::fresh()})).unwrap()).unwrap();
                engine.import_backup(backup.to_string_lossy().into_owned()).unwrap();
            }
        }
        let before = snapshot(&engine);
        let undo = engine.undo_top();
        for action in [NotificationAction::Done, NotificationAction::Snooze { minutes: 60 }] {
            let outcome = engine
                .perform_notification_action(request.clone(), action, now)
                .unwrap();
            assert!(!outcome.changed, "{change}: a stale action cannot edit the replacement");
            assert!(outcome.message.is_none() && outcome.undo.is_none());
            assert_eq!(snapshot(&engine), before);
            assert_eq!(engine.undo_top(), undo);
        }
    }
}

#[test]
fn notification_action_rejects_same_source_after_restore_and_accepts_its_new_identity() {
    for action in [NotificationAction::Done, NotificationAction::Snooze { minutes: 60 }] {
        let dir = tempfile::tempdir().unwrap();
        let engine = Engine::open(dir.path().to_string_lossy().into_owned());
        let now = local_ms("2026-09-16", 7, 0).unwrap();
        engine.with_store_mut(|store| {
            store.state.task.insert(
                "same-task",
                Task {
                    id: "same-task".into(),
                    title: "Same private title".into(),
                    due_day: Some("2026-09-16".into()),
                    remind_at: Some(now + 60_000),
                    ..Default::default()
                },
            );
            store.save().unwrap();
        });
        let old = engine.notification_plan(now).unwrap().requests.remove(0);
        assert!(engine.accept_notification(old.clone(), now).unwrap().accepted);
        let backup = dir.path().join("same-source-backup.json");
        engine.export_backup(backup.to_string_lossy().into_owned()).unwrap();
        engine.import_backup(backup.to_string_lossy().into_owned()).unwrap();
        let restored = snapshot(&engine);
        let outcome = engine
            .perform_notification_action(old.clone(), action.clone(), now)
            .unwrap();
        assert!(
            !outcome.changed,
            "a pre-restore action must not edit even an identical restored task"
        );
        assert!(outcome.message.is_none() && outcome.undo.is_none());
        assert_eq!(snapshot(&engine), restored);
        assert!(engine.undo_top().is_none());
        let reopened = Engine::open(dir.path().to_string_lossy().into_owned());
        assert_eq!(snapshot(&reopened), restored);
        let current = reopened.notification_plan(now).unwrap().requests.remove(0);
        assert_eq!(current.id, old.id);
        assert_eq!(current.content, old.content);
        assert_eq!(current.scheduled_at_ms, old.scheduled_at_ms);
        assert_ne!(current.source_revision, old.source_revision);
        assert!(reopened.accept_notification(current.clone(), now).unwrap().accepted);
        assert!(
            reopened
                .perform_notification_action(current, action, now)
                .unwrap()
                .changed
        );
    }
}

#[test]
fn notification_done_uses_existing_durable_completion_and_undo() {
    let (engine, dir, now, request) = setup();
    let outcome = engine
        .perform_notification_action(request.clone(), NotificationAction::Done, now)
        .unwrap();
    assert!(outcome.changed && outcome.undo.is_some());
    assert!(engine.task_detail("task".into()).unwrap().is_done);
    assert_eq!(
        snapshot(&engine),
        snapshot(&Engine::open(dir.path().to_string_lossy().into_owned()))
    );
    assert!(
        !engine
            .perform_notification_action(request, NotificationAction::Done, now)
            .unwrap()
            .changed
    );
    assert!(engine.undo().changed);
    assert!(!engine.task_detail("task".into()).unwrap().is_done);
    assert_eq!(
        snapshot(&engine),
        snapshot(&Engine::open(dir.path().to_string_lossy().into_owned()))
    );
}

#[test]
fn notification_snooze_rearms_once_using_the_shared_minutes_rule() {
    let (engine, dir, now, request) = setup();
    let outcome = engine
        .perform_notification_action(request.clone(), NotificationAction::Snooze { minutes: 60 }, now)
        .unwrap();
    assert!(outcome.changed);
    engine.with_store(|s| assert_eq!(s.state.task.entities["task"].remind_at, Some(now + 3_600_000)));
    let before = snapshot(&engine);
    assert!(
        !engine
            .perform_notification_action(request, NotificationAction::Snooze { minutes: 60 }, now + 1)
            .unwrap()
            .changed
    );
    assert_eq!(snapshot(&engine), before);
    assert_eq!(
        snapshot(&engine),
        snapshot(&Engine::open(dir.path().to_string_lossy().into_owned()))
    );
    let replacement = engine.notification_plan(now).unwrap().requests.remove(0);
    assert_eq!(replacement.scheduled_at_ms, now + 3_600_000);
}

#[test]
fn notification_action_save_failure_preserves_tasks_undo_disk_and_acceptance() {
    for action in [NotificationAction::Done, NotificationAction::Snooze { minutes: 60 }] {
        let (engine, dir, now, request) = setup();
        let before = snapshot(&engine);
        let undo = engine.undo_top();
        let ledger = std::fs::read(dir.path().join("notification-schedule.json")).unwrap();
        std::fs::create_dir(dir.path().join("pending.json.tmp")).unwrap();
        assert!(matches!(
            engine.perform_notification_action(request.clone(), action.clone(), now),
            Err(CoreError::Io { .. })
        ));
        assert_eq!(snapshot(&engine), before);
        assert_eq!(engine.undo_top(), undo);
        assert_eq!(
            snapshot(&Engine::open(dir.path().to_string_lossy().into_owned())),
            before
        );
        assert_eq!(
            std::fs::read(dir.path().join("notification-schedule.json")).unwrap(),
            ledger
        );
        std::fs::remove_dir(dir.path().join("pending.json.tmp")).unwrap();
        assert!(
            engine
                .perform_notification_action(request, action, now)
                .unwrap()
                .changed
        );
    }
}

#[test]
fn notification_action_rejects_non_reminders_and_preserves_a_malformed_ledger() {
    let (engine, dir, now, mut request) = setup();
    let before = snapshot(&engine);
    let summary = NotificationRequest {
        content: NotificationContent::Summary {
            day: "2026-09-16".into(),
            counts: MorningSummary {
                total: 1,
                morning: 0,
                tonight: 0,
            },
            task_ids: vec!["task".into()],
        },
        ..request.clone()
    };
    assert!(
        !engine
            .perform_notification_action(summary, NotificationAction::Done, now)
            .unwrap()
            .changed
    );
    let ledger = dir.path().join("notification-schedule.json");
    std::fs::write(&ledger, b"malformed existing ledger").unwrap();
    assert!(matches!(
        engine.perform_notification_action(request.clone(), NotificationAction::Done, now),
        Err(CoreError::Invalid { .. })
    ));
    assert_eq!(snapshot(&engine), before);
    assert_eq!(std::fs::read(&ledger).unwrap(), b"malformed existing ledger");
    request.fire_at_ms = request.scheduled_at_ms - 1;
    std::fs::remove_file(ledger).unwrap();
    assert!(
        !engine
            .perform_notification_action(request, NotificationAction::Done, now)
            .unwrap()
            .changed
    );
    assert_eq!(snapshot(&engine), before);
}
