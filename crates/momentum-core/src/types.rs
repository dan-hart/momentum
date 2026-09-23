// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Data the user interfaces consume. Everything here is plain data with no toolkit or
//! locale in it: labels are structured (a `DayLabel`, a `Message`) and each UI renders
//! them in its own language and date format. With the `ffi` feature every type also
//! carries the UniFFI attributes that generate the Swift bindings.

/// One screen of the app. Projects and tags are identified by their entity id.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum View {
    Today,
    Morning,
    Tonight,
    Upcoming,
    Archive,
    Search,
    Project { id: String },
    Tag { id: String },
}

impl View {
    pub fn project(id: &str) -> Self {
        View::Project { id: id.to_string() }
    }
    pub fn tag(id: &str) -> Self {
        View::Tag { id: id.to_string() }
    }
    /// Views whose quick-add plans the task for today.
    pub fn is_day(&self) -> bool {
        matches!(self, View::Today | View::Morning | View::Tonight)
    }
}

/// Which unfinished top-level tasks a desktop surface counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum TaskCountMode {
    DueToday,
    TodayIncludingOverdue,
    None,
}

/// A part of the day inside Today: tasks tagged "Morning" or "Evening".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum Slot {
    Morning,
    Tonight,
}

impl Slot {
    /// The tag (matched case-insensitively) that puts a task in this slot.
    pub fn tag_name(self) -> &'static str {
        match self {
            Slot::Morning => "Morning",
            Slot::Tonight => "Evening",
        }
    }
    pub fn other(self) -> Slot {
        match self {
            Slot::Morning => Slot::Tonight,
            Slot::Tonight => Slot::Morning,
        }
    }
    pub fn view(self) -> View {
        match self {
            Slot::Morning => View::Morning,
            Slot::Tonight => View::Tonight,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum SortKey {
    Manual,
    Title,
    Due,
    Estimate,
    Created,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum SortDirection {
    Ascending,
    Descending,
}

/// A local presentation choice; never changes task data or sync operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum GroupBy {
    #[default]
    MorningNight,
    None,
    Project,
    Tag,
    Estimate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum EstimateRange {
    UpTo15Minutes,
    UpTo30Minutes,
    UpTo60Minutes,
    UpTo2Hours,
    Over2Hours,
    NoEstimate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum TaskGroup {
    Today,
    Morning,
    Evening,
    Project {
        id: String,
        title: String,
        color: Option<String>,
    },
    Tag {
        id: String,
        title: String,
        color: Option<String>,
    },
    Estimate {
        range: EstimateRange,
    },
    NoProject,
    Untagged,
}

/// The preferences that change what the core computes. The UI owns their persistence
/// (GSettings, UserDefaults) and hands them over whenever they change.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Preferences {
    pub group_by: GroupBy,
    pub sort: SortKey,
    pub direction: SortDirection,
    /// Days shown in Coming Up: 7 or 30.
    pub upcoming_days: u32,
    /// Completing a top-level task moves it to the archive at once.
    pub auto_archive: bool,
    /// Local opt-in, independent of task reminders.
    pub morning_summary_enabled: bool,
    /// Local wall-clock time; hours 0–23, minutes 0–59.
    pub morning_summary_time: ClockTime,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            group_by: GroupBy::MorningNight,
            sort: SortKey::Manual,
            direction: SortDirection::Ascending,
            upcoming_days: 7,
            auto_archive: false,
            morning_summary_enabled: false,
            morning_summary_time: ClockTime { hour: 8, minute: 0 },
        }
    }
}

/// How a day relates to today, so the UI can say "Tomorrow" or "Friday" in its locale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum DayRelation {
    Today,
    Tomorrow,
    Yesterday,
    /// Two to six days ahead: show the weekday name.
    ThisWeek,
    /// Same year: "14 October".
    ThisYear,
    /// Another year: "3 January 2027".
    Other,
}

/// A calendar day with its relation to today. `day` is `YYYY-MM-DD` local time.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct DayLabel {
    pub day: String,
    pub relation: DayRelation,
    /// 0 = Sunday … 6 = Saturday, for the `ThisWeek` case.
    pub weekday: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct ClockTime {
    pub hour: u32,
    pub minute: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct ProjectRef {
    pub id: String,
    pub title: String,
    /// `#rrggbb` when the project has a usable theme colour.
    pub color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct TagRef {
    pub id: String,
    pub title: String,
    pub color: Option<String>,
}

/// Which day of the month a monthly schedule fires on.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum MonthlyRule {
    /// The start date's day of month, clamped to the month's length.
    DayOfMonth {
        day: u32,
    },
    /// The start date has no parsable day: "the same day".
    SameDay,
    LastDay,
    /// `week` 1–4 or -1 for the last; `weekday` 0 = Sunday.
    NthWeekday {
        week: i32,
        weekday: u32,
    },
}

/// A repeat schedule in plain terms; the UI turns it into "Repeats every Monday".
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum RepeatDescription {
    Daily,
    EveryNDays {
        n: u32,
    },
    /// Weekly on exactly one weekday, every week.
    EveryWeekday {
        weekday: u32,
    },
    /// Weekly on several weekdays (0 = Sunday), every `n` weeks.
    Weekly {
        n: u32,
        weekdays: Vec<u32>,
    },
    Monthly {
        n: u32,
        rule: MonthlyRule,
    },
    /// Yearly on a month (1–12) and day.
    Yearly {
        n: u32,
        month: u32,
        day: u32,
    },
    /// Unknown cycle string in the data.
    Repeats,
}

/// A task as one list row shows it. Everything is precomputed so a row renders from
/// this record alone; the rules for what to show (project outside its view, the day
/// unless the view already says it) are applied here, not in the UI.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct TaskRow {
    pub id: String,
    pub title: String,
    pub is_done: bool,
    pub is_subtask: bool,
    /// Read-only row from the archive: no checkbox, drag or menu.
    pub archived: bool,
    /// The project, unless the row is inside that project's own view.
    pub project: Option<ProjectRef>,
    pub estimate_ms: f64,
    /// The day to show next to the title, unless the view already says which day it is.
    pub day: Option<DayLabel>,
    pub time: Option<ClockTime>,
    /// The raw planned day (`dueDay`, or the local day of `dueWithTime`).
    pub due_day: Option<String>,
    pub repeat: Option<RepeatDescription>,
    pub tags: Vec<TagRef>,
    /// First non-empty line of the notes, at most 80 characters, with "…" when cut.
    pub notes_preview: Option<String>,
    pub reminder: Option<ClockTime>,
    /// For archived rows: the day the task was completed.
    pub done_day: Option<DayLabel>,
    pub sub_task_ids: Vec<String>,
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum Row {
    Task { row: TaskRow },
    Project { item: ProjectRef },
    Tag { item: TagRef },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum SectionKind {
    /// The only section of a view without groups.
    Plain,
    Overdue,
    Morning,
    Today,
    Tonight,
    /// One day in Coming Up.
    Day {
        label: DayLabel,
    },
    Completed,
    SearchTasks,
    SearchProjects,
    SearchTags,
    SearchArchived,
}

/// A note under a section's rows ("Showing 60 of 140. Add another word to narrow it down.").
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct SectionNote {
    pub shown: u32,
    pub total: u32,
    /// Suggest narrowing the query (search results) rather than paging (the archive).
    pub suggest_narrowing: bool,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Section {
    /// Optional subgroup within the existing date/completion/search section.
    pub group: Option<TaskGroup>,
    pub kind: SectionKind,
    /// Number of top-level items, shown as "Overdue (3)" for the kinds that carry a count.
    pub count: u32,
    pub rows: Vec<Row>,
    pub note: Option<SectionNote>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum ViewTitle {
    Today,
    Morning,
    Tonight,
    ComingUp,
    Archive,
    Search,
    /// A project's or tag's own name.
    Named {
        name: String,
    },
}

/// What an empty view says. The UI supplies the wording, the icon and the hint that
/// names the configured modifier and whether sync is set up.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum EmptyState {
    Today,
    Morning,
    Tonight,
    Upcoming { days: u32 },
    Archive,
    Search,
    NoResults,
    Project { name: String },
    Tag { name: String },
}

/// Shown above Completed when a view has done tasks but nothing open.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum AllDone {
    Today { completed: u32 },
    Morning,
    Tonight,
    Context,
}

/// A rendered view: the sections and rows of the task list plus what to show when the
/// list is empty.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Listing {
    pub view: View,
    pub title: ViewTitle,
    pub sections: Vec<Section>,
    pub empty: Option<EmptyState>,
    pub all_done: Option<AllDone>,
    /// Archive: how many older tasks a "Show More" would reveal.
    pub more_available: u32,
    /// Something can be archived (drives the Archive Completed menu item).
    pub can_archive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct SidebarEntry {
    pub view: View,
    /// A project's or tag's name; built-in views are named by the UI.
    pub title: String,
    pub color: Option<String>,
    /// Unique unfinished live root-task families assigned to this context.
    pub task_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Sidebar {
    /// Today, then Morning and Tonight only on days that use them, Coming Up, Archive, Search.
    pub fixed: Vec<SidebarEntry>,
    pub projects: Vec<SidebarEntry>,
    pub tags: Vec<SidebarEntry>,
}

/// The facts a context menu needs to offer the right verbs for a task.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct TaskMenuInfo {
    pub id: String,
    pub title: String,
    pub is_done: bool,
    pub planned_today: bool,
    pub slot: Option<Slot>,
    pub top_level: bool,
    pub repeats: bool,
}

/// One suggestion in the `#` autocomplete popover.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct TagCompletion {
    pub title: String,
    pub color: Option<String>,
    pub task_count: u32,
}

/// The `#word` under the cursor in the quick-add box.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct HashWord {
    /// Character offset where the `#` sits.
    pub start: u32,
    /// The typed prefix after `#`.
    pub prefix: String,
}

/// The text of the quick-add box after accepting a completion, and where the cursor goes.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct CompletedText {
    pub text: String,
    pub cursor: u32,
}

/// Everything the task dialog shows and edits.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct TaskDetail {
    pub id: String,
    pub title: String,
    pub notes: String,
    pub project_id: String,
    pub is_done: bool,
    pub parent_id: Option<String>,
    /// The planned day, whether it came from `dueDay` or `dueWithTime`.
    pub due_day: Option<String>,
    pub time: Option<ClockTime>,
    /// Minutes before the scheduled time; `None` when no reminder is set.
    pub reminder_minutes_before: Option<u32>,
    pub estimate_ms: f64,
    pub tag_ids: Vec<String>,
    pub sub_tasks: Vec<TaskRow>,
    pub repeat: Option<RepeatDescription>,
}

/// What the task dialog hands back. Used both to create and to save.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct TaskDraft {
    pub title: String,
    pub project_id: String,
    pub due_day: Option<String>,
    /// A time pins the task to a moment on `due_day` (today when no day was picked).
    pub time: Option<ClockTime>,
    pub reminder_minutes_before: Option<u32>,
    pub estimate_ms: f64,
    pub notes: String,
    /// Existing tags toggled on.
    pub tag_ids: Vec<String>,
    /// New tag names to create, or existing names to reuse (matched case-insensitively).
    pub new_tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum RepeatCycle {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

/// The repeat editor's model: what the dialog edits and previews.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct RepeatDraft {
    pub cycle: RepeatCycle,
    pub every: u32,
    /// Sunday first, seven entries.
    pub weekdays: Vec<bool>,
    pub monthly: MonthlyRule,
    pub start_date: Option<String>,
    pub paused: bool,
    /// The task already has a schedule (Save and Stop Repeating instead of Repeat).
    pub existing: bool,
}

/// A structured toast. The UI owns the wording so both apps localise it their own way.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum Message {
    SaveFailed { error: String },
    TaskCompleted,
    TaskCompletedArchived,
    TasksCompleted { n: u32 },
    TasksCompletedArchived { n: u32 },
    TaskDeleted,
    TasksDeleted { n: u32 },
    TaskAdded,
    TasksAdded { n: u32 },
    TaskDuplicated,
    MovedToTonight,
    MovedToMorning,
    MovedToToday,
    TasksMovedToTonight { n: u32 },
    TasksMovedToMorning { n: u32 },
    TasksMovedToToday { n: u32 },
    MovedToTomorrow,
    MovedToNextWeek,
    TasksMovedToTomorrow { n: u32 },
    TasksMovedToNextWeek { n: u32 },
    PlannedForToday,
    PlannedForMorning,
    PlannedForTonight,
    RemovedFromToday,
    TasksPlannedForToday { n: u32 },
    MovedToProject { name: String },
    Tagged { name: String },
    TasksTagged { n: u32 },
    Archived { n: u32 },
    Snoozed { minutes: u32 },
    Undone,
    NothingToUndo,
    ManualOrderOnly,
    PickAWeekday,
    NoLongerRepeats { title: String },
    RepeatSaved { description: RepeatDescription },
    BackupImported,
    BackupExported,
    Synced { ops_uploaded: u32 },
    LinkedWith { name: String },
    Linking,
    LinkFailed,
    LinkDeclined,
    IdentityChanged { name: String },
    NearbyStartFailed { error: String },
}

/// What a mutation did: whether anything changed, what to tell the user, and the undo
/// batch to attach to that toast.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Outcome {
    pub changed: bool,
    pub message: Option<Message>,
    /// Id of the undo batch this change pushed, for a toast's Undo button.
    pub undo: Option<u64>,
    /// The change should be uploaded right away (archiving does, as upstream does).
    pub sync_now: bool,
}

impl Outcome {
    pub fn none() -> Self {
        Self {
            changed: false,
            message: None,
            undo: None,
            sync_now: false,
        }
    }
    pub fn changed() -> Self {
        Self {
            changed: true,
            message: None,
            undo: None,
            sync_now: false,
        }
    }
}

/// The result and identity of a single task creation, captured in the same operation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct TaskCreation {
    pub outcome: Outcome,
    /// None when the draft was empty and no task was created.
    pub id: Option<String>,
}

/// A reminder that is due now, to be shown as a notification.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct ReminderDue {
    pub task_id: String,
    pub title: String,
    /// The scheduled time, for "Due at 15:30".
    pub time: Option<ClockTime>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct MorningSummary {
    pub total: u32,
    pub morning: u32,
    pub tonight: u32,
}

/// A task as the desktop search (GNOME Shell, KRunner, Spotlight) lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct TaskBrief {
    pub id: String,
    pub title: String,
    pub project: Option<String>,
    pub due_day: Option<DayLabel>,
    pub is_done: bool,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct NextcloudSettings {
    pub server_url: String,
    pub user_name: String,
    pub password: String,
    pub folder: String,
    pub compress: bool,
    pub encryption_password: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct SyncReport {
    pub downloaded: bool,
    pub uploaded: bool,
    pub ops_uploaded: u32,
}

/// Sync status for the caption under the list.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct SyncStatus {
    pub syncing: bool,
    pub pending_ops: u32,
    /// Epoch ms of the last successful Nextcloud sync, 0 when never.
    pub last_nextcloud_ms: u64,
    /// Epoch ms of the last exchange with a nearby device, 0 when never.
    pub last_nearby_ms: u64,
    pub nearby_running: bool,
    pub linked_devices: u32,
}

#[derive(Debug, Clone, thiserror::Error)]
#[cfg_attr(feature = "ffi", derive(uniffi::Error))]
pub enum CoreError {
    /// Sync is off or missing a server, user name, password or folder.
    #[error("Nextcloud sync is not configured")]
    NotConfigured,
    /// A sync problem the user can fix in Preferences: wrong password, unsupported file.
    #[error("{message}")]
    Actionable { message: String },
    /// A passing problem: network, server error, conflict after retries.
    #[error("{message}")]
    Transient { message: String },
    #[error("another sync is already running")]
    Busy,
    #[error("{message}")]
    Io { message: String },
    #[error("{message}")]
    Invalid { message: String },
    #[error("{message}")]
    Nearby { message: String },
}

impl From<std::io::Error> for CoreError {
    fn from(e: std::io::Error) -> Self {
        CoreError::Io { message: e.to_string() }
    }
}
