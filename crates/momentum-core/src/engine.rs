// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! The application engine: one object that owns the store and answers every question and
//! every change the user interfaces make. It is `Send + Sync` (a mutex around the state)
//! so a Swift app can call it from any thread and a sync can run off the main loop.
//!
//! Everything a change does to the data is here; what it looks like is the UI's job. A
//! mutation returns an [`Outcome`]: whether anything changed, a structured [`Message`]
//! for a toast, and the id of the undo batch it pushed.
use crate::listing::{self, SearchIndex};
use crate::sync_cancellation::{SyncCancellation, SyncOperation};
use crate::text::*;
use crate::types::*;
use serde_json::{json, Map, Value};
use sp_model::*;
use sp_oplog::Action;
use sp_store::Store;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

const UNDO_DEPTH: usize = 50;
/// Search-provider results: up to this many open tasks.
pub const QUICK_MATCHES: usize = 8;

#[derive(Clone)]
pub(crate) struct Inner {
    staged: bool,
    pub(crate) dirty: bool,
    pub(crate) store: Store,
    pub(crate) prefs: Preferences,
    undo: Vec<(u64, Vec<Action>)>,
    next_undo: u64,
    /// Invalidates exchanges superseded by cancellation or explicit replacement.
    sync_generation: u64,
    index: Option<Arc<SearchIndex>>,
    /// Reminders already shown this session (a snooze or a new time re-arms them).
    notified: HashSet<String>,
    last_day: String,
    /// Wakes the nearby-sync scheduler after every change (set while it runs).
    pub(crate) change_signal: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl Inner {
    pub(crate) fn invalidate(&mut self) {
        self.index = None;
        if let Some(f) = &self.change_signal {
            f();
        }
    }
    fn index(&mut self) -> Arc<SearchIndex> {
        if let Some(i) = &self.index {
            return i.clone();
        }
        let idx = Arc::new(listing::build_index(&self.store));
        self.index = Some(idx.clone());
        idx
    }
    fn add_task(&mut self, text: String, view: View) -> Outcome {
        let q = parse_quick_add(&text);
        if q.title.is_empty() {
            return Outcome::none();
        }
        let g = self;
        let project = g.project_for(&view);
        let mut task = Task::new(&q.title, &project);
        task.time_estimate = q.estimate_ms;
        if view.is_day() {
            task.due_day = Some(today_str());
        }
        let mut names = q.tags;
        if let View::Tag { id } = &view {
            if let Some(t) = g.store.state.tag.entities.get(id) {
                names.push(t.title.clone());
            }
        }
        for slot in [Slot::Morning, Slot::Tonight] {
            if view == slot.view() {
                let id = g.ensure_slot_tag(slot);
                task.tag_ids.push(id);
            }
        }
        for name in names {
            let id = g.ensure_tag(&name);
            if !task.tag_ids.contains(&id) {
                task.tag_ids.push(id);
            }
        }
        if task.title.is_empty() {
            return Outcome::none();
        }
        g.dispatch(Action::AddTask { task, bottom: true });
        Outcome::changed()
    }
    fn add_task_with_notes(&mut self, text: String, notes: Option<String>, due: Option<String>) -> Outcome {
        self.add_task_with_notes_in_view(text, notes, due, View::Search)
    }
    fn add_task_with_notes_in_view(
        &mut self,
        text: String,
        notes: Option<String>,
        due: Option<String>,
        view: View,
    ) -> Outcome {
        let out = self.add_task(text, view);
        if !out.changed {
            return out;
        }
        let g = self;
        let Some(last) = g.store.pending.last().and_then(|p| match &p.action {
            Action::AddTask { task, .. } => Some(task.id.clone()),
            _ => None,
        }) else {
            return out;
        };
        let mut ch = Map::new();
        if let Some(n) = notes.filter(|n| !n.trim().is_empty()) {
            ch.insert("notes".into(), json!(n));
        }
        if let Some(d) = due.filter(|d| d.len() == 10) {
            ch.insert("dueDay".into(), json!(d));
        }
        if !ch.is_empty() {
            g.update_task(&last, ch);
        }
        out
    }
    fn add_from_text(&mut self, text: String, view: View) -> Outcome {
        let n = match tasks_from_text(&text) {
            TextTasks::Nothing => return Outcome::none(),
            TextTasks::Link { title, url } => {
                self.add_task_with_notes_in_view(title, Some(url), None, view);
                1
            }
            TextTasks::Lines(lines) => {
                let n = lines.len();
                for l in lines {
                    self.add_task(l, view.clone());
                }
                n
            }
            TextTasks::Paragraph { title, notes } => {
                self.add_task_with_notes_in_view(title, Some(notes), None, view);
                1
            }
        };
        changed_with(
            if n == 1 {
                Message::TaskAdded
            } else {
                Message::TasksAdded { n: n as u32 }
            },
            None,
        )
    }
    fn add_tag_to(&mut self, ids: Vec<String>, tag_id: String) -> Outcome {
        let g = self;
        let Some(name) = g.store.state.tag.entities.get(&tag_id).map(|t| t.title.clone()) else {
            return Outcome::none();
        };
        let tasks = g.tasks(&ids);
        let mut undo = vec![];
        for t in &tasks {
            if t.tag_ids.contains(&tag_id) {
                continue;
            }
            let mut tids = t.tag_ids.clone();
            tids.push(tag_id.clone());
            undo.push(Action::UpdateTask {
                id: t.id.clone(),
                changes: [("tagIds".to_string(), json!(t.tag_ids))].into_iter().collect(),
            });
            g.update_task(&t.id, [("tagIds".to_string(), json!(tids))].into_iter().collect());
        }
        if undo.is_empty() {
            return Outcome::none();
        }
        let n = undo.len() as u32;
        let undo = g.push_undo(undo);
        changed_with(
            if ids.len() == 1 {
                Message::Tagged { name }
            } else {
                Message::TasksTagged { n }
            },
            Some(undo),
        )
    }
    fn add_tag_by_name(&mut self, ids: Vec<String>, name: String) -> Outcome {
        if name.trim().is_empty() || !ids.iter().any(|id| self.store.state.task.entities.contains_key(id)) {
            return Outcome::none();
        }
        let id = self.ensure_tag(&name);
        self.add_tag_to(ids, id)
    }
    fn set_done(&mut self, id: String, done: bool) -> Outcome {
        let g = self;
        if g.task(&id).is_none() {
            return Outcome::none();
        }
        g.update_task(&id, [("isDone".to_string(), json!(done))].into_iter().collect());
        if !done {
            return Outcome::changed();
        }
        if let Some(undo) = g.auto_archive(std::slice::from_ref(&id)) {
            let undo = g.push_undo(undo);
            return Outcome {
                changed: true,
                message: Some(Message::TaskCompletedArchived),
                undo: Some(undo),
                sync_now: true,
            };
        }
        let undo = g.push_undo(vec![Action::UpdateTask {
            id: id.clone(),
            changes: [("isDone".to_string(), json!(false))].into_iter().collect(),
        }]);
        changed_with(Message::TaskCompleted, Some(undo))
    }
    fn reopen_tasks(&mut self, ids: Vec<String>) -> Outcome {
        let g = self;
        let mut seen = HashSet::new();
        let tasks: Vec<Task> = ids
            .into_iter()
            .filter(|id| seen.insert(id.clone()))
            .filter_map(|id| g.task(&id))
            .filter(|task| task.is_done)
            .collect();
        if tasks.is_empty() {
            return Outcome::none();
        }
        let mut undo = Vec::with_capacity(tasks.len());
        for task in tasks {
            g.update_task(&task.id, [("isDone".to_string(), json!(false))].into_iter().collect());
            undo.push(Action::UpdateTask {
                id: task.id,
                changes: [("isDone".to_string(), json!(true))].into_iter().collect(),
            });
        }
        Outcome {
            undo: Some(g.push_undo(undo)),
            ..Outcome::changed()
        }
    }

    fn dispatch(&mut self, a: Action) {
        if self.staged {
            self.store.stage(a);
            self.dirty = true;
        } else {
            self.store.dispatch(a);
        }
        self.invalidate();
    }
    fn push_undo(&mut self, actions: Vec<Action>) -> u64 {
        self.next_undo += 1;
        let id = self.next_undo;
        self.undo.push((id, actions));
        if self.undo.len() > UNDO_DEPTH {
            self.undo.remove(0);
        }
        id
    }
    fn task(&self, id: &str) -> Option<Task> {
        self.store.state.task.entities.get(id).cloned()
    }
    fn tasks(&self, ids: &[String]) -> Vec<Task> {
        ids.iter().filter_map(|i| self.task(i)).collect()
    }
    fn sub_tasks(&self, t: &Task) -> Vec<Task> {
        t.sub_task_ids.iter().filter_map(|i| self.task(i)).collect()
    }
    fn ensure_slot_tag(&mut self, slot: Slot) -> String {
        if let Some(id) = listing::slot_tag_id(&self.store, slot) {
            return id;
        }
        let tag = Tag::new(slot.tag_name());
        let id = tag.id.clone();
        self.dispatch(Action::AddTag { tag });
        id
    }
    /// An existing tag by name (case-insensitive) or a new one.
    fn ensure_tag(&mut self, name: &str) -> String {
        let existing = self
            .store
            .state
            .tag
            .iter()
            .find(|t| t.title.eq_ignore_ascii_case(name.trim()))
            .map(|t| t.id.clone());
        existing.unwrap_or_else(|| {
            let tag = Tag::new(name.trim());
            let id = tag.id.clone();
            self.dispatch(Action::AddTag { tag });
            id
        })
    }
    /// Tasks created in a project view go to that project; elsewhere to the first one.
    fn project_for(&self, view: &View) -> String {
        match view {
            View::Project { id } => id.clone(),
            _ => self
                .store
                .state
                .project
                .ids
                .first()
                .cloned()
                .unwrap_or(INBOX_PROJECT_ID.into()),
        }
    }
    fn update_task(&mut self, id: &str, changes: Map<String, Value>) {
        self.dispatch(Action::UpdateTask { id: id.into(), changes });
    }
    /// With auto-archive on, moves just-completed top-level tasks (and their subtasks) to
    /// the archive, like Archive Completed does. Returns the restore batch.
    fn auto_archive(&mut self, ids: &[String]) -> Option<Vec<Action>> {
        if !self.prefs.auto_archive {
            return None;
        }
        let tasks: Vec<Task> = ids
            .iter()
            .filter_map(|i| self.task(i))
            .filter(|t| t.is_done && t.parent_id.is_none())
            .collect();
        if tasks.is_empty() {
            return None;
        }
        let sub_tasks: Vec<Task> = tasks.iter().flat_map(|t| self.sub_tasks(t)).collect();
        let undo: Vec<Action> = tasks
            .iter()
            .map(|t| Action::RestoreTask {
                task: t.clone(),
                sub_tasks: sub_tasks
                    .iter()
                    .filter(|s| s.parent_id.as_deref() == Some(&t.id))
                    .cloned()
                    .collect(),
            })
            .collect();
        self.dispatch(Action::MoveToArchive { tasks, sub_tasks });
        Some(undo)
    }
    /// Move tasks between the plain day and a slot: add or remove the slot's tag, drop
    /// the other slot's tag, and plan for today if needed. The batch goes the direction
    /// of its first task.
    fn toggle_slot(&mut self, ids: &[String], slot: Slot, force_into: bool) -> Outcome {
        let tasks = self.tasks(ids);
        if tasks.is_empty() {
            return Outcome::none();
        }
        let tag = self.ensure_slot_tag(slot);
        let today = today_str();
        let into_slot = force_into || listing::slot_of(&self.store, &tasks[0]) != Some(slot);
        let mut undo = vec![];
        for t in &tasks {
            let mut ids = t.tag_ids.clone();
            if into_slot {
                if !ids.contains(&tag) {
                    ids.push(tag.clone());
                }
                ids.retain(|id| {
                    self.store
                        .state
                        .tag
                        .entities
                        .get(id)
                        .is_none_or(|tag| !tag.title.eq_ignore_ascii_case(slot.other().tag_name()))
                });
            } else {
                ids.retain(|id| {
                    self.store
                        .state
                        .tag
                        .entities
                        .get(id)
                        .is_none_or(|tag| !tag.title.eq_ignore_ascii_case(slot.tag_name()))
                });
            }
            let mut ch = Map::new();
            ch.insert("tagIds".into(), json!(ids));
            let mut back = Map::new();
            back.insert("tagIds".into(), json!(t.tag_ids));
            if t.plan_day().as_deref() != Some(&today) || t.due_with_time.is_some() {
                ch.insert("dueDay".into(), json!(today));
                ch.insert("dueWithTime".into(), Value::Null);
                ch.insert("remindAt".into(), Value::Null);
                back.insert("remindAt".into(), json!(t.remind_at));
                back.insert("dueDay".into(), json!(t.due_day));
                back.insert("dueWithTime".into(), json!(t.due_with_time));
            }
            undo.push(Action::UpdateTask {
                id: t.id.clone(),
                changes: back,
            });
            self.update_task(&t.id, ch);
        }
        let n = tasks.len() as u32;
        let message = match (into_slot, n, slot) {
            (true, 1, Slot::Tonight) => Message::MovedToTonight,
            (true, 1, Slot::Morning) => Message::MovedToMorning,
            (false, 1, _) => Message::MovedToToday,
            (true, n, Slot::Tonight) => Message::TasksMovedToTonight { n },
            (true, n, Slot::Morning) => Message::TasksMovedToMorning { n },
            (false, n, _) => Message::TasksMovedToToday { n },
        };
        let undo = self.push_undo(undo);
        Outcome {
            changed: true,
            message: Some(message),
            undo: Some(undo),
            sync_now: false,
        }
    }
    /// Push tasks to a day (keeps tags; clears any time of day). Tasks already on that
    /// day are skipped.
    fn move_to_day(&mut self, ids: &[String], day: &str, message: fn(u32) -> Message) -> Outcome {
        let tasks = self.tasks(ids);
        let mut undo = vec![];
        for t in tasks
            .iter()
            .filter(|t| t.plan_day().as_deref() != Some(day) || t.due_with_time.is_some())
        {
            let mut back = Map::new();
            back.insert("dueDay".into(), json!(t.due_day));
            back.insert("dueWithTime".into(), json!(t.due_with_time));
            back.insert("remindAt".into(), json!(t.remind_at));
            undo.push(Action::UpdateTask {
                id: t.id.clone(),
                changes: back,
            });
            let mut ch = Map::new();
            ch.insert("dueDay".into(), json!(day));
            ch.insert("dueWithTime".into(), Value::Null);
            ch.insert("remindAt".into(), Value::Null);
            self.update_task(&t.id, ch);
        }
        if undo.is_empty() {
            return Outcome::none();
        }
        let n = undo.len() as u32;
        let undo = self.push_undo(undo);
        Outcome {
            changed: true,
            message: Some(message(n)),
            undo: Some(undo),
            sync_now: false,
        }
    }
    fn delete_task(&mut self, id: &str) -> Option<Vec<Action>> {
        let task = self.task(id)?;
        let sub_tasks = self.sub_tasks(&task);
        self.dispatch(Action::DeleteTask {
            task: task.clone(),
            sub_tasks: sub_tasks.clone(),
        });
        let mut parent = task.clone();
        parent.sub_task_ids.clear(); // AddSubTask re-links each subtask exactly once
        let mut undo = vec![Action::AddTask {
            task: parent,
            bottom: true,
        }];
        undo.extend(sub_tasks.iter().map(|st| Action::AddSubTask {
            task: st.clone(),
            parent_id: task.id.clone(),
        }));
        Some(undo)
    }
    /// Where a reorder or nudge writes: the Today list, a project list or a tag list.
    fn list_context(view: &View) -> Option<(&'static str, String)> {
        Some(match view {
            View::Today | View::Morning | View::Tonight => ("TAG", TODAY_TAG_ID.to_string()),
            View::Project { id } => ("PROJECT", id.clone()),
            View::Tag { id } => ("TAG", id.clone()),
            _ => return None,
        })
    }
    /// Reminders due now that have not been shown yet; marks them shown.
    fn due_reminders(&mut self) -> Vec<ReminderDue> {
        let now = now_ms();
        let due: Vec<Task> = self
            .store
            .state
            .task
            .iter()
            .filter(|t| !t.is_done && t.remind_at.is_some_and(|r| r <= now) && !self.notified.contains(&t.id))
            .cloned()
            .collect();
        due.into_iter()
            .map(|t| {
                self.notified.insert(t.id.clone());
                ReminderDue {
                    task_id: t.id,
                    title: t.title,
                    time: t.due_with_time.map(clock_time),
                }
            })
            .collect()
    }
    /// Create today's (or the newest missed day's) instance of every due repeat config.
    pub(crate) fn spawn_repeats(&mut self) -> u32 {
        let today = today_str();
        let due: Vec<(RepeatCfg, String)> = self
            .store
            .state
            .task_repeat_cfg
            .iter()
            .filter_map(|c| c.newest_due_day(&today).map(|d| (c.clone(), d)))
            .filter(|(c, day)| {
                let id = format!("rpt_{}_{}", c.id, day);
                !self.store.state.task.entities.contains_key(&id) && !listing::archived_has_task(&self.store, &id)
            })
            .collect();
        let n = due.len() as u32;
        for (cfg, day) in due {
            let task = cfg.task_for_day(&day, &self.project_for(&View::Today), now_ms());
            self.dispatch(Action::AddTask { task, bottom: true });
            let changes = [
                ("lastTaskCreationDay".to_string(), json!(day)),
                ("lastTaskCreation".to_string(), json!(now_ms())),
            ]
            .into_iter()
            .collect();
            self.dispatch(Action::UpdateRepeatCfg {
                id: cfg.id.clone(),
                changes,
            });
        }
        n
    }
}

fn changed_with(message: Message, undo: Option<u64>) -> Outcome {
    Outcome {
        changed: true,
        message: Some(message),
        undo,
        sync_now: false,
    }
}

fn draft_to_cfg(base: &RepeatCfg, d: &RepeatDraft) -> RepeatCfg {
    let mut c = base.clone();
    c.repeat_cycle = match d.cycle {
        RepeatCycle::Daily => "DAILY",
        RepeatCycle::Weekly => "WEEKLY",
        RepeatCycle::Monthly => "MONTHLY",
        RepeatCycle::Yearly => "YEARLY",
    }
    .into();
    c.repeat_every = d.every.max(1);
    let wd = |i: usize| d.weekdays.get(i).copied().unwrap_or(false);
    c.sunday = wd(0);
    c.monday = wd(1);
    c.tuesday = wd(2);
    c.wednesday = wd(3);
    c.thursday = wd(4);
    c.friday = wd(5);
    c.saturday = wd(6);
    c.monthly_last_day = d.monthly == MonthlyRule::LastDay;
    match d.monthly {
        MonthlyRule::NthWeekday { week, weekday } => {
            c.monthly_week_of_month = Some(week);
            c.monthly_weekday = Some(weekday);
        }
        _ => {
            c.monthly_week_of_month = None;
            c.monthly_weekday = None;
        }
    }
    c.start_date = d.start_date.clone();
    c.is_paused = d.paused;
    c
}

fn cfg_to_draft(c: &RepeatCfg, existing: bool) -> RepeatDraft {
    let cycle = match c.repeat_cycle.as_str() {
        "DAILY" => RepeatCycle::Daily,
        "MONTHLY" => RepeatCycle::Monthly,
        "YEARLY" => RepeatCycle::Yearly,
        _ => RepeatCycle::Weekly,
    };
    let monthly = if let Some((week, weekday)) = c.nth_weekday_anchor() {
        MonthlyRule::NthWeekday { week, weekday }
    } else if c.monthly_last_day {
        MonthlyRule::LastDay
    } else {
        MonthlyRule::SameDay
    };
    RepeatDraft {
        cycle,
        every: c.repeat_every.max(1),
        weekdays: c.weekdays().to_vec(),
        monthly,
        start_date: c.start_date.clone(),
        paused: c.is_paused,
        existing,
    }
}

/// The engine. One per data directory; both apps create exactly one.
#[cfg_attr(feature = "ffi", derive(uniffi::Object))]
pub struct Engine {
    pub(crate) inner: Mutex<Inner>,
    pub(crate) syncing: AtomicBool,
    #[cfg(feature = "p2p")]
    pub(crate) p2p: Mutex<Option<Arc<crate::p2p::Runtime>>>,
    pub(crate) ipc: Mutex<Option<crate::ipc::Server>>,
}

/// Keep the exchange marked active through its final guarded persistence, including
/// error exits. A second sync cannot snapshot the store in the commit gap.
struct SyncLease<'a>(&'a AtomicBool);
impl Drop for SyncLease<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

impl Engine {
    pub(crate) fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
    #[cfg(test)]
    pub(crate) fn has_search_index(&self) -> bool {
        self.lock().index.is_some()
    }
    /// Apply rules to a private candidate, persist once, then publish under the same
    /// owner lock. A failed write cannot consume undo or wake sync with unsaved state.
    fn try_edit<T>(&self, change: impl FnOnce(&mut Inner) -> T) -> Result<T, CoreError> {
        self.try_edit_checked(|candidate| Ok(change(candidate)))
    }
    pub(crate) fn try_edit_checked<T>(
        &self,
        change: impl FnOnce(&mut Inner) -> Result<T, CoreError>,
    ) -> Result<T, CoreError> {
        let mut live = self.lock();
        let mut candidate = live.clone();
        candidate.staged = true;
        candidate.dirty = false;
        candidate.change_signal = None;
        let result = change(&mut candidate)?;
        let changed = candidate.dirty;
        if changed {
            candidate.store.save()?;
        }
        candidate.staged = false;
        candidate.dirty = false;
        candidate.change_signal = live.change_signal.clone();
        *live = candidate;
        if changed {
            live.invalidate();
        }
        Ok(result)
    }
    fn edit(&self, change: impl FnOnce(&mut Inner) -> Outcome) -> Outcome {
        self.try_edit(change).unwrap_or_else(save_failure)
    }
    fn from_store(dir: PathBuf, store: Store) -> Arc<Self> {
        let _ = dir;
        Arc::new(Self {
            inner: Mutex::new(Inner {
                staged: false,
                dirty: false,
                store,
                prefs: Preferences::default(),
                undo: vec![],
                next_undo: 0,
                sync_generation: 0,
                index: None,
                notified: HashSet::new(),
                last_day: today_str(),
                change_signal: None,
            }),
            syncing: AtomicBool::new(false),
            #[cfg(feature = "p2p")]
            p2p: Mutex::new(None),
            ipc: Mutex::new(None),
        })
    }
}

#[cfg_attr(feature = "ffi", uniffi::export)]
impl Engine {
    /// Opens (or creates) the store in `dir`.
    #[cfg_attr(feature = "ffi", uniffi::constructor)]
    pub fn open(dir: String) -> Arc<Self> {
        let dir = PathBuf::from(dir);
        Self::from_store(dir.clone(), Store::load(dir))
    }
    /// Checked startup for native clients: recovery errors must not become empty data.
    #[cfg_attr(feature = "ffi", uniffi::constructor)]
    pub fn open_checked(dir: String) -> Result<Arc<Self>, CoreError> {
        let dir = PathBuf::from(dir);
        let store = Store::try_load(dir.clone())?;
        Ok(Self::from_store(dir, store))
    }
    /// A fresh store in `dir` filled with the sample data used for screenshots and tests.
    #[cfg_attr(feature = "ffi", uniffi::constructor)]
    pub fn demo(dir: String) -> Arc<Self> {
        let dir = PathBuf::from(dir);
        Self::from_store(dir.clone(), crate::demo::store(dir))
    }
    pub fn data_dir(&self) -> String {
        self.lock().store.dir().to_string_lossy().into_owned()
    }
    pub fn client_id(&self) -> String {
        self.lock().store.meta.client_id.clone()
    }
    pub fn set_preferences(&self, prefs: Preferences) {
        self.lock().prefs = prefs;
    }
    pub fn preferences(&self) -> Preferences {
        self.lock().prefs.clone()
    }

    // ---- queries ---------------------------------------------------------

    pub fn sidebar(&self) -> Sidebar {
        listing::sidebar(&self.lock().store)
    }
    /// The rows of a view. `archive_limit` is how many archived tasks to show (page size
    /// times pages revealed); ignored elsewhere.
    pub fn listing(&self, view: View, archive_limit: u32) -> Listing {
        let mut g = self.lock();
        let idx = matches!(&view, View::Archive).then(|| g.index());
        listing::group_listing(
            &g.store,
            listing::listing(&g.store, idx.as_deref(), &view, &g.prefs, archive_limit as usize),
            g.prefs.group_by,
        )
    }
    /// Upcoming top-level tasks in the configured window, optionally including completed
    /// tasks for automation queries. Uses the same local-day bounds and order as the view.
    pub fn upcoming_task_ids(&self, include_completed: bool) -> Vec<String> {
        let g = self.lock();
        let today = day_number(&today_str()).unwrap_or(0);
        let end = today + g.prefs.upcoming_days as i64;
        let mut tasks: Vec<&Task> = g
            .store
            .state
            .task
            .iter()
            .filter(|t| (include_completed || !t.is_done) && t.parent_id.is_none())
            .filter(|t| {
                t.plan_day()
                    .as_deref()
                    .and_then(day_number)
                    .is_some_and(|day| day > today && day <= end)
            })
            .collect();
        tasks.sort_by(|a, b| {
            a.plan_day()
                .cmp(&b.plan_day())
                .then_with(|| a.due_with_time.cmp(&b.due_with_time))
                .then_with(|| a.title.cmp(&b.title))
        });
        tasks.into_iter().map(|t| t.id.clone()).collect()
    }
    pub fn search(&self, query: String) -> Listing {
        let mut g = self.lock();
        let idx = g.index();
        listing::group_listing(
            &g.store,
            listing::search(&g.store, &idx, query.trim()),
            g.prefs.group_by,
        )
    }
    /// Up to eight open tasks matching every query word in the title, for the desktop's
    /// search (GNOME Shell, KRunner, Spotlight).
    pub fn quick_matches(&self, query: String) -> Vec<TaskBrief> {
        listing::quick_matches(&self.lock().store, &query, QUICK_MATCHES)
    }
    /// Every live task, briefly, for a search index such as Core Spotlight.
    pub fn all_tasks(&self) -> Vec<TaskBrief> {
        let g = self.lock();
        g.store
            .state
            .task
            .iter()
            .map(|t| listing::task_brief(&g.store, t))
            .collect()
    }
    pub fn view_title(&self, view: View) -> ViewTitle {
        listing::view_title(&self.lock().store, &view)
    }
    pub fn view_exists(&self, view: View) -> bool {
        let g = self.lock();
        match &view {
            View::Project { id } => g.store.state.project.entities.contains_key(id),
            View::Tag { id } => g.store.state.tag.entities.contains_key(id),
            View::Morning => listing::today_uses_slot(&g.store, Slot::Morning),
            View::Tonight => listing::today_uses_slot(&g.store, Slot::Tonight),
            _ => true,
        }
    }
    pub fn projects(&self) -> Vec<ProjectRef> {
        self.lock()
            .store
            .state
            .project
            .iter()
            .filter(|p| !p.is_archived)
            .map(listing::project_ref)
            .collect()
    }
    pub fn tags(&self) -> Vec<TagRef> {
        self.lock()
            .store
            .state
            .tag
            .iter()
            .filter(|t| t.id != TODAY_TAG_ID)
            .map(listing::tag_ref)
            .collect()
    }
    pub fn project(&self, id: String) -> Option<ProjectRef> {
        self.lock()
            .store
            .state
            .project
            .entities
            .get(&id)
            .map(listing::project_ref)
    }
    pub fn tag(&self, id: String) -> Option<TagRef> {
        self.lock().store.state.tag.entities.get(&id).map(listing::tag_ref)
    }
    /// How many tasks a project delete would take with it.
    pub fn project_task_count(&self, id: String) -> u32 {
        self.lock()
            .store
            .state
            .task
            .iter()
            .filter(|t| t.project_id == id)
            .count() as u32
    }
    pub fn task_row(&self, id: String) -> Option<TaskRow> {
        let g = self.lock();
        g.task(&id)
            .map(|t| listing::task_row(&g.store, &View::Search, &t, false))
    }
    pub fn task_detail(&self, id: String) -> Option<TaskDetail> {
        let g = self.lock();
        let t = g.task(&id)?;
        let reminder_minutes_before = match (t.remind_at, t.due_with_time) {
            (Some(r), Some(d)) if d >= r => Some(((d - r) / 60_000) as u32),
            _ => None,
        };
        let repeat = t
            .repeat_cfg_id
            .as_ref()
            .and_then(|id| g.store.state.task_repeat_cfg.entities.get(id))
            .map(describe_repeat);
        Some(TaskDetail {
            id: t.id.clone(),
            title: t.title.clone(),
            notes: t.notes.clone().unwrap_or_default(),
            project_id: t.project_id.clone(),
            is_done: t.is_done,
            parent_id: t.parent_id.clone(),
            due_day: t.plan_day(),
            time: t.due_with_time.map(clock_time),
            reminder_minutes_before,
            estimate_ms: t.time_estimate,
            tag_ids: t.tag_ids.clone(),
            sub_tasks: g
                .sub_tasks(&t)
                .iter()
                .map(|s| listing::task_row(&g.store, &View::Search, s, false))
                .collect(),
            repeat,
        })
    }
    pub fn task_menu(&self, id: String) -> Option<TaskMenuInfo> {
        let g = self.lock();
        let t = g.task(&id)?;
        Some(TaskMenuInfo {
            id: t.id.clone(),
            title: t.title.clone(),
            is_done: t.is_done,
            planned_today: t.due_day.as_deref() == Some(today_str().as_str()),
            slot: listing::slot_of(&g.store, &t),
            top_level: t.parent_id.is_none(),
            repeats: t.repeat_cfg_id.is_some(),
        })
    }
    pub fn task_title(&self, id: String) -> Option<String> {
        self.lock().task(&id).map(|t| t.title)
    }
    /// Read-only names for references to live or archived tasks. Unlike `task_title`,
    /// this does not imply that the task can be opened, edited or dragged. Requested
    /// IDs are resolved under one lock without materializing the full archive.
    pub fn task_reference_titles(&self, ids: Vec<String>) -> HashMap<String, String> {
        let g = self.lock();
        let mut titles = HashMap::new();
        for id in ids {
            if titles.contains_key(&id) {
                continue;
            }
            let title = g
                .store
                .state
                .task
                .entities
                .get(&id)
                .map(|task| task.title.clone())
                .or_else(|| {
                    ["archiveYoung", "archiveOld"].into_iter().find_map(|tier| {
                        let value = g.store.state.rest.get(tier)?.get("task")?.get("entities")?.get(&id)?;
                        serde_json::from_value::<Task>(value.clone())
                            .ok()
                            .map(|task| task.title)
                    })
                });
            if let Some(title) = title {
                titles.insert(id, title);
            }
        }
        titles
    }
    /// Every line is the id of a live task: our own row drags, not text to make tasks of.
    pub fn is_task_id_list(&self, text: String) -> bool {
        let g = self.lock();
        !text.trim().is_empty() && text.lines().all(|l| g.store.state.task.entities.contains_key(l.trim()))
    }
    /// Tags whose name starts with `prefix`, for the `#` popover (at most eight).
    pub fn tag_completions(&self, prefix: String) -> Vec<TagCompletion> {
        let lower = prefix.to_lowercase();
        self.lock()
            .store
            .state
            .tag
            .iter()
            .filter(|t| t.id != TODAY_TAG_ID && t.title.to_lowercase().starts_with(&lower))
            .map(|t| TagCompletion {
                title: t.title.clone(),
                color: tag_color(t),
                task_count: t.task_ids.len() as u32,
            })
            .take(8)
            .collect()
    }
    /// Unfinished top-level tasks for a desktop badge or background status.
    pub fn task_count(&self, mode: TaskCountMode) -> u32 {
        let g = self.lock();
        let ids = match mode {
            TaskCountMode::DueToday => g.store.state.today_ids(),
            TaskCountMode::TodayIncludingOverdue => {
                let mut ids = g.store.state.today_ids();
                ids.extend(g.store.state.overdue_ids());
                ids
            }
            TaskCountMode::None => return 0,
        };
        ids.into_iter()
            .filter(|id| {
                g.store
                    .state
                    .task
                    .entities
                    .get(id)
                    .is_some_and(|task| !task.is_done && task.parent_id.is_none())
            })
            .collect::<HashSet<_>>()
            .len() as u32
    }
    /// Open tasks planned for today, kept while existing clients migrate to `task_count`.
    pub fn today_open_count(&self) -> u32 {
        self.task_count(TaskCountMode::DueToday)
    }
    pub fn pending_count(&self) -> u32 {
        self.lock().store.pending.len() as u32
    }
    pub fn can_undo(&self) -> bool {
        !self.lock().undo.is_empty()
    }
    /// The batch Ctrl+Z would undo next, so a menu can say "Undo Delete Task".
    pub fn undo_top(&self) -> Option<u64> {
        self.lock().undo.last().map(|(id, _)| *id)
    }
    pub fn can_archive(&self) -> bool {
        !listing::done_tasks(&self.lock().store).0.is_empty()
    }

    // ---- creating --------------------------------------------------------

    /// Quick-add: `#tag` adds or creates tags, a trailing `1h 30m` sets the estimate; a
    /// day view plans the task for today, a slot view adds its tag, a tag view its tag.
    pub fn add_task(&self, text: String, view: View) -> Outcome {
        self.edit(|g| g.add_task(text, view))
    }
    /// Quick-add straight into Today (the quick-add window, `--add`, the search provider).
    pub fn add_task_for_today(&self, text: String) -> Outcome {
        self.add_task(text, View::Today)
    }
    /// URL scheme and drops: title with short syntax, optional notes and due day, added to
    /// the first project without planning it.
    pub fn add_task_with_notes(&self, text: String, notes: Option<String>, due: Option<String>) -> Outcome {
        self.edit(|g| g.add_task_with_notes(text, notes, due))
    }
    /// Text from a drop or a multi-line paste: a URL or a paragraph becomes one task with
    /// the text in its notes; several short lines become several tasks.
    pub fn add_from_text(&self, text: String, view: View) -> Outcome {
        self.edit(|g| g.add_from_text(text, view))
    }
    /// The New Task dialog. The view adds its slot or tag like quick-add does.
    pub fn create_task(&self, draft: TaskDraft, view: View) -> Outcome {
        self.create_task_with_id(draft, view).outcome
    }
    /// Creates a task and returns its own ID while holding the store lock, so concurrent
    /// automation requests never have to infer identity from the latest pending action.
    pub fn create_task_with_id(&self, draft: TaskDraft, view: View) -> TaskCreation {
        if draft.title.trim().is_empty() {
            return TaskCreation {
                outcome: Outcome::none(),
                id: None,
            };
        }
        self.try_edit(|g| {
            let project = if draft.project_id.is_empty() {
                g.project_for(&view)
            } else {
                draft.project_id.clone()
            };
            let mut t = Task::new(draft.title.trim(), &project);
            t.due_with_time = draft
                .time
                .and_then(|c| local_ms(&draft.due_day.clone().unwrap_or_else(today_str), c.hour, c.minute));
            t.due_day = draft.due_day.clone().filter(|_| t.due_with_time.is_none());
            t.remind_at = match (t.due_with_time, draft.reminder_minutes_before) {
                (Some(d), Some(m)) => Some(d.saturating_sub(m as u64 * 60_000)),
                _ => None,
            };
            t.time_estimate = draft.estimate_ms;
            if !draft.notes.is_empty() {
                t.notes = Some(draft.notes.clone());
            }
            t.tag_ids = draft.tag_ids.clone();
            for name in draft.new_tags.iter().map(|s| s.trim()).filter(|s| !s.is_empty()) {
                let id = g.ensure_tag(name);
                if !t.tag_ids.contains(&id) {
                    t.tag_ids.push(id);
                }
            }
            match &view {
                View::Tag { id } if !t.tag_ids.contains(id) => t.tag_ids.push(id.clone()),
                View::Morning | View::Tonight => {
                    let slot = if view == View::Morning {
                        Slot::Morning
                    } else {
                        Slot::Tonight
                    };
                    let id = g.ensure_slot_tag(slot);
                    if !t.tag_ids.contains(&id) {
                        t.tag_ids.push(id);
                    }
                }
                _ => {}
            }
            let id = t.id.clone();
            g.dispatch(Action::AddTask { task: t, bottom: true });
            TaskCreation {
                outcome: Outcome::changed(),
                id: Some(id),
            }
        })
        .unwrap_or_else(|error| TaskCreation {
            outcome: save_failure(error),
            id: None,
        })
    }
    /// The edit dialog closing: only the fields that differ become one `updateTask`.
    pub fn save_task(&self, id: String, draft: TaskDraft) -> Outcome {
        self.edit(|g| {
            let Some(t) = g.task(&id) else {
                return Outcome::none();
            };
            let mut ch = Map::new();
            let nt = draft.title.trim().to_string();
            if !nt.is_empty() && nt != t.title {
                ch.insert("title".into(), json!(nt));
            }
            if draft.estimate_ms != t.time_estimate {
                ch.insert("timeEstimate".into(), json!(draft.estimate_ms));
            }
            // A time fixes the day too: `dueWithTime` and `dueDay` never coexist upstream.
            let nw = draft
                .time
                .and_then(|c| local_ms(&draft.due_day.clone().unwrap_or_else(today_str), c.hour, c.minute));
            let nd = draft.due_day.clone().filter(|_| nw.is_none());
            if nw != t.due_with_time {
                ch.insert("dueWithTime".into(), json!(nw));
            }
            if nd != t.due_day {
                ch.insert("dueDay".into(), json!(nd));
            }
            let nr = match (nw, draft.reminder_minutes_before) {
                (Some(d), Some(m)) => Some(d.saturating_sub(m as u64 * 60_000)),
                _ => None,
            };
            if nr != t.remind_at {
                ch.insert("remindAt".into(), json!(nr));
                g.notified.remove(&id);
            }
            if t.parent_id.is_none() && !draft.project_id.is_empty() && draft.project_id != t.project_id {
                ch.insert("projectId".into(), json!(draft.project_id));
            }
            if draft.notes != t.notes.clone().unwrap_or_default() {
                ch.insert("notes".into(), json!(draft.notes));
            }
            let mut ids = draft.tag_ids.clone();
            for name in draft.new_tags.iter().map(|s| s.trim()).filter(|s| !s.is_empty()) {
                let tid = g.ensure_tag(name);
                if !ids.contains(&tid) {
                    ids.push(tid);
                }
            }
            if ids != t.tag_ids {
                ch.insert("tagIds".into(), json!(ids));
            }
            if ch.is_empty() {
                return Outcome::none();
            }
            g.update_task(&id, ch);
            Outcome::changed()
        })
    }
    pub fn add_subtask(&self, parent_id: String, title: String) -> Outcome {
        if title.trim().is_empty() {
            return Outcome::none();
        }
        self.edit(|g| {
            if g.task(&parent_id).is_none() {
                return Outcome::none();
            }
            let mut task = Task::new(title.trim(), "");
            task.parent_id = Some(parent_id.clone());
            g.dispatch(Action::AddSubTask { task, parent_id });
            Outcome::changed()
        })
    }
    /// A fresh standalone copy with the same project, tags, estimate, notes and schedule.
    pub fn duplicate_task(&self, id: String) -> Outcome {
        self.edit(|g| {
            let Some(t) = g.task(&id) else {
                return Outcome::none();
            };
            let mut copy = Task::new(&t.title, &t.project_id);
            copy.tag_ids = t.tag_ids.clone();
            copy.time_estimate = t.time_estimate;
            copy.notes = t.notes.clone();
            copy.due_day = t.due_day.clone();
            copy.due_with_time = t.due_with_time;
            copy.remind_at = t.remind_at;
            g.dispatch(Action::AddTask {
                task: copy.clone(),
                bottom: true,
            });
            let undo = g.push_undo(vec![Action::DeleteTask {
                task: copy,
                sub_tasks: vec![],
            }]);
            changed_with(Message::TaskDuplicated, Some(undo))
        })
    }
    /// The id of the most recently created task, to focus it after a duplicate or add.
    pub fn last_added_id(&self) -> Option<String> {
        self.lock().store.pending.iter().rev().find_map(|p| match &p.action {
            Action::AddTask { task, .. } => Some(task.id.clone()),
            _ => None,
        })
    }

    // ---- completing, deleting ------------------------------------------------

    pub fn set_done(&self, id: String, done: bool) -> Outcome {
        self.edit(|g| g.set_done(id, done))
    }
    pub fn toggle_done(&self, id: String) -> Outcome {
        let done = self.lock().task(&id).map(|t| t.is_done).unwrap_or(true);
        self.set_done(id, !done)
    }
    /// A notification's Done button, or `superproductivity://complete-task`.
    pub fn complete_by_title(&self, title: String) -> Outcome {
        let id = self
            .lock()
            .store
            .state
            .task
            .iter()
            .find(|t| !t.is_done && t.title.eq_ignore_ascii_case(title.trim()))
            .map(|t| t.id.clone());
        match id {
            Some(id) => self.set_done(id, true),
            None => Outcome::none(),
        }
    }
    pub fn delete_task(&self, id: String) -> Outcome {
        self.edit(|g| {
            let Some(undo) = g.delete_task(&id) else {
                return Outcome::none();
            };
            let undo = g.push_undo(undo);
            changed_with(Message::TaskDeleted, Some(undo))
        })
    }
    /// Mark several tasks done in one undoable batch.
    pub fn bulk_done(&self, ids: Vec<String>) -> Outcome {
        self.edit(|g| {
            let tasks = g.tasks(&ids);
            if tasks.is_empty() {
                return Outcome::none();
            }
            let n = tasks.len() as u32;
            let undo: Vec<Action> = tasks
                .iter()
                .map(|t| Action::UpdateTask {
                    id: t.id.clone(),
                    changes: [("isDone".to_string(), json!(t.is_done))].into_iter().collect(),
                })
                .collect();
            for t in &tasks {
                g.update_task(&t.id, [("isDone".to_string(), json!(true))].into_iter().collect());
            }
            let ids: Vec<String> = tasks.iter().map(|t| t.id.clone()).collect();
            if let Some(archived) = g.auto_archive(&ids) {
                // Restoring reopens the archived ones; the rest (subtasks) get their flag back.
                let mut batch = archived;
                batch.extend(undo.into_iter().filter(|a| match a {
                    Action::UpdateTask { id, .. } => {
                        !ids.iter().any(|i| i == id) || tasks.iter().any(|t| t.id == *id && t.parent_id.is_some())
                    }
                    _ => true,
                }));
                let undo = g.push_undo(batch);
                return Outcome {
                    changed: true,
                    message: Some(Message::TasksCompletedArchived { n }),
                    undo: Some(undo),
                    sync_now: true,
                };
            }
            let undo = g.push_undo(undo);
            changed_with(Message::TasksCompleted { n }, Some(undo))
        })
    }
    /// Reopen completed tasks as one undoable batch. Missing, duplicate and already-open
    /// tasks are ignored; reopening never triggers automatic archiving.
    pub fn reopen_tasks(&self, ids: Vec<String>) -> Outcome {
        self.edit(|g| g.reopen_tasks(ids))
    }
    pub fn bulk_delete(&self, ids: Vec<String>) -> Outcome {
        self.edit(|g| {
            let mut undo = vec![];
            for id in &ids {
                if let Some(batch) = g.delete_task(id) {
                    undo.extend(batch);
                }
            }
            if undo.is_empty() {
                return Outcome::none();
            }
            let n = ids.len() as u32;
            let undo = g.push_undo(undo);
            changed_with(Message::TasksDeleted { n }, Some(undo))
        })
    }

    // ---- day moves -------------------------------------------------------

    /// Plan for today (a drop on Today, Ctrl+T). Tasks already planned today are skipped.
    pub fn plan_for_today(&self, ids: Vec<String>) -> Outcome {
        self.edit(|g| {
            let today = today_str();
            let tasks: Vec<Task> = g
                .tasks(&ids)
                .into_iter()
                .filter(|t| t.plan_day().as_deref() != Some(&today) || t.due_with_time.is_some())
                .collect();
            if tasks.is_empty() {
                return Outcome::none();
            }
            let undo: Vec<Action> = tasks
                .iter()
                .map(|t| Action::UpdateTask {
                    id: t.id.clone(),
                    changes: [
                        ("dueDay".to_string(), json!(t.due_day)),
                        ("dueWithTime".to_string(), json!(t.due_with_time)),
                        ("remindAt".to_string(), json!(t.remind_at)),
                    ]
                    .into_iter()
                    .collect(),
                })
                .collect();
            let n = tasks.len() as u32;
            g.dispatch(Action::PlanForToday {
                task_ids: tasks.iter().map(|t| t.id.clone()).collect(),
                today,
            });
            let undo = g.push_undo(undo);
            changed_with(
                if n == 1 {
                    Message::PlannedForToday
                } else {
                    Message::TasksPlannedForToday { n }
                },
                Some(undo),
            )
        })
    }
    pub fn remove_from_today(&self, id: String) -> Outcome {
        self.edit(|g| {
            let Some(t) = g.task(&id) else {
                return Outcome::none();
            };
            if t.due_day.as_deref() != Some(today_str().as_str()) {
                return Outcome::none();
            }
            g.dispatch(Action::RemoveFromToday {
                task_ids: vec![id.clone()],
            });
            let undo = g.push_undo(vec![Action::PlanForToday {
                task_ids: vec![id],
                today: today_str(),
            }]);
            changed_with(Message::RemovedFromToday, Some(undo))
        })
    }
    /// Ctrl+T and the context menu: plan for today, or take it off today if it is there.
    pub fn toggle_today(&self, id: String) -> Outcome {
        let planned = self
            .lock()
            .task(&id)
            .map(|t| t.due_day.as_deref() == Some(today_str().as_str()))
            .unwrap_or(false);
        if planned {
            self.remove_from_today(id)
        } else {
            self.plan_for_today(vec![id])
        }
    }
    pub fn toggle_slot(&self, ids: Vec<String>, slot: Slot) -> Outcome {
        self.edit(|g| g.toggle_slot(&ids, slot, false))
    }
    pub fn move_to_tomorrow(&self, ids: Vec<String>) -> Outcome {
        self.edit(|g| {
            g.move_to_day(&ids, &tomorrow(), |n| {
                if n == 1 {
                    Message::MovedToTomorrow
                } else {
                    Message::TasksMovedToTomorrow { n }
                }
            })
        })
    }
    pub fn move_to_next_week(&self, ids: Vec<String>) -> Outcome {
        self.edit(|g| {
            g.move_to_day(&ids, &next_monday(), |n| {
                if n == 1 {
                    Message::MovedToNextWeek
                } else {
                    Message::TasksMovedToNextWeek { n }
                }
            })
        })
    }
    /// Move top-level tasks (with their subtasks) to another project, one undo for all.
    pub fn move_to_project(&self, ids: Vec<String>, project_id: String) -> Outcome {
        self.edit(|g| {
            let name = g.store.state.project.entities.get(&project_id).map(|p| p.title.clone());
            let Some(name) = name else {
                return Outcome::none();
            };
            let tasks: Vec<Task> = g
                .tasks(&ids)
                .into_iter()
                .filter(|t| t.parent_id.is_none() && t.project_id != project_id)
                .collect();
            if tasks.is_empty() {
                return Outcome::none();
            }
            let mut undo = vec![];
            for t in &tasks {
                let subs = g.sub_tasks(t);
                let moved = Task {
                    project_id: project_id.clone(),
                    ..t.clone()
                };
                undo.push(Action::MoveToProject {
                    task: moved,
                    sub_tasks: subs.clone(),
                    target_project_id: t.project_id.clone(),
                });
                g.dispatch(Action::MoveToProject {
                    task: t.clone(),
                    sub_tasks: subs,
                    target_project_id: project_id.clone(),
                });
            }
            let undo = g.push_undo(undo);
            changed_with(Message::MovedToProject { name }, Some(undo))
        })
    }
    /// Add a tag (by id) to tasks that lack it. `Tagged` for one task, `TasksTagged` for a batch.
    pub fn add_tag_to(&self, ids: Vec<String>, tag_id: String) -> Outcome {
        self.edit(|g| g.add_tag_to(ids, tag_id))
    }
    /// Add a tag by name, creating it when new (the bulk Add Tag… dialog's text field).
    pub fn add_tag_by_name(&self, ids: Vec<String>, name: String) -> Outcome {
        self.edit(|g| g.add_tag_by_name(ids, name))
    }
    /// A task (or several) dropped on a sidebar entry: move to project, add tag, plan for
    /// today, or move into a slot. False when nothing applied.
    pub fn drop_tasks(&self, ids: Vec<String>, dest: View) -> Outcome {
        match dest {
            View::Today => self.plan_for_today(ids),
            View::Morning | View::Tonight => {
                let slot = if dest == View::Morning {
                    Slot::Morning
                } else {
                    Slot::Tonight
                };
                self.edit(|g| {
                    let today = today_str();
                    // Already in the slot and planned today: nothing to do.
                    let ids: Vec<String> = g
                        .tasks(&ids)
                        .into_iter()
                        .filter(|t| {
                            !(t.plan_day().as_deref() == Some(&today)
                                && listing::slot_of(&g.store, t) == Some(slot)
                                && t.due_with_time.is_none())
                        })
                        .map(|t| t.id)
                        .collect();
                    if ids.is_empty() {
                        return Outcome::none();
                    }
                    // A drop always moves into the destination, even when a future task already has its tag.
                    let mut out = g.toggle_slot(&ids, slot, true);
                    if ids.len() == 1 {
                        out.message = Some(if slot == Slot::Morning {
                            Message::PlannedForMorning
                        } else {
                            Message::PlannedForTonight
                        });
                    }
                    out
                })
            }
            View::Project { id } => self.move_to_project(ids, id),
            View::Tag { id } => self.add_tag_to(ids, id),
            _ => Outcome::none(),
        }
    }

    // ---- ordering --------------------------------------------------------

    /// Drag reorder: put one task before a row in the displayed list.
    pub fn reorder(&self, moved: String, before: String, view: View) -> Outcome {
        self.reorder_tasks(vec![moved], before, view)
    }
    /// A selection drag is one ordered change and one undo batch on either platform.
    pub fn reorder_tasks(&self, moved: Vec<String>, before: String, view: View) -> Outcome {
        self.edit(|g| {
            if moved.is_empty() || moved.contains(&before) {
                return Outcome::none();
            }
            if g.prefs.sort != SortKey::Manual {
                return Outcome {
                    changed: false,
                    message: Some(Message::ManualOrderOnly),
                    undo: None,
                    sync_now: false,
                };
            }
            let Some((context_type, context_id)) = Inner::list_context(&view) else {
                return Outcome::none();
            };
            let visible = listing::view_task_ids(&g.store, &view);
            let is_top_level = |id: &String| g.task(id).is_some_and(|t| t.parent_id.is_none());
            if !visible.contains(&before) || !is_top_level(&before) {
                return Outcome::none();
            }
            let moved: HashSet<String> = moved
                .into_iter()
                .filter(|id| visible.contains(id) && is_top_level(id))
                .collect();
            if moved.is_empty() {
                return Outcome::none();
            }
            let group_for = |id: &String| {
                g.task(id).and_then(|t| {
                    listing::task_group(
                        &g.store,
                        &view,
                        &listing::task_row(&g.store, &view, &t, false),
                        g.prefs.group_by,
                    )
                })
            };
            let target_group = group_for(&before);
            if moved.iter().any(|id| group_for(id) != target_group) {
                return Outcome::none();
            }
            // Slots are filtered views of TODAY. Undo uses full-context predecessors so it
            // also restores the position relative to tasks hidden by the slot filter.
            let context_view = if matches!(view, View::Morning | View::Tonight) {
                View::Today
            } else {
                view
            };
            let original = listing::view_task_ids(&g.store, &context_view);
            let mut displayed = original.clone();
            if g.prefs.direction == SortDirection::Descending {
                displayed.reverse();
            }
            let selected: Vec<String> = displayed.iter().filter(|id| moved.contains(*id)).cloned().collect();
            displayed.retain(|id| !moved.contains(id));
            let Some(pos) = displayed.iter().position(|id| *id == before) else {
                return Outcome::none();
            };
            displayed.splice(pos..pos, selected);
            if g.prefs.direction == SortDirection::Descending {
                displayed.reverse();
            }
            if displayed == original {
                return Outcome::none();
            }
            let actions = |order: &[String]| -> Vec<Action> {
                order
                    .iter()
                    .enumerate()
                    .filter(|(_, id)| moved.contains(*id))
                    .map(|(i, id)| Action::MoveInList {
                        task_id: id.clone(),
                        after_task_id: i.checked_sub(1).map(|p| order[p].clone()),
                        context_type: context_type.into(),
                        context_id: context_id.clone(),
                    })
                    .collect()
            };
            let undo = actions(&original);
            for action in actions(&displayed) {
                g.dispatch(action);
            }
            let undo = g.push_undo(undo);
            Outcome {
                changed: true,
                message: None,
                undo: Some(undo),
                sync_now: false,
            }
        })
    }
    /// Ctrl+Up / Ctrl+Down: move a task one place among the open tasks of the view.
    pub fn nudge(&self, id: String, delta: i32, view: View) -> Outcome {
        self.edit(|g| {
            if g.prefs.sort != SortKey::Manual {
                return Outcome {
                    changed: false,
                    message: Some(Message::ManualOrderOnly),
                    undo: None,
                    sync_now: false,
                };
            }
            let Some((context_type, context_id)) = Inner::list_context(&view) else {
                return Outcome::none();
            };
            let group_for = |id: &String| {
                g.task(id).and_then(|t| {
                    listing::task_group(
                        &g.store,
                        &view,
                        &listing::task_row(&g.store, &view, &t, false),
                        g.prefs.group_by,
                    )
                })
            };
            let selected_group = group_for(&id);
            let list: Vec<String> = listing::view_task_ids(&g.store, &view)
                .into_iter()
                .filter(|i| g.task(i).is_some_and(|t| !t.is_done))
                .filter(|i| group_for(i) == selected_group)
                .collect();
            let Some(pos) = list.iter().position(|i| *i == id) else {
                return Outcome::none();
            };
            let delta = if g.prefs.direction == SortDirection::Descending {
                -delta.signum()
            } else {
                delta.signum()
            };
            if delta == 0 {
                return Outcome::none();
            }
            let target = pos as i32 + delta;
            if target < 0 || target >= list.len() as i32 {
                return Outcome::none();
            }
            // Moving down past X means "after X"; moving up before X means "after X's predecessor".
            let after_task_id = if delta > 0 {
                Some(list[target as usize].clone())
            } else if target == 0 {
                None
            } else {
                Some(list[target as usize - 1].clone())
            };
            let context_view = if matches!(view, View::Morning | View::Tonight) {
                View::Today
            } else {
                view
            };
            let full = listing::view_task_ids(&g.store, &context_view);
            let was_after = full
                .iter()
                .position(|i| *i == id)
                .and_then(|p| p.checked_sub(1))
                .map(|p| full[p].clone());
            let undo = g.push_undo(vec![Action::MoveInList {
                task_id: id.clone(),
                after_task_id: was_after,
                context_type: context_type.into(),
                context_id: context_id.clone(),
            }]);
            g.dispatch(Action::MoveInList {
                task_id: id,
                after_task_id,
                context_type: context_type.into(),
                context_id,
            });
            Outcome {
                changed: true,
                message: None,
                undo: Some(undo),
                sync_now: false,
            }
        })
    }

    // ---- archive, undo ----------------------------------------------------

    /// Move every done top-level task (with subtasks) to the young archive.
    pub fn archive_done(&self) -> Outcome {
        self.edit(|g| {
            let (tasks, sub_tasks) = listing::done_tasks(&g.store);
            if tasks.is_empty() {
                return Outcome::none();
            }
            let n = tasks.len() as u32;
            let undo: Vec<Action> = tasks
                .iter()
                .map(|t| Action::RestoreTask {
                    task: t.clone(),
                    sub_tasks: sub_tasks
                        .iter()
                        .filter(|s| s.parent_id.as_deref() == Some(&t.id))
                        .cloned()
                        .collect(),
                })
                .collect();
            g.dispatch(Action::MoveToArchive { tasks, sub_tasks });
            let undo = g.push_undo(undo);
            Outcome {
                changed: true,
                message: Some(Message::Archived { n }),
                undo: Some(undo),
                sync_now: true,
            }
        })
    }
    /// Ctrl+Z: the most recent batch, even after its toast is gone.
    pub fn undo(&self) -> Outcome {
        self.edit(|g| {
            let Some((_, actions)) = g.undo.pop() else {
                return Outcome {
                    changed: false,
                    message: Some(Message::NothingToUndo),
                    undo: None,
                    sync_now: false,
                };
            };
            for a in actions {
                g.dispatch(a);
            }
            changed_with(Message::Undone, None)
        })
    }
    /// A toast's Undo button: that batch, wherever it sits on the stack.
    pub fn undo_batch(&self, id: u64) -> Outcome {
        self.edit(|g| {
            let Some(pos) = g.undo.iter().position(|(i, _)| *i == id) else {
                return Outcome::none();
            };
            let (_, actions) = g.undo.remove(pos);
            for a in actions {
                g.dispatch(a);
            }
            Outcome::changed()
        })
    }

    // ---- projects and tags -------------------------------------------------

    pub fn add_project(&self, title: String) -> Option<String> {
        if title.trim().is_empty() {
            return None;
        }
        let project = Project::new(title.trim());
        let id = project.id.clone();
        self.try_edit(|g| {
            g.dispatch(Action::AddProject { project });
            id
        })
        .ok()
    }
    /// Rename and recolour. `color` is `#rrggbb`; None leaves the colour alone.
    pub fn update_project(&self, id: String, title: String, color: Option<String>) -> Outcome {
        self.edit(|g| {
            let Some(p) = g.store.state.project.entities.get(&id).cloned() else {
                return Outcome::none();
            };
            let mut ch = Map::new();
            if !title.trim().is_empty() && title.trim() != p.title {
                ch.insert("title".into(), json!(title.trim()));
            }
            if let Some(c) = color.filter(|c| p.color() != Some(c.as_str())) {
                let mut theme = p.theme.clone();
                if !theme.is_object() {
                    theme = json!({});
                }
                theme["primary"] = json!(c);
                ch.insert("theme".into(), theme);
            }
            if ch.is_empty() {
                return Outcome::none();
            }
            g.dispatch(Action::UpdateProject { id, changes: ch });
            Outcome::changed()
        })
    }
    /// Delete a project with all its tasks (after the UI confirmed). Inbox stays.
    pub fn delete_project(&self, id: String) -> Outcome {
        if id == INBOX_PROJECT_ID {
            return Outcome::none();
        }
        self.edit(|g| {
            let Some(p) = g.store.state.project.entities.get(&id).cloned() else {
                return Outcome::none();
            };
            let all: Vec<String> = g
                .store
                .state
                .task
                .iter()
                .filter(|t| t.project_id == id)
                .map(|t| t.id.clone())
                .collect();
            g.dispatch(Action::DeleteProject {
                project_id: id,
                note_ids: p.note_ids.clone(),
                all_task_ids: all,
            });
            Outcome::changed()
        })
    }
    pub fn add_tag(&self, title: String) -> Option<String> {
        if title.trim().is_empty() {
            return None;
        }
        self.try_edit(|g| g.ensure_tag(&title)).ok()
    }
    pub fn update_tag(&self, id: String, title: String, color: Option<String>) -> Outcome {
        self.edit(|g| {
            let Some(t) = g.store.state.tag.entities.get(&id).cloned() else {
                return Outcome::none();
            };
            let mut ch = Map::new();
            if !title.trim().is_empty() && title.trim() != t.title {
                ch.insert("title".into(), json!(title.trim()));
            }
            if let Some(c) = color.filter(|c| tag_color(&t).as_deref() != Some(c.as_str())) {
                let mut theme = t.theme.clone();
                if !theme.is_object() {
                    theme = json!({});
                }
                theme["primary"] = json!(c);
                ch.insert("theme".into(), theme);
                ch.insert("color".into(), json!(c));
            }
            if ch.is_empty() {
                return Outcome::none();
            }
            g.dispatch(Action::UpdateTag { id, changes: ch });
            Outcome::changed()
        })
    }
    /// Delete a tag; tasks keep their other tags.
    pub fn delete_tag(&self, id: String) -> Outcome {
        self.edit(|g| {
            if id == TODAY_TAG_ID || !g.store.state.tag.entities.contains_key(&id) {
                return Outcome::none();
            }
            g.dispatch(Action::DeleteTag { id });
            Outcome::changed()
        })
    }

    // ---- repeats ---------------------------------------------------------

    /// The schedule editor's starting point: the task's schedule, or upstream's default
    /// (weekly Monday to Friday from the task's day) for a new one.
    pub fn repeat_draft(&self, task_id: String) -> Option<RepeatDraft> {
        let g = self.lock();
        let t = g.task(&task_id)?;
        let existing = t
            .repeat_cfg_id
            .as_ref()
            .and_then(|id| g.store.state.task_repeat_cfg.entities.get(id).cloned());
        Some(match existing {
            Some(c) => cfg_to_draft(&c, true),
            None => cfg_to_draft(&RepeatCfg::for_task(&t), false),
        })
    }
    /// Live preview of what a draft means.
    pub fn describe_repeat_draft(&self, draft: RepeatDraft) -> RepeatDescription {
        describe_repeat(&draft_to_cfg(&RepeatCfg::default(), &draft))
    }
    /// Save the editor: creates the config linked to the task, or updates the existing one.
    pub fn save_repeat(&self, task_id: String, draft: RepeatDraft) -> Outcome {
        if draft.cycle == RepeatCycle::Weekly && !draft.weekdays.iter().any(|d| *d) {
            return Outcome {
                changed: false,
                message: Some(Message::PickAWeekday),
                undo: None,
                sync_now: false,
            };
        }
        self.edit(|g| {
            let Some(t) = g.task(&task_id) else {
                return Outcome::none();
            };
            let existing = t
                .repeat_cfg_id
                .as_ref()
                .and_then(|id| g.store.state.task_repeat_cfg.entities.get(id).cloned());
            let cfg = draft_to_cfg(&existing.clone().unwrap_or_else(|| RepeatCfg::for_task(&t)), &draft);
            let description = describe_repeat(&cfg);
            if existing.is_none() {
                g.dispatch(Action::AddRepeatCfg { task_id, cfg });
            } else {
                let mut changes = serde_json::to_value(&cfg)
                    .ok()
                    .and_then(|v| v.as_object().cloned())
                    .unwrap_or_default();
                // Fields serde skips when None must still be cleared on the other side.
                for key in ["monthlyWeekOfMonth", "monthlyWeekday"] {
                    changes.entry(key).or_insert(Value::Null);
                }
                g.dispatch(Action::UpdateRepeatCfg { id: cfg.id, changes });
            }
            changed_with(Message::RepeatSaved { description }, None)
        })
    }
    pub fn stop_repeat(&self, task_id: String) -> Outcome {
        self.edit(|g| {
            let Some(t) = g.task(&task_id) else {
                return Outcome::none();
            };
            let Some(id) = t.repeat_cfg_id.clone() else {
                return Outcome::none();
            };
            g.dispatch(Action::DeleteRepeatCfg { id });
            changed_with(Message::NoLongerRepeats { title: t.title }, None)
        })
    }
    /// Create today's instances of repeating tasks (at startup, after sync, on day change).
    pub fn spawn_repeats(&self) -> u32 {
        self.try_edit(|g| g.spawn_repeats()).unwrap_or_else(|error| {
            eprintln!("repeat creation failed: {error}");
            0
        })
    }

    // ---- reminders, day change, summaries -------------------------------------

    pub fn due_reminders(&self) -> Vec<ReminderDue> {
        self.lock().due_reminders()
    }
    /// Push a reminder forward by `minutes` and let it fire again.
    pub fn snooze(&self, id: String, minutes: u32) -> Outcome {
        self.edit(|g| {
            if g.task(&id).is_none_or(|task| task.is_done) {
                return Outcome::none();
            }
            g.notified.remove(&id);
            g.update_task(
                &id,
                [("remindAt".to_string(), json!(now_ms() + minutes as u64 * 60_000))]
                    .into_iter()
                    .collect(),
            );
            changed_with(Message::Snoozed { minutes }, None)
        })
    }
    /// True once per calendar day: the caller then spawns repeats and refreshes.
    pub fn day_changed(&self) -> bool {
        let mut g = self.lock();
        let today = today_str();
        if g.last_day == today {
            return false;
        }
        g.last_day = today;
        true
    }
    /// Opt-in daily summary, at or after the selected local time while the app runs.
    pub fn morning_summary(&self) -> Option<MorningSummary> {
        let (hour, minute) = time_of_ms(now_ms());
        self.morning_summary_at(ClockTime { hour, minute })
    }

    // ---- sync, backup, external changes ---------------------------------------

    pub fn sync_status(&self) -> SyncStatus {
        #[cfg(feature = "p2p")]
        let (nearby_running, linked) = {
            let p = self.p2p.lock().unwrap_or_else(|e| e.into_inner());
            match p.as_ref() {
                Some(r) => (true, r.linked_count()),
                None => (false, 0),
            }
        };
        #[cfg(not(feature = "p2p"))]
        let (nearby_running, linked) = (false, 0);
        let g = self.lock();
        SyncStatus {
            syncing: self.syncing.load(Ordering::SeqCst),
            pending_ops: g.store.pending.len() as u32,
            last_nextcloud_ms: g.store.meta.last_nextcloud_ms,
            last_nearby_ms: g.store.meta.last_nearby_ms,
            nearby_running,
            linked_devices: linked,
        }
    }
    /// One Nextcloud cycle, blocking (call it off the main thread). The store is
    /// snapshotted, synced without the lock held, and changes made meanwhile are re-applied
    /// on top and kept pending.
    pub fn sync_nextcloud(&self, settings: NextcloudSettings) -> Result<SyncReport, CoreError> {
        self.sync_nextcloud_cancellable(settings, SyncCancellation::new())
    }

    /// Read-only authenticated WebDAV probe. `timeout_ms` is explicit so native tests
    /// exercise the same Swift → UniFFI → Rust → HTTP path without a production wait.
    pub fn test_nextcloud_connection_cancellable(
        &self,
        settings: NextcloudSettings,
        cancellation: Arc<SyncCancellation>,
        timeout_ms: u64,
    ) -> Result<(), CoreError> {
        if !cancellation.begin() {
            return Err(CoreError::Transient {
                message: "Connection test was cancelled or already used".into(),
            });
        }
        let _operation = SyncOperation(&cancellation);
        let cfg = sp_sync::NextcloudCfg {
            server_url: settings.server_url,
            user_name: settings.user_name,
            password: settings.password,
            folder: settings.folder,
            compress: settings.compress,
            encrypt_key: settings.encryption_password.filter(|key| !key.is_empty()),
        };
        if !cfg.is_complete() {
            return Err(CoreError::NotConfigured);
        }
        sp_sync::probe_guarded(&cfg, std::time::Duration::from_millis(timeout_ms), || {
            !cancellation.is_cancelled()
        })
        .map_err(|error| CoreError::Transient {
            message: error.to_string(),
        })
    }

    /// A single-use handle can cancel queued work before this blocking call starts.
    /// Retain it until the call returns; cancellation does not release the I/O lease.
    pub fn sync_nextcloud_cancellable(
        &self,
        settings: NextcloudSettings,
        cancellation: Arc<SyncCancellation>,
    ) -> Result<SyncReport, CoreError> {
        if !cancellation.begin() {
            return Err(CoreError::Transient {
                message: "Sync operation was cancelled or already used".into(),
            });
        }
        let _operation = SyncOperation(&cancellation);
        let cfg = sp_sync::NextcloudCfg {
            server_url: settings.server_url,
            user_name: settings.user_name,
            password: settings.password,
            folder: settings.folder,
            compress: settings.compress,
            encrypt_key: settings.encryption_password.filter(|k| !k.is_empty()),
        };
        if !cfg.is_complete() {
            return Err(CoreError::NotConfigured);
        }
        // Serialize provider admission with nearby start/stop and restore.
        #[cfg(feature = "p2p")]
        let nearby = self.p2p.lock().unwrap_or_else(|e| e.into_inner());
        #[cfg(feature = "p2p")]
        if nearby.is_some() {
            return Err(CoreError::Busy);
        }
        let (mut snapshot, generation, original_ops) = {
            let g = self.lock();
            // Reserve and capture the generation under one lock: cancellation
            // must not slip between admission and taking the exchange snapshot.
            if self.syncing.swap(true, Ordering::SeqCst) {
                return Err(CoreError::Busy);
            }
            let ids: HashSet<String> = g.store.pending.iter().map(|p| p.op.id.clone()).collect();
            (g.store.clone(), g.sync_generation, ids)
        };
        let _lease = SyncLease(&self.syncing);
        #[cfg(feature = "p2p")]
        drop(nearby);
        let result = sp_sync::exchange_guarded(&cfg, &mut snapshot, sp_sync::DEFAULT_EXCHANGE_TIMEOUT, || {
            !cancellation.is_cancelled() && self.lock().sync_generation == generation
        });
        match result {
            Ok(r) => {
                let mut g = self.lock();
                if g.sync_generation != generation || !cancellation.begin_commit() {
                    return Err(CoreError::Transient {
                        message: "Sync was cancelled or superseded by a data replacement".into(),
                    });
                }
                // Identify concurrent edits by stable operation IDs, never an array offset.
                let live: Vec<_> = g
                    .store
                    .pending
                    .iter()
                    .filter(|p| !original_ops.contains(&p.op.id))
                    .cloned()
                    .collect();
                for p in &live {
                    sp_oplog::apply(&mut snapshot.state, &p.action);
                }
                snapshot.pending.extend(live);
                snapshot.meta.vector_clock =
                    sp_oplog::merge_clocks(&snapshot.meta.vector_clock, &g.store.meta.vector_clock);
                // These local claims can change while the HTTP exchange is in flight.
                snapshot.meta.last_summary_day = g.store.meta.last_summary_day.clone();
                snapshot.meta.last_nearby_ms = g.store.meta.last_nearby_ms;
                snapshot.meta.p2p_bootstrapped = g.store.meta.p2p_bootstrapped;
                snapshot.meta.last_nextcloud_ms = now_ms();
                snapshot.save()?;
                g.store = snapshot;
                g.invalidate();
                g.spawn_repeats();
                Ok(SyncReport {
                    downloaded: r.downloaded,
                    uploaded: r.uploaded,
                    ops_uploaded: r.ops_uploaded as u32,
                })
            }
            Err(e) => {
                use sp_sync::SyncError::*;
                let message = e.to_string();
                Err(
                    if matches!(e, Encrypted | Decrypt(_) | Schema(_) | Version(_) | FreshState) {
                        CoreError::Actionable { message }
                    } else {
                        CoreError::Transient { message }
                    },
                )
            }
        }
    }
    /// Invalidate the current Nextcloud exchange before its next request or commit.
    /// Returns whether an active exchange was signalled, not whether its I/O stopped.
    /// An in-flight HTTP request retains the shared whole-exchange deadline; await
    /// the exchange before admitting another provider. Call off the UI thread.
    pub fn cancel_nextcloud(&self) -> bool {
        let mut g = self.lock();
        if !self.syncing.load(Ordering::SeqCst) {
            return false;
        }
        g.sync_generation = g.sync_generation.wrapping_add(1);
        true
    }

    /// Import a Super Productivity backup: replaces local state, drops pending ops.
    pub fn import_backup(&self, path: String) -> Result<Outcome, CoreError> {
        let data = std::fs::read(Path::new(&path))?;
        let v: Value = serde_json::from_slice(&data).map_err(|e| CoreError::Invalid { message: e.to_string() })?;
        let d = AppData::from_backup(v).map_err(|e| CoreError::Invalid { message: e.to_string() })?;
        // Hold provider admission until replacement commits. A stopped runtime may
        // still have queued events, but its store guard rejects all of them.
        #[cfg(feature = "p2p")]
        let mut nearby = self.p2p.lock().unwrap_or_else(|e| e.into_inner());
        #[cfg(feature = "p2p")]
        self.stop_p2p_locked(&mut nearby);
        let mut g = self.lock();
        g.sync_generation = g.sync_generation.wrapping_add(1);
        g.store.replace_state(d)?;
        g.invalidate();
        g.undo.clear();
        Ok(changed_with(Message::BackupImported, None))
    }
    pub fn export_backup(&self, path: String) -> Result<Outcome, CoreError> {
        let body = {
            let g = self.lock();
            json!({"data": g.store.state, "timestamp": now_ms(), "crossModelVersion": 4.5})
        };
        std::fs::write(Path::new(&path), serde_json::to_vec_pretty(&body).unwrap())?;
        Ok(changed_with(Message::BackupExported, None))
    }
    /// Another process (`mo` without a running app, a second instance) wrote the store:
    /// reload it if it differs. Skipped while a sync is running.
    pub fn reload_from_disk(&self) -> bool {
        if self.syncing.load(Ordering::SeqCst) {
            return false;
        }
        let mut g = self.lock();
        let on_disk = match Store::try_load(g.store.dir().to_path_buf()) {
            Ok(store) => store,
            Err(error) => {
                // File monitors can fire after a failed write; preserve the live owner.
                eprintln!("store reload failed: {error}");
                return false;
            }
        };
        if on_disk.pending.len() != g.store.pending.len() || on_disk.meta.vector_clock != g.store.meta.vector_clock {
            g.store = on_disk;
            g.invalidate();
            return true;
        }
        false
    }
    /// An action handed over by `mo` (or a URL handler) as JSON; `"sync"` is not an action.
    pub fn dispatch_json(&self, payload: String) -> Result<Outcome, CoreError> {
        let action: Action =
            serde_json::from_str(&payload).map_err(|e| CoreError::Invalid { message: e.to_string() })?;
        let completion = match &action {
            Action::UpdateTask { id, changes } if changes.len() == 1 => changes
                .get("isDone")
                .and_then(Value::as_bool)
                .map(|done| (id.clone(), done)),
            _ => None,
        };
        self.try_edit(|g| {
            if let Some((id, done)) = completion {
                if done {
                    g.set_done(id, true)
                } else {
                    g.reopen_tasks(vec![id])
                }
            } else {
                g.dispatch(action);
                Outcome::changed()
            }
        })
        .map_err(|error| CoreError::Io {
            message: format!("could not persist CLI change: {error}"),
        })
    }
    /// Replace the whole state (tests, `MOMENTUM_SCREENSHOT_DONE`): a raw dispatch.
    pub fn dispatch_raw(&self, action_json: String) -> bool {
        match serde_json::from_str::<Action>(&action_json) {
            Ok(a) => self.try_edit(|g| g.dispatch(a)).is_ok(),
            Err(_) => false,
        }
    }
}

// Rust-only conveniences for the GTK app and tests (not exported over FFI).
impl Engine {
    // Injectable wall-clock time for today's eligibility boundary; task membership and
    // the persisted claim still use today's local date.
    pub(crate) fn morning_summary_at(&self, time: ClockTime) -> Option<MorningSummary> {
        let mut g = self.lock();
        let today = today_str();
        let scheduled = g.prefs.morning_summary_time;
        if !g.prefs.morning_summary_enabled
            || scheduled.hour > 23
            || scheduled.minute > 59
            || (time.hour, time.minute) < (scheduled.hour, scheduled.minute)
            || g.store.meta.last_summary_day == today
        {
            return None;
        }
        g.store.meta.last_summary_day = today;
        g.store.save().ok();
        let ids = g.store.state.today_ids();
        let open: Vec<&Task> = ids
            .iter()
            .filter_map(|i| g.store.state.task.entities.get(i))
            .filter(|t| !t.is_done)
            .collect();
        if open.is_empty() {
            return None;
        }
        Some(MorningSummary {
            total: open.len() as u32,
            morning: open
                .iter()
                .filter(|t| listing::slot_of(&g.store, t) == Some(Slot::Morning))
                .count() as u32,
            tonight: open
                .iter()
                .filter(|t| listing::slot_of(&g.store, t) == Some(Slot::Tonight))
                .count() as u32,
        })
    }

    pub fn dispatch(&self, action: Action) {
        if let Err(error) = self.try_edit(|g| g.dispatch(action)) {
            eprintln!("store edit failed: {error}");
        }
    }
    /// Read access to the store for the parts of the GTK app that still talk to it directly.
    pub fn with_store<T>(&self, f: impl FnOnce(&Store) -> T) -> T {
        f(&self.lock().store)
    }
    pub fn with_store_mut<T>(&self, f: impl FnOnce(&mut Store) -> T) -> T {
        let mut g = self.lock();
        let r = f(&mut g.store);
        g.invalidate();
        r
    }
    pub fn push_undo(&self, actions: Vec<Action>) -> u64 {
        self.lock().push_undo(actions)
    }
    pub fn notified_contains(&self, id: &str) -> bool {
        self.lock().notified.contains(id)
    }
    pub fn slot_tag_id(&self, slot: Slot) -> Option<String> {
        listing::slot_tag_id(&self.lock().store, slot)
    }
    pub fn ensure_slot_tag(&self, slot: Slot) -> String {
        self.lock().ensure_slot_tag(slot)
    }
    pub fn ensure_tag(&self, name: &str) -> String {
        self.lock().ensure_tag(name)
    }
    pub fn project_for(&self, view: &View) -> String {
        self.lock().project_for(view)
    }
    pub fn slot_of(&self, id: &str) -> Option<Slot> {
        let g = self.lock();
        g.task(id).and_then(|t| listing::slot_of(&g.store, &t))
    }
    pub fn is_syncing(&self) -> bool {
        self.syncing.load(Ordering::SeqCst)
    }
}

fn save_failure(error: CoreError) -> Outcome {
    Outcome {
        message: Some(Message::SaveFailed {
            error: error.to_string(),
        }),
        ..Outcome::none()
    }
}
