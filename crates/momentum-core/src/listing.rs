// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Turning the store into what a view shows: the sidebar, the sections and rows of each
//! view, search results, and the slot rules (Morning, Tonight) that shape Today.
use crate::text::*;
use crate::types::*;
use sp_model::*;
use sp_store::Store;

pub(crate) const SEARCH_LIMIT: usize = 60;

/// Precomputed lowercase haystacks so typing never re-lowercases or re-parses the archive.
#[derive(Default)]
pub(crate) struct SearchIndex {
    tasks: Vec<(String, String)>,      // (task id, haystack)
    pub archived: Vec<(Task, String)>, // parsed once, newest completion first
    projects: Vec<(String, String)>,   // (project id, lower title)
    tags: Vec<(String, String)>,       // (tag id, lower title)
}

/// Tasks in `archiveYoung` and `archiveOld` (kept in `state.rest` by sp-sync).
pub(crate) fn archived_tasks(store: &Store) -> Vec<Task> {
    ["archiveYoung", "archiveOld"]
        .iter()
        .filter_map(|k| store.state.rest.get(*k)?.get("task")?.get("entities")?.as_object())
        .flat_map(|e| {
            e.values()
                .filter_map(|v| serde_json::from_value::<Task>(v.clone()).ok())
        })
        .collect()
}

/// Check archive membership from the entity map without deserializing every task.
/// Repeat spawning only needs the identifier and runs during application startup.
pub(crate) fn archived_has_task(store: &Store, id: &str) -> bool {
    ["archiveYoung", "archiveOld"].iter().any(|tier| {
        store
            .state
            .rest
            .get(*tier)
            .and_then(|archive| archive.get("task"))
            .and_then(|tasks| tasks.get("entities"))
            .and_then(|entities| entities.as_object())
            .is_some_and(|entities| entities.contains_key(id))
    })
}

pub(crate) fn build_index(store: &Store) -> SearchIndex {
    let tag_name = |id: &String| {
        store
            .state
            .tag
            .entities
            .get(id)
            .map(|g| g.title.to_lowercase())
            .unwrap_or_default()
    };
    let hay = |t: &Task| {
        let mut h = t.title.to_lowercase();
        if let Some(n) = &t.notes {
            h.push('\n');
            h.push_str(&n.to_lowercase());
        }
        for tag in &t.tag_ids {
            h.push('\n');
            h.push_str(&tag_name(tag));
        }
        if let Some(p) = store.state.project.entities.get(&t.project_id) {
            h.push('\n');
            h.push_str(&p.title.to_lowercase());
        }
        h
    };
    let mut idx = SearchIndex {
        tasks: store.state.task.iter().map(|t| (t.id.clone(), hay(t))).collect(),
        archived: archived_tasks(store)
            .into_iter()
            .map(|t| {
                let h = hay(&t);
                (t, h)
            })
            .collect(),
        projects: store
            .state
            .project
            .iter()
            .map(|p| (p.id.clone(), p.title.to_lowercase()))
            .collect(),
        tags: store
            .state
            .tag
            .iter()
            .filter(|t| t.id != TODAY_TAG_ID)
            .map(|t| (t.id.clone(), t.title.to_lowercase()))
            .collect(),
    };
    idx.archived
        .sort_by_key(|(t, _)| std::cmp::Reverse(t.done_on.unwrap_or(t.created)));
    idx
}

// ---- slots -----------------------------------------------------------------

/// The tag (case-insensitive) that marks a slot: "Evening" for Tonight, "Morning".
pub(crate) fn slot_tag_id(store: &Store, slot: Slot) -> Option<String> {
    store
        .state
        .tag
        .iter()
        .find(|t| t.title.eq_ignore_ascii_case(slot.tag_name()))
        .map(|t| t.id.clone())
}

/// Which part of the day a task belongs to; Morning wins if both tags are present.
pub(crate) fn slot_of(store: &Store, t: &Task) -> Option<Slot> {
    slot_from_names(
        t.tag_ids
            .iter()
            .filter_map(|id| store.state.tag.entities.get(id))
            .map(|tag| tag.title.as_str()),
    )
}

fn slot_from_names<'a>(names: impl IntoIterator<Item = &'a str>) -> Option<Slot> {
    let mut evening = false;
    for name in names {
        if name.eq_ignore_ascii_case(Slot::Morning.tag_name()) {
            return Some(Slot::Morning);
        }
        evening |= name.eq_ignore_ascii_case(Slot::Tonight.tag_name());
    }
    evening.then_some(Slot::Tonight)
}

/// Today has at least one open or done task in this slot.
pub(crate) fn today_uses_slot(store: &Store, slot: Slot) -> bool {
    store
        .state
        .today_ids()
        .iter()
        .filter_map(|i| store.state.task.entities.get(i))
        .any(|t| slot_of(store, t) == Some(slot))
}

// ---- rows ------------------------------------------------------------------

pub(crate) fn project_ref(p: &Project) -> ProjectRef {
    ProjectRef {
        id: p.id.clone(),
        title: p.title.clone(),
        color: project_color(p),
    }
}
pub(crate) fn tag_ref(g: &Tag) -> TagRef {
    TagRef {
        id: g.id.clone(),
        title: g.title.clone(),
        color: tag_color(g),
    }
}

pub(crate) fn task_row(store: &Store, view: &View, t: &Task, archived: bool) -> TaskRow {
    let in_project_view = matches!(view, View::Project { .. });
    let project = if in_project_view {
        None
    } else {
        store.state.project.entities.get(&t.project_id).map(project_ref)
    };
    let repeat = t
        .repeat_cfg_id
        .as_ref()
        .and_then(|id| store.state.task_repeat_cfg.entities.get(id))
        .map(describe_repeat);
    let due_day = t.plan_day();
    // Day views already say which day it is, except for overdue tasks, whose day is the point.
    let day = due_day.as_deref().and_then(|d| {
        let day_known = matches!(view, View::Today | View::Morning | View::Tonight) && d >= today_str().as_str();
        (!day_known).then(|| day_label(d))
    });
    let done_day = if archived {
        t.done_on.map(|ms| day_label(&day_of_ms(ms)))
    } else {
        None
    };
    TaskRow {
        id: t.id.clone(),
        title: t.title.clone(),
        is_done: t.is_done,
        is_subtask: t.parent_id.is_some(),
        archived,
        project,
        estimate_ms: t.time_estimate,
        day,
        time: t.due_with_time.map(clock_time),
        due_day,
        repeat,
        tags: t
            .tag_ids
            .iter()
            .filter_map(|id| store.state.tag.entities.get(id))
            .map(tag_ref)
            .collect(),
        notes_preview: t.notes.as_deref().and_then(notes_preview),
        reminder: t.remind_at.filter(|_| !t.is_done).map(clock_time),
        done_day,
        sub_task_ids: t.sub_task_ids.clone(),
        parent_id: t.parent_id.clone(),
    }
}

/// A task row followed by its subtasks, as list views show them.
fn push_task_with_subs(rows: &mut Vec<Row>, store: &Store, view: &View, t: &Task) {
    rows.push(Row::Task {
        row: task_row(store, view, t, false),
    });
    for s in t.sub_task_ids.iter().filter_map(|i| store.state.task.entities.get(i)) {
        rows.push(Row::Task {
            row: task_row(store, view, s, false),
        });
    }
}

// ---- views -----------------------------------------------------------------

pub(crate) fn view_title(store: &Store, view: &View) -> ViewTitle {
    match view {
        View::Today => ViewTitle::Today,
        View::Morning => ViewTitle::Morning,
        View::Tonight => ViewTitle::Tonight,
        View::Upcoming => ViewTitle::ComingUp,
        View::Archive => ViewTitle::Archive,
        View::Search => ViewTitle::Search,
        View::Project { id } => ViewTitle::Named {
            name: store
                .state
                .project
                .entities
                .get(id)
                .map(|p| p.title.clone())
                .unwrap_or_default(),
        },
        View::Tag { id } => ViewTitle::Named {
            name: store
                .state
                .tag
                .entities
                .get(id)
                .map(|t| t.title.clone())
                .unwrap_or_default(),
        },
    }
}

pub(crate) fn empty_state(store: &Store, view: &View, prefs: &Preferences) -> EmptyState {
    match view {
        View::Today => EmptyState::Today,
        View::Morning => EmptyState::Morning,
        View::Tonight => EmptyState::Tonight,
        View::Upcoming => EmptyState::Upcoming {
            days: prefs.upcoming_days,
        },
        View::Archive => EmptyState::Archive,
        View::Search => EmptyState::Search,
        View::Project { id } => EmptyState::Project {
            name: store
                .state
                .project
                .entities
                .get(id)
                .map(|p| p.title.clone())
                .unwrap_or_default(),
        },
        View::Tag { id } => EmptyState::Tag {
            name: store
                .state
                .tag
                .entities
                .get(id)
                .map(|t| t.title.clone())
                .unwrap_or_default(),
        },
    }
}

/// The stored order of the tasks a list view shows (before sorting and splitting).
pub(crate) fn view_task_ids(store: &Store, view: &View) -> Vec<String> {
    match view {
        View::Today => store.state.today_ids(),
        View::Tonight | View::Morning => {
            let want = if *view == View::Morning {
                Slot::Morning
            } else {
                Slot::Tonight
            };
            store
                .state
                .today_ids()
                .into_iter()
                .filter(|id| {
                    store
                        .state
                        .task
                        .entities
                        .get(id)
                        .is_some_and(|t| slot_of(store, t) == Some(want))
                })
                .collect()
        }
        View::Upcoming | View::Archive | View::Search => vec![],
        View::Project { id } => store
            .state
            .project
            .entities
            .get(id)
            .map(|p| p.task_ids.clone())
            .unwrap_or_default(),
        View::Tag { id } => store
            .state
            .tag
            .entities
            .get(id)
            .map(|t| t.task_ids.clone())
            .unwrap_or_default(),
    }
}

/// Done top-level tasks with their subtasks, ready to archive.
pub(crate) fn done_tasks(store: &Store) -> (Vec<Task>, Vec<Task>) {
    let tasks: Vec<Task> = store
        .state
        .task
        .iter()
        .filter(|t| t.is_done && t.parent_id.is_none())
        .cloned()
        .collect();
    let sub_tasks = tasks
        .iter()
        .flat_map(|t| {
            t.sub_task_ids
                .iter()
                .filter_map(|i| store.state.task.entities.get(i).cloned())
        })
        .collect();
    (tasks, sub_tasks)
}

fn sort_tasks(list: &mut [&Task], prefs: &Preferences) {
    match prefs.sort {
        SortKey::Title => list.sort_by_key(|t| t.title.to_lowercase()),
        SortKey::Due => list.sort_by(|a, b| {
            let (x, y) = (a.plan_day(), b.plan_day());
            x.is_none()
                .cmp(&y.is_none())
                .then_with(|| x.cmp(&y))
                .then_with(|| a.due_with_time.cmp(&b.due_with_time))
        }),
        SortKey::Estimate => list.sort_by(|a, b| a.time_estimate.total_cmp(&b.time_estimate)),
        SortKey::Created => list.sort_by_key(|t| t.created),
        SortKey::Manual => {}
    }
    if prefs.direction == SortDirection::Descending {
        list.reverse();
    }
}

pub(crate) fn sidebar(store: &Store) -> Sidebar {
    let family_count = |ids: &[String]| -> u32 {
        ids.iter()
            .filter_map(|id| store.state.task.entities.get(id))
            .filter_map(|task| {
                let root = task
                    .parent_id
                    .as_ref()
                    .and_then(|id| store.state.task.entities.get(id))
                    .unwrap_or(task);
                (!root.is_done && root.parent_id.is_none()).then_some(root.id.as_str())
            })
            .collect::<std::collections::HashSet<_>>()
            .len() as u32
    };
    let entry = |view: View| SidebarEntry {
        view,
        title: String::new(),
        color: None,
        task_count: 0,
    };
    let mut fixed = vec![entry(View::Today)];
    // Morning and Tonight only exist as entries while today has tasks in that slot.
    if today_uses_slot(store, Slot::Morning) {
        fixed.push(entry(View::Morning));
    }
    if today_uses_slot(store, Slot::Tonight) {
        fixed.push(entry(View::Tonight));
    }
    fixed.extend([entry(View::Upcoming), entry(View::Archive), entry(View::Search)]);
    Sidebar {
        fixed,
        projects: store
            .state
            .project
            .iter()
            .filter(|p| !p.is_archived && !p.is_hidden_from_menu)
            .map(|p| SidebarEntry {
                view: View::project(&p.id),
                title: p.title.clone(),
                color: project_color(p),
                task_count: family_count(&p.task_ids),
            })
            .collect(),
        tags: store
            .state
            .tag
            .iter()
            .filter(|t| t.id != TODAY_TAG_ID)
            .map(|t| SidebarEntry {
                view: View::tag(&t.id),
                title: t.title.clone(),
                color: tag_color(t),
                task_count: family_count(&t.task_ids),
            })
            .collect(),
    }
}

/// Every non-search view. `archive_limit` is how many archived parents to show.
pub(crate) fn listing(
    store: &Store,
    index: Option<&SearchIndex>,
    view: &View,
    prefs: &Preferences,
    archive_limit: usize,
) -> Listing {
    let title = view_title(store, view);
    let can_archive = !done_tasks(store).0.is_empty();
    let mut out = Listing {
        view: view.clone(),
        title,
        sections: vec![],
        empty: None,
        all_done: None,
        more_available: 0,
        can_archive,
    };
    match view {
        View::Search => {
            out.empty = Some(EmptyState::Search);
            return out;
        }
        View::Upcoming => {
            let today_n = day_number(&today_str()).unwrap_or(0);
            let range = prefs.upcoming_days as i64;
            let mut tasks: Vec<&Task> = store
                .state
                .task
                .iter()
                .filter(|t| !t.is_done && t.parent_id.is_none())
                .filter(|t| {
                    t.plan_day()
                        .as_deref()
                        .and_then(day_number)
                        .is_some_and(|d| d > today_n && d <= today_n + range)
                })
                .collect();
            tasks.sort_by(|a, b| {
                a.plan_day()
                    .cmp(&b.plan_day())
                    .then_with(|| a.due_with_time.cmp(&b.due_with_time))
                    .then_with(|| a.title.cmp(&b.title))
            });
            // The selected grouping owns section structure. Dates remain on rows.
            sort_tasks(&mut tasks, prefs);
            if !tasks.is_empty() {
                out.sections.push(Section {
                    group: None,
                    kind: SectionKind::Plain,
                    count: tasks.len() as u32,
                    rows: tasks
                        .into_iter()
                        .map(|t| Row::Task {
                            row: task_row(store, view, t, false),
                        })
                        .collect(),
                    note: None,
                });
            }
        }
        View::Archive => {
            // Read-only view over archiveYoung + archiveOld, newest completion first, paged.
            let parents: Vec<&Task> = index
                .expect("archive listing requires the cached archive projection")
                .archived
                .iter()
                .map(|(t, _)| t)
                .filter(|t| t.parent_id.is_none())
                .collect();
            let rows: Vec<Row> = parents
                .iter()
                .take(archive_limit)
                .map(|t| Row::Task {
                    row: task_row(store, view, t, true),
                })
                .collect();
            out.more_available = parents.len().saturating_sub(archive_limit) as u32;
            if !rows.is_empty() {
                out.sections.push(Section {
                    group: None,
                    kind: SectionKind::Plain,
                    count: parents.len() as u32,
                    rows,
                    note: None,
                });
            }
        }
        _ => {
            let ids = view_task_ids(store, view);
            let (mut open, mut done): (Vec<&Task>, Vec<&Task>) = (vec![], vec![]);
            for id in ids {
                if let Some(t) = store.state.task.entities.get(&id) {
                    if t.is_done {
                        done.push(t)
                    } else {
                        open.push(t)
                    }
                }
            }
            // Overdue tasks remain visible in Today, inside the chosen groups.
            // Keep their due-day labels; do not split them into another section.
            if *view == View::Today {
                let mut overdue: Vec<&Task> = store
                    .state
                    .overdue_ids()
                    .iter()
                    .filter_map(|id| store.state.task.entities.get(id))
                    .collect();
                overdue.append(&mut open);
                open = overdue;
            }
            sort_tasks(&mut open, prefs);
            sort_tasks(&mut done, prefs);
            let open_count = open.len();
            if !open.is_empty() {
                let mut rows = vec![];
                for t in &open {
                    push_task_with_subs(&mut rows, store, view, t);
                }
                out.sections.push(Section {
                    group: None,
                    kind: SectionKind::Plain,
                    count: open.len() as u32,
                    rows,
                    note: None,
                });
            }
            // Everything is done: celebrate above the Completed section and offer to archive.
            if open_count == 0 && !done.is_empty() {
                out.all_done = Some(match view {
                    View::Morning => AllDone::Morning,
                    View::Tonight => AllDone::Tonight,
                    View::Today => AllDone::Today {
                        completed: done.len() as u32,
                    },
                    _ => AllDone::Context,
                });
            }
            if !done.is_empty() {
                let mut rows = vec![];
                for t in &done {
                    push_task_with_subs(&mut rows, store, view, t);
                }
                out.sections.push(Section {
                    group: None,
                    kind: SectionKind::Completed,
                    count: done.len() as u32,
                    rows,
                    note: None,
                });
            }
        }
    }
    if out.sections.is_empty() {
        out.empty = Some(empty_state(store, view, prefs));
    }
    out
}

/// Global search across tasks (open, done, subtasks, archived), projects and tags: every
/// word of the query must match somewhere in a haystack.
pub(crate) fn search(store: &Store, index: &SearchIndex, query: &str) -> Listing {
    let mut out = Listing {
        view: View::Search,
        title: ViewTitle::Search,
        sections: vec![],
        empty: None,
        all_done: None,
        more_available: 0,
        can_archive: !done_tasks(store).0.is_empty(),
    };
    let words: Vec<String> = query.to_lowercase().split_whitespace().map(str::to_string).collect();
    if words.is_empty() {
        out.empty = Some(EmptyState::Search);
        return out;
    }
    let matches = |h: &str| words.iter().all(|w| h.contains(w.as_str()));
    let view = View::Search;
    let mut tasks: Vec<&Task> = index
        .tasks
        .iter()
        .filter(|(_, h)| matches(h))
        .filter_map(|(id, _)| store.state.task.entities.get(id))
        .collect();
    tasks.sort_by(|a, b| {
        a.is_done
            .cmp(&b.is_done)
            .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
    });
    if !tasks.is_empty() {
        let total = tasks.len();
        out.sections.push(Section {
            group: None,
            kind: SectionKind::SearchTasks,
            count: total as u32,
            rows: tasks
                .iter()
                .take(SEARCH_LIMIT)
                .map(|t| Row::Task {
                    row: task_row(store, &view, t, false),
                })
                .collect(),
            note: (total > SEARCH_LIMIT).then_some(SectionNote {
                shown: SEARCH_LIMIT as u32,
                total: total as u32,
                suggest_narrowing: true,
            }),
        });
    }
    let projects: Vec<Row> = index
        .projects
        .iter()
        .filter(|(_, h)| matches(h))
        .filter_map(|(id, _)| store.state.project.entities.get(id))
        .map(|p| Row::Project { item: project_ref(p) })
        .collect();
    if !projects.is_empty() {
        out.sections.push(Section {
            group: None,
            kind: SectionKind::SearchProjects,
            count: projects.len() as u32,
            rows: projects,
            note: None,
        });
    }
    let tags: Vec<Row> = index
        .tags
        .iter()
        .filter(|(_, h)| matches(h))
        .filter_map(|(id, _)| store.state.tag.entities.get(id))
        .map(|g| Row::Tag { item: tag_ref(g) })
        .collect();
    if !tags.is_empty() {
        out.sections.push(Section {
            group: None,
            kind: SectionKind::SearchTags,
            count: tags.len() as u32,
            rows: tags,
            note: None,
        });
    }
    let archived: Vec<&Task> = index
        .archived
        .iter()
        .filter(|(_, h)| matches(h))
        .map(|(t, _)| t)
        .collect();
    if !archived.is_empty() {
        let cap = SEARCH_LIMIT / 2;
        out.sections.push(Section {
            group: None,
            kind: SectionKind::SearchArchived,
            count: archived.len() as u32,
            rows: archived
                .iter()
                .take(cap)
                .map(|t| Row::Task {
                    row: task_row(store, &view, t, true),
                })
                .collect(),
            note: (archived.len() > cap).then_some(SectionNote {
                shown: cap as u32,
                total: archived.len() as u32,
                suggest_narrowing: false,
            }),
        });
    }
    if out.sections.is_empty() {
        out.empty = Some(EmptyState::NoResults);
    }
    out
}

/// Up to `max` open tasks whose title contains every query word, plus nothing else: what
/// a desktop search provider lists.
pub(crate) fn quick_matches(store: &Store, query: &str, max: usize) -> Vec<TaskBrief> {
    let q: Vec<String> = query.to_lowercase().split_whitespace().map(str::to_string).collect();
    if q.is_empty() {
        return vec![];
    }
    store
        .state
        .task
        .iter()
        .filter(|t| !t.is_done)
        .filter(|t| {
            let hay = t.title.to_lowercase();
            q.iter().all(|w| hay.contains(w.as_str()))
        })
        .take(max)
        .map(|t| task_brief(store, t))
        .collect()
}

pub(crate) fn task_brief(store: &Store, t: &Task) -> TaskBrief {
    TaskBrief {
        id: t.id.clone(),
        title: t.title.clone(),
        project: store.state.project.entities.get(&t.project_id).map(|p| p.title.clone()),
        due_day: t.plan_day().as_deref().map(day_label),
        is_done: t.is_done,
        notes: t.notes.clone().filter(|n| !n.trim().is_empty()),
    }
}

/// Group headings are data; platform layers supply localized wording. Groups never
/// change task identity, saved ordering, or the store's pending operations.
pub(crate) fn task_group(store: &Store, view: &View, row: &TaskRow, by: GroupBy) -> Option<TaskGroup> {
    Some(match by {
        GroupBy::None => return None,
        GroupBy::MorningNight => {
            if !view.is_day() {
                return None;
            }
            let slot = slot_from_names(row.tags.iter().map(|tag| tag.title.as_str()));
            match slot {
                None => TaskGroup::Today,
                Some(Slot::Morning) => TaskGroup::Morning,
                Some(Slot::Tonight) => TaskGroup::Evening,
            }
        }
        GroupBy::Project => {
            let project = row.project.clone().or_else(|| match view {
                View::Project { id } => store.state.project.entities.get(id).map(project_ref),
                _ => None,
            });
            match project {
                Some(p) => TaskGroup::Project {
                    id: p.id,
                    title: p.title,
                    color: p.color,
                },
                None => TaskGroup::NoProject,
            }
        }
        GroupBy::Tag => match row.tags.iter().find(|tag| tag.id != TODAY_TAG_ID) {
            Some(tag) => TaskGroup::Tag {
                id: tag.id.clone(),
                title: tag.title.clone(),
                color: tag.color.clone(),
            },
            None => TaskGroup::Untagged,
        },
        GroupBy::Estimate => {
            let ms = row.estimate_ms;
            let range = if !ms.is_finite() || ms <= 0.0 {
                EstimateRange::NoEstimate
            } else if ms <= 900_000.0 {
                EstimateRange::UpTo15Minutes
            } else if ms <= 1_800_000.0 {
                EstimateRange::UpTo30Minutes
            } else if ms <= 3_600_000.0 {
                EstimateRange::UpTo60Minutes
            } else if ms <= 7_200_000.0 {
                EstimateRange::UpTo2Hours
            } else {
                EstimateRange::Over2Hours
            };
            TaskGroup::Estimate { range }
        }
    })
}

fn group_order(group: &TaskGroup) -> (u8, String, String) {
    match group {
        TaskGroup::Today => (0, String::new(), String::new()),
        TaskGroup::Morning => (1, String::new(), String::new()),
        TaskGroup::Evening => (2, String::new(), String::new()),
        TaskGroup::Project { id, title, .. } | TaskGroup::Tag { id, title, .. } => {
            (0, title.to_lowercase(), id.clone())
        }
        TaskGroup::Estimate { range } => (*range as u8, String::new(), String::new()),
        TaskGroup::NoProject | TaskGroup::Untagged => (1, String::new(), String::new()),
    }
}

pub(crate) fn group_listing(store: &Store, mut listing: Listing, by: GroupBy) -> Listing {
    if by == GroupBy::None {
        return listing;
    }
    let mut grouped = Vec::new();
    for section in listing.sections {
        if section.kind == SectionKind::Completed
            || section.rows.is_empty()
            || section.rows.iter().any(|r| !matches!(r, Row::Task { .. }))
        {
            grouped.push(section);
            continue;
        }
        let Some(row_groups) = section
            .rows
            .iter()
            .map(|row| match row {
                Row::Task { row } => task_group(store, &listing.view, row, by),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()
        else {
            grouped.push(section);
            continue;
        };
        let mut groups = std::collections::BTreeMap::<(u8, String, String), Section>::new();
        let mut rows = section.rows.into_iter().zip(row_groups).peekable();
        while let Some((Row::Task { row }, group)) = rows.next() {
            let target = groups.entry(group_order(&group)).or_insert_with(|| Section {
                kind: section.kind.clone(),
                group: Some(group),
                count: 0,
                rows: vec![],
                note: None,
            });
            target.count += 1;
            let parent_id = row.id.clone();
            target.rows.push(Row::Task { row });
            // Search results are independently matched items; ordinary lists are
            // parent/subtask families and group according to the parent's attributes.
            if listing.view != View::Search {
                while matches!(rows.peek(), Some((Row::Task { row }, _)) if row.parent_id.as_deref() == Some(parent_id.as_str()))
                {
                    if let Some((child, _)) = rows.next() {
                        target.rows.push(child);
                    }
                }
            }
        }
        let mut sections: Vec<_> = groups.into_values().collect();
        // Keep truncation information once, after all groups, with original totals.
        if let Some(last) = sections.last_mut() {
            last.note = section.note;
        }
        grouped.extend(sections);
    }
    listing.sections = grouped;
    listing
}
