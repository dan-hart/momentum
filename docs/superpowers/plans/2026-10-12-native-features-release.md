# Momentum Native Features Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans. Follow test-driven-development and verification-before-completion. Steps use checkbox syntax for tracking.

**Goal:** Deliver the approved native fixes, recurrence previews, overdue/copy feedback, portable settings transfer/sync and compatible Momentum cleanup, then verify and release 0.4.10 including internal TestFlight.

**Architecture:** Rust owns projection, validation, settings convergence and verified folder copying. Native layers own presentation and private preference adapters. Existing task/backup/sync formats and primary-worktree changes remain intact; new portable settings and consent metadata live outside task snapshots.

**Tech Stack:** Rust/UniFFI, GTK4/libadwaita/GSettings, SwiftUI/AppKit/UIKit/UserDefaults, Kotlin/Compose/DataStore, existing Nextcloud and LibreSync transports.

The approved [design](../specs/2026-10-12-native-features-release-design.md) is the acceptance contract. Isolated branch `task/native-features-release`, base `69682872`. Only the controller commits, integrates, pushes, tags or distributes. Maintain focused file ownership and review specification compliance before code quality.

## Task 1 Shared row contracts

**Files:** `crates/momentum-core/src/{types.rs,listing.rs,notifications.rs,engine.rs,tests.rs,lib.rs}`; new focused recurrence projection module/tests if necessary.

- [x] Add a failing engine-listing regression: a daily repeat due tomorrow appears in Upcoming without changing task count, cursor, pending IDs or persisted store bytes.
- [x] Run `cargo test -p momentum-core upcoming -- --nocapture`; confirm the new assertion fails because no future occurrence appears.
- [x] Extract/reuse the existing notification recurrence projection. Add explicit projected-occurrence metadata to rows; never represent an unpersisted preview as an actionable ordinary task. Deduplicate live/archived/moved occurrence IDs and retain horizon/sort behavior.
- [x] Add core-derived overdue status using the existing effective-date rule, with regressions for timed precedence, open/completed/archive/subtask and rollover behavior.
- [x] Run `cargo test -p momentum-core` and targeted model recurrence tests. Check ffi-feature compilation. Announce the exact public row contract to frontend workers before they integrate it.

## Task 2 Portable settings core and transports

**Files:** new `crates/momentum-core/src/portable_settings.rs` and focused tests; `engine.rs`, `lib.rs`, `types.rs`; focused additions in `crates/momentum-sync/src/{lib.rs,mock_dav.rs,feature_tests.rs}`, `crates/momentum-p2p/src/{lib.rs,feature_tests.rs}`, and core P2P/transport tests.

- [x] Write failing validation/round-trip tests for the nine allowed portable values, unknown fields, invalid known fields, incompatible versions, missing keys and secret sentinels.
- [x] Implement a versioned values document, durable device-local settings state/policy, and per-key causal metadata with deterministic concurrent resolution. Persist before publishing changes; failed persistence retains the previous state. Consent remains local and defaults off.
- [x] Expose a small generated-FFI API for native adapters: read/stage validated portable values, settings-only import/export, optional settings on backup export/restore, and explicit sync policy/bootstrap choice. Existing APIs preserve task-only behavior. Validate every included section before mutation and preserve task restoration recovery.
- [x] Add failing real-loopback two-client tests: off devices never publish/apply settings; independent edits merge; conflicts converge; a fresh enabled device bootstraps; disable/re-enable and provider switching preserve pending values. Confirm task snapshots never contain new settings or consent.
- [x] Implement a separately versioned Nextcloud settings payload using existing encryption/cancellation/budget/overwrite guards. Explicitly negotiate/version the LibreSync settings channel so old peers cannot silently acknowledge unsupported settings. Integrate only with the selected provider and preserve offline task edits on settings failure.
- [x] Run `cargo test --workspace --exclude momentum --all-features --all-targets`. Self-review transport races, retry receipts, rollback and secret exclusion. Announce exact native APIs and JSON keys.

## Task 3 Checked Nextcloud folder migration

**Files:** sync transport module/tests and core engine bridge; native connection adapters are handled in Tasks 4–6.

- [x] Add loopback failures for nonempty destination, source mutation during copy, short upload/readback, cancellation, missing folders and repeated successful migration.
- [x] Copy to `momentum` only on explicit request. Create target directory safely, preserve encryption bytes and source, require guarded destination creation/readback and unchanged source before success. Reject a conflicting target rather than overwriting it. Native saved folder changes only after successful verification; pending local task/settings edits survive.
- [x] Preserve configured connections with implicit legacy defaults before changing new-connection defaults. Keep the necessary legacy literal isolated to that compatibility adapter.
- [x] Run migration/transport regressions and report the public result contract without logging credentials/private endpoints.

## Task 4 Linux native changes

**Files:** `crates/app/src/{task_form.rs,prefs.rs,quick_add.rs,window.rs}`, tests under `crates/app/src/tests/`, Blueprint resources, GSettings schema, gettext resources.

- [x] Add a mapped GTK regression selecting a nonfirst project (including duplicate titles), checking actual display/color and persisted project ID; observe failure, then bind factories by model-object identity.
- [x] Make Preferences useful at the approved preferred size, with supported independent presentation if parent clamping defeats the fix. Verify compact scrolling/large text and preserved window state.
- [x] Replace dense quick-add placeholder/help with concise entry and native tooltip/accessibility guidance. Preserve parsing and destination context.
- [x] Add editor-local Notes copied feedback after actual clipboard copy; test exact Unicode/multiline contents and visible feedback.
- [x] Render shared overdue status and explicit recurrence previews with Edit Repeat, suppressing ordinary task actions for previews.
- [x] Implement the portable preference adapter, independent unchecked transfer controls, settings-only transfer, sync opt-in/bootstrap/recovery and checked folder-migration UI. Test opt-in matrix and excluded-key sentinels using memory GSettings.
- [ ] Run supported GNOME SDK/Broadway UI suite; macOS typechecking is a preliminary compatibility check only. Inspect actual allocations/accessibility and EN/DE large-text layouts with isolated data.

## Task 5 Apple native changes

**Files:** shared `macos/Packages/MomentumKit` adapters/tests/resources; `ios/Packages/MomentumMobile` worker/model/tests; `ios/Momentum/{Tasks,Settings}`, `ios/MomentumShare`, `ios/MomentumViewTests`; corresponding macOS views/resources.

- [x] Write share-inbox/worker regressions for captured Today, delayed midnight drain, timezone change, old queued payloads, retry/relaunch/dedup. Add optional captured `plannedDay`, import it through existing Rust rules and update destination wording.
- [x] Reproduce first valid-project navigation with controlled snapshot completion and compact-column changes before fixing. Add rapid-switch, revision, width/tab transition and deleted-project regressions. Preserve route ownership and reject stale results. Verify Mac synchronous first-navigation/fallback rather than applying an iOS-specific race fix there.
- [x] Add editor-visible copy feedback and native success haptics after successful full copy, respecting settings. Verify failure silence, repeated deadlines and accessible large text.
- [x] Integrate overdue/preview contracts and preview-only Edit Repeat actions. Fix every generated TaskRow fixture after regenerating bindings through build tooling.
- [x] Add shared portable UserDefaults adapter and native transfer/sync/folder controls on both platforms. Keep secrets, services, device controls and consent local. Add failure/rollback/default/disabled-path regressions.
- [x] Run generated core preparation, compiled-catalog MomentumKit/MomentumMobile tests, iOS hosted view/transport lanes on iOS 26.5/27, shipping Mac/iOS builds, and isolated native interaction/layout checks. No XCUITest or ViewInspector additions.

## Task 6 Android native changes

**Files:** `android/app/src/main/java/com/codedbydan/momentum/{data,ui,platform}`, JVM/instrumentation tests and EN/DE resources.

- [x] Add editor-local success Snackbar/toast and one system-respecting native haptic after PrivateClipboard succeeds. Preserve large-note provider/failure behavior. Test exact payloads, disabled/failure silence and feedback in the modal editor.
- [x] Add first-project navigation/rapid switch/deleted fallback regressions. Change navigation only for reproduced defects.
- [x] Integrate shared overdue and future-preview contracts, allowing Edit Repeat and excluding task mutation controls on previews.
- [x] Map portable keys/enums through Rust validation, preserving DataStore device values. Add independent unchecked settings transfer controls, settings-only SAF documents, opt-in selected-provider sync/bootstrap/recovery and migration UI that saves the target folder only after success.
- [x] Generate Kotlin bindings through existing tooling, run JVM/tooling tests, native API26/API37 instrumentation, release lint/build and relevant minified runtime/packaging checks with isolated stores.

## Task 7 Ownership cleanup and documentation

**Files:** all five task-foundation crate paths/manifests/imports, workspace/lockfile, CI/scripts, URL registrations/parsers/tests, tracked documentation/catalogs/metainfo/Homebrew; preserve NOTICE and mandatory source disclosures.

- [x] After source workers finish, rename crate directories/packages/imports together. Update tests/scripts/CI commands and regenerate bindings; do not hand-edit generated files.
- [x] Remove compatibility-only branded URL aliases; retain and regression-test `momentum://` entry points.
- [x] Scan every tracked text file for product branding and old crate names. Update historical documentation without losing acceptance evidence. Retain required legal attribution and the approved narrow existing-folder compatibility literals.
- [x] Update FEATURES/BUGS/PROGRESS with separate platform status, current revision/dirty state, actual tests, native limits and remaining gates. Allocate new IDs centrally after checking maxima. Update relevant specs and CHANGELOG Unreleased.
- [x] Run formatting, gettext/catalog checks, relative-document links, sound provenance and version alignment; review the complete diff for accidental personal/credential data.

## Task 8 Candidate verification and release

**Files:** existing `.github/workflows/{ci,macos,android,release}.yml`, release/version tooling, native signing outputs outside tracked source.

- [x] Complete specification compliance and independent code-quality reviews for shared and native changes; fix findings with regressions and rerun affected gates.
- [ ] Run both Rust workspace checks, CLI/FFI compatibility, Apple package/build/native lanes, Android native/release tooling, and local Mac nearby integration required by RELEASING.
- [ ] Run the existing nonpublishing CI/macOS/Android gates on the candidate task branch before main integration. Their push triggers now include task/** so both Linux architectures are checked before main. Use the exact source SHA and do not publish release artifacts from verification runs.
- [ ] With all applicable candidate gates passing, fetch main, resolve any concurrent changes without touching the primary checkout, verify the integrated source and bump with `build-aux/release.py bump 0.4.10 --summary ...`. Inspect all version/build copies and release notes before committing/pushing main.
- [ ] Wait for exact release-head CI. Push annotated `v0.4.10`, observe Release completion, verify asset inventory/source provenance/checksums and report optional channel skips accurately.
- [ ] Build a signed iOS archive/IPA at the verified release revision. Check both IDs, 0.4.10/410, signatures, profiles, App Group and privacy manifests. Resolve live App Store Connect access (initial read-only lookups stalled) before upload. Use existing app 6821523171/internal group, upload and verify VALID processing plus internal group access; keep secrets/receipts outside the repo.
- [ ] Record release/TestFlight evidence and remaining physical/manual scope, preserve the primary branch/HEAD/file hashes, and hand off links without calling an unverified platform Verified.
