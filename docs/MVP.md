# Momentum MVP

This document records everything Momentum does as of version 0.1.1 (see the version
notes at the end), written
so that the same product can be rebuilt on another platform. It is the spec, not the
history. If you start an iOS or macOS Momentum, this is the file to read first.

## 1. What Momentum is

Momentum is a daily task planner with one design rule: **feel native, first**. On Linux
that means GTK 4 and libadwaita, the GNOME Human Interface Guidelines, the system accent
color, portals, undo instead of confirmation dialogs, and no custom widgets. On another
platform it would mean the equivalent native toolkit and its guidelines, not a port of this
UI.

Momentum uses [Super Productivity](https://super-productivity.com)'s open data model and
sync format as its foundation. That was a deliberate choice: an established schema, an
existing sync protocol with conflict handling, and a family of clients on other platforms
that Momentum can sit next to. Momentum is not a clone of Super Productivity's interface
and does not aim for feature parity; it picks the planning features and does them well.

Everything below is implemented and working in the Linux app.

## 2. Feature inventory

### 2.1 Views

| View | Contents | Notes |
|---|---|---|
| **Today** | Top-level tasks due today, in the stored Today order, then any other task due today | Open tasks in the main list; completed tasks move to a "Completed (N)" section at the bottom (this applies to every list view). The Today membership follows upstream's virtual TODAY tag rule: `dueWithTime` if set, else `dueDay == today`. |
| **Tonight** | Today's tasks that carry the "Evening" tag (case-insensitive) | In the Today view these tasks are split into a second "Tonight" section under the day's tasks. Quick-add and the dialog from this view add today's date and the Evening tag, creating the tag if needed. Dropping a task here does the same. |
| **Coming Up** | Open top-level tasks due in the next 7 days (default) or 30 days | Grouped into one section per day with a relative heading (Tomorrow, Friday, 14 October). No day label on rows. |
| **Archive** | Archived tasks from both archive tiers, newest completion first | Read-only rows: no checkbox, no drag, no menu. Paged 100 at a time with a Show More button. |
| **Search** | Live results across everything | See 2.7. |
| **Project** | The project's task list in its stored order | One per non-archived, non-hidden project. Subtasks appear indented under their parent. |
| **Tag** | Tasks carrying the tag, in the tag's stored order | One per tag except the virtual TODAY tag. |

Lists are rendered as **sections**: each group (a day in Coming Up, a result type in
Search, Today and Tonight in the Today view) is its own inset boxed list with its heading
above the card and empty space between sections, the same structure libadwaita's preference
groups use. Views without groups are a single card.

The sidebar can be hidden with a toggle at the start of its header bar (F9 does the same);
while hidden, the same toggle appears at the start of the content header to bring it back.
On narrow windows the sidebar overlays the content and closes when you pick a view.

Sidebar order: Today, Tonight, Coming Up, Archive, Search, then a collapsible **Projects** section
and a collapsible **Tags** section. Collapsed state persists. Section headers show a
chevron and toggle on click, tap or Enter. If the current view is inside a collapsed
section, the view stays and nothing is selected.

Every task row shows: a done checkbox, the title, and a subtitle made of (in order) the
project name with a color dot (outside project views), an estimate like `~1h 30m`, a
relative due day (outside Today and Coming Up), a repeat description, and `#tag` names in
the tag's color. Repeating tasks also get a repeat badge with the description as tooltip.
Done rows are dimmed.

### 2.2 Creating tasks

- **Quick-add box** at the top of every view except Search. Enter creates. Short syntax:
  `#tag` adds an existing tag or creates it; a trailing `1h 30m`, `45m` or `2h` sets the
  estimate. Typing `#` opens an autocomplete popover anchored under the cursor listing
  matching tags with icon, color and task count; Up/Down select, Enter or Tab inserts,
  Escape closes; it closes when the box loses focus.
- **New Task dialog**, opened by the sidebar's Create Task button, Ctrl+N, or the
  system-wide shortcut. Header has Cancel and Create; Create is disabled until there is a
  title; Enter in the title creates. Fields: Title, Project (dropdown, preselected to the
  current view's project, color-coded), Due (calendar popover with Today, Tomorrow, None),
  Estimate, existing tags as toggle chips plus a "new tags, comma separated" field, Notes.
  Opening it from Today preselects today; from a tag view, that tag is added.
- Tasks created in a project view go to that project; elsewhere to the first project
  (Inbox on a fresh install).

### 2.3 Editing tasks

- Tapping a row (or Enter on it) opens the same form as an edit dialog; changes are applied
  when it closes. The Notes group has a copy button in its header that puts the whole
  note text on the clipboard; the editor is sized for paragraphs. Rows with notes show a
  document badge on the right whose tooltip is the first line of the note. It adds an "Add subtask" field (top-level tasks only) and a Delete button.
- Done/undone via the checkbox, Ctrl+D on the focused row, or the context menu. Marking
  done sets `doneOn`; undone clears it.
- Subtasks live under their parent, inherit its project, and move with it.

### 2.4 Organising

- **Drag and drop.** A task row is a drag source. Drop targets: a project row (move task
  and subtasks; upstream `moveToOtherProject`), a tag row (add the tag), the Today row
  (plan for today), or another task row (reorder before it, Manual Order only; upstream
  work-context move op). Rows highlight in the accent color while hovered.
- **Sorting** via the header's View Options menu: Manual Order (upstream's list order),
  Title, Due Day, Estimate, Created; plus Ascending or Descending. Applies to open and done
  tasks separately. Persists.
- **Projects and tags** can be created (Ctrl+Shift+N for projects; tags via `#` or the
  dialog), renamed and recolored (Edit dialog with a native color chooser), and deleted
  (confirmation dialog stating how many tasks go with a project; tags leave tasks intact).
  Inbox cannot be deleted.
- **Archive Completed Tasks** (View Options menu, disabled when nothing is done) moves every
  done top-level task and its subtasks into the young archive with upstream's
  `moveToArchive` op, then syncs.

### 2.4b Selection mode and bulk actions

Enter with "Select Tasks" in the header's View Options menu, Ctrl+click on a task, or
Ctrl+A (select all rows in the view). Rows swap their done checkbox for a selection checkbox, the title shows "N
selected", Cancel appears at the start of the header, and a bottom action bar offers Select
All, Mark as Done, Plan for Today, Move to Project…, Add Tag… (existing tag or a new one),
and Delete. Each bulk action gets one Undo toast for the whole batch and leaves selection
mode. Dragging a selected row drags the whole selection (ids newline-separated) onto a
project, tag, Today, Tonight, or another task for reordering. Escape or switching views
leaves selection mode.

### 2.4c Day moves: Today, Tonight, Tomorrow

Three one-keystroke moves, each available on the focused task, on the whole selection in
selection mode, and in the context menu, and each undoable as one batch:

- **Plan for Today / Remove from Today** (Ctrl+T, drop on Today): sets or clears `dueDay`.
- **Move to Tonight / Move to Today** (Ctrl+Shift+T, drop on Tonight): adds or removes the
  "Evening" tag, creating the tag on first use; moving to tonight also plans the task for
  today if it was not. A batch goes the direction of its first task.
- **Move to Tomorrow** (Ctrl+Shift+Right): sets `dueDay` to tomorrow, keeps tags, clears
  `dueWithTime`. Tasks already due tomorrow are skipped.
- **Move to Next Week** (Ctrl+Shift+Down): same, to the next Monday strictly after today
  (on a Monday that is the following Monday).

All three are plain `updateTask` ops (or `planTasksForToday` / `removeTasksFromTodayTag`),
so other clients see exactly the same change.

### 2.5 Repeating tasks

Repeat configurations come from the sync data. At startup, after each sync, and when the
date changes, every config that is due today and has not yet produced today's instance
spawns a task with the deterministic id `rpt_<cfgId>_<YYYY-MM-DD>` (so two clients never
double-create), the config's title, project, tags, estimate and notes, due today; the
config's `lastTaskCreationDay` is then updated. Due rules: DAILY every N days from the
start date; WEEKLY on the flagged weekdays every N weeks; MONTHLY on the start day of month
(clamped), the last day, or the nth weekday (`monthlyWeekOfMonth` 1–4 or -1 with
`monthlyWeekday`); YEARLY on the start date. Only the current day is checked; there is
no catch-up for days when no client ran. Description strings: "Repeats daily", "Repeats
every Monday", "Repeats weekly on Mon, Wed", "Repeats every 2 weeks on Monday", "Repeats
monthly on the 5th", "Repeats monthly on the second Tuesday", "Repeats yearly on 5 March".

### 2.5b Editing the repeat schedule

Opened from the task dialog's Repeat row, the context menu ("Repeat…" / "Edit Repeat…",
top-level tasks only) or Ctrl+Shift+R. The Repeat dialog shows a live description of the
schedule and offers: Repeats (Daily, Weekly, Monthly, Yearly), Every N (unit follows the
cycle), weekday chips (weekly, at least one required), "On the last day of the month"
(monthly: "Monthly on" = the same date, the last day, or a weekday of the month with
First–Fourth/Last and a weekday, i.e. upstream's `monthlyWeekOfMonth`/`monthlyWeekday`),
Starts (calendar), Paused. Cancel / Repeat (or Save) in the header; existing
schedules also get a destructive Stop Repeating button.

A new schedule is created with upstream's defaults: weekly on Monday to Friday, start day
= the task's due day or today, title/project/tags/estimate/notes copied from the task,
`lastTaskCreationDay` = today so the current task counts as today's instance. Ops:
`[TaskRepeatCfg][Task] Add TaskRepeatCfg to Task` (`{taskId, taskRepeatCfg}`, links the
task), `[TaskRepeatCfg] Update TaskRepeatCfg`, and `[Task Shared] deleteTaskRepeatCfg`
(`{taskRepeatCfgId}`, clears `repeatCfgId` on tasks).

### 2.6 Undo, feedback, errors

- Undo toasts, not confirmation dialogs, for: delete task, mark done, archive, plan for
  today, remove from today, move to project, add tag by drop.
- Confirmation dialogs only for deleting a project or tag.
- Toasts for sync outcomes; a persistent **banner** with a Preferences button for sync
  problems the user can fix (wrong or missing encryption password, unsupported file
  version or newer schema, sync not configured, fresh device with no data set).
- Sync progress: the caption under the list says "Syncing…" immediately and the header's
  sync button is disabled; a spinner replaces its icon only if the sync passes one second.
  The content header bar carries exactly two controls, Sync Now and View Options (plus
  Cancel while selecting), which is the HIG's "small number of controls" for a header bar.
  Sync Now is also in the primary menu and on Ctrl+R. The button is hidden while sync is
  off. Afterwards "Last synced just now" for the first ten seconds, then "N seconds /
  minutes / hours / days ago", refreshed every 30 seconds. The caption is hidden on
  empty views and when sync is off.
- Reminders (`remindAt`) fire desktop notifications, checked every 30 seconds.

### 2.6b Empty states and "all done"

Every view has its own empty state (icon, title, description) that says what to do next.
The description ends with the sync hint "turn on sync in Preferences to bring in your
tasks" when sync is not configured, or "press Ctrl+N" when it is.

| View | Icon | Title | Hint |
|---|---|---|---|
| Today | star | Nothing planned for today | Add a task above; drag tasks here from Coming Up; Ctrl+T on any task |
| Tonight | moon | Nothing planned for tonight | Tag a task "Evening", or Ctrl+Shift+T on a task |
| Coming Up | calendar | Nothing coming up | Tasks due in the next 7 (or 30) days appear here; set a due day in a task's details |
| Archive | archive box | No archived tasks | Completed tasks land here when archived with Ctrl+E |
| Search | magnifier | Search Everything | Tasks, notes, subtasks, projects, tags and the archive |
| Project | folder | No tasks in <project> | Add a task above; drag tasks here from any other view |
| Tag | tag | No tasks tagged #<tag> | Add a task above; drag tasks here to tag them |

**All done.** When a view has completed tasks but nothing open, the list shows a panel
above the Completed section instead of an empty state: a large checkmark, a title, a line
of copy, and a suggested-action Archive Completed button (same action as Ctrl+E, which
archives and then syncs). Copy by view:

- Today: "All done for today" / "You completed N tasks. Time to switch off."
- Tonight: "All done for tonight" / "Enjoy the rest of your evening."
- Project or tag: "All caught up" / "Every task here is complete."

The panel is a plain vertical box, not a status page, because a status page collapses
when placed inside the scrolling list column. After archiving, the view falls back to its
normal empty state.

### 2.7 Search

Own screen with a search field replacing the quick-add box. Live results with a 120 ms
debounce, backed by a lowercase index rebuilt only when data changes. Every word of the
query must match somewhere in a task's title, notes, tag names or project name. Groups
with counts: Tasks (open first, then done, alphabetical; capped at 60 with a "showing 60 of
N, add another word" note), Projects, Tags (both navigate on activation), Archived (capped
at 30). Empty prompt "Search Everything"; no-results state "No Results Found".

### 2.8 Keyboard and input

| Key | Action |
|---|---|
| Ctrl+N | New Task dialog |
| Ctrl+Shift+N | New project |
| Ctrl+F | Search screen, field focused |
| Ctrl+D | Toggle done on focused task |
| Ctrl+T | Plan focused task for today |
| Ctrl+Shift+T | Move focused task (or selection) between Today and Tonight |
| Ctrl+Shift+Right | Move focused task (or selection) to tomorrow |
| Ctrl+Shift+Down | Move focused task (or selection) to next week (the next Monday) |
| Ctrl+Shift+R | Repeat schedule for the focused task |
| Ctrl+M | Move focused task to a project (dialog) |
| Delete | Delete focused task |
| Enter | Open focused task |
| Alt+1…9 | Jump to the n-th sidebar entry |
| Ctrl+R, F5 | Sync now |
| Ctrl+, | Preferences |
| Ctrl+A | Select all (enters selection mode) |
| Ctrl+E | Archive completed tasks |
| Ctrl+Up / Ctrl+Down | Move focused task up / down (Manual Order) |
| Ctrl+Shift+D | Duplicate focused task |
| Ctrl+Shift+C | Copy focused task title |
| Ctrl+O | Open focused task details |
| Ctrl+Shift+A | Clear selection |
| Ctrl+L | Focus the quick-add box |
| F9 | Show or hide the sidebar |
| Ctrl+PageDown / Ctrl+PageUp | Next / previous view |
| Ctrl+? | Shortcuts dialog |
| Ctrl+W, Ctrl+Q | Close, quit |
| Menu, Shift+F10, right-click, long-press | Context menu on a task, project or tag row |
| Ctrl+Alt+T, Ctrl+Alt+M (system-wide) | Add a task from anywhere; show the window |

Context menu contents. Task: Open, Mark as Done/Not Done; a separated group with Plan for Today/Remove from
Today, Move to Tonight/Move to Today, Move to Tomorrow, Move to Next Week, Move to Project…; Repeat… / Edit Repeat… (top-level tasks); then Delete. Project: Open, New Task Here…, Edit…, Delete Project…
(not Inbox). Tag: Open, Edit…, Delete Tag….

### 2.9 Preferences

- Nextcloud Sync: master switch (off by default; nothing leaves the device until on),
  server URL, username, app password, sync folder, encryption password, sync
  automatically (at startup, every five minutes, and 20 seconds after the last local
  change so a burst of edits becomes one upload; archiving syncs at once), compress sync
  file, Sync Now. Rows are
  disabled while the switch is off. Secrets go to the system keyring.
- Appearance: color-code projects and tags (on by default).
- Backup: import and export Super Productivity backup JSON. Import replaces local state and
  discards pending ops.

### 2.10 Appearance

- Follows the system light/dark setting and accent color; nothing hard-codes a color.
- Project and tag colors come from the data (`theme.primary`, tag `color`) and are used for
  sidebar icons, subtitle dots and tag names, dialog chips and dropdown icons, all gated by
  one setting.
- Relative dates everywhere: Today, Tomorrow, Yesterday, weekday within six days, then
  "14 October" or "3 January 2027".
- App icon: orange tile (GNOME palette Orange 2 to 4), white check with three motion lines,
  plus a symbolic variant and a striped development variant.

## 3. Data model (what to reimplement)

Momentum types the slices it needs and preserves everything else opaquely so a round trip
never loses fields.

- `AppData`: `task`, `project`, `tag`, `taskRepeatCfg` as entity states (`ids` +
  `entities`), and every other slice (`globalConfig`, `note`, `boards`, archives, …) as raw
  JSON.
- `Task`: `id, title, notes?, timeEstimate, timeSpent, timeSpentOnDay{day: ms}, isDone,
  doneOn?, projectId, tagIds[], parentId?, subTaskIds[], created, modified?, dueDay?,
  dueWithTime?, remindAt?, repeatCfgId?, attachments[]` + extra fields.
- `Project`: `id, title, taskIds[], backlogTaskIds[], noteIds[], isArchived,
  isHiddenFromMenu, isEnableBacklog, icon?, theme{primary,…}` + extra.
- `Tag`: `id, title, taskIds[], color?, icon?, created, theme` + extra. The `TODAY` tag is
  virtual: never stored in a task's `tagIds`; its `taskIds` is the Today order.
- `RepeatCfg`: `id, projectId?, title?, tagIds[], defaultEstimate?, notes?, isPaused,
  repeatCycle, repeatEvery, startDate?, lastTaskCreationDay?, monthlyLastDay,
  monday…sunday` + extra.
- Days are `YYYY-MM-DD` strings in local time; timestamps are epoch milliseconds.
- Fresh state: an `INBOX_PROJECT` project and a `TODAY` tag with upstream's default themes.

Authoritative field lists: upstream `packages/shared-schema` and `src/app/features/*/
*.model.ts`; Momentum's `crates/sp-model/src/lib.rs`.

## 4. Operations Momentum emits and applies

Every change is an operation: `{id (uuid v7), a (action code), o (CRT/UPD/DEL/MOV), e
(entity type), d/ds (entity ids), p (payload), c (client id), v (vector clock), t
(timestamp ms), s (schema version 4)}`. The client id is `momentum_<random>`; the vector
clock increments the client's own counter on every op.

| Momentum action | Upstream action type | Payload |
|---|---|---|
| AddTask | `[Task Shared] addTask` | `{task, workContextId, workContextType:"PROJECT", isAddToBacklog:false, isAddToBottom, isIgnoreShortSyntax:true}` |
| AddSubTask | `[Task] Add SubTask` | `{task, parentId}` |
| UpdateTask | `[Task Shared] updateTask` | `{task:{id, changes}}` |
| DeleteTask | `[Task Shared] deleteTask` | `{task: {…task, subTasks:[…]}}` |
| MoveToProject | `[Task Shared] moveToOtherProject` | `{task:{…,subTasks}, targetProjectId}` |
| MoveToArchive | `[Task Shared] moveToArchive` | `{tasks:[{…,subTasks}]}` |
| RestoreTask | `[Task Shared] restoreTask` | `{task:{…,subTasks}, subTasks}` |
| PlanForToday | `[Task Shared] planTasksForToday` | `{taskIds, today}` |
| RemoveFromToday | `[Task Shared] removeTasksFromTodayTag` | `{taskIds}` |
| MoveInList | `[WorkContextMeta] Move Task in Today` | `{taskId, afterTaskId, workContextType, workContextId}` |
| AddProject / UpdateProject | `[Project] Add Project` / `Update Project` | `{project}` / `{project:{id,changes}}` |
| DeleteProject | `[Task Shared] deleteProject` | `{projectId, noteIds, allTaskIds, projectDeleteWins:true}` |
| AddTag / UpdateTag / DeleteTag | `[Tag] Add Tag` / `Update Tag` / `Delete Tag` | `{tag}` / `{tag:{id,changes}}` / `{id}` |
| UpdateRepeatCfg | `[TaskRepeatCfg] Update TaskRepeatCfg` | `{taskRepeatCfg:{id,changes}}` |

Local `apply()` semantics mirror upstream's meta-reducers: adding a task appends to the
project list (top or bottom), each tag's list, and the Today list if due today; updates
maintain tag lists, Today membership, project lists, `doneOn` and `modified`; deletes and
archives detach from every list; archives move tasks into `archiveYoung.task`. The full
short-code table (155 action types) is generated from upstream into `action_codes.rs`.

## 5. Sync protocol (Nextcloud, file based)

- Remote path: `<server>/remote.php/dav/files/<user>/<folder>/sync-data.json`, HTTP
  Basic auth with an app password.
- File body: prefix `pf_` + `C` if gzip + `E` if encrypted + `2__`, then the payload.
  Payload = JSON → optional gzip+base64 → optional encryption.
- JSON envelope (version 2): `{version:2, syncVersion, schemaVersion:4, vectorClock,
  lastModified, clientId, state, archiveYoung?, archiveOld?, recentOps[≤2000], oldestOpSyncVersion?}`.
  Momentum keeps the archives in `state.rest` locally and moves them back into the
  envelope on upload.
- Encryption: Argon2id (64 MiB, 3 iterations, 1 lane, 32-byte key, 16-byte random salt)
  then AES-256-GCM with a 12-byte nonce; wire = base64(salt ‖ nonce ‖ ciphertext+tag).
  Legacy files: PBKDF2-HMAC-SHA256, 1000 rounds, salt = password, base64(nonce ‖ ct).
- When sync runs (with "Sync automatically" on): at startup; every five minutes (to pull
  remote changes); 20 seconds after the last local change, debounced, so a burst of edits
  becomes one upload; immediately after Archive Completed; and on demand (Ctrl+R, header
  button, menus). The debounced sync is skipped when nothing is pending and deferred while
  another sync is running. A port should keep the same three triggers and the same order:
  pull first, then rebase and push.
- Cycle: download (ETag from `OC-ETag` or `ETag`); if the remote `syncVersion` differs
  from the last seen one, take the remote `state` and replay pending local ops onto it,
  merging vector clocks; if there are pending ops, append them to `recentOps` tagged with
  `sv = syncVersion+1`, trim to 2000, write the full state, and `PUT` with `If-Match` (or
  `If-None-Match: *` for a new file); on 412 retry the whole cycle, up to three times.
  Momentum never replays other clients' ops; the remote snapshot already contains them.
- Guards: refuse to create the remote file from a device that has no `globalConfig`
  slice (fresh install); reject the v3 split-file format and schema versions above 4 with
  actionable errors; demo mode never syncs.
- Verified: two-client convergence test against a mock WebDAV server, including the
  encrypted path, in `crates/sp-sync`.

## 6. Local storage

Three JSON files in the app data directory: `state.json` (the `AppData` snapshot),
`pending.json` (unsynced ops, each with its typed action for replay), `meta.json`
(client id, vector clock, last sync version, last ETag). Writes are atomic (temp file +
rename). GSettings hold non-secret preferences; the keyring holds secrets.

## 7. Explicitly out of scope for the MVP

Time tracking and the timer, pomodoro and break reminders, idle detection, boards,
metrics and worklog, issue providers (Jira, GitHub, …), standalone notes, scheduled
times (`dueWithTime`) and reminder editing, attachments, planner day view, the SuperSync
server, Dropbox/OneDrive/WebDAV-generic/LocalFile providers, the v3 split sync format,
repeat catch-up for missed days, translations, and Flathub
publication. All of their data is preserved untouched.

## 7b. Distribution

- **Own Flatpak repository** served from GitHub Pages (`gh-pages` branch, OSTree archive
  mode). A workflow on every `v*` tag builds x86_64 then aarch64 in sequence into the same
  repo, signs commits with a dedicated GPG key held in repository secrets, generates static
  deltas, prunes to three builds, and pushes. Users install with a `.flatpakref` (one click
  in GNOME Software or Discover) or `flatpak remote-add` with the `.flatpakrepo`; updates
  arrive automatically.
- **GitHub Releases** get standalone `.flatpak` bundles for both architectures from a
  second workflow on the same tag.
- **Flathub** files are kept ready (offline manifest pinned to the tag, vendored
  `cargo-sources.json`) but not submitted; Flathub's generative-AI policy applies.
- Release procedure: bump `meson.build`, `Cargo.toml` and the metainfo `<release>`, add a
  changelog entry, point the metainfo screenshot URLs at the new tag, commit, tag, push.

## 8. Porting notes for iOS and macOS

- **Reuse the core.** `sp-model`, `sp-oplog`, `sp-store` and `sp-sync` have no GTK
  dependency. They can be compiled as a Rust library and exposed to Swift via UniFFI or a
  C ABI, or reimplemented in Swift from sections 3 to 6 of this document. Keep the
  conformance tests: round-trip a real backup with zero diff, the op payload shapes, the
  encryption vectors, the two-client convergence test.
- **Platform services to map.** Keyring → Keychain. GSettings → UserDefaults. XDG data
  dir → Application Support. GNotification → UserNotifications. GlobalShortcuts portal →
  none on iOS, `NSEvent` global monitor or Shortcuts on macOS. Background portal →
  BGTaskScheduler. Accent color → `tintColor` / `NSColor.controlAccentColor`.
- **UI to re-express, not port.** Sidebar + content → `NavigationSplitView`. Boxed lists
  → `List` with inset grouped style. Toasts with Undo → a custom overlay or the platform
  undo manager. Context menus → `contextMenu`. Drag and drop → `draggable`/`dropDestination`.
  Search → `searchable`. The New Task sheet → a `Form` in a sheet with Cancel/Create in the
  toolbar. Collapsible sidebar sections → `DisclosureGroup` or `Section` with a header
  toggle.
- **Behaviours that must survive the port.** Enter-to-create with Create disabled on an
  empty title; `#` autocomplete; relative dates; the Today order rule and the Tonight
  split on the "Evening" tag; the three day moves with batch undo; selection mode with
  bulk actions and multi-item drag; undo for every destructive action; the fresh-device
  guard; deterministic repeat instance ids; the search index and result caps; secrets
  never in plain storage; sync off by default.
- **Getting started checklist.** 1) Read sections 3 to 6. 2) Build the model with
  unknown-field preservation. 3) Port `apply()` for the ops in section 4 with tests. 4) Port
  the sync cycle and crypto, test against `build-aux/mock-webdav.py`. 5) Build Today, the
  New Task form and the quick-add box. 6) Add Coming Up, projects, tags, Search, Archive.
  7) Undo, context menus, drag and drop, repeats, keyboard. 8) Preferences and the sync
  switch. 9) Screenshots, metadata, store listing.

## 9. Version notes

- **0.1.0** (2026-09-08): first preview; everything in sections 2 to 6 except the items
  below.
- **0.1.1** (2026-09-09): repeat schedule editor (2.5b) including monthly on the nth
  weekday; per-view empty states and the all-done panel (2.6b); Move to Next Week (2.4c);
  notes copy button, larger editor and notes badge (2.3); sync 20 s after the last local
  change (5); sidebar show/hide toggle (2.1); context-menu Move grouping (2.8); own Flatpak
  repository (7b).
