# Changelog

All notable changes to Momentum are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

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
[0.1.5]: https://github.com/dan-hart/momentum/compare/v0.1.1...v0.1.5
[0.1.1]: https://github.com/dan-hart/momentum/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/dan-hart/momentum/releases/tag/v0.1.0
