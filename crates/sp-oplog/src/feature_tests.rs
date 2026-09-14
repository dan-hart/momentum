// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Every action: its upstream payload, its op envelope, and what `apply` does to the state.
use super::*;
use sp_model::{today_str, AppData, Project, RepeatCfg, Tag, Task, INBOX_PROJECT_ID, TODAY_TAG_ID};

fn changes(pairs: &[(&str, Value)]) -> Map<String, Value> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
}
fn add(d: &mut AppData, title: &str, project: &str) -> Task {
    let t = Task::new(title, project);
    apply(
        d,
        &Action::AddTask {
            task: t.clone(),
            bottom: true,
        },
    );
    t
}
fn project_list(d: &AppData, p: &str) -> Vec<String> {
    d.project.entities[p].task_ids.clone()
}
fn today_list(d: &AppData) -> Vec<String> {
    d.tag.entities[TODAY_TAG_ID].task_ids.clone()
}

// ---- envelope ---------------------------------------------------------------

#[test]
fn every_action_has_an_upstream_code_and_compact_keys() {
    let t = Task::new("t", INBOX_PROJECT_ID);
    let cfg = RepeatCfg::for_task(&t);
    let all = vec![
        Action::AddTask {
            task: t.clone(),
            bottom: true,
        },
        Action::AddSubTask {
            task: t.clone(),
            parent_id: "p".into(),
        },
        Action::UpdateTask {
            id: t.id.clone(),
            changes: changes(&[("title", json!("x"))]),
        },
        Action::DeleteTask {
            task: t.clone(),
            sub_tasks: vec![],
        },
        Action::PlanForToday {
            task_ids: vec![t.id.clone()],
            today: today_str(),
        },
        Action::MoveToProject {
            task: t.clone(),
            sub_tasks: vec![],
            target_project_id: "q".into(),
        },
        Action::MoveToArchive {
            tasks: vec![t.clone()],
            sub_tasks: vec![],
        },
        Action::DeleteProject {
            project_id: "q".into(),
            note_ids: vec![],
            all_task_ids: vec![],
        },
        Action::DeleteTag { id: "g".into() },
        Action::MoveInList {
            task_id: t.id.clone(),
            after_task_id: None,
            context_type: "PROJECT".into(),
            context_id: INBOX_PROJECT_ID.into(),
        },
        Action::RestoreTask {
            task: t.clone(),
            sub_tasks: vec![],
        },
        Action::UpdateRepeatCfg {
            id: cfg.id.clone(),
            changes: changes(&[("isPaused", json!(true))]),
        },
        Action::AddRepeatCfg {
            task_id: t.id.clone(),
            cfg: cfg.clone(),
        },
        Action::DeleteRepeatCfg { id: cfg.id.clone() },
        Action::RemoveFromToday {
            task_ids: vec![t.id.clone()],
        },
        Action::AddProject {
            project: Project::new("P"),
        },
        Action::UpdateProject {
            id: "q".into(),
            changes: changes(&[("title", json!("Q"))]),
        },
        Action::AddTag { tag: Tag::new("g") },
        Action::UpdateTag {
            id: "g".into(),
            changes: changes(&[("title", json!("G"))]),
        },
    ];
    let mut clock = VectorClock::new();
    for (i, a) in all.iter().enumerate() {
        let (name, _, _, _, _) = a.parts();
        assert!(ACTION_CODES.iter().any(|(n, _)| *n == name), "no code for {name}");
        let op = a.to_op("client", &mut clock);
        assert_eq!(clock["client"], i as u64 + 1, "vector clock ticks per op");
        assert_eq!(op.v["client"], i as u64 + 1);
        assert_eq!(op.s, sp_model::SCHEMA_VERSION);
        let v = serde_json::to_value(&op).unwrap();
        for k in ["id", "a", "o", "e", "p", "c", "v", "t", "s"] {
            assert!(v.get(k).is_some(), "op key {k} missing for {name}");
        }
        assert!(v.get("sv").is_none(), "sync version is only set on upload");
        // Our own serialized form round-trips (it is what pending.json and peers carry).
        let back: Action = serde_json::from_value(serde_json::to_value(a).unwrap()).unwrap();
        assert_eq!(back.parts().0, name);
    }
}
#[test]
fn payload_shapes_match_upstream_reducers() {
    let t = Task::new("t", "proj");
    let (_, o, e, ids, p) = Action::AddTask {
        task: t.clone(),
        bottom: false,
    }
    .parts();
    assert_eq!((o, e), ("CRT", "TASK"));
    assert_eq!(ids, vec![t.id.clone()]);
    assert_eq!(
        p["isIgnoreShortSyntax"], true,
        "titles with #tags must not be re-parsed upstream"
    );
    assert_eq!(p["isAddToBottom"], false);
    assert_eq!(p["workContextId"], "proj");
    let (_, _, _, _, p) = Action::DeleteProject {
        project_id: "q".into(),
        note_ids: vec![],
        all_task_ids: vec!["a".into()],
    }
    .parts();
    assert_eq!(p["projectDeleteWins"], true);
    let del = Action::DeleteTask {
        task: t.clone(),
        sub_tasks: vec![Task::new("s", "proj")],
    };
    let (_, _, _, ids, p) = del.parts();
    assert_eq!(
        ids,
        vec![t.id.clone()],
        "the op targets the parent; subtasks ride in the payload"
    );
    assert_eq!(p["task"]["subTasks"].as_array().map(|v| v.len()), Some(1));
    let op = del.to_op("c", &mut VectorClock::new());
    assert_eq!(op.d.as_deref(), Some(t.id.as_str()));
    assert!(op.ds.is_none());
    let (_, _, _, _, p) = Action::DeleteRepeatCfg { id: "r".into() }.parts();
    assert_eq!(p["taskRepeatCfgId"], "r");
}
#[test]
fn merge_clocks_takes_the_maximum_per_client() {
    let a: VectorClock = [("x".to_string(), 3u64), ("y".to_string(), 1)].into_iter().collect();
    let b: VectorClock = [("y".to_string(), 5u64), ("z".to_string(), 2)].into_iter().collect();
    let m = merge_clocks(&a, &b);
    assert_eq!((m["x"], m["y"], m["z"]), (3, 5, 2));
}

// ---- apply: tasks ------------------------------------------------------------

#[test]
fn add_task_top_or_bottom_and_subtasks() {
    let mut d = AppData::fresh();
    let a = add(&mut d, "a", INBOX_PROJECT_ID);
    let b = add(&mut d, "b", INBOX_PROJECT_ID);
    let top = Task::new("top", INBOX_PROJECT_ID);
    apply(
        &mut d,
        &Action::AddTask {
            task: top.clone(),
            bottom: false,
        },
    );
    assert_eq!(
        project_list(&d, INBOX_PROJECT_ID),
        vec![top.id.clone(), a.id.clone(), b.id.clone()]
    );
    let sub = Task::new("sub", INBOX_PROJECT_ID);
    apply(
        &mut d,
        &Action::AddSubTask {
            task: sub.clone(),
            parent_id: a.id.clone(),
        },
    );
    assert_eq!(d.task.entities[&a.id].sub_task_ids, vec![sub.id.clone()]);
    assert_eq!(d.task.entities[&sub.id].parent_id.as_deref(), Some(a.id.as_str()));
    assert!(
        !project_list(&d, INBOX_PROJECT_ID).contains(&sub.id),
        "subtasks are not project list members"
    );
}
#[test]
fn update_task_done_stamps_done_on_and_clears_it_again() {
    let mut d = AppData::fresh();
    let t = add(&mut d, "t", INBOX_PROJECT_ID);
    apply(
        &mut d,
        &Action::UpdateTask {
            id: t.id.clone(),
            changes: changes(&[("isDone", json!(true))]),
        },
    );
    let done = &d.task.entities[&t.id];
    assert!(done.is_done && done.done_on.is_some() && done.modified.is_some());
    apply(
        &mut d,
        &Action::UpdateTask {
            id: t.id.clone(),
            changes: changes(&[("isDone", json!(false))]),
        },
    );
    assert!(d.task.entities[&t.id].done_on.is_none());
    // Unknown ids are ignored, not created.
    apply(
        &mut d,
        &Action::UpdateTask {
            id: "ghost".into(),
            changes: changes(&[("title", json!("x"))]),
        },
    );
    assert!(d.task.entities.get("ghost").is_none());
}
#[test]
fn update_task_keeps_tag_and_today_lists_in_sync() {
    let mut d = AppData::fresh();
    let g = Tag::new("g");
    apply(&mut d, &Action::AddTag { tag: g.clone() });
    let t = add(&mut d, "t", INBOX_PROJECT_ID);
    apply(
        &mut d,
        &Action::UpdateTask {
            id: t.id.clone(),
            changes: changes(&[("tagIds", json!([g.id, TODAY_TAG_ID]))]),
        },
    );
    assert_eq!(d.tag.entities[&g.id].task_ids, vec![t.id.clone()]);
    assert_eq!(
        d.task.entities[&t.id].tag_ids,
        vec![g.id.clone()],
        "TODAY never lands in tagIds"
    );
    apply(
        &mut d,
        &Action::UpdateTask {
            id: t.id.clone(),
            changes: changes(&[("dueDay", json!(today_str()))]),
        },
    );
    assert_eq!(today_list(&d), vec![t.id.clone()]);
    apply(
        &mut d,
        &Action::UpdateTask {
            id: t.id.clone(),
            changes: changes(&[("dueDay", json!("2030-01-01"))]),
        },
    );
    assert!(today_list(&d).is_empty());
    apply(
        &mut d,
        &Action::UpdateTask {
            id: t.id.clone(),
            changes: changes(&[("tagIds", json!([]))]),
        },
    );
    assert!(d.tag.entities[&g.id].task_ids.is_empty());
}
#[test]
fn update_task_project_change_moves_between_lists() {
    let mut d = AppData::fresh();
    let p = Project::new("P");
    apply(&mut d, &Action::AddProject { project: p.clone() });
    let t = add(&mut d, "t", INBOX_PROJECT_ID);
    apply(
        &mut d,
        &Action::UpdateTask {
            id: t.id.clone(),
            changes: changes(&[("projectId", json!(p.id))]),
        },
    );
    assert!(project_list(&d, INBOX_PROJECT_ID).is_empty());
    assert_eq!(project_list(&d, &p.id), vec![t.id.clone()]);
}
#[test]
fn delete_task_removes_subtasks_and_every_membership() {
    let mut d = AppData::fresh();
    let g = Tag::new("g");
    apply(&mut d, &Action::AddTag { tag: g.clone() });
    let mut t = Task::new("t", INBOX_PROJECT_ID);
    t.tag_ids = vec![g.id.clone()];
    t.due_day = Some(today_str());
    apply(
        &mut d,
        &Action::AddTask {
            task: t.clone(),
            bottom: true,
        },
    );
    let sub = Task::new("s", INBOX_PROJECT_ID);
    apply(
        &mut d,
        &Action::AddSubTask {
            task: sub.clone(),
            parent_id: t.id.clone(),
        },
    );
    let (parent, child) = (d.task.entities[&t.id].clone(), d.task.entities[&sub.id].clone());
    apply(
        &mut d,
        &Action::DeleteTask {
            task: parent,
            sub_tasks: vec![child],
        },
    );
    assert!(d.task.ids.is_empty());
    assert!(project_list(&d, INBOX_PROJECT_ID).is_empty() && today_list(&d).is_empty());
    assert!(d.tag.entities[&g.id].task_ids.is_empty());
}
#[test]
fn plan_for_today_and_remove_from_today() {
    let mut d = AppData::fresh();
    let mut t = Task::new("t", INBOX_PROJECT_ID);
    t.due_with_time = Some(1);
    t.remind_at = Some(1);
    apply(
        &mut d,
        &Action::AddTask {
            task: t.clone(),
            bottom: true,
        },
    );
    apply(
        &mut d,
        &Action::PlanForToday {
            task_ids: vec![t.id.clone()],
            today: today_str(),
        },
    );
    let planned = &d.task.entities[&t.id];
    assert_eq!(planned.due_day.as_deref(), Some(today_str().as_str()));
    assert!(
        planned.due_with_time.is_none() && planned.remind_at.is_none(),
        "a day plan clears the time and reminder"
    );
    assert_eq!(today_list(&d), vec![t.id.clone()]);
    apply(
        &mut d,
        &Action::RemoveFromToday {
            task_ids: vec![t.id.clone()],
        },
    );
    assert!(d.task.entities[&t.id].due_day.is_none() && today_list(&d).is_empty());
}
#[test]
fn move_to_project_carries_subtasks() {
    let mut d = AppData::fresh();
    let p = Project::new("P");
    apply(&mut d, &Action::AddProject { project: p.clone() });
    let t = add(&mut d, "t", INBOX_PROJECT_ID);
    let sub = Task::new("s", INBOX_PROJECT_ID);
    apply(
        &mut d,
        &Action::AddSubTask {
            task: sub.clone(),
            parent_id: t.id.clone(),
        },
    );
    apply(
        &mut d,
        &Action::MoveToProject {
            task: t.clone(),
            sub_tasks: vec![sub.clone()],
            target_project_id: p.id.clone(),
        },
    );
    assert_eq!(project_list(&d, &p.id), vec![t.id.clone()]);
    assert!(project_list(&d, INBOX_PROJECT_ID).is_empty());
    assert_eq!(d.task.entities[&sub.id].project_id, p.id);
}
#[test]
fn archive_and_restore_round_trip() {
    let mut d = AppData::fresh();
    let t = add(&mut d, "t", INBOX_PROJECT_ID);
    let sub = Task::new("s", INBOX_PROJECT_ID);
    apply(
        &mut d,
        &Action::AddSubTask {
            task: sub.clone(),
            parent_id: t.id.clone(),
        },
    );
    apply(
        &mut d,
        &Action::UpdateTask {
            id: t.id.clone(),
            changes: changes(&[("isDone", json!(true))]),
        },
    );
    let done = d.task.entities[&t.id].clone();
    let s = d.task.entities[&sub.id].clone();
    apply(
        &mut d,
        &Action::MoveToArchive {
            tasks: vec![done.clone()],
            sub_tasks: vec![s.clone()],
        },
    );
    assert!(d.task.ids.is_empty() && project_list(&d, INBOX_PROJECT_ID).is_empty());
    let archive = &d.rest["archiveYoung"]["task"];
    assert_eq!(archive["ids"].as_array().unwrap().len(), 2);
    assert_eq!(archive["entities"][&t.id]["title"], "t");
    assert!(
        d.rest["archiveYoung"]["timeTracking"].is_object(),
        "upstream archive shape"
    );
    apply(
        &mut d,
        &Action::RestoreTask {
            task: done,
            sub_tasks: vec![s],
        },
    );
    assert!(d.rest["archiveYoung"]["task"]["ids"].as_array().unwrap().is_empty());
    let back = &d.task.entities[&t.id];
    assert!(!back.is_done && back.done_on.is_none(), "restored tasks are open again");
    assert_eq!(back.sub_task_ids, vec![sub.id.clone()]);
    assert_eq!(project_list(&d, INBOX_PROJECT_ID), vec![t.id.clone()]);
}
#[test]
fn move_in_list_reorders_projects_and_today() {
    let mut d = AppData::fresh();
    let a = add(&mut d, "a", INBOX_PROJECT_ID);
    let b = add(&mut d, "b", INBOX_PROJECT_ID);
    let c = add(&mut d, "c", INBOX_PROJECT_ID);
    let mv = |after: Option<&str>| Action::MoveInList {
        task_id: c.id.clone(),
        after_task_id: after.map(String::from),
        context_type: "PROJECT".into(),
        context_id: INBOX_PROJECT_ID.into(),
    };
    apply(&mut d, &mv(None));
    assert_eq!(
        project_list(&d, INBOX_PROJECT_ID),
        vec![c.id.clone(), a.id.clone(), b.id.clone()]
    );
    apply(&mut d, &mv(Some(&a.id)));
    assert_eq!(
        project_list(&d, INBOX_PROJECT_ID),
        vec![a.id.clone(), c.id.clone(), b.id.clone()]
    );
    apply(&mut d, &mv(Some("ghost")));
    assert_eq!(
        project_list(&d, INBOX_PROJECT_ID)[0],
        c.id,
        "unknown anchor puts it first"
    );
    apply(
        &mut d,
        &Action::PlanForToday {
            task_ids: vec![a.id.clone(), b.id.clone()],
            today: today_str(),
        },
    );
    apply(
        &mut d,
        &Action::MoveInList {
            task_id: b.id.clone(),
            after_task_id: None,
            context_type: "TAG".into(),
            context_id: TODAY_TAG_ID.into(),
        },
    );
    assert_eq!(today_list(&d), vec![b.id.clone(), a.id.clone()]);
}

// ---- apply: projects, tags, repeats -----------------------------------------------

#[test]
fn delete_project_removes_its_tasks_everywhere_including_archives() {
    let mut d = AppData::fresh();
    let p = Project::new("P");
    apply(&mut d, &Action::AddProject { project: p.clone() });
    let t = add(&mut d, "t", &p.id);
    let keep = add(&mut d, "keep", INBOX_PROJECT_ID);
    d.rest.insert("archiveYoung".into(), json!({"task": {"ids": ["arch", "other"], "entities": {
        "arch": {"id": "arch", "title": "old", "projectId": p.id}, "other": {"id": "other", "title": "o", "projectId": INBOX_PROJECT_ID}}}}));
    apply(
        &mut d,
        &Action::DeleteProject {
            project_id: p.id.clone(),
            note_ids: vec![],
            all_task_ids: vec![t.id.clone()],
        },
    );
    assert!(d.project.entities.get(&p.id).is_none());
    assert_eq!(d.task.ids, vec![keep.id.clone()]);
    assert_eq!(d.rest["archiveYoung"]["task"]["ids"], json!(["other"]));
}
#[test]
fn delete_tag_strips_it_from_tasks() {
    let mut d = AppData::fresh();
    let g = Tag::new("g");
    apply(&mut d, &Action::AddTag { tag: g.clone() });
    let mut t = Task::new("t", INBOX_PROJECT_ID);
    t.tag_ids = vec![g.id.clone()];
    apply(
        &mut d,
        &Action::AddTask {
            task: t.clone(),
            bottom: true,
        },
    );
    apply(&mut d, &Action::DeleteTag { id: g.id.clone() });
    assert!(d.tag.entities.get(&g.id).is_none());
    assert!(d.task.entities[&t.id].tag_ids.is_empty());
}
#[test]
fn update_project_and_tag_merge_changes() {
    let mut d = AppData::fresh();
    let g = Tag::new("g");
    apply(&mut d, &Action::AddTag { tag: g.clone() });
    apply(
        &mut d,
        &Action::UpdateTag {
            id: g.id.clone(),
            changes: changes(&[("title", json!("G!")), ("color", json!("#123456"))]),
        },
    );
    assert_eq!(d.tag.entities[&g.id].title, "G!");
    assert_eq!(d.tag.entities[&g.id].color.as_deref(), Some("#123456"));
    apply(
        &mut d,
        &Action::UpdateProject {
            id: INBOX_PROJECT_ID.into(),
            changes: changes(&[("title", json!("In"))]),
        },
    );
    assert_eq!(d.project.entities[INBOX_PROJECT_ID].title, "In");
    apply(
        &mut d,
        &Action::UpdateProject {
            id: "ghost".into(),
            changes: changes(&[("title", json!("x"))]),
        },
    );
    assert!(d.project.entities.get("ghost").is_none());
}
#[test]
fn repeat_cfg_lifecycle() {
    let mut d = AppData::fresh();
    let t = add(&mut d, "t", INBOX_PROJECT_ID);
    let cfg = RepeatCfg::for_task(&t);
    apply(
        &mut d,
        &Action::AddRepeatCfg {
            task_id: t.id.clone(),
            cfg: cfg.clone(),
        },
    );
    assert_eq!(d.task.entities[&t.id].repeat_cfg_id.as_deref(), Some(cfg.id.as_str()));
    apply(
        &mut d,
        &Action::UpdateRepeatCfg {
            id: cfg.id.clone(),
            changes: changes(&[("repeatCycle", json!("DAILY")), ("isPaused", json!(true))]),
        },
    );
    let c = &d.task_repeat_cfg.entities[&cfg.id];
    assert!(c.is_paused && c.repeat_cycle == "DAILY");
    apply(&mut d, &Action::DeleteRepeatCfg { id: cfg.id.clone() });
    assert!(d.task_repeat_cfg.entities.is_empty());
    assert!(d.task.entities[&t.id].repeat_cfg_id.is_none());
}
