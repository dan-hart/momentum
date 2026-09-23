// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Pure, bounded notification snapshots. Planning is not scheduling or delivery.
use crate::{listing, ClockTime, MorningSummary, Slot};
use serde::{Deserialize, Serialize};
use sp_model::{day_number, day_of_ms, day_str, local_ms, time_of_ms, INBOX_PROJECT_ID};
use sp_store::Store;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationPlanningOptions {
    pub now_ms: u64,
    /// Caller-owned revision of the input snapshot, for reconciliation after edits.
    pub source_revision: String,
    /// Logical occurrences already delivered or consumed while elapsed. Suppression
    /// is unconditional, including after clock edits or rollback. The caller must
    /// exclude merely accepted future requests: they remain in the desired plan.
    pub consumed_ids: HashSet<String>,
    pub morning_summary_enabled: bool,
    pub morning_summary_time: ClockTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum NotificationContent {
    Reminder {
        task_id: String,
        title: String,
        due_at_ms: Option<u64>,
    },
    Summary {
        day: String,
        counts: MorningSummary,
        task_ids: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct NotificationRequest {
    pub id: String,
    pub source_revision: String,
    /// Original occurrence time, retained for stable identity and catch-up ordering.
    pub scheduled_at_ms: u64,
    /// Catch-up requests fire at `now_ms`; future requests keep their original time.
    pub fire_at_ms: u64,
    pub content: NotificationContent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct ProjectedOccurrence {
    pub task_id: String,
    pub repeat_cfg_id: String,
    pub day: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct NotificationPlan {
    /// Desired requests already accepted by the OS. Schedule only the remainder.
    pub accepted: Vec<NotificationObservation>,
    /// Cancel only these exact revisions, not a replacement sharing the logical ID.
    pub cancellations: Vec<NotificationObservation>,
    /// Exclusive local midnight after today and the following six calendar dates.
    pub horizon_end_ms: u64,
    /// Reminders first, ordered by occurrence time and ID; then chronological summaries.
    pub requests: Vec<NotificationRequest>,
    pub overflow_reminders: u64,
    pub overflow_summaries: u64,
    /// Date-only instances projected for opt-in summaries; never new task reminders.
    pub projected_occurrences: Vec<ProjectedOccurrence>,
}

/// Builds a snapshot without writing tasks, repeat cursors, pending ops or claims.
/// Accepted OS requests and delivery deduplication belong to the scheduling layer.
pub fn plan(store: &Store, options: &NotificationPlanningOptions) -> NotificationPlan {
    let today = day_of_ms(options.now_ms);
    let first_day = day_number(&today).expect("local timestamp has a calendar date");
    let horizon_end_ms = local_ms(&day_str(first_day + 7), 0, 0).expect("local horizon midnight");
    let mut requests: Vec<_> = store
        .state
        .task
        .iter()
        .filter_map(|task| {
            let at = task.remind_at?;
            if task.is_done || at >= horizon_end_ms {
                return None;
            }
            let id = format!("reminder:{}:{at}", task.id);
            if options.consumed_ids.contains(&id) {
                return None;
            }
            Some(request(
                options,
                id,
                at,
                NotificationContent::Reminder {
                    task_id: task.id.clone(),
                    title: task.title.clone(),
                    due_at_ms: task.due_with_time,
                },
            ))
        })
        .collect();
    requests.sort_by(|a, b| a.scheduled_at_ms.cmp(&b.scheduled_at_ms).then_with(|| a.id.cmp(&b.id)));
    let overflow_reminders = requests.len().saturating_sub(60) as u64;
    requests.truncate(60);
    let mut result = NotificationPlan {
        accepted: Vec::new(),
        cancellations: Vec::new(),
        horizon_end_ms,
        requests,
        overflow_reminders,
        overflow_summaries: 0,
        projected_occurrences: Vec::new(),
    };
    let time = options.morning_summary_time;
    if !options.morning_summary_enabled || time.hour > 23 || time.minute > 59 {
        return result;
    }

    // Advance only cloned cursors. Like spawn_repeats, the first date can produce
    // one newest missed occurrence; its old date never contributes to today's count.
    let mut state = store.state.clone();
    let fallback_project = store
        .state
        .project
        .ids
        .first()
        .map(String::as_str)
        .unwrap_or(INBOX_PROJECT_ID);
    let mut configs: Vec<_> = state.task_repeat_cfg.iter().cloned().collect();
    configs.sort_by(|a, b| a.id.cmp(&b.id));
    let archived: HashSet<_> = listing::archived_tasks(store).into_iter().map(|t| t.id).collect();
    let summary_capacity = (60 - result.requests.len()).min(4);
    let mut summaries = 0;
    for n in 0..7 {
        let day = day_str(first_day + n);
        for cfg in &mut configs {
            let Some(due_day) = cfg.newest_due_day(&day) else {
                continue;
            };
            let id = format!("rpt_{}_{}", cfg.id, due_day);
            // An existing or archived occurrence counts as already materialized.
            // Moving this temporary cursor also avoids repeatedly considering it.
            cfg.last_task_creation_day = Some(due_day.clone());
            if state.task.entities.contains_key(&id) || archived.contains(&id) {
                continue;
            }
            state
                .task
                .insert(&id, cfg.task_for_day(&due_day, fallback_project, options.now_ms));
            result.projected_occurrences.push(ProjectedOccurrence {
                task_id: id,
                repeat_cfg_id: cfg.id.clone(),
                day: due_day,
            });
        }
        let id = format!("summary:{day}");
        if store.meta.last_summary_day == day || options.consumed_ids.contains(&id) {
            continue;
        }
        let mut open: Vec<_> = state
            .planned_ids(&day)
            .iter()
            .filter_map(|id| state.task.entities.get(id))
            .filter(|task| !task.is_done)
            .collect();
        if open.is_empty() {
            continue;
        }
        open.sort_by(|a, b| a.id.cmp(&b.id));
        let counts = MorningSummary {
            total: open.len() as u32,
            morning: open
                .iter()
                .filter(|t| listing::slot_of(store, t) == Some(Slot::Morning))
                .count() as u32,
            tonight: open
                .iter()
                .filter(|t| listing::slot_of(store, t) == Some(Slot::Tonight))
                .count() as u32,
        };
        let Some(at) = summary_time_ms(&day, time) else {
            continue;
        };
        if summaries < summary_capacity {
            result.requests.push(request(
                options,
                id,
                at,
                NotificationContent::Summary {
                    day,
                    counts,
                    task_ids: open.iter().map(|t| t.id.clone()).collect(),
                },
            ));
        } else {
            result.overflow_summaries += 1;
        }
        summaries += 1;
    }
    result
        .projected_occurrences
        .sort_by(|a, b| a.day.cmp(&b.day).then_with(|| a.task_id.cmp(&b.task_id)));
    result
}

fn request(
    options: &NotificationPlanningOptions,
    id: String,
    at: u64,
    content: NotificationContent,
) -> NotificationRequest {
    NotificationRequest {
        id,
        source_revision: options.source_revision.clone(),
        scheduled_at_ms: at,
        fire_at_ms: at.max(options.now_ms),
        content,
    }
}

/// First real local minute eligible under the desktop summary policy. Walking this
/// one transition date uses libc's timezone rules without guessing an offset: a gap
/// at 02:30 fires at 03:00, and a repeated 01:30 chooses its first occurrence.
fn summary_time_ms(day: &str, time: ClockTime) -> Option<u64> {
    let mut at = local_ms(day, 0, 0)?;
    // Some zones repeat midnight itself. Begin at the first real minute of the date.
    while let Some(previous) = at.checked_sub(60_000) {
        if day_of_ms(previous) != day {
            break;
        }
        at = previous;
    }
    // Ordinary dates need no scan. Keep the transition path bounded by a single
    // local calendar date, including zones that skip or repeat midnight itself.
    let next_day = day_str(day_number(day)? + 1);
    let end = local_ms(&next_day, 0, 0)?;
    if end.checked_sub(at) == Some(24 * 60 * 60_000) {
        let candidate = local_ms(day, time.hour, time.minute)?;
        if day_of_ms(candidate) == day && time_of_ms(candidate) == (time.hour, time.minute) {
            return Some(candidate);
        }
    }
    while day_of_ms(at) == day {
        if time_of_ms(at) >= (time.hour, time.minute) {
            return Some(at);
        }
        at = at.checked_add(60_000)?;
    }
    // A local date can be skipped altogether by a timezone transition.
    None
}

/// Opaque revision metadata must accompany OS pending/delivered observations.
/// Adapters must use both fields to distinguish replacements and stale callbacks.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct NotificationObservation {
    pub id: String,
    pub source_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct NotificationAcceptance {
    pub accepted: bool,
    pub cancellation: Option<NotificationObservation>,
}

impl NotificationRequest {
    pub(crate) fn observation(&self) -> NotificationObservation {
        NotificationObservation {
            id: self.id.clone(),
            source_revision: self.source_revision.clone(),
        }
    }
}
