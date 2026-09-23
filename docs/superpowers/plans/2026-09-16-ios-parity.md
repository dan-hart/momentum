# Momentum iOS Implementation Plan

> Use superpowers:subagent-driven-development for independent bounded work and
> review, and test-driven-development for behavioral changes. No commits or pushes.

**Goal:** Deliver verified macOS feature parity in native SwiftUI on iOS 26 and 27.
**Architecture:** Shared Rust Engine and generated UniFFI; portable MomentumKit
support; dedicated iOS state/lifecycle and native views. Four tabs in the approved
order: Today, Upcoming, Search, Settings. Today has native Lists navigation for
projects/tags, Morning/Evening and Archive. DHFlatUIColors is the user-approved
palette dependency; other new third-party runtime dependencies remain out of scope.
**Worktree:** `.claude/worktrees/ios-app`, branch `task/ios-app`, base `7ec82e9`.
**Acceptance:** `docs/IOS-PARITY-CHECKLIST.md` plus the approved design spec. App code
is not equivalent to parity; every final claim requires dated scoped evidence.

## 1. Build the real core for iOS

Files: new `ios/scripts/build-core.sh`; modified MomentumCore `Package.swift`,
`.gitignore` and `macos/scripts/build-core.sh`; generated ignored XCFramework/bindings.
The existing builder becomes the single Apple artifact producer. The iOS wrapper
requires mobile targets; Mac invocations preserve/build installed iOS slices too,
with CLI compilation only for Mac. Serialize shared artifact publication.

- [ ] Establish baseline Rust workspace tests with pinned LibreSync v0.6.0.
- [ ] Install Rust device and arm64 simulator targets, verify installed target readback.
- [ ] Build `cargo build -p momentum-ffi --target aarch64-apple-ios-sim` with simulator
  SDK, then device target with device SDK. Resolve platform-only dependencies in the
  smallest boundary possible; do not remove LibreSync to obtain a passing build.
- [ ] Build host metadata library, generate Swift bindings using existing bindgen,
  package Mac/device/simulator static libraries into XCFramework with C module map.
- [ ] Add iOS 26 to the generated binding package deployment targets.
- [ ] Validate archive platform slices and compile a real Swift consumer for iOS.
- [ ] Verify iOS build → Mac build → iOS build/test preserves all platform slices.

## 2. Portable Apple support

Files: `macos/Packages/MomentumKit/Package.swift`, `Sources/MomentumKit/{AppState,
Services,TaskAutomation,Typography,Shortcuts,Preferences}.swift`, existing package tests.

- [ ] Identify a compile failure for an iOS package consumer before changing guards.
- [ ] Guard desktop AppState/services/automation/typography with macOS boundaries;
  preserve portable form mapping, strings, preferences, Keychain, transfers, color policy.
- [ ] Retain portable shortcut definitions where valid; isolate NSResponder and AppKit.
- [ ] Keep desktop APIs and behavior unchanged. Use UIKit only in iOS adapters.
- [ ] Run macOS package tests; compile iOS consumer against the same package resources.

## 3. Serialized engine boundary and native bootstrap

Files: new `ios/Packages/MomentumMobile/{Package.swift,Sources/MomentumMobile,
Tests/MomentumMobileTests}`; `ios/{project.yml,Momentum/App,MomentumTests,MomentumUITests}`.

- [ ] Add failing isolated-store tests for create/edit/relaunch, list snapshot, undo,
  cross-tab invalidation, archived guards and stale search suppression.
- [ ] Implement a worker owning a real Engine, all synchronous engine I/O off-main,
  immutable snapshots and stable IDs; one process owner used by all entry points.
  Blocking transport exchanges dispatch separately to a utility worker sharing the
  same Engine. A paused exchange test must prove edits complete before its release
  and survive the core-coordinated final commit.
- [ ] Implement `@MainActor @Observable` presentation state with per-tab navigation,
  loading/error state, command outcomes/undo and lifecycle coordinator boundaries.
- [ ] Create XcodeGen app/test targets, native four-tab shell and isolated demo launch
  arguments. Declare minimum iOS 26, Swift 6 strict concurrency, local packages.
- [ ] Run boundary tests and XCUITest tab-order/launch test on iOS 27 and 26.5.

## 4. Complete offline workflows

Files: new `ios/Momentum/Tasks/{TaskList,TaskRow,QuickAdd,TaskEditor,RepeatEditor,
ListPicker,ContextEditor,TaskActions}.swift`, mobile command/snapshot boundaries/tests.

- [ ] Add failing command tests per core mutation before exposing it in the UI.
- [ ] Implement all editor fields using shared TaskFormModel, native date/time controls,
  notes, subtasks, duplication and repeat editing. Guard invalid/no-title saves.
- [ ] Implement Today/list navigation, Upcoming range, Search caps/result navigation,
  Archive paging/read-only state, project/tag management and colors.
- [ ] Implement grouping/sorting, bulk actions, context/swipe actions, transfer payloads,
  manual reorder, accessible move alternatives and native multiline paste behavior.
- [ ] Native acceptance: add/edit/complete/undo/relaunch; every bulk action; grouped
  moves; archived rejection; input selection; drag/drop; all documented empty states.

## 5. Settings and native quality

Files: new `ios/Momentum/Settings`, `ios/Momentum/Design`, resource catalogs and tests.

- [ ] Test defaults/persistence, badge modes, modifier choices and Dynamic Type scaling.
- [ ] Implement Settings tab for tasks, notifications, typography, appearance, keyboard,
  sync and data. Reuse shared preference keys only where semantics match.
- [ ] Add relative iOS typography/font picker, semantic colors and reduced-motion
  completion feedback; keep 44-point touch targets and native control roles.
- [ ] Localize every new string EN/DE and adapt existing desktop-specific wording.
- [ ] Inspect compact/landscape/iPad, accessibility text, VoiceOver actions, contrast,
  keyboard navigation and reduced motion with real UI evidence.

## 6. Transport lifecycle and backup safety

Files: `crates/momentum-core/src/{engine,p2p,types,tests}.rs`, `crates/sp-sync/src`,
new iOS sync/Keychain adapters and document import/export UI/tests.

- [ ] Reproduce sync/import race in isolated tests; add generation/cancellation/deadline
  tests, stable operation-ID merging and guarded final persistence as spec requires.
- [ ] Implement transport generation and exclusive import/provider-switch barrier;
  cancel obsolete results across blocking FFI and preserve offline pending edits.
- [ ] Implement selected provider only, Keychain configuration, manual/debounced sync,
  persistent status/errors/retry, foreground pairing/discovery and background teardown.
- [ ] Add native document picker/share export and explicit replacement confirmation;
  test cancellation/invalid input and race with an active exchange.
- [ ] Validate mock WebDAV plus isolated real peer exchange, both store and UI readback.

## 7. Notifications and OS integrations

Files: new core notification-planning module/tests; iOS notifications/lifecycle,
Spotlight/AppIntents/URL/badge adapters and respective tests.

- [ ] Add deterministic tests for seven-day recurrence projection, notification budget,
  identities, opt-in summary, DST/restart/denial and accepted-schedule reconciliation.
- [ ] Implement nonmutating core plan and occurrence action resolution; preserve
  desktop due-now semantics and recurrence compatibility.
- [ ] Schedule UserNotifications with Done/Snooze, permissions recovery, cancellation,
  foreground reconciliation and bounded BackgroundTasks refresh. Test expiration.
- [ ] Implement app-hosted create/find/complete/reopen/plan/open intents, URLs, Spotlight
  and badge modes through the same process engine owner. Verify cold and warm paths.
- [ ] Record simulator versus physical-device delivery/background evidence separately.

## 8. Final parity and performance audit

- [ ] Run `cargo test --workspace --exclude momentum` and all-features/all-targets variant.
- [ ] Run `swift test --package-path macos/Packages/MomentumKit`, mobile boundary suites,
  full iOS XCUITest workflows on 26.5/27.0, unsigned device build and Mac build.
- [ ] Revisit every acceptance case and attach exact evidence; source review
  cannot promote native interaction to Verified. Preserve prior desktop deferrals.
- [ ] Profile large-list scrolling, startup, search and mutation responsiveness; inspect
  background activity and low-power behavior, record device/dataset/limits.
- [ ] Update all ledgers, specs and build/testing docs; format and review complete diff.
- [ ] Independent spec and code-quality reviews; fix findings and rerun affected checks.
- [ ] Goal complete only after actual requested parity is proven. If device/SDK access
  blocks a required check, report the exact remaining gap; do not claim completion.

Update FEATURES/BUGS/PROGRESS at every stage. Associate acceptance cases with named
tests and native workflow IDs as each stage begins; do not defer traceability to final audit.

## Commands and evidence convention

Run commands from this worktree. Save transient output under `/tmp/momentum-ios-*`;
record meaningful results in PROGRESS.md rather than treating transient logs as the
only evidence. No user store, production Keychain entries or real sync credentials.

The iOS build script creates the package before Xcode resolves it. Generate project
with `xcodegen generate --spec ios/project.yml`; use isolated DerivedData and test
devices. Prefer bounded tool waits and inspect actual exit/results before claiming
a pass. Platform/dependency compatibility failures are investigated, not silently
disabled or replaced with mock behavior.

## Notification adapter implementation notes — 2026-09-16

Read-only API review confirms the generated boundary is ready. This remains planned
native work; the following are correctness constraints, not completed verification:

- Use a single coalesced drain task (`running`/`dirty`) across async suspension;
  actor isolation alone does not serialize complete reconciliation passes. Cancelling
  one caller must not cancel a shared drain. Keep the core worker off the main actor.
- Read pending then delivered OS observations, use a fresh clock for reconciliation
  and acknowledgement, cancel exact obsolete revisions, and schedule desired minus
  accepted. Preserve partial failures as actionable status; no automatic permission
  prompt. Distinguish provisional authorization from full authorization.
- Acceptance is the commit point. Before acknowledgement starts, cancellation after
  a successful OS add removes that identity. Once acknowledgement starts, await its
  result even if cancellation arrives. Keep accepted requests; cancel the exact
  supplied identity on rejection/error. Never remove an accepted catch-up request
  merely because its waiter was cancelled: the core may have consumed its identity.
- Native identifiers use a namespaced SHA256 of an unambiguously encoded versioned
  tuple (logical ID, source revision). Validate that stored metadata reproduces the
  identifier. Track malformed owned identifiers for explicit cleanup; ignore other
  namespaces. Revision metadata contains private content and must never be logged.
- Remove exact obsolete identities from pending and delivered collections. SDK removal
  returns without completion; read back before claiming confirmed cancellation.
  Keep native framework objects inside the adapter and expose immutable Sendable
  values plus sanitized error categories. Never log arbitrary OS error dictionaries.
- One-shot triggers use a fresh positive interval for catch-up; acknowledge the
  original unchanged core request. Inject the localized text formatter. Demo/UI-test
  stores must receive a fake OS center and cannot obtain the real center.
- Behavioral tests cover add/ack failures, stale source during add, cancellation on
  both sides of the acceptance boundary, coalescing, pending-to-delivered transition,
  restart between add/ack, dismissal, missing future requests, denied/notDetermined/
  provisional authorization, malformed metadata, hash tuple boundaries, delayed old
  removal, positive catch-up triggers, horizon/overflow, and partial-failure status.

### Approved interaction refinement — 2026-09-16

Keep task capture/edit/update/delete fast. Use native sheet action symbols, accent
emphasis for the commit action, red destructive affordances, swipe shortcuts and
brief state motion that respects Reduce Motion. Never dismiss a failed capture or
allow a second edit path while its save is in flight. Keep existing Rust Undo.
Settings becomes a short category index with focused Task Lists, Appearance and
About pages; do not place future sync/notification controls in unrelated categories.
Verify category navigation and stored preferences, rejected capture/retry, editing,
delete/undo and relaunch in isolated native UI tests; capture before/after evidence.
