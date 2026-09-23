// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! The behaviours of MVP.md section 2, driven through the engine over the demo data. This
//! is the contract both apps rest on: the GTK window tests and the Swift unit tests check
//! that each toolkit renders what the engine says, and this file checks what it says.
use crate::*;
use sp_model::*;
use sp_oplog::Action;
use std::sync::Arc;

fn today_n() -> i64 {
    day_number(&today_str()).unwrap()
}
fn demo() -> (Arc<Engine>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let e = Engine::demo(dir.path().join("demo").to_string_lossy().into_owned());
    (e, dir)
}
fn empty() -> (Arc<Engine>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let e = Engine::open(dir.path().join("empty").to_string_lossy().into_owned());
    (e, dir)
}
fn id_of(e: &Engine, title: &str) -> String {
    e.with_store(|s| {
        s.state
            .task
            .iter()
            .find(|t| t.title == title)
            .map(|t| t.id.clone())
            .unwrap_or_else(|| panic!("no task {title}"))
    })
}
fn tag_id(e: &Engine, title: &str) -> String {
    e.with_store(|s| {
        s.state
            .tag
            .iter()
            .find(|t| t.title == title)
            .map(|t| t.id.clone())
            .unwrap_or_else(|| panic!("no tag {title}"))
    })
}
fn project_id(e: &Engine, title: &str) -> String {
    e.with_store(|s| {
        s.state
            .project
            .iter()
            .find(|p| p.title == title)
            .map(|p| p.id.clone())
            .unwrap()
    })
}
fn task(e: &Engine, id: &str) -> Task {
    e.with_store(|s| s.state.task.entities[id].clone())
}
fn has_task(e: &Engine, id: &str) -> bool {
    e.with_store(|s| s.state.task.entities.contains_key(id))
}
/// Task ids in listing order (subtasks included), like the GTK `row_ids` helper.
fn row_ids(l: &Listing) -> Vec<String> {
    l.sections
        .iter()
        .flat_map(|s| s.rows.iter())
        .filter_map(|r| match r {
            Row::Task { row } => Some(row.id.clone()),
            _ => None,
        })
        .collect()
}
fn kinds(l: &Listing) -> Vec<SectionKind> {
    l.sections.iter().map(|s| s.kind.clone()).collect()
}
fn sidebar_has(e: &Engine, v: View) -> bool {
    e.sidebar().fixed.iter().any(|x| x.view == v)
}

fn add_count_task(
    e: &Engine,
    title: &str,
    due_day: Option<String>,
    due_with_time: Option<u64>,
    is_done: bool,
    parent_id: Option<String>,
) -> String {
    let mut task = Task::new(title, INBOX_PROJECT_ID);
    task.due_day = due_day;
    task.due_with_time = due_with_time;
    task.is_done = is_done;
    let id = task.id.clone();
    match parent_id {
        Some(parent_id) => e.dispatch(Action::AddSubTask { task, parent_id }),
        None => e.dispatch(Action::AddTask { task, bottom: true }),
    }
    id
}

#[test]
fn task_count_is_zero_for_every_mode_in_an_empty_store() {
    let (e, _d) = empty();

    for mode in [
        TaskCountMode::DueToday,
        TaskCountMode::TodayIncludingOverdue,
        TaskCountMode::None,
    ] {
        assert_eq!(e.task_count(mode), 0);
    }
}

#[test]
fn task_count_due_today_includes_plain_today_tasks() {
    let (e, _d) = empty();
    add_count_task(&e, "Plain today", Some(today_str()), None, false, None);

    assert_eq!(e.task_count(TaskCountMode::DueToday), 1);
    assert_eq!(e.task_count(TaskCountMode::TodayIncludingOverdue), 1);
}

#[test]
fn task_count_due_today_includes_timed_today_tasks() {
    let (e, _d) = empty();
    add_count_task(
        &e,
        "Timed today",
        Some(day_str(today_n() - 1)),
        local_ms(&today_str(), 12, 0),
        false,
        None,
    );

    assert_eq!(e.task_count(TaskCountMode::DueToday), 1);
    assert_eq!(e.task_count(TaskCountMode::TodayIncludingOverdue), 1);
}

#[test]
fn task_count_includes_overdue_only_in_the_today_listing_mode() {
    let (e, _d) = empty();
    add_count_task(&e, "Overdue", Some(day_str(today_n() - 1)), None, false, None);

    assert_eq!(e.task_count(TaskCountMode::DueToday), 0);
    assert_eq!(e.task_count(TaskCountMode::TodayIncludingOverdue), 1);
}

#[test]
fn task_count_excludes_future_tasks() {
    let (e, _d) = empty();
    add_count_task(&e, "Future", Some(day_str(today_n() + 1)), None, false, None);

    assert_eq!(e.task_count(TaskCountMode::DueToday), 0);
    assert_eq!(e.task_count(TaskCountMode::TodayIncludingOverdue), 0);
}

#[test]
fn task_count_excludes_completed_tasks() {
    let (e, _d) = empty();
    add_count_task(&e, "Done today", Some(today_str()), None, true, None);

    assert_eq!(e.task_count(TaskCountMode::DueToday), 0);
    assert_eq!(e.task_count(TaskCountMode::TodayIncludingOverdue), 0);
}

#[test]
fn task_count_never_counts_subtasks_and_counts_the_parent_once() {
    let (e, _d) = empty();
    let parent = add_count_task(&e, "Parent", Some(today_str()), None, false, None);
    add_count_task(
        &e,
        "Child due today",
        Some(today_str()),
        None,
        false,
        Some(parent.clone()),
    );
    add_count_task(
        &e,
        "Overdue child",
        Some(day_str(today_n() - 1)),
        None,
        false,
        Some(parent),
    );

    assert_eq!(e.task_count(TaskCountMode::DueToday), 1);
    assert_eq!(e.task_count(TaskCountMode::TodayIncludingOverdue), 1);
}

#[test]
fn task_count_none_is_zero_for_a_populated_store() {
    let (e, _d) = empty();
    add_count_task(&e, "Today", Some(today_str()), None, false, None);
    add_count_task(&e, "Overdue", Some(day_str(today_n() - 1)), None, false, None);

    assert_eq!(e.task_count(TaskCountMode::None), 0);
}

#[test]
fn ordinary_task_lists_do_not_build_the_global_search_index() {
    let (e, _d) = demo();
    assert!(!e.has_search_index());

    let today = e.listing(View::Today, 100);
    assert!(!today.sections.is_empty());
    assert!(
        !e.has_search_index(),
        "Today should not parse and lowercase every live and archived task"
    );

    let _ = e.listing(View::Archive, 100);
    assert!(
        e.has_search_index(),
        "Archive still needs the cached archived projection"
    );
}

#[test]
fn today_view_groups_by_day_period_and_keeps_overdue_dates_visible() {
    let (e, _d) = demo();
    let l = e.listing(View::Today, 100);
    let k = kinds(&l);
    assert!(k.iter().all(|kind| *kind == SectionKind::Plain));
    assert_eq!(
        l.sections.iter().map(|s| s.group.clone()).collect::<Vec<_>>(),
        vec![
            Some(TaskGroup::Today),
            Some(TaskGroup::Morning),
            Some(TaskGroup::Evening)
        ]
    );
    let rows = row_ids(&l);
    assert_eq!(rows[0], id_of(&e, "Renew library books"), "overdue first");
    let tonight = id_of(&e, "Read two chapters");
    let dentist = id_of(&e, "Dentist appointment");
    assert!(rows.iter().position(|r| *r == dentist) < rows.iter().position(|r| *r == tonight));
    assert!(!k.contains(&SectionKind::Completed), "nothing done yet");
    assert!(l.empty.is_none() && l.all_done.is_none());
    assert_eq!(l.title, ViewTitle::Today);
    // Rows in a day view carry no day label unless overdue; the timed task shows its time.
    let dentist_row = l
        .sections
        .iter()
        .flat_map(|s| s.rows.iter())
        .find_map(|r| match r {
            Row::Task { row } if row.id == dentist => Some(row.clone()),
            _ => None,
        })
        .unwrap();
    assert!(dentist_row.day.is_none());
    assert_eq!(dentist_row.time, Some(ClockTime { hour: 15, minute: 30 }));
    assert!(dentist_row.reminder.is_some());
    assert_eq!(dentist_row.project.as_ref().map(|p| p.title.as_str()), Some("Home"));
    let overdue_row = match &l.sections[0].rows[0] {
        Row::Task { row } => row.clone(),
        _ => panic!(),
    };
    assert!(overdue_row.day.is_some(), "overdue rows say which day");
}

#[test]
fn quick_add_parses_tags_and_estimate_and_plans_for_the_current_view() {
    let (e, _d) = demo();
    assert!(e.add_task("Buy milk #home 30m".into(), View::Today).changed);
    let id = id_of(&e, "Buy milk");
    let t = task(&e, &id);
    assert_eq!(t.due_day.as_deref(), Some(today_str().as_str()), "added from Today");
    assert_eq!(t.time_estimate, 1_800_000.0);
    let home = tag_id(&e, "home");
    assert_eq!(t.tag_ids, vec![home]);
    assert!(row_ids(&e.listing(View::Today, 100)).contains(&id));
    assert!(!e.add_task("   ".into(), View::Today).changed);
    e.with_store(|s| assert!(s.state.task.iter().all(|t| !t.title.is_empty())));
    // The `#` popover: completions by prefix, then the text rewritten around the cursor.
    let c = e.tag_completions("ho".into());
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].title, "home");
    assert_eq!(c[0].task_count, 1);
    let done = complete_hash_word("Call #ho".into(), 8, "home".into()).unwrap();
    assert_eq!(done.text, "Call #home ");
}

#[test]
fn adding_from_tonight_tags_evening_and_from_a_tag_view_applies_that_tag() {
    let (e, _d) = demo();
    e.add_task("Wind down".into(), View::Tonight);
    let id = id_of(&e, "Wind down");
    assert!(row_ids(&e.listing(View::Tonight, 100)).contains(&id));
    assert!(task(&e, &id).tag_ids.contains(&tag_id(&e, "Evening")));
    let urgent = tag_id(&e, "urgent");
    e.add_task("Fire drill".into(), View::tag(&urgent));
    let id = id_of(&e, "Fire drill");
    assert!(task(&e, &id).tag_ids.contains(&urgent));
    assert!(row_ids(&e.listing(View::tag(&urgent), 100)).contains(&id));
    // Morning too, and a task added from a project goes to that project.
    e.add_task("Coffee first".into(), View::Morning);
    let coffee = id_of(&e, "Coffee first");
    assert!(task(&e, &coffee).tag_ids.contains(&tag_id(&e, "Morning")));
    let work = project_id(&e, "Momentum");
    e.add_task("Ship it".into(), View::project(&work));
    assert_eq!(task(&e, &id_of(&e, "Ship it")).project_id, work);
}

#[test]
fn completing_moves_to_completed_section_and_undo_reverts() {
    let (e, _d) = demo();
    let id = id_of(&e, "Write release notes for 0.1");
    let out = e.set_done(id.clone(), true);
    assert_eq!(out.message, Some(Message::TaskCompleted));
    assert!(task(&e, &id).is_done);
    let l = e.listing(View::Today, 100);
    let completed = l.sections.iter().find(|s| s.kind == SectionKind::Completed).unwrap();
    assert_eq!(completed.count, 1);
    assert_eq!(row_ids(&l).last(), Some(&id), "completed tasks sink to the bottom");
    assert_eq!(e.undo().message, Some(Message::Undone));
    assert!(!task(&e, &id).is_done);
    assert!(!kinds(&e.listing(View::Today, 100)).contains(&SectionKind::Completed));
    // A toast's Undo button undoes that batch by id, even after later changes.
    let out = e.set_done(id.clone(), true);
    let other = id_of(&e, "Read two chapters");
    e.set_done(other.clone(), true);
    assert!(e.undo_batch(out.undo.unwrap()).changed);
    assert!(!task(&e, &id).is_done && task(&e, &other).is_done);
    assert!(!e.undo_batch(out.undo.unwrap()).changed, "a batch undoes once");
}

#[test]
fn all_done_panel_appears_when_every_task_is_complete() {
    let (e, _d) = empty();
    e.add_task("only one".into(), View::Today);
    let id = id_of(&e, "only one");
    e.set_done(id, true);
    let l = e.listing(View::Today, 100);
    assert_eq!(l.all_done, Some(AllDone::Today { completed: 1 }));
    assert!(l.can_archive && e.can_archive());
    let (e2, _d2) = empty();
    assert!(!e2.listing(View::Today, 100).can_archive);
    assert_eq!(e2.listing(View::Today, 100).empty, Some(EmptyState::Today));
}

#[test]
fn archive_completed_moves_tasks_and_archive_view_lists_them() {
    let (e, _d) = demo();
    let a = id_of(&e, "Write release notes for 0.1");
    let b = id_of(&e, "Read two chapters");
    e.set_done(a.clone(), true);
    e.set_done(b.clone(), true);
    let out = e.archive_done();
    assert_eq!(out.message, Some(Message::Archived { n: 2 }));
    assert!(out.sync_now, "archiving syncs at once");
    assert!(!has_task(&e, &a) && !has_task(&e, &b));
    e.with_store(|s| assert_eq!(s.state.rest["archiveYoung"]["task"]["ids"].as_array().unwrap().len(), 2));
    let l = e.listing(View::Archive, 100);
    let rows = row_ids(&l);
    assert!(rows.contains(&a) && rows.contains(&b));
    let first = match &l.sections[0].rows[0] {
        Row::Task { row } => row.clone(),
        _ => panic!(),
    };
    assert!(first.archived && first.done_day.is_some());
    e.undo();
    assert!(has_task(&e, &a), "restore brings it back");
    assert!(!task(&e, &a).is_done);
    assert!(!e.archive_done().changed, "nothing to archive");
}

#[test]
fn reference_titles_cover_live_and_both_archive_tiers_without_enabling_edits() {
    let (e, _d) = empty();
    e.add_task("Live parent".into(), View::Today);
    let live = id_of(&e, "Live parent");
    let young = Task::new("Young parent", INBOX_PROJECT_ID);
    let old = Task::new("Old parent", INBOX_PROJECT_ID);
    let mut duplicate = young.clone();
    duplicate.title = "Older duplicate".into();
    let mut stale_live = task(&e, &live);
    stale_live.title = "Stale live title".into();
    e.with_store_mut(|s| {
        s.state.rest.insert(
            "archiveYoung".into(),
            serde_json::json!({"task": {"entities": {
                &young.id: &young, &live: stale_live, "invalid": {"title": 42}
            }}}),
        );
        s.state.rest.insert(
            "archiveOld".into(),
            serde_json::json!({"task": {"entities": {
                &old.id: &old, &young.id: duplicate
            }}}),
        );
    });
    let before = e.with_store(|s| serde_json::to_value(&s.state).unwrap());
    let can_undo = e.can_undo();
    let titles = e.task_reference_titles(vec![
        live.clone(),
        young.id.clone(),
        old.id.clone(),
        young.id.clone(),
        "missing".into(),
        "invalid".into(),
    ]);
    assert_eq!(titles.len(), 3);
    assert_eq!(titles.get(&live).map(String::as_str), Some("Live parent"));
    assert_eq!(titles.get(&young.id).map(String::as_str), Some("Young parent"));
    assert_eq!(titles.get(&old.id).map(String::as_str), Some("Old parent"));
    assert!(e.task_reference_titles(vec![]).is_empty());
    assert_eq!(
        e.task_title(young.id.clone()),
        None,
        "existing live-only API is unchanged"
    );
    assert!(e.task_detail(old.id.clone()).is_none(), "archive stays read-only");
    assert!(!e.set_done(old.id, false).changed);
    assert_eq!(e.with_store(|s| serde_json::to_value(&s.state).unwrap()), before);
    assert_eq!(e.can_undo(), can_undo);
}

#[test]
fn archive_view_pages_in_batches() {
    let (e, _d) = empty();
    let mut tasks = vec![];
    for i in 0..230 {
        let mut t = Task::new(&format!("old {i}"), INBOX_PROJECT_ID);
        t.is_done = true;
        t.done_on = Some(sp_model::now_ms() - i);
        tasks.push(t);
    }
    e.dispatch(Action::MoveToArchive {
        tasks,
        sub_tasks: vec![],
    });
    let l = e.listing(View::Archive, 100);
    assert_eq!(row_ids(&l).len(), 100);
    assert_eq!(l.more_available, 130);
    let l = e.listing(View::Archive, 300);
    assert_eq!(row_ids(&l).len(), 230);
    assert_eq!(l.more_available, 0);
    let newest = e.with_store(|s| {
        s.state.rest["archiveYoung"]["task"]["ids"][0]
            .as_str()
            .unwrap()
            .to_string()
    });
    assert_eq!(row_ids(&l)[0], newest, "newest completion first");
}

#[test]
fn archive_view_combines_both_tiers_in_completion_order() {
    let (e, _d) = empty();
    let mut young = Task::new("Young archive task", INBOX_PROJECT_ID);
    young.is_done = true;
    young.done_on = Some(200);
    let mut old = Task::new("Old archive task", INBOX_PROJECT_ID);
    old.is_done = true;
    old.done_on = Some(100);
    e.with_store_mut(|s| {
        s.state.rest.insert(
            "archiveYoung".into(),
            serde_json::json!({"task": {"entities": {&young.id: &young}}}),
        );
        s.state.rest.insert(
            "archiveOld".into(),
            serde_json::json!({"task": {"entities": {&old.id: &old}}}),
        );
    });

    let archive = e.listing(View::Archive, 100);
    assert_eq!(row_ids(&archive), [young.id.clone(), old.id.clone()]);
    assert!(archive
        .sections
        .iter()
        .flat_map(|section| &section.rows)
        .all(|row| matches!(row, Row::Task { row } if row.archived)));
    assert!(e.task_detail(young.id).is_none());
    assert!(e.task_detail(old.id).is_none());
}

#[test]
fn bulk_done_today_and_delete_with_undo() {
    let (e, _d) = demo();
    let a = id_of(&e, "Plan weekend hike");
    let home = task(&e, &a).project_id.clone();
    let b = e.with_store(|s| {
        s.state
            .task
            .iter()
            .find(|t| t.project_id == home && t.due_day.as_deref() > Some(today_str().as_str()))
            .map(|t| t.id.clone())
            .expect("an upcoming Home task")
    });
    let out = e.plan_for_today(vec![a.clone(), b.clone()]);
    assert_eq!(out.message, Some(Message::TasksPlannedForToday { n: 2 }));
    e.with_store(|s| assert!(s.state.today_ids().contains(&a) && s.state.today_ids().contains(&b)));
    assert!(!e.plan_for_today(vec![a.clone()]).changed, "already today");
    let out = e.bulk_done(vec![a.clone()]);
    assert_eq!(out.message, Some(Message::TasksCompleted { n: 1 }));
    assert!(task(&e, &a).is_done);
    let out = e.bulk_delete(vec![b.clone()]);
    assert_eq!(out.message, Some(Message::TasksDeleted { n: 1 }));
    assert!(!has_task(&e, &b));
    e.undo();
    assert!(has_task(&e, &b), "bulk delete is undoable");
    // Bulk tag and bulk move to project.
    let urgent = tag_id(&e, "urgent");
    let out = e.add_tag_to(vec![a.clone(), b.clone()], urgent.clone());
    assert_eq!(out.message, Some(Message::TasksTagged { n: 2 }));
    assert!(task(&e, &b).tag_ids.contains(&urgent));
    let out = e.add_tag_by_name(vec![b.clone()], "brand new".into());
    assert_eq!(
        out.message,
        Some(Message::Tagged {
            name: "brand new".into()
        })
    );
    assert!(task(&e, &b).tag_ids.contains(&tag_id(&e, "brand new")));
    let work = project_id(&e, "Momentum");
    let out = e.move_to_project(vec![a.clone(), b.clone()], work.clone());
    assert_eq!(
        out.message,
        Some(Message::MovedToProject {
            name: "Momentum".into()
        })
    );
    assert_eq!(task(&e, &a).project_id, work);
    e.undo();
    assert_eq!(task(&e, &a).project_id, home);
}

#[test]
fn day_moves_tomorrow_next_week_tonight_and_plan_today() {
    let (e, _d) = demo();
    let id = id_of(&e, "Write release notes for 0.1");
    let out = e.move_to_tomorrow(vec![id.clone()]);
    assert_eq!(out.message, Some(Message::MovedToTomorrow));
    let due = |e: &Engine| task(e, &id).due_day.clone();
    assert_eq!(due(&e).as_deref(), Some(day_str(today_n() + 1).as_str()));
    assert!(
        !row_ids(&e.listing(View::Today, 100)).contains(&id),
        "no longer in Today"
    );
    assert!(
        !e.move_to_tomorrow(vec![id.clone()]).changed,
        "already tomorrow: skipped"
    );
    let next_week = e.move_to_next_week(vec![id.clone()]);
    let monday = day_number(&due(&e).unwrap()).unwrap();
    assert_eq!(weekday(monday), 1);
    assert!(monday > today_n() && monday <= today_n() + 7);
    if next_week.changed {
        e.undo();
    }
    assert_eq!(
        due(&e).as_deref(),
        Some(day_str(today_n() + 1).as_str()),
        "undo restores the previous day; on Sunday both actions already mean Monday"
    );
    e.plan_for_today(vec![id.clone()]);
    assert!(row_ids(&e.listing(View::Today, 100)).contains(&id));
    let out = e.toggle_slot(vec![id.clone()], Slot::Tonight);
    assert_eq!(out.message, Some(Message::MovedToTonight));
    let evening = tag_id(&e, "Evening");
    assert!(task(&e, &id).tag_ids.contains(&evening));
    assert!(row_ids(&e.listing(View::Tonight, 100)).contains(&id));
    let out = e.toggle_slot(vec![id.clone()], Slot::Tonight);
    assert_eq!(out.message, Some(Message::MovedToToday));
    assert!(!task(&e, &id).tag_ids.contains(&evening));
    // Ctrl+T toggles: on today → off today, with undo.
    let out = e.toggle_today(id.clone());
    assert_eq!(out.message, Some(Message::RemovedFromToday));
    assert!(task(&e, &id).due_day.is_none());
    e.undo();
    assert_eq!(due(&e).as_deref(), Some(today_str().as_str()));
    // A time is cleared by a day move.
    let dentist = id_of(&e, "Dentist appointment");
    e.move_to_tomorrow(vec![dentist.clone()]);
    assert!(task(&e, &dentist).due_with_time.is_none());
    e.undo();
    assert!(task(&e, &dentist).due_with_time.is_some(), "undo restores the time");
}

#[test]
fn drop_targets_move_between_projects_tags_and_today() {
    let (e, _d) = demo();
    let id = id_of(&e, "Plan weekend hike");
    let home = project_id(&e, "Home");
    let work = project_id(&e, "Momentum");
    let urgent = tag_id(&e, "urgent");
    assert!(e.drop_tasks(vec![id.clone()], View::project(&work)).changed);
    assert!(
        !e.drop_tasks(vec![id.clone()], View::project(&work)).changed,
        "already there"
    );
    e.with_store(|s| {
        assert_eq!(s.state.task.entities[&id].project_id, work);
        assert!(!s.state.project.entities[&home].task_ids.contains(&id));
        assert!(s.state.project.entities[&work].task_ids.contains(&id));
    });
    let out = e.drop_tasks(vec![id.clone()], View::tag(&urgent));
    assert_eq!(out.message, Some(Message::Tagged { name: "urgent".into() }));
    e.with_store(|s| assert!(s.state.tag.entities[&urgent].task_ids.contains(&id)));
    assert_eq!(
        e.drop_tasks(vec![id.clone()], View::Today).message,
        Some(Message::PlannedForToday)
    );
    e.with_store(|s| assert!(s.state.today_ids().contains(&id)));
    assert_eq!(
        e.drop_tasks(vec![id.clone()], View::Tonight).message,
        Some(Message::PlannedForTonight)
    );
    assert!(task(&e, &id).tag_ids.contains(&tag_id(&e, "Evening")));
    assert!(
        !e.drop_tasks(vec![id.clone()], View::Tonight).changed,
        "already tonight"
    );
    assert!(e.drop_tasks(vec![id.clone()], View::Search).changed == false);
}

#[test]
fn manual_reorder_and_nudge_change_the_stored_order() {
    let (e, _d) = empty();
    for t in ["one", "two", "three"] {
        e.add_task(t.into(), View::Today);
    }
    let (a, b, c) = (id_of(&e, "one"), id_of(&e, "two"), id_of(&e, "three"));
    let ids = |e: &Engine| row_ids(&e.listing(View::Today, 100));
    assert_eq!(ids(&e), vec![a.clone(), b.clone(), c.clone()]);
    assert!(e.reorder(c.clone(), a.clone(), View::Today).changed);
    assert_eq!(ids(&e), vec![c.clone(), a.clone(), b.clone()]);
    assert!(
        !e.reorder(c.clone(), c.clone(), View::Today).changed,
        "dropping on itself is a no-op"
    );
    e.undo();
    assert_eq!(
        ids(&e),
        vec![a.clone(), b.clone(), c.clone()],
        "a drag reorder is undoable"
    );
    assert!(e.nudge(c.clone(), -1, View::Today).changed);
    assert_eq!(ids(&e), vec![a.clone(), c.clone(), b.clone()]);
    assert!(e.nudge(c.clone(), 1, View::Today).changed);
    assert_eq!(ids(&e), vec![a.clone(), b.clone(), c.clone()]);
    assert!(!e.nudge(a.clone(), -1, View::Today).changed, "already first");
    e.set_preferences(Preferences {
        sort: SortKey::Title,
        ..Preferences::default()
    });
    let out = e.reorder(c.clone(), a.clone(), View::Today);
    assert!(!out.changed && out.message == Some(Message::ManualOrderOnly));
    assert_eq!(e.nudge(c, 1, View::Today).message, Some(Message::ManualOrderOnly));
}

#[test]
fn dragging_a_selection_reorders_once_and_one_undo_restores_every_task() {
    let (e, _d) = empty();
    for title in ["a", "b", "c", "d"] {
        e.add_task(title.into(), View::Today);
    }
    let original = row_ids(&e.listing(View::Today, 100));
    let (a, b, c, d) = (
        original[0].clone(),
        original[1].clone(),
        original[2].clone(),
        original[3].clone(),
    );
    let out = e.reorder_tasks(vec![d.clone(), b.clone(), b.clone()], a.clone(), View::Today);
    assert!(out.changed);
    assert_eq!(
        row_ids(&e.listing(View::Today, 100)),
        vec![b.clone(), d.clone(), a.clone(), c.clone()]
    );
    e.undo();
    assert_eq!(row_ids(&e.listing(View::Today, 100)), original);
    assert!(!e.can_undo(), "one drag creates one undo batch");
    assert!(
        !e.reorder_tasks(vec![b.clone(), d.clone()], b, View::Today).changed,
        "drop within the selection is a no-op"
    );
    let pending = e.pending_count();
    assert!(!e.reorder_tasks(vec!["unknown".into()], a.clone(), View::Today).changed);
    assert!(!e.reorder_tasks(vec![a], c, View::Search).changed);
    assert_eq!(e.pending_count(), pending, "invalid drops emit no operations");
}

#[test]
fn dropping_a_future_slot_task_always_moves_into_the_slot_and_undo_restores_it() {
    let (e, _d) = demo();
    let id = id_of(&e, "Read two chapters");
    e.move_to_tomorrow(vec![id.clone()]);
    let before = task(&e, &id);
    assert!(e.drop_tasks(vec![id.clone()], View::Tonight).changed);
    assert!(
        row_ids(&e.listing(View::Tonight, 100)).contains(&id),
        "drop keeps the Evening tag and plans today"
    );
    e.undo();
    let restored = task(&e, &id);
    assert_eq!(restored.due_day, before.due_day);
    assert_eq!(restored.tag_ids, before.tag_ids);
}

#[test]
fn day_drops_clear_scheduled_times_and_reminders_and_undo_restores_both() {
    for destination in [View::Today, View::Morning, View::Tonight] {
        let (e, _d) = demo();
        let id = id_of(&e, "Dentist appointment");
        let original = task(&e, &id);
        assert!(original.due_with_time.is_some() && original.remind_at.is_some());
        assert!(e.drop_tasks(vec![id.clone()], destination.clone()).changed);
        let moved = task(&e, &id);
        assert_eq!(moved.due_day.as_deref(), Some(today_str().as_str()));
        assert!(
            moved.due_with_time.is_none() && moved.remind_at.is_none(),
            "{destination:?}"
        );
        e.undo();
        let restored = task(&e, &id);
        assert_eq!(restored.due_with_time, original.due_with_time);
        assert_eq!(restored.remind_at, original.remind_at);
        assert_eq!(restored.tag_ids, original.tag_ids);
    }
}

#[test]
fn a_future_timed_task_dropped_on_today_keeps_its_schedule_in_undo() {
    let (e, _d) = demo();
    let id = id_of(&e, "Dentist appointment");
    let time = local_ms(&day_str(today_n() + 2), 15, 30).unwrap();
    e.dispatch(Action::UpdateTask {
        id: id.clone(),
        changes: [
            ("dueWithTime".into(), serde_json::json!(time)),
            ("remindAt".into(), serde_json::json!(time - 60_000)),
        ]
        .into_iter()
        .collect(),
    });
    assert!(e.plan_for_today(vec![id.clone()]).changed);
    e.undo();
    let restored = task(&e, &id);
    assert_eq!(restored.due_with_time, Some(time));
    assert_eq!(restored.remind_at, Some(time - 60_000));
    assert!(restored.due_day.is_none());
}

#[test]
fn day_moves_clear_obsolete_reminders_and_restore_them_with_undo() {
    for next_week in [false, true] {
        let (e, _d) = demo();
        let id = id_of(&e, "Dentist appointment");
        let original = task(&e, &id);
        if next_week {
            e.move_to_next_week(vec![id.clone()]);
        } else {
            e.move_to_tomorrow(vec![id.clone()]);
        }
        assert!(task(&e, &id).remind_at.is_none());
        e.undo();
        assert_eq!(task(&e, &id).remind_at, original.remind_at);
        assert_eq!(task(&e, &id).due_with_time, original.due_with_time);
    }
}

#[test]
fn selection_reorder_respects_descending_display_and_rejects_subtasks() {
    let (e, _d) = empty();
    for title in ["a", "b", "c", "d"] {
        e.add_task(title.into(), View::Today);
    }
    let (a, b, c, d) = (id_of(&e, "a"), id_of(&e, "b"), id_of(&e, "c"), id_of(&e, "d"));
    e.set_preferences(Preferences {
        direction: SortDirection::Descending,
        ..Preferences::default()
    });
    assert!(
        e.reorder_tasks(vec![a.clone(), b.clone()], d.clone(), View::Today)
            .changed
    );
    assert_eq!(
        row_ids(&e.listing(View::Today, 100)),
        vec![b.clone(), a.clone(), d.clone(), c.clone()]
    );
    e.undo();
    assert_eq!(row_ids(&e.listing(View::Today, 100)), vec![d, c, b, a.clone()]);
    e.add_subtask(a.clone(), "child".into());
    let child = id_of(&e, "child");
    let pending = e.pending_count();
    assert!(!e.reorder(child, a, View::project(INBOX_PROJECT_ID)).changed);
    assert_eq!(e.pending_count(), pending);
}

#[test]
fn slot_reorder_undo_restores_position_among_hidden_tasks() {
    let (e, _d) = empty();
    for (title, view) in [
        ("a", View::Tonight),
        ("hidden", View::Today),
        ("b", View::Tonight),
        ("c", View::Tonight),
    ] {
        e.add_task(title.into(), view);
    }
    let original = e.with_store(|s| s.state.today_ids());
    let (a, b, c) = (id_of(&e, "a"), id_of(&e, "b"), id_of(&e, "c"));
    assert!(e.reorder_tasks(vec![b, c], a, View::Tonight).changed);
    e.undo();
    assert_eq!(e.with_store(|s| s.state.today_ids()), original);
}

#[test]
fn sorting_by_title_estimate_and_direction() {
    let (e, _d) = demo();
    let section = |e: &Engine| -> Vec<Task> {
        let l = e.listing(View::Today, 100);
        let s = l.sections.iter().find(|s| s.group == Some(TaskGroup::Today)).unwrap();
        s.rows
            .iter()
            .filter_map(|r| match r {
                Row::Task { row } => Some(task(e, &row.id)),
                _ => None,
            })
            .collect()
    };
    e.set_preferences(Preferences {
        sort: SortKey::Title,
        ..Preferences::default()
    });
    let titles: Vec<String> = section(&e).iter().map(|t| t.title.to_lowercase()).collect();
    let mut sorted = titles.clone();
    sorted.sort();
    assert!(titles.len() >= 3 && titles == sorted, "{titles:?}");
    e.set_preferences(Preferences {
        sort: SortKey::Title,
        direction: SortDirection::Descending,
        ..Preferences::default()
    });
    let titles: Vec<String> = section(&e).iter().map(|t| t.title.to_lowercase()).collect();
    let mut desc = titles.clone();
    desc.sort_by(|a, b| b.cmp(a));
    assert_eq!(titles, desc);
    e.set_preferences(Preferences {
        sort: SortKey::Estimate,
        ..Preferences::default()
    });
    let ests: Vec<f64> = section(&e).iter().map(|t| t.time_estimate).collect();
    assert!(ests.windows(2).all(|w| w[0] <= w[1]), "{ests:?}");
    e.set_preferences(Preferences {
        sort: SortKey::Due,
        ..Preferences::default()
    });
    let work = project_id(&e, "Momentum");
    let l = e.listing(View::project(&work), 100);
    let days: Vec<Option<String>> = row_ids(&l).iter().map(|i| task(&e, i).plan_day()).collect();
    let dated: Vec<&Option<String>> = days.iter().filter(|d| d.is_some()).collect();
    assert!(dated.windows(2).all(|w| w[0] <= w[1]), "{days:?}");
    assert!(days.last().unwrap().is_none(), "undated tasks last");
}

#[test]
fn coming_up_shows_dates_on_rows_and_honours_the_range() {
    let (e, _d) = demo();
    let mut far = Task::new("far away", INBOX_PROJECT_ID);
    far.due_day = Some(day_str(today_n() + 20));
    let far_id = far.id.clone();
    e.dispatch(Action::AddTask {
        task: far,
        bottom: true,
    });
    let l = e.listing(View::Upcoming, 100);
    let rows = row_ids(&l);
    assert!(!rows.is_empty() && !rows.contains(&far_id), "7-day window");
    assert!(l.sections.iter().all(|s| s.kind == SectionKind::Plain));
    let task_rows: Vec<_> = l
        .sections
        .iter()
        .flat_map(|s| &s.rows)
        .filter_map(|r| match r {
            Row::Task { row } => Some(row),
            _ => None,
        })
        .collect();
    assert!(
        task_rows.iter().all(|row| row.day.is_some()),
        "due dates remain visible without date headings"
    );
    assert_eq!(
        task_rows
            .iter()
            .filter(|row| row.day.as_ref().unwrap().relation == DayRelation::Tomorrow)
            .count(),
        2
    );
    e.set_preferences(Preferences {
        upcoming_days: 30,
        ..Preferences::default()
    });
    assert!(row_ids(&e.listing(View::Upcoming, 100)).contains(&far_id));
    let (empty_e, _d2) = empty();
    assert_eq!(
        empty_e.listing(View::Upcoming, 100).empty,
        Some(EmptyState::Upcoming { days: 7 })
    );
}

#[test]
fn search_finds_tasks_notes_projects_and_tags() {
    let (e, _d) = demo();
    e.add_task_with_notes("Quiet title".into(), Some("needle in the notes".into()), None);
    let noted = id_of(&e, "Quiet title");
    assert_eq!(task(&e, &noted).notes.as_deref(), Some("needle in the notes"));
    let l = e.search("orca".into());
    assert!(row_ids(&l).contains(&id_of(&e, "Test with Orca and high contrast")));
    assert_eq!(l.sections[0].kind, SectionKind::SearchTasks);
    assert!(
        row_ids(&e.search("needle".into())).contains(&noted),
        "notes are searched"
    );
    let l = e.search("gnome".into());
    assert!(
        l.sections.iter().any(|s| s.kind == SectionKind::SearchTags),
        "tag results are listed"
    );
    let l = e.search("home".into());
    assert!(l.sections.iter().any(|s| s.kind == SectionKind::SearchProjects));
    let l = e.search("zzz-nothing".into());
    assert!(row_ids(&l).is_empty());
    assert_eq!(l.empty, Some(EmptyState::NoResults));
    assert_eq!(e.search("   ".into()).empty, Some(EmptyState::Search));
    // Every word must match; the archive is searched too, capped at 30.
    assert!(row_ids(&e.search("orca contrast".into())).len() == 1);
    assert!(row_ids(&e.search("orca banana".into())).is_empty());
    let a = id_of(&e, "Write release notes for 0.1");
    e.set_done(a.clone(), true);
    e.archive_done();
    let l = e.search("release".into());
    assert!(l.sections.iter().any(|s| s.kind == SectionKind::SearchArchived));
    // Desktop search: open tasks by title only, plus the caller's "create" entry.
    let q = e.quick_matches("orca".into());
    assert_eq!(q.len(), 1);
    assert!(e.quick_matches("zzz".into()).is_empty());
    assert!(e.quick_matches("needle".into()).is_empty(), "titles only");
}

#[test]
fn search_caps_results_and_notes_the_rest() {
    let (e, _d) = empty();
    for i in 0..70 {
        e.add_task(format!("bulk item {i}"), View::Search);
    }
    let l = e.search("bulk".into());
    assert_eq!(l.sections[0].rows.len(), 60);
    assert_eq!(
        l.sections[0].note,
        Some(SectionNote {
            shown: 60,
            total: 70,
            suggest_narrowing: true
        })
    );
}

#[test]
fn context_menu_facts_for_a_task() {
    let (e, _d) = demo();
    let today_task = id_of(&e, "Write release notes for 0.1");
    let later = id_of(&e, "Plan weekend hike");
    let m = e.task_menu(today_task).unwrap();
    assert!(m.planned_today && m.slot.is_none() && m.top_level && !m.repeats && !m.is_done);
    let m = e.task_menu(later).unwrap();
    assert!(!m.planned_today);
    let m = e.task_menu(id_of(&e, "Read two chapters")).unwrap();
    assert_eq!(m.slot, Some(Slot::Tonight));
    assert!(e.task_menu("nope".into()).is_none());
}

#[test]
fn task_draft_builds_a_task_with_time_reminder_estimate_and_tags() {
    let (e, _d) = demo();
    let draft = TaskDraft {
        title: "Dentist follow-up".into(),
        project_id: INBOX_PROJECT_ID.into(),
        due_day: Some(today_str()),
        time: Some(ClockTime { hour: 14, minute: 30 }),
        reminder_minutes_before: Some(15),
        estimate_ms: 2_700_000.0,
        notes: String::new(),
        tag_ids: vec![],
        new_tags: vec!["health".into(), "urgent".into()],
    };
    assert!(e.create_task(draft.clone(), View::Today).changed);
    let t = task(&e, &id_of(&e, "Dentist follow-up"));
    assert_eq!(t.time_estimate, 2_700_000.0);
    assert!(t.due_day.is_none(), "a time replaces the plain day");
    assert_eq!(t.due_with_time, local_ms(&today_str(), 14, 30));
    assert_eq!(t.remind_at, t.due_with_time.map(|d| d - 15 * 60_000));
    let names: Vec<String> = t
        .tag_ids
        .iter()
        .map(|i| e.with_store(|s| s.state.tag.entities[i].title.clone()))
        .collect();
    assert_eq!(names, vec!["health", "urgent"], "existing tag reused, new one created");
    // Without a time, the plain day stays; an empty title creates nothing.
    let plain = TaskDraft {
        title: "Plain".into(),
        time: None,
        reminder_minutes_before: None,
        ..draft.clone()
    };
    e.create_task(plain, View::Today);
    assert_eq!(
        task(&e, &id_of(&e, "Plain")).due_day.as_deref(),
        Some(today_str().as_str())
    );
    assert!(
        !e.create_task(
            TaskDraft {
                title: "  ".into(),
                ..draft
            },
            View::Today
        )
        .changed
    );
}

#[test]
fn editing_an_existing_task_prefills_and_saves_only_differences() {
    let (e, _d) = demo();
    let id = id_of(&e, "Dentist appointment");
    let d = e.task_detail(id.clone()).unwrap();
    assert_eq!(d.title, "Dentist appointment");
    assert_eq!(d.time, Some(ClockTime { hour: 15, minute: 30 }));
    assert_eq!(d.reminder_minutes_before, Some(30));
    assert_eq!(d.estimate_ms, 3_600_000.0);
    assert_eq!(d.due_day.as_deref(), Some(today_str().as_str()));
    let before = e.pending_count();
    let same = TaskDraft {
        title: d.title.clone(),
        project_id: d.project_id.clone(),
        due_day: d.due_day.clone(),
        time: d.time,
        reminder_minutes_before: d.reminder_minutes_before,
        estimate_ms: d.estimate_ms,
        notes: d.notes.clone(),
        tag_ids: d.tag_ids.clone(),
        new_tags: vec![],
    };
    assert!(!e.save_task(id.clone(), same.clone()).changed, "no change, no op");
    assert_eq!(e.pending_count(), before);
    let edited = TaskDraft {
        title: "Dentist (moved)".into(),
        time: None,
        reminder_minutes_before: None,
        notes: "bring the card".into(),
        ..same
    };
    assert!(e.save_task(id.clone(), edited).changed);
    let t = task(&e, &id);
    assert_eq!(t.title, "Dentist (moved)");
    assert!(t.due_with_time.is_none() && t.remind_at.is_none());
    assert_eq!(
        t.due_day.as_deref(),
        Some(today_str().as_str()),
        "clearing the time keeps the day"
    );
    assert_eq!(t.notes.as_deref(), Some("bring the card"));
    let row = e.task_row(id.clone()).unwrap();
    assert_eq!(row.notes_preview.as_deref(), Some("bring the card"));
    // Subtasks: added under the parent, listed in its detail, indented in views.
    assert!(e.add_subtask(id.clone(), "Find insurance card".into()).changed);
    let d = e.task_detail(id.clone()).unwrap();
    assert_eq!(d.sub_tasks.len(), 1);
    assert!(d.sub_tasks[0].is_subtask);
    let home = project_id(&e, "Home");
    let l = e.listing(View::project(&home), 100);
    let ids = row_ids(&l);
    let p = ids.iter().position(|i| *i == id).unwrap();
    assert_eq!(ids[p + 1], d.sub_tasks[0].id, "subtask follows its parent");
}

#[test]
fn moving_a_parent_to_another_project_moves_its_subtasks_and_undo_restores_the_family() {
    let (e, _d) = demo();
    let parent = id_of(&e, "Dentist appointment");
    let original_project = task(&e, &parent).project_id.clone();
    assert!(e.add_subtask(parent.clone(), "Bring insurance card".into()).changed);
    let child = e.task_detail(parent.clone()).unwrap().sub_tasks[0].id.clone();
    assert_eq!(task(&e, &child).project_id, original_project);

    let destination = project_id(&e, "Momentum");
    let outcome = e.move_to_project(vec![parent.clone()], destination.clone());
    assert!(outcome.changed);
    assert_eq!(task(&e, &parent).project_id, destination);
    assert_eq!(task(&e, &child).project_id, destination);

    assert!(e.undo().changed);
    assert_eq!(task(&e, &parent).project_id, original_project);
    assert_eq!(task(&e, &child).project_id, original_project);
}

#[test]
fn repeat_instances_are_spawned_once_per_day() {
    let (e, _d) = demo();
    let today = today_str();
    let cfg = e.with_store(|s| s.state.task_repeat_cfg.entities["demo-weekly"].clone());
    let newest = cfg.newest_due_day(&today);
    let before = e.with_store(|s| s.state.task.ids.len());
    let n = e.spawn_repeats();
    assert_eq!(e.spawn_repeats(), 0, "never twice");
    e.with_store(|s| match newest {
        Some(day) => {
            assert_eq!(n, 1);
            let t = &s.state.task.entities[&format!("rpt_demo-weekly_{day}")];
            assert_eq!(t.due_day.as_deref(), Some(day.as_str()));
            assert_eq!(t.repeat_cfg_id.as_deref(), Some("demo-weekly"));
            assert_eq!(s.state.task.ids.len(), before + 1);
            assert_eq!(
                s.state.task_repeat_cfg.entities["demo-weekly"]
                    .last_task_creation_day
                    .as_deref(),
                Some(day.as_str())
            );
        }
        None => assert_eq!(s.state.task.ids.len(), before),
    });
}

#[test]
fn archived_repeat_entity_is_not_deserialized_or_respawned() {
    let (e, _d) = empty();
    let today = today_str();
    let repeat_id = "archived-repeat";
    let task_id = format!("rpt_{repeat_id}_{today}");
    let mut cfg = RepeatCfg::for_task(&Task::new("Archived repeat", INBOX_PROJECT_ID));
    cfg.id = repeat_id.into();
    cfg.repeat_cycle = "DAILY".into();
    cfg.start_date = Some(today);
    e.with_store_mut(|store| {
        store.state.task_repeat_cfg.insert(repeat_id, cfg);
        store.state.rest.insert(
            "archiveYoung".into(),
            serde_json::json!({"task": {"entities": {task_id.clone(): {"id": task_id.clone()}}}}),
        );
    });

    assert_eq!(e.spawn_repeats(), 0);
    assert!(!has_task(&e, &task_id));
}

#[test]
fn repeat_catch_up_creates_the_missed_instance_dated_that_day() {
    let (e, _d) = empty();
    let missed = day_str(today_n() - 2);
    let mut cfg = RepeatCfg::for_task(&Task::new("Standup", INBOX_PROJECT_ID));
    cfg.id = "daily".into();
    cfg.repeat_cycle = "DAILY".into();
    cfg.start_date = Some(day_str(today_n() - 30));
    cfg.last_task_creation_day = Some(day_str(today_n() - 3));
    e.with_store_mut(|s| s.state.task_repeat_cfg.insert("daily", cfg));
    e.spawn_repeats();
    assert!(
        has_task(&e, &format!("rpt_daily_{}", today_str())),
        "daily: newest day is today"
    );
    let mut weekly = RepeatCfg::for_task(&Task::new("Weekly", INBOX_PROJECT_ID));
    weekly.id = "weekly".into();
    weekly.start_date = Some(day_str(today_n() - 60));
    weekly.last_task_creation_day = Some(day_str(today_n() - 9));
    let wd = weekday(today_n() - 2);
    weekly.monday = wd == 1;
    weekly.tuesday = wd == 2;
    weekly.wednesday = wd == 3;
    weekly.thursday = wd == 4;
    weekly.friday = wd == 5;
    weekly.saturday = wd == 6;
    weekly.sunday = wd == 0;
    e.with_store_mut(|s| s.state.task_repeat_cfg.insert("weekly", weekly));
    e.spawn_repeats();
    let id = format!("rpt_weekly_{missed}");
    assert_eq!(task(&e, &id).due_day.as_deref(), Some(missed.as_str()));
    e.with_store(|s| assert!(s.state.overdue_ids().contains(&id)));
    assert!(e
        .listing(View::Today, 100)
        .sections
        .iter()
        .flat_map(|s| &s.rows)
        .any(|r| matches!(r, Row::Task { row } if row.id == id && row.day.is_some())));
}

#[test]
fn repeat_editor_round_trips_and_describes() {
    let (e, _d) = demo();
    let id = id_of(&e, "Write release notes for 0.1");
    let d = e.repeat_draft(id.clone()).unwrap();
    assert!(!d.existing);
    assert_eq!(d.cycle, RepeatCycle::Weekly);
    assert_eq!(
        d.weekdays,
        vec![false, true, true, true, true, true, false],
        "upstream default: Mon–Fri"
    );
    let mut d = d;
    d.weekdays = vec![false; 7];
    let out = e.save_repeat(id.clone(), d.clone());
    assert_eq!(out.message, Some(Message::PickAWeekday));
    d.weekdays[1] = true;
    assert_eq!(
        e.describe_repeat_draft(d.clone()),
        RepeatDescription::EveryWeekday { weekday: 1 }
    );
    let out = e.save_repeat(id.clone(), d.clone());
    assert!(out.changed);
    assert!(task(&e, &id).repeat_cfg_id.is_some());
    assert!(e.task_menu(id.clone()).unwrap().repeats);
    let d2 = e.repeat_draft(id.clone()).unwrap();
    assert!(d2.existing);
    assert_eq!(d2.weekdays, d.weekdays);
    let mut d2 = d2;
    d2.cycle = RepeatCycle::Monthly;
    d2.monthly = MonthlyRule::NthWeekday { week: -1, weekday: 5 };
    assert!(e.save_repeat(id.clone(), d2).changed);
    let cfg = e.with_store(|s| {
        let cid = s.state.task.entities[&id].repeat_cfg_id.clone().unwrap();
        s.state.task_repeat_cfg.entities[&cid].clone()
    });
    assert_eq!(cfg.nth_weekday_anchor(), Some((-1, 5)));
    assert_eq!(
        e.task_detail(id.clone()).unwrap().repeat,
        Some(RepeatDescription::Monthly {
            n: 1,
            rule: MonthlyRule::NthWeekday { week: -1, weekday: 5 }
        })
    );
    let out = e.stop_repeat(id.clone());
    assert!(matches!(out.message, Some(Message::NoLongerRepeats { .. })));
    assert!(task(&e, &id).repeat_cfg_id.is_none());
}

#[test]
fn undo_stack_reverses_several_changes_in_order() {
    let (e, _d) = empty();
    e.add_task("a".into(), View::Today);
    let a = id_of(&e, "a");
    e.set_done(a.clone(), true);
    e.delete_task(a.clone());
    assert!(!has_task(&e, &a));
    assert!(e.can_undo());
    e.undo();
    assert!(task(&e, &a).is_done);
    e.undo();
    assert!(!task(&e, &a).is_done);
    let out = e.undo(); // nothing left to undo: no panic, no change
    assert_eq!(out.message, Some(Message::NothingToUndo));
    assert!(has_task(&e, &a) && !e.can_undo());
    assert!(e.undo_top().is_none());
    let out = e.set_done(a.clone(), true);
    assert_eq!(e.undo_top(), out.undo);
}

#[test]
fn duplicate_copies_everything_but_identity() {
    let (e, _d) = demo();
    let id = id_of(&e, "Dentist appointment");
    let out = e.duplicate_task(id.clone());
    assert_eq!(out.message, Some(Message::TaskDuplicated));
    let copies: Vec<Task> = e.with_store(|s| {
        s.state
            .task
            .iter()
            .filter(|t| t.title.starts_with("Dentist appointment"))
            .cloned()
            .collect()
    });
    assert_eq!(copies.len(), 2);
    let copy = copies.iter().find(|t| t.id != id).unwrap();
    assert_eq!(copy.time_estimate, task(&e, &id).time_estimate);
    assert_eq!(copy.tag_ids, task(&e, &id).tag_ids);
    assert_eq!(copy.due_with_time, task(&e, &id).due_with_time);
    assert_eq!(copy.remind_at, task(&e, &id).remind_at);
    assert!(!copy.is_done);
    assert_eq!(e.last_added_id().as_deref(), Some(copy.id.as_str()));
    e.undo();
    assert!(!has_task(&e, &copy.id));
}

#[test]
fn keyboard_reorder_follows_descending_display_and_undo_restores_hidden_positions() {
    let (e, _d) = empty();
    for (title, view) in [("a", View::Tonight), ("hidden", View::Today), ("b", View::Tonight)] {
        e.add_task(title.into(), view);
    }
    let a = id_of(&e, "a");
    let b = id_of(&e, "b");
    let original = e.with_store(|s| s.state.today_ids());
    e.set_preferences(Preferences {
        direction: SortDirection::Descending,
        ..Preferences::default()
    });
    assert!(e.nudge(a.clone(), -1, View::Tonight).changed);
    assert_eq!(row_ids(&e.listing(View::Tonight, 100)), vec![a, b]);
    e.undo();
    assert_eq!(e.with_store(|s| s.state.today_ids()), original);
}

#[test]
fn sidebar_lists_views_and_titles_follow() {
    let (e, _d) = demo();
    let sb = e.sidebar();
    let fixed: Vec<View> = sb.fixed.iter().map(|x| x.view.clone()).collect();
    assert_eq!(
        fixed,
        vec![
            View::Today,
            View::Morning,
            View::Tonight,
            View::Upcoming,
            View::Archive,
            View::Search
        ]
    );
    assert_eq!(sb.projects.len(), 3, "Inbox, Momentum, Home");
    assert_eq!(sb.tags.len(), 4);
    assert!(
        sb.projects.iter().all(|p| p.color.is_some()),
        "projects carry theme colours"
    );
    assert_eq!(e.view_title(View::Upcoming), ViewTitle::ComingUp);
    let home = project_id(&e, "Home");
    assert_eq!(
        e.view_title(View::project(&home)),
        ViewTitle::Named { name: "Home".into() }
    );
}

#[test]
fn sidebar_context_counts_unique_unfinished_live_task_families() {
    let (e, _d) = empty();
    let project = e.add_project("Garden".into()).unwrap();
    let tag = e.add_tag("outside".into()).unwrap();
    let project_view = View::project(&project);
    let tag_view = View::tag(&tag);
    let count = |view: &View| {
        let sidebar = e.sidebar();
        sidebar
            .projects
            .iter()
            .chain(sidebar.tags.iter())
            .find(|entry| &entry.view == view)
            .unwrap()
            .task_count
    };
    assert_eq!(count(&project_view), 0);
    assert_eq!(count(&tag_view), 0);

    e.add_task("Family".into(), project_view.clone());
    let parent = id_of(&e, "Family");
    e.add_subtask(parent.clone(), "Child".into());
    let child = id_of(&e, "Child");
    e.add_tag_to(vec![child.clone()], tag.clone());
    assert_eq!(count(&project_view), 1);
    assert_eq!(count(&tag_view), 1, "a child-only tag counts its root family");

    e.add_tag_to(vec![parent.clone()], tag.clone());
    assert_eq!(count(&tag_view), 1, "parent and child membership do not double count");
    e.set_done(parent.clone(), true);
    assert_eq!(count(&project_view), 0);
    assert_eq!(count(&tag_view), 0, "a completed root excludes its family");

    // Even an inconsistent unfinished child cannot revive a completed root's family count.
    e.set_done(child, false);
    assert_eq!(count(&tag_view), 0);
    e.archive_done();
    assert_eq!(count(&project_view), 0);
    assert_eq!(count(&tag_view), 0, "archived roots never count");
}

#[test]
fn empty_states_name_the_view() {
    let (e, _d) = empty();
    assert_eq!(e.listing(View::Today, 100).empty, Some(EmptyState::Today));
    assert_eq!(e.listing(View::Archive, 100).empty, Some(EmptyState::Archive));
    assert_eq!(e.listing(View::Search, 100).empty, Some(EmptyState::Search));
    let pid = e.add_project("Garden".into()).unwrap();
    assert_eq!(
        e.listing(View::project(&pid), 100).empty,
        Some(EmptyState::Project { name: "Garden".into() })
    );
    let tid = e.add_tag("yard".into()).unwrap();
    assert_eq!(
        e.listing(View::tag(&tid), 100).empty,
        Some(EmptyState::Tag { name: "yard".into() })
    );
}

#[test]
fn paste_and_drop_text_become_tasks() {
    let (e, _d) = empty();
    let out = e.add_from_text("first thing\nsecond thing\n\n".into(), View::Today);
    assert_eq!(out.message, Some(Message::TasksAdded { n: 2 }));
    assert_eq!(e.with_store(|s| s.state.task.ids.len()), 2);
    assert_eq!(
        e.add_from_text("https://example.org/page/".into(), View::Today).message,
        Some(Message::TaskAdded)
    );
    let link = task(&e, &id_of(&e, "example.org/page"));
    assert_eq!(link.notes.as_deref(), Some("https://example.org/page/"));
    assert_eq!(
        link.due_day.as_deref(),
        Some(today_str().as_str()),
        "a URL import keeps the destination view"
    );
    let long = "A rather long paragraph ".repeat(10);
    e.add_from_text(long, View::Today);
    let t = e.with_store(|s| s.state.task.iter().last().cloned().unwrap());
    assert!(
        t.title.chars().count() <= 120 && t.notes.is_some(),
        "long text is title + notes"
    );
    assert_eq!(
        t.due_day.as_deref(),
        Some(today_str().as_str()),
        "a paragraph import keeps the destination view"
    );
    assert!(!e.add_from_text("  \n".into(), View::Today).changed);
    let ids = format!("{}\n{}", id_of(&e, "first thing"), id_of(&e, "second thing"));
    assert!(e.is_task_id_list(ids));
    assert!(!e.is_task_id_list("first thing".into()));
}

#[test]
fn reminders_fire_once_for_due_tasks() {
    let (e, _d) = empty();
    let mut t = Task::new("Ping", INBOX_PROJECT_ID);
    t.remind_at = Some(sp_model::now_ms() - 1000);
    t.due_with_time = local_ms(&today_str(), 9, 0);
    let id = t.id.clone();
    e.dispatch(Action::AddTask { task: t, bottom: true });
    let due = e.due_reminders();
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].task_id, id);
    assert_eq!(due[0].time, Some(ClockTime { hour: 9, minute: 0 }));
    assert!(e.due_reminders().is_empty(), "fires once");
    assert_eq!(e.snooze(id.clone(), 60).message, Some(Message::Snoozed { minutes: 60 }));
    let at = task(&e, &id).remind_at.unwrap();
    assert!(at > sp_model::now_ms() + 59 * 60_000);
    assert!(!e.notified_contains(&id), "a snooze re-arms the reminder");
    assert!(e.due_reminders().is_empty(), "and it does not fire before its time");
    // The notification's Done button and complete-task URL.
    assert!(e.complete_by_title("ping".into()).changed);
    assert!(task(&e, &id).is_done);
    assert!(!e.complete_by_title("ping".into()).changed, "already done");
}

#[test]
fn task_counts_share_today_membership_and_count_families_once() {
    let (e, _d) = empty();
    e.add_task("Today parent".into(), View::Today);
    let parent = id_of(&e, "Today parent");
    e.add_subtask(parent.clone(), "Child".into());
    let mut overdue = Task::new("Overdue", INBOX_PROJECT_ID);
    overdue.due_day = Some(day_str(today_n() - 1));
    let overdue_id = overdue.id.clone();
    e.dispatch(Action::AddTask {
        task: overdue,
        bottom: true,
    });
    let mut future = Task::new("Future", INBOX_PROJECT_ID);
    future.due_day = Some(day_str(today_n() + 1));
    e.dispatch(Action::AddTask {
        task: future,
        bottom: true,
    });
    let mut timed = Task::new("Timed today", INBOX_PROJECT_ID);
    timed.due_day = Some(day_str(today_n() + 1));
    timed.due_with_time = local_ms(&today_str(), 23, 0);
    e.dispatch(Action::AddTask {
        task: timed,
        bottom: true,
    });
    assert_eq!(e.task_count(TaskCountMode::DueToday), 2);
    assert_eq!(e.task_count(TaskCountMode::TodayIncludingOverdue), 3);
    assert_eq!(e.today_open_count(), e.task_count(TaskCountMode::DueToday));
    e.bulk_done(vec![parent, overdue_id]);
    assert_eq!(e.task_count(TaskCountMode::TodayIncludingOverdue), 1);
    e.archive_done();
    assert_eq!(e.task_count(TaskCountMode::TodayIncludingOverdue), 1);
    assert_eq!(e.task_count(TaskCountMode::DueToday), 1);
}

#[test]
fn stale_notification_cannot_snooze_completed_archived_or_deleted_tasks() {
    let (e, _d) = empty();
    e.add_task("Finish notification task".into(), View::Today);
    let id = id_of(&e, "Finish notification task");
    assert!(e.snooze(id.clone(), 10).changed);
    let reminder = task(&e, &id).remind_at;
    e.bulk_done(vec![id.clone()]);
    assert!(!e.snooze(id.clone(), 10).changed);
    assert_eq!(task(&e, &id).remind_at, reminder);
    e.archive_done();
    assert!(!e.snooze(id.clone(), 10).changed);
    e.add_task("Deleted notification task".into(), View::Today);
    let deleted = id_of(&e, "Deleted notification task");
    e.bulk_delete(vec![deleted.clone()]);
    assert!(!e.snooze(deleted, 10).changed);
    assert!(!e.snooze("missing-task".into(), 10).changed);
}

#[test]
fn morning_summary_requires_opt_in_and_waits_for_the_selected_local_time() {
    let (e, _d) = demo();
    let at = |hour, minute| ClockTime { hour, minute };
    assert!(!e.preferences().morning_summary_enabled);
    assert_eq!(e.preferences().morning_summary_time, at(8, 0));
    assert!(e.morning_summary_at(at(23, 59)).is_none());
    assert!(e.with_store(|s| s.meta.last_summary_day.is_empty()));
    e.set_preferences(Preferences {
        morning_summary_enabled: true,
        morning_summary_time: at(9, 30),
        ..Default::default()
    });
    assert!(e.morning_summary_at(at(9, 29)).is_none());
    assert!(e.with_store(|s| s.meta.last_summary_day.is_empty()));
    let summary = e.morning_summary_at(at(9, 30)).expect("due at selected minute");
    assert!(summary.total >= 5 && summary.morning == 1 && summary.tonight == 2);
    assert!(e.morning_summary_at(at(23, 59)).is_none(), "once per day");
    let reopened = Engine::open(e.data_dir());
    reopened.set_preferences(e.preferences());
    assert!(
        reopened.morning_summary_at(at(23, 59)).is_none(),
        "restart cannot duplicate"
    );
    // A prior day's claim must not prevent today's summary.
    reopened.with_store_mut(|s| s.meta.last_summary_day = "2000-01-01".into());
    assert!(
        reopened.morning_summary_at(at(10, 0)).is_some(),
        "late launch catches up"
    );
}

#[test]
fn morning_summary_disabling_preserves_time_and_does_not_affect_reminders() {
    let (e, _d) = demo();
    let mut prefs = Preferences {
        morning_summary_enabled: true,
        morning_summary_time: ClockTime { hour: 0, minute: 0 },
        ..Default::default()
    };
    prefs.morning_summary_enabled = false;
    e.set_preferences(prefs.clone());
    assert!(e.morning_summary().is_none());
    let id = id_of(&e, "Write release notes for 0.1");
    e.with_store_mut(|s| {
        let t = s.state.task.entities.get_mut(&id).unwrap();
        t.remind_at = Some(sp_model::now_ms() - 1);
    });
    assert!(e.due_reminders().iter().any(|r| r.task_id == id));
    prefs.morning_summary_enabled = true;
    e.set_preferences(prefs.clone());
    assert!(e.morning_summary().is_some());
    prefs.morning_summary_time = ClockTime { hour: 23, minute: 59 };
    e.set_preferences(prefs);
    assert!(
        e.morning_summary_at(ClockTime { hour: 23, minute: 59 }).is_none(),
        "changing the time cannot generate a second summary"
    );
}

#[test]
fn morning_summary_is_silent_without_open_tasks() {
    let (e, _d) = empty();
    e.set_preferences(Preferences {
        morning_summary_enabled: true,
        morning_summary_time: ClockTime { hour: 0, minute: 0 },
        ..Default::default()
    });
    assert!(e.morning_summary().is_none());
}

#[test]
fn project_and_tag_management() {
    let (e, _d) = empty();
    let pid = e.add_project("Garden".into()).unwrap();
    assert!(e.sidebar().projects.iter().any(|p| p.view == View::project(&pid)));
    e.add_task("Water plants".into(), View::project(&pid));
    let tid = id_of(&e, "Water plants");
    assert_eq!(task(&e, &tid).project_id, pid);
    assert!(
        e.update_project(pid.clone(), "Yard".into(), Some("#112233".into()))
            .changed
    );
    assert_eq!(
        e.view_title(View::project(&pid)),
        ViewTitle::Named { name: "Yard".into() }
    );
    assert_eq!(e.project(pid.clone()).unwrap().color.as_deref(), Some("#112233"));
    assert!(
        !e.update_project(pid.clone(), "Yard".into(), None).changed,
        "nothing to change"
    );
    assert_eq!(e.project_task_count(pid.clone()), 1);
    assert!(!e.delete_project(INBOX_PROJECT_ID.into()).changed, "Inbox stays");
    assert!(e.delete_project(pid.clone()).changed);
    assert!(!has_task(&e, &tid));
    assert!(!e.view_exists(View::project(&pid)));
    let gid = e.add_tag("yard".into()).unwrap();
    assert_eq!(e.add_tag("YARD".into()).unwrap(), gid, "names match case-insensitively");
    assert!(
        e.update_tag(gid.clone(), "garden".into(), Some("#abcdef".into()))
            .changed
    );
    assert_eq!(e.tag(gid.clone()).unwrap().color.as_deref(), Some("#abcdef"));
    assert!(e.delete_tag(gid.clone()).changed);
    assert!(!e.view_exists(View::tag(&gid)));
    assert!(e.add_project("   ".into()).is_none());
}

#[test]
fn morning_and_tonight_entries_appear_only_when_today_uses_them() {
    let (e, _d) = empty();
    assert!(
        !sidebar_has(&e, View::Morning) && !sidebar_has(&e, View::Tonight),
        "empty store: neither entry"
    );
    e.add_task("Plain".into(), View::Today);
    let id = id_of(&e, "Plain");
    assert!(!sidebar_has(&e, View::Morning));
    e.toggle_slot(vec![id.clone()], Slot::Morning);
    assert!(sidebar_has(&e, View::Morning) && !sidebar_has(&e, View::Tonight));
    assert_eq!(row_ids(&e.listing(View::Morning, 100)), vec![id.clone()]);
    assert!(e.view_exists(View::Morning));
    // Moving the only morning task away removes the entry (the UI then falls back to Today).
    e.toggle_slot(vec![id.clone()], Slot::Tonight);
    assert!(!sidebar_has(&e, View::Morning) && sidebar_has(&e, View::Tonight));
    assert!(!e.view_exists(View::Morning));
    let t = task(&e, &id);
    assert!(
        t.tag_ids.contains(&tag_id(&e, "Evening")) && !t.tag_ids.contains(&tag_id(&e, "Morning")),
        "one slot at most"
    );
    // A task tagged for the slot but planned for another day does not count.
    e.move_to_tomorrow(vec![id.clone()]);
    assert!(!sidebar_has(&e, View::Tonight));
    let (demo, _d2) = demo();
    assert!(
        sidebar_has(&demo, View::Morning) && sidebar_has(&demo, View::Tonight),
        "demo data has both"
    );
}

#[test]
fn morning_toggle_batch_swaps_slots_and_undo_restores() {
    let (e, _d) = demo();
    let notes = id_of(&e, "Write release notes for 0.1");
    let read = id_of(&e, "Read two chapters");
    let (morning, evening) = (tag_id(&e, "Morning"), tag_id(&e, "Evening"));
    let out = e.toggle_slot(vec![notes.clone(), read.clone()], Slot::Morning);
    assert_eq!(out.message, Some(Message::TasksMovedToMorning { n: 2 }));
    for id in [&notes, &read] {
        let t = task(&e, id);
        assert!(
            t.tag_ids.contains(&morning) && !t.tag_ids.contains(&evening),
            "{}",
            t.title
        );
    }
    e.undo();
    assert!(
        task(&e, &read).tag_ids.contains(&evening),
        "undo restores the evening tag"
    );
    let stretch = id_of(&e, "Stretch and plan the day");
    let out = e.toggle_slot(vec![stretch.clone()], Slot::Morning);
    assert_eq!(out.message, Some(Message::MovedToToday));
    assert!(!task(&e, &stretch).tag_ids.contains(&morning));
    assert!(!row_ids(&e.listing(View::Morning, 100)).contains(&stretch));
}

#[test]
fn auto_archive_sends_completed_tasks_straight_to_the_archive() {
    let (e, _d) = demo();
    let id = id_of(&e, "Write release notes for 0.1");
    e.set_done(id.clone(), true);
    assert!(has_task(&e, &id), "off by default: stays in the list");
    e.undo();
    e.set_preferences(Preferences {
        auto_archive: true,
        ..Preferences::default()
    });
    let out = e.set_done(id.clone(), true);
    assert_eq!(out.message, Some(Message::TaskCompletedArchived));
    assert!(out.sync_now);
    assert!(!has_task(&e, &id), "gone from the live list");
    e.with_store(|s| assert_eq!(s.state.rest["archiveYoung"]["task"]["entities"][&id]["isDone"], true));
    assert!(!kinds(&e.listing(View::Today, 100)).contains(&SectionKind::Completed));
    e.undo();
    assert!(!task(&e, &id).is_done, "undo restores it as an open task");
    let a = id_of(&e, "Read two chapters");
    let b = id_of(&e, "Prep tomorrow's lunch");
    let out = e.bulk_done(vec![a.clone(), b.clone()]);
    assert_eq!(out.message, Some(Message::TasksCompletedArchived { n: 2 }));
    assert!(!has_task(&e, &a) && !has_task(&e, &b));
    e.undo();
    assert!(has_task(&e, &a) && !task(&e, &b).is_done);
    e.set_done(a.clone(), false);
    assert!(has_task(&e, &a), "reopening never archives");
}

#[test]
fn backup_import_and_export_round_trip() {
    let (e, _d) = demo();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("backup.json").to_string_lossy().into_owned();
    assert_eq!(
        e.export_backup(path.clone()).unwrap().message,
        Some(Message::BackupExported)
    );
    let (fresh, _d2) = empty();
    assert_eq!(
        fresh.import_backup(path).unwrap().message,
        Some(Message::BackupImported)
    );
    assert_eq!(
        fresh.with_store(|s| s.state.task.ids.len()),
        e.with_store(|s| s.state.task.ids.len())
    );
    assert_eq!(fresh.pending_count(), 0, "import drops pending ops");
    assert!(matches!(
        fresh.import_backup("/nonexistent/x.json".into()),
        Err(CoreError::Io { .. })
    ));
}

#[test]
fn external_writes_are_picked_up_and_cli_actions_dispatched() {
    let (e, _d) = empty();
    e.add_task("mine".into(), View::Today);
    assert!(!e.reload_from_disk(), "nothing changed on disk");
    // Another process writes the same store.
    let mut other = sp_store::Store::load(std::path::PathBuf::from(e.data_dir()));
    other.dispatch(Action::AddTask {
        task: Task::new("theirs", INBOX_PROJECT_ID),
        bottom: true,
    });
    assert!(e.reload_from_disk());
    assert!(e.with_store(|s| s.state.task.iter().any(|t| t.title == "theirs")));
    let payload = serde_json::to_string(&Action::AddTask {
        task: Task::new("from mo", INBOX_PROJECT_ID),
        bottom: true,
    })
    .unwrap();
    assert!(e.dispatch_json(payload).unwrap().changed);
    assert!(e.with_store(|s| s.state.task.iter().any(|t| t.title == "from mo")));
    assert!(matches!(
        e.dispatch_json("nonsense".into()),
        Err(CoreError::Invalid { .. })
    ));
}

#[cfg(unix)]
#[test]
fn cli_socket_forwards_actions_to_the_running_engine() {
    use std::sync::atomic::{AtomicU32, Ordering};
    struct Counter(AtomicU32, AtomicU32);
    impl ipc::CliDelegate for Counter {
        fn store_changed(&self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
        fn sync_requested(&self) {
            self.1.fetch_add(1, Ordering::SeqCst);
        }
    }
    let (e, _d) = empty();
    let counter = Arc::new(Counter(AtomicU32::new(0), AtomicU32::new(0)));
    e.clone().serve_cli(counter.clone()).unwrap();
    let dir = std::path::PathBuf::from(e.data_dir());
    let payload = serde_json::to_string(&Action::AddTask {
        task: Task::new("over the socket", INBOX_PROJECT_ID),
        bottom: true,
    })
    .unwrap();
    assert_eq!(ipc::forward(&dir, &payload), Ok(true));
    assert!(e.with_store(|s| s.state.task.iter().any(|t| t.title == "over the socket")));
    assert_eq!(ipc::forward(&dir, "\"sync\""), Ok(true));
    assert_eq!(counter.0.load(Ordering::SeqCst), 1);
    assert_eq!(counter.1.load(Ordering::SeqCst), 1);
    assert!(ipc::forward(&dir, "garbage").is_err());
    e.stop_cli_server();
    std::thread::sleep(std::time::Duration::from_millis(50));
    assert_eq!(ipc::forward(&dir, &payload), Ok(false), "no socket, no app");
}

/// The server accepts from a non-blocking listener, and on BSD/macOS the accepted
/// connection inherits O_NONBLOCK. A client that connects a moment before it writes must
/// still be served, not answered "Resource temporarily unavailable" and hung up on.
#[test]
#[cfg(unix)]
fn cli_socket_waits_for_a_client_that_connects_before_it_writes() {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;
    struct Silent;
    impl ipc::CliDelegate for Silent {
        fn store_changed(&self) {}
        fn sync_requested(&self) {}
    }
    let (e, _d) = empty();
    e.clone().serve_cli(Arc::new(Silent)).unwrap();
    let path = ipc::socket_path(&std::path::PathBuf::from(e.data_dir()));
    let mut c = UnixStream::connect(&path).unwrap();
    // Longer than the accept loop's poll interval, so the server is already waiting on
    // a connection with nothing in it.
    std::thread::sleep(std::time::Duration::from_millis(250));
    let payload = serde_json::to_string(&Action::AddTask {
        task: Task::new("written late", INBOX_PROJECT_ID),
        bottom: true,
    })
    .unwrap();
    c.write_all(format!("{payload}\n").as_bytes()).unwrap();
    let mut reply = String::new();
    BufReader::new(&c).read_line(&mut reply).unwrap();
    assert_eq!(reply.trim(), "ok", "a slow writer is still served");
    assert!(e.with_store(|s| s.state.task.iter().any(|t| t.title == "written late")));
}

#[test]
fn sync_refuses_without_settings_and_reports_status() {
    let (e, _d) = demo();
    let s = e.sync_status();
    assert!(!s.syncing && s.pending_ops > 0 && s.last_nextcloud_ms == 0 && !s.nearby_running);
    let err = e
        .sync_nextcloud(NextcloudSettings {
            server_url: String::new(),
            user_name: String::new(),
            password: String::new(),
            folder: "x".into(),
            compress: false,
            encryption_password: None,
        })
        .unwrap_err();
    assert!(matches!(err, CoreError::NotConfigured));
    let err = e
        .sync_nextcloud(NextcloudSettings {
            server_url: "http://127.0.0.1:1".into(),
            user_name: "u".into(),
            password: "p".into(),
            folder: "f".into(),
            compress: false,
            encryption_password: None,
        })
        .unwrap_err();
    assert!(matches!(err, CoreError::Transient { .. }), "{err:?}");
    assert!(!e.is_syncing(), "the flag is released after a failure");
}

#[test]
fn free_functions_over_ffi_shapes() {
    assert_eq!(parse_estimate("1h 30m".into()), Some(5_400_000.0));
    assert_eq!(format_estimate(600_000.0), "10m");
    assert_eq!(parse_time("2pm".into()), Some(ClockTime { hour: 14, minute: 0 }));
    let ms = local_ms("2026-09-14", 15, 30).unwrap();
    assert_eq!(crate::text::clock_time(ms), ClockTime { hour: 15, minute: 30 });
    assert_eq!(parse_quick_add("Buy #x 5m".into()).tags, vec!["x"]);
    assert_eq!(day_offset(today(), 1), day_str(today_n() + 1));
    assert_eq!(day_offset("junk".into(), 1), "junk");
    assert!(crate::now_ms() > 0);
    assert_eq!(day_label(today()).relation, DayRelation::Today);
}

#[test]
fn task_creation_returns_its_own_id_even_with_concurrent_creators() {
    let (engine, _dir) = empty();
    let threads: Vec<_> = (0..8)
        .map(|i| {
            let engine = engine.clone();
            std::thread::spawn(move || {
                let title = format!("Concurrent task {i}");
                let created = engine.create_task_with_id(
                    TaskDraft {
                        title: title.clone(),
                        project_id: INBOX_PROJECT_ID.into(),
                        due_day: None,
                        time: None,
                        reminder_minutes_before: None,
                        estimate_ms: 0.0,
                        notes: String::new(),
                        tag_ids: vec![],
                        new_tags: vec![format!("tag{i}")],
                    },
                    View::Search,
                );
                assert!(created.outcome.changed);
                (created.id.unwrap(), title)
            })
        })
        .collect();
    for thread in threads {
        let (id, title) = thread.join().unwrap();
        assert_eq!(engine.task_title(id), Some(title));
    }
    let before = engine.pending_count();
    let empty = engine.create_task_with_id(
        TaskDraft {
            title: "  ".into(),
            project_id: String::new(),
            due_day: None,
            time: None,
            reminder_minutes_before: None,
            estimate_ms: 0.0,
            notes: String::new(),
            tag_ids: vec![],
            new_tags: vec!["must not create".into()],
        },
        View::Today,
    );
    assert_eq!(empty.outcome, Outcome::none());
    assert_eq!(empty.id, None);
    assert_eq!(engine.pending_count(), before);
}

#[test]
fn reopening_filters_deduplicates_and_undoes_as_one_batch() {
    let (e, _d) = empty();
    e.add_task("first".into(), View::Today);
    e.add_task("second".into(), View::Today);
    e.add_task("already open".into(), View::Today);
    let first = id_of(&e, "first");
    let second = id_of(&e, "second");
    let open = id_of(&e, "already open");
    e.bulk_done(vec![first.clone(), second.clone()]);
    let previous_undo = e.undo_top();
    e.set_preferences(Preferences {
        auto_archive: true,
        ..Preferences::default()
    });
    let before = e.pending_count();
    let out = e.reopen_tasks(vec![
        first.clone(),
        "missing".into(),
        first.clone(),
        open.clone(),
        second.clone(),
    ]);
    assert!(out.changed && out.undo.is_some() && out.undo != previous_undo);
    assert_eq!(
        e.pending_count(),
        before + 2,
        "only two completed, unique tasks changed"
    );
    for id in [&first, &second, &open] {
        assert!(!task(&e, id).is_done);
    }
    let no_change = e.reopen_tasks(vec![first.clone(), "missing".into()]);
    assert_eq!(no_change, Outcome::none());
    assert_eq!(e.undo_top(), out.undo);
    e.undo();
    assert!(task(&e, &first).is_done && task(&e, &second).is_done);
    assert!(!task(&e, &open).is_done);
    assert_eq!(e.undo_top(), previous_undo, "single undo batch restores both");
    assert_eq!(e.reopen_tasks(vec![]), Outcome::none());
}

#[test]
fn upcoming_ids_include_completed_only_when_requested_and_match_view_bounds() {
    let (e, _d) = empty();
    e.set_preferences(Preferences {
        upcoming_days: 3,
        ..Preferences::default()
    });
    let add = |title: &str, day: i64, done: bool, timed: bool, child: bool| {
        let mut task = Task::new(title, INBOX_PROJECT_ID);
        task.is_done = done;
        let date = day_str(today_n() + day);
        if timed {
            task.due_with_time = local_ms(&date, 12, 0);
        } else {
            task.due_day = Some(date);
        }
        if child {
            task.parent_id = Some("parent".into());
        }
        let id = task.id.clone();
        e.dispatch(Action::AddTask { task, bottom: true });
        id
    };
    let later = add("Later", 3, false, false, false);
    let tomorrow = add("Timed tomorrow", 1, false, true, false);
    let completed = add("Completed tomorrow", 1, true, false, false);
    add("Today", 0, false, false, false);
    add("Too far", 4, false, false, false);
    add("Child", 1, false, false, true);
    add("Completed child", 1, true, false, true);
    assert_eq!(e.upcoming_task_ids(false), vec![tomorrow.clone(), later.clone()]);
    assert_eq!(e.upcoming_task_ids(false), row_ids(&e.listing(View::Upcoming, 100)));
    assert_eq!(e.upcoming_task_ids(true), vec![completed, tomorrow, later]);
    e.set_preferences(Preferences {
        upcoming_days: 0,
        ..Preferences::default()
    });
    assert!(e.upcoming_task_ids(true).is_empty());
}

#[test]
fn group_by_estimate_uses_exact_boundaries_and_keeps_no_estimate_last() {
    let (e, _dir) = empty();
    let estimates = [
        0.0,
        -1.0,
        1.0,
        900_000.0,
        900_001.0,
        1_800_000.0,
        1_800_001.0,
        3_600_000.0,
        3_600_001.0,
        7_200_000.0,
        7_200_001.0,
    ];
    for (n, estimate) in estimates.into_iter().enumerate() {
        let mut task = Task::new(&format!("Estimate {n}"), INBOX_PROJECT_ID);
        task.time_estimate = estimate;
        e.dispatch(Action::AddTask { task, bottom: true });
    }
    e.set_preferences(Preferences {
        group_by: GroupBy::Estimate,
        ..Default::default()
    });
    let l = e.listing(View::project(INBOX_PROJECT_ID), 100);
    let groups: Vec<_> = l.sections.iter().map(|s| s.group.clone()).collect();
    use EstimateRange::*;
    assert_eq!(
        groups,
        [
            UpTo15Minutes,
            UpTo30Minutes,
            UpTo60Minutes,
            UpTo2Hours,
            Over2Hours,
            NoEstimate
        ]
        .into_iter()
        .map(|range| Some(TaskGroup::Estimate { range }))
        .collect::<Vec<_>>()
    );
    assert_eq!(
        l.sections.iter().map(|s| s.count).collect::<Vec<_>>(),
        [2, 2, 2, 2, 1, 2]
    );
    assert_eq!(row_ids(&l).len(), estimates.len());
}

#[test]
fn group_by_first_tag_is_unique_excludes_virtual_today_and_keeps_subtasks_with_parent() {
    let (e, _dir) = demo();
    let id = id_of(&e, "Review sync conflict handling");
    let urgent = tag_id(&e, "urgent");
    let gnome = tag_id(&e, "gnome");
    e.dispatch(Action::UpdateTask {
        id: id.clone(),
        changes: [("tagIds".into(), serde_json::json!([TODAY_TAG_ID, urgent, gnome]))]
            .into_iter()
            .collect(),
    });
    e.add_subtask(id.clone(), "Child without the parent's tags".into());
    let child = id_of(&e, "Child without the parent's tags");
    let before = e.listing(View::Today, 100);
    let pending = e.pending_count();
    e.set_preferences(Preferences {
        group_by: GroupBy::Tag,
        ..Default::default()
    });
    let l = e.listing(View::Today, 100);
    assert_eq!(row_ids(&l).iter().filter(|i| **i == id).count(), 1);
    let section = l
        .sections
        .iter()
        .find(|s| s.rows.iter().any(|r| matches!(r, Row::Task { row } if row.id == id)))
        .unwrap();
    let expected_color = e.tags().into_iter().find(|t| t.id == urgent).unwrap().color;
    assert!(
        matches!(&section.group, Some(TaskGroup::Tag { id: tag, color, .. }) if tag == &urgent && color == &expected_color)
    );
    let family: Vec<_> = section
        .rows
        .iter()
        .filter_map(|r| match r {
            Row::Task { row } => Some(row.id.clone()),
            _ => None,
        })
        .collect();
    let parent_position = family.iter().position(|i| i == &id).unwrap();
    assert_eq!(
        family[parent_position + 1],
        child,
        "child stays beside its parent despite having different grouping attributes"
    );
    assert!(l.sections.iter().any(|s| s.group == Some(TaskGroup::Untagged)));
    assert_eq!(row_ids(&l).len(), row_ids(&before).len());
    assert_eq!(e.pending_count(), pending, "grouping never emits task operations");
    e.set_preferences(Preferences::default());
    assert_eq!(
        e.listing(View::Today, 100),
        before,
        "Restoring the default restores the original listing exactly"
    );
}

#[test]
fn group_by_project_replaces_date_sections_and_sorts_within_groups() {
    let (e, _dir) = demo();
    e.set_preferences(Preferences {
        group_by: GroupBy::Project,
        sort: SortKey::Title,
        direction: SortDirection::Descending,
        ..Default::default()
    });
    let l = e.listing(View::Today, 100);
    assert!(l.sections.iter().all(|s| s.kind == SectionKind::Plain));
    assert!(l
        .sections
        .iter()
        .all(|s| matches!(s.group, Some(TaskGroup::Project { .. }))));
    for s in &l.sections {
        let titles: Vec<_> = s
            .rows
            .iter()
            .filter_map(|r| match r {
                Row::Task { row } if !row.is_subtask => Some(row.title.to_lowercase()),
                _ => None,
            })
            .collect();
        assert!(titles.windows(2).all(|w| w[0] >= w[1]));
    }
    let home = project_id(&e, "Home");
    let project = e.listing(View::project(&home), 100);
    let expected_color = e.projects().into_iter().find(|p| p.id == home).unwrap().color;
    assert!(project
        .sections
        .iter()
        .all(|s| matches!(&s.group, Some(TaskGroup::Project { color, .. }) if color == &expected_color)));
    assert!(
        project
            .sections
            .iter()
            .all(|s| matches!(&s.group, Some(TaskGroup::Project { id, .. }) if id == &home)),
        "project grouping must work even when row subtitles omit the current project"
    );
}

#[test]
fn group_by_reorder_stays_within_group_and_undo_restores_manual_order() {
    let (e, _dir) = empty();
    for (name, estimate) in [("First", 300_000.0), ("Other", 3_600_000.0), ("Last", 600_000.0)] {
        let mut t = Task::new(name, INBOX_PROJECT_ID);
        t.time_estimate = estimate;
        e.dispatch(Action::AddTask { task: t, bottom: true });
    }
    let view = View::project(INBOX_PROJECT_ID);
    let first = id_of(&e, "First");
    let other = id_of(&e, "Other");
    let last = id_of(&e, "Last");
    let original = row_ids(&e.listing(view.clone(), 100));
    e.set_preferences(Preferences {
        group_by: GroupBy::Estimate,
        ..Default::default()
    });
    assert!(
        !e.reorder(last.clone(), other.clone(), view.clone()).changed,
        "reorder cannot silently change grouping attributes"
    );
    let out = e.reorder(last.clone(), first.clone(), view.clone());
    assert!(out.changed);
    assert_eq!(
        row_ids(&e.listing(view.clone(), 100)),
        [last.clone(), first.clone(), other.clone()]
    );
    e.undo_batch(out.undo.unwrap());
    assert!(e.nudge(first.clone(), 1, view.clone()).changed);
    assert_eq!(
        row_ids(&e.listing(view.clone(), 100)),
        [last, first, other],
        "keyboard moves among the visible group, skipping hidden neighbors"
    );
    e.undo();
    e.set_preferences(Preferences::default());
    assert_eq!(row_ids(&e.listing(view, 100)), original);
}

#[test]
fn group_by_search_and_archive_keep_caps_notes_and_read_only_rows() {
    let (e, _dir) = empty();
    for n in 0..65 {
        e.add_task(
            format!("Group test {n} {}m", if n % 2 == 0 { 5 } else { 60 }),
            View::Today,
        );
    }
    e.set_preferences(Preferences {
        group_by: GroupBy::Estimate,
        ..Default::default()
    });
    let search = e.search("Group test".into());
    assert_eq!(row_ids(&search).len(), 60);
    assert_eq!(search.sections.iter().filter(|s| s.note.is_some()).count(), 1);
    assert_eq!(search.sections.last().unwrap().note.as_ref().unwrap().total, 65);
    let ids = row_ids(&e.listing(View::Today, 100));
    e.bulk_done(ids);
    e.archive_done();
    let archive = e.listing(View::Archive, 10);
    assert_eq!(row_ids(&archive).len(), 10);
    assert_eq!(archive.more_available, 55);
    assert!(archive
        .sections
        .iter()
        .flat_map(|s| &s.rows)
        .all(|r| matches!(r, Row::Task { row } if row.archived)));
}

#[test]
fn exclusive_grouping_is_the_only_section_layer() {
    let (e, _dir) = demo();
    for mode in [GroupBy::None, GroupBy::Project, GroupBy::Tag, GroupBy::Estimate] {
        e.set_preferences(Preferences {
            group_by: mode,
            sort: SortKey::Title,
            ..Default::default()
        });
        let l = e.listing(View::Today, 100);
        assert!(
            l.sections.iter().all(|s| s.kind == SectionKind::Plain),
            "{mode:?} should not nest under time-of-day or overdue headings"
        );
        let groups: Vec<_> = l.sections.iter().map(|s| s.group.clone()).collect();
        assert!(
            groups.iter().enumerate().all(|(i, g)| !groups[..i].contains(g)),
            "each group appears once"
        );
    }
}

#[test]
fn morning_night_is_the_default_and_retains_every_task_without_writing() {
    let (e, _dir) = demo();
    assert_eq!(format!("{:?}", e.preferences().group_by), "MorningNight");
    let pending = e.pending_count();
    let l = e.listing(View::Today, 100);
    assert_eq!(l.sections.len(), 3, "Today / Morning / Evening only");
    assert!(l.sections.iter().all(|s| s.kind == SectionKind::Plain));
    assert_eq!(row_ids(&l).len(), 8);
    assert_eq!(e.pending_count(), pending);
}

#[test]
fn morning_night_is_plain_outside_day_views() {
    let (e, _dir) = empty();
    let project = e.add_project("Plain context project".into()).unwrap();
    let tag = e.add_tag("Plain context tag".into()).unwrap();
    let due_day = day_str(today_n() + 1);
    let mut task = Task::new("Future plain context sentinel", &project);
    task.due_day = Some(due_day.clone());
    task.tag_ids = vec![tag.clone()];
    e.dispatch(Action::AddTask { task, bottom: true });

    for (context, listing, expected_kind) in [
        ("Upcoming", e.listing(View::Upcoming, 100), SectionKind::Plain),
        ("project", e.listing(View::project(&project), 100), SectionKind::Plain),
        ("tag", e.listing(View::tag(&tag), 100), SectionKind::Plain),
        (
            "search",
            e.search("Future plain context sentinel".into()),
            SectionKind::SearchTasks,
        ),
    ] {
        assert_eq!(listing.sections.len(), 1, "{context} keeps one section");
        let section = &listing.sections[0];
        assert_eq!(section.kind, expected_kind, "{context} keeps its section kind");
        assert_eq!(section.group, None, "{context} stays ungrouped");
        assert_eq!(section.rows.len(), 1, "{context} keeps the task once");
        assert!(
            matches!(
                &section.rows[0],
                Row::Task { row }
                    if row.day.is_some() && row.due_day.as_deref() == Some(due_day.as_str())
            ),
            "{context} retains visible and raw due-day metadata"
        );
    }
}

#[test]
fn morning_night_matches_tags_case_insensitively_and_keeps_families_together() {
    let (e, _dir) = empty();
    e.add_task("Any time".into(), View::Today);
    e.add_task("Both #Evening #Morning".into(), View::Today);
    e.add_task("Later #Evening".into(), View::Today);
    let both = id_of(&e, "Both");
    let later = id_of(&e, "Later");
    e.add_subtask(both.clone(), "Child without tags".into());
    let child = id_of(&e, "Child without tags");
    e.with_store_mut(|s| {
        for tag in s.state.tag.entities.values_mut() {
            if tag.title == "Morning" {
                tag.title = "mOrNiNg".into();
            }
            if tag.title == "Evening" {
                tag.title = "eVeNiNg".into();
            }
        }
    });
    let l = e.listing(View::Today, 100);
    assert_eq!(
        l.sections.iter().map(|s| s.group.clone()).collect::<Vec<_>>(),
        vec![
            Some(TaskGroup::Today),
            Some(TaskGroup::Morning),
            Some(TaskGroup::Evening)
        ]
    );
    assert_eq!(
        row_ids(&Listing {
            sections: vec![l.sections[1].clone()],
            ..l.clone()
        }),
        [both.clone(), child]
    );
    assert_eq!(e.task_menu(both.clone()).unwrap().slot, Some(Slot::Morning));
    assert!(!e.reorder(both.clone(), later, View::Today).changed);
    e.drop_tasks(vec![both.clone()], View::Tonight);
    let moved = e.listing(View::Today, 100);
    assert!(!moved.sections.iter().any(|s| s.group == Some(TaskGroup::Morning)));
    e.undo();
    assert_eq!(e.listing(View::Today, 100), l);
    e.toggle_slot(vec![both], Slot::Tonight);
    assert!(!e
        .listing(View::Today, 100)
        .sections
        .iter()
        .any(|s| s.group == Some(TaskGroup::Morning)));
    e.undo();
    assert_eq!(e.listing(View::Today, 100), l);
}

#[test]
fn exclusive_grouping_preserves_completed_tasks_as_one_separate_section() {
    let (e, _dir) = demo();
    let done = vec![id_of(&e, "Read two chapters"), id_of(&e, "Write release notes for 0.1")];
    e.bulk_done(done.clone());
    for mode in [
        GroupBy::MorningNight,
        GroupBy::None,
        GroupBy::Project,
        GroupBy::Tag,
        GroupBy::Estimate,
    ] {
        e.set_preferences(Preferences {
            group_by: mode,
            ..Default::default()
        });
        let l = e.listing(View::Today, 100);
        let completed: Vec<_> = l.sections.iter().filter(|s| s.kind == SectionKind::Completed).collect();
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].count, 2);
        assert!(completed[0].group.is_none());
        assert_eq!(l.sections.last().unwrap().kind, SectionKind::Completed);
        assert_eq!(row_ids(&l).iter().filter(|id| done.contains(id)).count(), 2);
    }
}

#[test]
fn exclusive_upcoming_groups_span_dates_without_hiding_due_days() {
    let (e, _dir) = empty();
    for (name, offset) in [("Tomorrow", 1), ("Later", 3)] {
        let mut t = Task::new(name, INBOX_PROJECT_ID);
        t.due_day = Some(day_str(today_n() + offset));
        e.dispatch(Action::AddTask { task: t, bottom: true });
    }
    for mode in [
        GroupBy::MorningNight,
        GroupBy::None,
        GroupBy::Project,
        GroupBy::Tag,
        GroupBy::Estimate,
    ] {
        e.set_preferences(Preferences {
            group_by: mode,
            ..Default::default()
        });
        let l = e.listing(View::Upcoming, 100);
        assert_eq!(l.sections.len(), 1, "same group spans both dates in {mode:?}");
        assert_eq!(l.sections[0].kind, SectionKind::Plain);
        assert_eq!(l.sections[0].rows.len(), 2);
        assert!(l.sections[0]
            .rows
            .iter()
            .all(|r| matches!(r, Row::Task { row } if row.day.is_some())));
    }
}
