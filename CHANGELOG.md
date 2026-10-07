# Changelog

All notable changes to Momentum are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.4.8] - 2026-10-07

### Added
- iOS: share a link or text to Momentum from any app's share sheet. The sheet shows a
  normalized title to edit and adds the task to Inbox with the link first in its notes when
  Momentum next opens; the app and its extension now share an App Group

## [0.4.7] - 2026-10-06

### Added
- Optional task-completion sound, off by default, in Linux Preferences, macOS Settings
  and iOS Settings. A short bundled chime plays for completion; reopen and undo are silent.

## [0.4.6] - 2026-10-02

### Added
- macOS and iOS: optional one-way import from a selected Apple Reminders list, with
  manual import and off-by-default automatic import while Momentum is active

### Fixed
- Apple Reminders: prevent a crash when EventKit returns reminders on a background queue
- Apple Reminders: retry failed fetches and saves, announce import results to assistive
  technology, and retain newly discovered reminder identities to avoid duplicate imports

### Changed
- README and screenshot notes refreshed: October 2026 previews of all three apps, the
  current platform status, what changed in 0.4, and how sync failures and recovery work
- Linux: `MOMENTUM_SCREENSHOT_SIZE=WIDTHxHEIGHT` sizes a capture, and captures render
  as the active window even when launched from a terminal

## [0.4.5] - 2026-10-01

### Fixed
- macOS: completing the last task of Today (with completed tasks archived immediately)
  could crash the app while the list emptied; the list now stays in place and shows the
  empty state over it, and its sections keep stable identities across updates
- Linux: closing the window no longer aborts the process when it is finalized

## [0.4.4] - 2026-09-30

### Fixed
- Sync: an upload cut off by a slow connection could be stored by the server as the sync
  file and lock every device out. Uploads now carry a checksum and expected length, the
  stored size is checked afterwards, and a partial copy is reported instead of trusted
- Sync: a large sync file over a slow or relayed connection no longer fails at 30 s; a
  foreground exchange may take up to five minutes while stalls still fail fast
- Sync: a damaged server copy is told apart from a wrong encryption password

### Added
- Every platform: sync failures are explained in plain words with what to do next; the
  technical message stays behind Details
- Every platform: Replace Server Copy publishes this device's tasks over a damaged server
  copy (kept as sync-data.json.damaged) after confirmation; `mo sync --replace-remote`
  does the same from the command line
- iOS: the sync failure section shows "Trying again…" and "Last tried" so Try Again is
  visibly acknowledged

## [0.4.3] - 2026-09-29

### Added
- iOS: pull down any list to reload it and run one exchange with the selected sync provider
- iOS: View Options on every list (the ••• menu) with Group By, Sort By, Order and, in
  Upcoming, the 7/30-day range; Settings › Task Lists keeps the same choices
- iOS: View Options › Layout offers Regular and Compact task rows; Regular rows are also
  a little tighter than before
- iOS: Settings › Sync has a Background section with the "Sync in the background" switch
- Every platform: the task context menu's Move To submenu offers the two days after
  tomorrow by their weekday names ("Wednesday", "Thursday")
- iOS: on an iPhone wide enough in landscape, the lists sidebar sits beside the task list
- Linux and macOS: View Options › Layout offers Regular and Compact task rows
- iOS: the floating Add task control is a round plus button; it keeps the "Add task"
  accessibility label, a 44 pt target, Dynamic Type scaling and the Large Content Viewer
- iOS: a sync status badge in every list's navigation bar (spinner while syncing, the
  provider symbol in the accent color, orange when attention is needed) opens Settings › Sync;
  the text status at the end of the list stays and now sits closer under the last task

### Changed
- Every platform: Morning & Night grouping shows Morning first, then Today, then Evening
- Every platform: sync runs twice as often: edits upload 10 s after they settle and an
  idle app polls every 150 s; iOS background wakes may come every 15 minutes
- iOS: navigation bars are Liquid Glass again; lists scroll underneath with the system's
  soft edge effect instead of an opaque header
- iOS: pull to refresh is available only while a sync provider is selected

## [0.4.2] - 2026-09-28

### Added
- Linux: Preferences › Desktop › "Sync in the background" decides whether automatic sync
  keeps running while the window is closed; Sync Now always works
- iOS: Settings › Sync › "Sync in the background" lets iOS wake Momentum occasionally for
  one bounded Nextcloud exchange; Low Power Mode and the connection's automatic switch
  still apply

### Changed
- Linux and iOS: the task context menu is grouped, with every Move to destination
  (Morning, Tonight, Tomorrow, Next Week, Project) in one Move To submenu; on Linux the
  menu now acts on the whole selection when the clicked row is selected
- Linux: the empty search screen says "Search"; iOS: the search prompt is "Search"
- iOS: Settings › Task Lists notes that Upcoming is always grouped by day

## [0.4.1] - 2026-09-28

### Added
- macOS: View › Customize Toolbar… rearranges, removes and restores the toolbar's items
  (New Task, Sync Now, View Options, plus optional Quick Add and Archive Completed)
- macOS: Settings › Sync › "Sync in the background" decides whether automatic sync keeps
  running while Momentum has no window open; Sync Now always works

### Changed
- Coming Up is always one section per day — Tomorrow, then the weekday names of this week,
  then dates — on every platform; Group By is disabled while it is showing, and its rows no
  longer repeat the day the heading names
- macOS: dragging a selected task lifts and moves the whole selection, one item per task
- macOS: the task context menu is grouped, with every Move to destination (Morning, Tonight,
  Tomorrow, Next Week, Project…) in one Move To submenu
- macOS: the search field and empty search screen say "Search"
- The Linux and macOS apps' iOS sibling now carries the same version and build number

### Fixed
- macOS: clicking the Dock icon while the app ran without a window could open two main
  windows; the main window is now a single-instance scene

## [0.4.0] - 2026-09-23

### Added
- Preferences › Tasks › "Archive completed tasks immediately": a completed task goes
  straight to the archive and syncs; undo brings it back open
- Linux desktop parity: a native font chooser with independent content/interface
  scaling, exact task reveal through `mo open`, GApplication actions and task URLs,
  plus configurable Background Apps task-count modes
- One release process for every platform: `build-aux/release.py` keeps one version across
  the Rust workspace, the GTK app, the macOS app, the metainfo and this changelog, and a
  single Release workflow runs every test against the tag, then publishes the GitHub
  Release with the Flatpak bundles, the macOS app and `mo` for macOS and Linux, updates
  the Homebrew tap (`momentum` cask, `momentum-cli` formula), the signed Flatpak
  repository, and the Flathub manifest

### Changed
- The macOS app's version now follows the workspace instead of its own number

## [0.3.5] - 2026-09-15

### Added
- Morning: a view and a section at the top of Today for tasks tagged "Morning", mirroring
  Tonight (Ctrl+Shift+M, context menu, drop target, `mo morning`, `mo add --morning`)

### Changed
- The Morning and Tonight sidebar entries show only while today has tasks in that slot

## [0.3.0] - 2026-09-15

### Added
- Test suite: 110+ tests across every crate plus 40 headless GTK window tests
  (`build-aux/test.sh`, Broadway display, in-process WebDAV mock replacing the Python one),
  run by CI on every push; `docs/TESTING.md`
- `mo projects` and `mo tags` honour `--json`

### Fixed
- Restoring an archived task, or merging a peer snapshot, linked each subtask twice
- Deleting a task could only be undone from its toast, not with Ctrl+Z; drag reorders are
  now undoable as well
- Editing a task scheduled at a time showed "Not scheduled" for its day
- `mo undone` and `mo rm` could not find completed tasks by title
- `mo` failed outright when no session bus was available
- Modifier key preference: every shortcut uses Ctrl, Alt or Super (Option or Command on
  macOS); the Keyboard Shortcuts overlay and the in-app hints follow the choice

## [0.2.0] - 2026-09-14

### Added
- Sync with nearby devices: direct, serverless, end-to-end encrypted sync over the local
  network using LibreSync as the transport. Link two devices once with a six-digit pairing
  code (Preferences › Nearby Devices › Manage Devices…); ops travel a few seconds after a
  change and every five minutes; a device with Nextcloud relays what it receives. New
  `sp-p2p` crate with a two-device integration test; `docs/P2P.md`
- Scheduled times: the task dialog has a Time row (`14:30`, `2pm`, `0930`), stored as
  `dueWithTime`; rows show the time and Coming Up and sorting use it
- Reminders: none, at the scheduled time, or 5/10/15/30 minutes, 1 hour or 1 day before;
  a bell badge on the row; notifications say "Due at 15:30"
- Overdue section at the top of Today for open tasks whose day has passed
- Repeating tasks catch up: a repeat missed while the app was closed is created for its
  newest missed day (weekly on Monday, opened Wednesday, shows under Overdue dated Monday)
- Accessibility pass: every icon-only control and every task check box has an accessible
  name, badges are labelled, high contrast turns colour coding off; `build-aux/a11y-dump.py`
  checks the AT-SPI tree; `docs/ACCESSIBILITY.md`
- Translation pipeline: complete `po/POTFILES.in`, committed `po/momentum.pot`, CI checks
  for POTFILES completeness and compiling `.po` files, `docs/TRANSLATING.md` with the
  Weblate setup

### Changed
- Today membership follows `dueWithTime` when set, else `dueDay`, matching upstream's
  virtual Today tag; `mo today` shows scheduled times

## [0.1.5] - 2026-09-09

### Added
- `mo` command-line companion for Linux and macOS (add, today, tonight, upcoming, list,
  search, done, undone, plan, rm, projects, tags, sync, config, `--json`); forwards to the
  running app over D-Bus on Linux
- GNOME Shell search provider and KDE KRunner runner
- Reminder notifications with Done and Snooze 1 hour buttons; once-a-day morning summary
- Run in the background (Background portal, autostart, Background Apps status line)
- Launcher actions (New Task, Today, Search), quick-add window, drop or paste text and
  links onto the list to create tasks
- `momentum --add/--quick-add/--today/--search/--background`; `momentum://` and
  `superproductivity://` URL schemes
- Ctrl+Z global undo
- German translation

### Changed
- The system-wide shortcut opens the quick-add window instead of the whole app
- The main window exists hidden from startup so services can use the store


## [0.1.1] - 2026-09-09

### Added
- Create, edit, pause and remove repeat schedules in the app (daily, weekly, monthly on a
  date, the last day, or the nth weekday, yearly); Repeat row in the task dialog, context
  menu item, Ctrl+Shift+R
- Move to Next Week (next Monday) in the context menu and Ctrl+Shift+Down
- Notes: copy button in the group header, taller editor, notes badge on task rows
- Per-view empty states that say what to do next; "All done" panel with Archive Completed
- Automatic sync 20 seconds after the last local change (debounced)
- Sidebar show/hide toggle in the header bars
- Own Flatpak repository on GitHub Pages with signed builds for x86_64 and aarch64

### Changed
- Task context menu groups the Move actions with a separator
- Header bar carries only Sync Now and View Options; Select Tasks moved into the menu
- Notes and task dialog are larger


## [0.1.0] - 2026-09-08

First preview.

### Added
- Today (with a Tonight section for "Evening"-tagged tasks), Tonight, Coming Up (7 or 30 days), project, tag, Archive and Search views
- Guided New Task dialog with project, due day, estimate, tags and notes
- `#tag` autocomplete and `1h 30m` estimates in the quick-add box
- Repeating tasks with a repeat badge and plain-language schedule text
- Nextcloud sync compatible with Super Productivity's `sync-data.json`, including
  end-to-end encrypted files; backup import and export
- Drag tasks onto projects, tags or Today; drag to reorder in Manual Order
- Selection mode with bulk done, plan, move, tag and delete, and multi-task drag
- Context menus, undo toasts, keyboard shortcuts, and system-wide shortcuts through
  the GlobalShortcuts portal
- Follows the system accent color; color-coded projects and tags

[Unreleased]: https://github.com/dan-hart/momentum/compare/v0.1.5...HEAD
[0.3.5]: https://github.com/dan-hart/momentum/compare/v0.3.0...v0.3.5
[0.3.0]: https://github.com/dan-hart/momentum/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/dan-hart/momentum/compare/v0.1.5...v0.2.0
[0.1.5]: https://github.com/dan-hart/momentum/compare/v0.1.1...v0.1.5
[0.1.1]: https://github.com/dan-hart/momentum/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/dan-hart/momentum/releases/tag/v0.1.0
