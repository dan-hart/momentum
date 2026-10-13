// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Pure recurrence snapshots shared by Upcoming and notification summaries.
use crate::{listing, ProjectedOccurrence};
use momentum_model::{day_number, day_str, Task, INBOX_PROJECT_ID};
use momentum_store::Store;

pub(crate) struct ProjectedTask {
    pub task: Task,
    pub occurrence: ProjectedOccurrence,
}

/// Project an inclusive calendar range, advancing only cloned recurrence cursors.
/// The first date may include the newest missed occurrence, just like startup
/// spawning. Callers choose whether that catch-up date belongs in their view.
pub(crate) fn project(store: &Store, first_day: &str, days: u32, created: u64) -> Vec<ProjectedTask> {
    let Some(first) = day_number(first_day) else {
        return Vec::new();
    };
    let fallback_project = store
        .state
        .project
        .ids
        .first()
        .map(String::as_str)
        .unwrap_or(INBOX_PROJECT_ID);
    let mut configs: Vec<_> = store.state.task_repeat_cfg.iter().cloned().collect();
    // These read-only tasks feed rows and notification counts. Bound their notes
    // once per template so each date does not copy the complete stored note.
    // Actual spawning still uses the original configuration and full notes.
    for cfg in &mut configs {
        cfg.notes = cfg.notes.as_deref().and_then(crate::text::notes_preview);
    }
    configs.sort_by(|a, b| a.id.cmp(&b.id));
    let mut result = Vec::new();
    for n in 0..days {
        let day = day_str(first + i64::from(n));
        for cfg in &mut configs {
            let Some(due_day) = cfg.newest_due_day(&day) else {
                continue;
            };
            let id = format!("rpt_{}_{}", cfg.id, due_day);
            cfg.last_task_creation_day = Some(due_day.clone());
            // Identity, rather than current planned date, suppresses instances that
            // were moved, completed or archived (even unparseable archive entities).
            if store.state.task.entities.contains_key(&id) || listing::archived_has_task(store, &id) {
                continue;
            }
            result.push(ProjectedTask {
                task: cfg.task_for_day(&due_day, fallback_project, created),
                occurrence: ProjectedOccurrence {
                    task_id: id,
                    repeat_cfg_id: cfg.id.clone(),
                    day: due_day,
                },
            });
        }
    }
    result.sort_by(|a, b| {
        a.occurrence
            .day
            .cmp(&b.occurrence.day)
            .then_with(|| a.occurrence.task_id.cmp(&b.occurrence.task_id))
    });
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use momentum_model::{local_ms, RepeatCfg};

    fn store() -> (Store, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        (Store::load(dir.path().into()), dir)
    }
    fn config(id: &str, cycle: &str, start: &str, last: &str) -> RepeatCfg {
        RepeatCfg {
            id: id.into(),
            title: Some(id.into()),
            repeat_cycle: cycle.into(),
            repeat_every: 1,
            start_date: Some(start.into()),
            last_task_creation_day: Some(last.into()),
            ..Default::default()
        }
    }
    fn days(store: &Store, first: &str, count: u32) -> Vec<String> {
        project(store, first, count, local_ms(first, 12, 0).unwrap())
            .into_iter()
            .map(|p| p.occurrence.day)
            .collect()
    }

    #[test]
    fn projection_respects_daily_interval_start_pause_and_cursor() {
        let (mut store, _dir) = store();
        let mut cfg = config("daily", "DAILY", "2026-10-14", "2026-10-12");
        cfg.repeat_every = 2;
        store.state.task_repeat_cfg.insert("daily", cfg);
        assert_eq!(
            days(&store, "2026-10-12", 8),
            ["2026-10-14", "2026-10-16", "2026-10-18"]
        );
        let cfg = store.state.task_repeat_cfg.entities.get_mut("daily").unwrap();
        cfg.last_task_creation_day = Some("2026-10-16".into());
        assert_eq!(days(&store, "2026-10-12", 8), ["2026-10-18"]);
        store.state.task_repeat_cfg.entities.get_mut("daily").unwrap().is_paused = true;
        assert!(days(&store, "2026-10-12", 8).is_empty());
    }

    #[test]
    fn projection_respects_weekday_and_week_interval_over_a_month() {
        let (mut store, _dir) = store();
        let mut cfg = config("weekly", "WEEKLY", "2026-10-12", "2026-10-12");
        cfg.repeat_every = 2;
        cfg.monday = true;
        cfg.wednesday = true;
        store.state.task_repeat_cfg.insert("weekly", cfg);
        assert_eq!(
            days(&store, "2026-10-12", 31),
            ["2026-10-14", "2026-10-26", "2026-10-28", "2026-11-09", "2026-11-11"]
        );
    }

    #[test]
    fn projection_preserves_monthly_clamping_last_day_and_nth_weekday() {
        let (mut store, _dir) = store();
        store
            .state
            .task_repeat_cfg
            .insert("clamped", config("clamped", "MONTHLY", "2026-01-31", "2026-01-31"));
        let mut last = config("last", "MONTHLY", "2026-01-15", "2026-01-31");
        last.monthly_last_day = true;
        store.state.task_repeat_cfg.insert("last", last);
        let mut nth = config("nth", "MONTHLY", "2026-01-15", "2026-01-30");
        nth.monthly_week_of_month = Some(-1);
        nth.monthly_weekday = Some(5);
        store.state.task_repeat_cfg.insert("nth", nth);
        let projected = project(&store, "2026-02-01", 31, 0);
        let ids: Vec<_> = projected.iter().map(|p| p.occurrence.task_id.as_str()).collect();
        assert_eq!(
            ids,
            ["rpt_nth_2026-02-27", "rpt_clamped_2026-02-28", "rpt_last_2026-02-28"]
        );
        store
            .state
            .task_repeat_cfg
            .entities
            .get_mut("clamped")
            .unwrap()
            .repeat_every = 2;
        assert!(!project(&store, "2026-02-01", 31, 0)
            .iter()
            .any(|p| p.occurrence.repeat_cfg_id == "clamped"));
    }

    #[test]
    fn projection_preserves_yearly_interval_and_leap_day_clamping() {
        let (mut store, _dir) = store();
        let mut cfg = config("leap", "YEARLY", "2024-02-29", "2024-02-29");
        cfg.repeat_every = 2;
        store.state.task_repeat_cfg.insert("leap", cfg);
        assert_eq!(days(&store, "2026-02-25", 7), ["2026-02-28"]);
        store
            .state
            .task_repeat_cfg
            .entities
            .get_mut("leap")
            .unwrap()
            .last_task_creation_day = Some("2026-02-28".into());
        assert!(days(&store, "2027-02-25", 7).is_empty());
        assert_eq!(days(&store, "2028-02-25", 7), ["2028-02-29"]);
    }

    #[test]
    fn projection_deduplicates_live_completed_moved_and_both_archive_tiers_by_id() {
        let (mut store, _dir) = store();
        store
            .state
            .task_repeat_cfg
            .insert("daily", config("daily", "DAILY", "2026-10-12", "2026-10-12"));
        for (day, moved_to, done) in [
            ("2026-10-13", "2026-10-13", false),
            ("2026-10-14", "2027-01-01", false),
            ("2026-10-15", "2026-10-15", true),
        ] {
            let id = format!("rpt_daily_{day}");
            let task = Task {
                id: id.clone(),
                due_day: Some(moved_to.into()),
                is_done: done,
                ..Default::default()
            };
            store.state.task.insert(&id, task);
        }
        // Archive identity is enough; projection must not deserialize the payload.
        for (tier, day) in [("archiveYoung", "2026-10-16"), ("archiveOld", "2026-10-17")] {
            store.state.rest.insert(
                tier.into(),
                serde_json::json!({
                    "task": {"entities": {format!("rpt_daily_{day}"): {"unknown": true}}}
                }),
            );
        }
        assert_eq!(days(&store, "2026-10-12", 8), ["2026-10-18", "2026-10-19"]);
    }

    #[test]
    fn projection_keeps_newest_missed_catch_up_and_moves_the_snapshot_with_the_day() {
        let (mut store, _dir) = store();
        let mut cfg = config("weekly", "WEEKLY", "2026-09-01", "2026-09-01");
        cfg.monday = true;
        store.state.task_repeat_cfg.insert("weekly", cfg);
        assert_eq!(days(&store, "2026-09-16", 7), ["2026-09-14", "2026-09-21"]);
        store.state.task_repeat_cfg.remove("weekly");
        store
            .state
            .task_repeat_cfg
            .insert("daily", config("daily", "DAILY", "2026-12-31", "2026-12-31"));
        assert_eq!(days(&store, "2026-12-31", 2), ["2027-01-01"]);
        assert_eq!(days(&store, "2027-01-01", 2), ["2027-01-01", "2027-01-02"]);
        assert_eq!(
            store.state.task_repeat_cfg.entities["daily"]
                .last_task_creation_day
                .as_deref(),
            Some("2026-12-31")
        );
    }

    #[test]
    fn projection_bounds_note_snippets_without_changing_materialized_notes() {
        let (mut store, _dir) = store();
        for notes in [
            " \n\tRésumé 👩🏽‍💻\t \nA second line".to_owned(),
            format!("\n {}\n{}", "é".repeat(100), "z".repeat(1024 * 1024)),
            " \n\t ".to_owned(),
        ] {
            let mut cfg = config("daily", "DAILY", "2026-10-12", "2026-10-12");
            cfg.notes = Some(notes.clone());
            cfg.extra
                .insert("futureMetadata".into(), serde_json::json!({"retained": true}));
            store.state.task_repeat_cfg.insert("daily", cfg.clone());
            let before = serde_json::to_value(&store.state).unwrap();
            let expected = crate::text::notes_preview(&notes);
            let projected = project(&store, "2026-10-13", 30, 0);
            assert_eq!(projected.len(), 30);
            for occurrence in &projected {
                assert_eq!(occurrence.task.notes, expected);
                assert!(occurrence.task.notes.as_ref().is_none_or(|n| n.chars().count() <= 81));
            }
            assert_eq!(serde_json::to_value(&store.state).unwrap(), before);
            assert_eq!(cfg.task_for_day("2026-10-13", INBOX_PROJECT_ID, 0).notes, Some(notes));
        }
    }
}
