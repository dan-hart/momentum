# Linux Desktop Parity Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Bring Linux to the repository-defined desktop parity baseline for F-022, F-031, and F-032, then close the documented Linux native-verification gaps for F-027, F-028, F-037, and F-038.

**Architecture:** Keep shared task-count semantics in `momentum-core`, with macOS and GTK adapting native presentation. Extend Linux's existing GApplication/URI/`mo` automation surface instead of introducing a parallel API. Keep typography and Background portal behavior in small testable GTK-side modules, and preserve the repository's native-UI and offline-first boundaries.

**Tech Stack:** Rust, GTK4/libadwaita, Gio/GApplication, ashpd Background portal, GSettings, Clap, UniFFI, Swift/MomentumKit, Blueprint, gettext, Cargo/Swift tests.

---

## Guardrails

- Work in `/var/home/danhart/.config/superpowers/worktrees/momentum/linux-desktop-parity` on `task/linux-desktop-parity`.
- Follow `docs/superpowers/specs/2026-09-17-linux-desktop-parity-design.md` exactly.
- Use test-driven development: add a focused failing test, run it and observe the expected failure, implement the minimum behavior, then rerun it green.
- Do not commit, push, open a PR, tag, release, log out, restart GNOME Shell, or restart the machine. At each normal commit checkpoint, inspect and report the local diff instead.
- Preserve iOS and Android as Planned. Do not scaffold mobile targets.
- A source or automated-test pass supports Implemented. Mark Linux Verified only where the native acceptance scenario is actually observed and recorded.
- This worktree's ignored `libresync-src` symlink points to the dedicated detached `v0.6.0` worktree at `/var/home/danhart/.config/superpowers/worktrees/libresync/momentum-v0.6.0`, matching `Cargo.lock` and CI. Do not repoint it to the user's newer LibreSync checkout.

## Task 1: Put configurable task-count semantics in the shared core

- [x] 1.1 Add and run the failing shared-core count tests.
- [x] 1.2 Implement `TaskCountMode` and `Engine::task_count` and turn the tests green.
- [x] 1.3 Regenerate the boundary and map the unchanged macOS preference to the core query.
- [x] 1.4 Inspect the local diff without committing.

**Files:**

- Modify: `crates/momentum-core/src/types.rs`
- Modify: `crates/momentum-core/src/engine.rs`
- Modify: `crates/momentum-core/src/tests.rs`
- Modify: `macos/Packages/MomentumKit/Sources/MomentumKit/Preferences.swift`
- Modify: `macos/Packages/MomentumKit/Sources/MomentumKit/AppState.swift`
- Modify: `macos/Packages/MomentumKit/Tests/MomentumKitTests/AppStateTests.swift`
- Regenerate as needed: `macos/Packages/MomentumCore/Sources/MomentumCore/momentum.swift`

### 1.1 Add failing shared-core count tests

Add tests that create an empty store plus unfinished top-level tasks with a plain due day today, a timed due value today, overdue, future, completed, and subtasks. Assert:

- `TaskCountMode::DueToday` counts only unfinished top-level tasks whose effective due/scheduled local day is today.
- `TaskCountMode::TodayIncludingOverdue` counts unique unfinished top-level tasks in the core Today listing, including overdue tasks.
- `TaskCountMode::None` returns zero.
- Completed tasks and subtasks never increment the count, and one parent is counted only once regardless of its subtasks.
- Every mode returns zero for the empty store.

Run:

```sh
cargo test -p momentum-core task_count -- --nocapture
```

Expected: compile failure because `TaskCountMode` and `Engine::task_count` do not exist.

### 1.2 Implement the enum and query

In `types.rs`, add an FFI-safe exhaustive enum:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum TaskCountMode {
    DueToday,
    TodayIncludingOverdue,
    None,
}
```

In `engine.rs`, add `pub fn task_count(&self, mode: TaskCountMode) -> u32`. Reuse the same local-day and Today-membership helpers used by listings; do not duplicate UI-specific date rules. Keep `today_open_count` temporarily only if an existing caller still needs it, then migrate callers and remove it if unused.

Run the focused test again. Expected: pass.

### 1.3 Map the macOS preference to the shared API

Give `DockBadgeMode` an exhaustive conversion to `TaskCountMode`; `none` maps to `.none`. Replace local Swift-side badge-count derivation with `engine.taskCount(mode:)`. Update AppState tests so the existing badge behavior still covers immediate preference changes, overdue inclusion, subtasks, completed tasks, invalid preference fallback, and zero hiding.

Run:

```sh
swift test --package-path macos/Packages/MomentumKit --filter AppStateTests
```

Expected: pass. If generated bindings are stale, run the repository's macOS core binding script first, then rerun the test.

### 1.4 Check the local diff

Run `git diff --check` and inspect `git diff --stat`. Do not commit.

## Task 2: Complete the Linux native automation surface with exact task opening

- [x] 2.1 Add the activation seam and failing CLI tests for warm, cold, rejected, and invalid open requests.
- [x] 2.2 Implement safe non-mutating GApplication activation and turn the CLI tests green.
- [x] 2.3 Implement and test exact reveal/navigation/editor behavior for GAction and URI entry points.
- [x] 2.4 Audit and document every mapped Linux automation capability.
- [x] 2.5 Inspect the local diff without committing.

**Files:**

- Modify: `crates/mo/src/main.rs`
- Create: `crates/mo/src/lib.rs`
- Modify: `crates/mo/tests/cli.rs`
- Modify: `crates/app/src/application.rs`
- Modify: `crates/app/src/window.rs`
- Modify: `crates/app/src/tests/ui.rs`
- Modify: `data/io.github.dan_hart.Momentum.desktop.in.in`
- Create: `docs/LINUX-AUTOMATION.md`
- Modify: `README.md` to link the focused automation reference

### 2.1 Add failing `mo open` black-box tests

Extend the CLI command enum with the intended syntax and add black-box tests for:

- exact task ID resolves successfully;
- the existing unique ID-prefix and case-insensitive title-fragment resolution rules are preserved;
- missing and ambiguous tasks use the existing human-readable error conventions and emit the same error through the CLI's JSON error envelope;
- archived and deleted/stale task requests fail nonzero with the same JSON error envelope;
- `--json` success is exactly `{"status":"requested","task":<standard task JSON>}`;
- connected mode sends a non-mutating open request and does not enqueue an oplog mutation;
- with no app running, the registered GApplication is activated and the request does not mutate the durable store;
- activation rejection, timeout, disappearance, or malformed acknowledgement is an error and never falls back to a write.

Move the reusable CLI runner/open path into `crates/mo/src/lib.rs`, leaving `main.rs` as the thin Clap/process adapter. Put desktop activation behind an injected `OpenActivator` trait accepted by `run_with`. Unit-test that runner with a deterministic fake recording activator for success, rejection, timeout, and no-owner/cold-start transitions. Keep `crates/mo/tests/cli.rs` black-box coverage for parsing, lookup errors, JSON/stdout/stderr, exit codes, and durable-store nonmutation. Add a mandatory private `dbus-daemon` integration test using existing `zbus` to verify the serialized `org.gtk.Actions.Activate` warm and cold paths; add no dependency.

Run:

```sh
cargo test -p mo open -- --nocapture
```

Expected: failure because the subcommand is absent.

### 2.2 Implement resolution and live-app coordination

Reuse the CLI's existing store loading, task JSON projection, D-Bus coordination, timeout, and error conventions. `open` must never create an oplog action. Request the app-level `open-task` action with the stable ID. If no owner exists, activate the registered GApplication through its standard desktop/D-Bus activation path and then deliver the action. Report success only after the request is accepted; never write around an activation failure. Keep the documented JSON success contract for accepted activation.

Run the focused CLI tests. Expected: pass.

### 2.3 Add the GApplication action and URI

In `application.rs`:

- add parameterized `app.open-task` accepting a string ID;
- add a mandatory `reveal_and_open_task` helper that navigates to the task's owning Project view, with a title Search fallback when the project is unavailable, then selects or scrolls to the stable task ID among the results, opens its editor, and presents the main window; do not treat the existing editor-only `open_task` helper as sufficient;
- accept `momentum://open-task?id=<percent-encoded-id>`;
- keep malformed/missing IDs safe and logged without changing data.

Add GTK tests that activate the action and URI against isolated demo data and assert the owning Project view (or title Search fallback with stable-ID selection) is visible, the row is selected/scrolled into view, and the editor is presented. Add archived, deleted/stale, malformed, and not-found cases that cause no data mutation or crash.

Run:

```sh
./build-aux/test.sh -p momentum open_task -- --nocapture
```

Expected: pass.

### 2.4 Document the Linux equivalents

Before adding tests beyond `open`, create an audit table mapping Create, Find, Complete, Reopen, Plan, Sync, Search, Quick Add, and Open/Reveal to an existing test name. Add a new behavioral test only for an uncovered mapped capability. Document every capability, examples, JSON success/error contracts, which commands work offline, the activatable-app behavior, live-app safety invariants, and Linux/macOS differences. Do not add an unrelated desktop action.

Run `git diff --check`; inspect the diff; do not commit.

## Task 3: Add native Linux font selection and independent content/interface scaling

- [x] 3.1 Add and run failing pure typography tests.
- [x] 3.2 Implement settings and accessible native preference controls.
- [x] 3.3 Register every app-owned surface and content widget through shared helpers.
- [x] 3.4 Run focused UI/layout/fallback tests and inspect the local diff.

**Files:**

- Create: `crates/app/src/typography.rs`
- Modify: `crates/app/src/main.rs`
- Modify: `crates/app/src/application.rs`
- Modify: `crates/app/src/prefs.rs`
- Modify: `crates/app/src/window.rs`
- Modify: `crates/app/src/quick_add.rs`
- Modify: `crates/app/src/task_form.rs`
- Modify: `crates/app/src/repeat_dialog.rs`
- Modify: `crates/app/src/p2p.rs`
- Modify: `crates/app/src/tests/logic.rs`
- Modify: `crates/app/src/tests/ui.rs`
- Modify: `data/resources/ui/prefs.blp`
- Modify: `data/io.github.dan_hart.Momentum.gschema.xml.in`
- Modify: `data/resources/style.css`

### 3.1 Add failing pure typography tests

In `typography.rs`, design pure helpers and test first:

- scale values clamp to 75–250 and preserve 5% steps;
- an empty font string means the GTK/system font;
- a chosen Pango font description retains family/face/style attributes but strips absolute/relative size fields;
- font families containing quotes, slashes, and non-ASCII text are escaped safely for CSS;
- missing/unresolvable fonts omit the override and fall back to the system font;
- invalid persisted scale values default or clamp without blocking startup;
- CSS generation scopes interface and content scale independently, with content using `content_scale / interface_scale` beneath the interface root;
- generated CSS remains valid at 75%, 100%, and 250%.

Run:

```sh
./build-aux/test.sh -p momentum typography -- --nocapture
```

Expected: compile failure because the module/helpers do not exist.

### 3.2 Implement settings and the native preference controls

Add GSettings keys:

- `typography-font`: string, default empty;
- `typography-content-scale`: integer range 75–250, default 100;
- `typography-interface-scale`: integer range 75–250, default 100.

Add a native `GtkFontDialogButton`, two `AdwSpinRow`s with 5% increments, and a reset `AdwButtonRow`. Bind them bidirectionally. Reset restores system font and both 100% scales. Add accessible titles/descriptions and keep controls usable by keyboard.

### 3.3 Apply scoped CSS from system-relative values

Install one application-level CSS provider and rebuild only the typography rules when related settings change. Derive the baseline from GTK/Pango settings rather than hardcoding a platform font or point size. Expose shared `register_interface_root(widget)` and `register_content_root(widget)` helpers from `typography.rs`. Use the interface helper for the main window, Preferences, quick-add window, task editor, repeat editor, project/tag dialogs, and Nearby Devices dialog. Use the content helper for task titles/subtitles, task rows, quick-add/task-title entries, note editors, and other task-content labels.

Set the interface root to `interface_scale / 100`. Because content widgets are descendants of it, set the content root to `content_scale / interface_scale`; this prevents the two choices from multiplying. The optional font family/face belongs on the interface root and therefore inherits into content. Startup registers/applies before presentation, and each related GSettings change reapplies immediately.

Add UI tests for settings bindings, five-percent steps, live refresh, reset, missing-font fallback, invalid persisted values, escaped CSS, every named registered surface/content class, preference focus order (font → content → interface → reset), accessible names/descriptions, and 250% content/interface layout survivability at 360×720 without unreachable controls or clipped task content.

Run:

```sh
./build-aux/test.sh -p momentum typography -- --nocapture
./build-aux/test.sh -p momentum preferences_rows_are_bound_to_settings -- --nocapture
```

Expected: pass.

### 3.4 Check the local diff

Run `git diff --check` and inspect the diff. Do not commit.

## Task 4: Make the GNOME Background Apps status count configurable and race-safe

- [x] 4.1 Add and run failing controller, wording, and truncation tests.
- [x] 4.2 Implement the persistent accessible count-mode preference and shared mapping.
- [x] 4.3 Replace fire-and-forget portal requests with the serialized controller.
- [x] 4.4 Run focused UI/controller tests and inspect the local diff.

**Files:**

- Create: `crates/app/src/background_status.rs`
- Modify: `crates/app/src/main.rs`
- Modify: `crates/app/src/window.rs`
- Modify: `crates/app/src/prefs.rs`
- Modify: `crates/app/src/tests/logic.rs`
- Modify: `crates/app/src/tests/ui.rs`
- Modify: `data/resources/ui/prefs.blp`
- Modify: `data/io.github.dan_hart.Momentum.gschema.xml.in`

### 4.1 Add failing controller and wording tests

Test a small non-GTK controller/state machine for:

- `desired`, `inflight`, and `last_acknowledged` states;
- one portal request at a time;
- refresh bursts coalesce to the newest desired message;
- success records the acknowledged message and sends a newer pending value;
- failure does not spin or immediately retry; the next explicit refresh may retry;
- identical acknowledged messages are suppressed;
- UTF-8 output never exceeds the Background portal's 96-character maximum.

Test exact English source strings through gettext/ngettext inputs:

- Due today: `All done for today`, `1 task due today`, `%d tasks due today`;
- Today including overdue: `All done for today`, `1 open task in Today`, `%d open tasks in Today`;
- Off: `Momentum is running`.

Run:

```sh
./build-aux/test.sh -p momentum background_status -- --nocapture
```

Expected: compile failure because the controller does not exist.

### 4.2 Add the preference and shared count mapping

Add `background-count-mode` with choices `due-today`, `today-including-overdue`, and `off`, defaulting to `due-today`. Add a native `AdwComboRow` named Background task count near Run in Background. Map it exhaustively to `TaskCountMode`; unknown stored values fall back to `due-today`. Test its accessible name, immediate mode-change refresh, Off's neutral message, and that the saved choice is retained while Run in Background is disabled.

### 4.3 Replace fire-and-forget portal updates

Route `MomentumWindow::update_background_status` through the controller. Refresh after task-affecting operations, sync results, startup, and count-mode preference changes. Send only through `ashpd::desktop::background::SetStatusOptions`, preserve the existing no-background early return, and keep portal errors nonfatal.

Use gettext plural APIs instead of concatenating a localized noun. If a localized message exceeds 96 characters, truncate on a Unicode scalar boundary to at most 95 characters and append an ellipsis, never producing invalid UTF-8.

Run the focused controller and UI preference tests. Expected: pass.

### 4.4 Check the local diff

Run `git diff --check` and inspect the diff. Do not commit.

## Task 5: Execute the documented Linux native acceptance matrix and fix only observed defects

- [x] 5.1 Build/install the Devel Flatpak and create isolated persistent data/settings.
- [ ] 5.2 Verify F-022 typography natively, including 250%, German, keyboard, and accessibility.
- [x] 5.3 Verify F-031 warm/cold CLI, GAction, and URI activation without store mutation.
- [ ] 5.4 Verify F-032 in Preferences and the GNOME Background Apps surface.
- [ ] 5.5 Verify the complete F-027/F-028 sync matrix.
- [ ] 5.6 Verify the complete F-037 grouping matrix.
- [ ] 5.7 Verify the complete F-038 morning-summary matrix.
- [x] 5.8 Reproduce any observed defect with a failing test before fixing it, then record exact evidence and limits.

**Files:**

- Modify only files required by a reproduced defect.
- Modify: `docs/BUGS.md` for any new defect and its resolution.
- Modify: `docs/PROGRESS.md` with exact dated evidence.

### 5.1 Build and launch safely

Build/install the development profile, then create one task-specific host directory whose data and keyfile GSettings persist across safe app relaunches but never touch the normal Devel profile:

```sh
flatpak run org.flatpak.Builder --user --install --force-clean flatpak_app \
  build-aux/io.github.dan_hart.Momentum.Devel.json
PARITY_STATE=$(mktemp -d -p /var/tmp momentum-linux-parity.XXXXXX)
PARITY_XDG_DATA="$PARITY_STATE/xdg-data"
mkdir -p "$PARITY_STATE/data" "$PARITY_STATE/config" "$PARITY_XDG_DATA/dbus-1/services"
flatpak run --filesystem="$PARITY_STATE" \
  --env=MOMENTUM_DATA_DIR="$PARITY_STATE/data" \
  --env=GSETTINGS_BACKEND=keyfile \
  --env=XDG_CONFIG_HOME="$PARITY_STATE/config" \
  io.github.dan_hart.Momentum.Devel
```

Seed only synthetic tasks through the bundled `mo` with the same `MOMENTUM_DATA_DIR`. Record the resolved `$PARITY_STATE` path in the progress evidence and leave it in place until every relaunch/persistence check is complete. Do not use the user's normal Flatpak data/settings, restart GNOME Shell, log out, or restart the machine.

### 5.2 Verify F-022

In native Preferences, confirm the controls are ordered font, Content size, Interface size, Reset; each has a useful accessible name/description; keyboard focus follows that order; the default is system font/100%/100%; changes apply immediately and independently; Reset removes the override; and values persist across a safe app close/relaunch using `$PARITY_STATE`. Inspect the main window, Preferences, quick add, task/repeat/project/tag dialogs, and Nearby Devices at 250% content and interface scales at 360×720. Repeat the layout/readability pass with `LANGUAGE=de`. Run `build-aux/a11y-dump.py` while each representative surface is open when AT-SPI is available. A missing system font must visibly fall back rather than blanking text.

### 5.3 Verify F-031

With synthetic data and the same isolated store:

- `mo open`: do not run this command against `$PARITY_STATE/data`. The command deliberately
  refuses custom stores because they cannot be mapped safely to one registered app ID.
  Use the automated private-`dbus-daemon` warm/cold tests, which set a temporary `HOME`,
  create synthetic exact stable and Devel Flatpak store paths, exercise both registered
  IDs (including coexistence and a real nonreply deadline), and never address the normal
  session bus. Retain the native `mo open` status as Implemented unless a safe live test
  can use an exact registered profile path without touching personal data;
- URI/GAction: exercise `momentum://open-task?id=...` and `org.gtk.Actions.Activate("open-task", ...)` against the isolated instance;
- failures: verify missing, ambiguous, archived, deleted/stale, rejected, and timed-out requests are nonzero/machine-readable and do not mutate `state.json` or `pending.json`;
- JSON: verify accepted output is exactly `{"status":"requested","task":<standard task JSON>}` and contains no secrets.

Audit Create, Find, Complete, Reopen, Plan, Sync, Search, Quick Add, and Open/Reveal with the commands/examples in `docs/LINUX-AUTOMATION.md`. Record the exact warm/cold commands and observed UI result.

### 5.4 Verify F-032

Enable Run in Background and switch Background task count through Due or scheduled today, Today including overdue, and Off. Use synthetic plain-today, timed-today, overdue, completed, and parent/subtask cases. Confirm immediate count changes, the neutral Off message, persistence while background mode is disabled, and readback after relaunch. Inspect GNOME's Background Apps surface for the actual portal-published message after each mode; if the desktop does not expose the surface or portal acknowledgement, retain Implemented and record the environmental limit.

### 5.5 Verify F-027 and F-028

In native GTK preferences and the main-window footer:

- switch among Off, Nextcloud, and LibreSync and verify only the matching settings group/controls are visible;
- confirm exactly one provider is selected while saved connection fields and linked-device records return after switching;
- confirm demo/preview mode starts no transport;
- inspect both empty and populated lists for distinct Off, progress, success/last-time, and error states;
- confirm progress disables conflicting actions, then activate Details and Retry against an isolated failure and verify Retry invokes only the selected provider;
- confirm CLI handoff respects the selected provider.

If a defect appears, first add the narrowest reproducing automated test, observe it fail, implement the fix, rerun it green, then repeat the native scenario.

### 5.6 Verify F-037

Switch through Morning & Night, Project, First tag, Similar estimates, and None and read back every native menu choice. Confirm exactly one open-task grouping layer, correct order and headings, first-tag-only placement, untagged/no-project buckets, all six estimate boundaries, due-date visibility, Completed separation, and that Project mode has no Morning/Evening headings. Exercise keyboard reorder and Undo, reject cross-group reorder, inspect colorful-off and system high-contrast fallbacks, and complete the native pointer/drag acceptance. If pointer/drag or high-contrast observation is unavailable, retain Implemented.

### 5.7 Verify F-038

Confirm morning summary is Off by default, the time picker starts at 08:00 and is disabled while Off, valid enabling/time changes update the running engine, and values persist across Preferences/app relaunch. Inspect the controls at 250% and with `LANGUAGE=de`. Use the existing core `morning_summary_at` policy tests plus GTK UI tests to cover eligibility, once-daily claiming, next-launch catch-up, and reminder independence without waiting for a real future time. The Linux client currently calls `gio::Application::send_notification` directly, so observe one actual isolated system notification and a denial/failure outcome when feasible; do not invent an adapter solely for verification. If system delivery or denial cannot be observed, retain Implemented and record the unavailable gate.

### 5.8 Defect and evidence discipline

Record exactly what was observed, what automation supported it, and any environment-limited checks. Do not promote a feature to Verified without the full native scenario.

## Task 6: Update localization, ledgers, and run complete regression verification

- [x] 6.1 Regenerate and validate gettext assets with the exact repository commands.
- [x] 6.2 Run every focused/shared/macOS boundary verification command.
- [x] 6.3 Run the full Linux and portable Rust regression suites.
- [x] 6.4 Run strict schema, formatting, link, accessibility, and diff checks.
- [x] 6.5 Update CHANGELOG and documentation from observed evidence.
- [x] 6.6 Update platform ledgers truthfully and perform one final clean-diff inspection.

**Files:**

- Modify: `po/POTFILES.in` if new source/UI files require it
- Modify: `po/momentum.pot`
- Modify: `po/de.po`
- Modify: `docs/FEATURES.md`
- Modify: `docs/PROGRESS.md`
- Modify: `docs/BUGS.md` only for actual defects
- Modify: `docs/TESTING.md` if new test entry points need documenting
- Modify: `docs/MVP.md` only where the native Linux behavior description is now stale
- Modify: `CHANGELOG.md`

### 6.1 Validate localization coverage

Add every new translatable Rust/Blueprint source to `po/POTFILES.in`, then run the documented regeneration inside the GNOME SDK:

```sh
flatpak run --devel --share=network --filesystem=home --command=bash org.gnome.Sdk//50 -c '
  export PATH=/usr/lib/sdk/rust-stable/bin:$PATH &&
  if [ -d target-sdk/pot-build/meson-private ]; then
    meson setup target-sdk/pot-build --reconfigure
  else
    meson setup target-sdk/pot-build
  fi &&
  ninja -C target-sdk/pot-build momentum-pot momentum-update-po'
build-aux/check-potfiles.sh
for f in po/*.po; do msgfmt --check --statistics -o /dev/null "$f"; done
diff <(grep -v '^#' po/LINGUAS | sort) <(ls po/*.po | xargs -n1 basename | sed 's/\.po$//' | sort)
```

Add reviewed German translations for the new source strings, including both plural forms. Expected: POTFILES completeness, every catalog, and LINGUAS all pass.

### 6.2 Run focused cross-platform contract verification

Run:

```sh
cargo test -p momentum-core task_count -- --nocapture
cargo test -p mo open -- --nocapture
./build-aux/test.sh -p momentum typography -- --nocapture
./build-aux/test.sh -p momentum background_status -- --nocapture
macos/scripts/build-core.sh --debug
swift test --package-path macos/Packages/MomentumKit
swift macos/scripts/a11y-audit.swift --self-test
swift macos/scripts/check-localization.swift
```

Run the binding build before Swift tests so the shared count API is current. If the Apple toolchain is unavailable on Linux, also run `cargo build -p momentum-ffi --features momentum-core/ffi` to verify the Rust boundary, record the exact unavailable Apple command/error, and do not claim the Swift consumer ran.

### 6.3 Run the full Linux regression suite

Run:

```sh
./build-aux/test.sh
cargo test --workspace --exclude momentum --all-features --all-targets
```

Expected: every workspace test passes. Record counts and pre-existing warnings separately from failures.

### 6.4 Run static checks and inspect all changes

Run:

```sh
cargo fmt -p momentum -p mo -p momentum-core -p momentum-ffi -p uniffi-bindgen \
  -p sp-model -p sp-oplog -p sp-store -p sp-sync -p sp-p2p -- --check
SCHEMA_CHECK=$(mktemp -d -p /var/tmp momentum-schema.XXXXXX)
sed 's/@app-id@/io.github.dan_hart.Momentum.Devel/g; s/@gettext-package@/momentum/g' \
  data/io.github.dan_hart.Momentum.gschema.xml.in > \
  "$SCHEMA_CHECK/io.github.dan_hart.Momentum.Devel.gschema.xml"
glib-compile-schemas --strict "$SCHEMA_CHECK"
build-aux/check-potfiles.sh
for f in po/*.po; do msgfmt --check --statistics -o /dev/null "$f"; done
git diff --check
git status --short
git diff --stat
```

For every Markdown file reported by `git diff --name-only -- '*.md'`, check each relative link target and fragment against the worktree; use `lychee --offline` if installed, otherwise use a read-only Python link parser and record its command/output. Re-run `build-aux/a11y-dump.py` against the isolated native app if AT-SPI is exposed. Inspect the entire diff for secrets, real user data, generated build output, accidental lockfile drift, and scope creep. `Cargo.lock` must be unchanged unless a deliberate dependency change was required; the baseline's local LibreSync 0.6.1 path resolution was restored to the repository's committed 0.6.0 lock entry before implementation.

### 6.5 Update CHANGELOG and documentation

Add a concise unreleased Linux parity entry to `CHANGELOG.md`. Update `README.md`, `docs/MVP.md`, `docs/TESTING.md`, `docs/LINUX-AUTOMATION.md`, and `docs/PROGRESS.md` only from the implemented behavior and observed command results. Include the branch/worktree, exact test totals, warnings, native acceptance evidence, and remaining environmental limits.

### 6.6 Update the ledgers truthfully

- Mark Linux F-022, F-031, and F-032 Implemented when source/build/test evidence supports them.
- Mark them Verified only for native acceptance scenarios actually completed.
- Update F-027, F-028, F-037, and F-038 evidence with the dated Linux native results; retain Implemented where any required native scenario remained environment-limited.
- Record the branch/worktree, exact commands, test totals, warnings, and remaining follow-ups in `docs/PROGRESS.md`.
- Leave iOS and Android statuses unchanged.

Run `git diff --check` once more. Do not commit or push; leave the completed, verified local changes in the worktree for review.
