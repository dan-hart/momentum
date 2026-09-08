// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Operation log: compact op envelope, vector clocks, the actions this client
//! emits, and `apply()` for them. Mirrors upstream `packages/sync-core` and the
//! `task-shared` meta reducers (MIT, (c) 2018 Johannes Millan).
mod action_codes;
pub use action_codes::ACTION_CODES;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sp_model::*;
use std::collections::BTreeMap;

pub type VectorClock = BTreeMap<String, u64>;
pub fn merge_clocks(a: &VectorClock, b: &VectorClock) -> VectorClock {
    let mut m = a.clone();
    for (k, v) in b {
        let e = m.entry(k.clone()).or_default();
        *e = (*e).max(*v);
    }
    m
}

/// Compact wire/storage form of an operation (`compact-operation.types.ts`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Op {
    pub id: String,
    /// Short action code (see `ACTION_CODES`).
    pub a: String,
    pub o: String,
    pub e: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub d: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ds: Option<Vec<String>>,
    pub p: Value,
    pub c: String,
    pub v: VectorClock,
    pub t: u64,
    pub s: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sv: Option<u64>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Actions this client can emit. Serialized form is our own; `payload()` produces upstream's.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    AddTask {
        task: Task,
        bottom: bool,
    },
    AddSubTask {
        task: Task,
        parent_id: String,
    },
    UpdateTask {
        id: String,
        changes: Map<String, Value>,
    },
    DeleteTask {
        task: Task,
        sub_tasks: Vec<Task>,
    },
    PlanForToday {
        task_ids: Vec<String>,
        today: String,
    },
    MoveToProject {
        task: Task,
        sub_tasks: Vec<Task>,
        target_project_id: String,
    },
    /// Done tasks (with their subtasks) moved to the young archive, upstream "finish day".
    MoveToArchive {
        tasks: Vec<Task>,
        sub_tasks: Vec<Task>,
    },
    RemoveFromToday {
        task_ids: Vec<String>,
    },
    AddProject {
        project: Project,
    },
    UpdateProject {
        id: String,
        changes: Map<String, Value>,
    },
    AddTag {
        tag: Tag,
    },
    UpdateTag {
        id: String,
        changes: Map<String, Value>,
    },
}

fn with_subs(t: &Task, subs: &[Task]) -> Value {
    let mut v = json!(t);
    v["subTasks"] = json!(subs);
    v
}

impl Action {
    /// (actionType, opType, entityType, entityIds, payload) exactly as upstream captures them.
    pub fn parts(&self) -> (&'static str, &'static str, &'static str, Vec<String>, Value) {
        use Action::*;
        match self {
            AddTask { task, bottom } => (
                "[Task Shared] addTask",
                "CRT",
                "TASK",
                vec![task.id.clone()],
                json!({"task": task, "workContextId": task.project_id, "workContextType": "PROJECT", "isAddToBacklog": false, "isAddToBottom": bottom, "isIgnoreShortSyntax": true}),
            ),
            AddSubTask { task, parent_id } => (
                "[Task] Add SubTask",
                "CRT",
                "TASK",
                vec![task.id.clone()],
                json!({"task": task, "parentId": parent_id}),
            ),
            UpdateTask { id, changes } => (
                "[Task Shared] updateTask",
                "UPD",
                "TASK",
                vec![id.clone()],
                json!({"task": {"id": id, "changes": changes}}),
            ),
            DeleteTask { task, sub_tasks } => (
                "[Task Shared] deleteTask",
                "DEL",
                "TASK",
                vec![task.id.clone()],
                json!({"task": with_subs(task, sub_tasks)}),
            ),
            PlanForToday { task_ids, today } => (
                "[Task Shared] planTasksForToday",
                "UPD",
                "TASK",
                task_ids.clone(),
                json!({"taskIds": task_ids, "today": today}),
            ),
            MoveToProject {
                task,
                sub_tasks,
                target_project_id,
            } => (
                "[Task Shared] moveToOtherProject",
                "UPD",
                "TASK",
                vec![task.id.clone()],
                json!({"task": with_subs(task, sub_tasks), "targetProjectId": target_project_id}),
            ),
            MoveToArchive { tasks, sub_tasks } => (
                "[Task Shared] moveToArchive",
                "UPD",
                "TASK",
                tasks.iter().map(|t| t.id.clone()).collect(),
                json!({"tasks": tasks.iter().map(|t| with_subs(t, &sub_tasks.iter().filter(|s| s.parent_id.as_deref() == Some(&t.id)).cloned().collect::<Vec<_>>())).collect::<Vec<_>>()}),
            ),
            RemoveFromToday { task_ids } => (
                "[Task Shared] removeTasksFromTodayTag",
                "UPD",
                "TASK",
                task_ids.clone(),
                json!({"taskIds": task_ids}),
            ),
            AddProject { project } => (
                "[Project] Add Project",
                "CRT",
                "PROJECT",
                vec![project.id.clone()],
                json!({"project": project}),
            ),
            UpdateProject { id, changes } => (
                "[Project] Update Project",
                "UPD",
                "PROJECT",
                vec![id.clone()],
                json!({"project": {"id": id, "changes": changes}}),
            ),
            AddTag { tag } => ("[Tag] Add Tag", "CRT", "TAG", vec![tag.id.clone()], json!({"tag": tag})),
            UpdateTag { id, changes } => (
                "[Tag] Update Tag",
                "UPD",
                "TAG",
                vec![id.clone()],
                json!({"tag": {"id": id, "changes": changes}}),
            ),
        }
    }

    /// Build the compact op, incrementing this client's clock entry.
    pub fn to_op(&self, client_id: &str, clock: &mut VectorClock) -> Op {
        *clock.entry(client_id.into()).or_default() += 1;
        let (a, o, e, ids, p) = self.parts();
        let code = ACTION_CODES
            .iter()
            .find(|(n, _)| *n == a)
            .map(|(_, c)| *c)
            .expect("action code");
        Op {
            id: new_id(),
            a: code.into(),
            o: o.into(),
            e: e.into(),
            d: ids.first().cloned(),
            ds: (ids.len() > 1).then(|| ids.clone()),
            p,
            c: client_id.into(),
            v: clock.clone(),
            t: now_ms(),
            s: SCHEMA_VERSION,
            sv: None,
            extra: Map::new(),
        }
    }
}

fn merge_into<T: Serialize + for<'a> Deserialize<'a>>(t: &T, changes: &Map<String, Value>) -> T {
    let mut v = serde_json::to_value(t).unwrap();
    for (k, c) in changes {
        v[k] = c.clone();
    }
    serde_json::from_value(v).unwrap()
}
fn list_set(list: &mut Vec<String>, id: &str, present: bool) {
    match (list.iter().position(|i| i == id), present) {
        (None, true) => list.push(id.into()),
        (Some(i), false) => {
            list.remove(i);
        }
        _ => {}
    }
}
fn set_today(d: &mut AppData, id: &str, present: bool) {
    if let Some(t) = d.tag.entities.get_mut(TODAY_TAG_ID) {
        list_set(&mut t.task_ids, id, present);
    }
}
fn detach(d: &mut AppData, t: &Task) {
    if let Some(p) = d.project.entities.get_mut(&t.project_id) {
        list_set(&mut p.task_ids, &t.id, false);
        list_set(&mut p.backlog_task_ids, &t.id, false);
    }
    for tag in d.tag.entities.values_mut() {
        list_set(&mut tag.task_ids, &t.id, false);
    }
    if let Some(pid) = &t.parent_id {
        if let Some(p) = d.task.entities.get_mut(pid) {
            list_set(&mut p.sub_task_ids, &t.id, false);
        }
    }
}

/// Apply an action to local state, following upstream reducer semantics for these actions.
pub fn apply(d: &mut AppData, action: &Action) {
    use Action::*;
    let today = today_str();
    match action {
        AddTask { task, bottom } => {
            let mut t = task.clone();
            t.tag_ids.retain(|i| i != TODAY_TAG_ID);
            t.recalc_time_spent();
            if let Some(p) = d.project.entities.get_mut(&t.project_id) {
                if *bottom {
                    p.task_ids.push(t.id.clone())
                } else {
                    p.task_ids.insert(0, t.id.clone())
                }
            }
            for tag in &t.tag_ids {
                if let Some(g) = d.tag.entities.get_mut(tag) {
                    g.task_ids.insert(0, t.id.clone());
                }
            }
            if t.due_day.as_deref() == Some(&today) {
                set_today(d, &t.id, true);
            }
            d.task.insert(&t.id.clone(), t);
        }
        AddSubTask { task, parent_id } => {
            let mut t = task.clone();
            t.parent_id = Some(parent_id.clone());
            if let Some(p) = d.task.entities.get_mut(parent_id) {
                t.project_id = p.project_id.clone();
                p.sub_task_ids.push(t.id.clone());
            }
            d.task.insert(&t.id.clone(), t);
        }
        UpdateTask { id, changes } => {
            let Some(old) = d.task.entities.get(id).cloned() else {
                return;
            };
            let mut t: Task = merge_into(&old, changes);
            t.tag_ids.retain(|i| i != TODAY_TAG_ID);
            if t.is_done && !old.is_done {
                t.done_on = Some(now_ms());
            } else if !t.is_done {
                t.done_on = None;
            }
            t.modified = Some(now_ms());
            if t.tag_ids != old.tag_ids {
                for g in d.tag.entities.values_mut() {
                    if g.id != TODAY_TAG_ID {
                        list_set(&mut g.task_ids, id, t.tag_ids.contains(&g.id));
                    }
                }
            }
            if t.due_day != old.due_day {
                let on = t.due_day.as_deref() == Some(&today);
                set_today(d, id, on && t.parent_id.is_none());
            }
            if t.project_id != old.project_id {
                if let Some(p) = d.project.entities.get_mut(&old.project_id) {
                    list_set(&mut p.task_ids, id, false);
                }
                if t.parent_id.is_none() {
                    if let Some(p) = d.project.entities.get_mut(&t.project_id) {
                        list_set(&mut p.task_ids, id, true);
                    }
                }
            }
            d.task.entities.insert(id.clone(), t);
        }
        DeleteTask { task, sub_tasks } => {
            for s in sub_tasks {
                detach(d, s);
                d.task.remove(&s.id);
            }
            detach(d, task);
            d.task.remove(&task.id);
        }
        PlanForToday { task_ids, today: day } => {
            for id in task_ids {
                if let Some(t) = d.task.entities.get_mut(id) {
                    t.due_day = Some(day.clone());
                    t.due_with_time = None;
                    t.remind_at = None;
                    t.modified = Some(now_ms());
                }
                set_today(d, id, true);
            }
        }
        MoveToProject {
            task,
            sub_tasks,
            target_project_id,
        } => {
            if let Some(p) = d.project.entities.get_mut(&task.project_id) {
                list_set(&mut p.task_ids, &task.id, false);
                list_set(&mut p.backlog_task_ids, &task.id, false);
            }
            if let Some(p) = d.project.entities.get_mut(target_project_id) {
                list_set(&mut p.task_ids, &task.id, true);
            }
            for id in std::iter::once(&task.id).chain(sub_tasks.iter().map(|t| &t.id)) {
                if let Some(t) = d.task.entities.get_mut(id) {
                    t.project_id = target_project_id.clone();
                    t.modified = Some(now_ms());
                }
            }
        }
        MoveToArchive { tasks, sub_tasks } => {
            let archive = d.rest.entry("archiveYoung".to_string()).or_insert_with(|| json!({}));
            if !archive.is_object() {
                *archive = json!({});
            }
            let a = archive.as_object_mut().unwrap();
            a.entry("timeTracking")
                .or_insert_with(|| json!({"project": {}, "tag": {}}));
            a.entry("lastTimeTrackingFlush").or_insert_with(|| json!(0));
            let store = a.entry("task").or_insert_with(|| json!({"ids": [], "entities": {}}));
            for t in tasks.iter().chain(sub_tasks.iter()) {
                if let Some(ids) = store["ids"].as_array_mut() {
                    if !ids.iter().any(|i| i == &t.id) {
                        ids.push(json!(t.id));
                    }
                }
                store["entities"][&t.id] = json!(t);
            }
            for t in sub_tasks.iter().chain(tasks.iter()) {
                detach(d, t);
                d.task.remove(&t.id);
            }
        }
        RemoveFromToday { task_ids } => {
            for id in task_ids {
                if let Some(t) = d.task.entities.get_mut(id) {
                    t.due_day = None;
                }
                set_today(d, id, false);
            }
        }
        AddProject { project } => d.project.insert(&project.id.clone(), project.clone()),
        UpdateProject { id, changes } => {
            if let Some(p) = d.project.entities.get(id).cloned() {
                d.project.entities.insert(id.clone(), merge_into(&p, changes));
            }
        }
        AddTag { tag } => d.tag.insert(&tag.id.clone(), tag.clone()),
        UpdateTag { id, changes } => {
            if let Some(t) = d.tag.entities.get(id).cloned() {
                d.tag.entities.insert(id.clone(), merge_into(&t, changes));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn add_then_delete_keeps_lists_consistent() {
        let mut d = AppData::fresh();
        let mut t = Task::new("x", INBOX_PROJECT_ID);
        t.due_day = Some(today_str());
        let mut clock = VectorClock::new();
        let a = Action::AddTask {
            task: t.clone(),
            bottom: true,
        };
        let op = a.to_op("me", &mut clock);
        assert_eq!(op.a, "HA");
        assert_eq!(op.p["workContextType"], "PROJECT");
        assert_eq!(clock["me"], 1);
        apply(&mut d, &a);
        assert_eq!(d.today_ids(), vec![t.id.clone()]);
        apply(
            &mut d,
            &Action::DeleteTask {
                task: t.clone(),
                sub_tasks: vec![],
            },
        );
        assert!(
            d.project.entities[INBOX_PROJECT_ID].task_ids.is_empty()
                && d.tag.entities[TODAY_TAG_ID].task_ids.is_empty()
        );
    }
}
