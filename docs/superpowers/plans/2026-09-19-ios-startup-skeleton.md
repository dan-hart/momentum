# iOS Startup Skeleton Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Implement F-042 by replacing only the root Today's first-load spinner with an adaptive skeleton and shimmer that redacts the real Today and four-tab hierarchy.

**Architecture:** `RootView` owns a latched startup phase: loading Today, ready, or permanently superseded by a cold Spotlight/reminder route. The root Today `TaskScreen` alone renders `TodaySkeletonView` and reports its first valid snapshot through `TaskWorkspace`; every other task screen retains the existing progress presentation. Root redacts, disables, and accessibility-hides the real tab subtree while an unredacted sibling owns the sole loading status.

**Tech Stack:** Swift 6, SwiftUI `TimelineView`, UIKit-hosted XCTest, Swift Testing, XcodeGen

**Constraints:** Simulator-only. Do not add XCUITest/XCUI, ViewInspector, a physical-device claim, dependencies, commits, pushes, merges, or cleanup of unrelated dirty work.

---

### Task 1: RED/GREEN the motion policy and skeleton component

**Files:**
- Create: `ios/Momentum/Design/TodaySkeletonView.swift`
- Modify: `ios/MomentumViewTests/ViewRenderingTests.swift`

- [x] **RED:** Add `testStartupSkeletonMotionPolicyCapsAndPausesAnimation`. Assert shimmer is enabled only while startup is incomplete, Reduce Motion is off, Low Power Mode is off, and the scene is active; assert `minimumInterval == 1.0 / 30.0`, standard fade duration is 0.2 seconds, and Reduce Motion fade duration is zero.
- [x] **RED:** Add `testTodaySkeletonRendersAdaptiveSystemPlaceholders`. Render light/dark, standard/increased contrast, 320-point AX5, and ordinary 390-point layouts. Require identifier `today-loading-skeleton`, a section-heading placeholder, exactly five rows, and for every row a completion circle plus primary and secondary bars. Require nonblank output without horizontal overflow.
- [x] Run both exact iOS 27 filters and observe missing-symbol failures:
  `python3 ios/scripts/test.py views --destination 'platform=iOS Simulator,id=681FF376-73A1-4EFC-807E-E858C7B5B7C2' --filter ViewRenderingTests/testStartupSkeletonMotionPolicyCapsAndPausesAnimation`
  `python3 ios/scripts/test.py views --destination 'platform=iOS Simulator,id=681FF376-73A1-4EFC-807E-E858C7B5B7C2' --filter ViewRenderingTests/testTodaySkeletonRendersAdaptiveSystemPlaceholders`
- [x] **GREEN:** Add `StartupSkeletonMotionPolicy` with `isComplete`, `reduceMotion`, `lowPowerMode`, and `sceneIsActive` inputs. Use `TimelineView(.animation(minimumInterval: 1.0 / 30.0, paused: !policy.shimmers))`.
- [x] **GREEN:** Build the semantic-system-fill Today skeleton: one section heading and five inset-grouped rows, each with a completion circle and two bars. Apply one opacity/translation shimmer overlay. Expose zero accessibility elements.
- [x] Rerun both exact filters and require pass.

### Task 2: RED/GREEN the one-shot root Today lifecycle and accessibility ownership

**Files:**
- Modify: `ios/Momentum/Tasks/TaskScreen.swift`
- Modify: `ios/Momentum/Tasks/TaskWorkspace.swift`
- Modify: `ios/Momentum/App/RootView.swift`
- Modify: `ios/MomentumViewTests/ViewRenderingTests.swift`
- Modify: `ios/MomentumViewTests/HostingFixture.swift`
- Modify: `ios/Momentum/Resources/Localizable.xcstrings`

- [x] **RED:** Add `testRootStartupSkeletonOwnsLoadingAccessibilityAndKeepsRealTabsMounted`. Mount full production RootView behind a controlled first-snapshot gate. Require all four real tab identifiers mounted, require the TabView subtree and toolbar/Add/placeholders absent from accessibility, and require exactly one `Loading Momentum` status until readiness.
- [x] **RED:** Add `testRootStartupSkeletonTransitionsOnceAfterFirstTodaySnapshot`. Require the skeleton before release, unchanged Add Task frame, restored task/tab accessibility afterward, and no skeleton during a later refresh.
- [x] **RED:** Add `testNonTodayTaskScreensNeverUseStartupSkeleton`. Mount Upcoming, Search, Archive, project and tag before first snapshots and require ordinary `Loading tasks` with no Today skeleton.
- [x] Run `python3 ios/scripts/test.py views --destination 'platform=iOS Simulator,id=681FF376-73A1-4EFC-807E-E858C7B5B7C2' --filter ViewRenderingTests` and observe lifecycle/accessibility failures.
- [x] **GREEN:** Add default `.progress` and opt-in `.todaySkeleton` loading modes. Invoke the readiness callback once only after assigning a valid first `.today` snapshot.
- [x] **GREEN:** Thread a defaulted `snapshotLoader` seam from `RootView` through `TaskWorkspace` into the root `TaskScreen`, preserving the production loader by default and allowing the hosted full-root tests to hold/release the first snapshot deterministically.
- [x] **GREEN:** Forward readiness through root TaskWorkspace. Add `RootStartupPhase`; keep the real tabs mounted but redacted, disabled, and accessibility-hidden while loading. Expose one sibling status. Remove redaction with a 0.2-second opacity transition, or immediately under Reduce Motion. When phase is `.superseded`, pass `.progress` so later navigation to Today cannot replay the startup skeleton.
- [x] **GREEN:** Add English `Loading Momentum` and German `Momentum wird geladen`.
- [x] Rerun `ViewRenderingTests` and require pass.

### Task 3: RED/GREEN cold-route and store-failure precedence

**Files:**
- Modify: `ios/Momentum/App/RootView.swift`
- Modify: `ios/MomentumViewTests/AutomationIntegrationTests.swift`
- Modify: `ios/MomentumViewTests/StoreRecoveryIntegrationTests.swift`

- [x] **RED:** Add `testColdSpotlightPermanentlySuppressesStartupSkeletonWithoutReplay`, `testColdReminderPermanentlySuppressesStartupSkeletonWithoutReplay`, and `testSummaryRouteKeepsTodayStartupEligible`. Spotlight/Search and reminder/Open Task must latch `.superseded`, show once, and never replay the skeleton on Today. Summary-on-Today stays eligible. URL mutations leave the phase and pending route unchanged.
- [x] Run `python3 ios/scripts/test.py views --destination 'platform=iOS Simulator,id=681FF376-73A1-4EFC-807E-E858C7B5B7C2' --filter AutomationIntegrationTests` and observe failures.
- [x] **GREEN:** While loading, latch `.superseded` when a system route moves away from Today or a reminder/Open Task editor appears. Never transition back to loading. Keep summary eligible and URL mutations independent.
- [x] Rerun AutomationIntegrationTests and require pass.
- [x] **RED:** Add `testStartupFailureReplacesSkeletonAndRetryReturnsThroughReadyState`. Require failure-only StoreUnavailableView with Retry, then skeleton after successful retry, then ready content.
- [x] Run `python3 ios/scripts/test.py views --destination 'platform=iOS Simulator,id=681FF376-73A1-4EFC-807E-E858C7B5B7C2' --filter StoreRecoveryIntegrationTests/testStartupFailureReplacesSkeletonAndRetryReturnsThroughReadyState` and observe failure.
- [x] **GREEN:** Preserve existing store ownership and failure precedence; add no second startup task.
- [x] Rerun the exact Store Recovery filter and require pass.

### Task 4: Lock out forbidden UI-test dependencies

**Files:**
- Modify: `ios/scripts/tests/test_runner.py`

- [x] **RED:** Add `test_ios_project_has_no_viewinspector_dependency_or_imports` against a temporary fixture containing a ViewInspector dependency/import and observe failure.
- [x] **GREEN:** Extend project-shape scanning to reject `ViewInspector` in `ios/project.yml`, iOS Package manifests, and non-DerivedData Swift sources.
- [x] Run `python3 -m unittest ios/scripts/tests/test_runner.py` and require pass.

### Task 5: Full verification and living ledgers

**Files:**
- Modify: `docs/FEATURES.md`
- Modify: `docs/PROGRESS.md`
- Modify: `docs/IOS-PARITY-CHECKLIST.md`
- Modify: `docs/BUGS.md` only if a product defect is discovered

- [x] Run `python3 ios/scripts/test.py prepare`.
- [x] Run `python3 ios/scripts/test.py unit --destination 'platform=iOS Simulator,id=681FF376-73A1-4EFC-807E-E858C7B5B7C2'`.
- [x] Run `python3 ios/scripts/test.py unit --destination 'platform=iOS Simulator,id=6C0B4007-CD8B-49DD-BF15-9A887BC80D9D'`.
- [x] Run `python3 ios/scripts/localize.py` and `python3 -m unittest ios/scripts/tests/test_runner.py`.
- [x] Run `xcodegen generate --spec ios/project.yml`, then `xcodebuild -project ios/Momentum.xcodeproj -scheme Momentum -configuration Debug -destination 'generic/platform=iOS Simulator' -derivedDataPath ios/DerivedData/AppBuild CODE_SIGNING_ALLOWED=NO build`.
- [x] Run `git diff --check` and `rg -n 'XCUIApplication|ViewInspector' ios --glob '!DerivedData/**'`.
- [x] Capture controlled loading and loaded screenshots from the injected hosted state separately on iOS 26.5 and 27, including light/dark and a static Reduce Motion or Low Power render where simulator controls permit. Use only an isolated disposable demo store.
- [x] After verification, add F-042 with all four platform statuses and concrete reasons, links to F-019/F-020/F-021/F-039, the exact dirty base revision, separate iOS 26.5/27 evidence, screenshots, and manual/system limits in Features, Progress, and the parity checklist.
- [x] Preserve the local branch/worktree without committing, pushing, merging, or cleaning unrelated changes.
