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

/// Civil-date helpers on `YYYY-MM-DD` strings (no timezone maths needed).
pub fn parse_day(d: &str) -> Option<(i64, u32, u32)> {
    let mut it = d.split('-');
    Some((
        it.next()?.parse().ok()?,
        it.next()?.parse().ok()?,
        it.next()?.parse().ok()?,
    ))
}
/// Days since 1970-01-01 (Howard Hinnant's algorithm).
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) as i64 + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}
pub fn day_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + (m <= 2) as i64, m, d)
}
pub fn day_str(days: i64) -> String {
    let (y, m, d) = day_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}
/// Days since epoch for a `YYYY-MM-DD` string.
pub fn day_number(day: &str) -> Option<i64> {
    parse_day(day).map(|(y, m, d)| days_from_civil(y, m, d))
}
/// 0 = Sunday … 6 = Saturday, matching JavaScript's `getDay()`.
pub fn weekday(days: i64) -> u32 {
    (days + 4).rem_euclid(7) as u32
}
pub fn days_in_month(y: i64, m: u32) -> u32 {
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    (days_from_civil(ny, nm, 1) - days_from_civil(y, m, 1)) as u32
}

/// `TaskRepeatCfg` (task-repeat-cfg.model.ts), typed subset with the rest preserved.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RepeatCfg {
    pub id: String,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub tag_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_estimate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default)]
    pub is_paused: bool,
    #[serde(default)]
    pub repeat_cycle: String,
    #[serde(default)]
    pub repeat_every: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_task_creation_day: Option<String>,
    #[serde(default)]
    pub monthly_last_day: bool,
    #[serde(default)]
    pub monday: bool,
    #[serde(default)]
    pub tuesday: bool,
    #[serde(default)]
    pub wednesday: bool,
    #[serde(default)]
    pub thursday: bool,
    #[serde(default)]
    pub friday: bool,
    #[serde(default)]
    pub saturday: bool,
    #[serde(default)]
    pub sunday: bool,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
impl RepeatCfg {
    pub fn weekdays(&self) -> [bool; 7] {
        [
            self.sunday,
            self.monday,
            self.tuesday,
            self.wednesday,
            self.thursday,
            self.friday,
            self.saturday,
        ]
    }
    /// Whether an instance is due on `today` (`YYYY-MM-DD`), following upstream's cycle rules
    /// for the current day only.
    pub fn is_due(&self, today: &str) -> bool {
        if self.is_paused || self.repeat_every == 0 {
            return false;
        }
        let Some((ty, tm, td)) = parse_day(today) else {
            return false;
        };
        let Some((sy, sm, sd)) = self.start_date.as_deref().and_then(parse_day) else {
            return false;
        };
        let (t, st) = (days_from_civil(ty, tm, td), days_from_civil(sy, sm, sd));
        if st > t || self.last_task_creation_day.as_deref().is_some_and(|l| l >= today) {
            return false;
        }
        let every = self.repeat_every as i64;
        match self.repeat_cycle.as_str() {
            "DAILY" => (t - st) % every == 0,
            "WEEKLY" => ((t - st) / 7) % every == 0 && self.weekdays()[weekday(t) as usize],
            "MONTHLY" => {
                let months = (ty - sy) * 12 + tm as i64 - sm as i64;
                let want = if self.monthly_last_day {
                    days_in_month(ty, tm)
                } else {
                    sd.min(days_in_month(ty, tm))
                };
                months >= 0 && months % every == 0 && td == want
            }
            "YEARLY" => (ty - sy) % every == 0 && tm == sm && td == sd.min(days_in_month(ty, tm)),
            _ => false,
        }
    }
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
    #[serde(default, rename = "taskRepeatCfg")]
    pub task_repeat_cfg: EntityState<RepeatCfg>,
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
    fn repeat_rules() {
        let mut c = RepeatCfg {
            repeat_cycle: "WEEKLY".into(),
            repeat_every: 1,
            start_date: Some("2026-09-01".into()),
            monday: true,
            ..Default::default()
        };
        assert!(c.is_due("2026-09-07")); // a Monday
        assert!(!c.is_due("2026-09-08"));
        c.last_task_creation_day = Some("2026-09-07".into());
        assert!(!c.is_due("2026-09-07"));
        let d = RepeatCfg {
            repeat_cycle: "DAILY".into(),
            repeat_every: 3,
            start_date: Some("2026-09-01".into()),
            ..Default::default()
        };
        assert!(d.is_due("2026-09-04") && !d.is_due("2026-09-05"));
        let m = RepeatCfg {
            repeat_cycle: "MONTHLY".into(),
            repeat_every: 1,
            start_date: Some("2026-01-31".into()),
            ..Default::default()
        };
        assert!(m.is_due("2026-02-28") && m.is_due("2026-03-31"));
        assert_eq!(weekday(days_from_civil(2026, 9, 7)), 1);
    }
    #[test]
    fn today_is_iso() {
        assert_eq!(today_str().len(), 10);
    }
}
