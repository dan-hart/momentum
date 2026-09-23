# Momentum iOS Parity Acceleration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish and verify Momentum's iOS parity with the macOS app for the user-approved Simulator scope, with Off and Nextcloud sync, native iOS 26.5/27 behavior, and no unexplained acceptance gaps.

**Architecture:** Keep task rules, persistence, sync orchestration, notification planning, and automation mutations in the shared Rust engine. Use the existing `MomentumMobile` actor boundary for Swift state and platform adapters, while SwiftUI/XCTest own native presentation and simulator-hosted unit coverage. Work evidence-first: add a focused unit regression for demonstrated defects, verify code and rendered SwiftUI on both supported runtimes, then update the existing ledgers.

**Tech Stack:** Rust, UniFFI, Swift 6, SwiftUI, UIKit, App Intents, Core Spotlight, UserNotifications, BackgroundTasks, Swift Testing, XCTest, XcodeGen, iOS 26.5 and iOS 27 Simulators.

**2026-09-18 scope revision:** the user directed removal of every iOS XCUITest target
and source. Tasks 1–4 retain their completed historical evidence, but their deleted
shipping-app test files and schemes are not active regression lanes. Remaining work
uses portable unit tests, in-process production SwiftUI rendering through
`UIHostingController`, and explicit manual simulator checks for OS interaction that
unit tests cannot establish.

---

## Scope and evidence map

This plan implements the approved design in `docs/superpowers/specs/2026-09-18-ios-parity-acceleration-design.md`. It preserves the user's current boundaries: Simulator only, no `dip17pm`, no physical device, iOS sync limited to Off and Nextcloud, and local uncommitted changes only. Any step labeled external records a precise limitation rather than claiming a pass.

| Open checklist area | Owning task | Required disposition |
|---|---|---|
| Organization create/rename/recolor/delete and Inbox protection | Task 1 | Two-runtime shipping acceptance plus focused package regression |
| Five groupings, five sorts, metadata, empty/loading/error presentation | Task 2 | Shared-rule coverage plus two-runtime native acceptance |
| Search fields, archive/subtasks, links, caps, debounce, cleared prompt | Task 2 | Two-runtime native acceptance and view-boundary regression where needed |
| Accent/font sizes, Dynamic Type, reduced motion, localization, compact/iPad | Task 5 | Existing native audits plus focused fixes and two-runtime acceptance |
| Drag assistive actions, external drag, IME, persistent Undo/recovery | Task 5 | Simulator acceptance where XCTest can invoke it; otherwise explicit manual limitation |
| Notification background execution and denial-to-enabled transition | Task 5 | Preserve portable/build evidence; record system execution as physical-only |
| Nextcloud defaults, switching, Keychain failure, local edits, expiration | Task 3 | Isolated transport/package/view/UI tests on both runtimes |
| Hosted Nextcloud/TLS and spoken VoiceOver | Task 3/5 | External limitation unless isolated credentials/service become available |
| Backup invalid/cancelled/valid restore | Task 3 | Existing package/view/UI coverage on both runtimes |
| App Intents, URLs, Spotlight, notification actions and concurrency | Task 4 | Existing system schemes and shared-owner regressions on both runtimes |
| Low Power Mode and suspended background work | Task 5 | Verify policy/build boundary; record unavailable Simulator execution precisely |
| LibreSync rows | Task 6 | Explicitly user-deferred, never counted as iOS failure or pass |
| Physical-device delivery/background/energy | Task 6 | Explicitly user-deferred, never run on `dip17pm` |

Use these destinations throughout:

```sh
export DEVELOPER_DIR=/Applications/Xcode-27.0.0.app/Contents/Developer
IOS26='platform=iOS Simulator,id=634E2736-095B-472A-9488-CAB080627674'
IOS27='platform=iOS Simulator,id=681FF376-73A1-4EFC-807E-E858C7B5B7C2'
IPAD26='platform=iOS Simulator,id=DE1ACA1D-E3C8-4F83-9DF1-1BE54318E04E'
IPAD27='platform=iOS Simulator,id=43142151-98C4-4AF1-B424-075E43FBE918'
```

Each native unit command uses `ios/scripts/test.py`, a simulator destination, local
ad-hoc signing and a unique result bundle under `ios/DerivedData/TestReports`.

### Task 1: Close the organization-management slice

**Files:**
- Modify: `ios/Packages/MomentumMobile/Sources/MomentumMobile/EngineWorker+Organization.swift`
- Modify: `ios/Packages/MomentumMobile/Tests/MomentumMobileTests/OrganizationTests.swift`
- Modify: `ios/Momentum/Tasks/ContextEditor.swift`
- Modify: `ios/MomentumUITests/OrganizationFoundationTests.swift`
- Modify: `docs/IOS-PARITY-CHECKLIST.md`
- Modify: `docs/FEATURES.md`
- Modify: `docs/BUGS.md`
- Modify: `docs/PROGRESS.md`

- [x] **Step 1: Write a failing duplicate-tag regression at the mobile boundary**

Add `creatingDuplicateTagDoesNotMutateExistingMetadata` to `OrganizationTests.swift`. Create `outing`, set color `#FF6600`, request `OUTING` through the proposed create-only API, and assert the result reports an existing tag while the stored title and color remain `outing` and `#ff6600`.

- [x] **Step 2: Run the focused package test and confirm the production API is missing or the metadata assertion fails**

Run:

```sh
python3 ios/scripts/test.py fast --filter OrganizationTests.creatingDuplicateTagDoesNotMutateExistingMetadata
```

Expected: FAIL before the fix because `createTag` does not distinguish a newly created tag from a case-insensitive match.

- [x] **Step 3: Add the smallest create-only result at the existing actor boundary**

Add a sendable result in `EngineWorker+Organization.swift`:

```swift
public struct ContextCreationResult: Sendable, Equatable {
    public let id: String
    public let created: Bool

    public init(id: String, created: Bool) {
        self.id = id
        self.created = created
    }
}
```

Implement `createTagIfAbsent(_:)` without reproducing Rust matching in Swift: capture the existing tag IDs, call the shared engine's `addTag`, and report `created == false` when the returned ID was already present. Rust remains the only owner of trimming, case-insensitive identity, and non-ASCII behavior.

- [x] **Step 4: Make the standalone tag editor update metadata only for a newly created tag**

In `ContextEditor.swift`, use `createTagIfAbsent` for new tags. If the result is existing, dismiss without calling `.updateTag`; if it is newly created, apply the chosen custom color. Existing-tag editing must continue to rename and recolor normally.

- [x] **Step 5: Run the focused package regression and the complete fast lane**

Run:

```sh
python3 ios/scripts/test.py fast --filter OrganizationTests
python3 ios/scripts/test.py fast
```

Expected: all organization tests and the full fast lane PASS.

- [x] **Step 6: Extend the shipping organization flow with the duplicate case**

In `OrganizationFoundationTests.swift`, after confirming the renamed `outdoors` tag retained its color, attempt to create `OUTDOORS`, reopen `outdoors`, and assert there is still one context with the original spelling and custom-color toggle enabled. Keep the existing coverage for project/tag create, rename, recolor, delete confirmation, exact project task count, Inbox protection, task survival after tag deletion, and cold-relaunch persistence.

- [x] **Step 7: Run the organization scheme on iOS 27, then iOS 26.5**

Run twice with the destination/result bundle changed:

```sh
xcodegen generate --spec ios/project.yml
xcodebuild -project ios/Momentum.xcodeproj -scheme MomentumOrganizationFoundationTests \
  -destination "$IOS27" -derivedDataPath ios/DerivedData/Organization \
  -parallel-testing-enabled NO -collect-test-diagnostics never \
  -resultBundlePath /tmp/momentum-organization27-final.xcresult \
  CODE_SIGNING_ALLOWED=YES CODE_SIGN_IDENTITY=- DEVELOPMENT_TEAM= test
```

Expected: PASS on iOS 27 and iOS 26.5, with the second result at `/tmp/momentum-organization26-final.xcresult`.

- [x] **Step 8: Reconcile the organization evidence immediately**

Record the exact result bundles and dirty revision in F-004, the organization checklist item, a new/resolved bug entry for duplicate-tag metadata mutation, and the dated PROGRESS handoff. Review `git diff --check` and `git status --short`; do not commit.

### Task 2: Verify grouping, sorting, search, and task presentation

**Files:**
- Modify: `ios/MomentumUITests/UpcomingFoundationTests.swift`
- Modify: `ios/MomentumUITests/ArchiveFoundationTests.swift`
- Modify: `ios/MomentumUITests/TabStateTests.swift`
- Modify if a defect is proven: `ios/Momentum/Tasks/TaskScreen.swift`
- Modify if a defect is proven: `ios/Momentum/Tasks/TaskRow.swift`
- Modify if a defect is proven: `ios/Momentum/Search/SearchScreen.swift`
- Modify if a boundary regression is needed: `ios/Packages/MomentumMobile/Tests/MomentumMobileTests/OrganizationTests.swift`
- Modify: `docs/IOS-PARITY-CHECKLIST.md`
- Modify: `docs/FEATURES.md`
- Modify: `docs/BUGS.md`
- Modify: `docs/PROGRESS.md`

- [x] **Step 1: Inventory existing shared-rule coverage before adding native assertions**

Map the five grouping choices, five sort choices in both directions, grouping/sort persistence, family placement, estimate buckets, missing contexts, duplicate names, and fallback colors to existing Rust or `MomentumMobile` tests. Add package/core tests only for an uncovered rule; do not mirror already exhaustive Rust tests in Swift.

- [x] **Step 2: Extend one durable shipping presentation flow**

Expand `UpcomingFoundationTests.swift` to seed disposable tasks that expose all grouping choices and six estimate intervals, change every grouping and sort control through the production UI, relaunch to verify preferences, and assert parent/subtask placement plus visible metadata labels. Use accessibility identifiers only where the production control has no stable semantic label.

- [x] **Step 3: Run the presentation flow on iOS 27 and fix only demonstrated failures**

Run `MomentumUpcomingFoundationTests` on `$IOS27`. For each failure, first add the smallest focused package/view regression, prove it fails, then change the owning production file and rerun the focused test before repeating the UI scheme.

- [x] **Step 4: Extend the existing Search/Archive flow instead of creating another harness**

Use `ArchiveFoundationTests.swift` and `TabStateTests.swift` to prove multi-word title/notes/project/tag matching, live subtasks and archive inclusion, project/tag link navigation, result caps/truncation notes, rapid-query stale-result rejection, cleared prompt, and read-only archive results. Add isolated fixture data through existing Debug-only launch arguments; never use the user's task store.

- [x] **Step 5: Exercise presentation states and metadata in compact layouts**

Verify empty, all-done, loading, offline, persistent-error/retry, and normal rows expose project, estimate, date/time, repeat, tags, notes, and reminder semantics without clipping the Add button or tab bar. If the test needs a deterministic state, add it to the existing disposable fixture rather than adding shipping behavior.

- [x] **Step 6: Run the completed presentation schemes on both runtimes**

Run `MomentumUpcomingFoundationTests`, `MomentumArchiveFoundationTests`, and `MomentumTabStateTests` first on `$IOS27`, then on `$IOS26`, with unique result bundles. Expected: every selected test passes; intentional skips must name an external boundary and cannot cover a changed production path.

- [x] **Step 7: Reconcile F-016, F-018–023, F-037 and related bugs/checklist rows**

Update only claims supported by current result bundles. Record any simulator-invocation limitation separately from implemented behavior. Run Swift parse checks for edited files, `python3 -m json.tool ios/Momentum/Resources/Localizable.xcstrings`, `xmllint --noout ios/Momentum.xcodeproj/xcshareddata/xcschemes/*.xcscheme`, and `git diff --check`. Review the local diff; do not commit.

### Task 3: Close Nextcloud, provider lifecycle, and backup boundaries

**Files:**
- Modify as defects require: `ios/Momentum/Settings/SyncSettings.swift`
- Modify as defects require: `ios/Packages/MomentumMobile/Sources/MomentumMobile/NextcloudConnection.swift`
- Modify as defects require: `ios/Packages/MomentumMobile/Sources/MomentumMobile/NextcloudSyncState.swift`
- Modify as defects require: `ios/Packages/MomentumMobile/Sources/MomentumMobile/NextcloudOperation.swift`
- Modify: `ios/Packages/MomentumMobile/Tests/MomentumMobileTests/NextcloudSyncStateTests.swift`
- Modify: `ios/Packages/MomentumMobile/Tests/MomentumMobileTests/SyncConnectionTests.swift`
- Modify: `ios/MomentumViewTests/SyncBoundaryIntegrationTests.swift`
- Modify: `ios/MomentumViewTests/SyncSettingsIntegrationTests.swift`
- Modify: `ios/MomentumTransportTests/NextcloudTransportTests.swift`
- Modify: `ios/MomentumSyncUITests/SyncSetupTests.swift`
- Modify if needed: `ios/MomentumUITests/BackupSettingsTests.swift`
- Modify if needed: `ios/MomentumViewTests/BackupIntegrationTests.swift`
- Modify: `docs/IOS-PARITY-CHECKLIST.md`
- Modify: `docs/FEATURES.md`
- Modify: `docs/BUGS.md`
- Modify: `docs/PROGRESS.md`

- [x] **Step 1: Run the existing fast sync filters and view boundary tests before editing**

Run:

```sh
python3 ios/scripts/test.py fast --filter 'NextcloudSyncStateTests|SyncConnectionTests'
python3 ios/scripts/test.py views --destination "$IOS27" --filter SyncBoundaryIntegrationTests
python3 ios/scripts/test.py views --destination "$IOS27" --filter SyncSettingsIntegrationTests
```

Expected: establish a current baseline for fresh Off state, preview isolation, Keychain protection, cancellation, and expiration.

- [x] **Step 2: Add missing regressions for demonstrated lifecycle gaps**

Cover only missing behavior: fresh install Off without transport/secrets, switching Off/Nextcloud during an exchange without task loss, Keychain write denial without false success, queued local edits during sync/import/switching, and late expiration unable to cancel a newer exchange. Each regression must fail against production behavior before a fix is written.

- [x] **Step 3: Implement the minimal fix at the owning boundary**

Keep credentials in Keychain, connection metadata in settings, transport/network behavior in the existing Nextcloud adapter, and task mutations in the shared engine. Do not expose LibreSync controls, introduce credentials, or change on-disk formats.

- [x] **Step 4: Run package, view, and real isolated WebDAV transport lanes**

Run the focused fast filters, affected view tests, then:

```sh
python3 ios/scripts/test.py transport --destination "$IOS27"
python3 ios/scripts/test.py transport --destination "$IOS26"
```

Expected: loopback WebDAV upload/download, encrypted/compressed exchange, conflicts, offline edits, missing-folder creation, credential correction, cancellation, and reopen all PASS without a real account.

- [x] **Step 5: Run shipping Nextcloud setup/recovery on both runtimes**

Run `MomentumSyncUITests` on iOS 27 and 26.5 with unique result bundles. Verify compact light/dark setup, German accessibility text, failed credentials/retry, provider retention, active-exchange switching, and local task survival. Hosted Nextcloud/TLS remains explicitly external unless isolated test credentials are supplied.

- [x] **Step 6: Verify backup rejection, cancellation, successful restore, and persistence**

Run the affected fast/view backup tests, then run scheme `Momentum` with `-only-testing:MomentumUITests/BackupSettingsTests` on both runtimes. Invalid input and cancelled pickers must not report success or alter tasks; a confirmed valid restore must replace the disposable store, rebuild task-dependent UI/index state, and persist after relaunch.

- [x] **Step 7: Reconcile sync/backup evidence and deferred provider scope**

Update F-024–029, F-031–032, F-040 and the corresponding checklist/bug/progress rows. Keep LibreSync explicitly user-deferred, keep hosted-server/spoken-VoiceOver limitations visible, and do not convert loopback protocol coverage into hosted-service acceptance. Review the local diff; do not commit.

### Task 4: Verify automation, URLs, Spotlight, and notification actions

**Files:**
- Modify as defects require: `ios/Momentum/App/MomentumApp.swift`
- Modify as defects require: `ios/Momentum/App/MobileAppModel.swift`
- Modify as defects require: `ios/Momentum/Automation/`
- Modify: `ios/MomentumIntentTests/AppIntentTests.swift`
- Modify: `ios/MomentumViewTests/AutomationIntegrationTests.swift`
- Modify: `ios/MomentumViewTests/SpotlightIntegrationTests.swift`
- Modify: `ios/MomentumUITests/AutomationTests.swift`
- Modify: `ios/MomentumUITests/NotificationActionSystemTests.swift`
- Modify: `ios/MomentumUITests/SpotlightSystemUITests.swift`
- Modify: `docs/IOS-PARITY-CHECKLIST.md`
- Modify: `docs/FEATURES.md`
- Modify: `docs/BUGS.md`
- Modify: `docs/PROGRESS.md`

- [x] **Step 1: Run the shared-owner and URL/Spotlight view regressions**

Run the fast automation/URL/Spotlight filters and the affected `MomentumFastTests` view filters on iOS 27. Confirm cold concurrent entry points share one worker, preserve selected tab/editor state, and ignore stale identifiers.

- [x] **Step 2: Run production URL/Shortcuts automation on iOS 27**

Run the scheme containing `AutomationTests` to cover cold/warm URLs plus create/find/plan/complete/reopen/open actions. If Shortcuts UI is unavailable or unstable, retain the `AppIntentsTesting` result as the platform boundary and record the precise system restriction.

- [x] **Step 3: Run `MomentumIntentTests` on iOS 27 and the supported automation boundaries on iOS 26.5**

Run `MomentumIntentTests` only on iOS 27 because the target deploys to iOS 27 and imports `AppIntentsTesting`. On iOS 26.5, use the existing URL/Shortcuts shipping UI flow plus package/view automation tests for equivalent create/find/plan/complete/reopen/open behavior. Use ordinary local simulator signing only. Do not install to a device or alter a personal signing team. A Customer OS security skip/failure must be recorded as a framework limitation unless a production defect is independently reproduced.

- [x] **Step 4: Run native Spotlight index, activation, and SpringBoard schemes**

Run `MomentumSpotlightTests`, `MomentumSpotlightActivationTests`, and `MomentumSpotlightSystemUITests` serially on iOS 27, then iOS 26.5. Require current task titles, cold/warm routing to Search, exact deletion from the index, disposable-domain cleanup, and no lost capture draft.

- [x] **Step 5: Run notification action concurrency and system delivery schemes**

Run focused package/view action tests plus `MomentumNotificationSystemTests` on both runtimes. Verify cold body activation, cold Done, resident Snooze, single mutation, auto-archive behavior, and exact one-hour rearm. Do not claim locked-device, Focus, physical-device, or suspended replenishment acceptance.

- [x] **Step 6: Fix only reproduced ownership/routing defects with a red regression first**

Keep App Intents, URLs, Spotlight, and notification callbacks as thin adapters into the same `MobileAppModel`/shared engine. No automation path may open a second production store owner or duplicate a mutation.

- [x] **Step 7: Reconcile F-029–032 and automation/notification checklist rows**

Record each scheme/result bundle separately because system integration evidence is not interchangeable. Run project generation, Swift parse, plist/catalog validation, and `git diff --check`. Review the local diff; do not commit.

### Task 5: Close accessibility, keyboard, interaction, and presentation acceptance

**Files:**
- Modify as defects require: `ios/Momentum/App/MobileKeyboardCommands.swift`
- Modify as defects require: `ios/Momentum/Tasks/TaskScreen.swift`
- Modify as defects require: `ios/Momentum/Tasks/TaskRow.swift`
- Modify as defects require: `ios/Momentum/Design/`
- Modify: `ios/MomentumViewTests/HostingFixture.swift`
- Modify: `ios/MomentumViewTests/ViewRenderingTests.swift`
- Modify: `ios/Momentum/Resources/Localizable.xcstrings`
- Modify: `docs/ACCESSIBILITY.md`
- Modify: `docs/audits/2026-09-16-ios-accessibility.md`
- Modify: `docs/IOS-PARITY-CHECKLIST.md`
- Modify: `docs/FEATURES.md`
- Modify: `docs/BUGS.md`
- Modify: `docs/PROGRESS.md`

- [x] **Step 1: Run the existing fast/view accessibility regressions**

Run `python3 ios/scripts/test.py fast` and the complete views lane on iOS 27. Preserve the exact default `#FF6600`, white button text in normal contrast, calculated high-contrast exception, the nine approved DHFlatUIColors accents, colorful-label toggle, native font choices, independent content/interface scaling, reset/persistence, reduced-motion behavior, and non-color selection cues.

- [x] **Step 2: Replace mutable XCUITest audits with deterministic rendered-unit coverage**

Use `HostingFixture` to supply explicit English/German, light/dark,
normal/increased contrast, Dynamic Type, content scale and compact/tablet widths.
Test production SwiftUI geometry, pixels and accessibility metadata without
ViewInspector. Spoken VoiceOver, alternate input and whole-screen focus order remain
manual boundaries.

- [x] **Step 3: Preserve keyboard logic coverage and retire synthetic key injection**

Keep bindings, command enablement, responder actions, editor isolation and mutations
covered at the portable/native-unit boundary. The former XCUITest key-dispatch cases
are removed; actual hardware-key routing is manual simulator acceptance and stays open
where B-045–B-047 record unresolved behavior.

- [x] **Step 4: Cover accessible actions and recovery at the state/view boundary**

Exercise the same move/reorder mutations, undo state, error persistence and recovery
through shared-core and production-view unit tests. Gesture recognizer delivery and
assistive invocation remain manual checks.

- [x] **Step 5: Keep external drop and IME as explicit manual boundaries**

Retain shared parser/editor unit regressions. Do not synthesize cross-app drag or IME
marked-text behavior in a unit test and do not add a test-only shipping API.

- [x] **Step 6: Verify notification/background policy boundaries in units**

Run package/view policy tests and production builds. Permission alerts, delivery,
suspended `BGAppRefreshTask`, Low Power Mode, locked-device behavior and system
expiration execution remain manual/system boundaries.

- [x] **Step 7: Reconcile accessibility and platform-boundary records**

Update the accessibility audit, F-009–023, notification rows, related bugs, checklist,
and PROGRESS with exact unit results and explicit manual limits. Validate the string
catalog, parse edited Swift, run `git diff --check`, and review the local diff; do not commit.

### Task 6: Run the final parity gate and publish the local handoff

**Files:**
- Modify: `docs/IOS-PARITY-CHECKLIST.md`
- Modify: `docs/FEATURES.md`
- Modify: `docs/BUGS.md`
- Modify: `docs/PROGRESS.md`
- Modify if current commands changed: `ios/TESTING.md`
- Modify if user-facing decisions/evidence changed: `README.md`

- [ ] **Step 1: Re-scan every unchecked checklist item**

Run `rg -n '^- \[ \]' docs/IOS-PARITY-CHECKLIST.md`. For each result, link it to current evidence or the user's explicit LibreSync/physical-device deferrals. Any other external limitation remains open and blocks an unqualified parity-complete claim until the user explicitly approves that disposition. No unexplained item may remain.

- [ ] **Step 2: Regenerate the shared iOS core and run the full fast lane**

Run:

```sh
python3 ios/scripts/test.py prepare
python3 ios/scripts/test.py fast
```

Expected: generated UniFFI bindings match current Rust and all package tests PASS.

- [ ] **Step 3: Run native view and transport lanes on both runtimes**

Run:

```sh
python3 ios/scripts/test.py views --destination "$IOS27"
python3 ios/scripts/test.py views --destination "$IOS26"
python3 ios/scripts/test.py transport --destination "$IOS27"
python3 ios/scripts/test.py transport --destination "$IOS26"
```

Expected: all executed tests PASS; record test counts, skips, durations, and result bundle paths.

- [ ] **Step 4: Run focused unit filters and record manual boundaries**

Use `fast`, `views`, and `transport` filters from Tasks 1–5. There are no XCUITest
schemes. Do not recreate deleted app-driving coverage; record gestures, external apps,
system prompts and assistive-technology checks as manual boundaries.

- [ ] **Step 5: Check shared consumers and repository integrity**

Run:

```sh
cargo fmt --check
cargo test --workspace --exclude momentum
swift test --package-path macos/Packages/MomentumKit
python3 -m json.tool ios/Momentum/Resources/Localizable.xcstrings >/dev/null
xcodegen generate --spec ios/project.yml
xmllint --noout ios/Momentum.xcodeproj/xcshareddata/xcschemes/*.xcscheme
git diff --check
```

If the Linux runtime is unavailable on this Mac, record compilation/test scope without claiming native Linux runtime validation.

- [ ] **Step 6: Reconcile all four ledgers against current evidence**

Require every iOS-applicable stable feature row to be Verified for the approved scope. The already approved LibreSync and physical-device deferrals may remain explicitly scoped; hosted Nextcloud, spoken VoiceOver, cross-app drag, IME, or any other newly identified limitation remains open and prevents an unqualified parity-complete claim unless the user separately approves its disposition. Retain resolved bugs and exact regression evidence.

- [ ] **Step 7: Review the complete local change set and deliver the handoff**

Inspect `git status --short`, `git diff --stat`, focused diffs for every touched file, generated project changes, and unexpected binaries/secrets/private addresses. Report what changed, exact verification, current platform status, and remaining external boundaries. Do not commit, push, install on `dip17pm`, open a PR, or release.
