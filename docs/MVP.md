# Momentum MVP

This document records everything Momentum does as of its first preview (0.1.0), written
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
| **Today** | Top-level tasks due today, in the stored Today order, then any other task due today | Open tasks first, done tasks after. The Today membership follows upstream's virtual TODAY tag rule: `dueWithTime` if set, else `dueDay == today`. |
| **Tonight** | Today's tasks that carry the "Evening" tag (case-insensitive) | In the Today view these tasks are split into a second "Tonight" section under the day's tasks. Quick-add and the dialog from this view add today's date and the Evening tag, creating the tag if needed. Dropping a task here does the same. |
| **Coming Up** | Open top-level tasks due in the next 7 days (default) or 30 days | Grouped into one section per day with a relative heading (Tomorrow, Friday, 14 October). No day label on rows. |
| **Archive** | Archived tasks from both archive tiers, newest completion first | Read-only rows: no checkbox, no drag, no menu. Paged 100 at a time with a Show More button. |
| **Search** | Live results across everything | See 2.7. |
| **Project** | The project's task list in its stored order | One per non-archived, non-hidden project. Subtasks appear indented under their parent. |
| **Tag** | Tasks carrying the tag, in the tag's stored order | One per tag except the virtual TODAY tag. |

Sidebar order: Today, Coming Up, Archive, Search, then a collapsible **Projects** section
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
  when it closes. It adds an "Add subtask" field (top-level tasks only) and a Delete button.
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

### 2.5 Repeating tasks

Repeat configurations come from the sync data. At startup, after each sync, and when the
date changes, every config that is due today and has not yet produced today's instance
spawns a task with the deterministic id `rpt_<cfgId>_<YYYY-MM-DD>` (so two clients never
double-create), the config's title, project, tags, estimate and notes, due today; the
config's `lastTaskCreationDay` is then updated. Due rules: DAILY every N days from the
start date; WEEKLY on the flagged weekdays every N weeks; MONTHLY on the start day of month
(clamped, or last day); YEARLY on the start date. Only the current day is checked; there is
no catch-up for days when no client ran. Description strings: "Repeats daily", "Repeats
every Monday", "Repeats weekly on Mon, Wed", "Repeats every 2 weeks on Monday", "Repeats
monthly on the 5th", "Repeats yearly on 5 March".

### 2.6 Undo, feedback, errors

- Undo toasts, not confirmation dialogs, for: delete task, mark done, archive, plan for
  today, remove from today, move to project, add tag by drop.
- Confirmation dialogs only for deleting a project or tag.
- Toasts for sync outcomes; a persistent **banner** with a Preferences button for sync
  problems the user can fix (wrong or missing encryption password, unsupported file
  version or newer schema, sync not configured, fresh device with no data set).
- Sync progress: the caption under the list says "Syncing…" immediately and the sync
  button is disabled; a spinner replaces the button icon only if the sync passes one
  second. Afterwards "Last synced just now / N minutes ago". The caption is hidden on
  empty views and when sync is off.
- Reminders (`remindAt`) fire desktop notifications, checked every 30 seconds.

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
| Ctrl+M | Move focused task to a project (dialog) |
| Delete | Delete focused task |
| Enter | Open focused task |
| Alt+1…9 | Jump to the n-th sidebar entry |
| Ctrl+R, F5 | Sync now |
| Ctrl+, | Preferences |
| Ctrl+? | Shortcuts dialog |
| Ctrl+W, Ctrl+Q | Close, quit |
| Menu, Shift+F10, right-click, long-press | Context menu on a task, project or tag row |
| Ctrl+Alt+T, Ctrl+Alt+M (system-wide) | Add a task from anywhere; show the window |

Context menu contents. Task: Open, Mark as Done/Not Done, Plan for Today/Remove from
Today, Move to Project…, Delete. Project: Open, New Task Here…, Edit…, Delete Project…
(not Inbox). Tag: Open, Edit…, Delete Tag….

### 2.9 Preferences

- Nextcloud Sync: master switch (off by default; nothing leaves the device until on),
  server URL, username, app password, sync folder, encryption password, sync
  automatically (startup and every five minutes), compress sync file, Sync Now. Rows are
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
repeat catch-up for missed days, editing repeat configurations, translations, and Flathub
publication. All of their data is preserved untouched.

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
  empty title; `#` autocomplete; relative dates; the Today order rule; undo for every
  destructive action; the fresh-device guard; deterministic repeat instance ids; secrets
  never in plain storage; sync off by default.
- **Getting started checklist.** 1) Read sections 3 to 6. 2) Build the model with
  unknown-field preservation. 3) Port `apply()` for the ops in section 4 with tests. 4) Port
  the sync cycle and crypto, test against `build-aux/mock-webdav.py`. 5) Build Today, the
  New Task form and the quick-add box. 6) Add Coming Up, projects, tags, Search, Archive.
  7) Undo, context menus, drag and drop, repeats, keyboard. 8) Preferences and the sync
  switch. 9) Screenshots, metadata, store listing.
