<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!-- Copyright (C) 2026 Dan Hart -->
# Momentum for macOS

A Mac app, not a port of the Linux one. The two share everything below the surface and
nothing above it: one Rust core decides what a task list contains and what a change does,
while each platform draws it the way its own users expect. GTK 4 and libadwaita follow the
GNOME HIG on Linux; SwiftUI, `NavigationSplitView`, real menus, Spotlight and the Keychain
do the same job here.

## Build it

```sh
cd macos
xcodegen generate          # writes Momentum.xcodeproj (not committed)
open Momentum.xcodeproj
```

The generated scheme runs `scripts/build-core.sh` before package compilation, which compiles the Rust core,
generates the Swift bindings from the compiled library, and packages both as
`Packages/MomentumCore`. Nothing it produces is committed, so the bindings can never
drift from the core they were generated against.

From the command line:

```sh
macos/scripts/build-core.sh                 # release, every installed Apple target
macos/scripts/build-core.sh --debug         # faster: this Mac's architecture only
cd macos && xcodebuild -scheme Momentum build
```

Requirements: Xcode 27 (macOS 26 SDK), a Rust toolchain, and `xcodegen`
(`brew install xcodegen`). For a universal binary, add the second architecture with
`rustup target add x86_64-apple-darwin`.

## Test it

```sh
swift test --package-path macos/Packages/MomentumKit    # ~1 second, no app launch
cargo test -p momentum-core                             # the shared logic
```

There is no UI automation. The app's logic lives in the `MomentumKit` package rather than
in the app target, so the tests drive the real state object directly — switching views,
selecting rows, applying changes, reading back the toast and the undo — with no window on
screen and no app to wait for. Xcode lists the same tests in its test navigator because
the package is a dependency of the project.

See [docs/TESTING.md](../docs/TESTING.md) for how the layers fit together.

## How it is put together

| Piece | What it is |
|---|---|
| `crates/momentum-core` | The shared core: views, listings, every change with its undo batch, repeats, reminders, sync, nearby devices. No toolkit, no locale. |
| `crates/momentum-ffi` | The core compiled with its UniFFI scaffolding — the static library the app links. |
| `Packages/MomentumCore` | Generated: the Swift bindings plus `momentum_ffi.xcframework`. |
| `Packages/MomentumKit` | The app's own logic: `AppState`, the wording of what the core reports, preferences, the Keychain. Where the tests live. |
| `Momentum/` | Views, menus, and the AppKit edges: notifications, Spotlight, system-wide shortcuts, URL handling. |

The core hands over structured values, never sentences: a `DayLabel` rather than
"Tomorrow", a `Message` rather than "3 tasks deleted". `Strings.swift` turns them into the
reader's language and date format, and `crates/app/src/messages.rs` does the same job on
Linux. Because both switches are exhaustive, adding a case to the core stops the other
platform compiling instead of quietly leaving a blank label.

## Platform services

| Linux | macOS |
|---|---|
| Secret Service keyring | Keychain (service `momentum`, shared with the `mo` command line) |
| GSettings | `UserDefaults`, under the same key names |
| XDG data dir | `~/Library/Application Support/momentum`, the directory `mo` uses |
| `GNotification` | `UNUserNotificationCenter`, with the same Done and Snooze buttons |
| GNOME Shell search provider, KRunner | Core Spotlight |
| GlobalShortcuts portal | Carbon hot keys: ⌃⌥T to add from anywhere, ⌃⌥M to show the window |
| Background portal with autostart | `SMAppService` login item, plus an optional menu bar item |
| D-Bus `org.gtk.Actions` for `mo` | A Unix socket in the data directory, which `mo` also uses on Linux |

## Fonts

Open **Settings → Fonts → Choose Font…** to use the macOS Fonts panel. The chosen
family and face apply to app text. **Content size** controls task titles, subtitles,
notes and task entry; **Interface size** controls navigation, settings and app controls.
Both range from 10–32 pt and default to 13 pt. The Fonts panel's size also changes the
content size. A live preview shows both sizes; **Reset Fonts to Defaults** restores
the system font and sizes. Changes persist locally and update open windows immediately.
Missing fonts fall back to the system font. System menu bars and window titles retain
macOS typography.

## Data and privacy

The macOS bundle identifier is `com.codedbydan.Momentum`. On first launch, saved
preferences from `io.github.dan_hart.Momentum` migrate once without overwriting choices
already saved for the new identity. The task-data directory and Keychain service stay
the same. LibreSync retains its shared protocol identifier for Linux compatibility.

The app and `mo` share one store and one Keychain service, so either can be used at any
time. Settings → Sync offers one method: Off, Nextcloud, or LibreSync. Only the selected
service can run, including automatic and CLI-triggered sync. Switching keeps saved
connections and pairings. Existing nearby-sync users migrate to LibreSync; other enabled
users migrate to Nextcloud. Preview mode never starts either transport.

The main window and Sync settings show the selected service, in-progress spinner, last
successful exchange, and persistent error details with Retry—even for an empty list.
LibreSync listens on TCP 52345 and discovers over Bonjour (`_libresync._tcp`), which is why the
app declares the local network usage description and is not sandboxed — a sandboxed app
could not share `~/Library/Application Support/momentum` with the command line.


## MVP status

The continuation is tracked against the original handoff in
[docs/MACOS-MVP-CHECKLIST.md](../docs/MACOS-MVP-CHECKLIST.md), including live drag/drop
acceptance. German app and package resources are documented in [LOCALIZATION.md](LOCALIZATION.md).
A successful build or preview does not by itself complete the MVP checklist.

## Command line on macOS

Every app build includes `Contents/MacOS/mo`, built from the same source and signed
with the app's identity. Install a link in `~/.local/bin` (already-existing commands
are never overwritten):

```sh
macos/scripts/install-cli.sh /Applications/Momentum.app
mo add "Prepare slides #work 1h 30m" --tomorrow
mo today
mo --json upcoming --days 14
mo done "Prepare slides"
mo undone "Prepare slides"
```

Put `~/.local/bin` on `PATH` if needed. For a development preview, pass that app's
actual path to the installer. Keep the app in that location; the link follows its
updates. `MOMENTUM_CLI_BIN_DIR` selects another install directory.

All commands are available on macOS: `add`, `today`, `morning`, `tonight`, `upcoming`,
`list`, `search`, `done`, `undone`, `plan`, `rm`, `projects`, `tags`, `sync`, `config`.
Use `mo --help` or `mo COMMAND --help`. `--json` works for queries, mutations, config
and sync requests. Lists/add return arrays; mutations return `{status, task}`;
config returns non-secret settings plus `data_dir`. A forwarded sync returns
`{status: "requested"}`: this acknowledges scheduling, not remote completion.

Both apps default to `~/Library/Application Support/momentum`. CLI precedence is
`--data-dir`, `MO_DATA_DIR`, `MOMENTUM_DATA_DIR`, then the platform default. Use
`MOMENTUM_DATA_DIR` for a profile shared by the GUI and CLI. A running app handles
writes through its local socket. A connected request's rejection or timeout is an
error, never permission to write around the app. With the app closed, `mo` writes
the same store directly. Completion through the running app respects auto-archive;
archived tasks are read-only. Standalone completion has no GUI preference context.

`mo config` and macOS Settings share non-secret Nextcloud connection fields in both
directions. Credentials use the same Keychain service; sync enablement remains an
explicit local app preference. `mo sync` honors the selected method in `cli-config.json`:
Off refuses sync, LibreSync forwards to the running app (or asks you to open it), and
Nextcloud can also run standalone. Legacy configs without a method retain their existing
Nextcloud behavior. Password prompts never emit passwords as JSON.

## Shortcuts.app

Open Momentum once, then search **Momentum** in Shortcuts' action library:

- **Create Task** — title, notes, plan, optional project and due date; returns a Task.
  Supports `#tags` and estimates. An explicit due date overrides the plan's date.
- **Find Tasks** — all, today, morning, tonight or upcoming; title contains filter and
  an include-completed switch. Today includes overdue tasks; archive is excluded.
- **Set Task Completed** — complete or reopen one or more current tasks; returns the
  count changed. Completion respects auto-archive, so an archived task cannot reopen
  through this action. Use Momentum's Undo to reverse an accidental completion.
- **Plan Tasks for Today** — returns the count changed, clearing previous time/reminders.
- **Open Task** — brings Momentum forward and reveals the selected task.

Task results expose ID, title, notes, project, due date and completion. Chain
**Create Task → Open Task**, or **Find Tasks → Set Task Completed**. Other actions run
in the background and keep the current list selected. Mutations use the app's existing
refresh, indexing, undo and sync paths. English and German action names, summaries and
App Shortcut phrases are included.

Native App Intents registration needs an Apple Development/Developer ID signature.
Ad-hoc builds compile the metadata but macOS may refuse to expose or execute it.
Choose your own team in Xcode, or use explicit local signing overrides:

```sh
xcodebuild -project macos/Momentum.xcodeproj -scheme Momentum \
  -configuration Debug CODE_SIGN_STYLE=Manual \
  CODE_SIGN_IDENTITY="Apple Development" DEVELOPMENT_TEAM=YOUR_TEAM_ID build
```

The generated Momentum scheme refreshes shared Rust APIs before compiling dependent
Swift packages. Before running standalone Swift package tests, run
`macos/scripts/build-core.sh --debug` so their generated bindings are current. No personal
signing identity or team is committed to the project defaults.
