<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# iOS Navigation and Interaction Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the ten approved iPhone/iPad navigation, creation, settings, interaction, and date-grouping regressions with fast unit coverage and simulator evidence.

**Architecture:** Keep task semantics in `momentum-core`, add small injectable mobile presentation policies in `MomentumMobile`, and compose them through native SwiftUI. Use a root-owned editor handoff, an adaptive `NavigationSplitView` workspace, and render tests through `UIHostingController`; add no dependency, XCUI target, or ViewInspector use.

**Tech Stack:** Rust, UniFFI, Swift 6, SwiftUI, Observation, Swift Testing, XCTest, UIKit hosting, Xcode 27, iOS 26.5/27 simulators.

**Local policy:** Do not commit, push, install on a physical device, or discard unrelated dirty changes. Replace commit steps with local diff/test checkpoints.

---

### Task 1: Correct Morning & Night grouping outside day views (B-082)

**Files:**
- Modify: `crates/momentum-core/src/listing.rs`
- Modify: `crates/momentum-core/src/tests.rs`

- [ ] Add `tests::morning_night_is_plain_outside_day_views` with explicit Upcoming, project, tag, and search assertions for `group == nil`, one preserved plain section, and due-day metadata. Expand preservation assertions for Today/Morning/Evening grouping, completed sections, parent/subtask families, ordering, and Undo.
- [ ] Run `cargo test -p momentum-core --lib tests::morning_night_is_plain_outside_day_views -- --exact`; expect FAIL because current code emits `TaskGroup::Today`.
- [ ] Run `cargo test -p momentum-core --lib tests::exclusive_upcoming_groups_span_dates_without_hiding_due_days -- --exact` to capture the old contract before changing it.
- [ ] Make `task_group` return `None` for Morning & Night outside Today-style views and make `group_listing` preserve the original plain section when grouping returns `None`.
- [ ] Run both exact tests plus `cargo test -p momentum-core --lib morning_night`; expect the new contract and existing slot behavior to pass.
- [ ] Review the focused diff and leave it uncommitted.

### Task 2: Return created task identity and sequence the editor handoff (B-077)

**Files:**
- Modify: `ios/Packages/MomentumMobile/Sources/MomentumMobile/EngineWorker+Editing.swift`
- Modify: `ios/Packages/MomentumMobile/Sources/MomentumMobile/EngineWorker+Organization.swift`
- Modify: `ios/Packages/MomentumMobile/Sources/MomentumMobile/QuickAddState.swift`
- Modify: `ios/Packages/MomentumMobile/Tests/MomentumMobileTests/TaskEditingTests.swift`
- Modify: `ios/Packages/MomentumMobile/Tests/MomentumMobileTests/QuickAddStateTests.swift`
- Modify: `ios/Momentum/App/MobileAppModel.swift`
- Modify: `ios/Momentum/App/RootView.swift`
- Modify: `ios/Momentum/Tasks/QuickAddSheet.swift`
- Modify: `ios/MomentumViewTests/TaskMutationIntegrationTests.swift`

- [ ] Add failing package tests for atomic single add and multiline import with last-created ID under concurrent actor callers. Preserve More Details behavior and unchanged/persistence-failure draft retention.
- [ ] Add failing `TaskMutationIntegrationTests/testCreatedTaskEditorPrecedesQueuedNotification` and `testMultilineQuickAddRoutesLastCreatedTask` tests.
- [ ] Run `python3 ios/scripts/test.py fast --filter TaskEditingTests` and `python3 ios/scripts/test.py fast --filter QuickAddStateTests`; expect compilation/assertion failure for the missing result identity.
- [ ] Run `python3 ios/scripts/test.py views --destination 'platform=iOS Simulator,id=6C0B4007-CD8B-49DD-BF15-9A887BC80D9D' --filter TaskMutationIntegrationTests/testCreatedTaskEditorPrecedesQueuedNotification` and the same command with `--filter TaskMutationIntegrationTests/testMultilineQuickAddRoutesLastCreatedTask`; expect FAIL because no pending editor route exists.
- [ ] Make `QuickAddState.submit` return a typed success carrying the ID. Add actor-isolated worker helpers that reuse existing add/import mutations and capture `lastAddedId()` before actor re-entry.
- [ ] Add root-owned pending editor state. Quick Add stores the ID and dismisses; `RootView` presents `TaskEditor` from `onDismiss`, then resumes queued notification routing only after the editor closes.
- [ ] Re-run the two `fast` commands and the two exact `views` commands above; expect PASS and exactly one sheet owner at each boundary.
- [ ] Review the focused diff and leave it uncommitted.

### Task 3: Build the adaptive leading sidebar workspace (B-076)

**Files:**
- Create: `ios/Packages/MomentumMobile/Sources/MomentumMobile/TaskWorkspacePolicy.swift`
- Create: `ios/Packages/MomentumMobile/Tests/MomentumMobileTests/TaskWorkspacePolicyTests.swift`
- Create: `ios/Momentum/Tasks/TaskWorkspace.swift`
- Modify: `ios/Momentum/App/RootView.swift`
- Modify: `ios/Momentum/Tasks/ListPicker.swift`
- Modify: `ios/Momentum/Tasks/TaskScreen.swift`
- Modify: `ios/MomentumViewTests/ViewRenderingTests.swift`

- [ ] Add `TaskWorkspaceState` beside the layout policy. Add failing tests for regular-wide landscape showing both columns, compact/narrow layouts preferring detail, tab-switch and rotation persistence, compact selection returning to detail, and separate fallback cases for disappearing Morning, Evening, project, and tag contexts.
- [ ] Run `python3 ios/scripts/test.py fast --filter TaskWorkspacePolicyTests`; expect compilation failure because the policy/state do not exist.
- [ ] Add failing `ViewRenderingTests/testTaskWorkspaceCollapsesAtPhonePortraitWidth` and `testTaskWorkspaceShowsBothColumnsAtCapableLandscapeWidth`. Run `python3 ios/scripts/test.py views --destination 'platform=iOS Simulator,id=6C0B4007-CD8B-49DD-BF15-9A887BC80D9D' --filter ViewRenderingTests/testTaskWorkspaceCollapsesAtPhonePortraitWidth` and the same command with `--filter ViewRenderingTests/testTaskWorkspaceShowsBothColumnsAtCapableLandscapeWidth`; expect missing workspace/incorrect presentation failures.
- [ ] Implement the width/size-class policy and state owner without device-name checks.
- [ ] Compose Today through `TaskWorkspace`: sidebar as primary column, selected `TaskScreen` as detail, and `.detail` as preferred compact column.
- [ ] Convert `ListPicker` to selection content and remove the old trailing pushed destination from `TaskScreen`.
- [ ] Re-run `python3 ios/scripts/test.py fast --filter TaskWorkspacePolicyTests` and both exact `views` commands above; expect PASS. Build both simulator runtimes in Task 7.
- [ ] Review the focused diff and leave it uncommitted.

### Task 4: Stabilize first load and selection affordances (B-075/B-081)

**Files:**
- Modify: `ios/Momentum/Tasks/TaskScreen.swift`
- Modify: `ios/Momentum/Design/FloatingAddTaskButton.swift`
- Modify: `ios/MomentumViewTests/HostingFixture.swift`
- Modify: `ios/MomentumViewTests/ViewRenderingTests.swift`

- [ ] Add failing `ViewRenderingTests/testInitialTaskSnapshotDoesNotAnimate`; initial snapshot must use no animation and later refreshes may animate only when Reduce Motion is off.
- [ ] Extend `HostingFixture` with a public-UIKit mounted lifecycle/accessibility helper. Do not use private reflection or ViewInspector.
- [ ] Add failing `ViewRenderingTests/testSelectionRowOmitsCompletionControl` and `testBrowsingRowKeepsCompletionControl`.
- [ ] Run three commands using `python3 ios/scripts/test.py views --destination 'platform=iOS Simulator,id=6C0B4007-CD8B-49DD-BF15-9A887BC80D9D'`, with filters `ViewRenderingTests/testInitialTaskSnapshotDoesNotAnimate`, `ViewRenderingTests/testSelectionRowOmitsCompletionControl`, and `ViewRenderingTests/testBrowsingRowKeepsCompletionControl`; expect FAIL because the first snapshot animates and selection mode exposes the completion control.
- [ ] Assign the first snapshot without animation and keep the floating button in a stable inset independent of snapshot content.
- [ ] Add an explicit row presentation mode and hide the completion circle in selection mode.
- [ ] Re-run those three exact commands, then run `python3 ios/scripts/test.py views --destination 'platform=iOS Simulator,id=6C0B4007-CD8B-49DD-BF15-9A887BC80D9D'`; expect PASS.
- [ ] Review the focused diff and leave it uncommitted.

### Task 5: Fix Sync lifecycle, iPad shortcut visibility, and accent selection (B-078/B-079/B-080)

**Files:**
- Create: `ios/Packages/MomentumMobile/Sources/MomentumMobile/MobilePlatformCapabilities.swift`
- Create: `ios/Packages/MomentumMobile/Tests/MomentumMobileTests/MobilePlatformCapabilitiesTests.swift`
- Modify: `ios/Momentum/Settings/SyncSettings.swift`
- Modify: `ios/Momentum/Settings/SettingsScreen.swift`
- Modify: `ios/Momentum/Settings/AccentSettings.swift`
- Modify: `ios/Momentum/App/RootView.swift`
- Modify: `ios/Momentum/App/MomentumApp.swift`
- Modify: `ios/Momentum/App/MobileKeyboardCommands.swift`
- Modify: `ios/Momentum/Tasks/TaskScreen.swift`
- Modify: `ios/MomentumViewTests/HostingFixture.swift`
- Modify: `ios/MomentumViewTests/SyncSettingsIntegrationTests.swift`
- Modify: `ios/MomentumViewTests/ViewRenderingTests.swift`

- [ ] Keep `MobilePlatformCapabilities` platform-neutral and inject an app-layer `isPad` Boolean derived from UIKit. Add failing tests proving shortcut discovery is visible only when that value is true.
- [ ] Add failing `SyncSettingsIntegrationTests/testSyncScreenPresentationDoesNotOwnStateLoading` using mounted hosting plus persistence/status counters.
- [ ] Add failing `ViewRenderingTests/testAccentSelectionCheckmarkFollowsBinding` that changes the bound ID while mounted and inspects public UIKit accessibility metadata.
- [ ] Run `python3 ios/scripts/test.py fast --filter MobilePlatformCapabilitiesTests`; expect compilation failure because the capability type is missing.
- [ ] Run `python3 ios/scripts/test.py views --destination 'platform=iOS Simulator,id=6C0B4007-CD8B-49DD-BF15-9A887BC80D9D' --filter SyncSettingsIntegrationTests/testSyncScreenPresentationDoesNotOwnStateLoading` and the same command with `--filter ViewRenderingTests/testAccentSelectionCheckmarkFollowsBinding`; expect lifecycle/checkmark assertion failures.
- [ ] Remove the duplicate `SyncSettings.task`; keep foreground ownership in `MobileAppModel`.
- [ ] Gate shortcut Settings/help/commands/fallbacks through the injected iPad capability, including `MobileKeyboardCommands`.
- [ ] Extract an accent row whose trailing checkmark and selected trait derive directly from the resolved selected ID.
- [ ] Re-run the exact `fast` and two `views` commands above; expect PASS. Inspect Settings on iPhone and iPad simulators in Task 7.
- [ ] Review the focused diff and leave it uncommitted.

### Task 6: Add restrained motion and haptic feedback (F-019/F-020)

**Files:**
- Modify: `ios/Momentum/App/MobileAppModel.swift`
- Modify: `ios/Momentum/App/RootView.swift`
- Modify: `ios/Momentum/Design/FloatingAddTaskButton.swift`
- Modify: `ios/Momentum/Settings/AccentSettings.swift`
- Modify: `ios/Momentum/Tasks/TaskScreen.swift`
- Modify: `ios/Momentum/Tasks/TaskWorkspace.swift`
- Modify: `ios/MomentumViewTests/TaskMutationIntegrationTests.swift`
- Modify: `ios/MomentumViewTests/ViewRenderingTests.swift`

- [ ] Add failing `TaskMutationIntegrationTests/testInteractionFeedbackTriggers` for opening Add Task, successful creation, list/accent selection, entering and leaving selection, and successful moves; retain the disabled-haptics policy.
- [ ] Add failing `ViewRenderingTests/testInteractionMotionRespectsReduceMotion` before changing production motion.
- [ ] Run `python3 ios/scripts/test.py views --destination 'platform=iOS Simulator,id=6C0B4007-CD8B-49DD-BF15-9A887BC80D9D' --filter TaskMutationIntegrationTests/testInteractionFeedbackTriggers` and the same command with `--filter ViewRenderingTests/testInteractionMotionRespectsReduceMotion`; expect FAIL because the triggers and reduced-motion presentation do not exist.
- [ ] Add model counters/triggers and apply `.sensoryFeedback` at the root or smallest stable owner.
- [ ] Add 140–220 ms scale/opacity/symbol transitions to the floating button, selected checkmarks, and selection controls; gate nonessential movement with Reduce Motion.
- [ ] Re-run both exact `views` commands above; expect PASS.
- [ ] Inspect normal and Reduce Motion behavior in the simulator in Task 7.
- [ ] Review the focused diff and leave it uncommitted.

### Task 7: Full verification and living records

**Files:**
- Modify: `docs/FEATURES.md`
- Modify: `docs/BUGS.md`
- Modify: `docs/PROGRESS.md`
- Modify: `docs/IOS-PARITY-CHECKLIST.md`

- [ ] Run `xcrun simctl list devices available`; record replacements if the named QA simulators are unavailable. Expected current IDs: iOS 26.5 `6C0B4007-CD8B-49DD-BF15-9A887BC80D9D`, iOS 27 `681FF376-73A1-4EFC-807E-E858C7B5B7C2`.
- [ ] Run `python3 ios/scripts/test.py prepare`; this rebuilds the Apple core, regenerates UniFFI Swift, and refreshes the XCFramework.
- [ ] Run `python3 ios/scripts/test.py unit --destination 'platform=iOS Simulator,id=6C0B4007-CD8B-49DD-BF15-9A887BC80D9D'` and the same command with `681FF376-73A1-4EFC-807E-E858C7B5B7C2`; expect all selected tests to pass with only documented opt-in skips.
- [ ] Run `python3 -m unittest ios/scripts/tests/test_runner.py`; expect PASS and no XCUI target/source.
- [ ] Run `cargo test -p mo --test cli`, `cargo test --workspace --exclude momentum`, `cargo test --workspace --exclude momentum --all-features --all-targets`, `swift test --package-path macos/Packages/MomentumKit`, `macos/scripts/build-core.sh --debug`, and `build-aux/test.sh`; distinguish an unsupported Linux/Flatpak host from a pass.
- [ ] Verify generated Swift/XCFramework consumption through `swift test --package-path ios/Packages/MomentumMobile`, `swift test --package-path macos/Packages/MomentumKit`, and both simulator app builds.
- [ ] Run `xcodegen generate --spec ios/project.yml`, then `xcodebuild -project ios/Momentum.xcodeproj -scheme Momentum -configuration Debug -destination 'platform=iOS Simulator,id=6C0B4007-CD8B-49DD-BF15-9A887BC80D9D' CODE_SIGNING_ALLOWED=NO build` and the same command with destination `681FF376-73A1-4EFC-807E-E858C7B5B7C2`; expect `BUILD SUCCEEDED`.
- [ ] Inspect all ten flows with isolated/demo data in portrait/landscape, light/dark, Reduce Motion, and representative Dynamic Type. Do not use XCUI or a physical device.
- [ ] Add B-075–B-082 and update F-003–F-006/F-010/F-018–F-020/F-023/F-025/F-028/F-037/F-039 with exact dirty-state evidence and remaining limits.
- [ ] Run `git diff --check`, inspect all changed files, and report actual pass/fail/skip evidence without committing.
