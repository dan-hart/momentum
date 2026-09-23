// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
use momentum_core::sp_model::{local_ms, Task};
use momentum_core::{Engine, NotificationObservation, NotificationRequest, Preferences};
use std::sync::Arc;

fn setup() -> (Arc<Engine>, tempfile::TempDir, u64) {
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
                remind_at: Some(now + 60_000),
                ..Default::default()
            },
        );
        s.save().unwrap();
    });
    (engine, dir, now)
}
fn observation(r: &NotificationRequest) -> NotificationObservation {
    NotificationObservation {
        id: r.id.clone(),
        source_revision: r.source_revision.clone(),
    }
}
fn first(e: &Engine, now: u64) -> NotificationRequest {
    e.notification_plan(now).unwrap().requests.remove(0)
}

#[test]
fn acceptance_persists_across_restart_without_claiming_delivery() {
    let (e, dir, now) = setup();
    let request = first(&e, now);
    assert!(e.accept_notification(request.clone(), now).unwrap().accepted);
    let reloaded = Engine::open(dir.path().to_string_lossy().into_owned());
    let p = reloaded.notification_plan(now).unwrap();
    assert_eq!(p.requests, vec![request.clone()]);
    assert_eq!(p.accepted, vec![observation(&request)]);
    assert!(!reloaded.notified_contains("task"));
    reloaded.with_store(|s| assert!(s.meta.last_summary_day.is_empty()));
}
#[test]
fn denied_or_failed_os_scheduling_never_consumes_a_request() {
    let (e, dir, now) = setup();
    let p = e.notification_plan(now).unwrap();
    assert!(p.accepted.is_empty());
    assert!(!dir.path().join("notification-schedule.json").exists());
    assert_eq!(p, e.reconcile_notifications(now, vec![], vec![]).unwrap());
    assert!(first(&e, now + 120_000).fire_at_ms > now);
    e.with_store(|s| assert_eq!(s.state.task.entities["task"].remind_at, Some(now + 60_000)));
}
#[test]
fn missing_future_acceptance_is_eligible_for_rescheduling() {
    let (e, _dir, now) = setup();
    let r = first(&e, now);
    e.accept_notification(r.clone(), now).unwrap();
    let pending = e.reconcile_notifications(now, vec![observation(&r)], vec![]).unwrap();
    assert_eq!(pending.accepted, vec![observation(&r)]);
    let missing = e.reconcile_notifications(now, vec![], vec![]).unwrap();
    assert_eq!(missing.requests, vec![r]);
    assert!(missing.accepted.is_empty());
}
#[test]
fn elapsed_dismissed_acceptance_stays_consumed_after_restart_and_clock_rollback() {
    let (e, dir, now) = setup();
    e.accept_notification(first(&e, now), now).unwrap();
    assert!(e
        .reconcile_notifications(now + 120_000, vec![], vec![])
        .unwrap()
        .requests
        .is_empty());
    let e = Engine::open(dir.path().to_string_lossy().into_owned());
    assert!(e.notification_plan(now).unwrap().requests.is_empty());
}
#[test]
fn source_edit_during_os_await_rejects_acceptance_and_returns_exact_cancellation() {
    let (e, _dir, now) = setup();
    let r = first(&e, now);
    e.with_store_mut(|s| s.state.task.entities.get_mut("task").unwrap().title = "Changed".into());
    let a = e.accept_notification(r.clone(), now).unwrap();
    assert!(!a.accepted);
    assert_eq!(a.cancellation, Some(observation(&r)));
    assert!(e.notification_plan(now).unwrap().accepted.is_empty());
    assert_ne!(first(&e, now).source_revision, r.source_revision);
}
#[test]
fn stale_pending_revision_does_not_acknowledge_new_source() {
    let (e, _dir, now) = setup();
    let old = first(&e, now);
    e.accept_notification(old.clone(), now).unwrap();
    e.with_store_mut(|s| s.state.task.entities.get_mut("task").unwrap().title = "Changed".into());
    let p = e.reconcile_notifications(now, vec![observation(&old)], vec![]).unwrap();
    assert_eq!(p.cancellations, vec![observation(&old)]);
    assert!(p.accepted.is_empty());
    assert_ne!(p.requests[0].source_revision, old.source_revision);
}
#[test]
fn source_completion_deletion_and_import_cancel_future_acceptances() {
    for change in ["complete", "delete", "import"] {
        let (e, dir, now) = setup();
        let r = first(&e, now);
        e.accept_notification(r.clone(), now).unwrap();
        match change {
            "complete" => e.with_store_mut(|s| s.state.task.entities.get_mut("task").unwrap().is_done = true),
            "delete" => e.with_store_mut(|s| {
                s.state.task.remove("task");
            }),
            _ => {
                let path = dir.path().join("backup.json");
                std::fs::write(
                    &path,
                    serde_json::to_vec(&serde_json::json!({"data": momentum_core::sp_model::AppData::fresh()}))
                        .unwrap(),
                )
                .unwrap();
                e.import_backup(path.to_string_lossy().into_owned()).unwrap();
            }
        }
        let p = e.reconcile_notifications(now, vec![observation(&r)], vec![]).unwrap();
        assert!(p.requests.is_empty());
        assert_eq!(p.cancellations, vec![observation(&r)]);
    }
}
#[test]
fn summary_consumption_survives_later_clock_edit_and_rollback() {
    let (e, dir, now) = setup();
    e.with_store_mut(|s| {
        s.state.task.entities.get_mut("task").unwrap().remind_at = None;
        s.save().unwrap();
    });
    let mut prefs = Preferences {
        morning_summary_enabled: true,
        ..Default::default()
    };
    e.set_preferences(prefs.clone());
    let r = first(&e, now);
    e.accept_notification(r, now).unwrap();
    e.reconcile_notifications(now + 2 * 3_600_000, vec![], vec![]).unwrap();
    prefs.morning_summary_time.hour = 11;
    e.set_preferences(prefs.clone());
    assert!(e.notification_plan(now + 2 * 3_600_000).unwrap().requests.is_empty());
    let reloaded = Engine::open(dir.path().to_string_lossy().into_owned());
    reloaded.set_preferences(prefs);
    assert!(reloaded.notification_plan(now).unwrap().requests.is_empty());
}
#[test]
fn changed_summary_time_and_opt_out_cancel_future_acceptance() {
    for disabled in [false, true] {
        let (e, _dir, now) = setup();
        e.with_store_mut(|s| s.state.task.entities.get_mut("task").unwrap().remind_at = None);
        let mut prefs = Preferences {
            morning_summary_enabled: true,
            ..Default::default()
        };
        e.set_preferences(prefs.clone());
        let r = first(&e, now);
        e.accept_notification(r.clone(), now).unwrap();
        prefs.morning_summary_enabled = !disabled;
        prefs.morning_summary_time.hour = 9;
        e.set_preferences(prefs);
        let p = e.reconcile_notifications(now, vec![observation(&r)], vec![]).unwrap();
        assert_eq!(p.cancellations, vec![observation(&r)]);
        assert!(p.accepted.is_empty());
        assert_eq!(p.requests.is_empty(), disabled);
    }
}
#[test]
fn corrupt_or_unreadable_ledger_fails_closed_without_replacing_it() {
    for is_dir in [false, true] {
        let (e, dir, now) = setup();
        let r = first(&e, now);
        let path = dir.path().join("notification-schedule.json");
        if is_dir {
            std::fs::create_dir(&path).unwrap();
        } else {
            std::fs::write(&path, b"broken").unwrap();
        }
        assert!(e.notification_plan(now).is_err());
        assert!(e.reconcile_notifications(now, vec![], vec![]).is_err());
        assert!(e.accept_notification(r, now).is_err());
        if !is_dir {
            assert_eq!(std::fs::read(path).unwrap(), b"broken");
        }
    }
}
#[test]
fn failed_atomic_save_preserves_previous_ledger() {
    let (e, dir, now) = setup();
    e.accept_notification(first(&e, now), now).unwrap();
    let path = dir.path().join("notification-schedule.json");
    let before = std::fs::read(&path).unwrap();
    std::fs::create_dir(dir.path().join("notification-schedule.json.tmp")).unwrap();
    assert!(e.reconcile_notifications(now, vec![], vec![]).is_err());
    assert_eq!(std::fs::read(path).unwrap(), before);
    assert_eq!(e.notification_plan(now).unwrap().accepted.len(), 1);
}

#[test]
fn earlier_acceptance_does_not_make_stale_callback_valid() {
    let (e, _dir, now) = setup();
    let r = first(&e, now);
    e.accept_notification(r.clone(), now).unwrap();
    e.with_store_mut(|s| s.state.task.entities.get_mut("task").unwrap().is_done = true);
    let a = e.accept_notification(r.clone(), now).unwrap();
    assert!(!a.accepted);
    assert_eq!(a.cancellation, Some(observation(&r)));
}
#[test]
fn overdue_acceptance_survives_await_latency_but_is_never_rescheduled() {
    let (e, _dir, now) = setup();
    let r = first(&e, now + 120_000);
    assert!(e.accept_notification(r.clone(), now + 180_000).unwrap().accepted);
    assert!(e.notification_plan(now + 180_000).unwrap().requests.is_empty());
    assert!(e.accept_notification(r, now + 180_001).unwrap().accepted);
}
#[test]
fn delivered_and_pending_observations_recover_os_success_before_durable_acceptance() {
    for delivered in [false, true] {
        let (e, _dir, now) = setup();
        let r = first(&e, now);
        let (pending, shown) = if delivered {
            (vec![], vec![observation(&r)])
        } else {
            (vec![observation(&r)], vec![])
        };
        let p = e.reconcile_notifications(now, pending, shown).unwrap();
        assert!(p.cancellations.is_empty());
        assert_eq!(p.requests.is_empty(), delivered);
        assert_eq!(p.accepted.is_empty(), delivered);
    }
}
#[test]
fn source_revision_ignores_unrelated_task_edits() {
    let (e, _dir, now) = setup();
    let r = first(&e, now);
    e.with_store_mut(|s| {
        s.state.task.insert(
            "other",
            Task {
                id: "other".into(),
                title: "Other".into(),
                ..Default::default()
            },
        )
    });
    assert_eq!(first(&e, now).source_revision, r.source_revision);
    assert!(e.accept_notification(r, now).unwrap().accepted);
}
#[test]
fn recurrence_stop_cancels_projected_summary() {
    let (e, _dir, now) = setup();
    e.with_store_mut(|s| {
        s.state.task.remove("task");
        s.state.task_repeat_cfg.insert(
            "daily",
            momentum_core::sp_model::RepeatCfg {
                id: "daily".into(),
                title: Some("Private repeat".into()),
                repeat_cycle: "DAILY".into(),
                repeat_every: 1,
                start_date: Some("2026-09-16".into()),
                ..Default::default()
            },
        );
    });
    e.set_preferences(Preferences {
        morning_summary_enabled: true,
        ..Default::default()
    });
    let r = first(&e, now);
    e.accept_notification(r.clone(), now).unwrap();
    e.with_store_mut(|s| s.state.task_repeat_cfg.entities.get_mut("daily").unwrap().is_paused = true);
    let p = e.reconcile_notifications(now, vec![observation(&r)], vec![]).unwrap();
    assert!(p.requests.is_empty());
    assert_eq!(p.cancellations, vec![observation(&r)]);
}
#[test]
fn consumed_overdue_requests_do_not_starve_capacity() {
    let (e, _dir, now) = setup();
    for n in 0..65 {
        e.with_store_mut(|s| {
            s.state.task.insert(
                &format!("past-{n}"),
                Task {
                    id: format!("past-{n}"),
                    title: "Past".into(),
                    remind_at: Some(now - 1),
                    ..Default::default()
                },
            );
        });
        let r = first(&e, now);
        assert!(r.id.starts_with("reminder:past-"));
        e.accept_notification(r, now).unwrap();
    }
    let p = e.notification_plan(now).unwrap();
    assert_eq!(p.requests.len(), 1);
    assert_eq!(p.overflow_reminders, 0);
    assert_eq!(p.requests[0].id, format!("reminder:task:{}", now + 60_000));
}

#[test]
fn timezone_change_invalidates_future_revision_in_isolated_process() {
    if let Ok(dir) = std::env::var("MOMENTUM_LEDGER_TZ_FIXTURE") {
        let e = Engine::open(dir.clone());
        let r: NotificationRequest =
            serde_json::from_slice(&std::fs::read(std::path::Path::new(&dir).join("request.json")).unwrap()).unwrap();
        let p = e
            .reconcile_notifications(r.scheduled_at_ms - 60_000, vec![observation(&r)], vec![])
            .unwrap();
        assert_eq!(p.cancellations, vec![observation(&r)]);
        assert!(p.accepted.is_empty());
        assert_eq!(p.requests.len(), 1);
        assert_ne!(p.requests[0].source_revision, r.source_revision);
        return;
    }
    let (e, dir, now) = setup();
    let r = first(&e, now);
    e.accept_notification(r.clone(), now).unwrap();
    std::fs::write(dir.path().join("request.json"), serde_json::to_vec(&r).unwrap()).unwrap();
    // Pick a timezone guaranteed to differ from the parent at this timestamp.
    let zone = if momentum_core::sp_model::time_of_ms(now).0 == 0 {
        "Pacific/Honolulu"
    } else {
        "UTC"
    };
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "timezone_change_invalidates_future_revision_in_isolated_process",
        ])
        .env("TZ", zone)
        .env("MOMENTUM_LEDGER_TZ_FIXTURE", dir.path())
        .output()
        .unwrap();
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stdout));
}
#[test]
fn acceptance_write_failure_never_claims_success_or_mutates_tasks() {
    let (e, dir, now) = setup();
    let r = first(&e, now);
    let before = e.with_store(|s| serde_json::to_value((&s.state, &s.pending, &s.meta)).unwrap());
    std::fs::create_dir(dir.path().join("notification-schedule.json.tmp")).unwrap();
    assert!(e.accept_notification(r.clone(), now).is_err());
    assert!(!dir.path().join("notification-schedule.json").exists());
    assert!(e.notification_plan(now).unwrap().accepted.is_empty());
    assert_eq!(first(&e, now), r);
    assert_eq!(
        before,
        e.with_store(|s| serde_json::to_value((&s.state, &s.pending, &s.meta)).unwrap())
    );
}
#[test]
fn a_stale_delivered_revision_does_not_consume_replacement() {
    let (e, _dir, now) = setup();
    let old = first(&e, now);
    e.with_store_mut(|s| s.state.task.entities.get_mut("task").unwrap().title = "New".into());
    let r = first(&e, now);
    let p = e.reconcile_notifications(now, vec![], vec![observation(&old)]).unwrap();
    assert_eq!(p.cancellations, vec![observation(&old)]);
    assert_eq!(p.requests, vec![r]);
    assert!(p.accepted.is_empty());
}

#[cfg(unix)]
#[test]
fn read_only_schedule_directory_propagates_acceptance_error() {
    use std::os::unix::fs::PermissionsExt;
    let (e, dir, now) = setup();
    let request = first(&e, now);
    let original = std::fs::metadata(dir.path()).unwrap().permissions();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o500)).unwrap();
    let result = e.accept_notification(request.clone(), now);
    std::fs::set_permissions(dir.path(), original).unwrap();
    assert!(result.is_err());
    assert!(!dir.path().join("notification-schedule.json").exists());
    assert_eq!(first(&e, now), request);
}
