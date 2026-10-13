// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Feature tests for the data model: dates, repeat rules, Today and Overdue membership,
//! round-tripping unknown fields. Pure Rust, no toolkit, runs on any OS.
use super::*;

fn cfg(cycle: &str, every: u32, start: &str) -> RepeatCfg {
    RepeatCfg {
        repeat_cycle: cycle.into(),
        repeat_every: every,
        start_date: Some(start.into()),
        ..Default::default()
    }
}

// ---- dates -----------------------------------------------------------------

#[test]
fn civil_days_round_trip_across_years() {
    for days in (-40_000..80_000).step_by(37) {
        let (y, m, d) = day_from_days(days);
        assert_eq!(days_from_civil(y, m, d), days, "{y}-{m}-{d}");
        assert_eq!(day_number(&day_str(days)), Some(days));
    }
}
#[test]
fn weekday_matches_known_dates() {
    assert_eq!(weekday(days_from_civil(1970, 1, 1)), 4); // Thursday
    assert_eq!(weekday(days_from_civil(2000, 1, 1)), 6); // Saturday
    assert_eq!(weekday(days_from_civil(2026, 9, 14)), 1); // Monday
}
#[test]
fn month_lengths_and_leap_years() {
    assert_eq!(days_in_month(2024, 2), 29);
    assert_eq!(days_in_month(2025, 2), 28);
    assert_eq!(days_in_month(2100, 2), 28);
    assert_eq!(days_in_month(2000, 2), 29);
    assert_eq!(days_in_month(2026, 12), 31);
}
#[test]
fn parse_day_rejects_garbage() {
    assert_eq!(parse_day("2026-09-14"), Some((2026, 9, 14)));
    assert_eq!(parse_day("2026-9-4"), Some((2026, 9, 4)));
    assert!(parse_day("").is_none());
    assert!(parse_day("2026/09/14").is_none());
    assert!(parse_day("yesterday").is_none());
}
#[test]
fn nth_weekday_edge_cases() {
    // First Monday of Feb 2027 is the 1st; last Sunday of Feb 2027 is the 28th.
    assert_eq!(nth_weekday_of_month(2027, 2, 1, 1), 1);
    assert_eq!(nth_weekday_of_month(2027, 2, 0, -1), 28);
    // A fifth Friday does not exist in a 28-day month: the fourth is returned.
    assert!(nth_weekday_of_month(2027, 2, 5, 4) <= 28);
}
#[test]
fn local_time_helpers_agree_with_each_other() {
    let ms = local_ms("2026-03-10", 9, 5).unwrap();
    assert_eq!(day_of_ms(ms), "2026-03-10");
    assert_eq!(time_of_ms(ms), (9, 5));
    assert!(local_ms("nope", 9, 0).is_none());
    assert_eq!(today_str(), day_of_ms(now_ms()));
}

// ---- repeat rules ----------------------------------------------------------

#[test]
fn daily_every_n_days() {
    let c = cfg("DAILY", 3, "2026-09-01");
    assert!(c.matches_day("2026-09-01") && c.matches_day("2026-09-04") && c.matches_day("2026-09-07"));
    assert!(!c.matches_day("2026-09-02") && !c.matches_day("2026-08-31"));
}
#[test]
fn weekly_every_two_weeks_on_chosen_days() {
    let mut c = cfg("WEEKLY", 2, "2026-09-07"); // Monday
    c.monday = true;
    c.friday = true;
    assert!(c.matches_day("2026-09-07") && c.matches_day("2026-09-11"));
    assert!(!c.matches_day("2026-09-14"), "off week");
    assert!(c.matches_day("2026-09-21") && c.matches_day("2026-09-25"));
    assert!(!c.matches_day("2026-09-22"), "Tuesday not chosen");
}
#[test]
fn monthly_on_date_last_day_and_nth_weekday() {
    let on_31st = cfg("MONTHLY", 1, "2026-01-31");
    assert!(on_31st.matches_day("2026-02-28") && on_31st.matches_day("2026-04-30"));
    let mut last = cfg("MONTHLY", 1, "2026-01-15");
    last.monthly_last_day = true;
    assert!(last.matches_day("2026-02-28") && last.matches_day("2026-03-31") && !last.matches_day("2026-03-15"));
    let mut nth = cfg("MONTHLY", 2, "2026-01-01");
    nth.monthly_week_of_month = Some(-1);
    nth.monthly_weekday = Some(5); // last Friday, every second month
    assert!(nth.matches_day("2026-01-30") && nth.matches_day("2026-03-27"));
    assert!(!nth.matches_day("2026-02-27"), "off month");
    assert_eq!(nth.nth_weekday_anchor(), Some((-1, 5)));
    nth.monthly_weekday = Some(9);
    assert_eq!(
        nth.nth_weekday_anchor(),
        None,
        "weekday out of range disables the anchor"
    );
}
#[test]
fn yearly_clamps_february_29() {
    let c = cfg("YEARLY", 1, "2024-02-29");
    assert!(c.matches_day("2025-02-28") && c.matches_day("2028-02-29"));
    assert!(!c.matches_day("2025-03-01"));
    let every_two = cfg("YEARLY", 2, "2024-06-01");
    assert!(every_two.matches_day("2026-06-01") && !every_two.matches_day("2025-06-01"));
}
#[test]
fn unknown_cycle_never_matches() {
    assert!(!cfg("HOURLY", 1, "2026-01-01").matches_day("2026-01-01"));
    assert_eq!(cfg("HOURLY", 1, "2026-01-01").newest_due_day("2026-01-01"), None);
}
#[test]
fn for_task_defaults_to_weekdays_and_marks_today_created() {
    let mut t = Task::new("standup", INBOX_PROJECT_ID);
    t.tag_ids = vec![TODAY_TAG_ID.into(), "tag1".into()];
    t.time_estimate = 900_000.0;
    let c = RepeatCfg::for_task(&t);
    assert_eq!(c.repeat_cycle, "WEEKLY");
    assert_eq!(c.weekdays(), [false, true, true, true, true, true, false]);
    assert_eq!(
        c.tag_ids,
        vec!["tag1".to_string()],
        "the virtual Today tag is never stored"
    );
    assert_eq!(c.default_estimate, Some(900_000.0));
    assert_eq!(c.last_task_creation_day.as_deref(), Some(today_str().as_str()));
    assert_eq!(c.extra["quickSetting"], "CUSTOM");
}
#[test]
fn catch_up_never_exceeds_one_cycle_and_respects_creation_day() {
    let mut c = cfg("DAILY", 2, "2026-01-01");
    c.last_task_creation_day = Some("2026-09-01".into());
    // Sep 8 is 250 days after Jan 1 (even offset): the newest match up to the 9th.
    assert_eq!(c.newest_due_day("2026-09-09").as_deref(), Some("2026-09-08"));
    c.last_task_creation_day = Some("2026-09-08".into());
    assert_eq!(c.newest_due_day("2026-09-09"), None);
    let mut w = cfg("WEEKLY", 1, "2026-09-07");
    w.wednesday = true;
    w.last_task_creation_day = None;
    // Missed for a month: only the newest Wednesday is returned, nothing older than a cycle.
    assert_eq!(w.newest_due_day("2026-10-12").as_deref(), Some("2026-10-07"));
}

// ---- tasks, Today and Overdue -------------------------------------------------

#[test]
fn task_new_has_sane_defaults() {
    let t = Task::new("  Buy milk  ", "P1");
    assert_eq!(t.title, "Buy milk");
    assert_eq!(t.project_id, "P1");
    assert!(!t.is_done && t.due_day.is_none() && t.due_with_time.is_none() && t.remind_at.is_none());
    assert!(t.created > 0 && t.id.len() >= 20);
    assert!(t.plan_day().is_none() && !t.is_overdue("2030-01-01"));
}
#[test]
fn today_ids_keeps_stored_order_then_appends_stragglers_and_skips_subtasks() {
    let mut d = AppData::fresh();
    let today = today_str();
    let mut mk = |title: &str, due: bool, parent: Option<&str>| {
        let mut t = Task::new(title, INBOX_PROJECT_ID);
        t.due_day = due.then(|| today.clone());
        t.parent_id = parent.map(String::from);
        let id = t.id.clone();
        d.task.insert(&id.clone(), t);
        id
    };
    let a = mk("a", true, None);
    let b = mk("b", true, None);
    let c = mk("c", true, None);
    let _sub = mk("sub", true, Some(&a));
    let _later = mk("later", false, None);
    d.tag.entities.get_mut(TODAY_TAG_ID).unwrap().task_ids = vec![c.clone(), a.clone(), "ghost".into()];
    assert_eq!(
        d.today_ids(),
        vec![c, a, b],
        "Today order first, then other due tasks; ghosts and subtasks dropped"
    );
}
#[test]
fn timed_tasks_count_for_their_local_day() {
    let mut d = AppData::fresh();
    let mut t = Task::new("call", INBOX_PROJECT_ID);
    t.due_with_time = local_ms(&today_str(), 23, 59);
    t.due_day = Some("2000-01-01".into()); // ignored: dueWithTime wins
    let id = t.id.clone();
    d.task.insert(&id.clone(), t);
    assert_eq!(d.today_ids(), vec![id.clone()]);
    assert!(d.overdue_ids().is_empty());
    d.task.entities.get_mut(&id).unwrap().due_with_time = Some(local_ms("2020-01-01", 8, 0).unwrap());
    assert_eq!(d.overdue_ids(), vec![id]);
}
#[test]
fn overdue_is_oldest_first_and_ignores_done_and_subtasks() {
    let mut d = AppData::fresh();
    let mut mk = |title: &str, day: &str, done: bool, parent: Option<&str>| {
        let mut t = Task::new(title, INBOX_PROJECT_ID);
        t.due_day = Some(day.into());
        t.is_done = done;
        t.parent_id = parent.map(String::from);
        let id = t.id.clone();
        d.task.insert(&id.clone(), t);
        id
    };
    let newer = mk("newer", "2026-01-05", false, None);
    let older = mk("older", "2026-01-02", false, None);
    let _done = mk("done", "2026-01-01", true, None);
    let _sub = mk("sub", "2026-01-01", false, Some(&older));
    assert_eq!(d.overdue_ids(), vec![older, newer]);
}

// ---- persistence shape -------------------------------------------------------

#[test]
fn fresh_state_has_inbox_and_today() {
    let d = AppData::fresh();
    assert_eq!(d.project.ids, vec![INBOX_PROJECT_ID.to_string()]);
    assert_eq!(d.tag.ids, vec![TODAY_TAG_ID.to_string()]);
    assert!(d.task.ids.is_empty());
}
#[test]
fn backup_wrapper_and_raw_forms_both_load() {
    let raw = json!({"task": {"ids": [], "entities": {}}, "project": {"ids": [], "entities": {}},
        "tag": {"ids": [], "entities": {}}, "globalConfig": {"misc": {}}});
    let wrapped = json!({"data": raw.clone(), "timestamp": 1, "crossModelVersion": 4});
    for v in [raw, wrapped] {
        let d = AppData::from_backup(v).unwrap();
        assert!(
            d.rest.contains_key("globalConfig"),
            "unknown top-level slices are preserved"
        );
    }
}
#[test]
fn every_entity_preserves_unknown_fields() {
    let p: Project =
        serde_json::from_value(json!({"id":"p","title":"P","taskIds":[],"issueIntegrationCfgs":{"x":1}})).unwrap();
    assert_eq!(serde_json::to_value(&p).unwrap()["issueIntegrationCfgs"]["x"], 1);
    let g: Tag = serde_json::from_value(json!({"id":"t","title":"T","taskIds":[],"icon":"star"})).unwrap();
    assert_eq!(serde_json::to_value(&g).unwrap()["icon"], "star");
    let c: RepeatCfg =
        serde_json::from_value(json!({"id":"r","repeatCycle":"DAILY","repeatEvery":1,"order":7})).unwrap();
    assert_eq!(serde_json::to_value(&c).unwrap()["order"], 7);
    let d: AppData = serde_json::from_value(json!({"task":{"ids":[],"entities":{}},"boards":{"k":1}})).unwrap();
    assert_eq!(serde_json::to_value(&d).unwrap()["boards"]["k"], 1);
}
#[test]
fn optional_task_fields_are_omitted_when_absent() {
    let v = serde_json::to_value(Task::new("x", "p")).unwrap();
    for k in [
        "notes",
        "doneOn",
        "parentId",
        "modified",
        "dueDay",
        "dueWithTime",
        "remindAt",
        "repeatCfgId",
    ] {
        assert!(v.get(k).is_none(), "{k} should be absent");
    }
    assert_eq!(v["timeSpentOnDay"], json!({}));
    assert_eq!(v["attachments"], json!([]));
}
#[test]
fn entity_state_keeps_insertion_order_and_dedups() {
    let mut s: EntityState<Project> = EntityState::default();
    s.insert("b", Project::new("B"));
    s.insert("a", Project::new("A"));
    s.insert("b", Project::new("B2"));
    assert_eq!(s.ids, vec!["b", "a"]);
    assert_eq!(s.iter().map(|p| p.title.as_str()).collect::<Vec<_>>(), ["B2", "A"]);
    assert!(s.remove("b").is_some() && s.remove("b").is_none());
    assert_eq!(s.ids, vec!["a"]);
}
#[test]
fn project_color_comes_from_theme_primary() {
    let mut p = Project::new("P");
    assert!(p.color().is_some(), "new projects get a default theme colour");
    p.theme = json!({"primary": "#ff8800"});
    assert_eq!(p.color(), Some("#ff8800"));
    p.theme = json!(null);
    assert!(p.color().is_none());
}

#[test]
fn planned_ids_preserves_today_order_and_filters_by_requested_day() {
    let mut d = AppData::fresh();
    for (id, day) in [
        ("a", "2026-09-17"),
        ("b", "2026-09-17"),
        ("c", "2026-09-17"),
        ("other", "2026-09-18"),
        ("child", "2026-09-17"),
    ] {
        d.task.insert(
            id,
            Task {
                id: id.into(),
                due_day: Some(day.into()),
                ..Default::default()
            },
        );
    }
    d.task.entities.get_mut("child").unwrap().parent_id = Some("a".into());
    d.tag.entities.get_mut(TODAY_TAG_ID).unwrap().task_ids =
        vec!["b".into(), "a".into(), "other".into(), "child".into()];
    assert_eq!(d.planned_ids("2026-09-17"), vec!["b", "a", "c"]);
    assert_eq!(d.planned_ids("2026-09-18"), vec!["other"]);
    assert_eq!(d.today_ids(), d.planned_ids(&today_str()));
}

#[test]
fn repeat_task_for_day_is_pure_and_preserves_existing_constructor_semantics() {
    let mut c = cfg("DAILY", 1, "2026-09-16");
    c.id = "repeat".into();
    c.title = Some("  Recurring title\n".into());
    c.tag_ids = vec![TODAY_TAG_ID.into(), "morning".into()];
    c.default_estimate = Some(15_000.0);
    c.notes = Some("  notes retained  ".into());
    c.extra.insert("dueWithTime".into(), json!(123));
    c.extra.insert("remindAt".into(), json!(100));
    for (project, expected_project) in [(None, "fallback"), (Some(""), "fallback"), (Some("chosen"), "chosen")] {
        c.project_id = project.map(String::from);
        let task = c.task_for_day("2026-09-17", "fallback", 1234);
        assert_eq!(
            task,
            Task {
                id: "rpt_repeat_2026-09-17".into(),
                title: "Recurring title".into(),
                project_id: expected_project.into(),
                created: 1234,
                repeat_cfg_id: Some("repeat".into()),
                due_day: Some("2026-09-17".into()),
                time_estimate: 15_000.0,
                notes: Some("  notes retained  ".into()),
                tag_ids: vec!["morning".into()],
                ..Default::default()
            }
        );
        assert_eq!(task, c.task_for_day("2026-09-17", "fallback", 1234));
    }
    c.title = None;
    c.notes = Some(String::new());
    c.default_estimate = None;
    let task = c.task_for_day("2026-09-17", "fallback", 1234);
    assert!(task.title.is_empty());
    assert!(task.notes.is_none());
    assert_eq!(task.time_estimate, 0.0);
}

#[test]
fn planned_ids_retains_stored_duplicates_and_appends_each_straggler_once() {
    let mut d = AppData::fresh();
    for id in ["a", "b", "c", "done", "child"] {
        d.task.insert(
            id,
            Task {
                id: id.into(),
                due_day: Some("2026-09-16".into()),
                ..Default::default()
            },
        );
    }
    d.task.entities.get_mut("done").unwrap().is_done = true;
    d.task.entities.get_mut("child").unwrap().parent_id = Some("a".into());
    d.task.ids = ["c", "a", "c", "done", "b", "child"].map(String::from).into();
    d.tag.entities.get_mut(TODAY_TAG_ID).unwrap().task_ids = ["b", "b", "ghost", "a", "child"].map(String::from).into();
    assert_eq!(d.planned_ids("2026-09-16"), vec!["b", "b", "a", "c", "done"]);
}

#[test]
fn backup_rejects_unrelated_json_and_incomplete_task_slices() {
    for invalid in [
        json!({}),
        json!({"settings": {}}),
        json!({"data": {}}),
        json!({"task": {}}),
        json!({"task": {"ids": []}}),
        json!({"task": {"entities": {}}}),
    ] {
        assert!(
            AppData::from_backup(invalid.clone()).is_err(),
            "must not treat {invalid} as an empty backup"
        );
    }
    assert!(
        AppData::from_backup(json!({"task": {"ids": [], "entities": {}}})).is_ok(),
        "an explicitly empty task collection is a valid backup"
    );
}
