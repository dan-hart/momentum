// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Super Productivity data model (schema v4). Field names mirror upstream
//! `*.model.ts` (MIT, (c) 2018 Johannes Millan); unknown fields round-trip via `extra`.
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

pub const SCHEMA_VERSION: u32 = 4;
pub const TODAY_TAG_ID: &str = "TODAY";
pub const INBOX_PROJECT_ID: &str = "INBOX_PROJECT";

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
pub fn new_id() -> String {
    uuid::Uuid::now_v7().to_string()
}
/// Local calendar day as `YYYY-MM-DD` (upstream `getDbDateStr`), via glibc `localtime_r`.
pub fn today_str() -> String {
    #[repr(C)]
    struct Tm {
        sec: i32,
        min: i32,
        hour: i32,
        mday: i32,
        mon: i32,
        year: i32,
        wday: i32,
        yday: i32,
        isdst: i32,
        gmtoff: i64,
        zone: *const u8,
    }
    extern "C" {
        fn time(t: *mut i64) -> i64;
        fn localtime_r(t: *const i64, out: *mut Tm) -> *mut Tm;
    }
    let mut tm = Tm {
        sec: 0,
        min: 0,
        hour: 0,
        mday: 1,
        mon: 0,
        year: 70,
        wday: 0,
        yday: 0,
        isdst: 0,
        gmtoff: 0,
        zone: std::ptr::null(),
    };
    unsafe {
        let mut t = 0i64;
        time(&mut t);
        localtime_r(&t, &mut tm);
    }
    format!("{:04}-{:02}-{:02}", tm.year + 1900, tm.mon + 1, tm.mday)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntityState<T> {
    #[serde(default)]
    pub ids: Vec<String>,
    #[serde(default)]
    pub entities: BTreeMap<String, T>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
impl<T> EntityState<T> {
    pub fn insert(&mut self, id: &str, t: T) {
        if !self.entities.contains_key(id) {
            self.ids.push(id.into());
        }
        self.entities.insert(id.into(), t);
    }
    pub fn remove(&mut self, id: &str) -> Option<T> {
        self.ids.retain(|i| i != id);
        self.entities.remove(id)
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.ids.iter().filter_map(|i| self.entities.get(i))
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default)]
    pub time_estimate: f64,
    #[serde(default)]
    pub time_spent: f64,
    #[serde(default)]
    pub time_spent_on_day: BTreeMap<String, f64>,
    #[serde(default)]
    pub is_done: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub done_on: Option<u64>,
    #[serde(default)]
    pub project_id: String,
    #[serde(default)]
    pub tag_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub sub_task_ids: Vec<String>,
    #[serde(default)]
    pub created: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_day: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_with_time: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remind_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat_cfg_id: Option<String>,
    #[serde(default)]
    pub attachments: Vec<Value>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
impl Task {
    pub fn new(title: &str, project_id: &str) -> Self {
        Self {
            id: new_id(),
            title: title.trim().into(),
            project_id: project_id.into(),
            created: now_ms(),
            ..Default::default()
        }
    }
    pub fn recalc_time_spent(&mut self) {
        self.time_spent = self.time_spent_on_day.values().sum();
    }
}

fn theme(primary: &str) -> Value {
    json!({"isAutoContrast": true, "isDisableBackgroundTint": false, "primary": primary, "huePrimary": "500",
        "accent": "#ff4081", "hueAccent": "500", "warn": "#e11826", "hueWarn": "500",
        "backgroundImageDark": null, "backgroundImageLight": null, "backgroundOverlayOpacity": 20})
}
fn advanced_cfg() -> Value {
    json!({"worklogExportSettings": {"cols": ["DATE","START","END","TIME_CLOCK","TITLES_INCLUDING_SUB"],
        "roundWorkTimeTo": null, "roundStartTimeTo": null, "roundEndTimeTo": null, "separateTasksBy": " | ", "groupBy": "DATE"}})
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub task_ids: Vec<String>,
    #[serde(default)]
    pub backlog_task_ids: Vec<String>,
    #[serde(default)]
    pub note_ids: Vec<String>,
    #[serde(default)]
    pub is_archived: bool,
    #[serde(default)]
    pub is_hidden_from_menu: bool,
    #[serde(default)]
    pub is_enable_backlog: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default = "Value::default")]
    pub theme: Value,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
impl Project {
    pub fn new(title: &str) -> Self {
        Self {
            id: new_id(),
            title: title.trim().into(),
            theme: theme("#29a1aa"),
            extra: Map::from_iter([
                ("advancedCfg".to_string(), advanced_cfg()),
                ("isDone".to_string(), json!(false)),
                ("doneOn".to_string(), Value::Null),
            ]),
            ..Default::default()
        }
    }
    pub fn inbox() -> Self {
        Self {
            id: INBOX_PROJECT_ID.into(),
            icon: Some("inbox".into()),
            theme: theme("rgb(144, 187, 165)"),
            ..Self::new("Inbox")
        }
    }
    pub fn color(&self) -> Option<&str> {
        self.theme.get("primary")?.as_str()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub task_ids: Vec<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub created: u64,
    #[serde(default = "Value::default")]
    pub theme: Value,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
impl Tag {
    pub fn new(title: &str) -> Self {
        Self {
            id: new_id(),
            title: title.trim().into(),
            created: now_ms(),
            theme: theme("#a05db1"),
            extra: Map::from_iter([("advancedCfg".to_string(), advanced_cfg())]),
            ..Default::default()
        }
    }
    pub fn today() -> Self {
        let mut t = Self {
            id: TODAY_TAG_ID.into(),
            icon: Some("wb_sunny".into()),
            ..Self::new("Today")
        };
        t.theme = theme("#6495ED");
        t.theme["huePrimary"] = json!("400");
        t.theme["backgroundImageDark"] = json!("");
        t.theme["isDisableBackgroundTint"] = json!(true);
        t
    }
}

/// `AppDataComplete`: the full state snapshot. Slices we do not model stay in `rest`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppData {
    #[serde(default)]
    pub task: EntityState<Task>,
    #[serde(default)]
    pub project: EntityState<Project>,
    #[serde(default)]
    pub tag: EntityState<Tag>,
    #[serde(flatten)]
    pub rest: Map<String, Value>,
}
impl AppData {
    pub fn fresh() -> Self {
        let mut d = Self::default();
        d.project.insert(INBOX_PROJECT_ID, Project::inbox());
        d.tag.insert(TODAY_TAG_ID, Tag::today());
        d
    }
    /// Accepts a raw `AppDataComplete` or the `{data, timestamp, crossModelVersion}` backup wrapper.
    pub fn from_backup(v: Value) -> serde_json::Result<Self> {
        match v.get("data") {
            Some(inner) if v.get("task").is_none() => serde_json::from_value(inner.clone()),
            _ => serde_json::from_value(v),
        }
    }
    pub fn today_ids(&self) -> Vec<String> {
        let today = today_str();
        let mut ids: Vec<String> = self
            .tag
            .entities
            .get(TODAY_TAG_ID)
            .map(|t| t.task_ids.clone())
            .unwrap_or_default();
        ids.retain(|i| {
            self.task
                .entities
                .get(i)
                .is_some_and(|t| t.parent_id.is_none() && t.due_day.as_deref() == Some(&today))
        });
        for t in self.task.iter() {
            if t.parent_id.is_none() && t.due_day.as_deref() == Some(&today) && !ids.contains(&t.id) {
                ids.push(t.id.clone());
            }
        }
        ids
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn task_roundtrips_unknown_fields() {
        let v = json!({"id":"a","title":"t","timeSpentOnDay":{"2026-01-01":1000},"issueId":"X","tagIds":[]});
        let t: Task = serde_json::from_value(v.clone()).unwrap();
        assert_eq!(serde_json::to_value(&t).unwrap()["issueId"], "X");
    }
    #[test]
    fn today_is_iso() {
        assert_eq!(today_str().len(), 10);
    }
}
