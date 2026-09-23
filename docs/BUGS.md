# Momentum bug ledger

Updated: 2026-09-18. Use the platform status vocabulary in [AGENTS.md](../AGENTS.md).
For each bug, also state impact: reproduced, reported, suspected, not reproduced,
unaffected, or unknown. A status alone must not imply that a bug reproduced on that OS.
Keep resolved entries. Verification gaps without a known product defect belong in
[PROGRESS.md](PROGRESS.md), not as invented bugs here.

Severity: Critical = data loss/security/core unusable; High = major workflow impaired;
Medium = meaningful defect with workaround; Low = minor presentation/polish. Assess
severity from evidence; do not infer security or data loss from an ordinary sync failure.

## B-001 — Linked LibreSync devices do not populate the Mac

- **Severity:** High. **Feature:** [F-026](FEATURES.md).
- **Reported behavior:** Linux receives data but the linked macOS app does not show it.
- **Expected:** valid changes from the selected LibreSync peer reach the shared store
  and refresh the native UI, or an actionable connection error explains the failure.
- **Reproduction:** link Linux/Mac, select LibreSync, create a task on Linux, trigger
  sync and inspect the Mac. Real-device acceptance is currently deferred by the user.

| Linux | macOS | iOS | Android |
|---|---|---|---|
| Blocked — receiving worked per user; sending/peer connectivity not fully verified | Implemented — receiving symptom reported; changes made, physical reproduction/acceptance deferred | Not applicable — app not implemented | Not applicable — app not implemented |

- **Findings/fix:** initial bootstrap must remain pending until a valid snapshot;
  bridge progress/failures must reach native state; disposable demo data must never
  sync. These fixes and isolated regressions are recorded in the
  [Mac acceptance history](MACOS-MVP-CHECKLIST.md). A linked peer also timed out over
  the network; the root cause of the original real-device symptom is not conclusively
  proven by passing loopback tests.
- **Evidence:** prior two-node Swift/UniFFI integration checks and shared-core
  regressions passed; details and dates are in that history. This documentation pass
  did not rerun them.
- **Next:** resume only when the user lifts the real-device deferral; verify both
  directions with isolated test data and confirm UI state plus durable-store readback.

## B-002 — macOS quick-add consumes multiline paste as one input

- **Severity:** Medium. **Feature:** F-015. **Resolved scope:** native Mac quick-add.
- **Reproduction:** paste two task lines into quick-add. Previously the native field
  consumed Paste before the SwiftUI handler and retained newlines in a single input.
- **Expected:** create separate tasks for multiline input while ordinary paste replaces
  the native selection without corrupting cursor/selection behavior.

| Linux | macOS | iOS | Android |
|---|---|---|---|
| Not applicable — this reproduced defect was in the AppKit text-field path | Verified — fixed and live-tested 2026-09-16 | Not applicable — app not implemented | Not applicable — app not implemented |

- **Fix:** observe multiline text changes in native quick-add and route them through
  shared text import; retain the editor's normal single-line behavior.
- **Evidence:** focused import regression, successful Debug build, and native
  two-task creation plus ordinary selection replacement recorded under “F5 and native
  paste” in [MACOS-MVP-CHECKLIST.md](MACOS-MVP-CHECKLIST.md).
- **Regression scope:** the common parsing contract applies to all future frontends;
  the Not applicable cells do not waive F-015 for mobile.

## New bug template

Use `B-<next ID>` with: severity; linked feature; report date; environment/revision;
reproduction steps; expected versus actual; separate status and impact for each OS;
cause (or explicitly unknown); fix; test/build/native evidence; remaining checks and
next action. Record a blocker and last completed stage rather than closing an issue
because its reproduction environment is unavailable.

## 2026-09-16 — F-037 regression assessment

Group By introduces no newly observed defect in the existing bug entries. Shared
regressions cover task preservation, parent/subtask placement, archive/search limits,
and reorder undo; native Mac checks cover menu changes and colored headings. Linux
runtime and additional grouped native drag/accessibility checks remain verification
gaps in [PROGRESS.md](PROGRESS.md), not assumed bug fixes. The existing LibreSync and
Spotlight deferrals are unchanged.

## B-003 — Dual time-of-day tags prevent a move to Evening

- **Severity:** Medium. **Feature:** F-037. Reproduced in the shared Rust core during
  the exclusive-grouping revision on 2026-09-16.
- **Reproduction:** create a task tagged both Morning and Evening. Morning takes
  precedence when displaying it. Drop it onto Tonight, or use the Evening toggle.
- **Expected:** it moves into Evening, removing opposing Morning tags; undo restores
  the original tags and grouping.
- **Actual:** drop was treated as already in Evening because the raw tag existed;
  toggle removed Evening while leaving the task in Morning.
- **Cause/fix:** use the resolved day period for drop/toggle decisions, and remove
  all matching opposing tag IDs by case-insensitive name. Listing, sidebar membership
  and grouping resolve the same names and Morning precedence.
- **Platforms:** Linux/macOS Implemented (shared-core reproduction and regression);
  iOS/Android Planned (consume the same core when native apps exist).
- **Evidence:** regression covers both-tag precedence, mixed-case names, parent/subtask
  placement, forced drop, toggle, regrouping and undo. Native gesture acceptance is not
  inferred from core tests; see the dated verification in PROGRESS.md.

## 2026-09-16 — F-038 regression assessment

The automatic morning summary is now an explicit opt-in feature (F-038), replacing
its prior specified behavior rather than treating it as a newly reproduced bug.
Regression coverage preserves task reminders, suppresses summaries while off, and
prevents duplicate daily claims after restart or preference edits. Mac adapter gating
also prevents consuming a summary before a notification adapter is attached. Existing
bug statuses and user-deferred checks are unchanged; native verification limits remain
in [PROGRESS.md](PROGRESS.md).

## 2026-09-16 — F-019 macOS task-row alignment

User-requested visual refinement: the row previously aligned its checkbox with the
first text baseline. It now centers the checkbox, archived checkmark and trailing
badges against the full title/metadata stack. This is a macOS-only presentation change;
no shared task rules or other platforms changed. Build and native two-line-row
inspection passed; no additional functional defect was observed.

## B-004 — iOS quick entry crashes on stale text-selection indices

- **Severity:** High. **Features:** F-005/F-015. **Impact:** reproduced on the iOS 27
  simulator in the new dirty iOS implementation; Linux/macOS paths are unaffected,
  Android has no corresponding native implementation.
- **Reproduction:** open Add task while SwiftUI publishes a text selection belonging
  to a different text update. The autocomplete cursor calculation used that index
  directly in the current string's Unicode-scalar view and trapped on the main thread.
- **Expected:** quick entry remains usable across focus changes, text replacement,
  emoji, selection and IME updates; native editing retains its cursor.
- **Cause/evidence:** both 2026-09-16 native runs produced EXC_BREAKPOINT in
  `_StringGuts.validateInclusiveScalarIndex` → `QuickAddSheet.cursor`. Result bundle
  `/tmp/momentum-ios-editor-red-v2.xcresult`; isolated test store, base `7ec82e9` dirty.
- **Fix:** validate index bounds and convert with `samePosition(in:)` before scalar
  distance; fall back to the end when the selection is stale. Added stable field
  identifier and a Unicode replacement native regression.
- **Status:** iOS Implemented, with scoped iOS 27 native regression verified by
  `testQuickAddReplacesUnicodeTextWithoutCrashing` in
  `/tmp/momentum-ios-editor-green.xcresult` (2026-09-16, same dirty worktree).
  iOS 26 and IME acceptance remain outstanding.

## B-005 — iOS palette labels lose contrast in native rendering

- **Severity:** Medium. **Features:** F-019/F-020. Reproduced in isolated iOS 26.5
  and 27 simulator acceptance on 2026-09-16, dirty `task/ios-app` at `7ec82e9`.
- **Reproduction:** choose a custom accent in Settings → Accent Color. The native
  button style tinted primary/secondary label hierarchies; small hex values became
  pale. The floating tab bar's soft scroll-edge treatment also faded nearby rows.
- **Expected:** readable color names, values and controls in light/dark/increased
  contrast, with selection conveyed by a checkmark and accessibility state.
- **Fix in progress:** explicit system primary text; contrast-adjusted secondary
  text and swatch outlines; native hard top/bottom scroll edges. Exact base values remain
  unchanged, while rendered tones adapt against native surfaces.
- **Evidence:** palette-wide numerical tests pass. Native screenshot/audit in
  `/tmp/momentum-ios26-accent-stable.xcresult` identifies the lower “Wet Asphalt”
  label. Subsequent hard-edge capture shows the label clearly but the audit still
  flags partially occluded content. Fully visible preview and appearance checks
  remain in progress; this is not a blanket accessibility acceptance.
- **Platforms/status:** iOS Implemented with verification outstanding. Linux/macOS
  unaffected by this SwiftUI mobile palette path; Android has no corresponding UI.

## B-006 — iOS Repeat and Duplicate ignore unsaved task edits

- **Severity:** Medium. **Features:** F-006/F-008. Reproduced in iOS 27 native tests,
  dirty `task/ios-app` at `7ec82e9`, 2026-09-16.
- **Reproduction:** change a task title, then choose Repeat or Duplicate before Save.
  Previously the action used the old persisted title; cancelling the outer editor
  discarded the draft while leaving the newly created repeat/duplicate behind.
- **Fix:** validate and save the current draft first, refresh stored form/tag values,
  then perform the action. Footers explain the save-first behavior. Invalid forms
  remain disabled and errors keep the editor open.
- **Evidence:** both action regressions failed before the fix and passed afterward
  in `/tmp/momentum-ios-editor-draft-green.xcresult`; five editor tests and one launch
  test passed. Its separate accent audit failed (B-005), so the combined run is not
  reported as green. Independent source review passed.
- **Platforms/status:** iOS Implemented; scoped iOS 27 native regression verified,
  iOS 26 still pending. Desktop paths unaffected; Android not implemented.

## B-007 — iOS task drag type is not declared in the app manifest

- **Severity:** Low. **Features:** F-011/F-014. The iOS 26.5 native run logs that
  `com.codedbydan.momentum.tasks` is exported by the shared Transferable type but
  absent from the app manifest when a draggable row appears. Actual gesture failure
  has not been reproduced; macOS already declares this type, Linux is unaffected,
  and Android has no corresponding native implementation.
- **Fix:** mirror the existing macOS exported type declaration in the iOS XcodeGen
  specification; retain the shared payload and public-data conformance.
- **Status/evidence:** iOS Implemented, dirty `task/ios-app`, 2026-09-16;
  warning in `/tmp/momentum-ios26-accent-hard-edges.log`. The corrected simulator
  app built and its exported-type declaration was read back from the built Info.plist
  (`/tmp/momentum-ios26-accent-readable-headings.log`). Native drag/drop acceptance
  remains outstanding.

## B-008 — iOS Quick Add dismisses after an unsuccessful mutation

- **Severity:** Medium. **Features:** F-002/F-005. iOS source-confirmed: the capture
  sheet unconditionally dismissed after awaiting add/import, ignoring Outcome.changed.
  Desktop consumers are not changed; their analogous behavior was not reproduced.
- **Expected:** keep entered text and show actionable failure feedback when no task
  was saved. **Fix:** native model exposes the core mutation result, Quick Add only
  dismisses on change, restores focus and retains draft text otherwise. No task rules
  were duplicated in Swift. **Status:** Implemented; scoped iOS 27 rejection/retry and successful-save native
  regressions passed in `/tmp/momentum-ios-flow-polish-v2.xcresult` on 2026-09-16,
  dirty `task/ios-app` worktree. Test uses an estimate-only input rejected by the real
  core, verifies the retained draft and retries successfully. The same regression also passed in the full 10-test iOS 26.5 dark-mode suite
  (`/tmp/momentum-ios26-flow-polish.xcresult`). Disk-failure injection remains separate.

## B-009 — iOS toolbar overrides a filled action's contrasting symbol color

- **Severity:** Medium. **Features:** F-019/F-020. Reproduced by native screenshot
  inspection on iOS 26.5 dark mode after the 10-test flow suite passed. Edit any task:
  Save's checkmark is pale on #FF6600 despite a black foreground style on the Button.
- **Expected:** readable contrasting ink on the filled Create/Save action while
  retaining the exact dark default accent. **Cause:** native toolbar promotion can
  override foreground styling on the enclosing Button.
- **Fix:** explicit palette rendering/foreground on the action image, retaining the
  accessible localized button label and native prominent style. **Status:**
  Verified for the iOS 26.5 dark editor/commit scope: final build and two native
  capture/retry + edit/delete/undo tests passed (`/tmp/momentum-sheet-ink-fix.xcresult`),
  and screenshot inspection confirms a black checkmark against the orange
  fill. iOS 27 light recheck also passed in `/tmp/momentum-sheet-ink-ios27.xcresult`;
  screenshot inspection confirms white ink on the darkened orange. Original source screenshot:
  `/tmp/momentum-ios26-polish-evidence/04A74F39-4FC5-412A-B16E-C9CFCAFACFEC.png`.
- iOS-specific toolbar component; other frontends do not use this implementation.


## B-010 — stale notification Snooze mutates completed tasks

- **Severity:** Medium. **Features:** F-007. Reproduced with a shared-core regression:
  create a task, snooze it, complete it, then invoke Snooze from the stale notification.
  Before the fix, the operation reported success and changed the reminder timestamp.
- **Expected:** completed, archived, deleted or missing tasks reject Snooze without
  changing their reminder. **Cause:** the shared operation checked existence only.
- **Fix:** the Rust engine rejects completed tasks, retaining the existing missing/
  archived guard. Applies to Linux, macOS and the new iOS notification action adapter;
  no frontend duplicates task eligibility rules. Android has no consumer yet.
- **Evidence:** failing regression observed before the guard; full all-feature/all-target
  core/CLI workspace checks passed afterward (excluding GTK app), and final focused
  regression covers actual deletion too. Logs `/tmp/momentum-notification-native-core.log`
  and `/tmp/momentum-stale-snooze-final.log`, 2026-09-16 dirty `task/ios-app`.
  Native stale-notification interaction on each OS remains unverified.

## B-011 — disabling a summary while permission is denied leaves its pending request

- **Severity:** Medium. **Features:** F-007/F-038. iOS coordinator regression:
  a future accepted summary exists, OS permission is denied, then summary is disabled.
  The unauthorized branch only reported the plan and skipped its cancellations.
- **Expected:** remove the exact obsolete request without adding notifications,
  requesting permission, or consuming a core scheduling claim. **Fix:** apply and
  verify the read-only plan's cancellations even when scheduling is unauthorized.
- **Evidence:** regression failed before the change (`/tmp/momentum-notification-denied-red.log`);
  all 52 mobile tests / nine suites passed afterward (`/tmp/momentum-notification-final-mobile.log`),
  2026-09-16 dirty `task/ios-app`. On 2026-09-18, shipping-app simulator tests on
  iOS 26.5/27 denied real authorization and verified the disabled recovery UI; separate
  fresh-grant runs verified `Allowed` plus the native planning horizon. The current
  Simulator exposes no notification service through `simctl privacy`, and its Settings
  switch rejected cross-app XCTest input, so the uninterrupted denied-to-enabled
  transition remains manual acceptance. This coordinator is iOS-specific; desktop
  reminder delivery uses separate adapters.

## B-012 — task editor presentation emits an invalid-frame warning

- **Severity:** Low; runtime diagnostic reproduced, visible impact not established.
  **Features:** F-020. Opening task editors in native iOS 26.5/27 tests emits
  `Invalid frame dimension (negative or non-finite)` during sheet presentation.
- The warning predates notification wiring: it is present in
  `/tmp/momentum-ios26-flow-polish.log` and `/tmp/momentum-sheet-ink-ios27.log`, and
  recurs in `/tmp/momentum-notification-final27.log`. Recorded editor interactions
  and inspected screenshots have not demonstrated a related functional failure.
- **Cause:** SwiftUI's `.keyboard` toolbar placement emits the warning as soon as
  any keyboard toolbar item is inserted for the presented editor. Removing the whole
  keyboard toolbar removes the warning; removing its symbol, spacer or item group does
  not. Conditional insertion moves the warning to the first field focus, confirming
  the framework accessory path rather than task data or sheet geometry.
- **Fix:** replace the `.keyboard` toolbar with a keyboard-aware bottom safe-area bar
  inside TaskEditor. It appears only while a field is focused, retains the SF Symbol
  Done action, uses native bar material, and gives the actual button label a 44-point
  minimum accessibility frame.
- **Status/evidence:** Verified for the iOS 26.5/27 simulator scope. The real
  open/focus/edit/save/reopen/copy flow passes without the warning on iOS 26.5
  (32.037s, `/tmp/momentum-b012-keyboard-layout26-final2.xcresult`) and iOS 27
  (33.339s, `/tmp/momentum-b012-keyboard-layout27-final.xcresult`). Assertions prove
  the action exists, is hittable, is at least 44 points tall and stays above the
  software keyboard. The retained iOS 26.5 screenshot was inspected. Final portable
  and native-view suites pass. macOS/Linux are unaffected; Android has no UI consumer.

## 2026-09-16 — F-019 curated accent assessment

The user requested AsNeeded’s nine-color selection instead of the full country-based
DHFlatUIColors catalog. This is an iOS product refinement, not a new reproduced defect.
Existing default/contrast safeguards remain; regression checks cover retained and retired
saved choices. macOS/Linux use their platform accents and are unaffected. Native evidence
for the reduced picker is recorded in [PROGRESS.md](PROGRESS.md).

## 2026-09-16 — exact default orange decision (F-019; B-005/B-009 scope)

The user explicitly replaced light-mode darkening with exact #FF6600 in both
appearances. Adaptive custom-color and neutral-text fixes remain; default-orange
text on light backgrounds no longer carries the prior contrast-pass claim. Filled
Create/Save symbols now derive black/white ink from the actual rendered accent
instead of assuming white in light mode. This preserves B-009’s toolbar-level
foreground fix while accommodating the brighter default. No new reproduced defect.

## 2026-09-16 — iOS accessibility audit findings

All entries below are from source inspection and isolated native simulator audits
of dirty `task/ios-app` based on `7ec82e9`. No production fix is included in this
audit change. The [audit report](audits/2026-09-16-ios-accessibility.md) records raw
results, screenshots, reproduction, candidates and remaining assistive testing.
Linux/macOS were not reassessed; these iOS findings do not change their statuses.
Android is Planned and has no implemented UI to reproduce these defects.

## B-013 — exact default orange has insufficient foreground contrast

- **Severity:** High. **Features:** F-018/F-019/F-020. **iOS:** In progress remediation.
- **Reproduce:** default accent, light appearance; inspect selected picker values,
  tab text and tint-colored actions. #FF6600 against white is 2.936:1, below
  Apple's text guidance. The default also remains exact with Increase Contrast.
- **Cause:** the latest user decision intentionally keeps the base orange exact;
  using the same token for all foreground roles creates the contrast deficit.
- **Expected/fix:** retain the approved brand/fill color while defining readable
  semantic foreground roles consistent with the latest approved button styling. No silent
  reversion to the previously rejected darker orange. See A-01.
- **2026-09-17 user steering:** All default-orange filled buttons now use white text and
  symbols, including Create/Save and every floating Add action, in both appearances
  with calculated contrasting ink as the sole exception when Increase Contrast is
  enabled. This supersedes the earlier Today-only white styling. White on #FF6600 is 2.936:1; the requested
  styling is not an accessibility contrast remediation. Custom accents retain
  calculated ink. The selected default-orange palette swatch follows the same policy.
- **Remaining:** foreground contrast findings remain open under the approved exact
  orange/white policy. Native trait and rendered-button regression tests verify styling,
  not an accessibility contrast pass.

## B-014 — largest text truncates Settings values and iPad empty-state title

- **Severity:** High. **Features:** F-019/F-020. **iOS:** Planned remediation.
- **Reproduce:** AX5, Task Lists settings in English/German; selected grouping and
  sort values truncate. iPad mini Today also truncates its empty-state heading;
  its AX5 editor Save glyph extends outside the filled action. Full-width iPad
  Settings values fit, so phone picker truncation is not reproduced there.
- **Expected:** full meaningful values/headings with wrapping/scrolling at large
  text. Native menu values remain full in the accessibility tree.
- **Cause/fix:** menu value/empty-state layout does not provide full visible text
  at AX5; adapt the layout without shrinking or capping Dynamic Type. See A-07.
- **2026-09-17 partial remediation:** fresh shipping-app regressions reproduce
  iPadOS 27 empty-title clipping (`/tmp/momentum-ax5-empty-ipad27-before.xcresult`)
  and iOS 26.5 Settings clipping (`/tmp/momentum-ax5-settings26-before.xcresult`).
  The empty title now uses an explicitly wrapping label inside ContentUnavailableView.
  Strict native clipping checks pass on iOS 26.5/27 phone and iPad mini simulators;
  dated results are in PROGRESS. The inspected iPad screenshot shows the complete
  two-line title. This check covers the English portrait heading, not every empty
  state, locale, orientation or spoken VoiceOver interaction.
- **AX5 description reachability:** separate shipping-app tests pass on iOS 26.5
  (8.429s, `/tmp/momentum-empty-scroll26.xcresult`) and 27 (9.816s,
  `/tmp/momentum-empty-scroll27.xcresult`). The whole description scrolls between
  the navigation bar and Add task, and Add opens quick entry. Both screenshots
  were inspected. This required no further layout change; heading and description
  need not fit simultaneously on the smallest viewport at AX5.
- Task Lists now retains compact menu pickers at standard sizes and uses wrapping
  selected-value rows with native inline selection pages at accessibility sizes.
  Initial screenshots show complete grouping/sort values on phone/iPad, but the
  whole-screen native audit still reports anonymous clipping on both. These findings
  remain unsuppressed. Selection/persistence now pass in English/German on iOS
  26.5/27 phone simulators, including all four preferences after a cold relaunch. Do not call the whole B-014 issue resolved
  from these scoped functional results.
- **Toolbar follow-up, 2026-09-17:** `SheetCommitButton` now clears the inherited
  content font on its symbol so native toolbar sizing applies. A rendered-pixel
  regression reproduces the old AX5 glyph crowding and verifies Save/Create ink
  stays inset within its orange fill. iOS 26.5 phone standard/light and AX5/dark
  flows pass, including saving and creating. The same cases pass on iOS 27
  phone and iPad mini 26.5/27 (eight scenarios total); both iPad AX5 screenshots
  were inspected. Dated bundles and timings are recorded in PROGRESS. Task
  typography and the approved ink policy are retained.
  This does not resolve the separate Settings audit findings. Recheck landscape
  and other window sizes independently.

## B-015 — task row exposes undersized accessibility frames

- **Severity:** Medium. **Features:** F-006/F-020. **iOS:** Implemented; native
  phone target-size/tap checks pass, broader assistive acceptance remains open.
- **Reproduce:** add a task with tag/estimate. XCTest reports actionable metadata
  at 130.3 × 14.3 points; Open title frames measure approximately 20.3 points high.
- **Expected:** adequately sized independent Open and Complete targets. Source
  gives only Complete a 44-point minimum. Effective touch slop remains unmeasured.
- **Fix (2026-09-17):** group title, metadata and note/reminder symbols into one
  Open button with a 44-point minimum height and rectangular hit shape. Complete
  stays independent; archive stays read-only. The Open accessibility value retains
  project, estimate, schedule, tag, notes and reminder information. Existing row
  context/swipe/drag actions and named reorder actions remain in place. See A-02.
- **Regression evidence:** isolated production-view baseline fails at 20.33 points
  (`/tmp/momentum-row-targets26-before.xcresult`). Post-fix native tests pass on
  iOS 26.5 and 27, including separate/nonoverlapping 44-point Open/Complete targets
  for parent/subtask/second rows, accessible metadata, unfiltered hit-region audit,
  a bottom-right Open tap, and completion/reopening. Bundles:
  `/tmp/momentum-row-targets26-metadata.xcresult` (19.914s) and
  `/tmp/momentum-row-targets27-metadata.xcresult` (22.213s). Screenshot inspected.
  An intermediate exact-double comparison failed at 43.99999999999994 points;
  geometry assertions now allow 0.001-point conversion noise.
- Three existing in-process row rendering regressions also pass (0.293s total,
  `/tmp/momentum-row-rendering26.xcresult`): Dynamic Type/custom content scaling,
  compact long-title wrapping, and completed appearance in light/dark. These
  component checks supplement the native target/tap regressions.
- **Remaining:** largest-text/native tablet/physical target checks, spoken
  VoiceOver and alternate-input acceptance are not established by this phone test.
  No claim that the original effective touch slop was measured. Existing B-012
  editor frame warning still appears when opening the editor; it is not hidden.
- **2026-09-16 scope note:** the new floating Add task control has its own
  minimum-target/layout regression checks; task row geometry is unchanged.
  Moving Add does not resolve B-015 or the other accessibility audit findings.

## B-016 — operation feedback automatically disappears after five seconds

- **Severity:** Medium. **Features:** F-017/F-020. **iOS:** Superseded by B-087;
  implemented with deterministic model and native-host coverage.
- **Reproduce:** after the 2026-09-17 correction, every operation result stayed visible
  until it was dismissed or replaced. This removed the former reading deadline but made
  routine success messages persistent across tabs.
- **Expected:** routine information is a transient, accessible toast; an undoable mutation
  keeps its exact Undo action long enough to act; a persistence failure remains until the
  person acknowledges it.
- **Fix (2026-09-19):** one root-owned presentation replaces the per-list banner. Ordinary
  information dismisses after 3.5 seconds, exact-batch Undo after 5 seconds, and
  `SaveFailed` remains focused with a Dismiss action. One cancellable sleep task restarts
  for replacement messages, while the existing low-priority announcement is emitted once.
- **Evidence:** see B-087 and the dated PROGRESS handoff. Spoken VoiceOver remains outside
  the current user-approved scope.

## B-017 — task-list subtask relationship lacks accessible context

- **Severity:** Medium. **Features:** F-006/F-020. **iOS:** Implemented; native simulator checks and spoken acceptance recorded below.
- **Reproduce:** add identically named children under two parents, return to Today,
  inspect child Open/Complete actions. Previously both controls had identical labels.
- **Expected:** the relationship communicated by indentation is also available
  nonvisually, including duplicate child titles and filtered search results.
- **Cause/fix:** `isSubtask` previously changed padding only. The mobile worker now
  resolves distinct parent titles off-main once per snapshot, reusing visible rows
  and querying hidden live/archived parents through the read-only core batch API. English/German Open/Complete/Reopen labels
  include the parent; the two independent 44-point actions remain intact. Current
  snapshots reflect parent renames. Read-only archived rows expose the same context,
  including filtered parents in either archive tier. Localized generic “subtask”
  remains a fallback only when the parent record is unavailable. The additive
  `task_reference_titles` API preserves live-only `task_title` semantics and data
  formats; bindings are regenerated by the normal tooling.
- **Regression:** the old native hierarchy contains two identical Open/Complete
  child labels (`/tmp/momentum-subtask-context26-before.xcresult`, failed expected
  contextual-label assertion). The worker tests cover duplicate titles, filtered
  parents, rename refresh and empty search; all four EngineWorker tests pass.
  English/German native open, complete/reopen and filtered Search checks plus the
  independent target/metadata regression pass on iOS 26.5 (3 tests, 61.605s;
  `/tmp/momentum-subtask-context26-after.xcresult`) and iOS 27 (3 tests, 69.958s;
  `/tmp/momentum-subtask-context27-verified.xcresult`). Exported English/German
  hierarchies confirm distinct parent labels and separate 44-point targets; German
  Search screenshot inspected. Dirty `task/ios-app`, 2026-09-17.
- **Archive follow-up:** an isolated mobile regression reproduced empty parent names
  after archival. The read-only batch lookup now passes core tests for both tiers,
  live/young precedence, invalid/missing records and unchanged task/undo state.
  All 116 mobile tests and 254 enabled all-feature workspace tests pass. Native
  English/German archive Search and live-action regression checks pass on iOS 26.5
  (3 tests, 51.699s; `/tmp/momentum-archived-context26.xcresult`) and 27 (3 tests,
  62.017s; `/tmp/momentum-archived-context27.xcresult`). Exported archive labels are
  read-only StaticText with parent names and metadata. Actual speech is not tested.
- **Remaining:** actual spoken VoiceOver/focus and alternate input. Further dip17pm testing is
  user-deferred, not passed. Other platforms are unchanged/unverified for this bug.

## B-018 — small validation error text falls short of contrast guidance

- **Severity:** Medium. **Features:** F-006/F-020. **iOS:** Verified for the recorded error-color, field-name and phone-simulator recovery scope.
- **Reproduce:** enter `invalid` in Estimate at ordinary text/light appearance.
  The visible red guidance triggers XCTest's “Contrast nearly passed” warning,
  whose detail requires a larger font for this contrast.
- **Cause/fix:** task/recurrence errors used `.red` directly. `AccentTheme.errorText`
  now resolves semantic system red against native backgrounds with 4.55:1 normal
  and 7.05:1 increased-contrast targets. Error SF Symbols complement the existing
  meaningful messages. Estimate guidance remains immediately after its field.
  The disabled recurrence value also uses readable secondary text after an
  unfiltered audit exposed its separate contrast issue. The Estimate field has an
  explicit localized name: a native regression first reproduced its previously
  empty label after text entry. Default orange is unchanged.
- **Evidence:** the original guidance failure is reproduced in
  `/tmp/momentum-validation26-before.xcresult`. The 48-combination native trait
  regression and existing default-orange/white-ink regression pass on iOS 26.5
  (0.007s; `/tmp/momentum-error-ink26.xcresult`) and 27 (0.006s;
  `/tmp/momentum-error-ink27.xcresult`). Native light/dark estimate correction,
  localized field labels and German AX5 recovery pass on both runtimes:
  `/tmp/momentum-validation26-current.xcresult` (three estimate cases) and
  `/tmp/momentum-validation27-final.xcresult`. Final recurrence rejection/recovery
  passes in `/tmp/momentum-repeat-error26.xcresult` (34.824s) and
  `/tmp/momentum-repeat-error27.xcresult` (40.644s).
- **Audit scope:** ordinary estimate audits are unfiltered. German AX5 handles only
  the verified toolbar-covered `Fällig` label with pixel/geometry guards (B-039).
  Recurrence defers only a partly-offscreen `Starts` finding and must pass another
  unfiltered audit once it is visible. Original failed findings remain attached;
  these scoped checks do not imply whole-app accessibility compliance.
- **Build:** normal production iOS simulator build passes, exit 0
  (`/tmp/momentum-validation-shipping-build.log`), dirty `task/ios-app`, 2026-09-17.
- **Remaining:** spoken error announcement/focus remains separately open under
  A-05, and B-014 retains the oversized AX5 Save symbol. Broader tablet/assistive
  acceptance is not implied by this phone-simulator scope.
  No physical acceptance is implied by the user-deferred dip17pm checks.

## 2026-09-16 — fast iOS regression infrastructure (F-039)

- No accessibility finding is closed by adding tests. B-013–B-018 remain open;
  component geometry/rendering is not whole-screen or assistive-technology acceptance.
- B-008's rejection/retry behavior now also has fast tests through the exact
  Quick Add presentation state consumed by the sheet, including a real Rust fixture.
  Deliberately accepting unchanged outcomes failed three assertions; restored source
  passes the full 67-test package suite. See the F-039 progress entry for UI evidence.
- B-009/default-orange regression protection includes rendered black symbol pixels
  inside the actual orange filled button. Forcing white ink failed in both appearances.
  Toolbar promotion remains covered by the existing native acceptance scope, not by
  the new standalone component test. The production accent policy is unchanged.
- Bootstrap-only fixture issues (scene lifecycle, absent key window and appearance
  forwarding) were corrected in the separate test host/fixture. They are not product
  defects. The runner rejects empty test selections and stale prepared core artifacts.

## 2026-09-16 — dark icon rendering assessment (F-019)

- The first native icon draft exposed an iOS 26 SVG stroke/color-override rendering
  difference: the checkmark's open interior filled in. Replaced strokes with closed
  filled outlines before acceptance; final iOS 26/27 exports retain the same mark.
- This was caught in design verification, not reported as a shipped defect. Existing
  accessibility findings remain open. No task behavior or other platform changed.

## 2026-09-16 — automation review and test-harness assessment (F-031/F-040)

- Review of the new shared entity query found that `try?` could hide a Mac startup
  error and that iOS resolved each ID through a separate lifecycle refresh. The final
  query acquires the runtime once and resolves a batch, omitting stale IDs while
  propagating startup errors. Mutation batches retain strict all-ID validation.
  A new portable regression proves ordered/deduplicated/stale resolution and strict
  mutation rejection; independent source re-review confirmed both fixes.
- Native URL test failures initially came from relaunching through
  `XCUIApplication.open` while expecting warm navigation and expecting an unscheduled
  URL task in Today. The acceptance test now uses Safari for the warm handoff and
  supplies an explicit due date. Direct URL opening from the UI-test runner is
  rejected by iOS as untrusted; no product workaround or security bypass was added.
- The App Intents test framework rejects Customer OS builds with security error 803.
  That exact condition is reported as a skip, never a passed native invocation.
  A loader failure on iOS 26 also showed that this framework must live in its own
  iOS 27-only test target; a runtime availability guard does not isolate its dylibs.
  Existing accessibility findings and broader parity gaps remain open.
- The iOS 26.5 Shortcuts provider error was isolated to simulator signing in the
  tested Xcode 27 configuration. A fresh one-action app reproduced code 1
  (`Couldn't find AppShortcutsProvider`) when unsigned/ad-hoc; `linkd` could not
  resolve its team identity. A verified Apple Development signature made the probe
  execute, and the same signing correction made Momentum's native Create Task
  acceptance pass. No product authentication workaround, account setup, provisioning
  change or security-setting change was needed. Reproduction steps are in
  [iOS testing](../ios/TESTING.md); final evidence is recorded in PROGRESS.md.

## B-019 — an obsolete Nextcloud exchange can overwrite a restored store

- **Severity:** Critical. **Features:** F-025/F-029. **Shared-core reproduction:**
  pause a real loopback GET, import a replacement, add another task, then release
  the exchange. The original implementation reports success from the obsolete
  snapshot; its cloned Store retains the live directory and may write there before
  any engine-level check. Pending-array offsets also become invalid on replacement.
- **Affected platforms:** Linux/macOS use this shared path; native reproduction is
  not claimed. iOS links the same core and now has a native backup screen; provider UI remains unfinished. Android is
  Planned. Shared fix implemented; native platform acceptance remains outstanding.
- **Fix:** memory-only exchange, replacement generation checks between requests and
  under the final store lock, stable pending-op identity merge, and a sync lease
  retained through persistence. An import during GET prevents the later PUT.
- **Evidence:** red/green `transport_tests::import_during_exchange_rejects_the_old_result_without_writing_over_replacement`;
  validates live and reopened replacement state and zero obsolete uploads. The
  actual HTTP wait is bounded by a 30-second whole-exchange deadline; cancellation
  is cooperative and cannot retract a request already accepted by a server.
- **Follow-through:** explicit cooperative Nextcloud cancellation now uses the same
  generation barrier; its active lease remains held until I/O returns. Journaled
  replacement recovery and P2P exclusion are implemented under B-022/B-027. Native
  provider/lifecycle integration and remote-peer restore acceptance remain unfinished; local iOS document-flow evidence is recorded in PROGRESS.md.

## B-020 — sync commit rolls back clocks and local metadata for concurrent edits

- **Severity:** High. **Feature:** F-025. **Shared-core reproduction:** pause GET,
  add a task locally, then finish sync. The new pending operation survived but its
  vector-clock counter regressed from 2 to 1 in the reproduced fixture.
- **Fix:** merge live vector clocks with the exchange result; retain live summary
  claims and nearby metadata. Pending operations are identified by stable IDs.
- **Evidence:** red/green `transport_tests::edits_during_exchange_remain_pending_and_keep_their_vector_clock`.
  Linux/macOS share the implementation; native/network-server acceptance is not
  inferred. iOS transport UI and Android remain unfinished.

## B-021 — unrelated JSON is accepted as an empty destructive backup

- **Severity:** Critical. **Feature:** F-029. **Shared-core reproduction:** import
  `{}` or `{ "data": {} }` over an existing task store. Serde defaults previously
  accepted either object as empty task data, so import replaced existing tasks.
- **Fix:** backup parsing requires an explicit task collection with both `ids`
  and `entities`; malformed typed values still fail deserialization. Raw/wrapped
  backups, explicitly empty collections and unknown fields remain supported.
- **Evidence:** model red/green validation plus
  `transport_tests::invalid_backup_preserves_tasks_pending_undo_and_disk`. The fixture
  has an actual undo batch before import; creation alone is not undoable.
- **Platforms:** shared fix implemented for Linux/macOS and linked iOS core. Native
  backup-state and app-model tests reject unrelated JSON without changing tasks or revision; system-picker/error presentation acceptance is tracked in PROGRESS.md. Android implementation is Planned.


## B-022 — failed or interrupted store replacement can report success or mix generations

- **Severity:** Critical. **Features:** F-001/F-029. **Reproduction:** block a later
  JSON temporary-file write during import/save. Import previously returned success
  despite persistence failure; a successful earlier rename could leave state, pending
  operations and metadata from different generations.
- **Fix in progress:** private undo journal around the existing three JSON files,
  synced preparation and commit boundaries, recovery before loading, and clone/save/
  publish replacement. Failed import preserves live tasks, pending operations and undo.
  Unknown/incomplete journals fail closed and remain available for recovery.
- **Evidence:** actual child-process exits at six write boundaries recover a coherent
  old or committed image; first-save interruption, failed writes, corrupt/incomplete
  records and orphan cleanup are covered by `sp-store` tests. Core tests exercise failed
  import and checked startup; CLI returns its normal JSON error instead of a panic.
- **Platform boundary:** iOS implements an unavailable screen with retry; macOS a
  native retry/quit alert. Linux still uses the legacy fatal startup boundary when a
  journal cannot be recovered; friendly Linux startup handling remains outstanding.
  Android is Planned. Process-exit tests do not establish physical power-loss safety.
- **Remaining:** native recovery/performance acceptance and write-cost optimization
  (B-023). P2P replacement exclusion is implemented under B-027 and ordinary mutation
  publication/error handling under B-025. Local iOS document-flow evidence is recorded
  in PROGRESS.md; remote-provider lifecycle and physical power-loss safety remain open.

## B-023 — durability barriers increase repeated-mutation latency

- **Severity:** High. **Features:** F-001/F-029. **Affected:** all shared-store callers;
  mobile actor work stays off the main thread, while existing Mac/GTK calls may block UI.
- **Measured on 2026-09-17 dirty `task/ios-app`:** the first post-journal core run took
  41.59 seconds versus 1.09 seconds in the preceding transport run; mobile package tests
  took 4.456 seconds versus 0.107 seconds. Builds/tests overlapped, so these are workload
  observations, not isolated benchmarks or device energy measurements.
- **Cause:** every mutation synchronously journals and flushes the three-file store;
  loops can persist more than once per user action. Correctness tests pass but this is
  not accepted as performance parity.
- **Partial mitigation:** demo/preview seeding now commits one ordered batch; core
  suite time fell to 17.41 seconds. Sequential final Swift runs took 2.168 seconds
  (mobile) and 15.823 seconds (Mac, down from 119.134 seconds before batching).
  This does not resolve ordinary mutation latency.
- **Further mitigation:** main core edit boundaries now stage a private candidate and
  commit once per action. A 100-task completion/undo/paste sample measured 48.93 /
  52.60 / 56.29 ms; committed batches wake the sync observer once. B-025 covers the
  associated failure atomicity. Full core suite was 13.43 seconds before the final
  no-op case; mobile tests were 1.704 seconds and Mac tests 11.214 seconds.
- **Next:** validate large-store and physical-device latency/energy, and assess remaining
  direct-store/native-main-thread writes while retaining crash recovery. Never disable
  durability in tests merely to conceal the cost. Physical battery checks remain open.


## B-024 — native view-test host lacks app translations

- **Severity:** Medium (verification infrastructure). **Feature:** F-039.
- **Reproduction:** render a production SwiftUI view with German environment locale
  in `MomentumFastTests`; screenshot still shows English. Strings resolve from the
  test host's main bundle, which previously contained no app catalog.
- **Fix:** include the production string catalog in the isolated host; the recovery
  rendering regression asserts that the host's German localization resolves its title.
- **Scope:** iOS view tests only. This invalidates German-localization claims based
  solely on the old component harness, not actual app/UI-test screenshots. Mac/Linux
  production behavior is unaffected. Native rerun and image inspection recorded in
  PROGRESS; existing whole-app accessibility acceptance remains separate.


## B-025 — ordinary edits publish unsaved state and consume undo after write failure

- **Severity:** High. **Features:** F-001/F-002/F-005/F-012/F-031. **Reproduction:** block a
  store temporary-file write, then complete/delete a task batch, create a tagged task,
  or undo a batch. Previous normal dispatch logged the error but changed live tasks
  and pending operations, consumed undo, and returned success. Backup-specific B-022
  handling did not cover these regular mutation paths.
- **Fix in progress:** evaluate an edit on a private engine candidate, retain operation
  order/clocks, perform one journaled save, then publish and notify once under the same
  owner lock. Save failure returns a structured error outcome without replacing live
  tasks/undo or waking sync. CLI mutation failure remains a normal error and no longer
  retains an uncommitted operation in memory.
- **Regression evidence:** red/green failure injection for bulk edits, task creation,
  undo retry and organization identities; callback records prove observers see only
  a complete committed batch. Additional tests cover compound capture/paste/tag edits,
  no-op form saves and repeat-cursor retry. Swift package checks cover failed capture,
  automation errors, notification-action retry and Mac form save outcomes. Current
  native checks and verification limits are recorded in PROGRESS.
- **Scope limits:** selected main engine edit boundaries are covered; direct external
  `with_store_mut`/Store callers and legacy Rust tag helpers retain their existing
  separate error contracts. B-027 extends durable publication to P2P adoption.
  Desktop dismissal/error presentation and full
  native interaction acceptance require their own evidence. Android remains Planned.


## B-026 — rejected capture or missing tag targets can still create tags

- **Severity:** Medium. **Features:** F-004/F-005. Shared reproduction: submit only
  `#orphan 30m`, or apply a new tag by name to a missing task ID. The engine returned
  no change while creating and persisting the tag as a side effect.
- **Fix:** validate the parsed title and target membership before creating any tags.
- **Evidence:** a real writable isolated-store regression compares complete state,
  pending operations and clocks before/after both rejected actions. Its initial
  fixture incorrectly dropped its temp-directory owner and was corrected before
  obtaining a real red/green failure; that accidental green is not acceptance.
- **Scope:** shared fix implemented; native Quick Add state coverage and current
  platform builds are recorded in PROGRESS. Android remains Planned.

## B-027 — nearby receive and restore bypass durable publication boundaries

- **Severity:** High. **Features:** F-002/F-026/F-029. Shared reproduction: fail a
  task-store write while receiving peer data, or restore and restart with an old
  transport inbox. Previously live data/bootstrap/timestamps changed despite save
  failure, the transport consumed the batch before persistence, and queued data could
  reappear after restore. Concurrent starts could also create separate runtimes.
- **Fix implemented:** commit received snapshot/operations/repeats/metadata on one
  private engine candidate; acknowledge exact journal entries only after commit.
  Failed acknowledgement retains its retry batch and preserves newer arrivals.
  Start/stop/restore share provider admission; stopped callbacks check their flag
  under the store lock. Nextcloud and LibreSync cannot be admitted simultaneously.
  Stop also waits for the entire store-commit/journal-acknowledgement section, so an
  old runtime cannot overwrite a replacement runtime's journal. Delegates are called
  after releasing that barrier to allow synchronous lifecycle requests.
- Restore increments a backward-compatible, default-zero internal metadata epoch
  in the same durable store transaction. On startup, a mismatched journal discards
  pre-restore queued data and unpublished operations, retains identity/trust/seen IDs,
  and publishes the current snapshot. Restored data is no longer eligible for initial
  peer-snapshot union. Invalid journals fail startup without overwriting the evidence.
  The task/backup/remote operation formats are unchanged; this adds internal metadata.
- **Evidence:** real red/green regressions for failed peer save, active-runtime
  restore, stale inbox after process/runtime restart, and corrupt-journal startup.
  Additional tests cover provider exclusion, concurrent startup, late stopped
  application, durable retry/acknowledgement and arrivals during acknowledgement.
  A paused-acknowledgement regression first demonstrated that restore returned before
  the old runtime finished, then passed with the receive barrier; a delegate reentrancy
  test exercises synchronous stop without holding that barrier.
- **Limits:** final builds/checks are in PROGRESS. Physical cross-device convergence,
  native provider switching/recovery, journal power-loss behavior and remote changes
  first received after a restore remain separate acceptance. Restore replaces local
  data; it cannot retract changes already exchanged with another device. B-029 covers
  acknowledgement failure followed by a provider switch. Android is
  Planned; Linux/macOS/iOS share the implemented core correction.

## B-028 — stopping Sync All waits for stalled peer I/O instead of cancelling it

- **Severity:** High for mobile lifecycle responsiveness. **Feature:** F-026.
  `BackgroundEngine::run` does not pass its command cancellation token into the
  custom discovery/exchange closure. Shutdown therefore waited for peer socket
  timeouts even though the background engine requested cancellation.
- **Fix implemented:** a runtime-owned LibreSync cancellation token is supplied to
  every Sync All request and checked before discovery and between peers. Shutdown
  cancels the token before joining the transport, closing registered sockets.
- **Evidence:** an isolated loopback peer accepts but stalls the TLS handshake.
  Before the fix, shutdown took 5.002 seconds and failed the under-two-second guard;
  after the fix the regression passes. The test still includes the normal two-second
  discovery pass before measuring shutdown. Discovery itself remains bounded by that
  timeout and is not instantaneously interruptible. Native background-expiration
  and physical network testing remain outstanding.

## B-029 — an unacknowledged peer operation replays after another provider clears pending

- **Severity:** High. **Features:** F-002/F-025/F-026. **Reproduction:** receive a
  peer AddTask, let its task-store commit succeed but fail transport acknowledgement,
  complete the task locally, upload through Nextcloud, reopen the store and retry
  the peer inbox. The original AddTask overwrote the newer completion because pending
  operations had been the only duplicate check. Snapshot-covered operations could
  also replay when acknowledgement retried without the original snapshot boundary.
- **Fix implemented:** persist exact peer-operation receipts in the same transaction
  as the received state, including operations covered by a bootstrap snapshot. They
  remain independent of Nextcloud's upload queue. Each subsequent receive retains
  only receipts still in the unacknowledged transport inbox; restore clears them in
  its durable replacement transaction. This is default-empty internal metadata,
  preserving task, backup and remote operation formats without a migration.
- **Evidence:** both cases reproduced before the fix. The provider-switch regression
  uses an actual isolated HTTP exchange, failed journal write and process reopen;
  the bootstrap regression also checks durable pruning once the inbox is empty.
- **Platforms:** Linux/macOS/iOS share the implemented core path; final compilation
  and package evidence is in PROGRESS. Physical provider switching and native sync
  UI acceptance remain outstanding. Android is Planned.

## B-030 — Settings button titles inherit insufficient orange contrast

- **Severity:** Medium. **Features:** F-029/F-020. **Affected:** iOS Settings labels
  used inside native buttons. No shared-core change; desktop/Android do not use this
  SwiftUI component.
- **Reproduction:** open Settings → Backups in light appearance with the default
  #FF6600 accent. Apple's native contrast audit flags Export Backup and Import Backup.
  Component screenshots independently show orange title text.
- **Cause/fix:** hierarchical `.primary` styling inherits the button's foreground;
  explicitly applying `Color.primary` keeps readable system title text while symbols
  retain the selected accent.
- **Evidence:** the failing audit is `/tmp/momentum-backup-ui-27.xcresult`; final
  light/dark/native regression results are recorded in PROGRESS.md. Spoken screen
  reader acceptance remains separate from these automated contrast checks.

## B-031 — credential errors can be mistaken for absence or successful removal

- **Severity:** High. **Feature:** F-025. **Affected:** shared Apple Keychain wrapper;
  Linux/Android use different credential adapters and are not reproduced here.
- **Cause:** legacy `get` returns nil for every OS error, and `set(account, "")`
  returned true without checking `SecItemDelete`. A locked or denied read must not
  be treated as permission to replace a missing connection.
- **Fix:** additive checked Data read/write/delete APIs distinguish item-not-found
  from denial, lock and storage errors. The legacy empty-password setter now returns
  false on deletion failure. Existing desktop accounts and protection defaults stay
  compatible. New mobile connections store all fields/secrets in one atomic item,
  with device-only access after first unlock. Corrupt records remain errors.
- **Evidence:** injected OS failures through the real wrapper exercise denied reads,
  failed deletion, no add fallback after denied updates and failed insertion. An
  isolated actual Keychain round trip and native attribute readback are recorded
  in PROGRESS.md. No production credential or account is used.
- **Remaining:** native mobile setup/retry UX must consume the checked errors. Existing
  desktop UI and LibreSync callbacks still use legacy optional reads; their denial
  handling remains separate work, not a claimed fix from this additive API.

## B-032 — unsigned native test host cannot verify Keychain access

- **Severity:** Medium. **Features:** F-025/F-039. **Affected:** iOS test harness only.
- **Reproduction:** the real native connection test fails with Security status
  `errSecMissingEntitlement` when the runner uses `CODE_SIGNING_ALLOWED=NO`. The
  cancellation test passes, separating the signing failure from FFI behavior.
- **Fix:** native fast tests use local ad-hoc simulator signing, an explicit empty
  development team and no certificate/provisioning requirement. Xcode supplies
  simulated application entitlements. No personal identity is embedded in source.
- **Evidence:** the unsigned failure is `views-1789658659094151000.xcresult`; the
  signed two-test probe passes at `/tmp/momentum-keychain-signed-probe.xcresult`.
  Full iOS 26.5/27 inventories and timings are recorded in PROGRESS.md. This does
  not alter the separate system Shortcuts development-signature requirement.


## B-033 — foreground return waits for the periodic sync interval

- **Severity:** Medium. **Feature:** F-025. **Affected:** iOS coordinator added in this
  dirty worktree; not a reproduced desktop regression.
- **Reproduction:** background an in-flight automatic exchange, return before its
  cancellation drains, then release the cancelled operation. The next scheduled
  delay was 300 seconds rather than an immediate foreground retry.
- **Cause/fix:** the resumed foreground event was ignored while work was active.
  Remember it until the exchange drains, then schedule an immediate retry. The old
  transport is still awaited; no overlap or false success is introduced.
- **Regression:** continuation-controlled `foregroundReturnRetriesImmediatelyAfterCancelledExchangeDrains`
  failed with the periodic delay and passes after the fix. Real suspended-device
  network acceptance remains outstanding. Final evidence is in PROGRESS.md.

## B-034 — sync execution allowance mishandles boundary expiration callbacks

- **Severity:** Medium. **Feature:** F-025. **Affected:** new iOS background allowance;
  discovered in injected native tests before physical-device acceptance.
- **Reproduction:** an expiration callback delivered before the begin function returns
  fails to balance the subsequently returned token. A retained callback invoked after
  completion can also request cancellation after the original exchange ended.
- **Fix:** record expiration/completion independently of the token, balance a returned
  token even after synchronous expiration, and ignore callbacks after completion.
  End the assertion promptly on expiration; do not wait for blocking network I/O.
- **Regression:** both native tests failed against the first implementation and pass
  after the fix. Normal completion, repeated finish and invalid tokens are covered.
  These injected orderings test defensive handling, not a claim that UIKit delivered
  either ordering on a physical device. Final evidence is in PROGRESS.md.


## B-035 — isolated Sync settings dims the provider label below readable contrast

- **Severity:** Medium. **Features:** F-019/F-025. **Affected:** iOS preview/test
  Settings; production provider controls were not reproduced as affected.
- **Reproduction:** navigate to Sync in an isolated app and run the native contrast
  audit. The disabled navigation row's “Sync Provider” label nearly passes contrast.
- **Fix:** isolated mode now presents an explicitly read-only Off status using native
  LabeledContent, rather than a disabled link with dimmed text. No setup action,
  credential access or transport becomes available in isolated mode.
- **Evidence:** `/tmp/momentum-native-sync-settings27.xcresult` records the original
  audit failure. Final reruns are recorded in PROGRESS.md; no broad app compliance
  claim follows from this scoped screen audit.


## B-036 — hard scroll-edge effects create two-tone screen backgrounds

- **Severity:** Medium. **Feature:** F-019. **Affected:** iOS tabbed interface. The
  user supplied a light-mode Today screenshot showing pale bands above and below
  the grouped task canvas on iOS 27.
- **Cause:** RootView explicitly forced `.scrollEdgeEffectStyle(.hard)` at both edges.
  Apple's hard effect intentionally paints a nearly opaque, sharply bounded area
  behind stationary controls; it does not preserve a continuous grouped canvas.
- **Fix:** hide scroll-edge effects throughout the tab hierarchy using the public
  iOS 26+ modifier. Native floating buttons and tab controls retain their own materials.
  The canvas continues to follow the system appearance, without hard-coded light colors.
- **Verification:** `BackgroundTests` samples clear canvas regions behind top/bottom
  controls and between cards in the actual app, with light/dark screenshots for all
  four tabs. Final iOS 26.5/27 evidence is recorded in PROGRESS.md. Component-only
  renders are deliberately not used as proof of safe-area appearance.

## B-037 — Nextcloud cannot create a missing sync folder

- **Severity:** High. **Feature:** F-025. **Affected:** shared transport used by
  Linux, macOS, CLI and iOS. Reproduced through the actual iOS Rust/HTTP boundary;
  native desktop interaction has not been rerun.
- **Reproduction:** import a disposable complete backup, configure an absent WebDAV
  collection, then sync. GET returns 404 and the first PUT returns 409. The client
  fails before sending MKCOL: `MKCOL not valid for HTTP version HTTP/1.1`.
- **Cause:** ureq 3 rejects extension methods by default. The shared transport's
  collection-creation path never enabled WebDAV methods. The original Rust mock
  accepted PUT without requiring a collection, so existing tests missed this branch.
- **Expected:** create the collection once, retry the upload within the original
  deadline/cancellation contract, and clear pending changes only after commit.
- **Fix:** explicitly permit WebDAV extension methods in the scoped ureq agent;
  existing deadline, cancellation, TLS validation and bounded folder retry remain.
- **Verification:** the missing-collection Rust regression failed before the fix,
  then passed; actual iOS 26.5/27 loopback exchanges create the folder and converge.
  Shared Rust/CLI and macOS Swift regressions pass. B-037 is resolved for this
  transport scope; hosted-server and native desktop interaction remain unverified.
  Exact results and bundles are recorded in PROGRESS.md.

## B-038 — saved connection status triggers a native text-clipping finding

- **Severity:** Medium. **Features:** F-019/F-020/F-025. **Affected:** iOS Sync settings.
- **Reproduction:** configure/save a connection and audit the saved-status row. With
  the production scroll-edge configuration, Apple's native text-clipping audit identifies
  the `sync-saved` label: “Connection Saved” may clip at larger Dynamic Type sizes.
- **Status:** Verified for the reproduced iOS 26.5/27 simulator scope. The saved
  confirmation renders completely in English light and German dark at AX5; both
  unfiltered audits pass on both runtimes. Normal-size auditing reports one
  unavailable text-clipping node on both runtimes even though the visible footer is
  complete. The captured hierarchy shows retained Form cells above the viewport;
  on iOS 26 it additionally reports exact secondary labels already covered by B-053.
- **Investigation boundary:** an earlier anonymous contrast finding came from the QA
  host omitting the production scroll-edge configuration. Neutral accent and primary
  status ink did not fix it; matching RootView's actual edge settings removed it.
  Those experimental color changes were reverted. iOS 26.5 still reports an anonymous
  contrast finding, also reproduced on iOS 27 with an opaque navigation surface;
  its element/cause remain unresolved. No default-orange policy changed.
- Reduced native Forms isolate clipping to Label variants while plain Text and
  explicit stacks pass. The same stack still fails in the complete form, so that
  speculative production change was reverted. Reduced contrast examples identify
  a partly occluded row and a disabled Save button, but do not establish the full
  form's anonymous finding. See the dated probe bundles in PROGRESS.md.
- **Remediation (2026-09-17):** the saved confirmation now lives in
  a wrapping Options footer; Save Connection appears only for unsaved changes. The
  redundant disabled action is removed and successful saving directs accessibility
  focus to its confirmation. Plain Text in the old row did not clear the native finding.
- **Typography isolation:** the same iOS 27 audit failed with native
  semantic body/caption fonts and with the QA host's app-font override removed.
  Both runs identify the fully on-screen `Options` header, so inherited typography
  did not explain the original result. Those speculative experiments were reverted.
- **Final audit treatment:** the normal-size regression may handle at most one
  unavailable `.textClipped` finding. On iOS 26 only, it also recognizes the exact
  Sync secondary labels through the separately verified B-053 classification. Every
  labeled clipping issue, visible hit-region issue, other label and every iOS 27
  contrast issue still fails. This bounded treatment is paired with unfiltered English
  light and German dark AX5 audits, exact saved-label/frame assertions, the 5.5:1/7.05:1
  token regression and retained issue attachments.
- **Evidence:** final matrices pass three tests on iOS 26.5 (108.551s,
  `/tmp/momentum-sync-saved26-matrix-final.xcresult`) and iOS 27 (109.295s,
  `/tmp/momentum-sync-saved27-final.xcresult`). Hosted Nextcloud/TLS, spoken
  VoiceOver and physical-device acceptance remain separate and unverified.

## B-039 — scrolled content overlaps the navigation title

- **Severity:** Medium. **Features:** F-019/F-020/F-025. **Affected:** iOS Sync settings, Today and Accent Color reproduced;
  shared task and Settings navigation also received the same canvas correction.
- **Reproduction:** save a connection and scroll to the bottom. The flat-canvas
  configuration hides the scroll-edge treatment, allowing form text behind the
  navigation title/status area. Reproduced in English/light and German AX5/dark
  screenshots on iOS 27; this was not covered by B-036's empty-screen pixel checks.
- **Cause/fix:** the pushed screen needs an opaque navigation background matching
  `systemGroupedBackground`. Applying toolbar preferences outside NavigationStack
  had no effect. SyncSettings now applies the shared navigation-canvas modifier on
  its own content. Extended to task/list and Settings destinations after reproducing
  the same problem on Today and Accent Color. Native controls and the exact accent
  policy remain intact. Automatic background visibility preserves expanded large
  titles; forcing the background visible hid those titles on iOS 26.5. Native tests
  now inspect title pixels as well as canvas pixels.
- **Evidence:** before `/tmp/momentum-sync-setup27-final-renders`, after
  `/tmp/momentum-sync-nav-inner27-renders`. Both inspected on September 17 in the
  dirty iOS worktree. Scoped light/dark regression and remaining results are recorded
  in PROGRESS.md. Light/dark pixel checks pass on iOS 26.5 and 27 for this Sync
  screen. The final automatic-visibility implementation also passes all four empty
  tab canvases/title-ink checks and eight scrolled destinations in light/dark mode
  on iOS 26.5 and 27; exact bundles are in PROGRESS.md. Remaining sheet, large-text
  navigation and full accessibility checks are outside this scoped verification.
- **Sheet follow-up (2026-09-17):** German AX5 TaskEditor reproduces the same
  overlap (`/tmp/momentum-validation26-final.xcresult`): `Fällig` at y=41 is visible
  beneath the navigation title and fails native contrast. Applied the existing
  canvas modifier to task and recurrence Form content. Inspected after screenshots
  show the label fully covered; German AX5 pixel/geometry checks pass on iOS 26.5/27
  as part of B-018's validation tests. XCTest retains the covered label and still
  reports its contrast; a narrow label/type/geometry exception is documented and
  guarded by opaque-toolbar pixel assertions. Original failing artifacts remain.
  This does not resolve B-014's oversized Save symbol.


- **Configured task-list regression (2026-09-17):** the new Nextcloud status top
  inset hid the large Today title even though the accessibility element existed.
  The native pixel regression measured zero title-ink pixels. Placing status in the
  scrolling List restores native title rendering; final cross-version evidence is
  in PROGRESS.md under F-028/B-043.

## B-040 — unreadable nearby-sync secrets can be replaced as if absent

- **Severity:** High. **Features:** F-026/F-027. **Affected:** shared Apple Keychain
  bridge used by macOS and the upcoming iOS LibreSync runtime. Linux uses a different
  secret-store adapter; its error handling has not been reproduced in this scope.
- **Reproduction:** return `errSecInteractionNotAllowed` when the core asks for a
  saved key, then permit a later write. The old optional-read bridge returned nil,
  authorizing the core's create-if-missing path to replace the key. Malformed data
  had the same unsafe absence interpretation. Reproduced with isolated Keychain stubs.
- **Fix:** latch read/write/delete failures for the lifetime of the transport's
  Keychain bridge; reject subsequent mutations. Validate base64 on read before it
  crosses the optional FFI callback. A new runtime creates a fresh bridge for retry.
  Existing item names/formats are preserved; no credential migration.
- **Evidence:** focused Swift regression first failed nine assertions, then all eight
  checked-Keychain tests passed (`/tmp/momentum-nearby-keys-red.log` and
  `/tmp/momentum-nearby-keys-green.log`, September 17 dirty iOS worktree). Broader
  The rebuilt Mac suite passes (136 tests including peer exchange), and all four
  native Keychain/sync boundary tests pass on iOS 26.5 and 27. Physical locked-device
  acceptance remains unverified.

## B-041 — nearby listener receives operations without notifying the app

- **Severity:** High. **Feature:** F-026. **Affected:** shared Rust transport used by
  Linux/macOS and the iOS adapter. Reproduced with isolated local peers on macOS;
  Linux runtime and physical iOS discovery remain separate checks.
- **Reproduction:** the mobile adapter receives a task, completes it on the second
  peer and syncs back. The first peer's transport journal receives the completion,
  but its task store stays unfinished until another outbound exchange occurs.
- **Cause:** the listener captured its event sink before `Engine.spawn()` installed
  one. Inbound events therefore never reached the application event pump.
- **Fix:** install the sink before starting the listener; retain the actual-port
  startup barrier and occupied-port fallback. Wake the app pump after runtime
  publication so queued startup events do not wait for another command.
- **Evidence:** the real two-node regression first timed out waiting for InboundSync,
  then passed in 0.14 seconds (`/tmp/momentum-nearby-inbound-green.log`). The immediate
  startup-notification regression also failed before its fix. All 15 shared core
  nearby tests pass in 0.88 seconds (`/tmp/momentum-nearby-core-green.log`), September
  17 dirty `task/ios-app`. Rebuilt mobile host and native iOS 26.5/27 two-peer
  exchanges pass; the Mac suite including peer exchange also passes. See PROGRESS
  for bundles and the later user deferral of iOS LibreSync.

## B-042 — nearby state-save completion causes an endless background save loop

- **Severity:** High (idle CPU, storage writes and UI callbacks). **Feature:** F-026.
  **Affected:** shared nearby runtime on Linux/macOS and the iOS adapter.
- **Reproduction:** start an isolated node and request a transport-state save. Every
  TaskFinished event queues another save, whose completion repeats the process.
- **Fix:** track internally submitted save tickets and consume their completion
  without scheduling another save. Failed saves report an error; inbound exchanges
  explicitly request persistence. Other commands retain their existing save behavior.
- **Evidence:** a real runtime regression failed after 32 continuing notifications;
  after the fix it persists `state.bin` and settles without further notifications
  (`/tmp/momentum-nearby-save-loop-red.log` and `-green.log`, 0.20 seconds). The 15-test
  shared nearby suite also passes. This verifies the loop, not physical-device energy
  consumption; foreground/background profiling remains required for iOS acceptance.


## B-043 — healthy Nextcloud status is absent from task lists

- **Severity:** Medium. **Feature:** F-028. **Affected:** iOS task screens; desktop
  frontends already have persistent status entries and are unchanged.
- **Reproduction:** configure Nextcloud without an error, then return to a task list.
  The old entry is guarded by `failure != nil`, hiding progress, pending changes and
  last success. The same guard affects both empty and populated list branches.
- **Fix:** a shared native navigation entry displays the selected provider, active
  progress/stopping state, last successful exchange and pending changes. Failures
  retain a persistent recovery entry. Every state opens the same Sync settings;
  selecting Off hides the entry. Absolute dates add no idle timer. The entry is a
  scrolling list row at every text size: the first top-inset version hid Today’s
  large title, caught by full-screen inspection and a failing title-pixel assertion.
  Native list disclosure replaces the component's custom chevron.
- **Evidence:** the new iOS 27 native test first fails because the healthy status
  entry is absent (`/tmp/momentum-status-link-red27.xcresult`), then passes after the
  change (`/tmp/momentum-status-link-green27.xcresult`, 22.134 seconds). Two extended
  native flows pass on iOS 26.5/27 (46.827/49.701 seconds). The initial test verifies
  the shared production component over a real loopback exchange, details navigation,
  successful status publication and Off. Full production-view Today/Upcoming/Search,
  quick-add, title pixels and AX5 scrolling checks now pass on both target runtimes
  (54.145s on iOS 27, 51.542s on iOS 26.5); final app build also passes. This verifies
  the visual/native interaction scope. Spoken VoiceOver remains part of F-020's
  outstanding accessibility acceptance. Bundles and revision scope are in PROGRESS.md.

## B-044 — task-open routing can collide with Keyboard Help

- **Severity:** Medium. **Feature:** F-023. **Affected:** iOS keyboard-help sheet only;
  no shared-core or desktop changes.
- **Reproduction:** open Keyboard Help, then route a task from an OS entry point.
  The existing presentation gate only considered capture/editor sheets and attempted
  a second sheet over help.
- **Fix:** keep the route queued while help is visible and retry it on dismissal,
  using the existing pending-notification/automation path.
- **Evidence:** the isolated native integration test fails before the gate change
  (`/tmp/momentum-keyboard-help-red26.xcresult`, September 17, dirty `task/ios-app`
  based on `7ec82e9`). The post-fix model/automation suite passes on iOS 26.5 and
  27. The actual iOS 27 help-sheet Done action now opens the queued editor and
  preserves the task (15.983s, `/tmp/momentum-keyboard-and-sync-final27.xcresult`).
  Matching iOS 26.5 native dismissal passes in 14.130s
  (`/tmp/momentum-keyboard-and-sync-final26.xcresult`), verifying the queued-sheet
  routing scope on both versions. This fix does not resolve F-023's separate
  failing navigation/Search keyboard commands.


## B-045 — iOS navigation and Search keyboard commands do not dispatch

- **Severity:** Medium. **Feature:** F-023. **Affected:** native iOS scene commands;
  no shared-core, Linux or macOS behavior changes.
- **Reproduction:** launch Today and press Command-2 or Command-F. Both the isolated
  Nextcloud QA scene and shipping app's isolated test mode remain on Today. The
  shipping regression fails before this fix in
  `/tmp/momentum-keyboard-shipping-probe27.xcresult` (September 17, dirty
  `task/ios-app`, base `7ec82e9`).
- **Cause and fix:** Settings/Command-Comma was registered inside Navigate instead
  of the standard app-settings group. Moving it to `.appSettings` restores that
  menu's dispatch. Search also used the text-editing insertion point, which did not
  dispatch from a task list; it now belongs to Navigate. Reduced scene tests
  isolated these placements while retaining the shared bindings, model, availability
  gates and action handlers. Repeated keys, explicit identities, renamed menu titles
  and replacing ForEach did not fix the original arrangement and are not retained.
- **Status:** In progress. The command-placement change restores navigation and
  Search in the warmed QA sequence; clean cross-version and shipping-entry results
  are recorded in the F-023/B-045 progress handoff. iOS 27 shipping Command-F still
  fails both immediately after launch and after Command-2 navigation. The same
  six QA scenarios and both shipping regressions pass on iOS 26.5. Registering QA
  commands eagerly, like the shipping scene, did not resolve the iOS 27 failure
  and that experiment was reverted. The retained change
  preserves every binding, including fixed Command-Comma after modifier changes.
- Temporary key-event tracing and probe tests were removed. This is not a claim
  about the framework's internal cause or a physical-keyboard acceptance pass.
- **September 17 follow-up:** the unchanged isolated QA Search test passes on a
  physical iPhone running iOS 27.0 (`/tmp/momentum-search-physical27.xcresult`,
  13.662s). It sends one Command-F after launch, types a query, and finds the seeded
  task. This is XCTest-generated input on a physical device, not a manually operated
  external keyboard or shipping-entry device acceptance. On a second iOS 27 simulator,
  the shipping navigation-then-Search test passes but cold Search still fails
  (`/tmp/momentum-search-second-device27.xcresult`). Keep both regressions enabled;
  the remaining simulator failure is not established as a production or XCTest bug.
- QA-only native Find configuration, held-modifier delivery, distinct Search labels,
  another text-editing insertion point and a priority UIKit key-command responder
  did not resolve the iOS 27 simulator failure. All experiments were removed; no
  Search-specific native adapter, diagnostic logging, warm-up or key retry was added
  to production.

- **2026-09-18 testing disposition:** the user removed the iOS XCUITest suite, so the
  synthetic key-dispatch regression no longer exists in source. This does not close
  the product behavior or turn prior failures into passes. Command binding and action
  logic remain unit-tested; actual cold hardware-key routing requires manual simulator
  evidence before this bug can be closed.

- **Latest physical evidence (2026-09-17):** the resumed seven-case device run
  reaches the tests after unlock. Six cases pass, but cold Search now also fails
  on the physical iOS 27 QA app (`/tmp/momentum-planning-keys-device27-unlocked.xcresult`,
  13.722s, overall exit 65). The screenshot remains on Today after Command-F and
  the assertion cannot find SearchField. This supersedes any interpretation that
  the earlier device pass establishes reliable cold Search. Manual external-keyboard
  behavior and the underlying dispatch cause remain unverified; no test is waived.

## B-046 — iOS task Delete keyboard acceptance does not reach its action

- **Severity:** Medium. **Feature:** F-023. **Status:** In progress.
- **Affected scope:** reproduced in iOS 26.5 simulator UI automation while adding
  task keyboard commands. The iOS 27 simulator and physical-device tests fail earlier
  at cold Select All (B-047), so neither establishes a Delete result. Physical-keyboard impact is
  unverified. No Rust, macOS or Linux behavior changed.
- **Reproduction:** launch the isolated task-list QA scene, select the seeded task
  with Command-A, and send Command-Delete. Selection works but the task remains;
  `testKeyboardTaskDeletionAndUndo` retains this as a separate failing regression.
- **Investigation:** a QA-only trace at `UIApplication.sendEvent` observed Command
  modifier events but no Delete press in `/tmp/momentum-task-delete-app-trace26.xcresult`.
  Earlier responder-only traces could not establish whether UIKit received the key.
  Named Delete, ASCII backspace and DEL attempts failed; a priority UIKit Delete
  command also failed. This evidence does not identify the upstream cause or prove
  that a physical keyboard is affected. Do not alter the shared binding or waive
  acceptance on that assumption.
- **Current implementation and next step:** task Delete uses the existing shared
  binding and Rust-backed delete operation. The named XCTest key and failure are
  retained; no key retries, input warm-up, event swizzling or Delete-specific adapter
  remains. Cross-runtime results follow in PROGRESS; verify physical key delivery
  and default-modifier deletion/undo before closing this scope. Selection, completion,
  text editing and modifier/tab isolation have independent tests so this failure
  cannot hide their results.

- **Simulator-only follow-up, 2026-09-17:** an independent System Events →
  Simulator keyboard path did not deliver the known-good Command-2 control on
  iOS 26.5. It therefore supplies no trustworthy Select All/Delete result and
  cannot replace or waive the existing XCTest failure. Disposable probes were
  cleaned; no production workaround or physical-device test was added.

- **2026-09-18 testing disposition:** the user removed the iOS XCUITest suite and its
  synthetic Delete input. Shared binding, enablement and Rust deletion/Undo behavior
  stay unit-tested. The unresolved input-delivery question remains In progress and
  needs manual external-keyboard simulator acceptance.

## B-047 — cold task Select All does not invoke the enabled iOS 27 responder

- **Severity:** Medium. **Feature:** F-023. **Status:** In progress.
- **Affected scope:** iOS 27 simulator and physical iPhone, isolated QA entry point
  using production task views. The same selection/completion/reopen/undo scenario
  passes on iOS 26.5.
  No macOS/Linux/shared-core change. Device automation reproduces the failure, but
  manually operated external-keyboard impact and the underlying cause are unverified.
- **Reproduction:** cold-launch the task-list fixture, wait for the seeded task row,
  then send Command-A as the first shortcut. The expected Actions (1) button never
  appears. Both independent selection and deletion tests fail at this prerequisite
  (`/tmp/momentum-task-keys-ready27.xcresult`).
- **Investigation:** temporary public responder lifecycle tracing confirmed successful
  first-responder acquisition in a key window. UIKit repeatedly queried Select All
  and received true while the view remained first responder; `selectAll(_:)` was
  never invoked (`/tmp/momentum-task-selection-lifecycle27.xcresult`). This rules out
  the tested hypothesis of failed initial focus acquisition; it does not establish
  why dispatch fails. A public native-menu rebuild after focus acquisition also
  failed (`/tmp/momentum-task-menu-refresh27.xcresult`). Both the experiment and
  temporary diagnostics were removed.
- **Device evidence:** `/tmp/momentum-task-keys-physical27.xcresult` reproduces the
  same cold Select All failure in both task cases (14.832s, 14.974s). Native Search
  text editing (21.982s) and modifier/tab/editor isolation (32.741s) pass on that
  device. These use production views in the disposable QA app, not the shipping
  entry point or manually operated keyboard.
- **Remaining validation:** investigate the remaining input/dispatch boundary and
  validate cold-start behavior manually. The modifier/help/tab/editor isolation scenario passes on both simulator
  versions, but its preceding interactions do not replace cold-start acceptance.
  Keep Delete independently tracked as B-046 and Search as B-045. Additional Open,
  Duplicate, Copy and Archive acceptance explicitly selects through touch; passing
  those action tests does not resolve this cold keyboard-selection failure. See
  the dated F-023 progress record for each runtime.

- **2026-09-18 testing disposition:** the user removed the iOS XCUITest source and
  target that injected Command-A. Responder/action state remains covered by unit
  tests; the previously reproduced dispatch failure remains open until a manual
  simulator keyboard check establishes current behavior.


## B-048 — selection Actions overlaps the native tab bar

- **Severity:** Medium. **Features:** F-010, F-018, F-020. **Status:** Verified for the recorded phone-simulator scope.
- **Affected scope:** reproduced visually on iOS 26.5 in the production TaskScreen
  hosted by the isolated QA app. The selected-task Actions button occupies the
  center of the tab bar and its symbol is partially obscured. macOS/Linux do not
  use this nested navigation/tab-bar layout; iOS 27 and largest-text validation
  follow below rather than being inferred from the screenshot.
- **Reproduction/evidence:** select the seeded task, open/edit through the keyboard,
  then dismiss the editor. The final screenshot in
  `/tmp/momentum-task-more-keys-final26.xcresult`, Open/edit/copy case, shows the
  overlap even though action dispatch assertions pass.
- **Cause and implementation:** Actions used a NavigationStack bottom toolbar inside
  a TabView. Both controls now share the bottom safe-area inset above the tab bar,
  using a horizontal row at standard sizes and a vertical stack at accessibility
  sizes. Its opaque system grouped canvas prevents scrolling text from showing
  behind the controls. Native geometry, actual menu completion and rendered Add ink
  are checked; existence alone cannot prove that a control is usable. The initial
  regression fails because Actions is not hittable (13.742s,
  `/tmp/momentum-selection-layout-red26.xcresult`).
- **Fixture correction:** per-argument defaults preserve an explicitly requested
  content size. A stacked-layout assertion and screenshots distinguish genuine AX5
  from the earlier incorrectly overridden standard-size dark run; see PROGRESS.

- **Verification:** September 17, dirty `task/ios-app`, base `7ec82e9`. Final
  light/standard-dark/genuine AX5-dark geometry, rendered white Add ink, touch-menu
  completion and scroll-to-result cases all pass on iOS 26.5 and 27
  (`/tmp/momentum-selection-canvas-final26.xcresult` and
  `/tmp/momentum-selection-canvas-final27.xcresult`). Final screenshots were inspected;
  the normal Momentum simulator build passes
  (`/tmp/momentum-task-actions-selection-build.log`). Physical-device/iPad/landscape
  and spoken VoiceOver acceptance remain outside this recorded fix scope.


## B-049 — duplicate selection can follow another task creation

- **Severity:** Medium. **Features:** F-006, F-023. **Status:** Verified for the
  actor identity regression and recorded phone-simulator duplicate flow.
- **Affected scope:** the iOS keyboard duplicate path awaited duplication, then
  separately awaited the global last-added ID. Another EngineWorker caller could
  create a task between those calls, causing selection to target the other task.
  The deterministic host test reproduces that actor-call ordering using the real
  core/store (`/tmp/momentum-duplicate-selection-red.log`, 0.160s failure). This is
  a selection error; it does not establish task loss or a reproduced device race.
  macOS's synchronous duplicate/ID sequence has no equivalent Swift actor suspension;
  this fix does not change Rust, Linux or macOS behavior.
- **Fix:** `EngineWorker.duplicateTaskWithID` returns the Outcome and optional copied
  identity from one uninterrupted actor method. The native list consumes that result
  directly; a failed/no-op duplication returns no identity. The existing Outcome-only
  API remains available for existing callers.
- **Fast evidence:** `TaskEditingTests` covers the returned identity surviving later
  creation, eight concurrent duplicates interleaved with other creations, unique copy
  IDs and invalid-source failure. The focused editing/organization suite passes all
  13 tests in 2.015s (5.296s including incremental build),
  `/tmp/momentum-planning-fast-focused.log`. Other create/last-added-ID call pairs
  are not claimed covered by this scoped correction.
- **Native/build evidence:** duplicate/archive/Undo passes on iOS 26.5 and 27 phone
  simulators (`/tmp/momentum-planning-keys-green26.xcresult` and
  `/tmp/momentum-planning-keys-green27.xcresult`); the normal Momentum simulator app
  builds successfully (`/tmp/momentum-planning-app-build.log`). These native cases
  verify serial user interaction; the fast actor tests prove concurrent identity
  isolation. Physical QA initialization was interrupted by device authentication,
  so this change has no new physical-device acceptance. All evidence is from the
  September 17 dirty `task/ios-app` worktree, base `7ec82e9`.

## B-050 — project destination label triggers maximum-text clipping audit

- **Severity:** Medium. **Features:** F-018, F-020, F-023. **Status:** Verified for
  the recorded maximum-text dark phone-simulator audit scope.
- **Reproduction:** in the new iOS Move to Project keyboard sheet, enable maximum
  Dynamic Type and run the native text-clipping audit. iOS 26.5 reports “Text clipped”
  for the Inbox label (`/tmp/momentum-organization-boundaries26.xcresult`, 17.804s).
  Exported element/screen attachments identify the combined folder/text label;
  multi-line project names also wrap under the symbol. This is an iOS sheet issue;
  Linux/macOS interfaces and shared rules are unchanged.
- **Correction:** render the decorative SF Symbol and vertically fitting title as
  separate baseline-aligned elements, with one project-name accessibility label on
  the button. Text wraps within its own column and the symbol is accessibility-hidden.
  The unfiltered contrast/hit-region/text-clipping audit remains enabled; no finding
  is suppressed. Final audits pass on iOS 26.5 (20.437s) and 27 (21.534s), with
  inspected screenshots and a successful normal app build. See the dated
  `/tmp/momentum-organization-final26.xcresult`,
  `/tmp/momentum-organization-final27.xcresult` and PROGRESS; no spoken VoiceOver,
  iPad or physical-device acceptance is implied.

## B-051 — keyboard New Project initially has no text focus

- **Severity:** Medium. **Features:** F-004, F-023. **Status:** Verified for
  keyboard-only creation through phone-simulator automation.
- **Reproduction:** open New Project with Command-Shift-N and type immediately,
  without tapping Name. The iOS 26.5 UI test fails because no element has keyboard
  focus (`/tmp/momentum-project-keyboard-entry26.xcresult`, 20.073s). The earlier
  creation test tapped Name and therefore did not establish a keyboard-only flow.
- **Correction:** the native project/tag editor binds its Name field to FocusState
  and focuses it when the creation sheet appears. Existing records keep their
  previous initial-focus behavior. The attempted defaultFocus-only approach did
  not fix the issue and was removed. No delays or warm-up input are retained.
- **Evidence:** direct application-level typing, Command-Return Save, creation in
  Lists and tab preservation pass on iOS 26.5 (28.308s) and 27 (29.344s), both exit 0
  (`/tmp/momentum-project-appearance-focus26.xcresult` and
  `/tmp/momentum-project-appearance-focus27.xcresult`). The normal app build passes.
  This is not manual external-keyboard or physical-device acceptance; no desktop
  focus code changes.

## B-052 — English repeat interval uses plural wording for one

- **Severity:** Low. **Features:** F-008, F-021. **Status:** Verified for the iOS
  phone-simulator editor and both compiled Apple catalogs; macOS native UI pending.
- **Reproduction:** the iOS 27 Repeat editor showed “1 weeks” in the keyboard flow
  (`/tmp/momentum-organization-final27-attachments/5BBFB9A1-F058-4E7C-9FF5-2B74918E2DD4.png`).
  Expected singular wording for one and plural wording otherwise.
- **Cause/fix:** both Apple application catalogs supplied German one/other forms
  but no English forms for `%u days`, `%u weeks`, `%u months` and `%u years`.
  Added the missing English variations, preserving the typed keys and German text.
  Recurrence rules and stored data are unchanged.
- **Regression evidence:** the new `macos/scripts/tests/test_repeat_catalogs.py`
  compiles each actual catalog and resolves resources with Foundation. Before the
  fix, both tests failed on all four English singulars (eight failures total);
  afterwards all 64 English/German checks pass at counts 0, 1, 2 and 99. The fast
  suite takes about 0.4 seconds on this host, without launching a simulator.
  Logs: `/tmp/momentum-repeat-catalog-red.log` and
  `/tmp/momentum-repeat-catalog-green.log`, dirty local worktree, 2026-09-17.
- **Affected-platform assessment:** iOS reproduced visually; macOS reproduced by
  compiled production resources and fixed in its catalog. Linux presents a static
  unit subtitle beside a separate SpinRow, so this Apple combined-string defect
  does not establish a Linux defect; Linux native assessment remains outstanding.
  Android remains planned. The separate repeat-description formatter is unchanged.
- **Native verification:** the first iOS 26.5 run shows the corrected “1 week” on
  screen but the new test incorrectly searched for that as a standalone element.
  Its actual combined accessibility label is “Every, 1 week”; the test now uses
  that label and the Stepper's named Increment button. This test-query failure
  (`/tmp/momentum-repeat-units26.xcresult`) is not a recurrence regression.
- **Native results:** corrected interval-change/save/reopen/stop test passes on
  iOS 26.5 (31.493s, `/tmp/momentum-repeat-label26.xcresult`) and iOS 27
  (32.510s, `/tmp/momentum-repeat-label27.xcresult`), both exit 0. Exported screens
  show “2 weeks”; assertions verify singular before increment and persisted plural
  after reopening. This scope does not claim German native interaction, macOS
  editor interaction, spoken VoiceOver or physical-device acceptance.

## B-053 — iOS 26 reports false contrast failures for readable Settings text

- **Severity:** Medium. **Features:** F-019/F-020. **Status:** Verified for the
  reproduced iOS 26.5 simulator scope; the app's rendered contrast is independently
  enforced and iOS 27 remains unfiltered.
- **Reproduction:** on iOS 26.5, the shipping isolated app's existing
  `LaunchTests.testSettingsCategoriesAndTaskPreferencePersist` completes preference
  persistence, opens Appearance and fails its native contrast audit at line 85.
  `/tmp/momentum-ax5-settings26-options.xcresult` (2026-09-17, dirty `task/ios-app`)
  identifies the “Colors” section header in its element screenshot.
- **Expected:** readable section headers, footers and actions meeting the platform
  contrast requirement without hiding real audit findings.
- **2026-09-17 investigation:** isolated Appearance audit reproduces light-mode
  failure while dark passes on iOS 26.5. Increasing the gray contrast, using
  hierarchical primary, explicit adaptive UIKit label color, or fixed-size layout
  did not remove the finding. Exported pixels verify solid-black text on the
  (242,242,247) canvas in the explicit-label experiment, 18.8194:1 contrast, yet
  Apple's audit still flags Colors. Original ink measured 4.5547:1. The same iOS 26
  behavior later reproduced for Notification section/footer text and the black
  Refresh Schedule label, while the equivalent iOS 27 workflows passed.
- **Fix:** strengthened the shared secondary-text token to a 5.5:1 normal-contrast
  floor across all supported native surfaces (7.05:1 with Increase Contrast) and
  kept Refresh Schedule primary text. XCTest handles only the exact verified iOS 26
  labels; every other label/runtime remains unfiltered. A native trait regression
  independently enforces the ratios, preventing the audit exception from masking a
  color-token regression.
- **Scope:** reproduced on the iOS 26.5 phone simulator in light mode; dark passes.
  Final isolated light/dark tests pass on iOS 26.5 (29.353s,
  `/tmp/momentum-appearance26-final.xcresult`) and unfiltered on iOS 27 (33.501s,
  `/tmp/momentum-appearance27-final.xcresult`). Shipping Appearance/Notification
  workflows pass on iOS 26.5 (three tests, 82.855s,
  `/tmp/momentum-secondary-native26-final.xcresult`) and iOS 27 (three tests,
  87.835s, `/tmp/momentum-secondary-native27.xcresult`). Exported iOS 26 pixels
  measure `#616164` on `#F2F2F7` at 5.5316:1 and black on white at 21:1.
  Spoken VoiceOver and physical-device acceptance remain separately deferred.


## B-054 — shared count messages use plural English for one

- **Severity:** Low. **Features:** F-017/F-021. **Status:** Verified for compiled Apple plural resources and
  the native iOS completion/summary regression; broader localization acceptance
  remains separate.
- **Reproduce:** complete the only task on Today. The iOS 26.5 screenshot shows
  “1 tasks completed” in feedback and “You completed 1 tasks. Time to switch off.”
  in the all-done summary (`/tmp/momentum-feedback26-after.xcresult`, 2026-09-17,
  dirty `task/ios-app`). Expected singular “task” for one, plural for other counts.
- Both texts come from MomentumKit Strings and its shared catalog; macOS impact
  is suspected from shared presentation resources but has not been reproduced in
  the running Mac app. Linux has separate resources; no reproduced Linux impact.
- Source inspection confirms the shared catalog has German one/other variants
  for these keys but no English plural variants. The adjacent completed-and-archived
  key has the same missing English variants; its native symptom is not yet reproduced.
- **Fix:** added English one/other variants for all 21 shared keys with the same
  missing-English-variant pattern: completion/summary, bulk task actions, relative
  times, recurrence/snooze wording, Upcoming description and linked-device count.
  This changes wording resources only; LibreSync implementation remains deferred.
  German wording, lookup keys and typed placeholders are retained.
- **Evidence:** `test_shared_plurals.py` compiles the real MomentumKit catalog and
  checks Foundation lookups for both languages at counts 0/1/2/99. Baseline:
  21 incorrect English singular outputs out of 168; after: all 168 pass (~0.4s).
  `/tmp/momentum-shared-plurals-before.log` retains the baseline. All 136 shared
  package tests pass (12.462s, `/tmp/momentum-shared-plurals-kit-tests.log`).
- Native iOS checks assert the exact completion banner and all-done description,
  then dismiss/Undo: iOS 26.5 23.150s (`/tmp/momentum-completion-plurals26.xcresult`)
  and iOS 27 26.285s (`/tmp/momentum-completion-plurals27.xcresult`). The inspected
  screenshot reads “1 task completed” and “You completed 1 task. Time to switch off.”
- B-052 covered repeat-editor intervals; this is the shared presentation catalog.
  Compiled German outputs pass, but this scope does not claim every affected
  message was exercised through a running German/macOS UI. Physical runs remain
  user-deferred; Linux resources are unchanged.

### B-054 consumer packaging evidence

Normal macOS and iOS simulator builds pass on 2026-09-17, dirty `task/ios-app`:
`/tmp/momentum-shared-plurals-macos-build.log` and
`/tmp/momentum-shared-plurals-ios-build.log`. The same 168-output verifier also
passes against the MomentumKit resources packaged inside each built application.
No source fallback or standalone-test bundle substitutes for these packaged checks.

## B-055 — repeated mobile snapshot reads rebuild all task rows

- **Severity:** Medium. **Features:** F-001/F-039. **iOS:** Verified for immutable task projection and native row/archive regression scope.
- **Cause identified in source:** Previously, TaskSnapshot.tasks flattened and filtered the entire
  listing on every access. TaskScreen reads it for selection, keyboard context and
  drag payloads during native view updates. EngineWorker already materializes the
  same ordered task array off the main actor while creating the snapshot.
- **Expected:** immutable presentation snapshots should reuse their task projection;
  filtering, ordering and task rules remain owned by Rust.
- **Fix:** TaskSnapshot retains the worker's existing ordered task array as an
  immutable value; view updates reuse it without changing Rust task semantics.
- **Verification:** opt-in Release `MomentumPerformanceTests` scheme measures
  100 reads of a real 1,000-task snapshot on the iOS simulator. The 26.5 baseline
  averages 15.939ms; the fixed path measures 2–4µs on 26.5 and 27, near the timer
  floor. Full fast suite (117 tests) and native Open/Complete/reopen/archive Search
  regressions and the production simulator build pass. Dated bundles and memory observations are in PROGRESS. Fixture
  creation/persistence and actor hops are outside the measured interval. No timing
  threshold, frame-rate or battery-life claim is made. Everyday tests skip this
  benchmark before creating its fixture. Shared Rust/macOS/Linux code is unchanged.

## B-056 — Plan for Today suggests only tasks already planned for Today

- **Severity:** Medium. **Features:** F-031. **Status:** Verified for the reproduced
  iOS simulator Shortcuts flow; macOS builds/tests pass, but native Mac Shortcuts
  interaction remains unverified.
- **Reproduction:** create an unscheduled task, run Momentum's **Plan for Today** App
  Shortcut and open its task picker. The real iOS 27 Shortcuts sheet contained no task
  row, so the action could not perform its purpose. The entity query supplied `.today`
  suggestions, which excluded the unscheduled task that needed planning.
- **Expected/fix:** entity suggestions used by Plan, Complete and Open should offer
  active tasks from all current lists. `MomentumTaskQuery.suggestedEntities()` now
  requests the shared `.all` scope. Rust still owns list membership and mutations;
  completed/archive filtering and stale-ID validation are unchanged.
- **Regression evidence:** the real system picker now offers the unscheduled task and
  planning moves it into Today. The complete five-action Shortcuts suites pass 5/5 on
  iOS 26.5 (`/tmp/momentum-appintents-all-actions26.xcresult`, 147.090s) and iOS 27
  (`/tmp/momentum-appintents-all-actions27.xcresult`, 180.360s). The final portable
  mobile suite passes 119 tests, both native view/model suites execute 46 tests with
  two intentional availability skips and no failures, MomentumKit passes 136 tests,
  and the macOS Debug app builds. No physical device or user task store was used.

## B-057 — cold notification action crashes after applying its mutation

- **Severity:** High. **Features:** F-007/F-038. **Status:** Verified for cold and
  resident notification actions on iOS 26.5/27 simulators; physical/locked checks are
  user-deferred.
- **Reproduction:** schedule a reminder, let SpringBoard deliver it, terminate Momentum,
  expand the notification and tap **Done**. The shared engine completed the task and the
  delivered notification disappeared, then the cold-launched app terminated with
  `NSInternalInconsistencyException: Call must be made on main thread` while UIKit updated
  its background state-restoration archive.
- **Cause:** the async `UNUserNotificationCenterDelegate` response method allowed the
  system completion continuation to resume away from the main actor after awaiting the
  engine. UIKit then ran state-restoration work on that executor. The task mutation,
  delivery identity and notification plan were already correct.
- **Fix:** use the completion-handler delegate method, transfer its one-shot Objective-C
  callback to `MainActor`, perform the engine route and delivery removal there, and call
  the completion on the same actor for success and rejection paths. A Debug-only system
  notification marker keeps OS cold launches on the disposable test store with sync and
  Spotlight disabled; Release builds ignore it.
- **Evidence:** the pre-fix native flow reproduced the crash in
  `/tmp/momentum-notification-action26-r4.xcresult`. The final real-system suite drives
  native scheduling and actual Notification Center routes: cold body activation, cold
  Done and resident Snooze pass 3/3 on iOS 26.5
  (`/tmp/momentum-notification-system-final26.xcresult`, 343.071s) and iOS 27
  (`/tmp/momentum-notification-system-final27.xcresult`, 340.348s). Body activation opens
  the exact task, Done completes once after termination, and Snooze clears delivery while
  leaving the task open. Package tests verify shared-owner deduplication, the exact
  one-hour replacement plan and auto-archive handling. Final `prepare`
  and portable gates pass, and both native view/model suites execute 47 tests with two
  intentional availability skips and no failures. Background
  replenishment, Focus, locked-device and physical-device behavior remain separate.

## B-058 — editing the iOS morning-summary time can consume the wrong time

- **Severity:** Medium. **Features:** F-038. **Status:** Verified on the iOS 26.5
  and 27 simulators; physical delivery remains user-deferred.
- **Reproduction:** enable Morning Summary, then use the inline wheel picker to change
  both hour and minute. Each wheel wrote through immediately. If the intermediate hour
  plus the old minute was already in the past, reconciliation accepted that intermediate
  time for the current day before the user reached the intended minute.
- **Expected:** editing is provisional until one explicit save; scheduling observes the
  final hour and minute together. **Cause:** the inline `DatePicker` bound directly to two
  `AppStorage` values, while preference observation reconciled after each separate wheel
  update. The shared at-most-once ledger correctly prevented a second same-day summary.
- **Fix:** the time row now opens a native medium sheet with a staged wheel picker and
  SF Symbol Cancel/Save actions. Save commits the final hour and minute together; Cancel
  leaves scheduling unchanged.
- **Evidence:** the pre-fix iOS 27 system run
  (`/tmp/momentum-notification-summary27-r2.xcresult`) ended with the intended preference
  time saved but the isolated ledger already claimed the earlier intermediate time. The
  final real SpringBoard summary delivers the exact counts, opens Today, and preserves
  the chosen time across relaunch on iOS 27
  (`/tmp/momentum-notification-summary27-r3.xcresult`, 100.790s) and iOS 26.5
  (`/tmp/momentum-notification-summary26.xcresult`, 81.800s). Focused Notification
  Settings suites also pass 2/2 on iOS 27
  (`/tmp/momentum-notification-settings27-atomic.xcresult`) and iOS 26.5
  (`/tmp/momentum-notification-settings26-atomic.xcresult`). The diagnostic accessibility
  traversal now includes the staged sheet and completes all 32 screens on iOS 27
  (`/tmp/momentum-summary-accessibility27-r2.xcresult`); its 117 retained raw findings
  remain an inventory, not a blanket compliance pass. No device or personal store was used.

## B-059 — iOS project and tag destinations ignore their saved colors

- **Severity:** Low. **Features:** F-004/F-037. **Status:** Verified on iOS 26.5/27
  simulators for list and grouped-heading presentation; broader color-setting acceptance
  remains tracked by the iOS parity checklist.
- **Reproduction:** create a project or tag with Custom color enabled, then inspect Lists
  or group a task list by Project/Tag. Before the fix, every destination symbol and every
  grouped heading used the same secondary label color even though the shared Rust model
  supplied each saved color. Task-row metadata already honored saved colors.
- **Expected:** saved project/tag colors remain visible wherever the destination is
  represented, while accessibility settings can replace color with a readable neutral
  treatment. Color must not carry the only meaning.
- **Fix:** Lists now applies each saved color to the project/tag SF Symbol while retaining
  primary text, and grouped section headings apply the same saved color. Colorful Labels
  off, Increase Contrast and Differentiate Without Color all use the shared secondary-text
  fallback. Native trailing swipe Edit Project/Edit Tag actions expose the existing editors
  without changing the context-menu path.
- **Evidence:** the shipping UI creates custom-color project/tag destinations, verifies
  their saved switch state before and after relaunch, keeps a parent/subtask family and
  multi-selection/tag assignment intact, then checks the exact two-task project deletion
  scope. It passes on iOS 27
  (`/tmp/momentum-organization-foundation27-r7.xcresult`, 100.758s) and iOS 26.5
  (`/tmp/momentum-organization-foundation26-r1.xcresult`, 102.163s test time). The retained
  iOS 27 screenshot was visually inspected at
  `/tmp/momentum-org-r7-export/8EA604E8-0352-4B47-8A1D-DFDDD220CFCA.png`. No physical device
  or personal store was used; Linux/macOS were unaffected by these iOS presentation files.

## B-060 — iOS tag autocomplete lacks complete native keyboard control

- **Severity:** Medium. **Features:** F-005/F-039. **Status:** Verified for the iOS 26.5/27
  simulator shipping flow and native responder-command boundary; physical keyboard testing
  is user-deferred with all physical-device work.
- **Reproduction:** enter `#a` when `alpha` and `alpine` exist. Before the fix, touch could
  accept a suggestion, but autocomplete exposed no selected row and did not handle Up/Down,
  Tab, Return or Escape. A multiline SwiftUI field also made the insertion cursor unavailable
  to reliable Unicode-aware completion replacement.
- **Expected:** the first suggestion is selected, arrows move selection, Tab/Return accept,
  Escape dismisses, touch remains equivalent and ordinary/Unicode text entry is preserved.
- **Fix:** the quick-add field now uses a native, dynamically sized `UITextView` bridge with
  explicit UTF-16/Unicode-scalar cursor conversion. Priority `UIKeyCommand` and HID paths own
  autocomplete keys only while suggestions are visible; the text delegate handles native
  Return input. Rows expose selected semantics plus a checkmark and restrained accent
  background, and touch calls the same completion path.
- **Evidence:** pre-fix `/tmp/momentum-tag-autocomplete-red27.xcresult` fails because no
  suggestion is selected. Final shipping suites pass 2/2 on iOS 27
  (`/tmp/momentum-quick-add-foundation27-r2.xcresult`, 80.567s) and iOS 26.5
  (`/tmp/momentum-quick-add-foundation26-r2.xcresult`, 78.330s), including parser persistence,
  Down Arrow, Tab, native Return input and touch. The native view regression dispatches the
  priority Return/Escape commands in
  `ios/DerivedData/TestReports/views-1789719784251416000.xcresult` and
  `views-1789719810528016000.xcresult`. XCUITest did not deliver direct Return/Escape
  `typeKey` events to the multiline view, so that simulator harness limitation is not
  represented as a physical-keyboard pass. Unicode replacement, rejected-draft retry and
  More Details parsing pass 3/3 in `/tmp/momentum-quick-add-regression27-r1.xcresult`.

## B-061 — iOS task editor project picker omits its selected accessibility value

- **Severity:** Medium. **Features:** F-006/F-020. **Status:** Verified on the iOS 26.5
  and 27 simulators for source-aware creation; spoken VoiceOver acceptance remains open.
- **Reproduction:** open a project, start Add task, choose More Details and inspect the
  Project picker through accessibility. The form selected the source project correctly,
  but the picker exposed an empty value instead of its visible project name.
- **Expected:** assistive technology and UI automation can determine the selected project
  before the task is created. **Cause:** the SwiftUI picker relied on implicit semantics;
  after assigning a stable identifier, its selected value was not preserved by the
  resulting accessibility element.
- **Fix:** the picker now supplies both a stable `task-editor-project` identifier and an
  explicit accessibility value resolved from the current project ID. Task rules and
  persistence remain in the shared Rust engine.
- **Evidence:** the pre-fix iOS 27 shipping run fails at the empty project value in
  `/tmp/momentum-task-creation-context27-red-r2.xcresult`. The complete Today, project,
  tag, Morning and Evening creation matrix passes 3/3 on iOS 27
  (`/tmp/momentum-task-creation-context27-final.xcresult`, 76.019s) and iOS 26.5
  (`/tmp/momentum-task-creation-context26-final.xcresult`, 76.733s), with no failures,
  skips or runtime warnings. No physical device or personal store was used.

## B-062 — creating a tag from task actions retains the completed selection

- **Severity:** Low. **Features:** F-004/F-010. **Status:** Verified on iOS 26.5/27
  simulators; no desktop/shared-core behavior changed.
- **Reproduction:** select a parent/subtask family, open Actions, choose New Tag, enter a
  name and Add. The shared engine creates and applies the tag, but the Actions control
  remains selected. Choosing an existing tag through the adjacent menu clears selection.
- **Expected:** every completed selection action returns the list to ordinary browsing.
  Cancel may retain selection so the user can choose a different action.
- **Cause/fix:** the new-tag alert awaited the same organization command but omitted the
  selection cleanup supplied to `TaskActionsMenu`. It now clears the selected IDs and
  exits edit mode after the awaited operation, matching existing-tag behavior.
- **Evidence:** the first expanded iOS 27 organization run reaches the new `trip` tag but
  fails because Actions remains visible at line 67
  (`/tmp/momentum-organization-new-tag27-r1.xcresult`). The complete project/tag family,
  relaunch and deletion flow then passes on iOS 27 in 118.447s
  (`/tmp/momentum-organization-new-tag27-r2.xcresult`) and iOS 26.5 in 118.510s
  (`/tmp/momentum-organization-new-tag26-final.xcresult`). Only disposable simulator data
  was used.

## B-063 — selection mode prevents the iOS task drag from beginning

- **Severity:** Medium. **Features:** F-011/F-012/F-013. **Status:** Verified for
  shipping row reorder and project/tag/day destination gestures on iOS 26.5/27
  simulators; assistive Move Up/Down invocation remains a separate acceptance gap.
- **Reproduction:** enter Select Tasks, select a row and drag it or a multi-selection.
  SwiftUI List owns the edit-mode gesture and the row's `.draggable` session never begins,
  so the existing `TaskTransfer` never reaches any drop destination.
- **Expected:** a selected row exposes a clear 44-point reorder handle that starts the
  same single/multiple payload, while ordinary non-selection rows keep native drag.
- **Cause/fix:** SwiftUI's edit-mode List gesture competes with the row drag modifier.
  Selection mode now presents a small `UIViewRepresentable` SF Symbol handle backed by
  `UIDragInteraction`. It exports the shared custom-UTI JSON plus the same plain-text
  proxy as `TaskTransfer`; its Dynamic Type symbol, secondary tint and localized label
  retain native accessibility behavior. Rust still owns ordering and Undo.
- **Platform assessment:** Linux/macOS are unaffected by this UIKit/List interaction;
  iOS is Verified for row reorder; Android remains Planned.
- **Evidence:** `/tmp/momentum-drag-drop27-red.xcresult` reproduces the absent drag
  session. The real native gesture then moves one task and a two-task selection, preserves
  their relative order and restores both with one Undo on iOS 26.5
  (`/tmp/momentum-drag-drop26-destinations-final-r2.xcresult`) and iOS 27
  (`/tmp/momentum-drag-drop27-destinations-final-r2.xcresult`).
  Focused shared-core reorder tests pass for Manual Order, grouping, descending order,
  hidden neighbors and invalid destinations. No physical device or personal data was used.

## B-064 — URL and paragraph imports discard their source list

- **Severity:** Medium. **Features:** F-014/F-015. **Status:** iOS Verified on 26.5/27
  simulators; shared fix Implemented for Linux/macOS with native desktop reruns outstanding.
- **Reproduction:** paste or drop a URL or long paragraph while Today, a project, a tag or
  a day-period list is active. Multiple short lines use that view, but URL/paragraph
  branches create the task in Inbox with no source metadata.
- **Expected:** every parsed text shape keeps the same destination; URL content becomes
  Notes and its readable host/path becomes the title.
- **Cause/fix:** `add_from_text` passed its `View` only to the short-lines branch.
  A view-aware notes helper now applies URL and paragraph tasks through `add_task` with
  the supplied destination before adding Notes or an explicit due date. The public
  explicit `add_task_with_notes` behavior remains unchanged.
- **Evidence:** the new core assertions fail before the fix because URL `due_day` is nil;
  `/tmp/momentum-paste27-r2.xcresult` independently shows the URL absent from Today while
  feedback incorrectly suggests success in that list. The focused parser regression and
  final two-runtime shipping suites above now pass, creating two Today lines and a Today
  URL task whose Notes retain `https://example.org/mobile-drop/`. Cross-app drag and marked-text IME
  acceptance remain open. No physical device, account or personal store was used.

## B-065 — selection-mode context menu captures long-distance task drags

- **Severity:** Medium. **Features:** F-011/F-013. **Status:** Verified on iOS 26.5/27
  simulators; desktop and shared-core behavior are unaffected.
- **Reproduction:** enter Select Tasks, select a row and drag its handle far enough toward
  a project, tag or day destination. The row's ordinary browsing context menu opens and
  consumes the gesture before the native task payload reaches the destination.
- **Expected:** selection-mode drags reach the visible destination strip. The selected
  task commands remain available from the Actions button.
- **Cause/fix:** the row kept a context-menu recognizer while SwiftUI List edit mode and
  the UIKit drag handle were active. Row context-menu content is now present only outside
  edit mode, where it remains useful; selection mode already exposes the same commands in
  its dedicated Actions menu.
- **Evidence:** the first iOS 27 project-destination attempts opened the context menu
  (`/tmp/momentum-drag-destinations27-green-r1.xcresult` and `-r2.xcresult`). After the
  fix, the expanded iOS 27 scheme passes all three tests
  (`/tmp/momentum-drag-drop27-destinations-final-r2.xcresult`), including a two-task project
  move plus tag and Morning moves. The matching iOS 26.5 scheme passes all three tests
  (`/tmp/momentum-drag-drop26-destinations-final-r2.xcresult`). Only disposable simulator
  data was used.

## B-066 — creating a case-insensitive duplicate tag mutates the existing tag

- **Severity:** Medium. **Features:** F-004. **Status:** Verified on iOS 26.5/27
  simulators; shared Rust identity behavior is unchanged.
- **Reproduction:** create a custom-color tag named `outdoors`, then use the standalone
  New Tag editor to save `OUTDOORS` without a custom color. The shared engine correctly
  returns the existing tag ID, but the editor treats that ID as newly created and sends
  an update that changes the existing spelling and clears its color.
- **Expected:** duplicate creation is idempotent. It must retain the existing tag's
  title and metadata and must not add another tag.
- **Cause/fix:** the editor could not distinguish a newly created ID from an ID resolved
  by Rust's case-insensitive identity rule. `EngineWorker.createTagIfAbsent` now captures
  existing IDs, asks Rust to resolve/create the tag, and reports whether the returned ID
  was new. Swift does not reproduce trimming or matching rules. The standalone editor
  applies metadata only when the ID was newly created; normal editing still renames,
  recolors and clears colors.
- **Evidence:** the focused package regression first failed to compile because the
  create-only result did not exist, then passes 1/1; all organization package tests pass
  7/7 and the full mobile lane passes 126 tests/21 suites. The expanded shipping flow
  passes on iOS 27 in 180.056s
  (`/tmp/momentum-organization27-final-r2.xcresult`) and iOS 26.5 in 187.383s
  (`/tmp/momentum-organization26-final-r2.xcresult`). It proves one case-insensitive tag,
  retained lowercase spelling and retained custom-color state using disposable stores.

## B-067 — saved Nextcloud section headers use secondary ink and trigger a stale native contrast report

- **Severity:** Medium. **Features:** F-020/F-025. **Status:** Verified on iOS 26.5/27
  simulators for the rendered contrast and shipping saved-connection flow; spoken
  VoiceOver and hosted Nextcloud remain separate open acceptance boundaries.
- **Reproduction:** save a Nextcloud connection, scroll the production form to
  `Connection Saved`, and run the native contrast audit. The `Connection` and `Options`
  section headers inherited the app's secondary-text token. After changing the visible
  header ink, XCTest could still report the old `Connection` contrast result even though
  the element screenshot contained primary black ink on the system grouped background.
- **Expected:** section headings use readable semantic primary ink in both appearances.
  A framework audit mismatch must remain narrowly bounded and must never hide a visible
  contrast failure or another element/type.
- **Fix:** both section headers now use `Color.primary`. The one accepted XCTest finding
  must be a contrast issue for the exact static-text label `Connection`, and the test
  independently rasterizes that element, finds its dominant background and requires a
  rendered contrast ratio of at least 7:1 with a material amount of foreground ink.
- **Evidence:** the original iOS 27 shipping matrix passed 7/8 cases and reproduced the
  header finding in `/tmp/momentum-nextcloud-shipping27-r1.xcresult`. The corrected exact
  audit passes in 20.762s at `/tmp/momentum-nextcloud-accessibility27-r5.xcresult`.
  The complete eight-case iOS 26.5 matrix passes at
  `/tmp/momentum-nextcloud-shipping26-r1.xcresult`; its database records 8/8 Success,
  including normal, English AX5 light and German AX5 dark audits. The broader sync
  package, view and five-case loopback transport lanes also pass on both runtimes.

## B-068 — installed sync QA host claims the production task URL schemes

- **Severity:** Medium. **Features:** F-031/F-040. **Status:** Verified on iOS 26.5/27
  simulators; the shipping app's URL registration and task behavior were unchanged.
- **Reproduction:** install the loopback `MomentumSyncTestHost`, then run the production
  Safari `momentum://add` acceptance. Before the fix, iOS could activate the QA host
  instead of `com.codedbydan.Momentum.ios`; the production test then waited on UI owned
  by the wrong process.
- **Expected:** only the shipping app registers `momentum` and `superproductivity`.
  The loopback host may export the shared public task UTType for Spotlight activities,
  but it must not own application URL schemes.
- **Cause/fix:** the fixture source excluded the production plist, but XcodeGen still
  resolved its generated target through `Momentum/Info.plist`. The host now has an
  explicit fixture-only plist with its display/scene metadata and exported task UTType,
  and no `CFBundleURLTypes`. Project generation preserves that boundary.
- **Evidence:** `/tmp/momentum-automation27-final-r1.xcresult` passes all five Shortcuts
  cases and fails only the URL case while the installed fixture plist contains both
  schemes. After the fix, the exact URL flow passes 1/1 on iOS 27 at
  `/tmp/momentum-url27-collision-fix-r1.xcresult` and iOS 26.5 at
  `/tmp/momentum-url26-collision-fix-r2.xcresult`. The rebuilt fixture plist was read
  back without `CFBundleURLTypes`; corrected Spotlight activation passes 2/2 with zero
  runtime warnings on each runtime at `/tmp/momentum-spotlight-activation27-r2.xcresult`
  and `/tmp/momentum-spotlight-activation26-r2.xcresult`.

## B-069 — iOS saves malformed or insecure Nextcloud connection records

- **Severity:** Medium. **Features:** F-025/F-027. **Status:** Verified for the
  connection/persistence/transport unit boundary on iOS 26.5/27; hosted acceptance
  remains part of F-025.
- **Reproduction:** enter a missing-scheme URL, remote HTTP URL, embedded credentials,
  query or fragment and choose Save Connection. The draft previously replaced the
  atomic Keychain record, then failed only after a transport attempt. A malformed
  record left by that path could also remain eligible for automatic sync.
- **Expected:** reject unsafe or ambiguous fields before cancellation or persistence,
  preserve the previous saved connection, identify the field in the UI and never admit
  an invalid saved record to transport.
- **Fix:** `NextcloudConnection.validated()` trims URL/user/folder presentation input,
  preserves password bytes, requires HTTPS except for localhost/IPv4/IPv6 loopback,
  and rejects user-info/query/fragment URLs. `NextcloudSyncState` validates before
  draining or saving, blocks invalid legacy records and presents a wrapping semantic
  error at accessibility sizes.
- **Evidence:** 24 focused connection/state tests pass; complete portable suites pass
  136/136. The five-case real loopback Swift/UniFFI/Rust WebDAV suite passes on iOS 27
  at `ios/DerivedData/TestReports/transport-1789754174230200000.xcresult` and iOS 26.5
  at `ios/DerivedData/TestReports/transport-1789754194672519000.xcresult`. The AX5
  error component passes phone/iPad widths. No hosted credentials or device were used.

## B-070 — asynchronous mobile form errors do not receive accessibility focus

- **Severity:** Medium. **Features:** F-006/F-017/F-020. **Status:** Implemented;
  rendered layout and focus wiring compile on iOS 26.5/27, while spoken VoiceOver
  focus order remains manual acceptance.
- **Reproduction:** make Quick Add, task save, recurrence save or project move fail.
  The visible error appeared without deterministic accessibility focus. The estimate
  field's visible correction text was also a separate element with no field hint.
- **Expected:** asynchronous failures become discoverable without color or a reading
  deadline, wrap at the largest text size, and keep inline correction guidance associated
  with the field a person is editing.
- **Fix:** one reusable `AccessibleErrorMessage` provides semantic error ink, an SF
  Symbol and an adaptive AX layout. Each asynchronous flow moves accessibility focus
  to the new error. Estimate retains keyboard focus and supplies the same correction as
  its accessibility hint. Nextcloud validation uses the same semantic presentation.
- **Evidence:** the production component renders within phone and iPad widths at AX5;
  the full unit lane passes 136 package plus 52 native tests on both supported simulator
  runtimes. Actual VoiceOver speech, rotor order and alternate input are not inferred.

## 2026-09-18 — iOS unit-only testing regression assessment

Removing the XCUITest targets is an approved testing-architecture change, not a
product bug or evidence that prior interaction findings were fixed. The replacement
suite covers portable behavior, production SwiftUI layout/rendering, accessibility
metadata, model integration, Keychain and isolated loopback transport through unit
targets. A repository-shape regression fails if an iOS UI-testing bundle,
`XCUIApplication`, or a retired UI-test source directory is added again. B-045–B-047
remain In progress because unit tests cannot establish hardware-key dispatch; other
gestures, system prompts, external-app flows and spoken assistive technology retain
their explicit manual acceptance boundaries.

## B-071 — iOS LibreSync groundwork is not reachable from the app

- **Severity:** High. **Features:** F-026/F-027/F-028. **Status:** Implemented; real
  two-peer simulator transport is Verified on iOS 26.5/27, while separate-device
  Bonjour/native interaction remains open.
- **Reproduction:** before the 2026-09-18 scope update, Settings exposed only Off and
  Nextcloud even though the mobile package contained a serial nearby lifecycle and the
  shared core could run LibreSync.
- **Expected:** people can select LibreSync, understand its state, pair and unlink
  devices, sync explicitly and recover without starting a second provider.
- **Fix:** Settings now exposes one Off/Nextcloud/LibreSync picker, status and recovery;
  the native device screen shows this device, a temporary six-digit code, discovered and
  linked peers, a one-time-code link sheet and destructive unlink confirmation. The app
  composes a Keychain-backed core runtime after opening the one shared worker. Provider,
  foreground and restore transitions serialize stop/drain/start, and local edits coalesce
  before a nearby exchange.
- **Evidence:** 140 portable tests pass, including lifecycle/event/provider/edit
  scheduling; the AX5 phone/iPad LibreSync screen renders in the native unit target.
  Real Swift/UniFFI/Rust peers pass pairing, bidirectional task changes, callbacks,
  foreground restart, stable identity/certificate reuse and unlink in both simulator
  runtimes. No physical device or personal store was used.

## B-072 — SwiftUI reconciliation can cancel Quick Add IME composition

- **Severity:** Medium. **Features:** F-005/F-015/F-020. **Status:** Verified at the
  production UIKit text boundary on iOS 26.5/27 simulators.
- **Reproduction:** while `UITextView` owns marked text, a model/suggestion update with
  a different value replaced the complete text storage and insertion point.
- **Expected:** marked text remains owned by the active input method until it commits;
  suggestion and paste updates must not discard the marked range.
- **Fix/evidence:** `QuickAddTextInput` now defers SwiftUI storage reconciliation while
  `markedTextRange` exists. The native regression creates real marked text, applies the
  production synchronizer and verifies both the composing text and range survive.
  Interactive third-party-keyboard acceptance remains manual.

## B-073 — cold iOS 27 Search, Select All and Delete depend on scene-command delivery

- **Severity:** Medium. **Feature:** F-023. **Status:** Implemented; manual external-key
  simulator acceptance remains before B-045/B-046/B-047 can be closed.
- **Reproduction:** the retired app-driving suite observed iOS 27 cold Command-F and
  Command-A failures and an iOS 26.5 Command-Delete failure even when the SwiftUI command
  was enabled; the shared binding and action logic were correct.
- **Expected:** these primary actions reach the same current model/list context on a cold
  launch while editors retain standard text behavior.
- **Fix:** public SwiftUI `onKeyPress` fallbacks route configured-modifier Search, fixed
  Command-A Select All and configured-modifier Delete through the existing action contexts.
  They return `.ignored` when the app/list action is unavailable, allowing native text
  input and system shortcuts to continue through the responder chain.
- **Evidence:** focused native unit tests prove cold Search state/focus revision, selection,
  configured Delete and wrong-modifier rejection; the complete 59-test view lane passes on
  iOS 26.5/27. Unit dispatch does not establish an external keyboard event, so the older
  bug IDs remain In progress rather than being waived.

## B-074 — iOS installed builds report a stale hardcoded marketing version

- **Severity:** Low. **Feature:** F-039. **Status:** Verified on dip17pm.
- **Reproduction:** build and install the current iOS target, then read the installed-app
  inventory. The project declares marketing version `0.4.0`, but the app reports `1.0`.
- **Expected:** the installed bundle reports the marketing and build versions defined by
  Xcode build settings so generated projects and release configuration stay authoritative.
- **Cause/fix:** `Info.plist` hardcoded both version strings. It now references
  `$(MARKETING_VERSION)` and `$(CURRENT_PROJECT_VERSION)`, and the XcodeGen source
  declares the same substitutions so project regeneration cannot restore the literals.
- **Evidence:** the signed Debug device build succeeds and `devicectl` reads back Momentum
  `0.4.0` build `1` from dip17pm after replacement installation. On 2026-09-19 a
  pre-install bundle readback caught the hardcoded value after it reappeared in the dirty
  worktree; the settings references were restored, a permanent runner regression was
  added, and a fresh signed install/readback again reports `0.4.0` build `1`. The sidebar
  follow-up exposed the generator as the remaining source on 2026-09-19; regenerating the
  project and all 12 runner regressions now preserve the substitutions.

## B-075 — the first iOS task snapshot moves the floating Add Task control

- **Severity:** Low. **Feature:** F-019. **Status:** Implemented; automated presentation
  policy passes on iOS 26.5/27 simulators, with visual simulator inspection pending.
- **Reproduction:** launch Today while the first task snapshot is still loading. The
  ProgressView-to-list transition participates in the same animated content replacement
  as later refreshes, so the bottom-right Add Task control can visibly shift.
- **Expected:** the floating control remains in a stable bottom inset and the first
  snapshot is assigned without movement. Later refreshes may animate when Reduce Motion
  is off.
- **Cause/fix:** initial and subsequent snapshots shared one animated update path, and
  the control lived inside changing content geometry. `TaskSnapshotTransition`
  makes the first assignment immediate and respects Reduce Motion; the control now stays
  in a stable safe-area inset independent of the snapshot.
- **Platform assessment:** iOS only; Linux/macOS layouts and Android's planned client are
  unaffected.
- **Evidence:** `ViewRenderingTests` covers immediate first load, later smooth refresh,
  search and reduced-motion policy. The complete unit lane passes 152 package tests and
  80 hosted SwiftUI tests with two intentional skips on both supported runtimes. Both
  Debug simulator app builds succeed. No manual visual or physical-device claim is made.

## B-076 — the iOS task-list sidebar enters from the wrong edge and cannot remain visible

- **Severity:** Medium. **Features:** F-003/F-004/F-018. **Status:** Implemented;
  adaptive policy and hosted-view boundaries pass on iOS 26.5/27, while manual
  portrait/landscape interaction remains pending.
- **Reproduction:** open Lists from Today on a phone. The old trailing pushed picker
  enters from the detail side and cannot become a persistent leading column on a capable
  landscape iPhone.
- **Expected:** compact layouts reveal a native leading sidebar and return to detail after
  selection. A regular-width container that fits the approved sidebar and accessible
  detail minima keeps both columns visible, without device-name checks.
- **Cause/fix:** Today used an independent `NavigationStack` destination rather than an
  adaptive workspace. `TaskWorkspace` now owns a `NavigationSplitView`, preserves its
  selected list across tab/size changes, prefers detail in compact mode and falls back to
  Today when a selected Morning, Evening, project or tag disappears.
- **Platform assessment:** iOS only; desktop navigation is unchanged.
- **Evidence:** package policy tests cover exact width/size-class boundaries, persistence,
  compact selection and each disappearing-context fallback. Hosted SwiftUI tests cover
  portrait, capable landscape and the combined-width boundary on both runtimes; app
  builds succeed. The leading-edge gesture and visibly persistent landscape columns have
  not yet been manually inspected.

## B-077 — Quick Add does not open the task it just created

- **Severity:** Medium. **Features:** F-005/F-006. **Status:** Implemented; actor and
  sheet-sequencing integration tests pass on iOS 26.5/27, with manual sheet-transition
  inspection pending.
- **Reproduction:** submit a valid single-line or multiline Quick Add entry. The sheet
  dismisses after creation but leaves the person on the list instead of opening the
  standard editor for the newly created task.
- **Expected:** successful single creation opens that task; multiline import opens the
  last created task. Failure or an unchanged outcome retains the current draft. A queued
  notification route waits until the editor closes.
- **Cause/fix:** the mutation returned only a changed outcome, so the UI had no durable
  identity to route after dismissal. Actor-isolated worker helpers now capture
  `lastAddedId()` before another caller can enter the worker. Quick Add returns a typed
  success, stores the identity in root-owned pending-editor state, dismisses, and lets
  the root present one standard `TaskEditor` before notification routing resumes.
- **Platform assessment:** iOS only; shared task rules are unchanged.
- **Evidence:** package tests cover concurrent single creation, multiline last identity,
  unchanged and persistence-failure outcomes. Native integration tests cover exact editor
  routing and queued notification ordering. Both complete unit lanes and app builds pass;
  no XCUI, ViewInspector or physical device was used.

## B-078 — Sync Settings starts redundant asynchronous work during back navigation

- **Severity:** Medium. **Features:** F-025/F-028. **Status:** Implemented; lifecycle
  integration tests pass on iOS 26.5/27, with manual back-transition inspection pending.
- **Reproduction:** push Sync Settings and immediately navigate back while status is
  loading. The page launches its own load/status task even though `MobileAppModel` already
  owns foreground refresh, so page disappearance can compete with the same state work.
- **Expected:** the app model remains the single lifecycle owner and Sync Settings only
  observes published state.
- **Cause/fix:** a page `.task` duplicated root-owned loading. It was removed; provider
  changes retain the existing drain/cancellation barriers and foreground refresh remains
  in `MobileAppModel`.
- **Platform assessment:** iOS only; transport and desktop provider behavior are unchanged.
- **Evidence:** `SyncSettingsIntegrationTests` mounts and dismisses the production screen
  while asserting no page-owned persistence/status load. Both native unit lanes and both
  simulator app builds pass. Hosted Nextcloud/TLS and physical navigation remain outside
  this evidence.

## B-079 — keyboard shortcut discovery is exposed on iPhone

- **Severity:** Low. **Feature:** F-023. **Status:** Implemented; capability policy and
  compiled production routing pass on iOS 26.5/27, with manual iPad presentation pending.
- **Reproduction:** inspect Settings, Help or app commands on iPhone. The app exposes
  keyboard shortcut discovery intended for the iPadOS hardware-keyboard experience.
- **Expected:** iPad shows the discovery row, help, scene commands and fallback handlers;
  iPhone omits them while ordinary software-keyboard editing remains unchanged.
- **Cause/fix:** shortcut presentation had no injected platform-capability boundary.
  `MobilePlatformCapabilities` now receives the UIKit interface idiom and gates all four
  entry points through one testable policy.
- **Platform assessment:** iOS/iPadOS presentation only; desktop shortcuts are unchanged.
- **Evidence:** package tests prove phone omission and iPad inclusion; production sources
  compile in both native unit lanes and app builds. External-key delivery and an actual
  iPad help presentation remain manual under F-023.

## B-080 — the iOS accent checkmark does not follow the live selection

- **Severity:** Low. **Features:** F-019/F-020. **Status:** Implemented; binding and
  accessibility metadata pass on iOS 26.5/27, with the full visual trait matrix pending.
- **Reproduction:** change the accent preview while the Appearance list is mounted. The
  visible color updates, but a row-local checkmark can remain on the previous choice.
- **Expected:** the resolved selected color ID drives one trailing native checkmark and
  selected accessibility trait immediately; the swatch remains a separate cue.
- **Cause/fix:** row presentation did not derive all selection state from the live
  resolved binding. The extracted accent row now uses that identity for its checkmark,
  trait, restrained transition and selection feedback.
- **Platform assessment:** iOS only; the approved palette and default orange are unchanged.
- **Evidence:** a mounted production-row regression changes the binding and checks public
  UIKit accessibility metadata. Palette/motion coverage and both unit lanes pass. Manual
  light/dark, Increase Contrast and Dynamic Type inspection remains pending.

## B-081 — completion and selection controls compete in iOS selection mode

- **Severity:** Medium. **Features:** F-010/F-020. **Status:** Implemented; hosted row
  semantics pass on iOS 26.5/27, with manual touch/assistive inspection pending.
- **Reproduction:** enter Select Tasks. Each row retains its ordinary completion circle
  beside the native list-selection indicator, presenting two nearby controls with
  different state-changing meanings.
- **Expected:** selection mode exposes only the native row-selection affordance and bulk
  Actions menu. Browsing mode retains completion and its accessibility actions.
- **Cause/fix:** `TaskRow` did not distinguish browsing from selection presentation. An
  explicit row mode now omits the completion control and action in selection while
  preserving ordinary completion behavior.
- **Platform assessment:** iOS only; shared bulk/completion rules are unchanged.
- **Evidence:** hosted SwiftUI tests assert that the selection row omits and the browsing
  row retains the production completion control. Selection feedback integration tests,
  both complete unit lanes and both app builds pass. No spoken assistive or physical
  device claim is made.

## B-082 — Morning & Night can label tomorrow tasks as Today

- **Severity:** Medium. **Features:** F-003/F-037. **Status:** shared-core behavior
  Verified by fresh Rust/CLI/Apple consumer automation; native Linux runtime remains
  unverified on this Mac.
- **Reproduction:** select Morning & Night grouping in Upcoming, a project, a tag or
  search containing an untagged task due tomorrow. The generic grouping fallback maps it
  to `TaskGroup::Today`, producing a false Today heading.
- **Expected:** Morning & Night sections exist only where Today/Morning/Evening describe
  the active day view. Other views keep one plain section and show due-day metadata on rows.
- **Cause/fix:** `task_group` used Today as the fallback outside a Today-style view, and
  `group_listing` assumed every row must receive a group. The former now returns no group
  outside day views; the latter preserves the original plain listing in that case.
- **Platform assessment:** shared core affects Linux, macOS, iOS and CLI consumers;
  Android remains Planned.
- **Evidence:** the focused Rust regression covers Upcoming, project, tag and search plus
  preserved Today/Morning/Evening, completed, family, ordering and Undo behavior. Both
  workspace Rust commands, the 23-test CLI suite, MomentumMobile's 152 tests,
  MomentumKit's 136 tests, generated Apple core build and both iOS app builds pass.
  `build-aux/test.sh` stops at `flatpak: not found`, so no Linux runtime pass is claimed.

## B-083 — compact Today presents a back affordance instead of an explicit sidebar control

- **Severity:** Medium. **Features:** F-018/F-020. **Status:** Verified on iOS 26.5/27
  simulators; corrected build installed and launched on dip17pm.
- **Reproduction:** open the compact Today workspace. The detail-first
  `NavigationSplitView` can synthesize a navigation back affordance for its hidden leading
  column, which communicates navigation history instead of the persistent list hierarchy.
- **Expected:** Today has a leading native button with the `sidebar.leading` SF Symbol and
  localized **Show lists** label. It reveals the sidebar from the leading edge. Ordinary
  pushed task destinations retain their standard back behavior.
- **Cause/fix:** the split view owned the compact column transition but left its root
  navigation item implicit. The first correction replaced the icon but only mutated
  `preferredCompactColumn`/column-visibility state. Apple documents that visibility is
  ignored after a split view collapses, and the device showed that this state change did
  not perform the navigation pop. The root `TaskScreen` now hides the synthesized Back
  item and its replacement button invokes the environment's native dismiss action. The
  split view then reports the resulting sidebar state through its binding. Nested screens
  do not receive the replacement button and retain native back navigation.
- **Evidence:** a public-API `UIHostingController` regression renders the real compact
  workspace, finds the single actionable leading control, asserts no Back accessibility
  element, retains a screenshot, invokes the control, requires the sidebar marker to
  become visible and the detail marker to disappear, and verifies the resulting binding
  state. This stronger assertion failed against the installed implementation before the
  native-dismiss fix. Focused and complete unit lanes pass on iOS 26.5 and iOS 27. Each
  complete lane runs 152 portable tests in 23 suites plus 90 native tests with two
  intentional opt-in skips. The corrected signed `0.4.0` build `1` installs and launches
  on dip17pm, and CoreDevice confirms a live Momentum process. Touch interaction is not
  claimed from process readback, and no user data was inspected.

## B-084 — configured sync providers have no connection test

- **Severity:** Medium. **Features:** F-025/F-026/F-043. **Status:** Implemented; shared,
  portable-mobile and focused iOS 27 native tests pass. Hosted and separate-device checks
  remain explicit acceptance limits.
- **Reproduction:** save a Nextcloud connection or link a LibreSync peer. Sync Settings
  previously offered only a full task exchange, so credentials or reachability could not
  be checked independently.
- **Expected:** every selected, configured provider offers one accessible **Test Connection**
  action with testing, success and actionable failure states. Off shows no action.
- **Cause/fix:** the transport boundary exposed exchange but no non-mutating health action.
  Nextcloud now runs a cancellable authenticated PROPFIND probe and accepts a missing target
  collection after the account root succeeds. LibreSync starts or joins an outbound cycle
  and awaits only that cycle ID, including the event-before-return race. Neither path changes
  tasks, pending operations or persistent last-sync state.
- **Evidence:** `sp-sync` and `momentum-core` regressions cover read-only success, missing
  collection, auth, deadline, cancellation and no state mutation. MomentumMobile covers
  provider lifecycle, exact-cycle attribution, unrelated events, timeout and cancellation.
  The iOS 27 transport Nextcloud suite passed seven tests with one intentional hosted-TLS
  skip, and Sync Settings integration passed ten tests. See PROGRESS for result bundles.

## B-085 — task-list sync status consumes primary vertical space

- **Severity:** Medium. **Features:** F-028/F-043. **Status:** Implemented; unit and focused
  hosted-view coverage pass, with final visual simulator inspection recorded in PROGRESS.
- **Reproduction:** enable a provider and open Today. The previous status entry appeared
  near the top of the list and competed with tasks and the empty state.
- **Expected:** one compact, tappable line after list content says `Last synced just now`,
  a localized minute/absolute time, `Not synced yet`, `Syncing…`, or an attention state.
- **Cause/fix:** `TaskScreen` placed the full provider summary before task content. The
  replacement footer uses footnote styling, a 44-point target and a minute-cadence
  `TimelineView` only while visible; tapping it opens Sync Settings.
- **Evidence:** status formatting and production Sync Settings integration tests pass.
  Empty and populated layouts remain part of the complete two-runtime native matrix.

## B-086 — the floating Add Task button reserves a full-width bottom strip

- **Severity:** Medium. **Features:** F-019/F-020. **Status:** Implemented; build and hosted
  geometry regressions pass, with final visual simulator inspection recorded in PROGRESS.
- **Reproduction:** open a task list. The bottom safe-area inset keeps the capsule stable
  but paints and reserves the entire row, hiding useful canvas and blocking surrounding taps.
- **Expected:** only the bottom-trailing capsule occupies visible space; the last list row
  can still scroll clear of it and its position remains stable on first load.
- **Cause/fix:** the control lived in a full-width safe-area inset. It is now a trailing
  overlay with an invisible scroll-content bottom margin. Its white default-orange label,
  Increase Contrast behavior, accessible sizing, animation and haptic policy are unchanged.
- **Evidence:** the app builds and the existing floating-control/startup regressions cover
  stable first-load geometry, Dynamic Type and foreground contrast.

## B-087 — routine task-operation feedback remains persistent and screen-owned

- **Severity:** Medium. **Features:** F-017/F-019/F-020. **Status:** Implemented; deterministic
  timeout, exact undo and persistent-error tests pass on iOS 27. Two-runtime completion is
  recorded in PROGRESS.
- **Reproduction:** delete or update a task, then switch tabs. The old banner remained until
  dismissal and each retained task screen owned a copy.
- **Expected:** one root-owned Liquid Glass toast appears above navigation, ordinary text
  disappears after 3.5 seconds, exact Undo remains for 5 seconds, and `SaveFailed` stays
  focused with Dismiss.
- **Cause/fix:** feedback was an untyped message held by each task screen. `MobileAppModel`
  now owns a typed presentation with a unique ID, severity and optional undo batch. One
  cancellable sleep drives transient deadlines; the root overlay respects Reduce Motion,
  contrast and transparency settings and invokes `undo_batch` with the captured ID.
- **Evidence:** `TaskMutationIntegrationTests` passes nine tests on iOS 27, including
  replacement deadlines, exact-batch Undo and persistent save failure. Focused model tests
  use an injected sleep seam and do not wait on wall-clock deadlines.

## B-088 — saved project and tag rows lack filled color identity and task counts

- **Severity:** Low. **Features:** F-004/F-020. **Status:** Implemented; shared count and
  focused hosted-view regressions pass.
- **Reproduction:** open Lists with colored projects or tags. Previous symbols read as
  outline accents and rows did not show how many active task families they contained.
- **Expected:** the full `folder.fill` or `tag.fill` symbol uses the saved color and each
  row shows its localized unfinished-family count. Color is supplementary to shape/text.
- **Cause/fix:** SwiftUI iterated editing projections without canonical counts. The Rust
  sidebar projection now maps every direct member to its live root family, deduplicates
  parent plus child membership and excludes completed/archived roots. `ListPicker` consumes
  those entries and applies the existing Colorful Labels, Increase Contrast and
  Differentiate Without Color fallbacks.
- **Evidence:** shared-core tests cover child-only membership, deduplication and completion.
  The production hosted view test verifies filled SF Symbols, saved colors and accessible
  counts. Linux/macOS compile against the additive field; their current rows are unchanged.

## B-089 — ordinary task lists eagerly build the global search/archive index

- **Severity:** High. **Features:** F-001/F-003/F-004/F-039/F-044. **Status:** Verified
  for the shared-core behavior and iOS 26.5/27 first-snapshot boundary.
- **Reproduction:** open Today, Upcoming, a project or a tag with a large task store. Before
  producing the requested rows, `Engine::listing` lowercased every searchable field and
  deserialized both archive tiers even though those screens never consume the index.
- **Expected:** ordinary task lists perform only their requested projection. Search and
  Archive may build and reuse the invalidation-aware global index after a mutation.
- **Cause/fix:** the listing entry point created `SearchIndex` unconditionally and recurrence
  startup parsed every archived task just to check generated IDs. Index creation is now lazy
  for Archive (Search keeps its existing path), and recurrence checks archive entity-map keys.
  No second UI snapshot cache was added, avoiding stale duplicated task state.
- **Evidence:** a red/green core regression proves Today leaves the index absent and Archive
  creates it; another preserves archived-repeat deduplication without task deserialization.
  The Release unit/view benchmark measures a cold persisted-store reopen through the full
  first 1,000-task Today snapshot at 80.8 ms on iOS 27 and 67.0 ms on iOS 26.5 with a
  100 ms regression ceiling. See PROGRESS.

## B-090 — stale conditional iOS destinations remain on the loading spinner

- **Severity:** High. **Features:** F-003/F-004/F-018/F-039. **Status:** Verified on
  the iOS 26.5 and 27 simulator unit hosts; physical interaction was not rerun.
- **Reproduction:** expose Morning in the Today sidebar, then make that conditional
  destination disappear before its detail snapshot is accepted, such as through a sync
  commit or local-day change. Selecting Morning can remain on “Loading tasks” indefinitely.
  The same stale-destination path applies to Evening, deleted projects and deleted tags.
- **Expected:** every completed snapshot ends the loading state. Conditional destinations
  still fall back to Today or dismiss according to their navigation context.
- **Cause/fix:** `TaskScreen` returned on `viewExists == false` before assigning the completed
  snapshot. Workspace reconciliation was asked to change routes, but the disappearing detail
  still owned a nil snapshot and could retain its spinner. Snapshot assignment now happens
  first; the existing dismissal/fallback policy runs afterward.
- **Evidence:** the hosted SwiftUI regression failed before the change because no snapshot
  assignment arrived and the loading state timed out, then passed after the ordering fix.
  A second regression switches through the compact sidebar into a valid Morning list and
  verifies its task appears. The complete lane passes 156 portable tests plus 95 native
  view/integration tests with two intentional opt-in skips on both supported runtimes.
