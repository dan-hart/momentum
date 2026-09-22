# Linux Desktop Parity Design

**Date:** 2026-09-17

**Scope:** F-022, F-031, F-032, plus Linux verification of F-027, F-028, F-037, and F-038
**Baseline:** `main` at `7ec82e9`; 213 tests pass through `build-aux/test.sh`

## Goal

Bring the established Linux client to the capability level recorded for the macOS
client, using native GNOME equivalents. Preserve the shared Rust core, offline-first
behavior, accessibility, localization, and existing sync compatibility. Retest the
complete Linux contract and record evidence without implying that an iOS client exists.

## Scope decisions

The feature ledger and `AGENTS.md` define parity. The older MVP inventory is an existing
behavior specification, not the complete platform status ledger.

This work includes:

- F-022: opt-in Linux typography controls with separate content and interface scaling;
- F-031: verify and document Linux-native automation equivalence;
- F-032: configurable task count through GNOME's Background Apps status;
- F-027/F-028: native Linux verification of sync selection and visible status;
- F-037: native Linux verification of exclusive Group By behavior;
- F-038: native Linux verification of configurable morning summaries; and
- the corresponding English/German resources, tests, and living project records.

This work excludes:

- an iOS or Android application foundation;
- the future product decisions F-033 through F-036;
- the MVP exclusions in `docs/MVP.md` section 7;
- new sync providers, data migrations, dependencies, analytics, billing, or release work;
- a new Linux automation framework when existing native surfaces already provide the
  capability; and
- a nonstandard dock integration that would bypass GNOME or Flatpak conventions.

The future iOS requirements remain Planned in the feature ledger. No mobile status is
promoted by desktop work.

## Architecture

Task behavior remains in `crates/momentum-core`. GTK/libadwaita owns Linux preferences,
presentation, portal calls, and platform wording. Typography and count preferences are
local device settings and never enter task data or sync.

The design follows two native-equivalence rules:

1. Linux does not copy an Apple API when GNOME already provides a different native
   surface for the same capability.
2. Platform differences must be explicit and tested; they cannot be labeled Not
   applicable merely to hide missing behavior.

## F-022: Linux typography

### User behavior

Preferences → Appearance gains a Typography group with:

- a `GtkFontDialogButton` native font chooser;
- an `AdwSpinRow` for Content size, expressed as a percentage of the system text size;
- an `AdwSpinRow` for Interface size, independently expressed as a percentage of the
  system text size; and
- an `AdwButtonRow` for Reset Typography, which clears the font override and restores
  both scales to 100%.

The default remains the system font at 100%. A font choice is opt-in and local to the
device. Content covers task titles, subtitles, notes, and task entry. Interface covers
navigation, settings, headings, controls, and helper text. System-managed window chrome
remains untouched.

GNOME's typography guidance recommends using the system font and avoiding hard-coded
font sizes. Therefore, Momentum stores relative scales rather than absolute points and
keeps the unmodified system appearance as the default:
<https://developer.gnome.org/hig/guidelines/typography.html>.

### Components and boundaries

Create `crates/app/src/typography.rs` as the single owner of Linux typography
presentation. It will:

- read a small immutable preference value (optional font description plus two scales);
- validate and clamp persisted values;
- generate escaped application CSS;
- apply the CSS provider to the display; and
- expose a reset operation.

It does not read tasks, write GSettings itself, create preference widgets, or contain
business rules. Preferences binds native controls to GSettings and passes the resulting
value to the typography module. Application startup applies the saved value before any
app-owned surface is presented. Preference changes reapply it immediately.

Every app-owned top-level surface receives the interface class when constructed: the
main window, Preferences, quick-add window, task and repeat dialogs, project/tag dialogs,
and Nearby Devices dialog. System-managed title bars, menus, notifications, and portal
UI are outside these roots and remain untouched. Task titles/subtitles, quick-add and
task-title entries, note editors, and other task-content labels receive the content
class. Shared construction helpers apply the classes so a newly added surface cannot
silently skip typography.

The interface class receives the optional font family/face and interface scale.
Task-content widgets receive a content class whose ratio is calculated against the
interface scale, keeping the two user choices independent while both remain relative
to the system base size.

### Settings

Add these local GSettings keys:

- `typography-font`, a string; empty means the system font;
- `typography-content-scale`, an integer from 75 through 250, default 100; and
- `typography-interface-scale`, an integer from 75 through 250, default 100.

Both spin rows use five-percent steps. The font dialog may return a Pango font
description containing a size. Before persistence, Momentum discards absolute size and
size fields and retains only family and face attributes (style, variant, weight, and
stretch). Size is owned exclusively by the two scale settings.

The supported range is 75% through 250%, covering approximately the macOS 10–32 point
range around its 13 point default without storing platform-specific point sizes.
Unknown or invalid values fall back to 100%. No migration is needed because defaults
preserve existing Linux appearance.

### Accessibility and failure handling

- GNOME Large Text continues to affect the base size because Momentum's values are
  relative.
- High contrast and project/tag color policies remain unchanged.
- Missing or removed fonts fall back to the system font.
- Font names are escaped before entering CSS.
- Invalid persisted scales are clamped or defaulted without preventing startup.
- Reset removes overrides rather than hard-coding a particular GNOME font.
- Keyboard focus reaches font, content size, interface size, and reset in that order.
- Acceptance includes 250% content and interface scales in the minimum supported window
  size, with no unreachable controls or clipped task content.

## F-031: Linux-native automation

Linux already exposes the macOS Shortcuts capabilities through native Unix/GNOME
surfaces:

| Capability | Linux surface |
|---|---|
| Create | `mo add`, `momentum --add`, quick-add launcher action, and `momentum://add` |
| Find | `mo today/morning/tonight/upcoming/list/search --json`, GNOME search provider, and KRunner |
| Complete or reopen | `mo done` / `mo undone`, with live-app IPC and auto-archive semantics |
| Plan for today | `mo plan` |
| Open or reveal | new `mo open TASK`, backed by the existing GApplication action/URI architecture |

`GActionGroup` is explicitly intended as a public action surface for external forces,
including incoming D-Bus messages:
<https://docs.gtk.org/gio/iface.ActionGroup.html>.

The audit found one real gap: Linux can search for a task but cannot ask Momentum to
open one exact current task as the Mac Open Task action does. Close that gap within the
existing architecture:

- add a parameterized `app.open-task` GApplication action that accepts a stable task ID,
  presents the main window, navigates to the task, and opens its editor;
- add `momentum://open-task?id=…` as the desktop-activation form of the same action; and
- add `mo open TASK`, using the CLI's existing exact-ID, unique-prefix, and unique-title
  resolution rules. It activates the registered desktop app rather than mutating the
  store. Missing, ambiguous, archived, or deleted tasks return a nonzero error and a
  JSON error when `--json` is requested. On accepted activation, `--json` returns
  `{"status":"requested","task":<standard task JSON>}`; “requested” does not claim
  that the user has already seen the editor.

This extends existing GApplication/URI/CLI surfaces; it does not add a second automation
service. The remaining work is to:

- add a focused Linux automation document containing the mapping, examples, JSON
  contracts, live-app safety rules, and platform differences;
- audit existing integration tests against every mapped capability;
- add a behavioral test only if the audit finds an actual uncovered behavior; and
- record F-031 as Verified only when the documented commands and live-app paths pass.

Automation retains the current safety invariant: once a running app accepts a socket or
D-Bus connection, rejection, timeout, disconnection, or persistence failure is an error.
The CLI must never write around the app.

Verification covers these outcomes explicitly:

- with no app running, queries read the durable store and mutations use the established
  offline path; `mo open` activates the registered app and does not write the store;
- with the app running, mutations use socket/D-Bus IPC and refresh the app; `mo open`
  targets that instance and opens the resolved task;
- a connected mutation rejected by the app, timed out, disconnected, or failed during
  persistence returns an error and never falls back to direct writing;
- every command's `--json` success/error shape remains machine-readable and contains no
  secrets; and
- URI, search-provider, launcher, and CLI activation use isolated test data and never
  open or modify the user's store.

## F-032: configurable Linux task count

GNOME does not provide Momentum with a standard launcher-icon badge API. Its native
equivalent is the Background Apps status message, which the app already publishes
through `org.freedesktop.portal.Background.SetStatus`:
<https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Background.html>.

### User behavior

Preferences → Desktop gains an `AdwComboRow` named Background task count:

- Due or scheduled today (default);
- Today including overdue; and
- Off.

When Run in the Background is enabled, Momentum publishes the selected count to the
Background Apps surface. Off publishes a neutral “Momentum is running” status instead
of a task count. The preference remains saved while background mode is disabled.

The GSettings key is `background-count-mode`, with choices `due-today`,
`today-including-overdue`, and `off`; the default is `due-today`. Unknown values fall
back to the documented default.

### Shared count contract

The Rust core owns count semantics through an exported `TaskCountMode` enum with
`DueToday`, `TodayIncludingOverdue`, and `None` variants, plus
`Engine::task_count(mode) -> u32`:

- Due or scheduled today counts unfinished top-level tasks whose plain or timed due day
  is today and excludes overdue tasks.
- Today including overdue counts unique unfinished top-level tasks in the Today listing,
  including overdue tasks.
- Completed tasks and subtasks never increase either count.
- `None` returns zero. GTK uses the neutral message for that mode; it does not format a
  zero-task message.

GTK owns only preference selection, localized singular/plural wording, and the portal
call. Count status refreshes after task mutations, undo, sync, external CLI changes,
view refreshes, and preference changes.

`TaskCountMode` crosses UniFFI. The macOS `DockBadgeMode` maps to it and replaces the
current Swift-side Today-list counting, so both desktops consume the same rule. The Mac
preference keys and visible behavior do not change. Regenerated bindings come from the
existing build tooling and are not hand-edited.

### Status messages and portal controller

GTK formats these messages with gettext and `ngettext`:

| Mode | Zero | One | Many |
|---|---|---|---|
| Due or scheduled today | All done for today | 1 task due today | N tasks due today |
| Today including overdue | All done for today | 1 open task in Today | N open tasks in Today |
| Off | Momentum is running | Momentum is running | Momentum is running |

Before the portal call, the localized message is limited to 96 Unicode characters, as
required by the portal. Overlong translations are shortened to 95 characters plus an
ellipsis; a test covers scalar-safe truncation.

Create a small background-status controller with no GTK dependency. It tracks desired,
in-flight, and last-acknowledged messages and issues at most one request at a time.
Rapid updates coalesce to the newest desired message. Success acknowledges only the
matching request. Failure leaves the desired message dirty, so the next refresh retries;
it does not spin in an immediate retry loop. Serial requests prevent stale completions
from overwriting a newer message. Controller tests use an injected sender rather than a
real portal.

Portal failures are logged without interrupting task management. A failed call is not
reported as a successful status update.

## Recent Linux feature verification

The pulled source already implements these capabilities but the ledger records native
Linux verification gaps:

- F-027: one selected sync provider;
- F-028: persistent provider/progress/success/error status and Retry;
- F-037: exclusive Group By modes; and
- F-038: opt-in morning summary and local time controls.

The parity pass will exercise their existing GTK regressions through the complete
Blueprint/resource/Broadway suite and inspect their native controls using isolated data
and settings. No feature is rewritten merely to obtain new evidence. The scenario gates
are:

| Feature | Automated gate | Native Linux gate |
|---|---|---|
| F-027 | Off/Nextcloud/LibreSync are exclusive; legacy dual-enabled selects LibreSync; switching preserves connection fields and linked devices; only the selected transport starts | Change through all three choices; verify only the matching group and controls appear, saved connection values return after switching, and preview mode starts no transport |
| F-028 | Empty and populated listings retain status; progress disables conflicting actions; success updates time; failure persists Details and Retry; Retry calls the selected provider | Inspect empty and populated states for Off/progress/success/error; activate Details and Retry with an isolated failure state |
| F-037 | All five modes, exclusive sections, order, first-tag placement, estimate boundaries, due-date visibility, Completed separation, colors/fallbacks, reorder/undo, and cross-group rejection | Select and read back every native menu choice; inspect ordering and headings; test keyboard reorder/undo; inspect colorful-off and high-contrast fallback; pointer/drag acceptance is required before Verified |
| F-038 | Default off/08:00, enablement, valid time changes, engine update, restart persistence, once-daily claim, reminder independence, and failure-safe notification adapter | Edit time, toggle off/on, reopen Preferences/app, inspect 250% and German layouts, and exercise an isolated eligible summary. If system delivery or denial cannot be observed, retain Implemented |

Any unavailable native gate leaves that feature Implemented. Automated coverage alone
does not produce a partial Verified status.

## Data flow

### Typography

1. GSettings provides font and scale values.
2. Preferences or startup constructs a validated typography preference value.
3. The font dialog's size is stripped and the typography module renders escaped,
   system-relative scoped CSS.
4. All registered app surfaces and content widgets inherit the updated provider
   immediately.

### Background count

1. Task or preference state changes request a status refresh.
2. The GTK layer asks the core for the selected count.
3. GTK formats a localized status message.
4. The background-status controller serializes and coalesces updates.
5. The Background portal receives `SetStatus` only when the desired message differs
   from the last acknowledged value.

### Automation

1. Shell, URI, launcher, search-provider, or D-Bus entry receives the request.
2. Existing parsing and lookup rules resolve it.
3. Mutations go through the running engine when present, otherwise the CLI's established
   safe offline path.
4. Normal refresh, persistence, undo, indexing, and sync scheduling follow.

## Testing strategy

Implementation follows red-green-refactor. Production behavior is not added before a
test demonstrates the missing behavior.

### Focused tests

- Typography: defaults, persistence, independent scale calculation, supported bounds,
  five-percent steps, stripped embedded font size, invalid-value fallback, CSS escaping,
  missing-font fallback behavior, every registered surface, focus order, and reset.
- Core count: timed/plain today, overdue inclusion difference, completed exclusion,
  parent/subtask uniqueness, and empty state.
- GTK: preference bindings, immediate typography refresh hooks, count-mode changes,
  exact plural status wording, 96-character truncation, controller coalescing/failure/
  retry/ordering, and control accessibility names.
- Automation: existing CLI/IPC safety plus `mo open`, GApplication/URI activation,
  exact task resolution, closed/running app behavior, and JSON errors.
- Regression: existing sync selector/status, Group By, and morning-summary tests remain
  green.

### Final verification

Run:

1. `build-aux/test.sh` for Blueprint compilation, resources, schema, Broadway GTK UI,
   core crates, CLI, and LibreSync loopback;
2. `cargo test --workspace --exclude momentum --all-features --all-targets`;
3. `macos/scripts/build-core.sh --debug` to regenerate and compile the UniFFI boundary;
4. `swift test --package-path macos/Packages/MomentumKit` for the Mac consumer of the
   shared count API; if the Apple toolchain is unavailable, record that limit and do not
   promote affected Mac evidence;
5. strict GSettings schema compilation;
6. POTFILES completeness and German catalog compilation;
7. Rust formatting and `git diff --check`;
8. changed-document link checks; and
9. isolated native Linux inspection of Typography, Background task count, Group By,
   Sync, and Morning summary, plus the accessibility-name audit when the session exposes
   AT-SPI.

The baseline before implementation is 213 passing tests through `build-aux/test.sh`.
Final results record actual counts and any environmental warnings rather than copying
this number as a promise.

## Acceptance and status updates

- F-022 becomes Implemented when the GTK code and settings exist; it becomes Verified
  only after focused tests, the complete Linux suite, and native preference/readability
  checks pass.
- F-031 becomes Verified when the mapping audit and applicable live-app/CLI tests pass.
- F-032 becomes Implemented when the preference and portal behavior exist; it becomes
  Verified only after core, GTK, and native Background Apps checks pass.
- F-027, F-028, F-037, and F-038 are promoted from Implemented to Verified only for the
  exact Linux scope exercised. External Nextcloud or physical LibreSync success is not
  inferred from isolated UI or loopback tests.
- iOS and Android remain Planned.

Update `docs/FEATURES.md`, `docs/PROGRESS.md`, `docs/MVP.md`, `docs/TESTING.md`, and
`CHANGELOG.md` in the same change. Add a bug-ledger entry only if a reproducible defect is
found; verification gaps alone remain in progress documentation.

## Repository and delivery constraints

- Preserve unrelated user work and existing data.
- Use isolated stores and settings for every test and native check.
- Do not restart GNOME Shell, log out, or restart the machine.
- No commit, push, PR, tag, publication, or release is authorized by this design.
