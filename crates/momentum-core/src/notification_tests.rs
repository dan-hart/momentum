// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
use crate::notifications::*;
use crate::{ClockTime, Engine, MorningSummary, Preferences};
use serde_json::json;
use sp_model::*;
use sp_store::Store;

fn options(day: &str) -> NotificationPlanningOptions {
    NotificationPlanningOptions {
        now_ms: local_ms(day, 7, 0).unwrap(),
        source_revision: "snapshot-1".into(),
        consumed_ids: Default::default(),
        morning_summary_enabled: false,
        morning_summary_time: ClockTime { hour: 8, minute: 0 },
    }
}
fn store() -> (Store, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    (Store::load(dir.path().into()), dir)
}
fn task(store: &mut Store, id: &str, day: &str, reminder: Option<u64>) {
    store.state.task.insert(
        id,
        Task {
            id: id.into(),
            title: id.into(),
            due_day: Some(day.into()),
            remind_at: reminder,
            ..Default::default()
        },
    );
}
fn repeat(store: &mut Store, id: &str, cycle: &str, start: &str) {
    store.state.task_repeat_cfg.insert(
        id,
        RepeatCfg {
            id: id.into(),
            title: Some(id.into()),
            repeat_cycle: cycle.into(),
            repeat_every: 1,
            start_date: Some(start.into()),
            ..Default::default()
        },
    );
}
fn summary(request: &NotificationRequest) -> (&str, &MorningSummary, &[String]) {
    match &request.content {
        NotificationContent::Summary { day, counts, task_ids } => (day, counts, task_ids),
        _ => panic!("expected summary"),
    }
}

#[test]
fn notification_horizon_is_seven_local_dates_with_actual_overdue_catch_up() {
    let (mut s, _dir) = store();
    let o = options("2026-03-06"); // Spans the North American spring DST change.
    let end = local_ms("2026-03-13", 0, 0).unwrap();
    task(&mut s, "overdue", "2026-03-01", Some(o.now_ms - 1000));
    task(&mut s, "last", "2026-03-12", Some(end - 1));
    task(&mut s, "outside", "2026-03-13", Some(end));
    task(&mut s, "done", "2026-03-06", Some(o.now_ms));
    s.state.task.entities.get_mut("done").unwrap().is_done = true;
    task(&mut s, "date-only", "2026-03-06", None);
    let p = plan(&s, &o);
    assert_eq!(p.horizon_end_ms, end);
    assert_eq!(p.requests.len(), 2);
    assert_eq!(p.requests[0].fire_at_ms, o.now_ms);
    assert_eq!(p.requests[0].scheduled_at_ms, o.now_ms - 1000);
    assert_eq!(p.requests[1].fire_at_ms, end - 1);
    assert_eq!(p.overflow_reminders, 0, "horizon exclusions are not capacity overflow");
}

#[test]
fn notification_capacity_59_60_61_prioritizes_actual_reminders_with_stable_ties() {
    for count in [59_usize, 60, 61] {
        let (mut s, _dir) = store();
        let mut o = options("2026-09-16");
        o.morning_summary_enabled = true;
        for n in (0..count).rev() {
            task(&mut s, &format!("task-{n:02}"), "2026-09-16", Some(o.now_ms + 1000));
        }
        let p = plan(&s, &o);
        assert_eq!(p.requests.len(), 60);
        assert_eq!(p.overflow_reminders, count.saturating_sub(60) as u64);
        assert_eq!(p.overflow_summaries, u64::from(count >= 60));
        for (n, request) in p.requests.iter().take(count.min(60)).enumerate() {
            assert!(
                matches!(&request.content, NotificationContent::Reminder { task_id, .. } if *task_id == format!("task-{n:02}"))
            );
        }
        if count == 59 {
            assert_eq!(summary(&p.requests[59]).1.total, 59);
        }
        s.state.task.ids.reverse();
        assert_eq!(p, plan(&s, &o));
    }
}

#[test]
fn notification_summaries_are_opt_in_nonempty_capped_and_date_identified() {
    let (mut s, _dir) = store();
    let mut o = options("2026-09-16");
    repeat(&mut s, "daily", "DAILY", "2026-09-16");
    assert!(plan(&s, &o).requests.is_empty());
    assert!(plan(&s, &o).projected_occurrences.is_empty());
    o.morning_summary_enabled = true;
    let p = plan(&s, &o);
    assert_eq!(p.requests.len(), 4);
    assert_eq!(p.overflow_summaries, 3);
    assert_eq!(p.projected_occurrences.len(), 7);
    assert_eq!(summary(&p.requests[0]).1.total, 1);
    assert_eq!(summary(&p.requests[0]).2, ["rpt_daily_2026-09-16"]);
    o.morning_summary_time.hour = 9;
    let later = plan(&s, &o);
    assert_eq!(p.requests[0].id, later.requests[0].id);
    assert_ne!(p.requests[0].scheduled_at_ms, later.requests[0].scheduled_at_ms);
    o.now_ms = local_ms("2026-09-16", 10, 0).unwrap();
    assert_eq!(plan(&s, &o).requests[0].fire_at_ms, o.now_ms);
    s.meta.last_summary_day = "2026-09-16".into();
    assert_eq!(summary(&plan(&s, &o).requests[0]).0, "2026-09-17");
    o.morning_summary_time.hour = 24;
    assert!(plan(&s, &o).requests.is_empty());
    let (empty, _empty_dir) = store();
    o.morning_summary_time.hour = 8;
    assert!(plan(&empty, &o).requests.is_empty());
}

#[test]
fn notification_summary_membership_matches_desktop_including_slot_precedence() {
    let dir = tempfile::tempdir().unwrap();
    let e = Engine::open(dir.path().to_string_lossy().into_owned());
    let today = today_str();
    e.with_store_mut(|s| {
        for (id, title) in [("morning", "mOrNiNg"), ("evening", "Evening")] {
            s.state.tag.insert(
                id,
                Tag {
                    id: id.into(),
                    title: title.into(),
                    ..Default::default()
                },
            );
        }
        for id in ["both", "evening", "plain", "done", "child", "wrong-date"] {
            task(s, id, &today, None);
        }
        s.state.task.entities.get_mut("both").unwrap().tag_ids = vec!["evening".into(), "morning".into()];
        s.state.task.entities.get_mut("evening").unwrap().tag_ids = vec!["evening".into()];
        s.state.task.entities.get_mut("child").unwrap().parent_id = Some("plain".into());
        s.state.task.entities.get_mut("done").unwrap().is_done = true;
        s.state.task.entities.get_mut("wrong-date").unwrap().due_with_time =
            local_ms(&day_str(day_number(&today).unwrap() + 1), 8, 0);
    });
    let mut o = options(&today);
    o.morning_summary_enabled = true;
    let p = e.with_store(|s| plan(s, &o));
    assert_eq!(
        *summary(&p.requests[0]).1,
        MorningSummary {
            total: 3,
            morning: 1,
            tonight: 1
        }
    );
    e.set_preferences(Preferences {
        morning_summary_enabled: true,
        ..Default::default()
    });
    assert_eq!(
        e.morning_summary_at(ClockTime { hour: 23, minute: 59 }).unwrap(),
        *summary(&p.requests[0]).1
    );
}

#[test]
fn notification_recurrence_projects_date_only_without_inheriting_reminder_timing() {
    let (mut s, _dir) = store();
    let mut o = options("2026-09-16");
    o.morning_summary_enabled = true;
    task(&mut s, "original", "2026-09-16", Some(o.now_ms + 1000));
    let t = s.state.task.entities.get_mut("original").unwrap();
    t.due_with_time = Some(o.now_ms + 60_000);
    let mut cfg = RepeatCfg::for_task(t);
    cfg.id = "daily".into();
    cfg.repeat_cycle = "DAILY".into();
    cfg.start_date = Some("2026-09-16".into());
    cfg.last_task_creation_day = Some("2026-09-16".into());
    s.state.task_repeat_cfg.insert("daily", cfg);
    let p = plan(&s, &o);
    assert_eq!(
        p.requests
            .iter()
            .filter(|r| matches!(r.content, NotificationContent::Reminder { .. }))
            .count(),
        1
    );
    assert_eq!(p.projected_occurrences.len(), 6);
    assert_eq!(p.projected_occurrences[0].task_id, "rpt_daily_2026-09-17");
    s.state.task_repeat_cfg.entities.get_mut("daily").unwrap().is_paused = true;
    assert!(plan(&s, &o).projected_occurrences.is_empty());
    s.state.task_repeat_cfg.remove("daily");
    assert!(plan(&s, &o).projected_occurrences.is_empty());
}

#[test]
fn notification_projection_reuses_newest_due_day_and_skips_existing_or_archived_instances() {
    let (mut s, _dir) = store();
    let mut o = options("2026-09-16");
    o.morning_summary_enabled = true;
    repeat(&mut s, "weekly", "WEEKLY", "2026-09-01");
    s.state.task_repeat_cfg.entities.get_mut("weekly").unwrap().monday = true;
    repeat(&mut s, "daily", "DAILY", "2026-09-16");
    task(&mut s, "rpt_daily_2026-09-16", "2026-09-16", None);
    s.state.rest.insert(
        "archiveYoung".into(),
        json!({"task": {"entities": {
            "rpt_daily_2026-09-17": {"id": "rpt_daily_2026-09-17", "title": "archived"}
        }}}),
    );
    let p = plan(&s, &o);
    let ids: Vec<_> = p.projected_occurrences.iter().map(|p| p.task_id.as_str()).collect();
    assert!(
        ids.contains(&"rpt_weekly_2026-09-14"),
        "only the newest missed instance is projected"
    );
    assert!(ids.contains(&"rpt_weekly_2026-09-21"));
    assert!(!ids.contains(&"rpt_daily_2026-09-16"));
    assert!(!ids.contains(&"rpt_daily_2026-09-17"));
    assert!(ids.contains(&"rpt_daily_2026-09-18"));
    assert_eq!(summary(&p.requests[0]).1.total, 1, "overdue catch-up is not Today");
    assert_eq!(summary(&p.requests[1]).0, "2026-09-18", "empty days are skipped");
}

#[test]
fn notification_recurrence_covers_monthly_yearly_and_cycle_edits() {
    let (mut s, _dir) = store();
    let mut o = options("2028-02-25");
    o.morning_summary_enabled = true;
    repeat(&mut s, "monthly", "MONTHLY", "2028-01-31");
    repeat(&mut s, "yearly", "YEARLY", "2024-02-29");
    s.state
        .task_repeat_cfg
        .entities
        .get_mut("monthly")
        .unwrap()
        .last_task_creation_day = Some("2028-01-31".into());
    s.state
        .task_repeat_cfg
        .entities
        .get_mut("yearly")
        .unwrap()
        .last_task_creation_day = Some("2027-02-28".into());
    let p = plan(&s, &o);
    assert_eq!(p.requests.len(), 1);
    assert_eq!(summary(&p.requests[0]).0, "2028-02-29");
    assert_eq!(summary(&p.requests[0]).1.total, 2);
    s.state
        .task_repeat_cfg
        .entities
        .get_mut("monthly")
        .unwrap()
        .repeat_every = 2;
    assert_eq!(summary(&plan(&s, &o).requests[0]).1.total, 1);
}

#[test]
fn notification_planning_is_repeatable_and_does_not_mutate_store_disk_or_delivery_claims() {
    let dir = tempfile::tempdir().unwrap();
    let e = Engine::open(dir.path().to_string_lossy().into_owned());
    let mut o = options("2026-09-16");
    o.morning_summary_enabled = true;
    e.with_store_mut(|s| {
        task(s, "due", "2026-09-16", Some(o.now_ms));
        repeat(s, "daily", "DAILY", "2026-09-16");
        s.dispatch(sp_oplog::Action::UpdateTask {
            id: "due".into(),
            changes: [("title".into(), json!("Pending title"))].into_iter().collect(),
        });
    });
    let snapshot = || {
        e.with_store(|s| {
            (
                json!([s.state, s.pending, s.meta]),
                ["state.json", "pending.json", "meta.json"].map(|name| std::fs::read(s.dir().join(name)).unwrap()),
            )
        })
    };
    let before = snapshot();
    let p = e.with_store(|s| plan(s, &o));
    assert_eq!(p, e.with_store(|s| plan(s, &o)));
    assert_eq!(before, snapshot());
    assert!(!e.notified_contains("due"));
    let mut revised = o.clone();
    revised.source_revision = "snapshot-2".into();
    let p2 = e.with_store(|s| plan(s, &revised));
    assert_eq!(p.requests[0].id, p2.requests[0].id);
    assert_ne!(p.requests[0].source_revision, p2.requests[0].source_revision);
    e.with_store_mut(|s| s.state.task.entities.get_mut("due").unwrap().remind_at = Some(o.now_ms + 60_000));
    assert_ne!(p.requests[0].id, e.with_store(|s| plan(s, &o)).requests[0].id);
}

#[test]
fn notification_reminder_order_uses_scheduled_time_before_identity_and_keeps_subtasks() {
    let (mut s, _dir) = store();
    let o = options("2026-09-16");
    task(&mut s, "a-later", "2026-09-16", Some(o.now_ms + 2000));
    task(&mut s, "z-earlier", "2026-09-16", Some(o.now_ms + 1000));
    s.state.task.entities.get_mut("z-earlier").unwrap().parent_id = Some("a-later".into());
    s.state.rest.insert(
        "archiveOld".into(),
        json!({"task": {"entities": {
            "archived": {"id": "archived", "remindAt": o.now_ms}
        }}}),
    );
    let p = plan(&s, &o);
    assert_eq!(p.requests.len(), 2);
    assert!(matches!(&p.requests[0].content, NotificationContent::Reminder { task_id, .. } if task_id == "z-earlier"));
    s.state.task.entities.get_mut("z-earlier").unwrap().title = "Edited title".into();
    let edited = plan(&s, &o);
    assert_eq!(edited.requests[0].id, p.requests[0].id);
    assert_ne!(edited.requests[0].content, p.requests[0].content);
}

#[test]
fn notification_dst_calendar_horizon_and_summary_id_follow_local_days() {
    // Each timezone gets a fresh process; TZ is never changed under concurrent tests.
    if std::env::var_os("MOMENTUM_NOTIFICATION_TZ_CHILD").is_none() {
        for zone in ["America/New_York", "UTC"] {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "notification_tests::notification_dst_calendar_horizon_and_summary_id_follow_local_days",
                    "--nocapture",
                ])
                .env("TZ", zone)
                .env("MOMENTUM_NOTIFICATION_TZ_CHILD", "1")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{zone}: {}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        return;
    }
    let new_york = std::env::var("TZ").unwrap() == "America/New_York";
    for (start, end, expected_hours) in [("2026-03-06", "2026-03-13", 167), ("2026-10-30", "2026-11-06", 169)] {
        let (s, _dir) = store();
        let o = options(start);
        let p = plan(&s, &o);
        assert_eq!(p.horizon_end_ms, local_ms(end, 0, 0).unwrap());
        assert_eq!(
            (p.horizon_end_ms - local_ms(start, 0, 0).unwrap()) / 3_600_000,
            if new_york { expected_hours } else { 168 }
        );
    }
    let (mut s, _dir) = store();
    task(&mut s, "spring", "2026-03-08", None);
    let mut o = options("2026-03-08");
    o.now_ms = local_ms("2026-03-08", 0, 0).unwrap();
    o.morning_summary_enabled = true;
    o.morning_summary_time = ClockTime { hour: 2, minute: 30 };
    let p = plan(&s, &o);
    assert_eq!(p.requests.len(), 1);
    assert_eq!(
        p.requests[0].scheduled_at_ms,
        local_ms(
            "2026-03-08",
            if new_york { 3 } else { 2 },
            if new_york { 0 } else { 30 }
        )
        .unwrap()
    );
    assert_eq!(day_of_ms(p.requests[0].fire_at_ms), "2026-03-08");
    if new_york {
        assert_eq!(time_of_ms(p.requests[0].fire_at_ms), (3, 0));
    }

    task(&mut s, "autumn", "2026-11-01", None);
    o.now_ms = local_ms("2026-11-01", 0, 0).unwrap();
    o.morning_summary_time = ClockTime { hour: 1, minute: 30 };
    let first = plan(&s, &o);
    let request = &first.requests[0];
    assert_eq!(request.id, "summary:2026-11-01");
    assert_eq!(
        request.scheduled_at_ms,
        local_ms("2026-11-01", 0, 0).unwrap() + 90 * 60_000,
        "the first 01:30 is eligible on a repeated-clock day"
    );
    o.now_ms = request.scheduled_at_ms + 3_600_000;
    let repeated = plan(&s, &o);
    assert_eq!(repeated.requests.len(), 1);
    assert_eq!(request.id, repeated.requests[0].id);
    assert_eq!(request.scheduled_at_ms, repeated.requests[0].scheduled_at_ms);
    s.meta.last_summary_day = "2026-11-01".into();
    assert!(plan(&s, &o).requests.is_empty());
}

#[test]
fn notification_consumed_overdue_identities_are_filtered_before_capacity_without_losing_future_requests() {
    let (mut s, _dir) = store();
    let mut o = options("2026-09-16");
    for n in 0..60 {
        task(&mut s, &format!("old-{n:02}"), "2026-09-15", Some(o.now_ms - 60_000));
    }
    o.consumed_ids = plan(&s, &o).requests.iter().map(|r| r.id.clone()).collect();
    task(&mut s, "future", "2026-09-16", Some(o.now_ms + 60_000));
    let before = json!([s.state, s.pending, s.meta]);
    let options_before = o.clone();
    let p = plan(&s, &o);
    assert_eq!(p.requests.len(), 1);
    assert_eq!(p.overflow_reminders, 0);
    assert!(matches!(&p.requests[0].content, NotificationContent::Reminder { task_id, .. } if task_id == "future"));
    assert_eq!(before, json!([s.state, s.pending, s.meta]));
    assert_eq!(o, options_before);
    assert_eq!(p, plan(&s, &o));
    o.now_ms += 60_000;
    o.consumed_ids.insert(p.requests[0].id.clone());
    assert!(plan(&s, &o).requests.is_empty());
    o.now_ms -= 120_000;
    assert!(
        plan(&s, &o).requests.is_empty(),
        "clock rollback cannot re-enable consumed reminders"
    );
}

#[test]
fn notification_consumed_summaries_do_not_spend_capacity_but_future_accepted_summaries_stay_planned() {
    let (mut s, _dir) = store();
    let mut o = options("2026-09-16");
    o.morning_summary_enabled = true;
    o.now_ms = local_ms("2026-09-16", 10, 0).unwrap();
    repeat(&mut s, "daily", "DAILY", "2026-09-16");
    o.consumed_ids = ["summary:2026-09-16".into()].into();
    let p = plan(&s, &o);
    assert_eq!(p.requests.len(), 4);
    assert_eq!(p.overflow_summaries, 2);
    assert_eq!(p.requests[0].id, "summary:2026-09-17");
    assert_eq!(p.requests[3].id, "summary:2026-09-20");
    o.morning_summary_time.hour = 11;
    let edited = plan(&s, &o);
    assert_eq!(edited.requests[0].id, "summary:2026-09-17");
    assert_eq!(edited.overflow_summaries, 2);
    assert!(
        edited.requests.iter().all(|r| r.id != "summary:2026-09-16"),
        "editing the clock later cannot re-deliver a consumed day's summary"
    );
}

#[test]
fn notification_projection_and_engine_recurrence_share_constructor_and_project_fallback() {
    for project_order in [["other", INBOX_PROJECT_ID], [INBOX_PROJECT_ID, "other"]] {
        for configured_project in [None, Some(""), Some("explicit")] {
            let dir = tempfile::tempdir().unwrap();
            let e = Engine::open(dir.path().to_string_lossy().into_owned());
            let today = today_str();
            e.with_store_mut(|s| {
                s.state.project.insert(
                    "other",
                    Project {
                        id: "other".into(),
                        ..Default::default()
                    },
                );
                s.state.project.ids = project_order.map(String::from).into();
                repeat(s, "repeat", "DAILY", &today);
                let cfg = s.state.task_repeat_cfg.entities.get_mut("repeat").unwrap();
                cfg.title = Some(" \tTrimmed title\n".into());
                cfg.project_id = configured_project.map(String::from);
            });
            let mut o = options(&today);
            o.morning_summary_enabled = true;
            let p = e.with_store(|s| plan(s, &o));
            let expected_id = format!("rpt_repeat_{today}");
            assert!(p.projected_occurrences.iter().any(|p| p.task_id == expected_id));
            assert_eq!(e.spawn_repeats(), 1);
            e.with_store(|s| {
                let actual = &s.state.task.entities[&expected_id];
                let cfg = &s.state.task_repeat_cfg.entities["repeat"];
                assert_eq!(*actual, cfg.task_for_day(&today, project_order[0], actual.created));
                assert_eq!(actual.title, "Trimmed title");
                assert_eq!(
                    actual.project_id,
                    configured_project.filter(|p| !p.is_empty()).unwrap_or(project_order[0])
                );
                assert!(actual.due_with_time.is_none() && actual.remind_at.is_none());
            });
        }
    }
}
