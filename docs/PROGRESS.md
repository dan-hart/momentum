# Momentum progress and handoff

Updated: 2026-09-19. Product rules live in [AGENTS.md](../AGENTS.md); exact capability
and defect status live in [FEATURES.md](FEATURES.md) and [BUGS.md](BUGS.md).

## Current workspace

Desktop development was consolidated from `.claude/worktrees/pull-latest-code-70308a`,
based on `0c9c0bf`, using integration branch `task/desktop-mvp` and target `main`.
The commit containing this handoff includes the accumulated macOS/shared-core work,
Linux parity, grouping, configurable summaries and the canonical project guidance.
Preserve any subsequent uncommitted work and inspect current Git state before continuing.

The earlier secret-scanner blocker is resolved: on 2026-09-16 the user explicitly
approved adding `FileCreateFlags::PRIVATE` to `.gitallowed` for the confirmed GIO
enum-name false positive, then committing and pushing to main. The existing scanner
hook remains enabled. Earlier blocked attempts below are historical, not a new request
for approval. The outer checkout's temporary routing guide is superseded by the full
tracked AGENTS.md when main fast-forwards to this integration.

The current uncommitted Linux parity work is on `task/linux-desktop-parity` at
`/var/home/danhart/.config/superpowers/worktrees/momentum/linux-desktop-parity`, based
on `7ec82e93271f`. No commit, push, release, dependency change, or data migration was
performed for this handoff.

## Platform baseline

| Platform | Current state | Verification limits / next direction |
|---|---|---|
| Linux | Established native GTK4/libadwaita MVP using shared Rust engine | Full SDK/GTK tests and broad isolated GNOME Wayland acceptance ran 2026-09-17. Typography, automation and Background Apps count parity are Implemented; remaining feature-specific native gates are listed below. |
| macOS | Established SwiftUI/AppKit MVP using UniFFI/shared core | Local build/tests and many native flows recorded; specific deferred/unverified checks below. |
| iOS | In progress in `task/ios-app`; approved native SwiftUI architecture and real shared Rust core | Four-tab offline workflows build and have scoped iOS 26.5/27 native evidence. Nextcloud and LibreSync are implemented; real simulator notification delivery/actions and App Intents pass. Unit coverage exercises the production background handler, IME boundary, external payload decoding and keyboard fallbacks. Hosted TLS, scheduler-originated execution, separate-device Bonjour and remaining manual system interactions stay open. |
| Android | Upcoming; no native app foundation yet | Plan native frontend/platform services and bindings around shared Rust core; scope/order to be agreed. |

## Current priorities

### 2026-09-16 — iOS implementation

- Architecture approved; work is isolated in `.claude/worktrees/ios-app`, based on
  `7ec82e9`, with local changes only. See the dated implementation evidence below.
- Current focus: the fast unit-only iOS regression architecture and remaining manual/system
  acceptance boundaries after the resumed LibreSync and integration pass. The default
  accent remains exactly #FF6600 in light/dark; AsNeeded's nine DHFlatUIColors custom
  choices use accessible rendered tones.
- Remaining checks are tracked in [IOS-PARITY-CHECKLIST.md](IOS-PARITY-CHECKLIST.md).
  An implemented screen or successful core test does not establish native parity.

### 2026-09-16 — historical iOS architecture preparation

- Baseline inspected: clean `main`, `7ec82e9`; subsequent local changes are design
  and project-record documentation only. No iOS code, commit or push yet.
- Full existing feature map and mobile equivalents documented in the linked proposal.
  Independent spec review approved the revised sync replacement/cancellation barrier
  and bounded recurring-notification plan. User architecture agreement is pending.
- Environment: Xcode 27.0 (`27A266a`), iOS 27 SDK, iOS 26.5/27.0 iPhone/iPad
  simulators available. Rust has only the macOS target installed. LibreSync dependency
  setup is missing from this checkout; reuse the existing pinned `v0.6.0` dependency,
  not an unverified current checkout. No build/runtime tests were run in this design pass.
- `git diff --check` passed. Next: obtain the requested scaffolding agreement, then
  build generated mobile FFI slices and the isolated native application foundation.
- Follow-up source audit produced [IOS-PARITY-CHECKLIST.md](IOS-PARITY-CHECKLIST.md),
  with separate cases for every bulk action, sorting/grouping choice, badge mode,
  shortcut modifier and automation action. It records source/spec discrepancies:
  archive restoration is via undo, current sync uses one provider, and summary is
  opt-in. All iOS acceptance remains Planned; architecture agreement is still pending.

- **iOS parity:** the user approved the
  [architecture/parity proposal](superpowers/specs/2026-09-16-ios-parity-design.md)
  and requested work through verified macOS feature parity. Android remains planned.

- **F-038: configurable morning summary.** Linux/macOS implementation and automated
  checks pass; Mac Settings off/default readback passed. Finish native time-edit/delivery
  acceptance and Linux Blueprint/runtime checks when those environments are available.
  iOS/Android carry explicit requirements in [NOTIFICATIONS.md](NOTIFICATIONS.md).

1. **F-037: Group By on Linux/macOS, planned for mobile.** Shared Rust grouping, native
   View Options controls, independent platform tests and status. The user approved first-tag-only
   placement and six estimate intervals, plus #tag and project headings in their saved colors. Native Mac
   grouping now defaults to Morning & Night with one exclusive grouping layer. Revised
   automated checks pass; native verification limits are recorded below. No
   data migration is involved.
2. **F-027 / F-028: verify Linux sync parity.** Implementation is complete; run
   Blueprint/resource compilation and GTK tests on Linux, then check provider switching,
   persistent status on empty/populated lists, Details/Retry, and CLI routing. Use isolated
   settings/data. Physical LibreSync testing remains separately deferred by the user.
3. Deliver the requested native iOS app through verified macOS parity. The platform
   order and mobile architecture are approved; implementation is underway.

1. Close only the explicit Linux native gaps below: directed keyboard/drag behavior,
   exact 360×720 observation, GNOME's visual Background Apps surface, successful/progress
   sync states, notification-denial handling, and a safe user-visible cold automation launch.
2. Re-run the Apple build and Swift boundary suites on macOS; this Linux host has no
   `swift`, `xcrun`, or `xcodebuild`.
3. Establish the next mobile milestone with the user; neither mobile app is implicitly
   scaffolded by desktop feature work. Do not choose a delivery order without a decision.
4. Preserve open acceptance gaps; resume user-deferred work only when requested.

## Approved decisions — 2026-09-16

- iOS is the next requested app: native Swift/SwiftUI, tab navigation, shared Rust
  behavior, iOS 26/27, smooth accessible UI and battery-efficient lifecycle. The user
  defined macOS feature parity as the completion criterion. No new external service,
  task-data migration, commit or publication was requested.
- Approved tabs: Today, Upcoming, Search, Settings. Default accent #FF6600, custom
  accent selection restricted to DHFlatUIColors, accessible light/dark/increased-
  contrast tones, SF Symbols, and an opaque icon that fills the asset edge-to-edge.

- F-037 revision: Morning & Night is the default; Today/Morning/Evening are tag-based
  groups in that mode only. Other modes show only their chosen grouping. Completed
  remains separate. Applied interpretation of “only grouping”: overdue/upcoming dates
  appear on rows instead of extra date sections; the optional clarification has not
  received a response yet. Preserve explicitly saved grouping preferences.

- F-037: Group By uses the first tag only, six approved estimate ranges, and #tag
  and project headings colored with their saved colors. See [GROUPING.md](GROUPING.md).

- Momentum respects user freedom and prioritizes excellent native experiences on Linux,
  macOS, iOS, and Android. Existing Linux/Mac technology is the architecture baseline.
- Reuse one Rust core for task rules, storage and sync. Native frontends own presentation
  and OS integration; do not duplicate product rules or share a cross-platform UI layer.
- New work covers every existing platform by default. Record concrete mobile requirements
  until those apps exist; thereafter include them by default too.
- Offline-first, account-free core task management; optional sync that is seamless when
  enabled. No advertising, tracking or analytics without explicit approval.
- Code remains open source and free to build, compile, self-host and use. Official-build
  pricing is undecided. Eventual Apple RevenueCat subscriptions and Apple/Buy Me a Coffee
  tips are intentions, not approved integrations.
- v2 aims to leave the SP foundation; format, migration and interoperability are undecided.
- Agents proceed with routine work; ask about major architecture, data migrations,
  dependencies/services, monetization and ambiguous product behavior.
- Verified is the completion bar, with independent per-platform status and evidence.
- Local changes only; commits, pushes and PRs require explicit requests.

## Open acceptance and decisions

| Item | State | Next action |
|---|---|---|
| Physical LibreSync Linux↔Mac, B-001/F-026 | Earlier desktop deferral remains; implementation is not a proven real-device fix | Preserve earlier evidence and diagnostics until desktop physical testing is resumed |
| Spoken VoiceOver, F-020 | Explicitly excluded by the user from the current iOS completion scope; AX/keyboard/contrast checks do not establish spoken output | Keep unverified without blocking the user-approved scope |
| Voice Control and Switch Control, F-020 | Visible touch alternatives, stable labels and named actions exist; live system activation/focus remains unverified | Exercise manually in isolated simulator data |
| iOS hosted Nextcloud and separate-device LibreSync, F-025/F-026 | Opt-in hosted TLS and real local two-peer transport harnesses exist; this run had no disposable hosted credentials or separate device | Run only with isolated credentials/devices when available |
| iOS scheduler/locked automation boundaries, F-007/F-031 | Production handlers and authentication policy compile and pass direct regressions; Simulator cannot force every system launch condition | Exercise scheduler-originated and locked invocation when a supported system harness is available |
| Remaining drag/drop edges, F-014/F-020 | Real single/multi row reorder and project/tag/day payloads are verified on iOS 26.5/27; a real cross-app text/URL gesture and assistive Move Up/Down invocation remain unverified | Drive the outstanding cross-app and assistive gestures in isolated simulator data |
| Linux runtime validation | Needs a Linux desktop or supported Broadway/Flatpak environment | Run Linux checks for affected changes; compilation alone is insufficient |
| Spotlight create/open, F-024/F-031 | User deferred testing after automation could not launch Spotlight | Await request to resume native discovery and warm/cold invocation |
| DND edge cases, F-012/F-013 | Shared tests cover tag/day, descending order, archive exclusion; separate native gestures unverified | Complete targeted native cases when environment permits |
| Linux parity native gaps, F-022/F-027/F-028/F-031/F-032/F-037/F-038 | Broad isolated GNOME Wayland acceptance ran; exact compact layout, directed keyboard/drag, successful/progress sync, notification denial, visual Background Apps status and safe registered-profile cold launch remain unverified | Repeat only the named scenarios in an environment that exposes the required compositor/portal/network interaction; retain Implemented until complete |
| Mac exact minimum width / independent German language review | Not separately verified | Target these when changing affected layouts/copy |
| Universal/release builds and remote CI | Local arm64 development evidence only; remote workflow not executed in the prior pass | Validate separately when release work is authorized |
| v2 SP departure, F-033 | Goal approved; implementation choices open | Propose migration/compatibility design before changing formats |
| Billing/tips, F-034–F-036 | Future intention; pricing and entitlements open | Product decision and current provider/store review before implementation |

## Dated handoffs

### 2026-09-20 — stale Morning loading state fixed (F-003/F-004/F-018/F-039, B-090)

- The conditional Morning detail could finish loading with `viewExists == false` after a
  sidebar-to-detail race, then return before accepting the completed snapshot. That left the
  disappearing screen's local snapshot nil and allowed “Loading tasks” to remain indefinitely.
  `TaskScreen` now accepts every completed snapshot before applying its existing dismiss or
  workspace-fallback policy. The same correction covers Evening and deleted project/tag routes.
- A host-based SwiftUI regression reproduced the missing assignment and stuck loading state
  before the fix, then passed after it. A companion test drives the compact sidebar into a
  valid Morning destination and verifies its task appears. The complete portable lane passes
  156 tests in 23 suites. The native view/integration lane passes 95 tests with two intentional
  opt-in skips on iOS 27 (`views-1789915849541309000.xcresult`) and iOS 26.5
  (`views-1789915881809201000.xcresult`). No XCUI test or production data was used.
- A fresh Xcode 27 signed device build completed, but dip17pm changed from available to
  unavailable before CoreDevice could begin installation. The install request failed without
  modifying the existing app, so the physical device still has the preceding build. Reconnect
  and unlock it before installing this correction; no physical-device acceptance is claimed.
- Work remains local on dirty `task/ios-app` at base
  `7ec82e93271fdddfae7dce5731bdec12a2ec141b`; no reset, commit, push or release occurred.

### 2026-09-20 — optimized F-044 build installed and launched on dip17pm

- A fresh signed Debug device bundle was built from the current dirty `task/ios-app`
  worktree with Xcode 27 and automatic development signing, then installed over the
  existing dip17pm app while preserving its data container. Bundle readback was
  `com.codedbydan.Momentum.ios`, version `0.4.0`, build `1`.
- CoreDevice launched the new installation and confirmed Momentum running as PID 22544
  from the new bundle URL. Per the user's request, this was build/install/launch only:
  no device tests, fixture population, task inspection, sync connection, data reset,
  commit, push or release occurred.

### 2026-09-20 — lazy task-list indexing and measured first-load performance (F-001/F-003/F-004/F-039/F-044, B-055/B-089)

- Investigation of slow task loading found that every non-search `Engine::listing` call
  eagerly built the global `SearchIndex`: it lowercased all live titles, notes, project/tag
  names and deserialized both archive tiers before Today, Upcoming, project or tag rows could
  render. The existing cache was already mutation-aware; the defect was when it was built,
  so no second persisted or SwiftUI snapshot cache was introduced.
- The shared engine now builds that index only for Archive; Search retains its dedicated
  cached path. Startup recurrence deduplication checks archive entity-map IDs directly rather
  than deserializing every archived task. A focused test failed on the eager Today behavior
  before the fix, then passed with Archive still creating the cache. Archived-repeat coverage
  preserves the no-respawn rule across old/minimal archived payloads.
- The opt-in Release unit/view benchmark now times a cold persisted-store reopen, startup
  recurrence checks and the actual first actor/core/native snapshot, not only repeated reads
  of an already-created array. With 1,000 imported Today tasks it passed at 80.811 ms on
  iOS 27 (`ios/DerivedData/TestReports/performance-cold-27-20260920.xcresult`) and 67.001 ms
  on iOS 26.5 (`performance-cold-265-20260920.xcresult`), both under the new 100 ms guard.
  These simulator measurements establish the local data-to-first-snapshot boundary; they do
  not claim physical-device, launch-animation or network-sync timing.
- Fresh generated Apple core artifacts and the portable mobile lane pass: 156 tests in
  23 suites. The complete native view lane passes 93 tests with two intentional opt-in
  skips on iOS 27 (`views-1789914770554241000.xcresult`) and iOS 26.5
  (`views-1789914801803173000.xcresult`). The Rust workspace passes 23 CLI tests,
  131/132 core tests with one manual latency sample ignored, 21 notification-ledger tests
  and all remaining workspace/doc tests. MomentumKit passes 136 tests in 32 suites with
  the opt-in nearby test skipped. The runner's 12 self-tests pass, localization reports
  364/364 German entries, and the full generic iOS Simulator app build succeeds.
- Two existing day-movement tests assumed Tomorrow and Next Week differ. On Sunday both
  correctly resolve to Monday, so the core regression now conditionally undoes only a real
  second move and the native integration uses a fresh task for Next Week. Both focused tests
  and the complete suites pass without changing production scheduling behavior.
- Work remains local on dirty `task/ios-app` at base
  `7ec82e93271fdddfae7dce5731bdec12a2ec141b`; no reset, commit, push or release occurred.

### 2026-09-20 — F-043 build installed and launched on dip17pm

- After the user unlocked dip17pm, the previously verified signed production bundle was
  installed over the existing app and launched. Bundle readback before installation was
  `com.codedbydan.Momentum.ios`, version `0.4.0`, build `1`; CoreDevice then confirmed a
  live Momentum process at the new installation URL. The app container was preserved.
- No device tests, fixture population, sync connection, task inspection, data reset,
  commit, push or release occurred. Simulator and automated evidence for F-043/B-084–B-088
  remains in the preceding 2026-09-19 handoff; process readback does not claim physical
  touch, layout, network-provider or accessibility acceptance.

### 2026-09-19 — compact sync, transient feedback and colorful list metadata (F-004/F-017/F-019/F-020/F-025/F-026/F-028/F-039/F-043, B-016/B-084–B-088)

- Dirty local `task/ios-app` at base `7ec82e93271fdddfae7dce5731bdec12a2ec141b`
  now gives every configured iOS provider a **Test Connection** action. Nextcloud uses a
  cancellable authenticated read-only WebDAV probe, including account-root success when
  the configured collection does not exist. LibreSync returns an outbound cycle ID and
  completes only the matching test, including event-before-return, timeout, cancellation
  and provider-transition handling. Neither path changes tasks, pending operations or
  persistent last-sync status.
- Task screens move sync state to a compact 44-point footer after list content and keep
  Add Task in a stable bottom-trailing overlay without a full-width background. One
  root-owned Liquid Glass toast now presents operation results: 3.5-second information,
  5-second exact-batch Undo and persistent focused `SaveFailed`. Projects and tags use
  fully filled saved-color symbols and display canonical unique unfinished-family counts;
  Increase Contrast, Differentiate Without Color and Colorful Labels fallbacks remain.
- Fresh UniFFI preparation passes. The portable iOS lane passes 156 tests in 23 suites.
  The complete native view lane passes 93 tests with two documented opt-in skips on iOS
  27 (`ios/DerivedData/TestReports/views-1789851438050310000.xcresult`) and iOS 26.5
  (`views-1789851495540195000.xcresult`). The real loopback transport lane passes eight
  tests with one hosted-TLS skip on iOS 27
  (`transport-1789851540899726000.xcresult`) and iOS 26.5
  (`transport-1789851590318444000.xcresult`). No XCUI or ViewInspector is present.
- `cargo test --workspace --exclude momentum` passes after granting its isolated CLI
  Unix-socket fixtures local socket access: 23 CLI tests, 129/130 core tests with the one
  manual latency sample ignored, and all remaining workspace/unit/doc tests pass. The
  first sandboxed attempt failed only because five socket fixtures received
  `Operation not permitted`. MomentumKit passes 136 tests in 32 suites after its event
  fixtures adopted the additive LibreSync cycle IDs. The runner's 12 self-tests pass,
  iOS localization is complete at 364/364 German entries and `git diff --check` passes.
- Hosted Nextcloud still requires disposable `MOMENTUM_NEXTCLOUD_TEST_*` credentials;
  separate-device Bonjour/LibreSync and spoken assistive technology remain unverified.
  Loopback transport does not establish those boundaries. No personal sync connection or
  task store was used.
- A warning-free generic simulator build succeeded and the isolated iOS 27 demo was
  installed, launched and visually inspected. It confirms the continuous canvas and
  stable bottom-trailing Add Task capsule in the running app. A signed physical bundle
  also built successfully and reads back bundle `com.codedbydan.Momentum.ios`, version
  `0.4.0`, build `1`. Installation on dip17pm stopped before modifying the app because
  CoreDevice reported that the phone was locked and could not mount its developer disk
  image. No physical-device test or data inspection ran.

### 2026-09-19 — functional compact sidebar correction installed (F-018/F-020/F-039, B-083)

- The prior replacement button changed `TaskWorkspaceState` but did not pop the compact
  split view, matching the user's dip17pm report. Apple documents that column visibility
  is ignored after `NavigationSplitView` collapses into a single stack. The replacement
  button now invokes the detail view's native dismiss action; SwiftUI performs the same
  compact pop as its synthesized Back control and reports `.sidebar` through the existing
  preferred-column binding. The button remains root-only, so nested pushes retain Back.
- The regression now requires rendered behavior instead of accepting model state alone:
  after activating the real leading control, the sidebar marker must become visible, the
  detail marker must disappear and the binding must report `.sidebar`. It fails against
  the previous implementation and passes after the fix on iOS 26.5 and 27. Focused
  results are `ios/DerivedData/TestReports/views-1789843028168990000.xcresult` and
  `views-1789843051432737000.xcresult`; complete current-source gates pass 152 portable
  tests in 23 suites plus 90 native tests with two intentional skips at
  `views-1789843082398583000.xcresult` and `views-1789843116992846000.xcresult`.
- Localization remains complete at 353/353, all 12 runner regressions pass and
  `git diff --check` passes. A warning/error-free signed `0.4.0` build `1` was installed
  over the existing dip17pm app while preserving its container. After the user unlocked
  the phone, CoreDevice launched the corrected build, read back version `0.4.0` build `1`
  and confirmed a live Momentum process. Touch interaction is not inferred from process
  readback; no tests or data inspection ran on the phone.

### 2026-09-19 — compact Today sidebar control installed on dip17pm (F-018/F-020/F-039, B-074/B-083)

- Dirty local `task/ios-app` at base `7ec82e93271fdddfae7dce5731bdec12a2ec141b`
  now gives the compact root `TaskScreen` one leading **Show lists** button with the
  `sidebar.leading` SF Symbol and hides the split view's synthesized Back item. The
  sidebar action is injected only at the workspace root, so task/project/tag pushes keep
  their normal back navigation. English/German localization is complete at 353/353.
- The new public-API hosted regression renders the real compact workspace, verifies one
  actionable leading control and no Back accessibility element, invokes the control and
  observes the sidebar state. Complete current-source gates pass on iOS 26.5 and iOS 27:
  152 portable tests in 23 suites plus 90 native tests with two intentional opt-in skips
  per runtime. Native results are
  `ios/DerivedData/TestReports/views-1789830169875490000.xcresult` (26.5) and
  `ios/DerivedData/TestReports/views-1789830206313302000.xcresult` (27). The focused
  results are `views-1789830122195492000.xcresult` and
  `views-1789830141434207000.xcresult`, respectively.
- XcodeGen exposed B-074's remaining source: its plist properties still generated
  literal version strings. The generator and produced plist now reference
  `$(MARKETING_VERSION)` and `$(CURRENT_PROJECT_VERSION)`; regeneration plus all 12
  runner regressions preserve them. `git diff --check` passes. A warning/error-free
  signed device build reads back bundle `com.codedbydan.Momentum.ios`, version `0.4.0`,
  build `1`, and installs over the existing dip17pm app without replacing its container.
  After the user unlocked the phone, CoreDevice launched the app and confirmed a live
  Momentum process. No tests, fixture population or data inspection ran on the phone;
  device interaction beyond installation and launch is not claimed.

### 2026-09-19 — F-042 installed and launched on dip17pm (F-039/F-042, B-074)

- At the user's explicit request, the current dirty `task/ios-app` source was built for
  the paired physical dip17pm with the existing automatic development-signing identity.
  The production Momentum target was installed over the existing app and launched in the
  foreground. `devicectl` reads back bundle `com.codedbydan.Momentum.ios`, version `0.4.0`,
  build `1`, and a live Momentum process. Existing app-container data was preserved; no
  tests, fixture population, task inspection, reset, commit, push or release was performed
  on the phone.
- The required pre-install bundle readback caught B-074 recurring in the dirty source:
  `Info.plist` had returned to hardcoded `1.0`/`1` despite project settings declaring
  `0.4.0`/`1`. The signed bundle was not installed in that state. The plist again uses
  `$(MARKETING_VERSION)` and `$(CURRENT_PROJECT_VERSION)`, and a new host regression keeps
  those references from drifting. The corrected signed build completed without compiler
  warnings/errors before installation. The local strict trust check reports
  `CSSMERR_TP_NOT_TRUSTED` for the development chain; successful CoreDevice installation
  and launch are the device-acceptance evidence rather than that host trust result.

### 2026-09-19 — adaptive iOS Today startup skeleton (F-019–F-021/F-039/F-042)

- Dirty local `task/ios-app` at base `7ec82e93271fdddfae7dce5731bdec12a2ec141b`
  now replaces only the root Today's first-load spinner with an adaptive skeleton. The
  real native navigation and four-tab hierarchy remains mounted, redacted and disabled,
  so the transition retains production geometry. A section placeholder and five task-row
  placeholders use semantic system fills and scaled metrics; loaded content appears with
  a restrained 0.2-second opacity transition. Later refreshes and non-Today screens retain
  their existing presentation and never replay the startup skeleton.
- `RootView` owns one latched startup phase. Cold Spotlight/Search and reminder/Open Task
  routes permanently supersede the skeleton without consuming or replaying their route;
  a summary that selects Today remains eligible. `StoreUnavailableView` wins immediately,
  and a successful retry returns through skeleton to ready content. A deterministic
  snapshot-loader seam covers these cases without adding a second store or task owner.
- The shimmer schedule is capped at 30 fps and pauses under Reduce Motion, Low Power Mode
  or an inactive scene. Reduce Motion also removes the content fade. Placeholder rows and
  the native tab hierarchy are hidden from accessibility during loading; one localized
  frequently-updating `Loading Momentum` status is exposed instead. The production
  hierarchy returns when the first valid Today snapshot is assigned. English/German
  localization is complete at 352/352.
- Fresh current-source unit gates pass on iOS 27
  (`681FF376-73A1-4EFC-807E-E858C7B5B7C2`) and iOS 26.5
  (`6C0B4007-CD8B-49DD-BF15-9A887BC80D9D`): each runs 152 portable tests in 23 suites
  plus 89 public-`UIHostingController` native tests, with two documented opt-in skips and
  no failures. Result bundles are
  `ios/DerivedData/TestReports/views-1789827764194576000.xcresult` (27) and
  `ios/DerivedData/TestReports/views-1789827801088568000.xcresult` (26.5). The focused
  iOS 27 lifecycle capture is
  `ios/DerivedData/TestReports/views-1789827669345409000.xcresult`; the full bundles retain
  separate loading/ready attachments plus adaptive component output for both runtimes.
- The generic iOS Simulator production build exits successfully with no warning/error
  diagnostics after regenerating the current Rust/UniFFI artifacts. The 12 runner
  regressions pass and continue to reject XCUITest targets, `XCUIApplication` and
  ViewInspector. `git diff --check` and the project-shape scan pass. No physical device,
  personal store, hosted account, commit, push or release was used. Hosted screenshots
  prove component pixels and transitions; their in-process safe-area emulation is not a
  substitute for a physical-device layout or energy claim.

### 2026-09-18 — iOS navigation and interaction correction automation (F-003–F-006/F-010/F-018–F-020/F-023/F-025/F-028/F-037/F-039, B-075–B-082)

- Dirty `task/ios-app` at base `7ec82e93271f` implements eight recorded corrections:
  stable first-snapshot Add Task geometry; an adaptive leading task workspace; atomic
  Quick Add identity and root-sequenced editor routing; observer-only Sync Settings;
  iPad-only shortcut discovery; a live accent checkmark/selected trait; one selection
  affordance per row; and shared Morning & Night grouping limited to Today-style views.
  Restrained selection/success haptic triggers remain preference-controlled and their
  140–220 ms transitions remove nonessential movement under Reduce Motion.
- Both requested QA destinations were available: iOS 26.5
  `6C0B4007-CD8B-49DD-BF15-9A887BC80D9D` and iOS 27
  `681FF376-73A1-4EFC-807E-E858C7B5B7C2`. Fresh
  `python3 ios/scripts/test.py prepare` regenerated UniFFI Swift and the Apple XCFramework.
  The complete `unit` lane passed on each destination: 152 portable tests in 23 suites,
  then 80 hosted SwiftUI tests with 2 intentional opt-in skips and no failures. Native
  results are
  `ios/DerivedData/TestReports/views-1789782845258366000.xcresult` (26.5) and
  `ios/DerivedData/TestReports/views-1789782886345777000.xcresult` (27).
- The runner's 10 self-tests passed and continue to reject an XCUITest target,
  `XCUIApplication` dependency or retired UI-test directory. No XCUI or ViewInspector
  test was added or run. `swift test --package-path ios/Packages/MomentumMobile` passed
  152 tests/23 suites and `swift test --package-path macos/Packages/MomentumKit` passed
  136 tests/32 suites; each has its documented unavailable nearby-runtime suite skip.
- `cargo test -p mo --test cli` passed 23/23 after rerunning outside the filesystem
  sandbox so its local Unix sockets were permitted. Both
  `cargo test --workspace --exclude momentum` and the `--all-features --all-targets`
  variant passed; the manual latency sample remained ignored by design and the only
  diagnostic was the existing unused-`mut` warning in `sp-store` tests.
  `macos/scripts/build-core.sh --debug` completed all macOS/device/simulator slices.
  `build-aux/test.sh` could not start because this Mac has no `flatpak` executable, so
  Linux runtime behavior remains unverified rather than passed.
- `xcodegen generate --spec ios/project.yml` succeeded. Unsigned Debug Momentum builds
  reported `BUILD SUCCEEDED` for both QA simulator destinations. `git diff --check`
  passed, and every Swift file in the untracked iOS tree plus every tracked modified
  Swift file passed `swiftc -parse`.
- Remaining acceptance is deliberate: no physical-device installation/test occurred;
  no whole-app XCUI flow was used; and the affected flows still need visual/manual
  simulator inspection in portrait/landscape, light/dark, normal/Reduce Motion,
  representative Dynamic Type, iPhone/iPad shortcut presentation, leading-edge sidebar
  interaction, Quick Add sheet handoff, Sync back navigation, accent selection and
  selection-mode touch/assistive behavior. Existing F-020 spoken/alternate-input and
  F-023 external-key boundaries remain open; prior Verified feature scopes were not
  downgraded.

### 2026-09-18 — iOS native task transfer and clipboard import (F-011–F-015, B-063–B-064)

- Added `MomentumDragDropFoundationTests`, a focused shipping-app scheme with a reset
  disposable store. Its real gesture selects one task and then a two-task selection,
  drags the selection-mode SF Symbol handle onto a shipping row destination, verifies
  relative ordering and uses the visible Undo action after each move.
- The first iOS 27 native run reproduced B-063: SwiftUI List edit mode consumed the
  selection gesture before the row `.draggable` session began. Selection mode now uses a
  44-point UIKit `UIDragInteraction` handle that publishes the same custom-UTI JSON and
  plain-text proxy as shared `TaskTransfer`. Ordinary rows retain their SwiftUI drag.
- Added a compact, horizontally scrolling destination strip above the selection actions.
  Today, Morning, Evening, each project and each tag are 44-point SF Symbol targets that
  accept the same `TaskTransfer`, provide equivalent accessible buttons and animate
  targeting only when Reduce Motion permits it. A two-task payload reaches a project;
  separate native payloads reach a tag and Morning on iOS 26.5/27. F-011 and F-013 are
  Verified for iOS.
- The first long-distance destination runs exposed B-065: the redundant row context menu
  captured the selection-mode drag before it reached the strip. Context menus are now
  limited to ordinary browsing; selection mode keeps the same commands in Actions.
- The clipboard case copies content inside Momentum so iOS 26.5 and 27 exercise the
  app-owned system pasteboard. It verifies no pasteboard read when Quick Add appears,
  insertion at the native cursor, selected-text replacement, two-task multiline import,
  URL title normalization and exact original URL Notes.
- That case exposed B-064: shared URL and paragraph import discarded its source `View`,
  putting a successful Today import into Inbox. A new failing core assertion and the
  shipping failure in `/tmp/momentum-paste27-r2.xcresult` captured the defect. The
  view-aware notes helper now applies every parser branch through the supplied source;
  the public explicit notes API retains its existing behavior.
- The original combined schemes pass 2/2 with no failures or skips: iOS 26.5 in 145.198s
  (`/tmp/momentum-drag-drop26-final.xcresult`) and iOS 27 in 142.540s
  (`/tmp/momentum-drag-drop27-final-v2.xcresult`). The expanded three-test iOS 27 scheme
  passes in 254.406s (`/tmp/momentum-drag-drop27-destinations-final.xcresult`); after
  correcting the reorder gesture to target the row body rather than its nested drag
  source, the final iOS 27 scheme also passes in 260.529s
  (`/tmp/momentum-drag-drop27-destinations-final-r2.xcresult`). Its project/tag/day case
  moves two project tasks plus one tag and one Morning task. The
  corrected iOS 26.5 reorder target passes independently in 55.397s
  (`/tmp/momentum-drag-reorder26-focused-r2.xcresult`), and the complete three-test iOS
  26.5 scheme passes in 267.075s
  (`/tmp/momentum-drag-drop26-destinations-final-r2.xcresult`). F-011–F-013 are Verified
  for iOS. Real cross-app
  text/URL drag, assistive Move Up/Down invocation and marked-text IME acceptance remain
  open under F-014/F-015/F-020.
- Focused shared-core checks pass all six reorder tests plus destination routing, selected
  drag and paste/drop parsing. Generated-core preparation passes in 3.725s. The full fast
  mobile lane passes 125 tests across 21 suites in 9.267s (4.484s framework time). Native
  views pass 48 tests with two intentional opt-in skips on iOS 26.5
  (`views-1789733164414563000.xcresult`, 10.368s) and iOS 27
  (`views-1789733179955910000.xcresult`, 6.123s). Only simulators were used; dip17pm and every
  other physical device were excluded. No account, network, commit, push or release.
- The broader sandboxed `cargo test -p momentum-core --lib` run discovered 126 tests:
  105 passed, the manual latency sample was ignored and 20 IPC/Nearby/loopback transport
  cases were rejected with `Operation not permitted`. The scoped parser, destination and
  reorder regressions remained green; this environment result is not a core pass.

### 2026-09-18 — iOS completion, archive and bulk-selection acceptance (F-009/F-010, B-062)

- Added `MomentumArchiveFoundationTests`, a focused shipping-app scheme using the
  disposable demo store. It completes, reopens and completes a task again; checks that
  Archive Completed is disabled before it applies and enabled afterward; archives the
  result; and verifies one Undo restores the batch.
- The auto-archive case creates a child through the production editor, enables the real
  Task Lists preference, completes the parent, and verifies the top-level family in the
  read-only Archive. It finds the archived child through Search with its parent context
  and no Open/Complete controls, then proves one Undo restores parent and child together.
- A third shipping case uses touch selection to bulk-complete and bulk-delete every
  visible Today task, Undo each batch, switch lists without leaking the selection, exit
  with Done Selecting and confirm Archive offers no selection controls. The complete
  three-test scheme passes on iOS 27 in 143.082s
  (`/tmp/momentum-archive-selection27-final.xcresult`) and iOS 26.5 in 147.787s
  (`/tmp/momentum-archive-selection26-final.xcresult`), with no failures or skips.
  Only simulators were used; dip17pm and every other physical device were excluded.
- Shared-core coverage now explicitly combines `archiveYoung` and `archiveOld` in newest
  completion order while preserving read-only task detail. Existing regressions verify
  100-row paging, family and multi-task restoration, and automatic family archiving.
- All eight archive-filtered core tests pass. After the final view-state fix,
  generated-core preparation passes in 2.851s and the fast mobile lane passes all 125
  tests across 21 suites in 8.644s (4.045s framework time). The native lane passes 48
  tests with two intentional opt-in skips on iOS 27 (2.928s framework,
  `views-1789728053774684000.xcresult`) and iOS 26.5 (2.841s framework,
  `views-1789728071928713000.xcresult`). Swift parsing, Rust formatting, diff whitespace,
  every generated scheme XML file and Xcode scheme discovery also pass.
- A broader sandboxed core run passed 105 tests, ignored the manual latency sample and
  rejected 20 unrelated IPC/nearby/loopback cases with `Operation not permitted`; the
  scoped archive suite remained green. LibreSync work and physical-device testing were
  not resumed.
- Expanded organization acceptance applies both an existing tag and a newly named tag
  to a selected parent/subtask family. The first iOS 27 run exposed B-062: New Tag applied
  correctly but retained selection, unlike every sibling action. The production alert now
  clears selection after the awaited shared-engine result. The full flow passes on iOS 27
  in 118.447s (`/tmp/momentum-organization-new-tag27-r2.xcresult`) and iOS 26.5 in
  118.510s (`/tmp/momentum-organization-new-tag26-final.xcresult`). Existing native
  planning and family-move matrices plus shared-core batch tests complete the remaining
  F-010 action evidence; keyboard dispatch issues B-046/B-047 remain under F-023.

### 2026-09-18 — iOS notification permission boundaries (F-007/F-038, B-011)

- Added two focused shipping-app schemes around the real notification authorization
  prompt and the disposable Debug system store. Denial requires `Off in iOS Settings`
  and the Open Settings recovery action. An independent fresh grant requires `Allowed`
  plus the native planning horizon. Both clear the temporary system-test marker and do
  not enable sync, Spotlight or a personal task store.
- Both boundaries pass through their final split schemes on iOS 27: denial 20.697s
  (`/tmp/momentum-notification-permission-denial-scheme27.xcresult`) and grant 20.645s
  (`/tmp/momentum-notification-permission-grant-scheme27.xcresult`). They also pass on iOS
  26.5: denial 17.479s (`/tmp/momentum-notification-permission-denial26.xcresult`)
  and grant 17.612s (`/tmp/momentum-notification-permission-grant26.xcresult`). Each
  bundle contains one executed test with no failure or skip.
- Existing portable coordinator tests verify cancellation while unauthorized, edit
  coalescing, explicit cancellation, crash/restart and failed-readback recovery. Shared
  Rust tests cover the seven-day horizon and 59/60/61-request capacity edges. Those
  checks complement the native permission state without duplicating product rules in
  Swift.
- The installed Simulator toolchain lists no notification service for `simctl privacy`.
  Its Settings switch also rejected the cross-app XCTest tap after a real denial, even
  though the recovery destination was reachable. The two native runs therefore prove
  the denied and granted boundaries independently, not an uninterrupted Settings
  transition. That transition remains manual/physical acceptance. Only simulators were
  used; dip17pm and every other physical device were excluded.
- Final generated-core preparation passes in 2.827s. The fast lane then passes all
  125 tests across 21 suites in 9.133s (4.482s framework time). Swift parsing, every
  generated scheme XML file, Rust formatting, diff whitespace and Xcode scheme
  discovery also pass.

### 2026-09-18 — iOS planning and recurrence acceptance (F-007/F-008)

- Added `MomentumPlanningFoundationTests`, a focused shipping-app scheme backed by a
  fresh disposable store. Its reminder case enables the default 09:00 time, opens and
  selects None, at time, 5/10/15/30 minutes, 1 hour and 1 day before, saves one day,
  reopens, then clears time and verifies the task remains scheduled as a plain due day
  with no reminder.
- Its recurrence case switches the native picker through Daily, Yearly and Monthly,
  verifies singular interval labels, visits same-date and last-day monthly rules, saves
  a paused last-Friday rule, reopens the exact description/state and stops recurrence.
  Its validation case clears the default Monday-Friday selection, receives the core's
  `Pick at least one weekday` error, selects Monday and saves successfully.
- The complete three-test scheme passes with no failures, skips or runtime warnings on
  iOS 26.5 (`/tmp/momentum-planning-foundation26-final.xcresult`, 3/3, 176.212s). The
  same cases pass individually with no failures, skips or runtime warnings on iOS 27:
  reminder 69.251s (`/tmp/momentum-planning-foundation27-reminder-r1.xcresult`), repeat
  controls 70.401s (`/tmp/momentum-planning-foundation27-repeat-r1.xcresult`) and weekly
  validation 38.550s (`/tmp/momentum-planning-foundation27-weekly-r1.xcresult`). No
  physical device, account, network or personal task store was used.
- The retained portable suite already round-trips cycle, interval, start date, pause and
  each monthly rule. The full shared-core pass in this worktree verifies day-move
  time/reminder clearing and Undo, deterministic recurrence identities, weekly/monthly/
  yearly rules and newest-missed catch-up: 124 tests pass with one intentional manual
  performance sample ignored, followed by 21/21 notification-ledger tests.
- The localized editor case enters 14:30 with German 24-hour wheels, saves into the
  disposable demo store, relaunches in English, verifies the same hour/minute, clears
  time and reopens the task with its scheduled day intact. It passes on iOS 27
  (`/tmp/momentum-planning-locale27-r5.xcresult`, 1/1, 34.794s) and iOS 26.5
  (`/tmp/momentum-planning-locale26-r1.xcresult`, 1/1, 32.597s). Stable editor
  accessibility identifiers keep the test independent of translated labels.
- After that production-source change, preparation passes in 3.04s; the fast suite
  passes 125 tests/21 suites in 4.333s framework time (8.898s lane). Native views pass
  48 tests with two intentional opt-in skips on iOS 27 (2.922s framework,
  `views-1789723645534160000.xcresult`) and iOS 26.5 (2.873s framework,
  `views-1789723661320383000.xcresult`). Only simulators and disposable stores were used.

### 2026-09-18 — iOS source-aware task creation acceptance (F-006/F-020, B-061)

- `MomentumTaskCreationContextTests` launches the shipping app with the disposable demo
  seed and enters More Details from Today, Home, urgent, Morning and Evening. It verifies
  the scheduled Today default, selected project and selected tag before creation, then
  proves each created task remains in its source destination. Morning and Evening also
  verify the scheduled default before the Rust core applies the exclusive slot tag.
- The initial iOS 27 run passed the Today, tag and both day-period paths but could not
  inspect the selected project: the native picker exposed an empty accessibility value.
  `TaskEditor` now gives that picker a stable identifier and the visible selected project
  as its accessibility value; the focused Home regression then passed.
- The complete three-test shipping scheme passes without failures, skips or runtime
  warnings on iOS 27 (`/tmp/momentum-task-creation-context27-final.xcresult`, 3/3,
  76.019s test operation) and iOS 26.5
  (`/tmp/momentum-task-creation-context26-final.xcresult`, 3/3, 76.733s). The pre-fix
  accessibility failure is retained in
  `/tmp/momentum-task-creation-context27-red-r2.xcresult`. No physical device, network,
  account or personal task store was used.
- A fourth focused shipping flow creates a complete draft, chooses Home, retains the
  Today schedule, enters a 90-minute estimate, selects `urgent`, creates `Zebra`, `Alpha`
  and `Middle`, and enters multiline notes. It then terminates the app, relaunches the
  same disposable store without reseeding it, finds the task in Search and verifies every
  value after reopening. It passes on iOS 27
  (`/tmp/momentum-task-editor-mapping27-r5.xcresult`, 1/1, 57.236s test) and iOS 26.5
  (`/tmp/momentum-task-editor-mapping26-r1.xcresult`, 1/1, 59.142s test operation), with
  no skips or runtime warnings. The portable `fullFormRoundTripsAndPreservesTagOrder`
  regression in the complete fast pass separately proves retained and newly created tag
  ordering at the shared model/core boundary.
- The editor safety flow saves `Original note`, reopens the task, uses Copy the note and
  Command-V to prove the system pasteboard received the exact note, then cancels that
  pasted title and confirms the original task remains. A final reopen clears the entire
  title and verifies Save becomes disabled. It passes without failures, skips or runtime
  warnings on iOS 27 (`/tmp/momentum-task-editor-safety27-r2.xcresult`, 1/1, 60.176s
  test operation) and iOS 26.5 (`/tmp/momentum-task-editor-safety26-r1.xcresult`, 1/1,
  61.500s).
- The task-family action flow selects the demo parent, invokes Copy Title through the
  shipping keyboard command and proves its exact system-pasteboard value in the editor.
  It adds `Bring insurance card`, moves the selected parent with the native project
  sheet, confirms parent and child in Momentum, then duplicates the parent through the
  editor. It passes without failures, skips or runtime warnings on iOS 27
  (`/tmp/momentum-task-family-actions27-r2.xcresult`, 1/1, 48.600s test) and iOS 26.5
  (`/tmp/momentum-task-family-actions26-r1.xcresult`, 1/1, 49.163s). The new Rust
  regression proves the child inherits its parent's project and that moving and undoing
  the parent changes and restores the whole family. The full `momentum-core` run passes
  124 tests with one intentional manual performance sample ignored; all 21 integration
  tests pass. No physical device or personal task store was used.
- Final generated-core preparation passes in 3.028s. The portable lane passes 125
  tests/21 suites in 4.501s framework time (9.046s lane). Native SwiftUI/model suites
  execute 48 tests with two intentional availability skips and no failures on iOS 27
  (2.892s framework; `ios/DerivedData/TestReports/views-1789720557347349000.xcresult`)
  and iOS 26.5 (2.898s framework;
  `ios/DerivedData/TestReports/views-1789720574287184000.xcresult`).

### 2026-09-18 — iOS Quick Add parser and autocomplete acceptance (F-005/F-039, B-060)

- `MomentumQuickAddFoundationTests` launches the shipping app against an empty,
  disposable store. One flow creates an existing tag, enters
  `Prepare launch #existing #fresh 1h 30m`, and proves the Rust parser strips the
  metadata from the title while retaining the 90-minute estimate and both tag
  relationships across relaunch.
- The autocomplete flow creates two overlapping tag names and verifies the first
  selection, Down Arrow movement, Tab acceptance, native Return input and touch
  acceptance. The multiline UIKit editor preserves Unicode-scalar cursor positions and
  gives autocomplete priority commands for Up/Down, Tab, Return and Escape. Suggestions
  expose selected state and never require color to communicate selection.
- The complete two-test scheme passes on iOS 27
  (`/tmp/momentum-quick-add-foundation27-r2.xcresult`, 2/2, 80.567s) and iOS 26.5
  (`/tmp/momentum-quick-add-foundation26-r2.xcresult`, 2/2, 78.330s). Focused legacy
  rejected-input retry, Unicode replacement and More Details parsing pass 3/3 on iOS 27
  (`/tmp/momentum-quick-add-regression27-r1.xcresult`, 38.250s).
- XCUITest on these simulators does not deliver direct Return/Escape `typeKey` events to
  the focused multiline text view. Return is therefore exercised through its native
  text-input delegate path; the in-process native lane verifies priority Return/Escape
  responder commands are present and dispatch their actions. Arrow and Tab use hardware
  key injection in the shipping app. No physical device was used, per user instruction.
- Final generated-core preparation passes in 3.117s. The portable lane passes 125
  tests/21 suites in 4.039s framework time (9.065s lane). Native view/model suites execute
  48 tests with two intentional availability skips and no failures on iOS 27 (2.918s;
  `ios/DerivedData/TestReports/views-1789719784251416000.xcresult`) and iOS 26.5
  (2.796s; `ios/DerivedData/TestReports/views-1789719810528016000.xcresult`). No account,
  personal store, commit, push or release was used.

### 2026-09-18 — iOS project/tag organization and saved-color acceptance (F-004/F-037, B-059)

- `MomentumOrganizationFoundationTests` launches the shipping app with an empty,
  disposable store. It creates a custom-color project and tag, creates a parent plus
  subtask in the project, proves both remain independently selectable as `Actions (2)`,
  applies the tag and verifies the complete task family in both destinations.
- Project/tag symbols in Lists and project/tag grouped headings now render the saved
  color instead of a fixed secondary color. Colorful Labels off, Increase Contrast and
  Differentiate Without Color use the readable secondary-text fallback. Native trailing
  swipe actions expose the existing project/tag editors without relying on a long press.
- The flow opens both native editors to verify the saved custom-color state, terminates
  and relaunches without resetting the store, repeats the saved-state checks, confirms
  the project deletion dialog reports exactly two tasks, deletes the disposable project
  and verifies the tag remains editable. An inspected retained iOS 27 screenshot shows
  the saved-color symbols on one uniform grouped canvas:
  `/tmp/momentum-org-r7-export/8EA604E8-0352-4B47-8A1D-DFDDD220CFCA.png`.
- The exact flow passes on iOS 27
  (`/tmp/momentum-organization-foundation27-r7.xcresult`, 1/1, 100.758s) and iOS 26.5
  (`/tmp/momentum-organization-foundation26-r1.xcresult`, 1/1, 102.163s test time;
  104.118s session time). Earlier iOS 27 iterations corrected only the XCTest switch
  interaction and cleanup path; the retained run is the final product evidence. No
  physical device, account, personal store, commit, push or release was used.
- Generated Apple-core preparation passes in 3.389s. The terminal portable lane passes
  125 tests/21 suites in 4.485s framework time (9.088s including the command/build
  boundary). Native SwiftUI/model suites execute 47 tests with two intentional
  availability skips and no failures on iOS 27 (2.946s framework, 10.331s lane;
  `ios/DerivedData/TestReports/views-1789717384497012000.xcresult`) and iOS 26.5
  (2.809s framework, 6.022s lane;
  `ios/DerivedData/TestReports/views-1789717402876038000.xcresult`). The first portable
  attempt was denied access to the host Swift cache by the sandbox; the identical
  authorized command is the retained pass.
- The final management expansion also renames both contexts, verifies their custom
  colors after editing, prevents Inbox deletion, confirms tag deletion leaves the task
  family in its project, and checks the surviving project/new tag after cold relaunch.
  It then confirms project deletion reports exactly two affected tasks.
- B-066 fixes a standalone New Tag edge where Rust correctly resolved `OUTDOORS` to an
  existing `outdoors` ID but Swift then renamed that tag and cleared its color. The
  mobile actor now reports whether the Rust-returned ID was already present; no identity
  rule moved out of Rust. The focused regression passes, organization package tests pass
  7/7, and the full fast lane passes 126 tests/21 suites in 4.396s framework time.
- The expanded shipping flow passes on iOS 27 in 180.056s
  (`/tmp/momentum-organization27-final-r2.xcresult`) and iOS 26.5 in 187.383s
  (`/tmp/momentum-organization26-final-r2.xcresult`). F-004 is Verified for the approved
  simulator scope. No physical device, account, personal store, commit, push or release
  was used.

### 2026-09-18 — iOS Upcoming range and grouping acceptance (F-003)

- `MomentumUpcomingFoundationTests` launches the shipping app with the disposable demo
  seed and proves the default seven-day window includes Tomorrow while excluding a
  task 21 days away. The Tomorrow row keeps its localized due-day accessibility value
  under Morning & Night, None, Project, Tag and Time Estimate grouping.
- The same flow selects Next 30 Days through the shipping Settings screen, confirms the
  day-21 task appears with its project and date still exposed, then terminates and
  relaunches the app without resetting its store to prove the selected range persists.
  The disposable demo gained only the day-21 task needed to distinguish the ranges.
- The focused flow passes on iOS 27
  (`/tmp/momentum-upcoming-foundation27-r1.xcresult`, 1/1, 88.547s) and iOS 26.5
  (`/tmp/momentum-upcoming-foundation26-r1.xcresult`, 1/1, 82.139s). The initial
  sandboxed 26.5 invocation could not access CoreSimulator or Swift caches and exited
  before launch; the authorized identical simulator command is the retained passing
  evidence. No physical device, account, personal store, commit, push or release was used.
- Generated Apple-core preparation passes in 3.082s. The affected Rust package passes
  123 library and 21 notification-ledger tests, with its manual latency benchmark still
  intentionally ignored. The terminal portable iOS lane passes 125 tests/21 suites in
  4.408s framework time (9.689s including the command/build boundary).

### 2026-09-18 — iOS grouping, sorting, search and Archive closure (F-016/F-018/F-037)

- The shared Rust suite already owns the projection rules: five exclusive grouping
  choices, first-tag and family placement, six estimate intervals and exact boundaries,
  missing/duplicate contexts, saved colors, all five sort keys and directions, every-word
  search across title/notes/project/tags, live/archive children, caps and truncation notes.
  No duplicate Swift rule tests were added.
- `MomentumUpcomingFoundationTests` now drives Manual Order, Title, Due Day, Estimate and
  Created through the shipping Settings UI in both Ascending and Descending order, checks
  a known row pair after each selection, and proves None/Created/Descending persist after
  a cold relaunch. Its existing flow still selects every grouping and persists the 30-day
  range. Both tests pass on iOS 27 in 318.085s
  (`/tmp/momentum-presentation-upcoming27-r1.xcresult`) and iOS 26.5 in 304.594s
  (`/tmp/momentum-presentation-upcoming26-r1.xcresult`).
- `MomentumTabStateTests` now verifies the cleared Search prompt, `orca contrast`
  every-word matching, rapid replacement without a stale row, project and tag result
  navigation, and returning to the cleared prompt. The first run exposed only that a
  lazy project section needed scrolling after matching task rows; the corrected test
  passed on iOS 27 in 37.828s
  (`/tmp/momentum-presentation-search27-r2.xcresult`). The complete two-test suite passes
  on iOS 26.5 in 91.547s (`/tmp/momentum-presentation-search26-r1.xcresult`); its existing
  cross-tab state case also passed on iOS 27 in the earlier combined run before its result
  bundle was interrupted during failure cleanup.
- The complete three-test shipping Archive suite passes on iOS 27 in 143.296s
  (`/tmp/momentum-presentation-archive27-r1.xcresult`) and iOS 26.5 in 145.075s
  (`/tmp/momentum-presentation-archive26-r1.xcresult`). It proves read-only archived
  parent/child results, complete/reopen, auto-archive, list-scoped selection and Undo.
- Current native state/row evidence remains applicable: empty/all-done, offline,
  persistent error/retry, compact Add/tab placement and complete metadata semantics have
  dedicated shipping/view/accessibility tests recorded under B-015, B-016, B-048 and
  F-028. F-016 and the iOS column of F-037 are Verified for the approved simulator scope.
  Spoken VoiceOver, external input and physical-device behavior remain separate; no
  physical device, account, personal store, commit, push or release was used.

### 2026-09-18 — iOS Today and day-period foundation acceptance (F-003)

- `MomentumTodayFoundationTests` launches the shipping app with the disposable demo
  seed. It confirms an unfinished overdue task stays in Today with its local due-date
  value, the 15:30 task exposes the simulator's native 12/24-hour value, and completing
  an open task moves it beneath `Completed (1)` with a Reopen action.
- The flow then opens the only Morning task, selects it through the native list action
  menu and moves it to Evening. The vanished Morning screen dismisses back to Lists,
  its stable day-period destination disappears without confusing the separate Morning
  tag, Evening remains, and the same task appears in the Tonight destination. Stable
  `list-morning`/`list-evening` accessibility identifiers distinguish those native
  day-period links from identically named user tags without changing visible UI.
- The focused flow passes on iOS 27
  (`/tmp/momentum-today-foundation27-r7.xcresult`, 1/1, 27.893s) and iOS 26.5
  (`/tmp/momentum-today-foundation26.xcresult`, 1/1, 27.466s). Earlier r1-r6 bundles
  refined locale-safe clock matching, the counted Completed heading, duplicate visible
  names and the test interaction path; one long-press probe exposed an iOS 27 context-menu
  animation wait, so final acceptance uses the app's ordinary selection menu. No physical
  device, account, personal store, commit, push or release was used.

### 2026-09-18 — iOS independent tab state and shared refresh acceptance (F-001/F-002/F-004)

- `MomentumTabStateTests` drives the shipping app through the exact Today, Upcoming,
  Search and Settings tab bar. It leaves Today on Archive, Settings on About Momentum
  and Search with `Cross tab`, then verifies each state survives switches and mutations.
  Creating `Cross tab refresh` in Today refreshes the retained Search results; completing
  it in Search refreshes Today to Reopen while the query and navigation stacks remain.
  The flow uses native Search submission to dismiss the software keyboard before tab
  selection and a tab-bar coordinate fallback only when XCTest retains a stale hit point.
- The focused flow passes on iOS 27
  (`/tmp/momentum-tab-state27-r8.xcresult`, 1/1, 54.758s) and iOS 26.5
  (`/tmp/momentum-tab-state26.xcresult`, 1/1, 52.406s). Earlier r1-r7 bundles were
  harness probes for keyboard/tab-bar hit testing; their temporary product experiments
  were reverted and are not recorded as app defects.
- The exact iOS 27 simulator product used by the acceptance run links UIKit/SwiftUI and
  no AppKit according to `otool -L`; its symbol table contains none of MomentumKit's
  desktop AppState, CLI socket or external-change service symbols. On the same dirty
  revision, MomentumKit passes 136 tests/32 suites in 12.265s and the macOS Momentum
  Debug app builds successfully with Xcode 27 after rebuilding all shared-core Apple
  slices. No physical device, personal data, commit, push or release was used.

### 2026-09-18 — iOS offline create/edit persistence acceptance (F-001/F-002/F-006)

- Refreshed shipping-UI acceptance now covers both sides of the first foundation
  persistence case. One flow creates a task, completes and undoes it, force-terminates
  Momentum and finds the active task after relaunch. The second creates and edits a task,
  saves it, deletes and undoes it, force-terminates Momentum and finds the edited title
  after relaunch. The portable `offlineTaskPersistsAcrossReopen` regression separately
  opens the same isolated on-disk store through a new engine worker.
- Both UI tests pass 2/2 on iOS 27
  (`/tmp/momentum-offline-persistence27.xcresult`, 53.203s) and iOS 26.5
  (`/tmp/momentum-offline-persistence26.xcresult`, 48.894s) using the freshly built
  Debug app and disposable test store. Sync is unavailable in this mode, so the result
  establishes account-free local persistence without relying on transport. It does not
  claim crash-during-write recovery, physical-device storage behavior or the remaining
  broader editor/parity cases. No device, personal data, commit, push or release was used.

### 2026-09-18 — iOS morning-summary atomic editing and real delivery (F-038, B-058)

- The first real iOS 27 summary run exposed B-058: changing an inline hour/minute wheel
  wrote each component directly to preferences. Reconciliation accepted a past
  intermediate time and the shared at-most-once ledger then correctly rejected the
  intended final time for the same day. The pre-fix result is
  `/tmp/momentum-notification-summary27-r2.xcresult`.
- Settings now presents a native medium time sheet with SF Symbol Cancel/Save actions.
  Its draft remains local until Save commits the final hour and minute together. The
  summary preference observer, Rust eligibility rules and persisted daily ledger are
  unchanged. Focused Settings suites verify badge/summary separation, opt-in persistence
  and their existing contrast boundary on iOS 27
  (`/tmp/momentum-notification-settings27-atomic.xcresult`, 2/2) and iOS 26.5
  (`/tmp/momentum-notification-settings26-atomic.xcresult`, 2/2).
- `MomentumNotificationSummarySystemTests` uses the shipping notification adapter and an
  isolated Debug store. One Today task produces a real **Good morning** SpringBoard
  notification with `Today: 1 · Morning: 0 · Tonight: 0`; tapping it returns to Today,
  and a cold relaunch retains the opt-in and exact saved time. The flow passes on iOS 27
  (`/tmp/momentum-notification-summary27-r3.xcresult`, 100.790s) and iOS 26.5
  (`/tmp/momentum-notification-summary26.xcresult`, 81.800s).
- Final preparation passes in 3.154s; portable checks pass 125 tests/21 suites in
  4.378s framework time (9.156s lane).
  All 47 native view/model tests pass with the two existing intentional availability
  skips on iOS 27 (`views-1789712354629536000.xcresult`) and iOS 26.5
  (`views-1789712369303784000.xcresult`). The iOS 27 diagnostic accessibility traversal
  now opens the staged time sheet and completes all 32 screens
  (`/tmp/momentum-summary-accessibility27-r2.xcresult`, 300.124s). Its 117 raw findings
  remain tracked diagnostic candidates and are not presented as an accessibility pass.
  No device, personal store, sync account, commit, push or release was used. F-038 remains
  **In progress** only for its separately tracked background/locked-device/physical acceptance.

### 2026-09-18 — iOS notification background replenishment (F-007/F-038)

- Momentum now registers `com.codedbydan.Momentum.ios.notifications.refresh` before
  application launch completes, declares the permitted identifier and `fetch` background
  mode, and requests a short discretionary `BGAppRefreshTask` when the app enters the
  background and notification authorization permits scheduling. The request has a
  one-day minimum cadence; iOS can defer it based on system conditions.
- The handler opens the existing shared engine owner, reconciles only the local
  notification plan, installs its expiration callback before work starts, reports a
  cancelled or issue-bearing reconciliation as failed, and requests the next wake. It
  does not start Nextcloud or any other network sync. Preview, UI-test and Debug system
  acceptance stores cannot request background work.
- iOS 27 uses the new async scheduler submission API; iOS 26 uses its supported legacy
  submission path. Five portable policy tests cover allowed/denied authorization, the
  no-current-request case needed for the bounded horizon, cadence and completion. The
  full portable suite passes 125 tests/21 suites (4.061s framework time), and `prepare`
  passes in 3.097s.
- All 47 native view/model tests pass with two intentional availability skips on iOS
  26.5 (`views-1789711162778599000.xcresult`) and iOS 27
  (`views-1789711173935765000.xcresult`). Production app builds and isolated simulator
  installs/launches also pass on both runtimes, including launch-time task registration.
- Apple's Simulator does not execute `BGAppRefreshTask` launches. System-triggered
  suspended execution, timing and expiration therefore remain unverified and explicitly
  user-deferred with physical-device testing. dip17pm was not used. F-007/F-038 remain
  **In progress**; no commit, push or release was performed.

### 2026-09-18 — iOS system notification delivery/actions and cold callback crash (F-007/F-038, B-057)

- A live iOS 26.5 simulator reminder delivered through SpringBoard and exposed the
  registered **Done** action. The action mutated the isolated shared store and removed
  delivery, but the cold-launched app then crashed while UIKit saved background state.
  The exception required the call on the main thread. Source and stack inspection traced
  this to the async `UNUserNotificationCenterDelegate` bridge resuming UIKit's completion
  after leaving the main actor, not to the Rust mutation or notification plan.
- `MobileAppDelegate` now uses the completion-handler delegate form and performs the
  shared-engine action, delivery removal and UIKit completion on `MainActor`. The
  Objective-C completion is wrapped only for that one actor transfer. Invalid routes and
  actions also complete on the main actor. No task rule moved out of Rust.
- A Debug-only `--system-notification-testing` marker gives OS cold launches the existing
  disposable QA store with live notifications while keeping sync and Spotlight disabled.
  Release ignores the marker; teardown clears it. A focused model regression verifies
  arm, cold reuse and clear behavior so the system test cannot route into user data.
- `MomentumNotificationSystemTests` drives the native editor, real permission state,
  calendar-trigger delivery and actual Notification Center actions. Cold task-body
  activation, cold **Done** and resident **Snooze 1 hour** pass **3/3** on iOS 26.5 in
  `/tmp/momentum-notification-system-final26.xcresult` (343.071s) and iOS 27 in
  `/tmp/momentum-notification-system-final27.xcresult` (340.348s). The body opens the
  exact task editor, Done reaches the completed Today state after termination, and
  Snooze removes delivery while leaving the task open. Portable action tests separately
  verify one shared engine owner, deduplication, the exact one-hour replacement request
  and Done's shared auto-archive behavior.
- Final gates on the regenerated project/core pass: `prepare` in 2.828s; 120 portable
  mobile tests in 20 suites; and 47 native view/model tests on each runtime with the two
  intentional availability skips and no failures
  (`views-1789709961898217000.xcresult` on iOS 26.5 and
  `views-1789709971527465000.xcresult` on iOS 27). The result bundles report no runtime
  warnings. The native lanes compile the final Swift 6 callback bridge.
- This closes native simulator action routing and B-057's reproduced crash scope. It does
  not establish locked-device, Focus, physical-device or background-replenishment
  behavior. Physical testing remains explicitly user-deferred, and dip17pm was not used.
  F-038 remains **In progress**. No commit, push or release was performed.

### 2026-09-17 — iOS system App Intents acceptance (F-031, B-056)

- Actual Shortcuts.app acceptance now invokes all five Momentum App Shortcuts on both
  supported simulator runtimes. Discovery/Create, Find, Complete, Plan for Today and
  Open Task pass **5/5** on iOS 26.5 in
  `/tmp/momentum-appintents-all-actions26.xcresult` (147.090s) and **5/5** on iOS 27 in
  `/tmp/momentum-appintents-all-actions27.xcresult` (180.360s), with no failures,
  skips or runtime warnings. Create persists after relaunch; Complete changes the real
  task; Plan selects an unscheduled task and moves it to Today; Open foregrounds the
  exact editor; cold Find returns the exact task through a visible system dialog.
- The first Plan probe reproduced B-056: `MomentumTaskQuery` suggested `.today`, so
  the system picker could not offer an unscheduled task. Suggestions now use the
  shared `.all` active-task scope. Find also returns a localized count/title dialog
  while preserving its machine-readable task-entity array. English and German app
  catalogs contain the new result strings.
- A Debug-only `--system-app-intents-testing` mode keeps system-launched cold intents
  on the disposable UI-test store across process launches. It disables notifications,
  Spotlight and sync through the existing isolated launch policy; Release ignores the
  marker, and test teardown clears it. The focused launch-mode integration regression
  passes on iOS 27. Simulator logs show Plan and Find executing without foregrounding
  Momentum; cold Find passes after explicitly terminating the app.
- The iOS 26 discovery failure during this pass was a test defect: the global **All
  Shortcuts** preview exposes only Momentum's first four tiles, and the helper mistook
  that row for the provider page. Generated metadata contained all five actions. The
  helper now requires the **Momentum** navigation page; focused discovery/Open passed
  2/2 before the clean 5/5 run above.
- Final regression gates after refreshed Rust bindings: 119 portable mobile tests pass;
  native view/model lanes execute 46 tests with two intentional availability skips and
  no failures on each iOS 26.5/27 simulator
  (`views-1789706958770505000.xcresult` and
  `views-1789706974629780000.xcresult`); 136 MomentumKit tests pass; and the macOS Debug
  app builds. The AppIntentsTesting target still skips only Apple's Customer OS error
  803 and is not counted as acceptance.
- F-031 remains **In progress**. The default Complete tile proves `completed=true`,
  while the `completed=false` reopen parameter is covered at the portable boundary but
  not yet by a constructed system shortcut. Simulator tooling cannot establish
  authenticated locked-device behavior, and the user explicitly deferred further
  physical-device testing. Mac cold Shortcuts invocation remains unverified. No commit,
  push, phone installation or release was performed.

### 2026-09-17 — iOS system Spotlight result acceptance (F-024)

- A live system-UI probe found that the named `MomentumTasks` index journaled data but
  did not surface a Momentum result in SpringBoard Spotlight. The shipping client now
  uses `CSSearchableIndex.default()` and keeps deletion/reconciliation scoped to
  Momentum's existing domain identifier. The normal two-second mutation debounce,
  delta updates, foreground cancellation and reindex delegate remain unchanged.
- Added `MomentumSpotlightSystemUITests`, a Debug-only acceptance lane that uses the
  disposable UI-test task store while enabling only the real Core Spotlight adapter.
  Its zero-delay test configuration avoids timing the production debounce; Release
  builds ignore the launch argument. The test creates a unique task through the
  production UI, backgrounds Momentum, opens system Spotlight from SpringBoard,
  searches for and taps the result in the **Momentum** section, then verifies Search
  is selected with the exact query and current task. It deletes the disposable task
  through the production UI so the normal index deletion delta runs before exit.
- Terminal passes: **1/1** on iOS 26.5 in
  `/tmp/momentum-spotlight-system26/Logs/Test/Test-MomentumSpotlightSystemUITests-2026.09.17_22-29-59--0500.xcresult`
  and **1/1** on iOS 27 in
  `/tmp/momentum-spotlight-system27/Logs/Test/Test-MomentumSpotlightSystemUITests-2026.09.17_22-29-00--0500.xcresult`.
  The earlier native index tests still cover exact add/update/delete and recovery;
  isolated model/UI tests cover cold and warm route semantics. No physical device or
  user task store was used. F-024 is Verified for iOS; the macOS Spotlight deferral and
  F-031's remaining App Intent actions stay separate.
- Generated-core preparation passes. The terminal fast lane passes **119 tests / 20
  suites** in 3.875s. Native view/integration coverage passes **45 tests** with the two
  intentional opt-in skips on each runtime in
  `ios/DerivedData/TestReports/views-1789702570387552000.xcresult` (iOS 26.5) and
  `ios/DerivedData/TestReports/views-1789702586434828000.xcresult` (iOS 27). The generic
  Momentum iOS Simulator build succeeds in `/tmp/momentum-ios-spotlight-final`.
- That build exposed the existing iOS 26 deprecation on styled task-subtitle `Text`
  concatenation. The row now uses SwiftUI's styled-text interpolation; the final build
  is quiet for that path and both native suites above pass with the rendered change.
  `git diff --check` is clean. No commit, push, device install or release was performed.

### 2026-09-17 — real iOS Home Screen badge acceptance (F-032)

- Added a Debug-only system-badge acceptance mode: task data/preferences remain in the
  disposable UI-test store, while notifications use the production
  `SystemNotificationCenter`. Sync and Spotlight remain disabled for the isolated store;
  Release builds ignore the launch argument.
- Test-first launch-mode coverage distinguishes ordinary UI tests, demo, production and
  the badge lane. The native test creates one Today task, grants notification access
  through the real iOS prompt, refreshes through the production coordinator, reads
  SpringBoard's actual `1 new item` icon value, completes the task and verifies the
  badge disappears.
- Fresh-install simulator runs pass **1/1** on iOS 26.5 and **1/1** on iOS 27 in
  `/tmp/momentum-badge-system-26.log` and `/tmp/momentum-badge-system-27.log`.
  The first 26.5 attempt already proved the real badge but exposed duplicate SpringBoard
  icon nodes; the regression now scopes readback to Home Screen icons. A second harness
  failure found tab selection could settle on Upcoming after returning from SpringBoard;
  the final test uses the app's verified Command-1 navigation and waits for Today.
- Existing shared/core tests cover None, due/scheduled Today, Today including overdue,
  zero hiding, family/subtask counting, permission denial, coalescing and failures.
  Physical-device delivery and system-initiated background App Intent execution remain
  outside this simulator-only result.

### 2026-09-17 — initial iOS Spotlight indexing and activation routing (F-024)

- **Implementation:** unfinished tasks project to a platform-neutral search document,
  then the initial named Core Spotlight index. The first snapshot reconciles the
  app domain; later snapshots delete only removed identifiers and index only new or
  changed tasks. A two-second cancellable debounce coalesces mutation bursts, work is
  cancelled outside the foreground, and Core Spotlight recovery callbacks can request
  all or selected current items without rewriting unchanged data on every activation.
- **Activation:** the app accepts only `CSSearchableItemActionType`, resolves the task's
  current title through the shared Rust engine, selects Search and focuses that query.
  Stale identifiers are ignored and an already-presented add draft remains intact.
- **Evidence:** test-first projection, delta, recovery, activity-decoding and route
  coverage passed. The complete fast lane passed **119 tests / 20 suites**. Native
  view/integration lanes passed **42 tests** with two intentional opt-in skips on each
  iOS 26.5 and 27 simulator. Real custom-index add/update/delete passed in
  `/tmp/momentum-spotlight-native-26.log` and `/tmp/momentum-spotlight-native-27.log`.
  Isolated production-UI cold/warm routing passed **2/2** in
  `/tmp/momentum-spotlight-activation-26.log` and
  `/tmp/momentum-spotlight-activation-27.log`.
- **Limit at this stage:** XCUITest had not yet delivered a tap from the system Spotlight
  UI into the app. The later system-result handoff above identified the named-index
  visibility problem, moved production indexing to the default system index and closed
  the iOS simulator gap. The macOS Spotlight deferral is unchanged.

### 2026-09-17 — iOS 1,000-task search/edit performance evidence (F-001, F-039, B-055)

- Added opt-in production-view search and representative short-title edit samples to
  the existing isolated 1,000-task QA fixture. Search resets through the native Clear
  Text action; edit asserts the resulting row and floating Add task button remain
  hittable and inside the app frame.
- The iOS 26.5/27 search matrix passed in
  `/tmp/momentum-ui-search-edit-matrix.xcresult`: means were **1.430 s / 0.155 s CPU /
  90.491 MB peak** and **1.571 s / 0.257 s CPU / 103.959 MB peak**, respectively.
  Wall time includes XCTest and the deliberate 120 ms debounce.
- The representative edit matrix passed in
  `/tmp/momentum-ui-edit-representative-matrix.xcresult`: iOS 26.5 mean **6.489 s /
  2.462 s CPU / 98.683 MB peak**; iOS 27 mean **6.043 s / 2.146 s CPU / 105.969 MB
  peak**. Xcode stalled collecting diagnostics after the completed iOS 27 test; a
  focused rerun passed the frame assertions and retained a correct screenshot in
  `/tmp/momentum-ui-edit-layout-red27.xcresult`. The clipped diagnostic attachment was
  not reproduced as an app layout defect. Existing B-012 invalid-frame warnings remain.
- Source audit confirms store/query work stays behind EngineWorker, Nextcloud I/O uses
  a utility detached task, and search/sync/notification/badge/Spotlight work is bounded,
  cancellable or coalesced. iOS has no desktop CLI socket/polling loop. Simulator
  measurements do not establish physical-device energy behavior; device testing is
  explicitly deferred by the user.

### 2026-09-17 — Linux desktop parity native acceptance (F-022, F-027, F-028, F-031, F-032, F-037, F-038)

- **Scope and isolation:** tested dirty worktree `task/linux-desktop-parity` at
  `/var/home/danhart/.config/superpowers/worktrees/momentum/linux-desktop-parity` on
  GNOME Wayland. The one persistent test profile is
  `/var/tmp/momentum-linux-parity.TlekM6`, with task data in `data`, keyfile settings
  in `config`, and user-data integration files in `xdg-data`. It contains synthetic
  tasks only and was preserved for relaunch evidence. No normal Momentum settings or
  task files were changed.
- **Build:** `flatpak run org.flatpak.Builder --user --install --force-clean flatpak_app
  build-aux/io.github.dan_hart.Momentum.Devel.json` exited 0. Its packaged Meson run
  passed 4/4 targets, including the workspace cargo-test target. The focused compact
  typography test passed 1/1. The focused private-D-Bus open suite passed 3 tests with
  its activation helper intentionally ignored (1).
- **Final automated verification (dirty worktree based on `7ec82e93271f`):** focused
  checks passed 8 shared count tests, 6 selected `mo open` tests, 18 typography tests,
  and 7 background-status tests. `./build-aux/test.sh` passed 264 tests with one
  intentional private-D-Bus helper ignored; portable all-features/all-targets testing
  excluding GTK passed 186 with the same one ignore. The Rust UniFFI boundary built.
  The two established Rust warnings remained (`MockDav::gets` unused and one unnecessary
  `mut`); headless GTK also emitted IBus-unavailable, null test D-Bus connection, and
  Adwaita width-pressure warnings without test failures.
- **Final static/localization evidence:** package formatting, strict temporary-schema
  compilation, POTFILES completeness, `msgfmt --check`, LINGUAS/catalog parity,
  whitespace checks and changed-document relative links/fragments passed. The rebuilt
  Devel Flatpak passed all 4 Meson targets, including the complete cargo test target; a
  fresh isolated launch then exposed 243 objects through AT-SPI with no unnamed controls
  on the main surface. The runtime lacks a supported German locale and warned that it
  fell back to C, so this post-catalog audit does not replace the earlier German layout
  limits. `Cargo.lock` remained unchanged. The host had no `lychee`, so the exact
  read-only [Python fallback command](TESTING.md#run-it) was used; it checked 33
  relative links across 9 changed Markdown files with zero missing targets or fragments.
  `msgfmt` reported 125 translated, 96 fuzzy, and 278 untranslated German messages while
  validating the catalog successfully.
- **Apple boundary limit:** `macos/scripts/build-core.sh --debug` could not create the
  XCFramework because `xcrun` and `xcodebuild` are absent; `swift test`, the accessibility
  self-test, and the localization checker could not start because `swift` is absent.
  These are unavailable checks, not inferred passes; the fallback
  `cargo build -p momentum-ffi --features momentum-core/ffi` succeeded.
- **Isolation correction:** the first custom-store launch joined the already-running
  stable app through the intentional shared LibreSync identity and copied its snapshot
  into the otherwise isolated store. No mutation command had run. Devel alone was
  stopped; only the copied files under the test directory were removed; the stable
  snapshot hash was verified unchanged. Every subsequent launch used
  `--unshare=network`. `XDG_CONFIG_HOME` and `XDG_DATA_HOME` were exported by a shell
  inside the sandbox because Flatpak replaces those two `--env` values before exec.
- **F-022:** live AT-SPI extents demonstrated independent scaling: at content/UI
  100/100 the task-title and View Options heights were 24/34 px; 250/100 produced
  58/34; 100/250 produced 24/58; and 250/250 produced 58/58. A deliberately missing
  font still exposed readable named text. Scale/font choices survived relaunch. The
  250% German pass exposed 270 named main-window objects with zero unnamed controls;
  Quick Add, New Project, and Nearby Devices also reported zero. The task editor had
  two unnamed list items, Preferences had three, and libadwaita `AdwSpinRow` scale/time
  controls did not appear on the live AT-SPI bus despite widget-level accessible
  properties. Directed keyboard input and screenshots were blocked by Wayland focus
  policy and GNOME's `ScreenshotWindow` authorization. The display/compositor expanded
  the requested 360×720 window and libadwaita reported width-pressure warnings, so the
  exact compact native layout was not observed. New parity strings were still English
  during this pre-localization build. Linux F-022 therefore remains Implemented.
- **F-031:** bundled `mo` created, found, completed, reopened, and planned synthetic
  tasks with the documented JSON shapes; live sync while Off returned the documented
  machine-readable error. GNOME SearchProvider returned the exact task plus create
  result and activated the native editor. Exact GAction and URI opening both exposed
  `Plain today` in the task editor, with identical `state.json` and `pending.json`
  hashes before/after. Missing, ambiguous, deleted/stale, and archived action requests
  were also non-mutating. Private-bus tests supplied safe stable/Devel warm, cold,
  coexistence, and real nonreply-timeout coverage. The injected activation runner
  covered rejection, disappearance, and malformed acknowledgements. Live `mo open`
  was not aimed at a custom or personal registered store. Keep Linux F-031 Implemented
  rather than treating the private bus as a full user-visible cold-launch observation.
- **F-032:** live portal calls published `9 tasks due today`, then `10 open tasks in
  Today` after including overdue work, then the neutral `Momentum is running` message.
  The choice retained `today-including-overdue` while Run in Background was disabled
  and survived relaunch. GNOME's visual Background Apps surface was not automatable in
  this session, so F-032 remains Implemented.
- **F-027 / F-028:** Off exposed neither provider group; Nextcloud exposed only its
  connection controls; LibreSync exposed only Manage/Sync Devices. Switching back
  restored the synthetic URL, user, and folder, and `cli-config.json` matched the one
  selected provider. A Nextcloud failure produced persistent Details/Retry and the
  native `Nextcloud sync is not configured` dialog. LibreSync CLI handoff returned
  requested, then Details reported `No linked devices`, with no Nextcloud fallback.
  A real successful exchange, progress-state timing, and linked-device transfer were
  unavailable in this networkless profile; retain Implemented.
- **F-037:** each native stateful action read back its saved value. Morning & Night
  rendered Today/Morning/Evening and Completed; Project rendered Inbox and Completed
  with no day-period headings; Tag used only each task's first tag and an Untagged
  bucket; Time Estimate rendered Up to 15 min, 16–30 min, 31–60 min, 1–2 hours,
  Over 2 hours, No estimate, then Completed; None left only the Completed division.
  Overdue row metadata remained visible and the saved `none` choice survived relaunch.
  Pointer drag, directed keyboard reorder/Undo, cross-group drag rejection, and visual
  high-contrast/color fallback were not available, so F-037 remains Implemented.
- **F-038:** default Off/08:00 was read before changes. Enabling and changing the time
  updated keyfile settings immediately; enabled 09:45 survived app/Preferences relaunch.
  At an eligible synthetic time the live app made one accepted
  `org.freedesktop.portal.Notification.AddNotification` call with ID `morning`, title
  `Good morning`, body `9 tasks today, 1 this morning, 1 tonight`, normal priority,
  and default action `app.today`; no second call appeared on the next timer interval.
  The live SpinRows were absent from AT-SPI, denial/failure delivery was not available,
  and new German strings awaited catalog regeneration, so F-038 remains Implemented.
- No product defect was changed during acceptance and no BUGS entry was added. Native
  observations that could not meet the full gate remain explicit rather than being
  promoted to Verified.

### 2026-09-16 — Morning & Night default and exclusive grouping (F-037, B-003)

- **Request:** Morning & Night becomes the default; Today/Morning/Evening derive from
  tags only in that mode. Project/Tag/Time Estimate ignore those divisions. This
  supersedes the earlier nested-grouping design and None default below.
- **Behavior:** one flat set of open tasks is sorted, then grouped once. Default
  sections are Today (no time-of-day tag), Morning, Evening, in that order; matching
  is case-insensitive and Morning wins when both tags exist. None is a flat list.
  Completed stays one separate section. Overdue tasks remain visible and Coming Up
  retains its date window, with due dates shown on rows instead of date sections.
  Search retains result-type separation. Explicit saved grouping choices are preserved.
- **Implementation:** shared Rust rules and smaller base-list construction; native
  GTK/SwiftUI menu/default mappings; regenerated UniFFI; English/German strings;
  tests updated to the new contract rather than the superseded section structure.
  Mobile requirements and MVP.md now reference the same exclusive-grouping contract.
- **B-003:** dual-tag drop/toggle previously treated raw tag presence as the active
  day period. Resolve the actual period and remove opposing matching tag IDs; regression
  covers mixed case, both tags, parent/subtask placement, moving groups and undo.
- **Evidence (dirty worktree based on 0c9c0bf):** default portable Rust and all-features/
  all-targets suites each pass 164 tests (72 shared-core); Swift passes 123 tests in
  29 suites, with the existing opt-in nearby-network test skipped. arm64 Debug Xcode
  build succeeds. GTK application/tests compile. Strict GSettings schema compile and
  isolated memory-backend readback confirm `morning-night` as the default.
- **Localization/quality:** compiler extraction reports complete German catalogs
  (286 app / 159 package strings); built resource checks include Morning & Night and
  Evening. Seven localization-tool tests, formatting, whitespace and changed-doc links
  pass. No new dependencies, data migration, commit or push.
- **Native evidence:** rebuilt isolated Mac preview shows only colored Home/Momentum
  project sections, combining Morning/Evening tasks and retaining overdue row dates.
  Further menu automation repeatedly reported “user changed the app”; preview left
  available without fighting concurrent interaction. Default-mode native selection,
  revised grouped gestures, spoken VoiceOver and native Linux acceptance remain open;
  core and Swift tests do cover default switching, None, family placement and undo.
- **Next:** finish native menu/keyboard/grouped-move acceptance when the UI is available;
  run the Linux Blueprint/GTK suite in a supported environment. Prior real-device
  LibreSync and Spotlight testing remains deferred by the user.


### 2026-09-16 — Group By implemented (F-037)

- **Workspace:** new local edits on the staged, uncommitted desktop baseline at
  `0c9c0bf` in `pull-latest-code-70308a`. No commit, push, scanner exception, dependency,
  data migration or mobile scaffold was added.
- **Shared core:** None/Project/Tag/Time Estimate; first valid stored tag only;
  approved six contiguous estimate intervals; alphabetical named groups; preserved
  date/completion sections, family placement, search caps and archive paging. Manual
  reordering and keyboard nudges stay inside a group and undo restores full order.
  Group metadata includes saved project/tag colors. See [GROUPING.md](GROUPING.md).
- **Linux:** native settings-backed radio submenu, persistent `group-by` preference,
  localized headings and colored project/#tag names with safe markup escaping,
  wrapping and neutral high-contrast/color-preference fallback. GTK tests compile;
  schema compilation and supported-choice readback pass. Native Blueprint/GTK runtime
  checks still require a Linux desktop/SDK (unavailable on this host).
- **macOS:** native Picker in View Options and View menu, local persistence, localized
  list sections and saved project/#tag colors. Existing color accessibility policy and
  typography apply; date context stays neutral and overdue context stays red.
- **Automated evidence:** five new core grouping regressions cover exact boundaries,
  first-tag uniqueness, subtasks, per-group sorting, metadata colors, grouping without
  writes, None restoration, reorder/nudge/undo, search truncation and archive paging.
  Portable Rust suites pass in default and all-features/all-targets configurations
  (159 tests each). Final Swift suite reports 123 tests in 29 suites passing, with the
  existing opt-in nearby-network test skipped. Final arm64 development Xcode build
  succeeds. GTK application and test binary compile; runtime tests were not run.
- **Native Mac evidence:** isolated `MOMENTUM_DEMO=1` preview with sync disabled;
  native menu selection/readback for Project, Tag and Time Estimate; screenshots
  confirm colored project and #tag headings and separate overdue context. Grouped
  rows retain the demo tasks and expected ranges. The feature has not separately
  repeated real drag gestures, spoken VoiceOver, or every font/contrast/window size;
  core reorder regressions and existing accessibility policy tests pass.
- **Localization/checks:** compiler-extracted catalogs are complete (285 app and 158
  package German entries); compiled German grouping strings and existing interpolation/
  plural/intent resources pass. Seven localization-tool tests, Rust formatting and
  whitespace checks pass. German wording has not had independent language review.
- **Mobile:** explicit iOS/Android native control, shared-core behavior, persistence,
  accessibility, reordering and test requirements recorded in GROUPING.md. Both Planned.
- **Next:** native Linux acceptance (including menu/schema resources, grouped row
  ordering, selection and color fallbacks); targeted grouped native drag/accessibility
  checks where available. Real-device LibreSync and Spotlight remain user-deferred.


### 2026-09-16 — Integration into main authorized

- User explicitly requested committing and pushing to `main`; this authorizes this
  integration, without changing the local-only default for future work.
- Remote main was fetched and matched the development base; integration can fast-forward.
- The full native Mac/shared-core work, Linux sync parity, AGENTS.md and platform ledgers
  were staged. Generated binaries, personal data/recovery files and signing identities
  remain excluded. Integration and replacement of the routing guide did not occur.
- Commit stopped at a scanner false positive; the proposed allowlist exception was
  rejected by automatic approval review and is awaiting explicit user approval. No
  exception was written and no commit/push occurred.
- Formatting, whitespace and seven localization-tool tests passed. Fresh regression
  checks passed: 24 CLI tests; Swift reported 120 tests in 28 suites with its opt-in
  network test skipped. Existing Linux native UI and deferred physical LibreSync/
  Spotlight limits remain unchanged; pushing is not verification.

### 2026-09-16 — Project guidance and parity ledger established

- Created the full AGENTS.md and three ledgers from the user's explicit decisions;
  added an outer-checkout routing guide to preserve the current development location.
- Seeded platform status conservatively from current source and recorded acceptance.
  No blanket “Verified” migration from the older “MVP implemented” description.
- Found a current source-backed gap: Linux GSettings/preferences bind independent
  `sync-enabled` and `p2p-enabled` switches, while macOS selects one provider. The user
  requested Linux parity; work is active as F-027/F-028.
- Documentation links, ledger IDs/status cells and whitespace were checked. No commit/push.

### 2026-09-16 — Linux sync parity implemented (F-027 / F-028)

- Replaced the independent Nextcloud/nearby switches with one native libadwaita ComboRow.
  Only the selected provider's settings are shown. Legacy dual-enabled preferences resolve
  to LibreSync as on macOS; no tasks, secrets or pairings are migrated/deleted.
- Manual/automatic/CLI sync respects the selected provider. Switching cancels a queued
  Nextcloud debounce, stops unselected LibreSync, and waits for an in-flight Nextcloud
  cycle to finish before starting LibreSync. Preview mode never starts a sync transport.
- Added a persistent content footer for Off/provider/progress/last-success/setup state,
  including empty lists. Failures retain Details and Retry. LibreSync lifecycle events
  now update native progress/error state rather than merely repainting a caption.
- Export/import non-secret connection settings for the desktop CLI; selected method is
  owned by native preferences. Linux keyfile resolution honors Off/LibreSync even with
  cached Nextcloud credentials and rejects unknown methods rather than falling back.
- Updated English/German UI strings, schema, user-facing sync docs and feature statuses.
- **Validation:** native GSettings provider-policy test passed using an isolated memory
  backend; two CLI policy tests and all 22 CLI integration tests passed. The first CLI
  attempt could not create sandboxed Unix sockets; the permitted isolated rerun passed.
  `cargo check -p momentum`, `cargo test -p momentum --no-run`, Rust formatting, strict
  GSettings schema compilation, POTFILES completeness, German PO compilation and whitespace
  checks passed. Existing dependency warning: `block v0.1.6` future incompatibility.
- **Limits:** this Mac has no Blueprint compiler, Broadway display, or active Linux
  container. The new GTK settings/empty-state/error regression compiles but has not run;
  Rust compilation does not compile Blueprint resources or verify native Linux UI.
  Linux-only keyfile loading integration is source-reviewed; its parsing policy is tested
  on this host. No real-service/physical LibreSync test, remote CI, release, commit or push.
- **Source/evidence:** dirty worktree based on `0c9c0bf`; tests live in
  `crates/app/src/prefs.rs`, `crates/app/src/tests/ui.rs`, and `crates/mo/src/main.rs`.
  Local diagnostic logs: `/tmp/momentum-linux-check-final.log`,
  `/tmp/momentum-linux-sync-policy.log`, `/tmp/momentum-linux-cli-verified.log`.
  These ephemeral logs are supporting evidence, not portable project dependencies.

### 2026-09-16 — Prior desktop acceptance evidence (carried forward)

This is historical evidence from [MACOS-MVP-CHECKLIST.md](MACOS-MVP-CHECKLIST.md),
not a claim that those commands were rerun while writing this guide. Source state:
dirty worktree based on `0c9c0bf`; affected changes must be revalidated.

- Portable Rust: 152 tests per default/all-features variant; GTK compilation and test
  compilation; formatting and gettext/catalog checks passed in the recorded sweep.
- Swift: 120 tests reported across 28 suites, including one skipped opt-in network test.
  Signed Mac Debug build and compiled German resource checks passed.
- User confirmed native single/multi-project drag, reorder/Undo and external text/URL
  drops. Native Mac keyboard, fonts, empty-state and contrast checks are documented.
- AX audits reported zero unnamed controls (22 main-window, 36 task-editor controls).
  Spoken VoiceOver is not covered by those audits.
- Normal Mac app was reopened on the user's store with Today selected, system accent
  background and contrasting star. Nextcloud success was observed. Real-device
  LibreSync and Spotlight checks were explicitly deferred, not passed.

## Handoff format

Append a dated entry with feature/bug IDs, platforms changed, observable behavior,
source revision or dirty state, actual commands/results and native/user evidence,
remaining gaps and next steps. Refresh current priorities and ledger cells in the same
change. Never include private tasks, credentials or machine-specific server addresses.

## 2026-09-16 — F-038 configurable morning summary

- **Scope/state:** dirty development worktree based on `0c9c0bf`; inherited staged
  changes preserved. Linux/macOS Implemented, iOS/Android Planned. No commit/push.
- Replaced the hard-coded automatic 05:00 rule with shared-core opt-in and local time.
  Off by default, initially 08:00, independent task reminders. Native settings retain
  the time when disabled and update the running engine. English/German copy explains
  local time, next-launch catch-up and the running-app requirement.
- Regenerated UniFFI through `macos/scripts/build-core.sh --debug`; native GTK and
  SwiftUI controls map to the same Rust preference record. Mac notification adapter
  must be attached before consuming reminders/summary eligibility.
- **Automated:** 166 Rust tests passed in each default and all-features/all-targets
  workspace configuration (excluding GTK runtime); 74 shared-core tests include default
  off, before/at selected minute, late launch, persisted once-per-day claim, next-day
  eligibility, disable/re-enable/time edits, empty tasks and independent task reminders.
  126 Swift tests passed in 29 suites; existing opt-in nearby integration remains skipped.
  New Swift tests cover preference persistence/validation and notification adapter gating.
- **Build/settings/localization:** signed arm64 macOS Debug build passed; GTK test binary
  compiled on this Mac; strict GSettings schema compilation and two memory-backend
  preference tests passed. Linux native regression for switch/time bindings and immediate
  engine updates added and compiled, not executed. German coverage: app 290/290, kit
  159/159, permission strings 2/2 and shortcuts 5/5; runtime bundle check and seven
  localization-tool tests passed.
- **Native Mac readback:** opened isolated demo Settings → General. Accessibility tree
  exposed Notifications, Morning summary Off, disabled system time picker at 08:00,
  and the explanatory copy. Subsequent screenshot targeted a changed main window rather
  than Settings, so visual layout/time-edit interaction and actual OS summary delivery
  are not marked verified. No test enabled morning notifications in the user's settings.
- **Limits/next:** run Blueprint/resource compilation and GTK UI regression on Linux;
  verify native time editing, restart persistence and system delivery using an isolated
  notification profile, including denial, larger text and German layout. Mobile app
  foundations remain absent. LibreSync and Spotlight real-device deferrals unchanged.
- Local logs: `/tmp/momentum-morning-{core,ffi,swift,build,linux,schema,rust,rust-all}.log`.

## 2026-09-16 — F-019 macOS centered task checkboxes

- User explicitly scoped this refinement to macOS. Changed `TaskRowView`'s outer
  HStack from first-text-baseline to center alignment. Checkboxes, archived checkmarks
  and trailing badges center against the entire title/metadata stack, including when
  text wraps; subtask indentation and native checkbox behavior are unchanged.
- Verified scope: signed arm64 Debug build passed (`/tmp/momentum-checkbox-build.log`);
  launched the updated isolated demo and visually inspected Today, Morning and Evening
  rows with title plus metadata. Checkboxes and reminder/repeat icons are centered.
  Larger text and wrapped-title variants were not separately exercised this pass.
- No logic change or new unit tests; no Linux/mobile changes requested. Preview is
  open with the rebuilt app. Existing staged work preserved; no commit or push.

## 2026-09-16 — Local Release installation

- User requested a release installation on this Mac. Built current dirty-worktree
  source as optimized arm64 Xcode Release, Momentum 0.4.0 (build 1), with local Apple
  Development signing. Installed at `/Applications/Momentum.app`; application data and
  preferences retained in their existing locations. Updated the existing development
  `~/.local/bin/mo` symlink to the installed app's bundled companion.
- Release initially failed loading Rust proc-macro helpers. A minimal `extern crate`
  reproduction exposed `mis-aligned LINKEDIT string pool` on macOS 27, matching
  [Rust issue 157750](https://github.com/rust-lang/rust/issues/157750). Local build override
  `CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_STRIP=none` avoids stripping compiler helpers;
  app/CLI release optimization and LTO remain enabled. Use that environment variable
  for release builds on this toolchain. No project-wide compiler setting was changed.
- Verification: Xcode Release build succeeded, installed app and bundled `mo` passed
  strict/deep code-signature validation, installed executable hashes match the built
  release, German bundle checks passed, and the installed CLI responds to `--version`.
  Launched the installed app successfully with Today selected and existing task data.
  This is a local installation, not a notarized distribution or published release.
- Logs: `/tmp/momentum-release-install.log` and `/tmp/momentum-release-core-fixed.log`.
  Previous feature verification/deferred acceptance remains as recorded. No Git commit,
  push, tag or publication occurred.

## 2026-09-16 — Desktop integration approved after release installation

- User requested commit/push and explicitly approved the enum-name scanner exception.
  The exception is limited to `FileCreateFlags::PRIVATE`; the pre-commit secret hook
  remains enabled. Consolidation includes all 127 reviewed pending files plus the
  approved `.gitallowed` update; installed/generated app bundles remain excluded.
- Remote main matched base `0c9c0bf` at preparation. Integration uses a normal
  fast-forward push from `task/desktop-mvp`; no force push or history rewrite.
- Formatting, staged whitespace checks, localization-tool tests and the full cached
  secret scan are integration gates. Recent Rust/Swift, GTK compilation, GSettings,
  localization, Debug/Release builds and native acceptance are recorded above.
  This integration does not promote any deferred or unverified platform checks.

## 2026-09-16 — iOS implementation authorized

- User approved the design with four tabs: Today, Upcoming, Search, Settings.
  Projects/tags, Morning/Evening and Archive remain accessible within Today via Lists.
- Work is isolated in `.claude/worktrees/ios-app`, branch `task/ios-app`, base `7ec82e9`.
  No commits/pushes. Rust iOS device and simulator targets installed; pinned LibreSync
  v0.6.0 unpacked into the ignored dependency directory without modifying its source repo.
- Existing Rust workspace suite passed with local socket permission; initial sandbox
  CLI failures were permission errors. iOS Rust compilation is in progress.
- [Implementation plan](superpowers/plans/2026-09-16-ios-parity.md) tracks delivery
  and reviews. All iOS features remain unverified until code/build/native evidence.

## 2026-09-16 — iOS foundation and offline editing in progress

- Dirty `task/ios-app` worktree at base `7ec82e9`; local only. Real Rust core with
  LibreSync builds for arm64 iOS device and arm64/x86_64 simulator. One locked Apple
  build producer preserves Mac and mobile slices. Independent source review passed;
  Release and full iOS → Mac → iOS sequencing remain outstanding.
- MomentumKit portable support compiles for device/simulator; desktop boundaries
  retain AppKit on macOS. Existing 126 Mac tests passed, plus public AutomationTask
  construction tests. Evidence: `/tmp/momentum-portable-kit-mac-tests.log` and
  `/private/tmp/momentum-portable-kit-ios-build.log` (automated).
- Native four-tab target builds with Swift 6 using the real shared Engine. Eight
  isolated mobile tests passed for create/relaunch, completion/undo, archive guards,
  grouping/organization, full form mapping, recurrence and task-family actions:
  `/tmp/momentum-ios-mobile-all.log`. UI code exists for lists, selection, editors,
  recurrence, context management, native drag/drop and initial preferences.
- First iOS 27 simulator UI run passed create/complete/undo/settings/relaunch checks
  but failed the tab-order assertion: the special Search role moved Search last.
  Changed it to a regular tab to preserve Today, Upcoming, Search, Settings. Integrated
  unsigned simulator build passed (`/tmp/momentum-ios-integrated-build.log`);
  native retest is pending. No full parity or accessibility claim is made.
- Native scheduling-clear regression is under test; sync, notifications, automation,
  full localization, performance and both-version acceptance remain outstanding.

- User refinement: iOS global accent **#FF6600**, defined once in the native sRGB
  AccentColor asset and applied at the scene root; simulator readback pending.

## 2026-09-16 — iOS native editing and appearance refinements

- Four iOS 27 native tests passed in `/tmp/momentum-ios-editor-green.xcresult`:
  exact Today/Upcoming/Search/Settings order, create/complete/undo/relaunch;
  scheduling removal stays unscheduled after reopening; quick-add expansion parses
  the existing grammar; Unicode replacement remains usable. Tests used a disposable
  store, dirty `task/ios-app` at `7ec82e9`. Further editor draft-action fixes are under
  review; this is scoped evidence, not full parity.
- User approved SF Symbols throughout navigation/settings/actions/editor sections,
  exact default #FF6600, a DHFlatUIColors-only custom accent picker, an edge-to-edge
  icon, and readable light/dark/high-contrast rendering. Pinned the requested package
  at `821b077fc94ba45422ded8f38ee5b532dbabfd3e`; no transitive dependencies. Three
  palette tests pass across every source color, normal 4.5:1/increased 7:1 text targets,
  default fallback and persistence. Native appearance/audit still pending.
- Icon replaced with full-bleed artwork preserving the existing mark; packaging
  inspection confirms 1024×1024, no alpha, fully opaque corner/edge pixels. Its
  generated background has slight color variation; exact #FF6600 is maintained in
  the accent model, not asserted for every bitmap pixel.
- Appearance value tests passed for independent relative sizes and corrupt defaults;
  lifecycle tests passed for 23/25-hour DST days and one engine owner under concurrent
  bootstrap. Native font picker, completion haptics, reduced symbol motion and a
  cancellable midnight wakeup are implemented but await expanded native acceptance.
- iOS 26.5 demo launched and its first layout was inspected at iPhone 17 Pro size
  (`/tmp/momentum-ios26-before-polish.png`); later appearance changes require fresh
  captures. iOS 26 acceptance is not yet equivalent to the iOS 27 workflow evidence.

- Focused organization boundary tests now cover disappearance of deleted project/tag
  destinations, last-task Morning/Evening movement, and clearing custom colors across
  reopening. Five tests passed in `/tmp/momentum-ios-context-periods.log`; independent
  source review passed. Lists uses core sidebar availability and active screens
  recover from a removed destination. Native navigation recovery remains to verify.
- Native accent audit caught a contrast-nearly-passed finding on iOS 27. Palette
  math passes, but native contrast is not yet accepted; diagnosis is in progress.

## 2026-09-16 — bounded notification planner and native contrast investigation

- Pure shared Rust notification planning now covers seven local dates, the 60-request
  budget, reminder priority, four optional summary snapshots, date-only recurrence,
  stable identities, consumed-occurrence suppression and overflow counts. It shares
  repeat task construction with the existing engine and preserves ordered day queries
  while replacing quadratic membership scans. No early tasks or delivery claims.
- The revised planner passed independent spec/quality review. Agent-run affected
  regression evidence: 87 core, 31 model and 24 CLI tests plus FFI compilation;
  these counts are scoped to this dirty worktree pass, not iOS delivery evidence.
  Persistent acceptance/reconciliation and native scheduling are the next layer.
- Six iOS 27 launch/editor tests passed after B-006 save-first fixes. Palette math
  passes across the complete DHFlatUIColors catalog, but native B-005 contrast
  acceptance remains open. Explicit readable text and a hard native scroll edge
  address observed rendering issues; audits of complete visible previews and
  light/dark/increased-contrast appearances are in progress.

## 2026-09-16 — durable mobile notification scheduling boundary

- Added core-owned `notification-schedule.json`, separate from task data and desktop
  delivery claims. The generated boundary exposes desired plans, OS reconciliation,
  and acceptance only after successful scheduling. Stale revisions are rejected;
  missing future requests can be rescheduled and elapsed identities remain consumed
  across restart or clock rollback. Atomic/read failures are surfaced without claiming
  success. Revision metadata is private and must never enter logs.
- Independent source/spec review passed. Agent-run verification in the dirty
  `task/ios-app` worktree: all-features core 87 + ledger 21 + model 31 + CLI 24 tests
  passed (`/tmp/momentum-ledger-all-features.log`); no-default pass 161 tests
  (`/tmp/momentum-ledger-regression.log`); FFI compile passed
  (`/tmp/momentum-ledger-ffi.log`). Native scheduling, actions and delivery remain
  outstanding; these results do not make F-007/F-038 verified on iOS.
- Accent acceptance distinguishes calculated contrast from native rendering. The
  iOS 26.5 audit identified retained list text wholly behind navigation chrome. Audit
  evidence is retained; visible/unknown findings remain failures. Current native
  appearance matrix is still in progress, including Dynamic Type and dark mode.

- The full mobile Swift package suite passed: 20 tests in six suites, including all
  catalog contrast calculations, persistence, editing, organization and lifecycle
  boundaries (`/tmp/momentum-ios-mobile-complete-current.log`, same dirty worktree).
  Localization validation passed with 175/175 German entries. Icon resource readback
  confirms 1024×1024 with no alpha; visual inspection confirms orange reaches every
  edge. Native chrome-aware contrast test source passed independent review; this
  does not replace its pending simulator appearance results.

- Full-bleed icon also confirmed on the iOS 26.5 Home Screen, with normal OS corner
  masking (`/tmp/momentum-ios26-home-icon.png`). Native accent persistence and both
  palette previews reached Today in the current fresh build; the audit then caught
  a standard section heading with insufficient contrast. The task-section header now
  uses the same readable secondary color; revalidation remains outstanding.

- The corrected heading/type-declaration build succeeded, but its test runner exited
  inside UIKit/CFBundle initialization before running tests (not an app test failure).
  `/tmp/momentum-ios26-accent-readable-headings.log` records the bootstrap error.
  Dedicated Momentum QA simulators were created for 26.5/27.0 and the already-built
  tests are being replayed there. No existing simulator data was erased. The preceding
  failed audit collector stalled and was terminated after preserving its failure log
  and live screenshots; do not treat that incomplete result bundle as a finished run.

## 2026-09-16 — fast task flows and focused Settings

- User refinement: Momentum Default remains exactly #FF6600 in dark mode, including
  Increase Contrast, and receives only the minimal readable darkening in light mode.
  Custom palette choices retain normal/increased adaptation; neutral text remains
  independently readable. Earlier appearance results do not verify this new policy.
- F-019/F-020: Settings now leads to Task Lists, Appearance and About pages. Task
  grouping/sorting/archive options stay together; accent, label colors, text and
  feedback stay under Appearance. Native sheet actions use SF Symbols; schedule
  changes have short transitions disabled by Reduce Motion, and rows offer leading
  completion/edit swipes plus trailing delete with existing Undo.
- Quick Add now retains the draft if the core reports no saved mutation, exposes
  feedback in the sheet, and restores typing focus. Previous code dismissed even
  on failure; B-008 records the scope. Native flow and localization checks ongoing.
- Prior dark/increased iOS 26.5 v2 audit did not execute: the XCTest runner bundle
  disappeared during bootstrap (`/tmp/momentum-accent-ios26-5-dark-contrast-enabled-v2.log`).
  It was a runner failure, not a new contrast finding.

- Mobile package verification passed: 27 tests / seven suites, including all custom
  palette rendered/fill contrast combinations and exact dark default policy
  (`/tmp/momentum-mobile-polish-tests.log`). German catalog: 180/180 after compiler
  sync. Independent source review passed after fixes for save-in-flight editing and
  stale failure feedback.
- iOS 27 native build succeeded. Eight launch/editor tests passed, covering capture,
  complete/undo/relaunch, schedule clearing, Unicode replacement, rejected input and
  retry, expanded details, repeat/duplicate draft preservation, and save/swipe-delete/
  undo/relaunch. The additional Settings test failed because its center tap did not
  toggle the switch; corrected to target the visible switch and rerun pending.
  Full run is therefore not green: `/tmp/momentum-ios-flow-polish-v2.xcresult`.
- Visually inspected exported Settings index, Task Lists, Appearance and task editor
  screenshots. Native symbol-only toolbar presentation retains accessible action
  labels; Save is accent-filled with contrasting ink. Source snapshots under
  `/tmp/momentum-flow-polish-evidence` and `/tmp/momentum-editor-polish-evidence`.

- iOS 26.5 dark/normal contrast: full native suite passed **10/10** in
  `/tmp/momentum-ios26-flow-polish.xcresult` (2026-09-16, dirty worktree). Includes
  corrected Settings preference persistence, Settings/Appearance contrast audits,
  custom/default accent persistence and visible contrast, and all seven editor flows.
  This verifies the focused polish scope on that simulator, not entire iOS parity.

- Additional appearance checks passed: Settings preference/contrast + custom/default
  accent persistence/contrast on iOS 27 light/normal, iOS 26.5 dark/Increase Contrast,
  and iOS 27 dark/Increase Contrast. Bundles:
  `/tmp/momentum-accent-ios27-0-light-contrast-disabled-polish.xcresult`,
  `/tmp/momentum-accent-ios26-5-dark-contrast-enabled-polish.xcresult`,
  `/tmp/momentum-accent-ios27-0-dark-contrast-enabled-polish.xcresult`.
  Native palette audits exclude only exact palette rows wholly covered by top chrome;
  visible, partially covered, unknown and Today findings still fail.
- Visual inspection separately caught B-009: native toolbar promotion did not retain
  the intended dark filled-action foreground. Explicit symbol rendering is being
  rebuilt and inspected; functional and palette audits alone did not establish that
  button's readability. Large Dynamic Type, spoken VoiceOver and energy profiling
  remain outstanding.

## 2026-09-16 — notification coordinator foundation

- Added injected Sendable center/core boundaries, immutable OS records and a single
  coalesced scheduling drain. Pending/delivered observations reconcile with the Rust
  ledger; obsolete exact revisions require cancellation readback before replacement.
  Add precedes acknowledgment; rejected/error/cancelled-before-ack adds are removed,
  while accepted catch-up requests survive cancellation after the commit point.
- No permission prompts, native center, OS registration or app wiring in this layer.
  Status contains sanitized categories and capacity/horizon data, never task text or
  private revision metadata. Individual waiter cancellation does not kill the drain.
- Source review found a permission-revocation race during add. After correction,
  agent-run host package suite passed **48 tests / eight suites**, including real
  ledger crash/restart recovery and gated concurrency/failure regressions
  (`/tmp/momentum-ios-coordinator-revocation-all.log`, same dirty worktree; parent read
  the test output and reviewed the correction). Native scheduling/actions/background
  delivery still remain; F-007/F-038 are In progress.

- B-009 correction built successfully for iOS, including the new coordinator module.
  Capture/retry and save/delete/undo tests passed again on iOS 26.5 dark mode in
  `/tmp/momentum-sheet-ink-fix.xcresult`. Visual inspection of
  `/tmp/momentum-sheet-ink-fix-evidence/F51669F3-7DE7-4DBC-A1F4-14C1064BB249.png`
  confirms the explicit black Save checkmark on the orange fill; localized accessible
  Create/Save names remain usable by native tests. iOS 27 light recheck ongoing.

- Final B-009 iOS 27 light recheck passed both native tests in
  `/tmp/momentum-sheet-ink-ios27.xcresult`; inspected screenshot
  `/tmp/momentum-sheet-ink-ios27-evidence/216FC456-3B76-4367-A6A2-DACDCDA45B82.png`
  confirms white Save ink. Final compiler localization sync: 180/180 German entries;
  `git diff --check` passed. No commits or remote writes. The focused task/Settings
  polish is implemented and verified in the recorded simulator scopes; full parity,
  physical-device/accessibility/performance acceptance remains in progress.

## 2026-09-16 — native notification integration and focused Settings

- F-007/F-038: added the live `UNUserNotificationCenter` adapter and immutable typed
  reminder/summary routes. Native adds await Apple's actual callback even after
  cancellation; errors expose fixed categories. Delegate/category registration occurs
  before launch finishes, with SF Symbol Done and Snooze 1 hour actions and body routes.
- App composition connects the coordinator after its shared engine opens. Foreground,
  successful task changes and preference edits refresh scheduling. Settings →
  Notifications separates explicit permission, opt-in Morning Summary/time, and
  horizon/overflow/retry. Preview/UI-test stores use a fake center, never request real
  permission, and preview preferences now have a separate defaults suite.
- Done/Snooze callbacks use the same engine owner as the UI, with in-process duplicate
  protection; persistent cross-process action receipts are not implemented. Reminder
  body navigation queues behind an active capture/task editor instead of discarding a
  draft. System callback/body navigation acceptance remains outstanding.
- B-010: fixed shared Snooze accepting completed tasks. Regression failed before the
  fix and passed after; the full all-feature/all-target workspace suite (excluding GTK)
  passed after rerunning outside the sandbox for isolated CLI sockets
  (`/tmp/momentum-notification-native-core.log`). Final focused regression adds actual
  deletion (`/tmp/momentum-stale-snooze-final.log`). Shared FFI build succeeded for
  Mac, iOS device and both simulator architectures through the existing generator.
- Host verification: all 51 mobile package tests passed (nine suites), including
  coordinator failure/cancellation handling and concurrent callback deduplication
  (`/tmp/momentum-notification-native-mobile.log`). The final two action tests also
  verify one-hour timing and completed-task rejection through Swift/UniFFI
  (`/tmp/momentum-notification-action-final.log`). Existing macOS package tests:
  127 passed, 30 suites (`/tmp/momentum-notification-native-macos-package.log`).
- Native iOS 26.5: two existing launch/Settings flows passed in
  `/tmp/momentum-notification-native26.xcresult`; the newly compiled notification
  Settings test required a separate run and passed in
  `/tmp/momentum-notification-settings26.xcresult`. It covers off-by-default opt-in,
  time picker visibility, persistence, no live permission prompt in isolated mode,
  and contrast audit. Screenshot `572ACA57-FEE3-4AAE-A79B-10FFA5A7BD3D.png` under
  `/tmp/momentum-notification-settings26-evidence` was visually inspected: grouped
  delivery/overview/schedule controls, readable dark appearance and default 08:00.
- Compiler localization sync: 207/207 German entries translated. iOS 27 checks and
  the final deferred-body-navigation build remain in progress. Actual permission
  grant/denial, delivery, cold/warm action interaction, suspension/background budget,
  badge updates, and device accessibility/energy acceptance remain outstanding.
  F-007/F-038 stay In progress; no broad parity rows are marked Verified.

- Final review added B-011: denied permission no longer prevents cancellation of a
  core-declared obsolete future request. The new regression failed before correction;
  all **52 mobile tests / nine suites** pass in `/tmp/momentum-notification-final-mobile.log`.
- iOS 27 initial native run: both launch/organized-Settings flows passed; the new
  notification Settings contrast audit failed specifically on the default gray
  permission status (`/tmp/momentum-notification-native27.xcresult`). Screenshot and
  issue attachment were inspected. Permission and planning-date values now explicitly
  use the app's contrast-adjusted secondary text (same B-005 policy). Full final iOS 27
  UI rerun is in progress in `/tmp/momentum-notification-final27.xcresult`; earlier
  failed audit is retained as evidence, not reported as a pass.

- Final iOS 27 simulator suite passed **11/11** in
  `/tmp/momentum-notification-final27.xcresult`: accent persistence/contrast,
  organized Settings, create/complete/undo/relaunch, notification opt-in/persistence/
  contrast, and all seven editor flows. B-005 permission-status contrast correction
  is covered. After that build, summary body routing was also queued behind drafts
  and the displayed planning date was adjusted for the core's exclusive horizon;
  these final small changes are included in the requested device build below.
- User requested installation and launch on their connected iPhone. Preparing a
  signed Debug device build using their existing individual development identity;
  no task-data reset, test fixture population, commit or release is authorized by
  this install request. Physical installation/launch result pending.

- Requested physical-device install: signed Debug build succeeded from the latest dirty
  worktree (`/tmp/momentum-dip17pm-build.log`), including the final queued summary route
  and exclusive-horizon display correction. `devicectl` confirmed successful installation
  and foreground launch on the user's connected iPhone 17 Pro Max running iOS 27.
  Existing app data was preserved; no synthetic tasks or test flags were used. This
  confirms build/install/launch only, not physical notification delivery or full parity.
  Private device/signing identifiers are intentionally omitted from project records.

## 2026-09-16 — shared badge counts and iOS badge controls (F-032)

- Moved native badge count policy into the Rust engine. Both existing Today count and
  the including-overdue query reuse model membership; families count once, completed/
  archived tasks are excluded, and timed-date precedence is preserved. macOS now uses
  this shared query instead of deriving the including-overdue count from rendered rows.
- Added an injected iOS badge coordinator: checks actual OS badge permission, serializes
  writes, refreshes after coalesced mutations/preferences, clears with zero for None,
  and returns sanitized failure status. A mode change during an in-flight write gets
  the final count from the latest mode. Preview/UI tests do not access the live badge.
- Settings → Notifications includes App Badge with the same three localized modes as
  macOS, explanatory family counting, and permission/error guidance. The OS adapter
  uses `UNNotificationSettings.badgeSetting` and `setBadgeCount`; it never prompts.
- Regression evidence: missing-API Rust and Swift tests failed before implementation.
  Full all-feature/all-target Rust workspace (excluding GTK) passed
  (`/tmp/momentum-badge-core.log`); generated Apple FFI/slices succeeded
  (`/tmp/momentum-badge-ffi.log`). Mobile package: **56 tests / 10 suites** passed
  (`/tmp/momentum-badge-mobile.log`); existing macOS package: **127 tests / 30 suites**
  passed (`/tmp/momentum-badge-macos.log`). The independent family/timing core test
  and existing macOS badge mode/completion/undo tests cover the behavior extraction.
- Native iOS badge picker/persistence/contrast tests are running. Actual Home Screen
  badge/permission-toggle and background refresh acceptance remain outstanding.
  German catalog currently validates at 211/211 entries. No new physical-device
  installation was made after the user's requested build; user data remains untouched.

- iOS 27 final badge/summary Settings checks passed **2/2** in
  `/tmp/momentum-badge-native27-coverage.xcresult`. Selection uses the actual native
  picker label and survives relaunch; changing badge mode leaves summary opt-in alone.
  The contrast audit now covers both ends of the form. Only five named retained text
  labels outside the visible content viewport may be excluded in that viewport, and
  every excluded label must be fully visible in another audited viewport or the test
  fails. Visible/unknown findings still fail. This replaces the insufficient single-
  viewport audit of text obscured by floating system bars; failed runs are retained.
  Passing screenshots are under `/tmp/momentum-badge-native27-coverage-evidence`.
- Scoped `cargo fmt -p momentum-core -- --check` and `git diff --check` pass. The broad
  formatter also reports pre-existing style differences in vendored `libresync-src`;
  those external sources were not reformatted. The core changes were formatted locally.

## 2026-09-16 — AsNeeded’s curated iOS accent selection (F-019/F-020)

- User refinement: use only AsNeeded’s nine custom colors. Read its current
  `MedicationColors.swift` and `ColorPickerComponent.swift` at revision
  `8b514a9c36345290e17d5053c26e6f505f9437df`; reproduced the same spectrum order
  using typed cases from the already-pinned DHFlatUIColors dependency.
- Removed the country/palette selector. One native list contains Alizarin, Carrot,
  Orange, Emerald, Turquoise, Peter River, Amethyst, Pomegranate, and Green Sea,
  alongside Momentum Default. Existing selected IDs remain stable when retained;
  removed selections resolve to default without rewriting preferences or task data.
- Default #FF6600 dark behavior and minimally darkened light tone remain unchanged.
  All 57 mobile tests / 10 suites pass (`/tmp/momentum-curated-mobile.log`), including
  exact curated order, persistence/fallback, and normal/increased contrast. The
  catalog regression failed against the original 280 choices before implementation
  (`/tmp/momentum-curated-red.log`). Dirty `task/ios-app`; native checks pending.
- iOS 27 light native build/test passed in `/tmp/momentum-curated-native27.xcresult`:
  removed selector, retained color selection, relaunch persistence, default reset,
  preview/Today contrast, and task capture. Inspected top/bottom picker screenshots
  in `/tmp/momentum-curated-native27-evidence`; nine named rows replace the larger
  palette flow. Previous-layout evidence is in `/tmp/momentum-curated-before-evidence`.
  German resource validation remains 211/211; no new display strings were added.
- iOS 26.5 dark native build/test also passed, including the new curated-choice
  assertions (`/tmp/momentum-curated-native26.xcresult`). Inspected saved-default
  screenshot `603CBD27-595C-4725-A82F-986372EE026A.png` under
  `/tmp/momentum-curated-native26-evidence`; native names/swatches/selection fit.
  Scoped picker behavior is verified on both simulator versions; broader F-019/F-020
  acceptance remains in progress. No phone reinstall or task-data change in this pass.
- Validation commands: `swift test --package-path ios/Packages/MomentumMobile`;
  `python3 ios/scripts/localize.py`; `git diff --check`; and, for each recorded
  simulator destination, `xcodebuild -project ios/Momentum.xcodeproj -scheme Momentum
  -configuration Debug -destination <simulator> -derivedDataPath
  /tmp/momentum-ios-appearance-current -parallel-testing-enabled NO
  -collect-test-diagnostics never -only-testing:MomentumUITests/AccentUITests
  CODE_SIGNING_ALLOWED=NO test` with Xcode 27.

## 2026-09-16 — exact #FF6600 in both iOS appearances (F-019/F-020)

- Latest user decision supersedes minimal darkening in light mode. The default
  rendering path returns its exact base in all appearances, including Increase
  Contrast. AsNeeded’s nine custom choices retain their contrast adaptation.
- Sheet Create/Save symbols derive black/white ink from the rendered accent rather
  than appearance alone; default orange uses black. EN/DE picker copy now describes
  the separate default and custom-color behavior accurately.
- Regression failed against the previous light-mode policy
  (`/tmp/momentum-exact-orange-red.log`); all 57 mobile tests / 10 suites passed
  afterward (`/tmp/momentum-exact-orange-mobile.log`). German catalog: 211/211.
- Native custom-color contrast audit remains. Default-orange native checks now
  verify selection, persistence, action rendering and capture rather than claiming
  text-contrast compliance that conflicts with the explicit fixed-color decision.
  Native build/screenshot inspection pending. Dirty `task/ios-app`; no phone reinstall.
- iOS 27 light build and focused AccentUITests passed in
  `/tmp/momentum-exact-orange-native27.xcresult`. Inspected screenshots in
  `/tmp/momentum-exact-orange-evidence`: bright default tint in Settings and
  task entry, with a black plus on the filled orange Create control. Custom-color
  native contrast/persistence still passes. Exact default in both appearances is
  covered by package tests; iOS 26 was not rerun for this light-default refinement.
  `git diff --check` and EN/DE validation pass.

## 2026-09-16 — iOS accessibility audit (F-019/F-020)

- Audited implemented iOS view families against current Apple accessibility HIG
  and accessibility-support evaluation criteria. Added a diagnostic XCTest
  inventory with per-screen screenshots, native hierarchies and all raw findings.
  No production UI or color behavior changed during the audit.
- [Full report and evidence](audits/2026-09-16-ios-accessibility.md): B-013–B-018
  track exact-orange foreground contrast, largest-text truncation, exposed task
  target geometry, feedback timing, subtask context and validation-error contrast.
  Four additional candidates remain explicitly unconfirmed pending interaction.
- Preserve the user's exact #FF6600 base accent in light/dark. Remediation should
  separate readable semantic foreground roles from brand/fill color. Black filled
  action symbols and the nine-choice AsNeeded palette remain unchanged.
- English/German, ordinary/AX5 iPhone audits and supplemental subtask/validation
  traversal completed on iOS 27. A green diagnostic test means navigation completed,
  not accessibility compliance. Platform matrix and failed attempts are recorded
  in the report; selected raw JSON and screenshots are retained beside it.
- iPad native screenshot confirms truncated Today empty-state text at AX5. The
  first iPad run used an iPhone-only tab query; a later runner terminated during
  collection. Those attempts are not counted as passed checks or app crash proof.
- F-019/F-020 stay **In progress**. Spoken VoiceOver, Voice Control, Switch
  Control, Full Keyboard Access, real notification/permission flows and remaining
  device/accessibility-setting combinations are not verified. The user's phone
  and personal store were not changed. No commit, push or deployment.
- Next: remediate the prioritized report findings, reproduce semantic/input
  candidates with assistive technology, and rerun the matrix before claiming
  accessibility support. Shared-core tests were not rerun for this audit-only
  change; app/test compilation and native traversal are the relevant checks.
- Final iOS 26.5 dark audit with Increase Contrast configured: AX5 traversal
  completed across 12 states (18 raw findings); subtask/validation across three
  states (27 findings), `/tmp/momentum-a11y-dark26.xcresult`. Screenshot review
  confirms dark mode, repeated picker truncation and actual Notifications bottom.
  Simulator Increase Contrast was restored to its original disabled setting.
- Final iPad mini/iPadOS 27 AX5 retry completed 12 states with 50 raw findings,
  `/tmp/momentum-a11y-ipad27-v3.xcresult`. Most are unresolved system Dynamic Type
  warnings, not 50 distinct app defects. Screenshot review confirms the empty
  heading truncation and oversized Save symbol; full-width Settings values fit.
- German/landscape retry completed eight states with 16 raw findings,
  `/tmp/momentum-a11y-landscape27.xcresult`. The landscape screenshot still has
  black/cropped areas after a settle delay; hierarchy reports landscape bounds.
  Root cause is unconfirmed, and landscape visual acceptance stays open.
- Final audit-only checks: eight diagnostic traversal methods completed across
  the successful bundles; none is a compliance pass. `git diff --check`, retained
  JSON parsing and audit-relative-link checks pass. Production fixes are pending.

## 2026-09-16 — README platform update and native preview screenshots

- Updated the root README to reflect the approved native/shared-core direction,
  iOS 26/27 work, four tabs, organized Settings, exact default orange and curated
  accent choices, grouping, optional single-provider sync, opt-in summaries,
  open-source commitments and undecided future distribution/billing plans.
- Platform status explicitly separates desktop baselines, iOS work in progress
  and planned Android. Linked the parity checklist and accessibility audit; no
  feature or accessibility status was promoted by documentation/screenshots.
- Captured macOS and iOS Today with built-in preview tasks and sync disabled,
  retained under `data/resources/screenshots/`. Mac capture uses a fresh Debug
  build of this dirty worktree and an isolated bundle/preferences identity; the
  visible footer confirms Preview mode. iOS uses the current simulator build
  with `--demo`, ordinary text and exact #FF6600. Personal task stores and the
  user's physical phone were not seeded or changed.
- Added screenshot provenance/reproduction notes. Earlier Linux images remain
  labeled as historical previews. No image compositing or retouching.
- Verification: fresh macOS Debug build succeeded in
  `/tmp/momentum-readme-macos-build.log`; inspected both native captures, checked
  local README links/images and `git diff --check`. Documentation-only change;
  no new production behavior and no commit/push/publication.
## 2026-09-16 — fast iOS testing foundation (F-039)

- User requested fast unit coverage and a way to test SwiftUI without ViewInspector.
  Added [the iOS test guide](../ios/TESTING.md) and an incremental runner with
  `fast`, `views`, `all`, filtering, optional coverage and per-lane wall timing.
  The normal lane has no simulator. Core preparation is fingerprinted separately;
  changed/untracked Rust inputs and regenerated bindings cannot silently reuse a
  stale preparation stamp. Empty test selections fail. No dependency was added.
- Extracted Quick Add submission state and existing editor validation into
  MomentumMobile; the actual SwiftUI sheets consume them. Task parsing/mutation
  rules remain in Rust. The existing 57 package tests grew to 67 across 12 suites,
  covering blank input, failure/draft retention/retry, overlapping submissions,
  multiline presentation, real-core rejection and editor validation.
- `MomentumFastTests` builds an empty scene-based UIKit host and compiles real app
  view sources into the unit bundle, excluding production app entry points. Eight
  in-process tests measure row/Settings-label wrapping, Dynamic Type/content scale,
  completion rendering, trait-resolved default color and actual commit symbol ink.
  The fixture retains one test-process window and cleans each child/defaults domain.
  Native UI automation remains separate for event wiring and system behavior.
- Automated evidence on dirty `task/ios-app`, based on `7ec82e9`: package tests pass
  in 0.087s of execution; final iOS 26.5 native suite passes in 0.269s (11.988s total
  command). iOS 27 native suite with coverage passes in 0.335s (16.931s command).
  Results: `ios/DerivedData/TestReports/views-1789609125874564000.xcresult` (26.5),
  `views-1789609036539384000.xcresult` in the same directory (27, coverage).
- Fault injection verified meaningful failures: accepting unchanged Quick Add
  outcomes failed three assertions (`/tmp/momentum-fast-state-fault.log`); forcing
  white button ink failed both light/dark rendered assertions
  (`/tmp/momentum-fast-view-fault-confirmed-xcode.log`). Faults were restored;
  the subsequent full package/native 26.5 run is green. The earlier UI fault attempt
  hit a missing host key window, so it was not counted as color-regression evidence.
- Optional Swift package coverage reports 490/509 executable MomentumMobile source
  lines (96.3%); generated FFI/tests excluded. This is not app-wide or Rust coverage.
  Native coverage is exported via xccov; its bundle also includes test helpers and
  unexercised app screens. Nine Python runner regressions pass. German catalog
  validation is 211/211; documentation links and `git diff --check` pass.
- Bootstrap fixture errors were corrected before acceptance: SDK 27 requires scenes,
  XCTest may not activate a key window, and rapid root replacement produced lifecycle
  warnings. The final fixture uses public UIKit containment in its own persistent
  test window, with no reflection, sleeps or production lifecycle. Automatic simulator
  diagnostics stalled an intermediate run after it passed; disabled for this fast
  lane while retaining assertions, logs and result bundles.
- Final acceptance: all four existing iOS 27 sheet UI regressions passed in
  `/tmp/momentum-fast-sheet-integration.xcresult` (70.772s execution): retry,
  Unicode replacement, More Details parsing, edit/save/swipe-delete/undo. This also
  built the actual app target after extraction. The separate fast view target reran
  green on 27 in `ios/DerivedData/TestReports/views-1789609336799507000.xcresult`
  (eight tests, 0.382s execution; 20.442s including rebuilding after core preparation).
  Warm full package commands measured 1.098s and 1.017s, with 0.082s/0.074s test
  execution. A real nonexistent Swift test filter returned failure; the subsequent
  full default run passed. Logs `/tmp/momentum-warm-fast.log`,
  `/tmp/momentum-zero-filter.log`, `/tmp/momentum-final-default.log`.
- F-039 is Verified for this testing bootstrap. No commit/push or phone installation. F-019/F-020 and B-013–B-018 are not promoted:
  component tests do not prove full-screen layouts, toolbar promotion, gestures,
  Reduce Motion, translations, spoken accessibility, notifications or full parity.

## 2026-09-16 — README support button (F-036)

- Added the same yellow Buy Me a Coffee button, dimensions, accessible image label
  and codedbydan destination as the clings README, at the bottom of Momentum’s README.
- Documentation only; native tipping remains Planned. Checked source markup, final
  placement and `git diff --check`. Changes remain local in `task/ios-app`.

## 2026-09-16 — README header shields

- Replaced the older badges with two centered rows directly under the title:
  live main-branch CI, GPL license, Rust core, SwiftUI, Linux, macOS and iOS 26/27.
- Consistent for-the-badge styling, dark backgrounds, orange technology icons,
  descriptive alt text and useful links. iOS remains explicitly in development.
- Documentation only; checked badge responses, link targets and formatting.

## 2026-09-16 — floating iOS Add task (F-019/F-020)

- User requested AsNeeded's bottom-right floating action. Moved the existing
  context-aware Quick Add action out of TaskScreen's top toolbar into a trailing
  bottom safe-area inset, above the tab bar and operation feedback. List options,
  Undo and task creation semantics remain unchanged; Settings has no Add action.
- Native capsule button uses the selected accent and its readable foreground,
  SF Symbols plus, standard press feedback and localized accessibility label.
  The full label yields to the symbol when it cannot fit horizontally, avoiding
  a multi-line floating control at large Dynamic Type. No polling, new animation,
  dependency or shared-core change.
- Actual preview captures exposed an oversized German AX5 capsule in the first
  revision; the responsive label corrects it. Refreshed the README iOS preview
  using isolated demo data. Before: `/tmp/momentum-floating-before.png`.
- Validation results and final capture evidence recorded below. Desktop/Android
  unaffected by this explicitly iOS-scoped request. Existing accessibility audit
  findings remain open; this change does not establish full accessibility/parity.
- Final dirty-worktree verification: both scoped XCTest UI flows passed on iOS
  26.5 and 27 (two per OS): three-tab floating-action geometry/label/sheet opening,
  Settings exclusion, create/complete/undo and persistence after relaunch.
  Results: `/tmp/momentum-floating-final-26.xcresult` and
  `/tmp/momentum-floating-final-27.xcresult`.
- Native component suites: nine tests per OS passed, including compact-width
  large-text fallback in both appearances. iOS 26.5: 0.361s test execution,
  13.071s command; iOS 27: 0.416s test execution, 6.2s command.
  Reports: `ios/DerivedData/TestReports/views-1789610149465647000.xcresult` (26.5)
  and `views-1789610121280371000.xcresult` (27).
- Reproduction commands (Xcode 27 selected):
  `python3 ios/scripts/test.py views --destination 'platform=iOS Simulator,id=681FF376-73A1-4EFC-807E-E858C7B5B7C2'`
  (substitute `634E2736-095B-472A-9488-CAB080627674` for 26.5); app UI checks:
  `xcodebuild -project ios/Momentum.xcodeproj -scheme Momentum -destination 'platform=iOS Simulator,id=681FF376-73A1-4EFC-807E-E858C7B5B7C2' -derivedDataPath /tmp/momentum-ios-appearance-current -parallel-testing-enabled NO -collect-test-diagnostics never -only-testing:MomentumUITests/LaunchTests/testFloatingAddStaysAboveTabsAndOpensQuickAdd -only-testing:MomentumUITests/LaunchTests/testTabsCreateCompleteUndoAndRelaunch CODE_SIGNING_ALLOWED=NO test`.
- Visual evidence: final native iOS 27 light/dark and German AX5 captures at
  `/tmp/momentum-floating-light.png`, `/tmp/momentum-floating-dark.png`,
  `/tmp/momentum-floating-large-de.png`; light preview is also in
  `data/resources/screenshots/ios-preview.png`. German catalog 211/211 and
  `git diff --check` pass. Scoped change verified; physical-device interaction,
  spoken VoiceOver and broader audit remediation were not rerun.

## 2026-09-16 — native dark app icon (F-019)

- User requested black gradient / #FF6600 checkmark and preferred `.icon`. Added
  `ios/Momentum/Resources/MomentumIcon.icon` with two editable SVG outline layers,
  default white/orange appearance and a dark variant over Apple's automatic dark
  gradient. Motion lines retain their softer hierarchy. Full-bleed source; system
  enclosure/masks. Foreground effects disabled to preserve requested color.
- `ios/project.yml` selects MomentumIcon as primary icon. XcodeGen recognizes
  `wrapper.icon`; excluded it from the fast native view-test resource target.
  Existing bitmap retained as reference. No dependencies/shared-core/UI logic changes.
- Scoped verification on dirty `task/ios-app` based on `7ec82e9`: final Xcode 27
  simulator app build passed (`/tmp/momentum-dark-icon-final-build.log`); app plist
  selects MomentumIcon for both iPhone and iPad. Existing unrelated Text-concatenation
  deprecation remains. No icon compiler errors or warnings.
- Native `ictool` exports inspected for iOS 26/27 dark and default, iOS 27 tinted,
  and 64-point legibility. Filled outlines corrected an intermediate iOS 26 stroked-path artifact.
  Both dark exports decode to exact `[255,102,0,255]` in sRGB inside the checkmark,
  with a neutral dark gradient. A false shifted reading came from converting an
  already decoded calibrated NSColor; direct sRGB Core Graphics decode verified it.
- Reviewable previews, source and exact export command: [icon design notes](../ios/Design/README.md).
  JSON/SVG parse and `git diff --check` pass. Physical-device Home Screen appearance
  selection not tested; renderer/build validation does not close broader iOS parity
  or accessibility findings. No commit, push or phone installation.

## 2026-09-16 — shared task automation and URL entry (F-031/F-040)

- Dirty `task/ios-app`, based on `7ec82e9`; local changes only. iOS now compiles
  the existing Mac App Intents, entities and localized phrases. Create, find,
  complete/reopen, plan Today and open preserve Mac parameters/results. Shared
  `EngineAutomation` maps native parameters to Rust; iOS runs it on `EngineWorker`,
  while the Mac wrapper retains `AppState.apply` for refresh/index/undo/sync.
- One mobile model/engine handles UI, URL, notification and intent entry. Concurrent
  cold model tests prove a single worker and list invalidation. Open Task defers
  presentation while an existing capture/editor owns a draft. Background mutations
  await the notification reconciliation drain. iOS intent authentication is required.
- Shared URL decoding supports both registered schemes and desktop aliases,
  first-value parameters, notes/due/tags and title completion. URL creation without
  a due date remains unscheduled; receiving a URL does not select another tab.
- Independent source review caught suppressed startup errors and repeated per-ID
  lifecycle work in entity resolution. Replaced that loop with one throwing runtime
  acquisition and one actor batch; stale query IDs are omitted, while mutations
  still validate the entire batch. Red/green regression and follow-up review passed.
- Native URL acceptance passed on iOS 26.5 and 27 through a warm Safari handoff
  and a cold `XCUIApplication.open` invocation, including selected-tab preservation,
  persistence, complete and Undo. Final iOS 27 suite:
  `/tmp/momentum-automation-native27-registration.xcresult` (two passed, 72.246s);
  final iOS 26.5 suite: `/tmp/momentum-automation-native26-development.xcresult`
  (two passed, 62.500s). Earlier URL failures
  were test assumptions, Safari selector discovery and a prohibited URL open
  from the test runner, not hidden product fixes.
- **The App Intents test framework remains blocked.** Apple's iOS 27 test framework
  rejected the Customer runtime with `AppIntentsServicesSecurityErrorDomain` 803.
  The dedicated iOS 27 test target skips only this restriction; unexpected errors
  still fail. A separate target prevents iOS 27 framework dependencies from breaking
  the iOS 26 UI-test loader. Source/build/adapter tests do not prove Shortcuts
  invocation, authentication or suspended execution. Native Shortcuts discovery
  separately shows all five actions on iOS 26.5/27, and Create Task passes via the
  actual system title prompt with task readback and persistence after relaunch:
  `/tmp/momentum-automation-native27-registration.xcresult` (Create Task 38.991s).
  The other actions and cold/background/locked-device invocation still need
  acceptance. Mac cold invocation and earlier deferred Spotlight acceptance
  are unchanged.
- Final package regression: 73 mobile tests/14 suites (0.110s execution) and
  127 Mac tests/30 suites (1.968s) passed. Logs:
  `/tmp/momentum-automation-mobile-final-tests.log` and
  `/tmp/momentum-automation-macos-final-tests.log`. Final Mac app build also passed
  (`/tmp/momentum-automation-macos-final-build.log`). Full parity is still open,
  including sync/backup, Spotlight, keyboard commands, accessibility remediation
  and device/background/performance work.
- iOS startup refreshes App Shortcut parameters outside preview mode, matching
  the existing Mac hook and [Apple’s sample](https://developer.apple.com/documentation/appintents/acceleratingappinteractionswithappintents/).
  The final iOS 27 native pass includes this registration.
- The separate `MomentumIntentTests` target builds and loads on iOS 27. Its one
  chain test explicitly skipped on security error 803 (5.700s); no native chain
  acceptance is claimed. `/tmp/momentum-intent-harness27-final.xcresult`.
- Final iOS 27 fast native suite: 11 tests passed (0.406s execution, 8.845s
  command), including the two real-model concurrency/routing tests and nine
  rendering/layout checks. Result:
  `ios/DerivedData/TestReports/views-1789617930383299000.xcresult`.
- iOS 26.5 initially failed Shortcuts provider resolution before the title prompt.
  A disposable one-action app reproduced the same code 1 error under unsigned and
  ad-hoc signing. Its runtime log reported missing team identity. Actual signing
  with an existing Apple Development identity (verified certificate authority and
  populated TeamIdentifier) made the probe pass, then made Momentum pass too.
  Xcode signing arguments alone retained an ad-hoc product; exact bundle signature
  inspection was necessary. No application workaround or security/account change.
  Probe evidence: `/tmp/momentum-intent-probe/actual-development-result.xcresult`;
  Momentum: `/tmp/momentum-automation-native26-development.xcresult`, Create Task
  33.735s and URL 28.765s. [QA signing steps](../ios/TESTING.md#url-and-app-intents-acceptance).
- Final iOS 26.5 fast native suite: 11 tests passed (0.334s execution, 3.825s
  command); `ios/DerivedData/TestReports/views-1789618136425182000.xcresult`.
  Final German catalog coverage 239/239 and `git diff --check` pass. URL acceptance
  is separately Verified as F-040; broader native automation remains In progress
  as F-031. Physical-device behavior and the remaining actions are not inferred
  from simulator or adapter results.
- Local documentation links pass. Both dedicated QA simulators were restored to
  their initial Shutdown state; no phone installation, commit, push or release.

## 2026-09-17 — backup and Nextcloud safety foundation (F-025/F-029)

- Prior goal turn classified as progress: shared automation code and native evidence
  changed authoritative state. Current dirty `task/ios-app` remains based on `7ec82e9`;
  all work local. Read current source before choosing this next parity prerequisite.
- Reproduced B-019/B-020 with a paused real HTTP exchange and disposable stores.
  Original sync reported success after restore and regressed the concurrent edit's
  clock. Initial fixture failure was an accepted nonblocking socket on macOS;
  explicitly blocking that fixture socket exposed the intended product failures.
- Shared exchange now mutates only its in-memory snapshot. Final persistence is under
  the engine lock after generation validation and stable-ID pending merge; sync stays
  busy until that commit finishes. Local edits run while the GET is paused. Retain
  live vector clocks, summary claims and nearby metadata. Persistence errors propagate.
- Replacement invalidates later network steps as well as the local commit. A default
  30-second whole-exchange deadline bounds network waits; each request uses its remaining
  budget. Collection creation retries once rather than recursively without a bound.
  Cancellation cannot recall a server-accepted request; it prevents subsequent work
  and local commit. External provider/background cancellation and P2P integration are
  not yet implemented. No mobile sync/restore interface has been exposed prematurely.
- B-021: arbitrary JSON can no longer silently mean an empty backup. Valid raw/wrapped
  exports and explicitly empty task collections still parse. Invalid import preserves
  tasks, pending operations, an actual existing undo batch and reopened disk state.
- Targeted evidence: `/tmp/momentum-transport-red.log`,
  `/tmp/momentum-transport-cancel-red.log`, `/tmp/momentum-backup-validation-red.log`,
  `/tmp/momentum-backup-core-red.log`, and `/tmp/momentum-transport-final-targeted.log`.
  Disabling the deadline check deliberately fails the stalled-request regression
  (`/tmp/momentum-deadline-fault.log`); source was restored before broad checks.
- Remaining before native restore: P2P late-result exclusion and provider ownership,
  explicit cancellation/expiration boundary, import persistence-error handling and
  interrupted multi-file commit recovery, then security-scoped picker/export and
  destructive confirmation UI. Current Store replacement still ignores save errors;
  these fixes do not establish that broader durability claim.
- Broad Rust validation passed: 213 tests with
  `cargo test --workspace --exclude momentum`, and 213 with
  `--all-features --all-targets`, including CLI, notification ledger, model,
  store, sync and local P2P fixtures. Logs: `/tmp/momentum-transport-workspace.log`
  and `/tmp/momentum-transport-all-features.log`. Five new core transport/import
  tests, two exchange tests and one model validation test are included.
- Regenerated Apple Rust/UniFFI artifacts for Mac, device, arm64 simulator and
  x86_64 simulator (`/tmp/momentum-transport-apple-core.log`). Public generated FFI
  APIs were not hand-edited or expanded; native clients retain their existing API.
- Independent agent review could not start because the session's agent-thread
  limit rejected both dispatch and reuse. Local source/diff review continues;
  independent final review remains an explicit completion gate.
- Final Apple consumer validation: macOS app build and iOS simulator app build passed
  (`/tmp/momentum-transport-macos-build.log`, `/tmp/momentum-transport-ios-build.log`).
  127 MomentumKit tests/30 suites passed in 2.313s; 73 mobile tests/14 suites passed
  in 0.107s (`/tmp/momentum-transport-macos-tests.log`,
  `/tmp/momentum-transport-mobile-tests.log`). These are package execution timings,
  not full build times. No native UI or physical-device acceptance is claimed for
  this shared-core step; no mobile document interface was added.
- Final source review confirms exchange contains no persistence call, public `sync`
  retains persistence for exclusive-store consumers, and FFI signatures are unchanged.
  Documentation links and `git diff --check` pass. User stores, credentials, physical
  devices and existing simulators were not used; all network fixtures use loopback.


### 2026-09-17 — journaled restore and checked Apple startup (B-022/B-023)

- **Goal classification: progress.** macOS feature parity on iOS remains the completion
  criterion. Work is local in dirty `task/ios-app` (base `7ec82e9`); no commit, push,
  release, personal task-store mutation or physical-device installation.
- Added an undo journal for the existing state/pending/metadata JSON files. Failed
  import now reports an I/O error and retains live tasks, pending operations and undo;
  startup recovers an interrupted transaction before exposing a store. Corrupt or
  incomplete recovery records remain preserved. The CLI reports a normal JSON error,
  and file-monitor reload retains its existing owner after a recovery failure.
- Added checked UniFFI startup. iOS shows a native unavailable screen with retry and
  diagnostic details; successful retry restores the existing tab selection. macOS
  presents a native retry/quit alert. English/German copy is included. Linux's legacy
  fatal recovery boundary remains an explicit implementation gap.
- Tests exit a real child process at six commit boundaries and compare all three
  files with their coherent old/new images. They also cover interrupted first save,
  failed later writes, invalid/incomplete recovery records, safe orphan cleanup,
  failed import, checked startup and retry. This is process-interruption evidence,
  not a physical power-loss guarantee or cross-process locking implementation.
- Durability increased repeated-write cost. Demo/preview seeding now uses one ordered
  durable batch; task rules, operation ordering and clocks remain tested. The core
  suite fell from 41.59 to 17.41 seconds with batching, but remains slower than the
  preceding 1.09-second run. B-023 remains open for isolated action/bulk measurements,
  broader batching and device energy/latency acceptance. No test disables durability.
- Shared verification: store 14/14; final standard and all-feature/all-target Rust
  workspaces each passed 225 tests, including the final file-monitor regression.
  Final Swift package and native-view verification is recorded below when complete.
- Independent review remains unavailable because this task's agent thread capacity
  was exhausted earlier; no fresh independent review is claimed. Manual review found
  and covered the file-monitor panic path. Remaining parity also includes provider/
  P2P cancellation, mobile document flows, notifications/background/device checks,
  Spotlight, keyboard and the outstanding accessibility findings.

- Final package checks passed: 74 mobile tests/14 suites in 2.168 seconds and 127 Mac
  tests/30 suites in 15.823 seconds, run sequentially after core preparation. Demo
  batching reduced the Mac run from its initial 119.134 seconds, but both remain
  slower than the previous transport-only baseline; B-023 stays open.

- Image inspection caught B-024: a German locale in the old component host still
  rendered English because its main bundle lacked the app catalog. Added the real
  catalog and an explicit German-resource assertion. Corrected captures now show
  German in both appearances. Retry stays in a bottom safe-area inset so the long
  explanation can scroll independently at AX5; a half-point size tolerance accounts
  for native pixel rounding at the 280-point fixture width.

- Final native component/model tests: 13/13 on iOS 27 in 0.900 seconds and 13/13 on
  iOS 26.5 in 0.932 seconds (runner wall time 19.029/14.231 seconds). Result bundles:
  `ios/DerivedData/TestReports/views-1789652315821890000.xcresult` and
  `views-1789652334970159000.xcresult`. Scoped recovery/locale/rendering checks passed;
  spoken VoiceOver, full-app retry interaction and the Mac alert remain unverified.
- An otherwise idle 30-add offline CLI fixture measured median 55.89 ms, p95 61.63 ms,
  max 64.38 ms including process startup/load; all data was temporary. This is not an
  isolated core, iPhone, bulk-action or energy measurement. B-023 remains open.
- Both QA simulators were already Shutdown after native tests; no device data was
  erased. German catalogs have complete coverage (247 iOS / 301 Mac nonempty
  translatable entries); changed-document relative links and diff whitespace pass.

- Verification logs for this pass are `/tmp/momentum-durability-final-{workspace,
  all-features,mobile-tests,macos-tests,views27,views26}.log`; the final component
  results supersede the first English-only captures. Both 26.5 and 27 German AX5
  light/dark images were inspected. Final build logs use the `verified-` prefix.
- Next concrete work: batch production core mutations without weakening failure/
  crash semantics (B-023), add a recoverable Linux startup boundary (B-022), then
  finish provider/P2P replacement exclusion and native iOS backup import/export.
  Full parity and accessibility completion are not claimed by these scoped checks.

- Final macOS and iOS simulator app builds passed after the last UI/core edits;
  both linked binaries contain the final reload guard. All four Apple Rust slices
  were regenerated; fast-test preparation passed in 2.812 seconds. Platform-wide
  status remains In progress/Implemented because the recorded native and performance
  gaps remain, despite the passing scoped builds and tests.


### 2026-09-17 — one durable commit per logical edit (B-023/B-025)

- **Previous goal turn: progress.** The crash journal and checked startup are real
  implementation/test evidence, while iOS parity remains unfinished. Current work
  stays local in dirty `task/ios-app`, preserving the existing user work.
- Root cause confirmed: completing twenty tasks previously produced twenty sync
  wakeups observing one additional completed task each. Private candidate transactions
  now publish the complete durable batch once. Failed saves retain original tasks,
  pending operations, clocks and undo, with a structured save-error outcome.
- Bulk completion/delete/reopen, scheduling/tag/project moves, reorder/undo, form
  saves, compound capture/paste, repeat creation and CLI dispatch use the shared Rust
  rules. No duplicate native task rules or disabled durability were introduced.
- One disposable debug sample of 100-task actions measured completion 48.93 ms,
  undo 52.60 ms, and pasting 100 tasks 56.29 ms, including journaled persistence.
  This is a local sample, not iPhone frame/battery acceptance. Single-write cost,
  large-store memory/latency and native error handling remain part of acceptance.
- The independent-agent review gate is still unavailable at the task's exhausted agent
  capacity; no new independent review is claimed.
- Final shared checks passed 234 tests in each standard and all-feature/all-target
  workspace run, with the manual latency benchmark intentionally ignored. The GTK
  consumer compiles with its tests; this is not Linux interaction verification.
- Final Swift package checks passed: 78 mobile tests/14 suites in 2.171 seconds and
  128 Mac tests/31 suites in 11.466 seconds. These include failed notification-action
  retry, rejected capture without orphan tags (B-026), automation failure propagation
  and Mac form outcomes. macOS explicit Create/Done retains the sheet on save error;
  alternate dismissal and Linux draft recovery still need separate acceptance.
- Three full-app flows passed on each iOS 26.5 and 27 simulator: edit/save/delete/undo,
  duplicate with current draft, and tabs/create/complete/undo/relaunch. Result bundles
  are `/tmp/momentum-batching-native26.xcresult` and
  `/tmp/momentum-batching-native27.xcresult`. The 26.5 app preceded the last rejected-
  capture guard; the final native component/model runs below cover that guard on both.
- Final native component/model suites passed 14/14 on iOS 27 in 1.026 seconds and
  iOS 26.5 in 1.025 seconds. The added app-model test uses the real Rust engine to
  verify rejected tag-only input, retained drafts after write failure, no success
  revision/feedback on failure, and exactly one task after retry. Result bundles:
  `ios/DerivedData/TestReports/views-1789654125131988000.xcresult` and
  `views-1789654147643274000.xcresult`. Both QA simulators ended Shutdown; no erase or
  personal-device installation occurred.
- All three Apple string catalogs have German coverage for nonempty translatable
  keys. Changed-ledger relative links and diff whitespace checks pass. These scoped
  checks do not verify spoken VoiceOver, full-device energy/frame performance,
  provider/P2P replacement exclusion, native backup flows or complete app parity.
- Final macOS app build/relink passed with the final core guard; all four Apple core
  slices were regenerated and fast-test preparation passed in 3.025 seconds. Logs:
  `/tmp/momentum-batching-final-{mobile,macos,views27,views26,macos-build,prepare}.log`;
  shared checks use `/tmp/momentum-batching-verified-{workspace,all-features,gtk}.log`.
  No commit or push. Next: close provider/P2P replacement exclusion before exposing
  native backup restore, then continue the outstanding platform integration checks.

### 2026-09-17 — durable nearby receive and restore exclusion (B-027/B-028)

- **Previous goal turn: progress.** It completed final shared/Apple regression
  verification and recorded real remaining parity gaps. Current work stays local in
  dirty `task/ios-app` based on `7ec82e9`; no release or personal data is involved.
- Closed shared-core gaps that blocked safe native backup work: nearby inbox
  consumption now follows a durable candidate commit, stopped runtimes cannot apply
  late data, start/stop/restore serialize provider admission, and opposite providers
  are rejected while one is active. A persisted restore epoch excludes pre-restore
  queued data after process restart while retaining pairing identity and trust.
- Regression fixtures reproduced failed peer publication, active-runtime restore,
  stale queued data after restart and silent corrupt-journal reset, then passed with
  the fixes. Corrupt journals now remain intact and startup returns an error.
- A separate stalled-socket fixture exposed uncancelled Sync All requests: shutdown
  took 5.002 seconds before the fix. Supplying LibreSync's cancellation token closes
  those sockets promptly; bounded discovery still has its two-second timeout.
- Focused checks passed nine core P2P tests and eight transport unit tests, including
  actual isolated runtimes, failed saves/retry and acknowledgement with newer arrivals.
  Final full workspace and Apple consumer verification is recorded below when done.
- Independent review remains unavailable at this task's previously confirmed agent
  capacity limit. Manual review covered lock order, durable restore boundaries,
  cancellation propagation and acknowledgement ordering; this is not independent
  review or physical-device sync/accessibility acceptance.
- Before the final acknowledgement-barrier correction, standard and all-feature/
  all-target Rust workspace runs each passed 243
  tests, with the manual edit benchmark ignored. GTK consumer/test compilation also
  passed. The full runs include the real local two-device pairing/exchange/bootstrap
  fixture, not just mocks. Logs: `/tmp/momentum-p2p-final-{workspace,all-features,gtk}.log`.
- Regenerated all four Apple core slices. Swift packages passed 78 mobile tests in
  2.119 seconds and 128 Mac tests in 12.511 seconds. Logs:
  `/tmp/momentum-p2p-{prepare,mobile,macos}.log`. These precede the final barrier
  correction; final acceptance checks follow below.
- Remaining transport acceptance includes acknowledgement failure followed by
  provider switching, bounded Nextcloud lifecycle cancellation, native recovery and
  physical-device behavior. Close those boundaries before treating mobile backup
  restore or full iOS sync parity as verified.
- Manual review found a final journal race: store application could finish before
  stop, but acknowledgement could write the old journal after a replacement runtime
  started. A deterministic paused-acknowledgement test reproduced it. Stop now drains
  the whole receive/commit/ack section; callbacks run after releasing the barrier to
  permit synchronous stop. The earlier iOS 26 component run correctly refused stale
  preparation after this source edit; it is not counted as a pass. Verification was
  restarted from frozen sources, using `/tmp/momentum-p2p-accepted-*.log` below.
- Frozen-source Rust acceptance passed 245 tests in each workspace configuration,
  including both new barrier/delegate cases; GTK consumer/test compilation passed.
  Regenerated Apple packages passed 78 mobile tests/14 suites in 2.239 seconds and
  128 Mac tests/31 suites in 12.318 seconds. These supersede the pre-barrier counts.
- Both final Apple app builds passed. Native component/model suites passed 14/14
  on iOS 27 in 1.094 seconds and iOS 26.5 in 0.868 seconds. Final result bundles:
  `ios/DerivedData/TestReports/views-1789655405060174000.xcresult` and
  `views-1789655546764984000.xcresult`. The first 26.5 run reported only 13 tests,
  omitting TaskMutationIntegrationTests despite its presence in the built binary.
  Explicit selection passed that test; a subsequent full run executed all 14.
  The omission's cause remains unconfirmed, and the 13-test result is not used as
  complete-suite evidence. Keep checking executed inventory, not exit status alone.
- Both QA simulators ended Shutdown. No personal-device installation, data erasure,
  commit or push occurred. Final preparation refreshed in 2.973 seconds; changed
  documentation links and diff whitespace pass. Native P2P permission/lifecycle and
  physical-device convergence remain unverified; these component tests validate
  existing app/model behavior against the updated core, not the future sync UI.
- Next concrete work: reproduce and cover acknowledgement failure across a provider
  switch, then complete explicit Nextcloud cancellation and native backup/provider
  flows. Full parity remains active; accessibility and device acceptance are still
  required in addition to the passing scoped checks.

### 2026-09-17 — peer retry receipts and explicit Nextcloud cancellation (B-029/F-025)

- **Previous goal turn: progress.** It fixed receive/restore/acknowledgement races,
  added cancellable peer requests and verified the shared/Apple consumers. Current
  work remains local in dirty `task/ios-app` based on `7ec82e9`.
- Reproduced the remaining cross-provider retry failure with a real isolated
  Nextcloud exchange: a previously received AddTask replayed after pending cleared,
  undoing a later completion. Added durable exact receipts separate from the upload
  queue. Bootstrap-covered operations get receipts too; the next receive prunes
  acknowledged IDs so ordinary receipt history does not grow indefinitely.
- Added the core/UniFFI `cancel_nextcloud` boundary. Admission and snapshot-generation
  capture now share one owner lock; cancellation invalidates that generation under
  the same lock used for final commit. The active lease remains held until the actual
  request/exchange returns, preventing premature admission of another provider.
- Cancellation is cooperative between requests and before commit. It does not
  forcibly close an in-flight ureq socket; the existing 30-second whole-exchange
  deadline still applies. Its Boolean means an active exchange was signalled, not
  that networking has stopped. Native lifecycle code must await the exchange.
- Red/green HTTP-boundary tests verify no later PUT/commit/success after cancellation,
  preserved edits on disk, a retained active lease while the request drains, idle
  cancellation behavior and a successful subsequent exchange. Full final checks
  follow below; native lifecycle/permission/device acceptance is still outstanding.
- Independent review remains unavailable at the previously confirmed agent capacity
  limit. No independent review, physical-device validation or full parity is claimed.
- Final shared verification passed 248 tests in each standard and all-feature/
  all-target workspace configuration, with the manual benchmark ignored. The core
  also compiles without its optional P2P feature, and the GTK consumer/test target
  compiles. Logs: `/tmp/momentum-receipts-final-{workspace,all-features}.log`,
  `/tmp/momentum-receipts-no-p2p.log` and `/tmp/momentum-receipts-gtk.log`.
- Regenerated all four Apple core slices and the Swift binding, which exposes
  `cancelNextcloud() -> Bool`. Package checks passed 78 mobile tests/14 suites in
  1.972 seconds and 128 Mac tests/31 suites in 12.492 seconds. Logs:
  `/tmp/momentum-receipts-{prepare,mobile,macos}.log`. Final app/native checks follow.
- Final macOS and iOS simulator app builds passed. All 14 native component/model
  tests executed and passed on iOS 27 (1.167 seconds) and iOS 26.5 (0.954 seconds).
  Result bundles: `ios/DerivedData/TestReports/views-1789656163057638000.xcresult`
  and `views-1789656187910677000.xcresult`; complete test inventories were checked.
  Final build/view logs are `/tmp/momentum-receipts-{macos-build,ios-build,views27,
  views26}.log`. Preparation refreshed in 2.785 seconds; documentation links and
  diff whitespace pass. No commit, push or physical-device installation.
- Next concrete work: native Settings backup import/export, using the shared core
  and system document interfaces, explicit restore confirmation, preserved failure
  state, and task/notification refresh after successful replacement. Then integrate
  provider settings and lifecycle cancellation with native permission/device checks.
  Passing component tests do not verify those still-missing flows or full parity.

### 2026-09-17 — native iOS backups and Settings text contrast (F-029/B-030)

- **Previous goal turn: progress.** Peer retry receipts and explicit Nextcloud
  cancellation were implemented and checked across shared/Apple consumers. This
  turn continues locally in dirty `task/ios-app`, based on `7ec82e9`; no commit/push.
- Added a focused Backups page in Settings with native JSON document import/export,
  SF Symbols, English/German descriptions, explicit irreversible replacement
  confirmation, progress, retryable errors and success only after the actual save
  or restore completes. File reads coordinate security-scoped access off the main
  actor and retain immutable bytes through confirmation. Temporary core files are
  removed after use; Rust retains serialization, validation and replacement rules.
- Restore protects an active capture/editor draft, prevents interactions during
  replacement, preserves the selected tab and uses normal task invalidation and
  notification reconciliation after success. No new dependency, schema or Rust
  change. Physical notification delivery is not inferred from that refresh path.
- The new native audit reproduced insufficient title contrast in Settings buttons
  (B-030). Explicit `Color.primary` fixes the text while retaining exact #FF6600
  decorative symbols. The iOS 27 contrast/hit-region/element-detection audit passed
  after the fix; the original failure is `/tmp/momentum-backup-ui-27.xcresult`.
- Fast mobile checks passed 86 tests/15 suites in 2.627 seconds, including eight new
  backup tests. Those cover actual Rust round trips, malformed JSON, failed-write
  retry, cancelled reads, operation exclusion and export completion semantics.
  The first native iOS 27 run passed all 18 component/model tests in 1.570 seconds.
  Final native/UI results after the contrast correction are recorded below.
- Initial UI round-trip testing saved a real local file, then failed because the
  importer opened an empty Recents screen. The test now follows Browse → On My
  iPhone. Cancellation assertions were strengthened to require re-enabled app
  controls, rather than presence of views beneath a system sheet. Earlier weak
  cancellation results are not used as complete acceptance evidence.
- Independent review remains unavailable at the previously confirmed agent-capacity
  limit; review here is manual. Third-party/cloud file providers, spoken VoiceOver,
  physical-device performance and provider-lifecycle acceptance remain separate.
- Final backup UI acceptance passed two tests on each runtime: iOS 27 in 82.988
  seconds and iOS 26.5 in 72.420 seconds. Both exercised real local Files export,
  cancelled confirmation preserving newer tasks, confirmed replacement, selected-tab
  preservation, cold persistence, interactive export dismissal and import cancellation.
  Both initial-screen contrast/target/element audits and replacement-alert contrast
  audits passed. Bundles: `/tmp/momentum-backup-final-audit27.xcresult` and
  `/tmp/momentum-backup-final-audit26.xcresult`. No cloud/provider account was used.
- Final backup component/model checks executed all 18 tests on each runtime:
  iOS 27 in 1.622 seconds (`views-1789657867984507000.xcresult`) and iOS 26.5 in
  1.412 seconds (`views-1789657892897116000.xcresult`), under
  `ios/DerivedData/TestReports`. Preparation refreshed in 3.670 seconds.
- During final verification, the user requested white text on Today’s Add task
  button. Today now uses white label/plus without changing the fill; Increase
  Contrast retains adaptive contrasting ink. This is an explicit styling preference,
  not a resolution of B-013’s standard-contrast limitation. Native checks after
  that scoped change are recorded below. Other task screens retain their existing
  foreground behavior.
- Next parity work: native provider settings/lifecycle cancellation with Keychain
  error handling and isolated transport acceptance, then remaining Spotlight, native
  automation, notification/background and physical accessibility/performance checks.
  Full macOS parity is still active and incomplete.
- After the white Today-button change, all 18 native component/model tests passed
  again on iOS 27 (1.497 seconds) and iOS 26.5 (1.331 seconds). Bundles:
  `ios/DerivedData/TestReports/views-1789657942025548000.xcresult` and
  `views-1789657961533229000.xcresult`. Light/dark button renders were inspected and
  show white text/plus. The German AX5 backup render also shows the corrected system
  title color. This final view build compiles the real changed TaskScreen; a new
  physical-device installation was not performed.
- Both QA simulators were confirmed Shutdown after verification. Changed local
  documentation links and diff whitespace pass. No data erasure, commit, push or
  personal-device installation occurred.

### 2026-09-17 — sync operation admission and checked mobile credentials (F-025/B-031/B-032)

- **Previous goal turn: progress.** Native backups were implemented and verified on
  both iOS versions; the requested white Today action was applied with the existing
  Increase Contrast fallback. Current changes remain local in dirty `task/ios-app`,
  based on `7ec82e9`; full macOS parity is still active and incomplete.
- Added a single-use Rust `SyncCancellation` object and additive
  `sync_nextcloud_cancellable`. An atomic phase transition covers the gap before
  blocking FFI admission, ongoing HTTP checkpoints and final commit. Queued/active
  cancellation is accepted; cancellation after commit admission or completion is
  rejected. The transport lease remains held until I/O drains. No immediate socket
  abort is claimed; the existing exchange deadline remains in force. The desktop
  method delegates through a fresh operation and preserves its signature.
- Added checked shared Apple Keychain Data APIs and accurate failed-deletion results.
  The mobile connection is a single atomic device-only record, including all fields
  and secrets. Missing, denied and corrupt records are distinct outcomes. Existing
  desktop accounts/default protections and task/backup formats are unchanged.
- Added the mobile operation adapter: network FFI runs on a detached utility task,
  leaving the local engine actor available. Swift cancellation signals the core
  handle even when already cancelled before dispatch. A commit that wins the final
  boundary remains a reported success; cancellation cannot conceal committed data.
- Initial new-API tests failed to compile as expected before implementation; these
  are API-first red checks, not claims of pre-fix runtime reproduction. Runtime
  tests now cover pre-admission cancellation, paused real HTTP cancellation, no
  upload/commit, pending edits, lease retention, fresh/reused handles, Keychain
  failure statuses, actual isolated record round trips and corrupt-record retention.
- Standard and all-feature/all-target Rust workspace configurations each passed
  250 tests; no-P2P core and GTK consumer/test compilation passed. Logs:
  `/tmp/momentum-sync-operation-{workspace,all-features,no-p2p,gtk}.log`. All four
  Apple slices and bindings were regenerated through the existing build tooling.
- Apple packages passed 92 mobile tests/16 suites in 2.339 seconds and 133 Mac
  tests/32 suites in 11.607 seconds. Nine Python runner regressions pass. Logs:
  `/tmp/momentum-sync-operation-{mobile,macos,runner-tests}.log`.
- The real native Keychain test exposed unsigned-host failure `-34018` (B-032).
  Local ad-hoc simulator signing fixes it without a personal certificate or team.
  Both final native runs executed all 20 tests: iOS 27 in 1.540 seconds and iOS 26.5
  in 1.556 seconds. Keychain attributes read back After First Unlock This Device Only,
  and queued cancellation preserves the app’s real isolated core store. Bundles:
  `ios/DerivedData/TestReports/views-1789658783726549000.xcresult` and
  `views-1789658803097932000.xcresult`. Locked physical-device behavior is unverified.
- Review capacity was rechecked; a precisely scoped independent review dispatch
  still failed with the agent thread limit. Review for this change is manual; no
  independent review is claimed. Both Debug app builds subsequently succeeded:
  `/tmp/momentum-sync-operation-macos-build.log` and
  `/tmp/momentum-sync-operation-ios-build.log`. Core preparation was refreshed before
  the following native sync work.
- **Next concrete work:** bind the checked connection and cancellable operation to
  native Sync Settings and an event-driven coordinator. Preserve Off on fresh
  installs and in preview/tests, save errors and drafts, selected-provider exclusion,
  debounced edits, foreground/background teardown, restore admission and persistent
  status/retry. Then implement native LibreSync pairing and isolated peer acceptance.
  These foundation tests do not verify the still-missing UI or lifecycle behavior.


### 2026-09-17 — native Nextcloud settings and foreground coordinator (F-025/B-033/B-034)

- **Previous goal turn: progress through verification.** Readback confirmed the white
  Today action and both previously running app builds. Work remains local in dirty
  `task/ios-app` based on `7ec82e9`; no full parity completion is claimed.
- Added native Sync settings with a dedicated provider selection page, permanently
  labeled connection fields, secure password fields, encryption/compression/automatic
  options, save/retry, pending changes, last successful exchange and progress. EN/DE
  resources are included. A persistent task-list recovery link opens the same screen.
- New presentation coordinator starts Off by default, never reads production secrets
  or starts transport in preview/test mode, retains failed saves/drafts and saved
  connections when Off, and records persistent failure categories. Only Nextcloud is
  currently available on iOS; LibreSync remains a parity requirement, not a completed
  or intentionally inapplicable provider.
- Manual/automatic exchanges consume the existing shared engine through its utility
  adapter. Automatic edits coalesce for 20 seconds, successful/failed foreground
  checks recur after five minutes, and a foreground return retries after the prior
  cancellation drains. No schedule runs while inactive/Off. Sync commits refresh
  tasks/notifications without scheduling a new local-edit sync loop.
- Changing connections/providers and restoring a backup block admission and await
  cancellation drain. Scene inactivity cancels promptly. A UIKit execution allowance
  begins before transport and ends on completion/expiration; it does not promise
  indefinite background I/O or immediate socket abort. B-033/B-034 record runtime-red
  regressions caught and fixed during this implementation.
- First render inspection found provider truncation at AX5 and disappearing field
  labels. The provider now uses a navigation selection list, and fields retain their
  captions. The fixture was corrected to host the screen inside NavigationStack;
  otherwise SwiftUI disables navigation links and renders misleading dimmed labels.
- Verification and final native UI evidence follow below. Full iOS WebDAV exchange,
  production setup interaction, real suspension/expiration, LibreSync pairing,
  physical VoiceOver and energy verification remain outstanding. No real credentials,
  task data, external accounts or personal devices were used.

- The user then reported two-tone Today background bands (B-036). Source and
  [Apple's scroll-edge documentation](https://developer.apple.com/documentation/swiftui/scrolledgeeffectstyle)
  identify the explicit hard edge effect as the cause. RootView now hides those
  effects, preserving the native floating controls and adaptive grouped canvas.
  Native light/dark pixel checks cover all four tabs; results follow.
- The first actual Sync Settings audit caught a dimmed disabled provider label in
  isolated mode (B-035). It is now a readable, noninteractive Off status. The original
  failed Xcode invocation continued collecting a confirmed live `simctl diagnose`
  child (600-second bound); it was not restarted or treated as complete during that
  collection. Independent background verification uses the other QA simulator and
  a separate derived-data directory, with optional diagnostic collection disabled.

- Final coordinator package validation passed 103 tests/17 suites in 2.623 seconds
  (`/tmp/momentum-sync-ui-final-fast.log`). The native suites passed all 25 tests:
  iOS 27 in 1.891 seconds and iOS 26.5 in 1.808 seconds, with bundles
  `ios/DerivedData/TestReports/views-1789659753897716000.xcresult` and
  `views-1789659774066848000.xcresult`. These include actual isolated core/Keychain
  boundaries and injected UIKit allowance ordering, not live iOS HTTP acceptance.
- The original failed Sync audit finished with exit 65 after its optional diagnostic
  collector was explicitly stopped following process/parent confirmation. The failed
  bundle was finalized and preserved. The fixed Sync screen passed the scoped native
  contrast/hit-region/element audit on iOS 26.5 in the following UI run.
- The first background run passed three UI tests in 51.786 seconds at
  `/tmp/momentum-uniform-background26.xcresult`. Screenshot inspection showed that
  both appearance cases rendered light despite setting appearance before launch.
  This run proves the light canvas and Sync audit only. The test now switches style
  after launch and independently asserts canvas brightness, preventing false dark
  coverage. Final strengthened runs are recorded below.

- Strengthened actual-app background verification passed on iOS 27: three tests in
  56.655 seconds (`/tmp/momentum-uniform-background27.xcresult`), including Sync's
  native audit. The corrected iOS 26.5 light/dark cases passed in 41.339 seconds
  (`/tmp/momentum-uniform-background26-verified.xcresult`). All four tabs are sampled,
  actual appearance brightness is asserted, and iOS 27 light/dark Today screenshots
  were visually inspected. B-036's reported canvas bands are fixed for this scope.

### 2026-09-17 — consistent white default-orange button foreground (F-019/B-013)

- Latest user steering supersedes the Today-only override: default #FF6600 filled
  buttons use white text/symbols. The user clarified that Increase Contrast is the
  only exception and must retain calculated contrasting ink. The shared
  native accent policy now supplies this ink to every floating Add and sheet commit
  action; the default palette selection check follows it too. Custom colors keep
  their adaptive foregrounds. No shared-core or desktop behavior changed.
- Before implementation, updated native tests failed with six assertions across
  light/dark rendered sheet symbols and all four light/dark/contrast trait combinations
  (`views-1789660783584712000.xcresult`). The existing calculated default was black.
  B-013 remains open: approved standard-contrast white/orange styling is not a
  contrast remediation.
- The clarified Increase Contrast exception first failed both high-contrast trait
  assertions (`views-1789660880878941000.xcresult`), then passed after moving the
  decision into the dynamic UIColor resolver. Final full native suites passed
  25 tests each: iOS 27 in 1.933 seconds (`views-1789660911047873000.xcresult`)
  and iOS 26.5 in 1.870 seconds (`views-1789660931739741000.xcresult`).
  These verify exact orange, all light/dark/contrast trait combinations, actual
  white rendered commit symbols, and existing layout/integration regressions.
  Result bundles are under `ios/DerivedData/TestReports/`. Diff whitespace checks
  passed. Changes remain local; physical-device and full parity acceptance remain open.

### 2026-09-17 — real iOS Nextcloud transport and missing-folder fix (F-025/B-037)

- **Previous goal turn: progress.** The shared orange-button foreground and continuous
  canvas changes reached their recorded iOS 26.5/27 verification scope. Continued work
  uses the same dirty `task/ios-app` checkout based on `7ec82e9`; local changes only.
- Added an iOS loopback-only WebDAV peer using public Network APIs. Native tests execute
  the actual Swift coordinator, isolated Keychain persistence, UniFFI Rust transport,
  HTTP and disk. Authentication, ETags, collection creation and bounded held requests
  are implemented by the fixture; no live service or personal settings are touched.
- The first real exchange caught B-037: ureq rejected MKCOL before dispatch. A new Rust
  missing-collection case reproduced `MethodVersionMismatch(MKCOL, HTTP/1.1)` before
  the fix (`/tmp/momentum-dav-mkcol-red.log`). Allowing WebDAV extension methods in
  the existing scoped agent fixes every client of this shared transport. The initial
  Rust sync suite passed 13 tests, and Apple core artifacts were regenerated.
- Initial native iOS 27 transport acceptance then passed three tests in 1.656 seconds
  (`views-1789661226122134000.xcresult`). These cover plain/encrypted-compressed
  exchange, two peers' offline edits, ETag conflict retry, durable reopen, no unnecessary
  upload, authentication recovery, preserved credentials on Off, and an actual held
  GET cancelled by scene inactivity while local editing remains responsive.
- Added a fourth case for wrong-encryption-password failure, preserving both the remote
  payload and local pending edits, saving a corrected password, then converging both
  peers. Final full native, mobile, shared Rust/CLI and macOS results follow below.
- Remaining F-025 scope: interactive production-style setup, hosted Nextcloud/TLS,
  physical lifecycle/energy behavior and switching to the pending LibreSync native UI.
  A passing local HTTP fixture is not a claim that these broader cases are complete.
- Final shared checks passed: `cargo test --workspace --exclude momentum` and its
  `--all-features --all-targets` variant each ran 251 passing tests, retaining the
  one explicitly ignored manual latency sample. Logs: `/tmp/momentum-dav-workspace.log`
  and `/tmp/momentum-dav-workspace-all.log`. All 133 macOS MomentumKit tests passed
  in 26.680 seconds (`/tmp/momentum-dav-macos-tests.log`); 103 mobile package tests
  passed in 2.494 seconds (`/tmp/momentum-dav-fast.log`). GTK native runtime and
  hosted desktop server interaction were not rerun for this transport-only fix.
- The expanded native suite passed all 29 tests on iOS 26.5 and 27 before separation.
  To preserve the fast UI iteration loop, HTTP acceptance now has its own native
  `MomentumTransportTests` target/scheme and `test.py transport` lane. `all` includes
  fast, views and transport; normal fast/views never start the WebDAV peer. Empty
  transport filters fail, as verified by a runtime-red/green runner regression;
  all nine Python harness tests passed. No new runtime package dependency was added.
- The final separate transport target passed four tests on iOS 27 in 2.194 seconds
  (`transport-1789661397994970000.xcresult`) and iOS 26.5 in 2.367 seconds
  (`transport-1789661418312645000.xcresult`). The regular 25 native view tests remain
  green in 1.934 seconds on iOS 27 (`views-1789661431867614000.xcresult`). Bundles
  are under `ios/DerivedData/TestReports/`; generation and Swift consumers used the
  rebuilt current shared library. Rust formatting, diff whitespace and changed-doc
  local-link checks passed. No commits, pushes, releases or personal-device installs.
- Next: verify native setup/save/reopen/retry interactions in an isolated host, then
  implement native LibreSync pairing and provider switching. Hosted HTTPS, physical
  background/VoiceOver/energy checks and the rest of the parity checklist remain open.

### 2026-09-17 — native Nextcloud setup interaction acceptance (F-025/F-039)

- **Previous goal turn: progress.** Actual iOS loopback WebDAV, the shared missing-folder
  fix and separate transport lane reached their documented verification scope. Continued
  local work remains in dirty `task/ios-app` based on `7ec82e9`.
- Added a separate Sync QA application and UI-test target. It compiles the production
  SyncSettings/AccentTheme/SettingsLabel and coordinator instead of copies. SettingsLabel
  moved unchanged into Design so both products share it. The QA app accepts only a UUID
  fixture identity, uses a separate bundle/container and scoped Keychain/defaults, and
  refuses every transport URL except its own ephemeral loopback peer. The shipping
  app's disabled preview/test transport policy remains unchanged.
- UI acceptance enters credentials, edits native switches, saves/reopens the form,
  corrects an authentication failure and retries, then switches Off/back while checking
  retained options. Initial test-driver issues were diagnosed from UI hierarchies:
  the native Passwords prompt interrupted input; a switch row contains a separate
  switch control; and the form preserves scroll position on reopening. Drivers now
  decline password saving, operate the actual switch and reveal offscreen buttons.
- Async XCTest teardown removes the exact fixture credentials, defaults and store;
  startup also removes UUID-validated stale fixtures from this QA app's own sandbox
  after an interrupted runner. No production credentials or user task store is read.
- The full setup/recovery assertions reached completion on iOS 27, but the following
  native contrast audit failed (`/tmp/momentum-sync-setup27-scroll.xcresult`). This is
  not a passing suite. A focused saved-connection audit is collecting the exact finding
  before any production styling change. Final results and remaining scope follow.
- Final iOS 27 functional setup acceptance passed in 73.746 seconds, including a
  custom folder, actual encrypted/compressed payload prefix, Off/back retention and
  a cold process relaunch. German AX5/dark saved-status audit also passed (33.373s).
  `/tmp/momentum-sync-setup27-final.xcresult` is nevertheless a failed suite: the
  normal-size saved-status text-clipping audit remains open as B-038. Vertical sizing,
  shared-label and typography experiments did not resolve it and were reverted.
- iOS 26.5 reproduced an additional anonymous contrast finding at normal size; its
  German AX5/dark saved-status audit passed. The original functional run failed in
  the driver: full-screen swipes skipped fields with the software keyboard open.
  The driver now scrolls in bounded steps above the keyboard and explicitly sets
  English/default text for normal runs. A second interruption was captured before
  cleanup: the native Save Password sheet appeared after the one-second probe.
  The test now waits up to five seconds and declines saving these disposable details.
- Screenshot inspection found B-039: after hiding scroll-edge effects, scrolled form
  text overlapped the navigation title. A matching opaque grouped navigation surface
  fixes the Sync screen when applied to the pushed content itself. Applying toolbar
  preferences outside NavigationStack had no effect and was reverted. The before/after
  images under `/tmp/momentum-sync-setup27-final-renders` and
  `/tmp/momentum-sync-nav-inner27-renders` were inspected. Both light/dark scrolled
  pixel checks passed on iOS 27 (`/tmp/momentum-sync-nav-pixels27.xcresult`, 34.880s).
  Other navigation screens still need their own scrolled-state assessment; empty-tab
  B-036 evidence does not cover that behavior. Native control styling stays intact.
- The ordinary view lane passed 25 tests in 2.250 seconds after extracting SettingsLabel
  (`views-1789663112368124000.xcresult`), including exact orange and the normal-white /
  Increase-Contrast-calculated foreground policy. The final canvas change also passed all 25 tests in 2.176 seconds
  (`views-1789663534907551000.xcresult`). Nine runner regressions and local doc-link /
  whitespace checks passed. No new package dependency, commits, pushes or releases.
- Both scrolled-navigation pixel checks also passed on iOS 26.5 (14.616s dark,
  14.107s light; `/tmp/momentum-sync-setup26-password.xcresult`). That bundle still
  failed the functional case: XCTest considered a field behind the navigation bar
  hittable. The driver now requires its center inside the visible form viewport before
  tapping. Reopening now waits for the navigation link to be hittable and confirms arrival
  before searching the form. Sync Now is revealed before querying it. Later iOS 27
  snapshots showed the earlier tap had left the app at the root while a system sheet
  finished dismissing; offscreen-cell recycling alone did not explain that failure. Final setup reruns follow below.
- Final iOS 26.5 functional setup/recovery/cold-relaunch acceptance passed in 102.008
  seconds (`/tmp/momentum-sync-setup26-viewport.xcresult`). This is a separate slower
  UI lane; the 25 native component tests remain around two seconds of test execution.
- Final iOS 27 functional setup/recovery/cold-relaunch acceptance passed in 105.352
  seconds (`/tmp/momentum-sync-setup27-prompt.xcresult`). The last driver correction
  waits for Not Now to become hittable, verifies dismissal, and waits for the root
  navigation link to become hittable before opening the form. Both platforms now
  have passing scoped functional setup and light/dark scrolled-navigation evidence.
- Corrected the stale black-ink wording in `ios/README.md`: default-orange filled
  actions use white text/symbols, with calculated ink only under Increase Contrast.
  The top-level README and F-019 already record this latest decision. No accent
  policy changes were made during the sync investigation.
- Fixtures were removed by native teardown and no QA simulator remained booted at
  handoff. Final whitespace/doc-link checks passed. Work remains local on the dirty
  iOS branch. Full sync/accessibility acceptance is not complete: B-038 findings,
  other scrolled navigation screens, hosted Nextcloud/TLS, LibreSync UI and physical
  VoiceOver/background/energy checks remain open. Next: resolve the scoped native
  audit findings and assess navigation readability before advancing LibreSync setup.

### 2026-09-17 — isolate native status semantics and scrolled navigation (B-038/B-039)

- **Previous goal turn: progress.** Native Nextcloud setup passed on both target OS
  versions; scoped Sync navigation pixels and fast native regressions passed. Continued
  work uses the same dirty `task/ios-app` checkout based on `7ec82e9`, local changes only.
- A four-row native Form reproduced B-038 without sync, scrolling or network state:
  ordinary Label and SettingsLabel triggered textClipped; plain Text and an HStack
  with a decorative symbol plus wrapping Text did not. This contradicts the earlier
  theory that adding vertical size to Label would fix it. Evidence:
  `/tmp/momentum-label-probe27.xcresult`, two identified findings on the Label rows.
- The explicit status stack still failed the complete-form EN/default audit; German
  AX5/dark passed (`/tmp/momentum-saved-status27.xcresult`). Baseline alignment and a
  conditional Section/disabled parent in reduced examples did not reproduce that
  clipping. A bottom-row example instead identified contrast on a partially occluded
  row behind navigation (`/tmp/momentum-label-bottom-probe27.xcresult`); the conditional
  example identified disabled Save contrast (`/tmp/momentum-label-context-probe27.xcresult`).
  These narrow the investigation without establishing the full-form cause. Reverted
  the speculative production stack and removed temporary probe screens/tests. B-038
  remains open, with the real form audit retained and no findings waived.
- B-039 reproduced in the actual app on Today and Accent Color on iOS 26.5:
  `/tmp/momentum-navigation26-red.xcresult`, eight pixel assertions failed and retained
  screenshots visibly showed content through navigation. Applied the existing shared
  opaque canvas modifier to task/list navigation and Settings, Appearance, Accent Color,
  Text & Feedback, Task Lists, Notifications, Backups and About content. No business
  logic or accent policy changed. Cross-version native verification follows.
- The initial iOS 26.5 four-test background/navigation run passed in 117.674 seconds,
  but screenshot review caught missing large titles under forced-visible navigation
  backgrounds. Changed visibility to automatic while retaining the opaque matching
  color when scrolled. Added actual title-ink pixel assertions, since accessibility
  existence alone missed this regression. Final cross-version results follow.
- Default-orange policy remains exact #FF6600 with white normal text/symbols and
  calculated Increase Contrast ink. Nine native rendering tests passed on iOS 26.5
  in 0.461 seconds (`/tmp/momentum-accent-final26.xcresult`), including both appearances
  and both contrast traits plus actual rendered Add/commit button ink.
- Final iOS 27 BackgroundTests passed: four tests, 118.082 seconds,
  `/tmp/momentum-navigation27-fixed.xcresult`. This covers all four empty-tab canvases
  and visible title ink, plus eight scrolled task/Settings destinations in both
  appearances. Inspected retained Today screenshots at rest and scrolled; the title
  remains visible and the navigation area matches the body canvas. Results apply to
  the dirty local worktree; full accessibility and physical-device acceptance remain
  separate.
- The same nine native rendering tests passed on iOS 27 in 0.524 seconds
  (`/tmp/momentum-accent-final27.xcresult`). This verifies native dynamic trait
  resolution keeps orange unchanged and selects white normally/calculated black
  under Increase Contrast, together with rendered button-symbol checks.
- The production Sync form's light/dark scrolled-navigation tests also pass with
  automatic background visibility on iOS 27 (two tests, 38.881 seconds,
  `/tmp/momentum-sync-navigation-final27.xcresult`). This verifies navigation pixels,
  not resolution of the separate B-038 audit findings.
- Final iOS 26.5 BackgroundTests passed: four tests, 117.564 seconds,
  `/tmp/momentum-navigation26-titles.xcresult`. The added title-ink checks pass in
  light/dark mode along with the continuous canvas and eight scrolled destinations.
  Inspected the retained light Today screenshot: expanded title is restored and the
  background remains uniform. The initial forced-visible experiment is superseded.
- Final iOS 26.5 Sync navigation checks passed in both appearances
  (`/tmp/momentum-sync-navigation-final26.xcresult`). All 30 focused checks passed
  across the two OS versions: 18 native component checks and 12 app/Sync navigation
  checks. Documentation links and whitespace checks passed. Temporary diagnostic
  probe UI is removed; isolated Sync fixtures are cleaned by test teardown.
- Remaining parity work is unchanged: B-038 native audit findings, spoken VoiceOver,
  sheets/large-text navigation coverage, hosted Nextcloud/TLS, LibreSync UI and
  physical-device lifecycle/energy acceptance. No commit, push or phone installation
  was performed for this change.


### 2026-09-17 — LibreSync mobile lifecycle and credential boundary (F-026/B-040)

- **Previous goal turn: progress.** Color policy and continuous navigation backgrounds
  have passing scoped iOS 26.5/27 evidence. Rechecked the dirty `task/ios-app` worktree:
  the mobile provider selector still lacks LibreSync, so parity is not complete.
- Source review found raw `mdns-sd` multicast in the existing LibreSync dependency.
  Apple TN3179 requires the multicast entitlement for raw multicast on iOS; native
  Bonjour with the declared service is the intended platform integration to assess.
  Do not mistake simulator success for physical discovery/permission acceptance.
- B-040 fixes a reproduced shared Apple credential-read hazard before mobile startup.
  Eight focused tests pass after a failing regression; live personal credentials are
  never used. No saved-key format, account name, core rule or provider default changed.
- Added a serial mobile runtime lifecycle, gated by foreground, provider selection,
  restore suspension and isolated-mode permission. Seven deterministic lifecycle tests
  pass after the missing-start regression failed. Cases cover startup racing background,
  resume racing stop, retry without spinning, stale callbacks, restore and preview/test
  isolation. App wiring and native Bonjour discovery remain outstanding.

### 2026-09-17 — nearby receive delivery and idle persistence (F-026/B-041/B-042)

- **Previous goal turn: no progress.** It confirmed the already implemented orange
  contrast policy. Revalidated the actual worktree and resumed the incomplete
  LibreSync adapter acceptance; no goal scope was reduced.
- The earlier inbound regression terminated successfully (one test, 0.14 seconds).
  The unrelated native Keychain test run had completed its four tests but stalled
  collecting diagnostics; stopped only that identified test/diagnostic process.
  Its malformed-data fixture was corrected from valid base64 text to invalid UTF-8.
- B-041 fixes listener event-sink ordering and immediate startup event delivery.
  B-042 fixes the reproduced infinite state-save completion loop. Both had failing
  behavioral regressions before implementation. Shared nearby tests now pass: 15
  tests, 0.88 seconds, `/tmp/momentum-nearby-core-green.log`.
- Broader Rust consumers, regenerated Apple libraries, two-way mobile exchange and
  corrected native Keychain fixtures are the next verification steps. LibreSync is
  not yet exposed in mobile Settings; native Bonjour and provider lifecycle wiring
  remain required, along with physical-device and accessibility acceptance.

- Rebuilt all Apple target libraries with the shared generator (20.119 seconds).
  Both Rust configurations pass: 253 tests each, no failures, one intentional manual
  latency benchmark ignored. Logs: `/tmp/momentum-nearby-workspace-default.log` and
  `/tmp/momentum-nearby-workspace-all.log`. Linux GTK runtime remains untested on Mac.
- The previously failing real mobile adapter exchange now passes in 4.623 seconds
  (`/tmp/momentum-nearby-mobile-fixed.log`). The ordinary fast mobile suite passes
  111 tests in 2.924 seconds; the Mac suite including real peer exchange passes
  136 tests in 20.961 seconds (`/tmp/momentum-nearby-kit-fixed.log`).
- Corrected native Keychain fixtures pass on both versions: four boundary tests on
  iOS 27 in 0.569 seconds (`views-1789666829715957000.xcresult`) and iOS 26.5 in
  0.103 seconds (`views-1789666879755330000.xcresult`), under
  `ios/DerivedData/TestReports/`. The earlier malformed-ASCII fixture failure is
  superseded; no production behavior was changed to satisfy that invalid fixture.
- Added native NearbyTransportTests using unique disposable Keychain services and
  real iOS sockets. Initial two-way exchange, callbacks, stop, restart and unlink
  pass on both versions. Review strengthened restart acceptance to check the same
  certificate and another trusted exchange after restart; final results follow.

### 2026-09-17 — user narrows current mobile sync delivery to Nextcloud

- User explicitly requested: “skip libresync for now, just implement nextcloud sync”.
  Stop further LibreSync implementation/verification and leave it unexposed. Its
  incomplete native discovery, UI and physical-device acceptance are deferred by the
  user, not complete or inapplicable. Existing local work is preserved.
- Before the request, the strengthened native iOS 27 nearby test passed (7.597
  seconds, `transport-1789667014201715000.xcresult`); iOS 26.5 passed the earlier
  narrower version. The strengthened iOS 26.5 rerun is now deferred. Thread Performance
  Checker reported a startup priority inversion between the Swift actor and Rust
  listener worker; no executor change was applied. Track this before resuming LibreSync.
- Current app provider choices are already Off and Nextcloud. Active work returns to
  Nextcloud setup and recovery, B-038 accessibility, and remaining integration checks.
  The next concrete check is the current production-source Sync form's native audit.

- Nextcloud baseline audit reproduced B-038 on current iOS 27: contrast plus the
  saved-row clipping finding (`/tmp/momentum-nextcloud-audit-baseline27.xcresult`).
  Replacing Label with plain wrapping Text in the same row did not help
  (`/tmp/momentum-nextcloud-status-text27.xcresult`).
- Reworked the saved state: a wrapping confirmation in the Options footer, an
  actionable Save button only when the draft changes, and accessibility focus after
  successful save. The isolated normal-size audit now reports only one anonymous
  clipping issue (`/tmp/momentum-nextcloud-status-footer27.xcresult`); contrast is
  clear in that run. Retained screenshots show the full confirmation. B-038 remains
  open; no findings were waived. The audit helper now returns false for findings so
  Xcode retains native diagnostic attachments rather than only a custom assertion.
- Fresh real Nextcloud iOS 26.5 transport checks pass: four tests, 2.613 seconds,
  `transport-1789667438525553000.xcresult` in `ios/DerivedData/TestReports`.
  Full production-source setup/recovery and cross-version UI verification are running.
- A local real Nextcloud server is not available: Docker CLI exists but its default
  local socket has no running daemon. No daemon/service was started and no real
  account credentials were requested or changed. Hosted/TLS verification remains open.

- Revised production-form iOS 27 checks: setup/save/reopen/authentication correction,
  Off/Nextcloud retention and cold relaunch pass (104.878 seconds); light/dark scrolled
  navigation and German AX5 dark audit also pass. The normal-size audit still fails
  with one anonymous text-clipping finding. Bundle:
  `/tmp/momentum-nextcloud-focused27.xcresult` (four of five tests pass).
- iOS 26.5 normal-size audit identifies a `Connection` section header at y=64.7–105,
  entirely behind the navigation surface, plus anonymous contrast/clipping findings.
  This is evidence of auditing occluded content, not proof all findings are false.
  German AX5 dark and both navigation checks pass; interactive setup is still running.

- Final iOS 26.5 functional setup/recovery flow passes (108.508 seconds), as do light/
  dark navigation and German AX5 dark checks. Its normal-size audit remains red on
  the findings above. `/tmp/momentum-nextcloud-focused26.xcresult`: four of five pass.
- Fresh Nextcloud iOS 27 real transport checks also pass: four tests, 2.195 seconds,
  `transport-1789667690367789000.xcresult` under `ios/DerivedData/TestReports`.
  Both versions verify actual Rust HTTP exchanges, encryption/compression, offline
  convergence, ETag retry, failed credentials/correction and cancellation without
  losing local edits. These use isolated loopback WebDAV, not a hosted Nextcloud server.
- The revised audit helper preserves an actual Xcode Text clipped failure and native
  screenshot (`/tmp/momentum-nextcloud-native-audit27.xcresult`). Inspection shows the
  saved footer at the bottom edge after the audit's font/layout change; the returned
  issue has no element identity, so its exact cause is still unproven. Do not apply
  another color/layout workaround or waive the finding from this evidence alone.
- All changes remain local. LibreSync stays explicitly deferred; next actions are
  resolving B-038 with the retained native diagnostics and completing hosted/TLS,
  physical-device and spoken accessibility acceptance for Nextcloud.

- Production Momentum simulator app build succeeds with the final Nextcloud UI
  (`/tmp/momentum-nextcloud-app-build.log`, Xcode 27, generic iOS Simulator; both
  arm64/x86_64 compilation). No physical phone installation was performed. The app
  build regenerates the shared XCFramework; the test runner may require `prepare`
  to refresh its artifact stamp before the next fast/native lane.
- Reviewed final source, native screenshots and diagnostics. Documentation links and
  `git diff --check` pass. Normal-size audit failures are retained as B-038; no claim
  of full accessibility, hosted-server or overall iOS parity acceptance is made.


### 2026-09-17 — Nextcloud scope and typography isolation (F-025/B-038)

- **Previous goal turn: progress.** Nextcloud production-source setup/recovery and
  real loopback HTTP acceptance passed on both target OS versions; the normal-size
  native accessibility finding remains open. LibreSync is explicitly deferred.
- Tested the hypothesis that manually scaled root fonts cause B-038. Native semantic
  body/caption defaults still fail the saved-screen audit on iOS 27 (22.126 seconds,
  `/tmp/momentum-nextcloud-semantic-font27b.xcresult`), identifying the `Options`
  header at y=555 with height 40.3. The first experimental build failed because an
  explicit return was needed outside the ViewBuilder; the corrected build ran the
  actual audit. The experiment was reverted.
- Removing the QA host's app-font override also leaves the `Options` finding
  (`/tmp/momentum-nextcloud-native-font27.xcresult`, terminal TEST FAILED). Restored
  the host's production typography. No speculative font changes or waived findings
  remain. The font override is not the sole cause; the audit's exact cause is still
  unconfirmed. The original production source remains the last built app version.
- Added iOS Nextcloud setup, saved-versus-synced semantics, manual/automatic exchange,
  offline behavior and recovery instructions. Corrected the remaining README wording
  that described iOS LibreSync pairing as in progress. Split the parity checklist's
  verified functional controls from outstanding accessibility/hosted acceptance.
- Local dirty `task/ios-app` only; no phone installation or external service change.
  Next work: investigate the identified native Form header independently, complete
  remaining native Nextcloud status/provider-transition checks, and retain hosted/TLS,
  physical-device and spoken accessibility as explicit outstanding acceptance.


### 2026-09-17 — persistent Nextcloud task-list status (F-028/B-043)

- **Previous goal turn: progress.** Two isolated typography experiments showed that
  the app's font override does not solely explain B-038. Both were reverted; updated
  Nextcloud guidance and split functional acceptance from outstanding native audits.
- Current source confirms F-028's task-list gap: only failures expose the old status
  link. Extracted that existing entry, reproduced the missing healthy state with a
  native test, then extended it to provider/progress/last success/pending changes.
  The entry opens the existing Sync settings. Sync Off hides it; timestamps do not
  run a timer. At standard text sizes it sits above either list state; at accessibility
  sizes it scrolls as the first list row so it cannot consume the entire task viewport.
- Red: `/tmp/momentum-status-link-red27.xcresult`, missing status entry. Green:
  `/tmp/momentum-status-link-green27.xcresult`, 22.134 seconds, real loopback exchange,
  return from details, updated last success and removal after Off. The separate QA
  host uses the production component; this does not yet prove full production-list
  interaction or spoken VoiceOver.
- Added deterministic held-HTTP and authentication-failure fixture controls, plus
  native progress/retry checks and compact English/German AX5 rendering in light/dark.
  Cross-version results follow below. Changes are local to dirty `task/ios-app`;
  LibreSync remains deferred, and no shared Rust/task behavior changed.

- Extended iOS 27 status flows pass (two tests, 49.701 seconds,
  `/tmp/momentum-status-flows27.xcresult`): a held real HTTP request exposes progress,
  release publishes success, rejection persists a recovery link, and retry clears it.
- The first iOS 26.5 AX5 render revealed narrow metadata columns and truncated German
  text; the initial render suite failed. Accessibility sizes now stack metadata at
  full width and omit decorative side symbols. The row scrolls with production tasks
  at those sizes. Revised iOS 27 six-test component/lifecycle suite passes in 0.673
  seconds (`/tmp/momentum-status-render27.xcresult`); exported English/dark and German
  AX5/light images were inspected and show the full content. Final iOS 26.5 checks
  and a leading-alignment polish rerun follow.

- iOS 26.5 native status flows also pass: two tests, 46.827 seconds,
  `/tmp/momentum-status-flows26.xcresult`. Both target runtimes now verify the shared
  component's idle/in-flight/success/failure/retry/Off navigation against real Rust
  HTTP exchanges. This remains isolated component-host evidence, not full production
  task-list or physical-device acceptance.

- Final component/integration checks pass on iOS 26.5 (six tests, 0.637 seconds,
  `/tmp/momentum-status-render26-final.xcresult`) and iOS 27 (six tests, 0.641 seconds,
  `/tmp/momentum-status-render27-final.xcresult`). Final light/dark German AX5 images
  from iOS 26.5 were inspected: labels, timestamp and pending count are complete.
  The status row is excluded from task selection. Both production task-screen branches
  compile in the native view-test target; full-screen scrolling/touch/VoiceOver with
  a configured production model remains an explicit next acceptance check.
- Documentation links, changed Swift whitespace and `git diff --check` pass. No
  changes to Rust, app dependencies, localization keys, accent policy or real accounts.
  No physical install this turn. F-028/B-043 remain scoped implementation plus native
  component evidence; B-038 and broader iOS parity are not marked complete.


### 2026-09-17 — full task-screen Nextcloud acceptance (F-028/B-039/B-043)

- **Previous goal turn: progress.** Added the persistent status component and
  cross-version HTTP/UI checks; AX5 renders caught and fixed narrow metadata columns.
  Full production-list interaction was explicitly left open.
- Extended the separate QA app to compile the production RootView/tabs, TaskScreen,
  quick-add sheet and model. A SwiftUI environment dependency supplies the isolated
  Nextcloud state; the production entry supplies its model's own state. The QA model
  still uses disposable storage and disables its production credentials/transport;
  only the fixture's exact loopback URL is admitted by the separate sync factory.
- Initial iOS 27 full-screen checks pass (two tests, 45.082 seconds,
  `/tmp/momentum-status-full27b.xcresult`): empty-to-populated Today through quick add,
  real exchange/details return, and largest-text scrolling into task editing and
  sync details. This is stronger interaction evidence than the previous component host.
- Screenshot inspection caught a gap in those functional assertions: the standard-size
  top status inset leaves Today title's region blank, while the scrolling AX5 row
  preserves it. Added a title-pixel regression before changing placement. Upcoming/
  Search coverage was also added; final cross-version results follow.
- Production simulator app build passed before the placement fix
  (`/tmp/momentum-status-production-build.log`). No app installation or real account
  changes were performed; LibreSync stays deferred.

- The new pixel assertion reproduces the title regression: zero ink pixels in the
  Today title bounds (`/tmp/momentum-status-title-red27.xcresult`). Moved status into
  the native task list at every text size and removed the extra custom disclosure
  chevron, relying on List/NavigationLink's native accessory. No toolbar workaround
  was added. Cross-version title/tab/search/large-text checks are running.

- Final full-screen tests pass on both runtimes: two tests each, iOS 27 54.145s
  (`/tmp/momentum-status-full27-final.xcresult`) and iOS 26.5 51.542s
  (`/tmp/momentum-status-full26-final.xcresult`). Coverage includes an empty Today,
  quick-add to a populated list, real sync/details return, painted Today title,
  Upcoming, keyboard-open Search results, and AX5 dark scrolling into task editing
  and Sync Now. Final iOS 27 Today and iOS 26.5 Search screenshots were inspected:
  the large title and status are visible with the native list disclosure, and the
  search keyboard does not block the status link.
- Final production simulator app builds successfully with this placement and
  environment wiring (`/tmp/momentum-status-production-final-build.log`). Native
  isolation/render/store-recovery regression checks are running after the shared
  framework rebuild. The builder invalidates the fast runner's preparation stamp;
  run `python3 ios/scripts/test.py prepare` before a future runner-based lane.

- iOS 27 native isolation/adaptive-render/store-recovery suite passes: eight tests,
  0.845 seconds (`/tmp/momentum-status-boundary27-final.xcresult`). F-028 is now Verified
  for the explicitly recorded Nextcloud visual/native-interaction scope, with the
  corresponding checklist case checked. This does not change F-020's spoken
  accessibility or F-025's hosted/TLS/physical acceptance status, and does not include
  deferred LibreSync. Full iOS parity remains incomplete.

- Matching iOS 26.5 native regression suite also passes: eight tests, 0.797 seconds,
  `/tmp/momentum-status-boundary26-final.xcresult`, using the same built test product.
  Documentation links, changed Swift whitespace and `git diff --check` pass. All
  processes are terminal; no simulator or phone installation of the shipping app
  was performed. Next independent parity work includes remaining keyboard/OS
  integration checks; B-038, spoken accessibility, hosted/TLS and physical-device
  acceptance remain explicitly open. The production build also reports an existing
  iOS 26 Text-concatenation deprecation in task subtitles; it is not a build failure.

### 2026-09-17 — F-023 keyboard foundation, B-044 deferred task routing

- Scope: dirty `task/ios-app` worktree based on `7ec82e9`, local changes only. Native
  scene commands reuse MomentumKit's bindings for capture, tab navigation, search,
  Nextcloud sync and help. Settings exposes the Command/Control/Option preference
  with English/German help. This is an **In progress** implementation, not verified
  keyboard parity: native navigation/Search dispatch remains unresolved, and task
  selection/action shortcuts are still outstanding.
- B-044: task-opening requests now remain queued while Keyboard Help is visible.
  RootView retries the existing route on sheet dismissal. The native model test
  first failed (`/tmp/momentum-keyboard-help-red26.xcresult`), then passed after the
  gate change. Eleven automation, Nextcloud isolation/rendering and store-recovery
  checks pass on both versions: iOS 27 1.055s
  (`/tmp/momentum-keyboard-boundary27-final.xcresult`) and iOS 26.5 1.040s
  (`/tmp/momentum-keyboard-boundary26-final.xcresult`).
- Final iOS 27 native acceptance has **four passes and two failures**
  (`/tmp/momentum-keyboard-and-sync-final27.xcresult`): both full Nextcloud task-screen
  flows pass (31.609s and 22.169s), Help dismissal opens the queued task (15.983s),
  and Command-N/Command-Return captures exactly one task (17.415s). Navigation fails
  at Command-2; Search fails at Command-F. Consequently modifier/help/sync shortcuts
  later in the navigation scenario are not verified. These failures remain in the
  native regression suite and are not waived or excluded from parity acceptance.
- Diagnostic limits: XCTest's Return constant/carriage-return event did not reach
  UIKit in the recorded SDK 27 trace; the line-feed representation delivered HID
  key 40 and exercised the existing save shortcut. No TextField submit override is
  retained. Minimal direct command controls can switch tabs, while the full command
  implementation still fails native dispatch. Menu placement, enabled-state and
  observation experiments did not establish a fix. No SDK-defect claim is made.
  Temporary event swizzling, logging, ungated controls and probe tests were removed.
- The QA scene owns its isolated model from initialization and admits only its exact
  loopback sync endpoint. A QA-only queued-route fixture exercises the actual help
  sheet dismissal and editor presentation; production startup accepts none of these
  fixture flags. The shipping simulator app builds successfully
  (`/tmp/momentum-keyboard-production-final-build.log`); all 307 active German catalog
  entries validate. The core rebuild invalidates the fast runner's preparation stamp,
  so run `python3 ios/scripts/test.py prepare` before its next invocation.
- Nextcloud remains the only active mobile provider (alongside Off); LibreSync is
  still deferred. No physical or shipping-app installation, real credentials,
  hosted/TLS validation, commits or publishing occurred. F-023, B-038, spoken
  accessibility and remaining OS/device acceptance keep the overall parity goal open.
- Final iOS 26.5 acceptance has the same **four passes and two failures**
  (`/tmp/momentum-keyboard-and-sync-final26.xcresult`): full task-screen sync flows
  pass in 29.258s/19.539s, queued Help dismissal in 14.130s and capture/save in
  14.247s. Navigation and Search remain on Today after their shortcuts. The iOS 27
  failure screenshots/hierarchies were inspected and confirm the same state, not
  merely an ambiguous test selector. B-044's queued-sheet routing scope is verified
  on both versions; F-023 remains In progress. All build/test processes are terminal.
- Next concrete work: isolate why the complete scene-command dispatch fails while
  direct native control commands work; preserve the existing failing native tests.
  Then implement/test task-specific selection/actions. Do not treat the passing
  focused suite or app build as a full native acceptance pass.


### 2026-09-17 — F-023/B-045 native keyboard command placement

- Scope: dirty `task/ios-app` based on `7ec82e9`, local changes only. iOS remains
  Off/Nextcloud; LibreSync stays deferred. No new dependencies or core changes.
- A shipping-entry regression reproduced the navigation failure using only the
  app's isolated UI-test store (`/tmp/momentum-keyboard-shipping-probe27.xcresult`).
  Key-event tracing then confirmed repeated Command-2/F events reached UIKit while
  their full-scene handlers were never called; enabled state and modifiers were
  correct. Reduced scenes narrowed the fault to command placement.
- B-045 moves Settings into the standard `.appSettings` replacement group and Search
  into Navigate. Shared bindings, availability gates, original labels and ForEach
  remain. The native navigation sequence additionally checks the Search tab alias.
  The instrumented candidate passed navigation, modifier changes, fixed Settings,
  Help and a real loopback Nextcloud sync in 25.863s
  (`/tmp/momentum-keyboard-settings-group27.xcresult`). Clean verification follows;
  diagnostic passes are not the final acceptance result.
- All diagnostic swizzling, logging, temporary UI and probe methods were removed.
  The shipping-entry regression is retained. German catalog validation passes
  (307/307); no labels or translations changed. Full task-specific keyboard parity,
  B-038, hosted Nextcloud/TLS, spoken accessibility and OS/device acceptance remain
  open independently of this scoped fix.

- Clean iOS 27 acceptance has five passes and one failure
  (`/tmp/momentum-keyboard-final27.xcresult`): full task-screen sync 31.534s, AX5 dark
  task-screen sync 20.729s, queued Help routing 15.676s, navigation/modifier/Settings/
  Nextcloud 26.789s, and capture/save 16.284s. The separate cold-launch Command-F
  Search test still fails. No retry, warm-up key, sleep or availability bypass was
  added to make it pass. B-045 remains In progress for that remaining scope.
- Eager QA scene-command registration also fails the cold-launch Search test
  (`/tmp/momentum-keyboard-eager-scene27.xcresult`, 17.382s). It was reverted. A second
  shipping-entry regression now checks Command-F as the first shortcut after its
  isolated empty store finishes loading; this distinguishes startup from navigation
  followed by Search. Cross-version and shipping-entry results follow below.

- Matching clean iOS 26.5 QA acceptance passes **all six** tests
  (`/tmp/momentum-keyboard-final26.xcresult`): task-screen sync 29.642s, AX5 dark sync
  19.714s, queued Help routing 13.987s, navigation/modifier/Settings/Nextcloud 25.158s,
  capture/save 13.489s and cold-launch Search 12.709s. The remaining recorded
  cold-launch Search failure is on iOS 27; do not generalize it to iOS 26.5.
- The shipping simulator app and both keyboard regressions build successfully
  (`/tmp/momentum-keyboard-shipping-final-build.log`). Its core rebuild again
  invalidates the fast runner's preparation stamp. Shipping-entry runtime results
  follow below; this build alone is not runtime acceptance.

- Shipping-entry iOS 27 acceptance fails both Search scenarios
  (`/tmp/momentum-keyboard-shipping-final27.xcresult`): navigation followed by Search
  13.296s (Command-2 selection passes, then Search field is absent) and cold-launch
  Search 11.544s. This broadens the remaining Search finding beyond the first
  shortcut after launch; the QA warmed sequence is not proof of shipping Search.
  Both failing regressions remain enabled. This is not yet attributed to XCTest,
  input delivery or a framework defect. No physical install or live server was used.

- The same freshly built shipping test product passes both scenarios on iOS 26.5
  (`/tmp/momentum-keyboard-shipping-final26.xcresult`): navigation followed by Search
  9.751s and cold-launch Search 7.527s. Across this handoff's two lanes, iOS 26.5 has
  eight passes; iOS 27 has five passes and three Search failures. B-045/F-023 remain
  In progress. The native app build, German catalog, changed Swift whitespace,
  documentation links and `git diff --check` pass.
- All build/test processes are terminal. Both shipping simulator runs used only
  `--uitesting --reset-test-store`; no personal tasks/credentials or physical phone
  were used. All changes remain local and uncommitted. Next concrete work is to
  isolate iOS 27 Search command resolution using the retained shipping regressions,
  including native Find-command precedence as an unproven hypothesis. Do not repeat
  the disproved modifier, availability, identity, title or eager-QA-registration
  changes, and do not waive the failures with key retries or warm-up events.

### 2026-09-17 — F-023/B-045 physical iOS 27 keyboard evidence

- Scope: unchanged production keyboard source in dirty `task/ios-app`, base
  `7ec82e9`; local changes only, Off/Nextcloud, LibreSync deferred. This follow-up
  changes evidence and documentation; no experimental command implementation remains.
- The iOS 27 simulator failure screenshot and hierarchy show Today after Command-F,
  so the failure is not just an unfocused or hidden Search field. QA-only native Find
  configuration, held-modifier delivery, distinct Search labels and a before-editing
  insertion point all failed to resolve it. A temporary UIKit hosting responder
  became first responder and advertised a priority Command-F, but its action was not
  queried or executed. All probes/logging were removed; no UIKit adapter was shipped.
- A second dedicated iOS 27 simulator ran the unchanged shipping test product:
  navigation then Search passed in 10.832s, cold Search failed in 11.990s
  (`/tmp/momentum-search-second-device27.xcresult`). This does not establish simulator
  corruption or a framework defect. Direct host UI inspection timed out and provided
  no manual keyboard evidence. Both shipping regressions remain enabled.
- With the user's phone unlocked on request, the separately signed QA app ran on a
  physical iPhone 17 Pro Max, iOS 27.0 (24A437). Its own bundle, UUID data/preferences,
  disposable Keychain records and loopback WebDAV fixture isolate it from the normal
  app and personal data. Tests clear their fixtures at teardown; no personal settings
  or normal app data were reset.
- All four device keyboard scenarios passed without retries or warm-up keys:
  cold Search/query/result 13.662s (`/tmp/momentum-search-physical27.xcresult`);
  queued task after Help dismissal 13.152s; tab navigation, modifier change, fixed
  Command-Comma and a real Nextcloud protocol exchange 24.408s; new-task capture/save
  13.100s (last three: `/tmp/momentum-keyboard-physical27-remaining.xcresult`). Both
  Xcode processes exited zero. The exported device screenshot was inspected and
  shows Today with the seeded task, a successful-sync timestamp, a continuous
  background and white Add task text on orange.
- These are automated device events using production views in the QA entry point.
  They do not verify a manually operated external keyboard, the shipping device entry
  point, hosted Nextcloud/TLS or background sync. The iOS 27 simulator Search failure
  remains open; B-045/F-023 stay In progress. No SDK defect is asserted from the
  device/simulator difference. Task-specific selection/action shortcuts are still
  missing and are the next independent implementation step while preserving these
  Search regressions. Full parity, B-038, spoken accessibility and remaining OS
  integration/lifecycle acceptance are not complete.
- FEATURES, BUGS, the parity checklist and keyboard testing guide now distinguish
  device passes from simulator failures. All native sessions are terminal. Changes
  remain local and uncommitted; no dependency or shared-core change in this follow-up.

### 2026-09-17 — F-023 task selection commands and B-046 input acceptance

- Scope: dirty `task/ios-app`, base `7ec82e9`, local iOS presentation changes only.
  Off/Nextcloud remains the mobile scope; LibreSync is deferred. The preceding goal
  turn made progress through four physical-device keyboard passes. This turn adds
  task keyboard behavior rather than treating that global-command subset as parity.
- The new native regression first failed at Command-A before implementation
  (`/tmp/momentum-task-keyboard-red26.xcresult`). The visible task list now publishes
  its eligible task IDs, selection and available actions through an equatable focused
  scene value. Equality includes the action inputs, avoiding the render loop observed
  with a fresh closure value on every update. Snapshot refresh removes ineligible
  selections; archived tasks remain excluded. Task operations reuse the existing
  Rust-backed model methods and desktop shortcut definitions.
- Select All uses a small list-scoped UIKit responder implementing the standard
  `selectAll(_:)` action; the native Edit menu remains intact. An earlier priority
  key-command prototype was simplified to this standard action. It yields when the list disappears, Search has text focus, or an editor,
  alert or other task-blocking sheet opens. Deselect, complete/reopen, Undo and Delete
  use stable SwiftUI scene commands and state-driven enablement. Undo replaces its
  native group: inserting another Command-Z alongside the system entry produced a
  confirmed UIKit duplicate-key exception during development. Dynamic task-menu
  insertion also failed dispatch; it is not retained. Help lists the task bindings
  using the same adaptive row layout as global shortcuts; German Deselect All reuses
  the macOS translation (308/308 catalog entries validate).
- The native Search text check covers Select All, replacement typing and text Undo,
  then confirms the task survived. The modifier check covers fixed Command-A,
  Control task actions, an empty foreground tab leaving another tab's selection
  untouched, and cancelling a text-edited task without a background completion.
  These pass in the focused iOS 26.5 run; final cross-runtime results follow below.
- Task Delete remains unresolved (B-046). QA-only tracing at UIApplication's public
  event boundary saw Command modifier events but no Delete press in the failing
  named-key run. Responder-level observations alone had been insufficient to claim
  that boundary. ASCII backspace/DEL probes and a priority Delete adapter did not
  resolve the failure. All tracing, swizzling and the unsuccessful Delete adapter
  were removed; the test again uses XCTest's standard named Delete key. Selection
  and deletion now have separate tests so the latter cannot conceal other outcomes.
- `/tmp/momentum-task-keyboard-responder26.xcresult` records the first passing
  Search text-editing (19.861s) and modifier/tab/editor isolation (33.838s) scenarios,
  while its combined selection/delete scenario fails at Delete. An editor frame
  warning appears during presentation; the same warning is present in the earlier
  physical Help-routing log, before this implementation. Do not claim that warning
  or complete editor layout/accessibility acceptance is resolved by these assertions.
- No native key retries, warm-up events, sleeps, availability bypass or user-data
  fixture was introduced. Remaining task shortcuts include open, duplicate, copy,
  planning, project moves, recurrence, reorder and new project. Bulk/family selection,
  manual external keyboards, physical acceptance of these new task commands,
  performance profiling and full parity remain open independently of passing cases.

- Final standard-action production source, iOS 26.5:
  `/tmp/momentum-task-keys-native-edit26.xcresult` has five passes and one failure.
  Navigation/modifier/Settings/sync (25.976s), cold Search (12.986s), native Search
  text selection/Undo (19.377s), modifier/tab/editor isolation (34.334s), and task
  selection/complete/reopen/two Undo operations/deselect (19.019s) pass. Delete
  fails after successful selection (15.366s); B-046 remains open.
- The same production source on iOS 27 has three passes and three failures in
  `/tmp/momentum-task-keys-native-edit27.xcresult`: navigation (27.441s), Search text
  editing (24.024s), and modifier/tab/editor isolation (35.093s) pass. Cold Search
  fails (16.896s); both task cases fail at initial Select All (16.850s and 16.997s),
  before Delete can run. These are not six passing cases. An earlier eight-case
  suite likewise had five passes/three failures; an incorrect interim six-pass
  report was explicitly corrected in the conversation.
- Adding visible seeded-row readiness before the first key did not fix cold iOS 27
  selection: both cases still fail at Actions (1) in
  `/tmp/momentum-task-keys-ready27.xcresult` (18.353s, 18.189s). Public lifecycle
  tracing subsequently confirmed that the list successfully owns first responder
  in a key window and advertises Select All as enabled, but receives no invocation
  (`/tmp/momentum-task-selection-lifecycle27.xcresult`, 18.510s failure). The
  first-responder-acquisition hypothesis is unsupported; no speculative lifecycle
  retry was added. B-047 records this separate unresolved cold-start acceptance.
  Diagnostics are removed from retained production and QA source.

- New physical iOS 27 task-command acceptance on the user-authorized unlocked phone:
  `/tmp/momentum-task-keys-physical27.xcresult` has two passes/two failures, exit 65.
  Search text selection/Undo (21.982s) and modifier/tab/editor isolation (32.741s)
  pass. Both task cases fail at their initial Command-A (14.832s, 14.974s); neither
  reaches Delete. B-047 therefore affects device automation as well as the simulator.
  B-046 still has no iOS 27 Delete result. Separate QA identity, UUID stores/defaults
  and disposable credentials protect the normal app; teardown cleared fixtures.
  This does not verify a manually operated external keyboard or shipping entry point.

- Exported physical screenshots were visually reviewed. Today retains a continuous
  canvas behind navigation/content, white Add task text on orange, an intact task
  row and pending-sync status. This screenshot review does not close the keyboard
  failures or broader accessibility acceptance.

- A bounded native-menu rebuild experiment after list focus acquisition did not
  resolve either cold Search (17.516s) or cold Select All (18.120s), exit 65
  (`/tmp/momentum-task-menu-refresh27.xcresult`). The experiment used Apple's
  public `UIMenuSystem.setNeedsRebuild()` API and was removed after the failed
  result. Retained source has no menu rebuild workaround, polling or input retries.

- Retained source builds successfully in the normal Momentum simulator app target
  with Xcode 27 (`/tmp/momentum-task-commands-shipping-build.log`, exit 0). The normal
  prebuild regenerated Apple Rust slices through existing tooling; no binding was
  hand-edited. This is a build result, not shipping-entry keyboard acceptance.
  German localization validates at 308/308; changed-Swift whitespace, local
  documentation links and `git diff --check` pass. All native sessions are terminal.
  Changes remain local/uncommitted. F-023, B-045–B-047 and full iOS parity remain
  In progress; next work should preserve these failing regressions while completing
  the remaining task commands and resolving input dispatch.


### 2026-09-17 — F-023 open, duplicate, copy title and archive shortcuts

- Previous goal turn: progress, with new physical-device evidence for B-047,
  successful normal-app build, and current regression records. This follow-up
  continues the missing keyboard capabilities instead of repeating the same
  unsuccessful cold-input experiments. Dirty `task/ios-app`, base `7ec82e9`, local
  iOS-only changes; shared Rust and desktop behavior remain unchanged.
- Two new native tests first failed at the missing Duplicate and Open bindings
  (`/tmp/momentum-task-more-keys-red26.xcresult`, 19.277s and 19.272s). They select
  through the existing touch menu so B-047 cannot mask action dispatch; the separate
  cold Select All cases remain intact and failing in their recorded scope.
- Native scene commands now expose Open, Duplicate, Copy Title and Archive Completed
  using the shared desktop binding definitions. Single-task commands require exactly
  one eligible selected row; archive requires the core listing's canArchive flag.
  Editing uses the existing TaskEditor; duplicate and archive call the existing core
  adapters. Duplication selects the new task, matching macOS. No business rule or
  generated binding was reimplemented in Swift.
- Focused command equality includes the selected title so a save cannot leave Copy
  using a stale captured title. Keyboard and touch Copy both use the native pasteboard
  and show localized Title copied feedback through the existing banner. The German
  translation is reused from MomentumKit; catalog validation is 309/309.
- First implementation run: Open/edit/copy of a newly saved title passes (30.085s).
  Duplicate and archive execute, but the combined test initially expected archive
  Undo to restore a completed task (`/tmp/momentum-task-more-keys-green26.xcresult`).
  Source inspection of the shared RestoreTask reducer and the captured Complete
  button establish that the existing restore semantics reopen the task. The test
  assertion was corrected; production core behavior was preserved. Added multiple
  selection checks prevent Open or Duplicate from choosing an arbitrary target.
- Native cross-runtime results and final build checks follow below. B-045–B-047,
  remaining planning/move/repeat/reorder/new-project shortcuts and complete parity
  remain open independently of this implementation.

- Final iOS 26.5 native run: five passes, exit 0
  (`/tmp/momentum-task-more-keys-final26.xcresult`). Duplicate/archive/Undo and
  multiple-selection guards pass in 33.001s; open/edit/copy updated title in 30.030s;
  Search text selection/Undo in 19.348s; modifier/tab/editor isolation in 34.383s;
  task selection/complete/reopen/Undo in 19.815s. The previously recorded editor
  frame warning is still present; these passes do not claim that layout warning
  or full accessibility acceptance is resolved.

- Visual review of the passing iOS 26.5 Open/edit/copy screenshot found B-048:
  selected-task Actions overlaps the native tab bar. Keyboard dispatch tests had
  only asserted the button existed. This follow-up includes a geometry/touch
  regression and a native safe-area layout correction, with largest-text checks;
  it is not counted as verified from the keyboard passes.

- Before the layout correction, all four selected iOS 27 command/focus tests pass,
  exit 0 (`/tmp/momentum-task-more-keys-final27.xcresult`): duplicate/archive/Undo
  35.300s, open/edit/copy 31.467s, Search text editing 23.560s, modifier/tab/editor
  isolation 34.657s. These remain distinct from cold keyboard-selection acceptance.
- B-048's native regression fails before the fix because Actions is not hittable
  (13.742s, `/tmp/momentum-selection-layout-red26.xcresult`). The menu now shares the
  existing bottom safe-area inset with Add instead of using a nested bottom toolbar.
  Standard text uses a horizontal arrangement; accessibility sizes stack the controls.
  Native menu behavior, action labels, selection clearing and the floating Add button
  are preserved. Geometry, actual menu completion and screenshot checks follow.

- The first B-048 layout run exits 0 on iOS 26.5
  (`/tmp/momentum-selection-layout-green26.xcresult`): duplicate/archive 32.252s,
  open/edit/copy 29.931s, standard layout/touch 18.675s, nominal AX5/dark 18.592s.
  Screenshot review invalidated the AX5 claim: launchFixture appended the standard
  size whenever AppleLanguages was not supplied, overriding this test's requested
  size. The latter result proves standard-size dark layout only. This was explicitly
  corrected in the conversation. Existing older AX5 tests supplied AppleLanguages
  and are not invalidated by this specific override. The helper and a stacked-layout
  assertion are being corrected before recording genuine largest-text acceptance.

- The corresponding iOS 27 pre-fixture-correction run also exits 0 with four passes
  (`/tmp/momentum-selection-layout-green27.xcresult`): new command cases 35.082s and
  31.621s; standard light layout 21.262s and standard dark layout 21.319s. The nominal
  largest-text name still does not establish AX5 for that bundle.
- With per-argument fixture defaults corrected, actual iOS 26.5 AX5 verifies the
  stacked controls and executes completion, but initially fails when looking for
  the offscreen result without scrolling (22.482s,
  `/tmp/momentum-selection-layout-ax5-26.xcresult`). The captured All done state and
  incremented pending count support the completed action. The test now scrolls to
  the resulting task using the existing native scroll helper. A white-ink assertion
  was added because an earlier standard-dark screenshot showed an empty Add-button
  capsule despite a valid accessibility label; pixel/render checks must not be
  inferred from hit testing alone. No production workaround was added for that
  isolated image before reproducing it with the new assertion.

- Corrected-fixture iOS 26.5 results: genuine AX5/dark passes in 28.390s, including
  scrolling to the completed task; standard dark passes in 18.752s, both exit 0
  (`/tmp/momentum-selection-layout-dark-ax5-26.xcresult`). Both also assert visible
  white Add-button ink from rendered pixels. The earlier blank-capsule image was
  not reproduced in this run; no unproven rendering workaround was added.
- Maximum-text screenshot review showed list content visible through the translucent
  action-control area. The bottom inset now uses the same opaque system grouped
  canvas as the screen, so text cannot show behind the controls. Final light/dark/AX5
  runs and screenshots are recorded separately after that visual correction.

- Final opaque-canvas iOS 26.5 scope: all three native selection cases pass, exit 0
  (`/tmp/momentum-selection-canvas-final26.xcresult`): standard light 19.198s,
  genuine AX5/dark 27.528s, standard dark 18.636s. Checks cover tappable nonoverlapping
  controls above the tab bar, 44-point minimum Actions height, stacked AX5 geometry,
  rendered white Add ink, menu completion and scrolling to its result. Exported
  final dark/AX5 screenshots were inspected: the opaque canvas removes the underlying
  text from the controls and retains a visible white Add label. No spoken VoiceOver,
  iPad or physical-device layout acceptance is claimed by this phone-simulator scope.

- Final opaque-canvas iOS 27 scope: all three native selection cases pass, exit 0
  (`/tmp/momentum-selection-canvas-final27.xcresult`): standard light 21.865s,
  genuine AX5/dark 29.712s and standard dark 21.254s. The same geometry, touch,
  stacking, scrolling and visible-white-ink assertions pass. Exported light and
  AX5 screenshots were inspected; selection actions no longer overlap the tabs
  or show underlying list text. All regression commands are retained in the first
  lines of their corresponding `/tmp/momentum-selection-canvas-final26.log` and
  `/tmp/momentum-selection-canvas-final27.log` files.

- The final normal Momentum simulator app build succeeds with Xcode 27, exit 0
  (`/tmp/momentum-task-actions-selection-build.log`). The existing Text-concatenation
  deprecation warning remains; no clean-warning or complete-accessibility claim is
  made. B-048 is Verified for the recorded phone-simulator overlap fix. F-010/F-018/
  F-020/F-023 remain In progress for their broader scope.
- German catalog validation (309/309), changed-Swift whitespace, documentation links
  and `git diff --check` pass. All native sessions are terminal. No Rust behavior,
  external dependency, commit, push or release changed. Next steps remain the missing
  planning/move/repeat/reorder/new-project shortcuts and B-045–B-047 input dispatch,
  followed by broader parity acceptance. Review the duplicate/last-added-ID actor
  call pair for concurrent entry-point selection before declaring all action races
  covered; the current tests prove the serial native flows described above.


### 2026-09-17 — F-023 planning shortcuts and B-049 atomic duplicate identity

- Previous goal turn was progress: four additional task keyboard actions and the
  verified B-048 phone-simulator selection-layout fix. Current dirty `task/ios-app`,
  base `7ec82e9`; local iOS/mobile-adapter work only. Remaining parity is not narrowed
  to the passing shortcut subset; LibreSync remains deferred.
- The previous handoff's duplicate/last-added-ID concern is now reproduced. A real
  EngineWorker/core test inserts another creation between the same two awaits used
  by the native list and fails its selected-title assertion in 0.160s
  (`/tmp/momentum-duplicate-selection-red.log`). The worker now returns Outcome plus
  optional copy ID without suspending between the mutation and identity read; the
  list uses that result. B-049 records the scope and preserves the existing public
  Outcome-only duplicate API for other callers.
- Native planning regressions first fail at the missing actions on iOS 26.5:
  Next Week 19.148s, Morning/Evening/Today 18.878s, Tomorrow 18.795s
  (`/tmp/momentum-planning-keys-red26.xcresult`, exit 65). The new scene commands use
  the same desktop shortcut definitions and existing core operations. Single Today
  uses core toggleToday; a selection uses planForToday, matching macOS routing.
  Slot exclusivity, date moves, schedule clearing and Undo stay in Rust.
- Dynamic Today/slot command labels use the core TaskMenuInfo for the selected task.
  The visible list queries only when selection or model revision changes; hidden
  lists, focused Search and active editors suppress the request. Cancellation and
  request identity prevent a stale result replacing the current selection's menu.
  Focused-value equality includes the menu state; no polling or eager per-row menu
  queries were added. Move to Today reuses the German macOS translation (310/310).
- Focused fast editing/organization validation passes 13 tests in 2.015s, 5.296s
  including incremental compilation (`/tmp/momentum-planning-fast-focused.log`).
  Coverage includes later/concurrent creation not changing duplicate result IDs,
  eight distinct copied identities, failure returning nil and single/bulk Today
  semantics with Undo. Preparation used the existing generated-core tooling and
  passed in 3.49s; no generated binding was manually edited.
- Final iOS 26.5 planning/action run passes all five cases, exit 0
  (`/tmp/momentum-planning-keys-green26.xcresult`): duplicate/archive/Undo 32.077s,
  Next Week/Undo 23.102s, modifier/tab/editor isolation 33.349s,
  Today/Morning/Evening/Undo 23.711s and Tomorrow/Undo 22.926s. Touch selection
  deliberately isolates these actions from the separately tracked cold-key failures.
- The current physical iOS 27 QA build/signing succeeds, but UI-test initialization
  stops before any test case with LocalAuthentication code -2, “Authentication
  canceled” (`/tmp/momentum-planning-keys-device27.xcresult`, exit 65). Xcode first
  waited for unlock, then the runner could not authenticate. This is an environment
  interruption, not a passing device check or a reproduced app-action failure.
  The user was asked whether to retry after approving the device automation prompt;
  simulator validation continues independently with the already-built test product.
- iOS 27 simulator validation passes all six selected cases, exit 0
  (`/tmp/momentum-planning-keys-green27.xcresult`): duplicate/archive/Undo 35.368s,
  Next Week/Undo 26.789s, native Search text editing 23.855s, modifier/tab/editor
  isolation 34.593s, Today/Morning/Evening/Undo 27.037s and Tomorrow/Undo 26.566s.
  The six tests execute in 174.209s. This uses the current simulator product from
  the preceding iOS 26.5 build via test-without-building; it does not waive B-045–B-047
  or establish physical-keyboard acceptance.
- The remaining iOS 26.5 native Search text-selection/Undo check passes in 19.647s,
  exit 0 (`/tmp/momentum-planning-search26.xcresult`). Together with the five-case
  run above, the same six-case planning/action/focus matrix passes on both phone
  simulator runtimes. No shortcut retries or warm-up keys were added.
- The normal Momentum simulator app builds for arm64/x86_64 with Xcode 27, exit 0
  (`/tmp/momentum-planning-app-build.log`). The existing Text-concatenation
  deprecation warning remains. B-049 is Verified for its actor-identity regression
  and recorded simulator flow; F-023 remains In progress for missing commands,
  B-045–B-047 and manual external-keyboard acceptance. Physical validation was
  interrupted before test execution as recorded above; it is not waived.
- German catalog validation (310/310), changed-Swift whitespace, local documentation
  links and `git diff --check` pass. All native sessions are terminal. No Rust rules,
  dependency, commit, push or release changed. Next work is move/repeat/reorder/
  new-project commands, then the remaining input-dispatch and broader parity checks.


### 2026-09-17 — F-023 project, repeat and reorder shortcuts

- Previous goal turn was progress: planning shortcuts and atomic duplicate identity
  passed scoped fast/native checks; physical QA initialization was interrupted by
  authentication. Current dirty `task/ios-app`, base `7ec82e9`; this turn continues
  available native parity work without retrying the pending device prompt.
- Native regressions first fail on iOS 26.5 for missing Move to Project (19.437s),
  reorder (15.735s) and Repeat (19.083s), exit 65
  (`/tmp/momentum-organization-keys-red26.xcresult`). Selection works in that fixture;
  failures occur at the missing sheet or unchanged task order, not cold Command-A.
- Four shared bindings now drive native actions: Move to Project, Repeat Schedule,
  Move Up and Move Down. Desktop eligibility is preserved: project commands capture
  selected parents, repeat requires one parent, and nudge passes one task and its
  current view to Rust. Manual-order, direction, group and family semantics stay in
  the core; no new rule or FFI change was added.
- Task sheets share one item-based presentation route. Repeat reuses RepeatEditor;
  the project sheet offers native destination buttons, Cancel, busy protection and
  persistent failure feedback. These sheets join the existing editor count so global
  commands, restore and pending task-opening routes respect an active presentation.
  Existing touch editing uses the same route. The shared help list includes the new
  bindings; German entries are 312/312 before final compiler catalog synchronization.
- Initial iOS 26.5 actions pass all five cases, exit 0
  (`/tmp/momentum-organization-keys-green26.xcresult`): bulk move/Undo 23.570s,
  open/edit/copy 29.942s, reorder/Undo 16.402s, repeat save/reopen/stop 28.183s,
  modifier/tab/editor isolation 33.556s. The subsequent family fixture adds a child:
  bulk family move/Undo passes in 24.242s; subtask-only move/repeat rejection passes
  in 16.453s; repeat with sheet screenshots passes in 28.269s. That boundary run
  fails its maximum-text sheet audit (B-050), so it is not an all-pass result.
- B-050's screenshot/audit identifies the combined Inbox folder/text label, and the
  longer name wraps underneath the symbol. The new sheet separates the decorative
  symbol from the wrapping title without ignoring any audit findings. Native
  maximum-text revalidation is underway.
- The remaining New Project shortcut first fails at its absent sheet in 19.364s,
  exit 65 (`/tmp/momentum-new-project-red26.xcresult`). It now reuses ContextEditor
  through RootView, preserves the active tab and supports Command-Return Save.
  Project/tag editors participate in the existing editor counter; pending task-open
  routes resume after dismissal from either keyboard or list-management entry.
  Explicit new-project admission guards cover the pre-presentation interval for
  commands, backup restore and task-open routing. Focused in-process model tests
  exercise those restore/route boundaries.
- Final iOS 26.5 checks pass, exit 0 (`/tmp/momentum-organization-final26.xcresult`):
  New Project/Command-Return/tab preservation 28.317s, project sheet at maximum text
  in dark appearance 20.437s, modifier/tab/editor isolation 33.855s. The project
  sheet's contrast, hit-region and text-clipping audit is unfiltered and passes.
  Exported before/after screenshots were inspected: the title now wraps in its own
  column beside the decorative folder, and every project is a labeled button.
  This does not claim spoken VoiceOver, iPad or physical-device acceptance.
- iOS 27 passes all eight selected cases, exit 0, in 211.006s
  (`/tmp/momentum-organization-final27.xcresult`): family move/Undo 25.953s,
  New Project with a Name-field tap 29.521s, open/edit/copy 31.777s, maximum-text
  project sheet plus unfiltered contrast/hit-region/clipping audit 21.534s,
  reorder/Undo 19.145s, repeat save/reopen/stop 29.331s, subtask-only eligibility
  19.190s, modifier/tab/editor isolation 34.556s. Exported light project, dark AX5
  project and Repeat screenshots were inspected. The Repeat screenshot reveals
  existing English “1 weeks” wording, now B-052, not claimed fixed by keyboard work.
- Tightening New Project to type without tapping Name reproduces B-051 on iOS 26.5
  in 20.073s (`/tmp/momentum-project-keyboard-entry26.xcresult`, exit 65). Native
  The initial defaultFocus declaration also fails (19.915s,
  `/tmp/momentum-project-focused26.xcresult`); it was removed. The retained correction
  sets the bound FocusState when the creation sheet appears, matching the existing
  capture editor. No sleeps, keyboard warm-ups or focus polling were introduced.
  The earlier tapped-name passes remain evidence of saving/tab preservation, not
  keyboard-only creation.
- B-051 keyboard-only creation now passes on iOS 26.5 (28.308s,
  `/tmp/momentum-project-appearance-focus26.xcresult`) and iOS 27 (29.344s,
  `/tmp/momentum-project-appearance-focus27.xcresult`), both exit 0. The test types
  at the application level without selecting a field, checks Name's value, saves
  with Command-Return, verifies the new project in Lists and checks tab preservation
  after both save and cancellation from Settings.
- Fast native model regressions are proven red/green with isolated stores. Temporarily
  removing only the new-project route/restore guards makes queued task opening occur
  prematurely (0.848s) and allows the test backup to replace an active draft's context
  (0.099s), exit 65 (`/tmp/momentum-project-guards-red26.xcresult`). The verification
  script restores the original source in `finally`; the restored guards were read back
  before the next build. With the real guards, all five in-process integration tests
  pass in 0.312s, exit 0 (`/tmp/momentum-project-guards-green26.xcresult`), covering
  shared worker entry, capture/help/project route deferral and restore admission.
  This duration is test execution, excluding compilation and simulator startup.
- The normal Momentum app builds for both simulator architectures with Xcode 27,
  exit 0 (`/tmp/momentum-organization-app-build.log`). The existing Text-concatenation
  deprecation warning remains. B-050/B-051 are Verified only for their recorded
  phone-simulator scopes. F-023 stays In progress for B-045–B-047 and full keyboard/
  device acceptance; the new commands do not waive those input failures.
- This turn added local native behavior and regression evidence without Rust-rule,
  FFI, dependency, commit or release changes. All native sessions are terminal.
  Next concrete work includes the B-052 English repeat interval defect and remaining
  keyboard/input and broader native parity acceptance. Physical QA retry still
  requires resolving the previously reported device authentication interruption;
  LibreSync remains explicitly deferred.
- Final compiler catalog synchronization found one missing German “Off” entry;
  it now reuses MomentumKit's existing “Aus” translation. Validation passes 313/313.
  The normal app was rebuilt after that resource update, exit 0
  (`/tmp/momentum-organization-catalog-build.log`), and its compiled German resource
  was read back to confirm `Off = Aus`. Changed-Swift whitespace, local documentation
  links and `git diff --check` pass. The final build session is terminal.

### 2026-09-17 — B-052 repeat interval resources and resumed device QA

- Previous goal turn: progress on project/repeat/reorder commands and their native
  focus/accessibility regressions. This turn also makes progress; full iOS parity
  and B-045–B-047 remain open. All changes are local in the inherited dirty
  `task/ios-app` worktree based on `7ec82e9`.
- The English interval defect is now reproduced in both Apple catalogs using
  actual `xcstringstool` products and Foundation lookups. Both tests failed on
  four singulars before the change (`/tmp/momentum-repeat-catalog-red.log`), then
  all 64 outputs passed after adding English one/other variations for days, weeks,
  months and years (`/tmp/momentum-repeat-catalog-green.log`). Existing German
  forms, typed interpolation, recurrence behavior and storage stay unchanged.
- The new fast catalog suite runs without an app/simulator (0.383s on the initial
  green run; 0.407s on the follow-up). It covers both apps, English/German, four
  units and counts 0/1/2/99. Testing guides now document the command and scope.
- The first native iOS 26.5 attempt failed a test query: SwiftUI combines the
  interval with its label as “Every, 1 week”. The screenshot and hierarchy show
  correct singular text (`/tmp/momentum-repeat-units26.xcresult`). Corrected the
  query and used the named Increment button; the subsequent interval-change,
  save/reopen persistence and stop flow passes in 31.493s, exit 0
  (`/tmp/momentum-repeat-label26.xcresult`). Its exported screenshot was inspected
  and shows “2 weeks” with the corresponding repeat description.
- Following the user's unlock reply, resumed the seven-case isolated physical
  keyboard run. The current QA app builds/signs, but Xcode reports the device
  locked again at preflight and waits before any test executes
  (`/tmp/momentum-planning-keys-device27-unlocked.log`). Requested that the phone
  remain awake through launch. No personal tasks/settings or real sync credentials
  are involved; no physical pass is claimed while preflight is waiting.
- The same built native regression also passes on iOS 27 in 32.510s, exit 0
  (`/tmp/momentum-repeat-label27.xcresult`). Exported interval screens were visually
  inspected on both runtimes. B-052 is Verified for this iOS editor scope and both
  compiled catalogs; macOS/German native interaction, physical-device and spoken
  accessibility acceptance remain separate. F-008/F-021 retain In progress.
- Final shipping iOS simulator build succeeds for arm64/x86_64, exit 0
  (`/tmp/momentum-repeat-app-build.log`). The existing TaskScreen text-concatenation
  deprecation warning remains. Catalog completeness (iOS 313/313, macOS app 295/295,
  MomentumKit 160/160, permission descriptions 2/2 and shortcuts 5/5), seven
  localization-tool tests, changed-document local links and diff whitespace pass.
- Direct device lock-state readback confirms a passcode is currently required.
  Physical QA remains waiting at Xcode preflight; no device test has executed in
  this resumed run. Reuse the same running session after unlock rather than
  launching a duplicate run. No commit, push or release was performed.

### 2026-09-17 — physical keyboard result and B-014 large-text remediation

- Previous goal turn: progress (B-052 resources/native regressions). This turn
  confirms the previously waiting device session progressed after unlock and
  terminated with exit 65; it is no longer waiting or blocked by the passcode.
- Physical iOS 27 QA result: six passes, one failure in
  `/tmp/momentum-planning-keys-device27-unlocked.xcresult`. Passes: duplicate/archive/
  Undo 34.958s; Next Week/Undo 23.025s; native Search text editing 20.065s;
  modifier/tab/editor isolation 32.128s; Today/Morning/Evening/Undo 24.553s;
  Tomorrow/Undo 22.971s. Cold Search fails after a single Command-F (13.722s).
  The exported screenshot remains on Today. No keyboard retry or warming was used.
  B-045 now explicitly records this physical failure alongside its earlier pass.
- No personal data, real account or hosted sync server was used. This remains
  XCTest input on a phone, not manual external-keyboard acceptance. F-023 stays
  In progress and full parity is not claimed.
- Resumed B-014 with strict native text-clipping regressions for the largest-text
  empty state and Task Lists settings, using the shipping app's isolated test store.
  This complements the older diagnostic inventory; these new tests do not suppress
  native findings. iPadOS 27 baseline execution is in progress.
- B-014 baseline failures are confirmed in the current shipping test app: iPadOS 27
  empty state 9.188s (`/tmp/momentum-ax5-empty-ipad27-before.xcresult`) and iOS 26.5
  Task Lists 9.924s (`/tmp/momentum-ax5-settings26-before.xcresult`). Both exit 65
  on native text clipping; screenshots reproduce the ellipsis.
- Explicitly wrapping the ContentUnavailableView label clears the empty-state audit
  on iOS 26.5 phone (6.651s, `/tmp/momentum-ax5-layout26-first.xcresult`) and iPadOS 27
  mini (8.503s, `/tmp/momentum-ax5-ipad27-wrapping.xcresult`). The latter screenshot
  now shows the complete two-line heading. Those bundles still exit 65 because the
  separate Settings clipping case fails; a passing case is not a passing suite.
- Task Lists uses full wrapping selected values and native selection pages at
  accessibility sizes, with standard-size menus retained. Screenshots confirm the
  original value ellipses are gone. The native audit still reports an anonymous
  text-clipping finding on phone/iPad, so B-014 remains In progress. A callback
  retains the unfiltered issue and confirms it has no current element; no findings
  are excluded. The oversized iPad editor glyph remains outside this change.
- Initial English/German selection-flow checks used a StaticText query that did
  not find the native inline Picker option. Video inspection shows all options,
  including fully wrapped Time Estimate. The query now matches the option's exact
  accessible label independently of element type. This is a test-query correction,
  not a production behavior fix; the rerun is in progress.
- The existing normal-size Settings persistence check reaches Appearance after
  persisting the toggle, then fails native contrast on the Colors header (23.710s,
  `/tmp/momentum-ax5-settings26-options.xcresult`). B-053 records this concrete
  finding separately; that Appearance surface was not changed by the picker work.
- Corrected selection queries now pass both full flows on iOS 26.5: German
  37.891s and English 32.189s, exit 0 (`/tmp/momentum-ax5-settings26-selection.xcresult`).
  Each sets grouping, sort, direction and Upcoming range, checks the displayed value,
  terminates/relaunches the isolated app and checks all four persisted values.
  Exported choice-page screenshots were inspected; German long words wrap without
  ellipses. These functional tests are separate from the still-failing screen audit.
- The same compiled app also passes English/German persistence and the strict
  empty-state clipping check on iOS 27, exit 0
  (`/tmp/momentum-ax5-phone27-selection.xcresult`). Screenshots were inspected.
  No Rust rules, preference keys, serialized values, shared bindings or dependencies
  changed. macOS/Linux presentation is unaffected by these iOS view changes.
- Final iOS 27 phone cases pass: German choices 39.858s, empty heading 8.853s,
  English choices 34.830s. iPadOS 26.5 mini's empty-heading check also passes,
  exit 0 (`/tmp/momentum-ax5-empty-ipad26-wrapping.xcresult`). Thus the English
  portrait heading audit is green on phone/iPad across both installed OS versions;
  other empty states, orientations, spoken VoiceOver and physical layout remain
  outside this scoped evidence. The description's scrolled reachability at AX5
  should be checked separately; heading/audit success alone does not prove it.
- Current code builds in the shipping iOS test target; 313/313 German entries,
  seven localization-tool tests and whitespace checks pass. Native sessions are
  terminal. No commit, push or release was performed. B-014 remains In progress
  for the unfiltered Settings finding, toolbar glyph and broader acceptance;
  B-053 and B-045–B-047 remain open. Next work: identify the remaining native
  accessibility findings, check large-text scrolling, and continue the parity list.


### 2026-09-17 — B-015 independent row targets and empty-state reachability

- Previous turn: progress on B-014 and physical keyboard evidence. Current dirty
  `task/ios-app` based on `7ec82e9`: progress, no external blocker and no parity
  completion claim. All changes remain local.
- B-015 native baseline fails at 20.33-point Open height. Open now includes title,
  metadata and note/reminder symbols, with a minimum 44-point height and explicit
  rectangular hit shape. Completion stays separate and archived rows read-only.
  Accessible values retain the existing metadata; shared task behavior is unchanged.
- Native production-view QA regression passes on iOS 26.5 (19.914s) and 27
  (22.213s): parent/subtask/second-row target dimensions and separation, unfiltered
  hit-region audit, project/estimate/tag/notes/reminder accessibility values,
  bottom-right Open tap, and complete/reopen. Bundles:
  `/tmp/momentum-row-targets26-metadata.xcresult` and
  `/tmp/momentum-row-targets27-metadata.xcresult`. The iOS 26.5 screenshot was
  inspected. Baseline: `/tmp/momentum-row-targets26-before.xcresult`.
- The first post-fix run caught double-coordinate rounding (43.99999999999994),
  so the regression permits 0.001-point noise. No hit-region findings are waived.
  The pre-existing B-012 editor warning still appears; this is not an all-audits pass.
- Added a separate shipping-app AX5 empty-description scrolling/quick-entry check;
  it passes on iOS 26.5 (8.429s, `/tmp/momentum-empty-scroll26.xcresult`) and 27
  (9.816s, `/tmp/momentum-empty-scroll27.xcresult`). Both screenshots were inspected:
  the full description scrolls above Add task, which opens quick entry. No further
  production layout change was necessary for this B-014 follow-up.
- Focused SwiftUI row rendering regressions pass: three tests, 0.293s test time,
  `/tmp/momentum-row-rendering26.xcresult`. They cover Dynamic Type/custom content
  scaling, long-title wrapping and completed appearance in light/dark. This is
  in-process execution time, not build/simulator startup time.
- Shipping app build/test succeeds; 313/313 German catalog entries, seven
  localization-tool tests, affected documentation file links and whitespace checks
  pass. All native sessions are terminal. No shared Rust changes, new dependencies,
  migration, commit, push or release. B-015 is Implemented with scoped native phone
  evidence; spoken VoiceOver, alternate input and physical/tablet row geometry
  remain unverified. B-014's Settings audit and toolbar glyph, B-012/B-053 and cold
  keyboard failures remain open. Next: native row accessibility at largest text,
  remaining unfiltered accessibility findings and the parity checklist.


### 2026-09-17 — B-016 persistent feedback and user device deferral

- Previous goal turn: progress (B-015 row targets and AX5 empty-description
  reachability). Current work continues in dirty `task/ios-app`, based on `7ec82e9`.
- The user explicitly said to finish but skip dip17pm. Further tests use simulators
  and local tooling; no unlock request or physical run is pending. Previous physical
  failures remain evidence, not a new reason to retry the phone.
- B-016 cause: each TaskScreen scheduled a five-second dismissal for every result,
  including errors; no announcement was posted. The native baseline confirms
  disappearance within six seconds (`/tmp/momentum-feedback26-deadline-before.xcresult`).
  An initial test used the singular message name; inspection confirmed the existing
  row action emits the bulk-completion message. The corrected baseline fails on
  the actual timing behavior.
- Feedback now remains until explicit dismissal or replacement by a later result.
  The model posts one low-priority attributed announcement per event. Hidden list
  reconstruction cannot repost it; dismissal/no-message results produce no speech
  request. Removed the timer and inappropriate updatesFrequently trait; text wraps.
  No task semantics, storage format or external dependency changed.
- Two in-process tests pass in 0.093s (`/tmp/momentum-feedback-unit-after.xcresult`):
  announcement payload/priority/count and unchanged model presentation, plus real
  isolated durable-save failure/draft retention. This does not prove spoken delivery.
- iOS 26.5 native regressions pass (22.672s success/dismiss/Undo; 21.891s AX5 dark
  error persistence/tab changes/nonoverlap/hit-region audit/dismiss):
  `/tmp/momentum-feedback26-after.xcresult`. Both screenshots inspected. iOS 27,
  tablet adaptation and the normal shipping build are pending below.
- iOS 27 phone cases pass: 25.648s success/dismiss/Undo and 25.208s AX5 dark
  error (`/tmp/momentum-feedback27-after.xcresult`). The iPadOS 27 mini AX5 error
  case passes in 26.143s (`/tmp/momentum-feedback-ipad27-adaptive.xcresult`), with
  an inspected screenshot showing the complete error and independent Add/dismiss.
  The initial tablet attempt failed only in the harness: native top tabs are
  buttons outside XCTest's TabBar container. Queries now use observed tab labels;
  overlap checks support both native tab placements. No app layout workaround.
- B-054 records the separately observed English singular-completion copy defect;
  shared MomentumKit resources lack English one/other variants. It is not hidden
  by the new feedback tests and remains Planned.
- No physical device was used. B-016 is Implemented with scoped native evidence;
  actual VoiceOver speech/focus and other open accessibility issues remain outside
  these passes. The overall goal remains active, with useful local work available.
- Final normal shipping simulator build passes, exit 0
  (`/tmp/momentum-feedback-shipping-build.log`). All native sessions are terminal.
  Catalog validation (313/313 German entries), seven localization-tool tests,
  affected documentation links and whitespace checks pass. No shared-core behavior,
  external dependency or data format changed; macOS/Linux presentation is untouched.
  No commit/push/release or dip17pm action. Next local work: remaining accessibility
  defects, B-054 English completion plurals and the full parity checklist.


### 2026-09-17 — B-054 shared English plurals

- Previous goal turn: progress (B-016 persistent feedback and scoped simulator
  evidence). Current dirty `task/ios-app` remains based on `7ec82e9`; local only,
  dip17pm excluded, no blocker or completion claim for overall parity.
- Found 21 shared catalog keys with German plural variants but none for English.
  Added compiled-resource regression first: 168 outputs (21 keys, English/German,
  0/1/2/99), 21 English singular failures (`/tmp/momentum-shared-plurals-before.log`).
  Added English one/other variants; all 168 pass in ~0.4s. Typed UInt32/UInt64
  Foundation interpolation matches the production keys. No source string parsing
  or JSON-shape-only assertion stands in for real localization behavior.
- Shared resources now handle singular task actions/summary, relative time,
  snoozing/recurrence, Upcoming days and device counts. German text, existing
  keys/placeholders, core behavior and formats are preserved. The linked-device
  resource correction does not implement or resume deferred LibreSync work.
- All 136 MomentumKit tests pass in 12.462s
  (`/tmp/momentum-shared-plurals-kit-tests.log`). Catalog validation passes:
  shared 160/160 German entries, Mac app 295/295, InfoPlist 2/2, AppShortcuts 5/5.
- Native iOS completion/summary regressions pass on 26.5 (23.150s) and 27 (26.285s),
  including existing persistent-feedback/dismissal/Undo behavior:
  `/tmp/momentum-completion-plurals26.xcresult` and
  `/tmp/momentum-completion-plurals27.xcresult`. The 26.5 screenshot was inspected.
  Normal macOS/iOS consumer builds are pending below. Actual German/Mac UI speech,
  broader accessibility and the remaining parity checklist are not implied passed.
- The normal macOS build passes, exit 0
  (`/tmp/momentum-shared-plurals-macos-build.log`). Running the same 168-output
  Foundation verifier against the shared resource bundle packaged inside
  Momentum.app also passes. The final shipping iOS build is running; no physical
  device or personal task store has been touched.
- Final standalone resource checker passes in 0.471s, including an explicit
  assertion that all 168 lookups executed. The test runner prints the verified
  count, avoiding a green result from an accidentally empty selection.
- Final normal iOS simulator build passes, exit 0
  (`/tmp/momentum-shared-plurals-ios-build.log`). The 168-output Foundation check
  also passes against the MomentumKit bundle packaged in that app. Both consumers
  build successfully; all native sessions are terminal. B-054 is Verified for
  the recorded resource and iOS completion/summary scope, not all localization UI.
- Whitespace and affected documentation links pass. No core rule/API/data-format
  changes, personal-data use, device run, commit, push or release. Overall parity
  remains active. Next: remaining accessibility defects (including B-017 parent
  context), cold keyboard failures and the existing acceptance checklist.


### 2026-09-17 — B-017 localized subtask action context

- Previous goal turn: progress (B-054 compiled plural resources and both Apple
  consumer builds). Current dirty `task/ios-app` remains based on `7ec82e9`;
  local only, no dip17pm run, no overall parity completion or blocker claim.
- Baseline native iOS 26.5 test failed because identically named child tasks exposed
  indistinguishable Open/Complete labels. Artifact:
  `/tmp/momentum-subtask-context26-before.xcresult`. The new worker regression also
  failed compilation before parent context existed in TaskSnapshot.
- The worker now resolves distinct live parent titles once per snapshot off-main,
  reusing visible rows and fetching hidden Search parents. TaskRowContent exposes
  English/German parent context for separate Open/Complete/Reopen actions. Visible
  archived parents also supply context; absent filtered archived parents currently
  fall back to localized generic subtask wording. This remaining edge case and
  actual spoken VoiceOver/focus acceptance stay open in B-017.
- Four EngineWorker tests pass in 0.466s, including duplicate child titles, hidden
  Search parents, rename refresh and empty search. Log:
  `/tmp/momentum-parent-snapshot-after.log`. No Rust rules, FFI, data format,
  dependencies or desktop behavior changed.
- Three native tests pass on iOS 26.5 (61.605s) and 27 (69.958s): English/German
  contextual Open, edit/cancel, complete/reopen, filtered Search, plus the existing
  independent 44-point target/metadata/edge-tap regression. Bundles:
  `/tmp/momentum-subtask-context26-after.xcresult` and
  `/tmp/momentum-subtask-context27-verified.xcresult`. English/German hierarchies
  and the German Search screenshot inspected. Existing B-012 invalid-frame warning
  remains visible when opening the editor; these passes do not resolve it.
- The first iOS 27 launch stopped before tests because sandboxed SwiftPM caches
  were unwritable. The authorized simulator run used a fresh result path after
  that terminal startup failure; no test failure was warmed or hidden.
- German catalog validation passes 315/315; whitespace check passes. Full mobile
  suite, native component regressions and final shipping build follow below.
- Final snapshot lookup uses the existing lightweight core `taskTitle` query for
  hidden parents, avoiding construction of unused row metadata. All 115 portable
  mobile tests pass in 3.622s (`/tmp/momentum-subtask-mobile-final.log`).
- Three actual SwiftUI row-rendering tests pass in 0.297s (text scaling, compact
  wrapping and completion appearance in light/dark):
  `/tmp/momentum-subtask-rendering26.xcresult`. Native action tests above preceded
  the equivalent lightweight lookup substitution; the full worker suite covers
  that final lookup. The unchanged row rendering code remains the tested version.
- Normal production iOS simulator build is running with core regeneration enabled:
  `/tmp/momentum-subtask-shipping-build.log`. No personal task store, hosted account,
  physical device, commit, push or release has been used.
- Final normal production iOS simulator build passes, exit 0. All sessions are
  terminal; affected documentation links, German catalog coverage and whitespace
  checks pass. B-017 remains Implemented with scoped simulator evidence and explicit
  archived-parent/spoken-acceptance limits. Overall parity remains in progress;
  next work is the remaining archive context, keyboard and accessibility defects.


### 2026-09-17 — B-017 archived parent context

- Previous goal turn: progress (localized live subtask controls, mobile/native tests
  and production build). Continuing in the same dirty `task/ios-app`, local only;
  dip17pm remains excluded. Overall parity is active, with no external blocker.
- Added an isolated archived-search regression first. It reproduced missing parent
  names (empty parentTitles) while live context returned after Undo:
  `/tmp/momentum-archive-parent-mobile-before.log`, 2 failed assertions, 0.333s.
- Existing `task_title` is used by desktop live-task actions and remains live-only.
  Added a separate batch `task_reference_titles` core query for read-only names
  across live state, archiveYoung and archiveOld. It locks once, reads only requested
  records and does not enumerate/materialize the whole archive. Mobile snapshots
  reuse visible parents and batch only hidden parents. No persistence format changes.
- New core regression passes for both archive tiers, duplicate IDs, missing/invalid
  records, live/young precedence, no task-state or undo mutation and unchanged
  read-only archive semantics. Initial test failed to compile before the API existed;
  `/tmp/momentum-archive-parent-core-after.log` passes after implementation.
- The first workspace run hit sandbox-denied local IPC in five existing CLI tests.
  The same suite is running with authorized host fixture access, not with assertions
  disabled. Binding regeneration and final consumer/native checks follow below.
- Host-authorized all-feature/all-target workspace suite passes: 254 tests across
  13 binaries, one intentionally ignored manual latency sample. Includes `mo`
  consumer and existing live/archive mutation regressions. Log:
  `/tmp/momentum-archive-parent-workspace-verified.log`. Rust formatting passes.
- Official iOS test preparation regenerates the XCFramework/Swift FFI successfully
  in 16.228s (`/tmp/momentum-archive-parent-prepare.log`). The generated Swift method
  returns `[String: String]` with its new checksum; no generated source was edited.
- Full mobile fast lane passes: 116 tests/19 suites, 3.519s test runtime, 8.478s
  including incremental build (`/tmp/momentum-archive-parent-mobile-after.log`).
  This includes the previously failing archive/search/Undo context test.
- Existing macOS package regression passes: 136 tests/32 suites, 12.012s
  (`/tmp/momentum-archive-parent-mackit.log`). Existing consumer API semantics
  remain unchanged; full macOS/iOS app builds follow native checks.
- Native iOS 26.5 passes all three tests in 51.699s: English/German archived
  Search parent context with no edit/completion buttons, plus live subtask
  open/complete/reopen/Search regression. Bundle:
  `/tmp/momentum-archived-context26.xcresult`. Exported hierarchies show distinct
  parent labels on read-only StaticText elements and retain Inbox metadata.
  The German archived Search screenshot was inspected. iOS 27 is running.
- The same three native tests pass on iOS 27 in 62.017s:
  `/tmp/momentum-archived-context27.xcresult`. This closes the known missing
  archived-parent-name implementation gap for both supported simulator runtimes.
  B-017 remains Implemented pending spoken VoiceOver/focus and alternate input;
  UI hierarchy inspection is not a claim about actual speech.
- German coverage remains 315/315; no new translation key is required. Normal
  macOS production build is running, then the iOS production build will verify
  the shipping consumer. No physical device or real sync account has been used.
- Normal macOS production build passes, exit 0:
  `/tmp/momentum-archive-parent-macos-build.log`. The iOS production build is
  running. The shared API addition preserves existing desktop calls; Linux native
  runtime/GTK interaction was not rerun on this Mac and is not marked Verified.
- Normal iOS production simulator build passes, exit 0:
  `/tmp/momentum-archive-parent-ios-build.log`. All sessions are terminal. Final
  Rust formatting, whitespace, localization and affected documentation links pass.
  Shared core, CLI, generated Swift boundary, both Apple packages and both app
  builds have evidence; English/German native archive Search passes on 26.5/27.
- Current goal turn is progress: the archived-parent implementation gap is closed
  without narrowing behavior or making archive rows editable. Overall parity remains
  active. Next: remaining accessibility defects (including B-018 validation text),
  cold keyboard failures and outstanding native acceptance. Spoken VoiceOver and
  user-deferred dip17pm checks are not implied passed. No commit/push/release.


### 2026-09-17 — B-018 semantic validation contrast

- Previous goal turn: progress (archived parent context, shared/native regressions
  and both Apple production builds). Same dirty `task/ios-app`, local changes only,
  dip17pm excluded. Overall parity remains active; no external blocker.
- Reproduced the original regular-size estimate guidance contrast failure with
  an unfiltered native iOS 26.5 audit:
  `/tmp/momentum-validation26-before.xcresult` (15.302s, failed “Contrast nearly
  passed” on the guidance). No issue handler waived the finding.
- Added `AccentTheme.errorText`, resolving semantic system red through the existing
  surface-aware text-color algorithm (normal minimum 4.55, Increase Contrast 7.05).
  Task-save, estimate and recurrence errors now use this token with an error SF
  Symbol. Wording, parser, draft/save behavior and default orange policy are unchanged.
- The first after-run passed dark contrast and reached save/reopen, where the test
  incorrectly expected `2h` rather than the existing canonical `2h 00m` value. Fixed
  that assertion. The light audit exposed a different disabled “Does not repeat”
  value; its source now explicitly uses the readable secondary text token. The
  finding remains in `/tmp/momentum-validation26-after.xcresult`, not suppressed.
- Native trait test passes 48 combinations: six system surfaces, light/dark,
  normal/high contrast, base/elevated interface level. Error text meets at least
  4.5:1 or 7:1 respectively. Existing exact-orange/white-button exception test also
  passes; both tests take 0.007s (`/tmp/momentum-error-ink26.xcresult`).
- Native checks are running for light/dark estimate correction and save/reopen,
  German maximum-text guidance, and recurrence error/recovery. Whole-screen results
  and final production build follow below. Actual spoken error announcements/focus
  remain separate under audit A-05; this change addresses readable presentation.
- `/tmp/momentum-validation26-final.xcresult` passes ordinary light (21.109s),
  dark (21.859s) and recurrence rejection/recovery (34.479s). German AX5 fails on
  the scrolled `Fällig` label at y=41 beneath the sheet navigation bar, not on the
  error message. Inspected screenshot shows text visible behind the toolbar.
- This reproduces the previously unverified sheet scope of B-039. TaskEditor and
  RepeatEditor now apply the existing `momentumNavigationCanvas` on their Form
  content, matching other native destinations. The same four strict regressions
  are rerunning; no finding was waived. The screenshot also retains B-014's known
  oversized Save symbol at maximum text, which is a separate open layout issue.
- After applying the canvas, the German screenshot shows the `Fällig` label fully
  covered by the opaque toolbar, while XCTest still reports its retained frame
  (y=41…98). `/tmp/momentum-validation26-canvas.xcresult` retains this failure;
  ordinary light/dark and recurrence flows continue to pass. The visible error
  guidance is complete and readable. Exported before/after screenshots inspected.
- Following Apple's documented audit triage guidance
  (https://developer.apple.com/videos/play/wwdc2023/10035/, 9:57), the German test
  now handles only a contrast issue on StaticText `Fällig` whose bottom is at or
  above the navigation-bar bottom. Findings remain attached. Before auditing, it
  verifies the complete error is inside the viewport and three navigation-surface
  pixels match the canvas. A visible `Fällig`, other label or other issue still
  fails. This is an explicit occluded-element false-positive exception, not an
  unfiltered whole-screen compliance claim. Other audit helpers remain unchanged.
- The initial pixel-gated German rerun failed before auditing: the generic first
  navigation query selected underlying Today, and the sampling set included the
  Save control. Corrected the harness to the named active editor bar and three
  clear canvas samples. Tightened the handled issue to an existing positive-height
  StaticText beginning above that bar and ending no lower than its bottom. This
  preserves failures for any visible portion extending below the toolbar.
- German AX5 now passes the active-sheet pixel/geometry and recovery check on
  iOS 26.5 (27.907s; `/tmp/momentum-validation26-sheet.xcresult`). The following
  iOS 27 run passed all estimate flows (22.534s dark, 21.710s light, 28.106s German),
  but recurrence reported `Starts` at y=864…884, partly outside the 874-point
  screen. Screenshot confirms this is clipped at the bottom, not low-contrast
  error text (`/tmp/momentum-validation27.xcresult`).
- The recurrence test now retains/handles only that specific partly-offscreen
  Starts finding, scrolls the field fully into view, asserts its bounds and runs
  another contrast audit with no exclusions before proceeding with recovery.
  A stable `repeat-editor-form` identifier targets the topmost sheet's Form.
- Native hierarchy inspection also found the Estimate field had an empty label
  after typing (only its example placeholder remained). New assertion reproduces
  the empty label (`/tmp/momentum-estimate-label-before.xcresult`, 12.732s failure).
  Added the existing localized Estimate accessibility label; final cross-version
  estimate checks assert English/German labels without changing values or input.
- Final iOS 27 suite passes all four tests in 112.738s:
  `/tmp/momentum-validation27-final.xcresult`. Estimate light/dark and German AX5
  assert localized field names, disabled Save, readable/visible guidance and durable
  corrected values. Recurrence passes the second fully-visible date audit with no
  exclusions, then saves after choosing Monday. This validates rather than merely
  dismisses the offscreen Starts finding. Final iOS 26.5 parity run follows.
- Final named-field estimate checks pass on iOS 26.5: 21.615s dark, 21.381s
  light, 27.473s German AX5 (`/tmp/momentum-validation26-current.xcresult`).
  Its recurrence follow-up initially scrolled even though that runtime's date
  already passed the first audit, exposing a different covered summary label.
  The test now repositions/re-audits only if it actually deferred the Starts issue.
  No initial finding is skipped by this change. Final 26.5 recurrence passes in
  34.824s (`/tmp/momentum-repeat-error26.xcresult`); 27's deferred branch is rechecking.
- Final recurrence deferred-date branch passes on iOS 27 in 40.644s:
  `/tmp/momentum-repeat-error27.xcresult`. Its attachment confirms the second
  fully-visible date audit actually executed; that audit has no exclusions.
  Both runtimes now have passing final estimate and recurrence recovery evidence.
  The iOS 27 native color matrix and normal production build remain in flight.
- iOS 27 native color/brand tests pass in 0.006s:
  `/tmp/momentum-error-ink27.xcresult`. Both supported runtimes now validate all
  48 error-color trait combinations and the exact default-orange/white-ink policy.
- Normal production iOS simulator build is running with its usual core-generation
  gates (`/tmp/momentum-validation-shipping-build.log`). No Rust/task-rule changes
  were made in this turn, and existing desktop consumers are unchanged. German
  catalog coverage remains 315/315; whitespace and affected documentation links pass.
- Final production iOS simulator build passes, exit 0
  (`/tmp/momentum-validation-shipping-build.log`). All sessions are terminal.
  B-018 is Verified for semantic error-color traits, localized Estimate names and
  the recorded phone-simulator recovery scope. B-039's task/repeat sheet canvas
  improvement has visual/pixel evidence; whole-app spoken/alternate-input
  acceptance remains open. Final whitespace and affected documentation links pass.
- Current goal turn is progress: readable errors, named input, corrected sheet
  navigation, behavioral regressions and explicit audit triage now exist in the
  authoritative worktree. No device testing, real accounts, commit, push or release.
  Overall parity stays active. Next: B-014's oversized toolbar Save symbol and the
  remaining cold keyboard/accessibility acceptance gaps.


### 2026-09-17 — native sheet commit symbols at large text (B-014, F-019/F-020)

- Dirty `task/ios-app`, base `7ec82e9`; local changes only. Physical dip17pm
  checks remain explicitly skipped at the user's request.
- Reproduced the toolbar glyph defect on iOS 26.5: the white Save ink reaches
  within two points of its fill edge at AX5. Pixel regression fails in
  `/tmp/momentum-toolbar-symbol26-baseline.xcresult`. The initial harness also
  established that the native visual button frame is 36 points, which is not
  evidence of its effective touch area; no custom hit-target claim is made.
- `SheetCommitButton` now clears the inherited app content font on its symbol,
  letting the native toolbar choose its font. Task text keeps Dynamic Type and
  app font preferences. Accessible Save/Create names, actions and orange/white
  versus Increase Contrast ink policy are retained.
- Added native Save/Create regression flows at standard text in light mode and
  AX5 in dark mode. They inspect actual enabled-button pixels for white ink with
  at least four points of orange fill around it, excluding the capsule's glass
  rim and background corners, and complete both actions using disposable data.
- iOS 26.5 phone checks pass: 19.158s AX5 and 18.846s standard,
  `/tmp/momentum-toolbar-symbol26-native.xcresult`. Inspected both AX5 screenshots:
  toolbar symbols fit inside their native round controls; task text remains large.
  iOS 27 phone and iPad mini 26.5/27 verification is in progress.
- Remaining matrix passes, exit 0: `/tmp/momentum-toolbar-symbol-matrix.xcresult`.
  AX5/standard times are iOS 27 phone 20.799/20.319s, iPadOS 27 mini
  22.323/20.929s, and iPadOS 26.5 mini 20.260/19.267s. All six cases verify
  both Save and Create. Inspected the two iPad AX5 Save screenshots; the original
  oversized checkmark is contained inside its native orange control on both OSes.
  Combined with the 26.5 phone pair, all eight scoped native scenarios pass.
- Existing native color-policy/component regressions pass on iOS 27 (two tests,
  0.080s), `/tmp/momentum-toolbar-ink27.xcresult`, covering actual white ink in
  light/dark and calculated Increase Contrast traits. No further production edits
  followed these checks. Normal production simulator build is running with its
  usual Rust-generation gates. Affected documentation links and whitespace pass.
- Normal production iOS simulator build passes, exit 0:
  `/tmp/momentum-toolbar-shipping-build.log`. Toolbar glyph remediation is verified
  for the eight recorded scenarios; B-014 remains open for separate Settings audit
  findings and broader window/assistive acceptance. Physical-device work was skipped.

### 2026-09-17 — Appearance contrast finding investigation (B-053)

- Added isolated production-root Appearance light/dark audit cases to
  `MomentumSyncUITests.SyncSetupTests`. Neither suppresses findings. Reproduced
  the Colors header failure on iOS 26.5 light; dark passes:
  `/tmp/momentum-appearance26-baseline.xcresult`.
- Controlled rendering experiments did not repair the audit: a 5:1 secondary
  target (`/tmp/momentum-appearance26-after.xcresult`), hierarchical `.primary`
  (`/tmp/momentum-appearance26-primary.xcresult`), explicit native UIColor.label
  (`/tmp/momentum-appearance26-label.xcresult`), and a fixed-size header
  (`/tmp/momentum-appearance26-bounds.xcresult`) all still flag Colors in light.
- Exported element screenshots prove the colors actually changed. Dominant glyph
  pixels versus the (242,242,247) canvas are original (110,110,113), 4.5547:1;
  strengthened (104,104,107), 4.9773:1; and solid black, 18.8194:1. The unchanged
  audit failure with solid-black glyphs rules out simply inadequate text ink as
  the explanation. The 1110×120-pixel accessibility screenshot includes the
  whole header row and a faint native chrome shadow; shadow/bounds attribution
  is a hypothesis, not a confirmed root cause.
- All diagnostic production changes were reverted. The verified toolbar font
  fix remains. B-053 stays under investigation with a suspected native-audit false
  positive; no exclusion or compliance claim was added. Cross-version baseline
  is running. Device work remains skipped at the user's request.
- Final iOS 27 baseline passes with restored original styling: dark 16.894s,
  light 16.395s, `/tmp/momentum-appearance27-baseline.xcresult`. The current
  finding is runtime-specific in the tested scope. B-053 remains under
  investigation; iOS 26.5's light audit still fails and is not suppressed.
- All sessions are terminal. Production source equals the successfully built
  toolbar-fix source; only diagnostic tests/docs were retained from B-053 work.
  Current goal turn is progress, not parity completion. Next: resolve the
  iOS 26.5 audit discrepancy and remaining Settings/cold-keyboard acceptance.
  No physical-device work, accounts, commit, push or release.

### 2026-09-17 — independent Simulator input boundary check (F-023/B-045–047)

- Previous goal turn made progress: B-014's toolbar fix and B-053's controlled
  color/audit evidence. User reiterated simulator-only verification; do not use
  dip17pm for any install, launch or test.
- Tried an input path independent of XCTest: macOS System Events keyboard events
  routed through Simulator to a UUID-isolated QA app. The installed Xcode 26.5
  Simulator UI rendered booted devices as blank external displays. Switching to
  the installed Xcode 26.6 Simulator UI displayed the iOS 26.5 phone normally.
- On that visible fresh fixture, Command-A did not change selection; the known-good
  Command-2 control also did not navigate. Therefore this alternate automation path
  is inconclusive and does not establish a new app defect or a keyboard pass.
  Screenshots: `/tmp/momentum-independent-keyboard26-ready.png`, `-selected.png`,
  `-control.png`. No production keyboard workaround, retry or regression waiver.
- Isolated iOS 26.5 fixture cleanup was launched. The unused iOS 27 fixture's final
  cleanup attempt found its simulator Shutdown after switching Simulator hosts;
  no live test was restarted or user data touched. Existing UUID-only QA cleanup
  recovers leftover fixtures on its next launch. Physical-device work stays skipped.

### 2026-09-17 — reusable snapshot projection and opt-in performance lane (B-055)

- F-001/F-039, dirty `task/ios-app` based on `7ec82e9`; iOS presentation package
  only, no Rust rules/FFI or desktop behavior change. Simulator-only scope.
- Added `SnapshotPerformanceTests` to the existing minimal native host and a
  dedicated Release `MomentumPerformanceTests` scheme. It selects just that class
  and supplies an opt-in environment flag. The everyday fast scheme skips the
  benchmark before allocating its isolated 1,000-task fixture. XcodeGen regenerated
  the project from `ios/project.yml`; no extra target/dependency was introduced.
- Baseline: 100 reads of a real 1,000-task snapshot cost mean 15.939ms over five
  iterations on iOS 26.5 simulator, Release Swift:
  `/tmp/momentum-snapshot-performance-before26.xcresult`. Fixture generation,
  Rust persistence/query and actor hops are excluded. Prepared Rust is debug;
  this measurement concerns the Swift presentation projection only.
- TaskSnapshot now stores the ordered task array that EngineWorker already builds
  while resolving parent titles. The immutable array crosses the actor boundary
  once and subsequent selection/keyboard/drag reads reuse it. No second task rule,
  mutable cache invalidation or global storage is added.
- After: 2–4 microseconds per same 100-read block (measurement-floor scale),
  `/tmp/momentum-snapshot-performance-after26.xcresult`, 0.538s total test.
  Memory peak observations were 33.557MB before and 33.907MB after; both measured
  zero net physical growth across a block. Separate-process noise and the retained
  array prevent treating that difference as an allocation or energy measurement.
  No frame-rate, launch, battery-life or physical-device performance claim.
- Added a behavioral regression proving old snapshots retain their rows/completion
  values through later completion, deletion and undo while fresh snapshots update.
  The required preparation guard first rejected stale generated artifacts; normal
  `prepare` then passed in 3.266s. Full fast mobile suite passes: 117 tests in
  3.956s, command 9.0s, `/tmp/momentum-snapshot-fast.log`. Nothing bypassed the guard.
- iOS 27 opt-in benchmark and final native/build acceptance follow below.
- iOS 27 Release benchmark also passes (0.783s total test),
  `/tmp/momentum-snapshot-performance-after27.xcresult`; measured read blocks are
  2–4 microseconds. This is a cross-runtime after-change check, not a separate
  before/after comparison. Final production-view row/archive interaction checks
  are running on both simulator versions. Generated scheme readback confirms
  Release plus the opt-in class/flag; the fast scheme has no benchmark flag.
- Production-view interaction matrix passes, exit 0:
  `/tmp/momentum-snapshot-native-matrix.xcresult`. Archived subtask Search retains
  parent context (26.5: 16.926s; 27: 20.301s), and independent Open/Complete targets,
  metadata, native hit-region audit, editing and completion/reopen pass
  (26.5: 19.804s; 27: 22.390s). These tests do not depend on the unresolved cold keys.
- Fast-lane opt-in guard verified on iOS 26.5:
  `/tmp/momentum-performance-opt-in-guard26.xcresult`; benchmark skips before setup,
  and the real native accent/Increase Contrast test passes. Two discovered tests,
  one skip, zero failures, 0.005s. This is deliberately not two executed passes.
- iOS 26.5 keyboard-probe task store is absent and its residual preferences plist
  is an empty dictionary. Initial existence-only check was too strict; no retained
  preference entries remain. iOS 27 shut down after native verification, so its
  container cleanup is checked read-only via its exact QA bundle metadata.
- Normal production simulator build passes, exit 0, through its existing core gates:
  `/tmp/momentum-snapshot-shipping-build.log`. B-055 is Verified for this scoped
  immutable projection and row/archive regression; broad performance acceptance remains open.
- Final read-only inspection of the exact iOS 27 QA bundle container confirms its
  keyboard-probe store is absent and preferences have zero entries. Both runtime
  fixtures are cleaned. No physical device was accessed.

### 2026-09-17 — native large-list performance baseline (F-001/F-039)

- Added a separate opt-in Release UI scheme using the existing isolated QA target,
  production RootView/TaskScreen/model, and a 1,000-task Rust-seeded disposable store.
  Initial seeding, fixture cleanup and process termination are outside measurements.
  Ordinary UI runs skip the class before fixture creation; no production hook,
  dependency, data migration or everyday fast-suite work was added.
- Three iterations sample responsive launch, launch-to-first-task wall time, and
  scrolling/deceleration with app CPU/memory metrics. Assertions require persisted
  rows, actual scrolling and the first row becoming visible again. Apple SDK headers and
  primary documentation confirm the metric scope; the guide records QA-entry-point,
  automation-overhead and potentially debug-Rust limitations. No battery claim.
- The first harness run failed because explicit stopMeasuring requires the native
  manual-stop option. Corrected that option, async main-actor cleanup, and screenshot
  capture only while the app is running. This was a measurement-harness error, not
  an app regression. The original failed bundle is `/tmp/momentum-ui-performance26.xcresult`.
- Corrected matrix passes all four cases, exit 0:
  `/tmp/momentum-ui-performance-matrix.xcresult`. Launch/scroll test durations:
  iOS 26.5 28.638s/57.171s; iOS 27 38.534s/59.470s. Each test validates fixture
  cleanup. Swift is Release optimized; the prepared Rust artifact is debug.
- Three measured launch iterations average 0.894s (26.5) and 2.819s (27) until the
  system's first-frame/responsive point. Launch-through-visible-row wall time averages
  3.109s/4.525s including XCTest overhead. These are QA application relaunches with
  a persisted store, not shipping cold-install latency or an inter-version regression claim.
- Scroll signpost duration averages 2.573s (26.5) / 2.572s (27) per four-gesture block;
  mean app CPU time 1.600s/1.319s and mean peak physical memory 62.016MB/67.335MB.
  No frame-rate/hitch metric was returned on these simulators, so scroll smoothness
  and energy are not proven by the duration metric. Raw metrics were exported to
  `/tmp/momentum-ui-performance-metrics.json`.
- Both retained screenshots were inspected: ordered task rows, consistent canvas,
  white-on-orange Add button above the four tabs. The assertion only establishes the
  first row is visible again; the native large-title collapse state can differ.
  Search/edit profiling, energy and broader parity acceptance remain separate.
  Only simulators were used.
- Normal UI-scheme opt-in guard passes, exit 0:
  `/tmp/momentum-ui-performance-opt-in26.xcresult`; both tests skip before fixture
  allocation, 0.103s, zero failures. This is two intentional skips, not two executed
  performance passes. All producer sessions are terminal. The production app source
  remains the version that passed `/tmp/momentum-snapshot-shipping-build.log`;
  subsequent changes are test fixtures, schemes, test diagnostics and documentation.
- This goal turn made concrete progress on B-055 and repeatable performance evidence;
  full parity remains active. Next: search/edit profiling and remaining native
  keyboard/accessibility acceptance, respecting the simulator-only constraint.

### 2026-09-17 — Settings contrast closure and cross-platform regression (B-053/F-020/F-039)

- Dirty `task/ios-app`, base `7ec82e9`; local changes only. The user required
  simulator-only verification, so dip17pm and every other physical device were excluded.
- The iOS 26.5 native audit continued to flag Settings text after its rendered ink was
  strengthened. Exported element pixels measure `#616164` on `#F2F2F7` at 5.5316:1;
  Refresh Schedule renders black on white at 21:1. The corresponding iOS 27 shipping
  workflows pass, consistent with the earlier solid-black Colors experiment still
  failing only on iOS 26. B-053 is therefore classified as a runtime audit false positive
  for the exact reproduced labels, rather than an unreadable app color.
- `AccentTheme.secondaryText` now keeps a 5.5:1 normal-contrast margin across six native
  surfaces and 7.05:1 under Increase Contrast. The native trait regression enforces both
  thresholds. The Refresh Schedule action uses primary label ink and an accent symbol.
  XCTest handles only Colors, Delivery, Schedule, Refresh Schedule and the exact schedule
  footer on iOS 26; all other findings and all iOS 27 audits remain unfiltered.
- Shipping Appearance/Notification persistence and contrast workflows pass on iOS 26.5:
  three tests, 82.855s, `/tmp/momentum-secondary-native26-final.xcresult`. The same three
  pass unfiltered on iOS 27: 87.835s, `/tmp/momentum-secondary-native27.xcresult`.
  Isolated Appearance light/dark also passes on 26.5 (29.353s,
  `/tmp/momentum-appearance26-final.xcresult`) and 27 (33.501s,
  `/tmp/momentum-appearance27-final.xcresult`).
- Repaired audit queries now validate every Task Lists viewport and contextual subtask
  label. The focused matrix passes on iOS 26.5 (separate retained bundles) and iOS 27:
  25-screen audit 291.178s, subtask audit 33.750s and five-viewport Task Lists audit
  21.601s, `/tmp/momentum-accessibility-focus27.xcresult`. Spoken VoiceOver and external
  keyboard acceptance remain open; these automated traversals do not substitute for them.
- Final portable mobile suite passes 119 tests/20 suites in 4.015s
  (`/tmp/momentum-fast-final.log`). Native view integration passes 44 tests with two
  intentional opt-in skips on each runtime (`/tmp/momentum-views-final26.log`,
  `/tmp/momentum-views-final27.log`). The ordinary UI scheme skips all four 1,000-task
  benchmarks before fixture creation (0.175s, `/tmp/momentum-ui-performance-opt-in-final26.xcresult`).
- Shared compatibility passes: `cargo test --workspace --exclude momentum` succeeds
  across the Rust workspace and CLI (`/tmp/momentum-rust-workspace-final.log`), MomentumKit
  passes 136 tests/32 suites (`/tmp/momentum-macos-kit-final.log`), generated Apple core
  preparation passes, and the macOS Debug app build succeeds
  (`/tmp/momentum-macos-app-final.log`). B-012 and the cold keyboard cases B-045–B-047
  remain open; LibreSync remains user-deferred on iOS. B-038 follows below.

### 2026-09-17 — saved Nextcloud accessibility closure (B-038)

- The normal saved-connection audit was reproduced unchanged on iOS 26.5 and 27:
  one `.textClipped` finding whose element is unavailable. Screenshots show the
  complete wrapping Options footer and Connection Saved text. The captured hierarchy
  shows retained Form cells above the visible viewport; the saved node itself is intact.
- iOS 26 additionally reports contrast on Sync secondary labels even after they render
  through the verified 5.5:1 token. This is the exact runtime behavior established by
  B-053. iOS 27 reports no Sync contrast finding in the same final matrix.
- The normal-size regression now handles at most one unavailable clipping issue. Its
  iOS 26 path recognizes only the exact secondary labels in this form. Every labeled
  clipping finding, other label/type, hit-region issue and iOS 27 contrast issue still fails;
  native issue details and attachments are retained.
- Added an unfiltered English AX5 light regression with exact label and screen-bound
  assertions. It complements the existing unfiltered German AX5 dark case, whose longer
  content also remains complete. Final matrices pass all three cases on iOS 26.5
  (108.551s, `/tmp/momentum-sync-saved26-matrix-final.xcresult`) and iOS 27
  (109.295s, `/tmp/momentum-sync-saved27-final.xcresult`).
- B-038 is Verified for this simulator scope. Hosted Nextcloud/TLS, spoken VoiceOver,
  physical-device delivery and the combined Nextcloud checklist case remain open.
  B-012 follows below; keyboard cases B-045–B-047 remain independent parity blockers.

### 2026-09-17 — TaskEditor keyboard accessory geometry (B-012)

- Reproduced `Invalid frame dimension (negative or non-finite)` at field focus on
  iOS 26.5/27. Removing TaskEditor's complete keyboard toolbar removed the warning but
  intentionally broke the Done-control assertion. Text-only labels, removing the
  flexible spacer and replacing `ToolbarItemGroup` with one `ToolbarItem` all retained
  the warning. Conditional insertion moved it from sheet presentation to first focus.
  These probes were reverted; their logs/bundles remain under `/tmp/momentum-b012-*`.
- Root cause is the SwiftUI `.keyboard` toolbar placement while it inserts an accessory
  for the presented editor, not task state, validation, the SF Symbol or explicit app
  frame arithmetic. TaskEditor now presents the same Done action in a focused-field
  bottom safe-area bar with native bar material. The actual labeled button has a
  44-point minimum frame and remains above the keyboard.
- Production-source edit/copy acceptance passes without the warning on iOS 26.5
  (32.037s, `/tmp/momentum-b012-keyboard-layout26-final2.xcresult`) and iOS 27
  (33.339s, `/tmp/momentum-b012-keyboard-layout27-final.xcresult`). Each run asserts
  existence, hittability, 44-point height, keyboard separation, save/reopen and copy
  semantics. The iOS 26.5 screenshot was inspected and shows the SF Symbol Done capsule
  between the form and software keyboard.
- Refreshed generated-core preparation passes. Final mobile package suite passes
  119 tests/20 suites in 3.891s (`/tmp/momentum-fast-b012-final.log`). Native views pass
  44 tests with two intentional opt-in skips on iOS 26.5 and 27
  (`/tmp/momentum-views-b012-final26.log`, `/tmp/momentum-views-b012-final27.log`).
  B-012 is Verified for this simulator scope. Cold keyboard dispatch B-045–B-047,
  spoken VoiceOver, hosted Nextcloud/TLS and physical-device work remain open.
- The real Momentum Debug target then passed its generic iOS Simulator build through
  the generated-core gate (`/tmp/momentum-ios-shipping-final.log`). Preparation was
  refreshed afterward and the terminal fast run again passed 119 tests/20 suites in
  3.717s (`/tmp/momentum-fast-terminal.log`). Documentation links and whitespace pass;
  all native/build processes are terminal. No commit, push, device install or release.

### 2026-09-18 — Nextcloud lifecycle and backup parity closure (F-025/F-027–029, B-067)

- Scope is dirty `task/ios-app` at base `7ec82e93271fdddfae7dce5731bdec12a2ec141b`,
  simulator only. The user-deferred iOS LibreSync and physical-device work were not run.
  Hosted Nextcloud/TLS and spoken VoiceOver remain external, so F-025/F-020 stay open.
- Existing lifecycle coverage was inventoried before editing. The focused package lane
  passes 17 tests in two suites for fresh Off/isolated state, atomic Keychain records,
  read/write denial, corrupt credentials, provider drain, queued edits, restore exclusion,
  admission cancellation, persistent failure and expiration ownership. Native boundary
  suites pass 4/4 at
  `ios/DerivedData/TestReports/views-1789736380091620000.xcresult` and 6/6 at
  `ios/DerivedData/TestReports/views-1789736388981338000.xcresult`.
- The five-case real loopback WebDAV suite passes on iOS 27 at
  `ios/DerivedData/TestReports/transport-1789736405676139000.xcresult` and iOS 26.5 at
  `ios/DerivedData/TestReports/transport-1789736424838929000.xcresult`. It covers
  encrypted/compressed upload/download, offline edits, ETag retry, collection creation,
  cancellation with a concurrent local edit, credential/encryption correction,
  convergence and on-disk reopen without a hosted account.
- The shipping Nextcloud matrix passes all eight cases on iOS 26.5 at
  `/tmp/momentum-nextcloud-shipping26-r1.xcresult` (8/8 Success in the result database).
  On iOS 27, seven unchanged setup/status/recovery/AX5 cases pass in
  `/tmp/momentum-nextcloud-shipping27-r1.xcresult`; the one corrected normal audit passes
  separately in 20.762s at `/tmp/momentum-nextcloud-accessibility27-r5.xcresult`.
  B-067 gives `Connection` and `Options` primary ink and accepts XCTest's remaining exact
  `Connection` report only when an independent element raster measures at least 7:1.
- Backup package behavior passes 8/8 in 0.348s. The host sandbox cannot contact the
  Foundation file-coordination daemon, so the fast state test injects an ordinary real
  file read; the production `NSFileCoordinator` path is exercised by the simulator.
  Native integration passes 4/4 on iOS 27
  (`ios/DerivedData/TestReports/views-1789738993912808000.xcresult`) and iOS 26.5
  (`ios/DerivedData/TestReports/views-1789739009172408000.xcresult`). Shipping Files
  export/cancel/import/confirm/replace/relaunch passes 2/2 on iOS 27 in 79.381s
  (`/tmp/momentum-backup27-r1.xcresult`) and iOS 26.5 in 70.445s
  (`/tmp/momentum-backup26-r1.xcresult`). Only disposable stores and local simulator
  providers were used; no device, personal account, commit, push or release was involved.

### 2026-09-18 — automation, Spotlight and notification-action closure (F-024/F-031/F-040, B-068)

- Scope is dirty `task/ios-app` at base `7ec82e93271fdddfae7dce5731bdec12a2ec141b`,
  simulator only. No physical device, personal store/account, commit, push or release
  was used. Eleven focused automation/URL/notification package tests pass in three
  suites; automation and Spotlight native integration pass 10/10 and 6/6 on iOS 27.
- The first iOS 27 production automation run passes all five Shortcuts cases but sends
  its Safari URL into the installed sync QA host. B-068 gives that target an explicit
  fixture plist with no production URL schemes while retaining the public task UTType.
  The exact corrected URL flow passes 1/1 on iOS 27
  (`/tmp/momentum-url27-collision-fix-r1.xcresult`) and iOS 26.5
  (`/tmp/momentum-url26-collision-fix-r2.xcresult`). Installed-bundle readback confirms
  the QA host has no `CFBundleURLTypes`.
- `MomentumIntentTests` passes 1/1 on iOS 27 at
  `/tmp/momentum-intents27-r1.xcresult`, executing Create, Find, Complete, Reopen,
  Plan Today and Open through `AppIntentsTesting` against the shipping app. A clean
  iOS 26.5 Shortcuts run reaches the action tiles, but `linkd` rejects the ad-hoc
  Customer OS client with `requiresValidBundle`; its parameter UI never appears.
  Shared/package/view behavior and production URL routing still pass, so this is an
  Apple framework/signing limitation rather than a reproduced product defect.
- On each runtime, `MomentumSpotlightTests` passes 1/1 protected-index mutation,
  `MomentumSpotlightActivationTests` passes 2/2 cold/warm routing and
  `MomentumSpotlightSystemUITests` passes 1/1 live SpringBoard result activation.
  Corrected activation bundles are `/tmp/momentum-spotlight-activation26-r2.xcresult`
  and `/tmp/momentum-spotlight-activation27-r2.xcresult`; both have zero runtime warnings.
- The focused notification lane passes 36 tests in four suites in 0.472s. Shipping
  notification delivery passes 3/3 on iOS 27 in 328.420s
  (`/tmp/momentum-notification-actions27-r1.xcresult`) and iOS 26.5 in 343.203s
  (`/tmp/momentum-notification-actions26-r1.xcresult`). It covers cold body activation,
  cold Done and resident Snooze through real SpringBoard delivery; portable tests prove
  single-owner deduplication, shared auto-archive and exact one-hour rearming. Locked
  device, Focus, suspended background execution and physical-device behavior remain
  outside this simulator evidence and are user-deferred.

### 2026-09-18 — XCUI removal and fast code-plus-SwiftUI unit gate (F-039)

- Per the user's testing decision, every iOS UI-testing bundle, source tree and
  XCUITest-only QA host was removed: `MomentumUITests`, `MomentumSyncUITests`,
  `MomentumIntentTests` and `MomentumSyncTestHost`. `project.yml` now generates only
  the app, minimal unit host, view unit target and transport unit target, with five
  app/unit schemes. The ordinary `--uitesting` launch path was also removed.
- `ios/scripts/test.py unit` is the quick combined gate. It runs portable Swift code
  tests and the native rendered-view unit target serially, requires an iOS Simulator
  destination and rejects physical devices. The runner's ten host-only regressions
  pass and include a project-shape guard that fails on a UI-testing bundle,
  `XCUIApplication` usage or a retired XCUI source directory.
- Production SwiftUI coverage uses public `UIHostingController` APIs, the real app
  sources and isolated UUID stores/preferences. The fixture now overrides light/dark
  and normal/increased contrast traits at the controller boundary, preventing retained
  simulator accessibility settings from changing pixel assertions. It contains no
  ViewInspector, private reflection, app launch, arbitrary sleeps or snapshot goldens.
  New coverage verifies the four requested tabs and stable SF Symbols/identifiers,
  real compact-width German AX5 Settings wrapping, and white action ink over the
  exact default `#FF6600` in normal contrast. Existing semantic tests retain the
  calculated Increase Contrast exception.
- The combined lane passes on iOS 26.5: 126 package tests/21 suites in 4.052s test time
  and 8.741s whole command, followed by 49 native tests with two intentional opt-in
  performance skips, zero failures, 2.905s test time and 9.869s whole command. Native
  result: `ios/DerivedData/TestReports/views-1789747724157923000.xcresult`.
- The same lane passes on iOS 27: 126 package tests/21 suites in 3.924s test time and
  4.926s whole command, followed by 49 native tests with the same two intentional skips,
  zero failures, 2.976s test time and 6.690s whole command. Native result:
  `ios/DerivedData/TestReports/views-1789747741582779000.xcresult`.
- The separate real loopback transport unit target remains green: 5/5 on iOS 26.5 in
  12.455s (`ios/DerivedData/TestReports/transport-1789747773125773000.xcresult`) and
  5/5 on iOS 27 in 11.895s
  (`ios/DerivedData/TestReports/transport-1789747788019261000.xcresult`). Core
  preparation, XcodeGen, string-catalog JSON parsing and `git diff --check` pass.
- The generated `Momentum` Debug app also builds successfully for the generic iOS
  Simulator destination after the target removal, including all four Rust/UniFFI
  slices, production resources, Icon Composer asset and App Intents metadata. The
  app build refreshed generated core artifacts; a new `prepare` pass and terminal
  iOS 27 `unit` run then pass 126/126 package and 49/49 native tests (two opt-in
  skips) at `ios/DerivedData/TestReports/views-1789748018445130000.xcresult`.
- Historical shipping/system results remain recorded, but they are no longer an active
  automated regression lane. Gestures, whole-app keyboard dispatch, external-app and
  system-prompt flows, notification delivery and spoken assistive technology require
  explicit manual/system evidence. B-045–B-047 remain open; removing their synthetic
  key-input tests does not establish a product fix. All work used simulators only;
  dip17pm, personal stores/accounts, commits, pushes and releases were excluded.

### 2026-09-18 — prioritized iOS sync, accessibility and energy pass (F-020/F-025/F-041, B-069/B-070)

- Scope is the dirty local `task/ios-app` worktree at base
  `7ec82e93271fdddfae7dce5731bdec12a2ec141b`, simulator only. The requested order was
  Nextcloud, iPhone/iPad Dynamic Type and accessibility, drag/drop, background
  notifications, Low Power Mode, then system automation boundaries. No physical device,
  hosted account, personal store, dependency, commit, push or release was used.
- B-069 closes the unsafe save boundary: iOS validates and normalizes server/user/folder
  input before drain or Keychain replacement, preserves password whitespace, requires
  HTTPS outside isolated loopback, rejects embedded credentials/query/fragment data and
  prevents invalid legacy records from automatic transport. The previous valid record
  survives a rejected draft. Field-specific error UI wraps at AX5 phone/iPad widths.
- B-070 remediates audit candidate A-05 in source: async Quick Add, task, recurrence and
  project-move failures use a shared semantic error view and receive accessibility focus;
  the estimate field retains correction focus with an explicit hint. Spoken VoiceOver,
  rotor order, Voice Control and Switch Control remain manual acceptance boundaries.
- F-011–F-013 drag/drop source and prior simulator evidence were re-audited with no new
  implementation gap: single/multi payloads, row reorder, project/tag/day destinations,
  archive rejection, external text/URL import and tap/custom-action alternatives remain.
  Real cross-app payload and assistive Move Up/Down invocation stay manual.
- F-007/F-038 background notification replenishment was already implemented as a bounded,
  local-only `BGAppRefreshTask`; its portable policy, expiration and production build
  remain green. Simulator cannot establish a system-launched suspended execution, so the
  existing physical/system acceptance row remains unchecked.
- F-041 now observes `NSProcessInfoPowerStateDidChange` and the current system value.
  Low Power Mode pauses automatic Nextcloud, Spotlight and discretionary notification
  refresh work; manual Sync Now, local editing, existing local notifications and an
  admitted exchange remain available. Automatic work resumes after the mode turns off.
  This follows Apple's documented guidance to reduce discretionary/background activity.
- F-031/F-040 automation boundaries were re-audited: App Intents require authentication,
  share the one engine actor, validate stale mutation batches atomically, defer Open over
  drafts/sheets and keep QA URL ownership isolated. Existing iOS 27 system evidence and
  the iOS 26 ad-hoc `linkd` limitation remain accurately recorded; no duplicate test
  harness or production behavior change was needed.
- Verification: focused TDD failures were observed before each Nextcloud, error-view and
  power-policy implementation. Final `unit` passes 136 package tests/21 suites plus 52
  native tests with two intentional performance skips on iOS 27 at
  `ios/DerivedData/TestReports/views-1789754952098471000.xcresult` and iOS 26.5 at
  `ios/DerivedData/TestReports/views-1789754563510639000.xcresult`. The five transport
  tests pass on iOS 27 at `transport-1789754174230200000.xcresult` and iOS 26.5 at
  `transport-1789754194672519000.xcresult`. The production generic simulator app builds
  successfully, including App Intents metadata and the power-state observer.
- Remaining: hosted Nextcloud/TLS, spoken/alternate-input accessibility, real external
  drag, system-triggered background refresh, a real Low Power Mode toggle, locked/system
  automation and the explicitly deferred iOS LibreSync scope. These prevent a complete
  parity claim even though the requested production code boundaries are now present.

### 2026-09-18 — resumed iOS parity boundaries (F-007/F-014/F-015/F-023/F-025–F-028/F-031/F-041, B-071–B-073)

- Scope is the dirty local `task/ios-app` worktree at base
  `7ec82e93271fdddfae7dce5731bdec12a2ec141b`, with simulators only. The user resumed
  LibreSync and the remaining parity work, while explicitly excluding spoken VoiceOver
  acceptance and a real Low Power Mode toggle. No physical device, personal store,
  hosted credential, commit, push or release was used.
- iOS now exposes Off, Nextcloud and LibreSync in one provider selector. LibreSync has
  native status/retry, manual Sync Now, this-device identity, temporary six-digit pairing,
  discovery, linked-device management and unlink confirmation. Provider, foreground,
  background and restore transitions serialize stop/drain/start through the one worker;
  local edits coalesce before a nearby exchange. Real disposable Swift/UniFFI/Rust peers
  verify pairing, two-way convergence, callbacks, restart, stable identity/certificate
  reuse and unlink on iOS 26.5 and 27. Separate-device Bonjour interaction remains open.
- B-072 protects Quick Add marked text from SwiftUI reconciliation while an IME owns the
  composition range. Native `NSItemProvider` text and URL inputs also pass through the
  production external-drop `Transferable`; the cross-app spatial gesture remains a
  manual system boundary.
- The production background-refresh handler now has direct success and expiration tests:
  it installs expiration before work, cancels expired work, recomputes from fresh state,
  reschedules and reports truthful completion. Foreground authorization refresh covers
  denied-to-authorized recovery. Simulator cannot originate a suspended scheduler wake.
- An opt-in hosted Nextcloud TLS test opens two isolated workers against disposable HTTPS
  credentials, uploads/downloads a unique task, deletes it and reconverges. It correctly
  skipped in this run because no `MOMENTUM_NEXTCLOUD_TEST_*` credentials were configured;
  the existing loopback protocol matrix remains green.
- B-073 adds SwiftUI key-event fallbacks for cold Search, Select All and configured-modifier
  Delete through the existing action contexts. Wrong modifiers and editor/sheet contexts
  return ignored so native text and system handling continue. Direct unit dispatch passes;
  a manual Simulator keyboard attempt could not establish reliable window focus and is
  not counted as product evidence, so B-045–B-047 remain In progress.
- Voice Control and Switch Control retain visible tap alternatives, stable labels and named
  Move Up/Down actions. Live system activation and focus-order acceptance remain manual.
  App Intents continue to require authentication and use the shared engine actor; a locked
  system invocation cannot be forced by the current simulator unit harness.
- Final verification: `unit` passes 140 portable tests/21 suites plus 59 native tests with
  two intentional opt-in performance skips on iOS 27 at
  `ios/DerivedData/TestReports/views-1789758914948973000.xcresult`. The same 140 + 59
  matrix passes on iOS 26.5 at
  `ios/DerivedData/TestReports/views-1789758459331313000.xcresult`. The six-case transport
  target passes on both runtimes with one expected hosted-TLS skip and no failures at
  `transport-1789758117133744000.xcresult` and
  `transport-1789758477585104000.xcresult`. Core preparation and the production generic
  iOS Simulator app build pass after the final source changes, including App Intents
  metadata. German localization is complete at 351/351; the ten runner regressions,
  Markdown link scan and `git diff --check` pass.
- Remaining acceptance is system/environmental: hosted Nextcloud TLS, separate-device
  Bonjour and pairing rejection/expiry UI, a spatial cross-app drag, external hardware-key
  delivery, live Voice Control/Switch Control, scheduler-originated background launch and
  locked-device intent invocation. VoiceOver and the real Low Power Mode toggle are outside
  the user-approved completion scope. These limits prevent an unqualified all-system-boundary
  parity claim, while the requested production code and fast regression boundaries exist.

### 2026-09-18 — dip17pm installation and version metadata correction (F-039/B-074)

- At the user's explicit request, built the current dirty `task/ios-app` source for the
  paired physical dip17pm with automatic development signing and installed only the
  production Momentum target. No test suite, personal task inspection or app interaction
  was performed.
- The first successful device readback exposed B-074: `Info.plist` reported hardcoded
  version `1.0` while the project declares `0.4.0`. Version keys now use Xcode's
  `MARKETING_VERSION` and `CURRENT_PROJECT_VERSION` settings. The corrected signed build
  was installed over the first copy and device inventory reports `0.4.0` build `1` for
  `com.codedbydan.Momentum.ios`.

### 2026-09-18 — iOS navigation and interaction fixes (F-003–F-006/F-010/F-018–F-020/F-023/F-025/F-028/F-037, B-075–B-082)

- The ten requested fixes are implemented in the dirty local `task/ios-app` worktree at
  base `7ec82e93271fdddfae7dce5731bdec12a2ec141b`. Work stayed simulator-only; dip17pm,
  personal stores/accounts, commits, pushes and releases were excluded.
- Today now uses an adaptive leading `NavigationSplitView`; compact selection returns to
  detail and regular containers at least 700 points wide keep a 280-point sidebar beside
  a 420-point accessible detail column. The first task snapshot uses an immediate
  transaction and stable bottom inset, so the floating Add Task control does not animate
  from its loading position. Selection mode removes the completion control.
- Quick Add returns the created identity atomically and routes single-line creation, or
  the last multiline creation, through the root-owned standard editor without presenting
  two sheets. Sync Settings is observer-only, keyboard discovery and fallback commands
  are iPad-only, and the accent row's live resolved ID owns its checkmark and selected
  accessibility trait.
- Restrained selection/success haptics and 180 ms transitions now cover Add, creation,
  sidebar/accent choice, selection and changed task movement. Reduce Motion uses short
  opacity-only or immediate changes. Morning & Night grouping is limited to day views in
  the shared core, so tomorrow tasks retain their due-day metadata outside Today instead
  of appearing under a false Today heading.
- Static iOS 27 simulator inspection covered light, dark, Increase Contrast and the
  largest accessibility text size. The default `#FF6600` action uses white text in normal
  contrast in both appearances and calculated black text only with Increase Contrast;
  the main background is continuous. The locked Mac prevented the remaining touch,
  transition, rotation and back-navigation interaction matrix, so those limits remain in
  B-075–B-081.
- A final test with the simulator left at AX XXXL exposed that mounted view tests inherited
  mutable simulator Dynamic Type. `HostingFixture.mount` now pins `.large` by default,
  while tests that exercise accessible sizes opt in explicitly. The complete unit gate
  then passes unchanged at AX XXXL on iOS 27. After the final production app build and
  core preparation, the terminal gate passes 152 portable tests and 80 hosted SwiftUI
  tests with two intentional opt-in skips at
  `ios/DerivedData/TestReports/views-1789784223799710000.xcresult`. The same 152 + 80
  matrix passes on iOS 26.5 at
  `ios/DerivedData/TestReports/views-1789784255924727000.xcresult`.
- Both shared Rust workspace commands pass, including the 23-test CLI suite and the new
  cross-view grouping regression; one manual timing sample remains intentionally ignored.
  MomentumKit passes 136 tests in 32 suites with its opt-in nearby test skipped. Final
  independent spec/code review found no blocking or material issue. Linux native runtime
  verification remains unavailable because Flatpak is not installed on this Mac.
