<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# iOS parity acceptance

Prepared 2026-09-16 from desktop source at `7ec82e9`. The native iOS target and shared-core build are **In progress**. Unchecked cases
remain unverified for their full required scope. Architecture was approved with Today, Upcoming, Search and Settings tabs in the
[design proposal](superpowers/specs/2026-09-16-ios-parity-design.md).
These unchecked cases define evidence to collect, not implementation or verification.

For each completed case, record date, revision/dirty state, OS/device, fixture,
automated/native evidence and remaining limits in PROGRESS.md. A checkbox requires
the evidence named by the case, not merely a source implementation. Run native
workflows on both iOS 26 and 27; cover iPad adaptation separately. Shared Rust tests
prove rules; iOS boundary tests prove mapping; native inspection proves interactions.

**2026-09-17 user deferral:** finish the remaining work using simulators/local
checks and skip further dip17pm testing. Existing physical failures and evidence
remain recorded; simulator passes do not turn deferred physical acceptance into a
pass. Do not request another unlock or resume that device without new authorization.

**2026-09-18 testing decision:** the iOS project no longer contains XCUITest targets
or sources. Portable unit tests and in-process `UIHostingController` tests cover code,
presentation state, real SwiftUI layout and rendered pixels. Prior shipping-app and
system-UI results below remain historical evidence for the recorded dirty revision;
they are not an active regression lane. Gestures, keyboard routing, cross-app/system
presentation and spoken assistive technology require explicit manual simulator
evidence when affected. Do not infer those behaviors from the fast unit suite.

**2026-09-18 completion scope:** the user resumed LibreSync and the remaining parity
work, while excluding spoken VoiceOver acceptance and a real Low Power Mode toggle.
Validation remains simulator-only. Voice Control, Switch Control, external keyboard,
cross-app drag, scheduler and locked-device checks retain their explicit system limits.

## Sync, feedback and list polish — F-043/B-084–B-088

- [x] Every configured provider exposes one Test Connection action. Nextcloud uses a
  read-only authenticated probe and LibreSync awaits only its attributed outbound cycle;
  Off exposes no action and cancellation/provider changes clear stale results.
- [x] Enabled task lists end with one compact, tappable last-sync footer instead of a
  primary status block. Its minute cadence exists only while the footer is visible.
- [x] Add Task remains a stable bottom-trailing capsule without a full-width background;
  an invisible scroll-content margin lets the final row move clear of it.
- [x] One root-owned Liquid Glass toast replaces screen-owned persistent information:
  3.5-second information, 5-second exact-batch Undo and persistent focused SaveFailed.
- [x] Lists use filled folder/tag SF Symbols and canonical unique unfinished-family
  counts, with the established high-contrast and non-color fallbacks.
- [x] Complete native view/transport lanes pass on iOS 26.5 and 27: 93 hosted view tests
  with two opt-in skips and eight real loopback transport tests with one hosted-TLS skip
  per runtime. The portable lane passes 156 tests in 23 suites.
- [ ] Inspect the affected simulator layouts across appearance, contrast, transparency
  and accessibility text. Hosted Nextcloud credentials and separate-device LibreSync
  remain opt-in boundaries.

## Navigation and interaction corrections — B-075–B-082

- [x] The initial task snapshot is immediate and the floating Add Task control has a
  stable inset; later motion follows Reduce Motion (B-075).
- [x] Today uses an adaptive leading `NavigationSplitView`; policy and hosted-view tests
  cover compact detail preference, regular-width two-column presentation, rotation/tab
  persistence and fallback when Morning, Evening, project or tag disappears (B-076).
- [x] Compact Today replaces the synthesized Back affordance with one localized
  `sidebar.leading` control; the real rendered control opens the leading column and
  ordinary pushed destinations retain native back navigation (B-083).
- [x] Adding a task always opens the full task editor; the quick-entry sheet was removed
  so there is one creation flow. Queued notification routing resumes only after the
  editor closes (B-077).
- [x] Sync Settings observes root-owned lifecycle state without starting a duplicate
  persistence/status load during navigation (B-078).
- [x] Shortcut discovery, help, scene commands and key fallbacks are present for the
  injected iPad capability and omitted for iPhone (B-079).
- [x] Accent selection derives its trailing checkmark and selected accessibility trait
  from the live resolved choice (B-080).
- [x] Selection rows omit the completion control/action while browsing rows retain it
  and bulk Actions remains available (B-081).
- [x] Morning & Night grouping remains plain in Upcoming, project, tag and search while
  Today-style views preserve their Today/Morning/Evening sections and due-day metadata
  (B-082).
- [x] Fresh automated gate on dirty `task/ios-app`/`7ec82e93271f`: core/UniFFI prepare;
  152 portable plus 80 native tests with 2 intentional skips on each iOS 26.5/27 QA
  simulator; 10 runner tests; 23 CLI tests; both Rust workspace commands; 152
  MomentumMobile and 136 MomentumKit tests; Apple core build; both unsigned Debug app
  builds; changed-Swift parse; and `git diff --check`. No XCUI or ViewInspector ran.
- [ ] Manually inspect the eight affected flows on isolated/demo simulator data across
  portrait/landscape, light/dark, normal/Reduce Motion and representative Dynamic Type;
  include iPhone/iPad shortcut presentation and touch/assistive selection behavior.
  No physical-device claim is part of this pass. Linux runtime B-082 validation is also
  pending because `build-aux/test.sh` reports `flatpak: not found` on this Mac.

## Foundation, views and entry — F-001–006

- [x] Generated Rust FFI builds for an iOS device and simulator; no hand-edited bindings.
  2026-09-16 dirty `task/ios-app`: shared generator emitted Swift bindings and
  Mac/device/arm64-simulator/x86_64-simulator slices (`/tmp/momentum-badge-ffi.log`).
  Earlier signed device build/install/launch and native simulator builds are recorded
  in PROGRESS.md; this check covers generation/build, not complete app behavior.
- [x] Offline first launch, create/edit, force termination and relaunch preserve data.
  2026-09-18 dirty `task/ios-app`: the shipping isolated UI creates, completes,
  undoes and survives termination; a separate flow edits, saves, deletes, undoes and
  survives termination with the edited title. Both pass on iOS 26.5 and 27 in
  `/tmp/momentum-offline-persistence26.xcresult` and
  `/tmp/momentum-offline-persistence27.xcresult`. The portable engine test independently
  reopens its on-disk store. No account, transport, device or personal data was used.
- [x] iOS links no AppKit or desktop CLI service; macOS builds/tests still pass.
  2026-09-18 dirty `task/ios-app`: `otool -L` on the iOS 27 simulator app and
  debug dylib lists UIKit/SwiftUI and no AppKit; its symbol table contains none of
  MomentumKit's desktop AppState/CLI/external-change service symbols. MomentumKit
  passes 136 tests/32 suites and the macOS Momentum Debug app builds with Xcode 27.
- [x] Four native tabs preserve navigation state; mutations refresh every affected tab.
  2026-09-18 dirty `task/ios-app`: the shipping UI visits all four exact tabs, retains
  independent Today/Settings navigation stacks and a Search query, then proves a task
  created in Today refreshes Search and completion in Search refreshes Today without
  resetting those states. It passes on iOS 26.5 and 27 in
  `/tmp/momentum-tab-state26.xcresult` and `/tmp/momentum-tab-state27-r8.xcresult`.
- [x] Today includes unfinished overdue tasks, local timed due dates and completed section.
  2026-09-18 dirty `task/ios-app`: the shipping demo-isolated UI exposes the overdue
  task's local date and the timed task's native 12/24-hour value, then moves a completed
  task beneath the `Completed (1)` heading with a Reopen action. The exact flow passes
  on iOS 26.5 and 27 in `/tmp/momentum-today-foundation26.xcresult` and
  `/tmp/momentum-today-foundation27-r7.xcresult`.
- [x] Morning/Evening membership is exclusive; a disappearing selected context recovers.
  The same two-runtime flow opens the only Morning task, moves it to Evening through
  the native selection action menu, automatically returns to Lists when Morning stops
  existing, removes only the Morning day-period destination, and finds the task in
  Tonight. The separate user tag named Morning remains intact.
- [x] Upcoming offers 7/30 days and keeps task dates visible with every grouping mode.
  2026-09-18 dirty `task/ios-app`: the shipping UI starts with a seven-day window,
  excludes a disposable day-21 task, and keeps Tomorrow in the row's accessibility
  value under Morning & Night, None, Project, Tag and Time Estimate. Selecting 30 days
  reveals the day-21 task with its date still exposed and survives app relaunch. The
  exact flow passes on iOS 26.5 and 27 in
  `/tmp/momentum-upcoming-foundation26-r1.xcresult` and
  `/tmp/momentum-upcoming-foundation27-r1.xcresult`.
- [x] Project/tag views preserve subtasks, counts, saved colors and selection behavior.
  2026-09-18 dirty `task/ios-app`: the shipping UI creates a custom-color project and
  tag, keeps a parent and subtask together in both destinations, independently selects
  both rows as `Actions (2)`, applies the tag, and reports the exact two-task project
  deletion scope. Native swipe editing exposes both saved colors after app relaunch;
  deleting the disposable project leaves the tag editable. Project/tag list symbols and
  grouped section headings now use saved colors, with secondary-text fallbacks for
  Colorful Labels off, Increase Contrast and Differentiate Without Color. The exact flow
  passes on iOS 26.5 and 27 in
  `/tmp/momentum-organization-foundation26-r1.xcresult` and
  `/tmp/momentum-organization-foundation27-r7.xcresult` (B-059).
- [x] Task creation goes through the full editor with explicit tag and estimate fields;
  the quick-add text parser is no longer part of the iOS creation flow (F-005/B-060).
- [x] Tag autocomplete supports touch and hardware keyboard without breaking text entry.
  The same two-runtime shipping flow verifies selection state, Down Arrow, Tab, native
  Return input, Unicode-safe cursor replacement and touch acceptance. The native view
  lane dispatches priority Return/Escape responder commands; see B-060 for the simulator
  `typeKey` injection boundary.
- [x] New-task defaults match source context: project, Today/date and tag/day-period.
  Shipping More Details flows expose and persist the Today schedule, selected project,
  selected tag and Morning/Evening destination on iOS 26.5/27 in
  `/tmp/momentum-task-creation-context26-final.xcresult` and
  `/tmp/momentum-task-creation-context27-final.xcresult` (F-006/B-061).
- [x] Editor maps title, project, date, estimate, ordered tags/new tags and notes correctly.
  A shipping create/persist/relaunch/Search/reopen flow verifies the visible form on both
  runtimes; the fast shared-model regression verifies exact retained/new tag order in
  `/tmp/momentum-task-editor-mapping26-r1.xcresult` and
  `/tmp/momentum-task-editor-mapping27-r5.xcresult` (F-006).
- [x] Cancel preserves original values; save rejects empty title; note copy is available.
  Shipping acceptance copies a note through the system pasteboard, pastes it into the
  draft title, cancels without mutation, reopens the original task and verifies Save is
  disabled after clearing the title on iOS 26.5/27 in
  `/tmp/momentum-task-editor-safety26-r1.xcresult` and
  `/tmp/momentum-task-editor-safety27-r2.xcresult` (F-006).
- [x] Add subtask, duplicate and copy title work; subtasks inherit/move with their parent.
  One shipping flow copies the selected title through the system pasteboard, adds a
  subtask, moves the selected parent through the native project sheet, confirms both
  family members in the destination and duplicates the parent on iOS 26.5/27. A shared
  core regression separately verifies inherited project identity plus family move/Undo
  in `/tmp/momentum-task-family-actions26-r1.xcresult` and
  `/tmp/momentum-task-family-actions27-r2.xcresult` (F-006).

## Planning, recurrence and notifications — F-007–008, F-038

- [x] Local date/time entry round-trips across locales; clearing time restores plain date.
  The shipping editor accepts German 24-hour input at 14:30, persists it through an
  English relaunch, then removes only the time while retaining the scheduled day. The
  isolated flow passes on iOS 26.5 and 27 in `/tmp/momentum-planning-locale26-r1.xcresult`
  and `/tmp/momentum-planning-locale27-r5.xcresult`.
- [x] Reminder offsets: none, at time, 5/10/15/30 minutes, 1 hour and 1 day before.
  The shipping editor exposes and accepts every option, persists one day before, then
  clears the reminder when time is removed while preserving the due day on iOS 26.5/27.
- [x] Day moves clear scheduled time/reminder; undo restores the previous values.
  Existing two-runtime planning-command acceptance exercises Tomorrow/Next Week and
  Undo; the full shared-core suite verifies the exact time/reminder restoration rule.
- [x] Repeat editor supports daily/weekly/monthly/yearly, interval, start and paused state.
  The shipping editor switches through all four cycles, persists a paused monthly rule
  and reopens it on both runtimes. B-052 separately verifies interval change/persistence;
  portable recurrence tests verify start-date and complete draft round trips.
- [x] Weekly rejects zero weekdays; monthly supports date, last day and nth/last weekday.
  Shipping acceptance verifies weekday validation/recovery and all three monthly modes,
  including last Friday, on iOS 26.5/27.
- [x] Create/edit/stop repeat, deterministic instance identity and newest-missed catch-up.
  The native flow creates, reopens and stops a schedule; the shared Rust suite verifies
  deterministic identities, missed-day catch-up and the exact retained due date.
- [ ] Future notification planning follows the approved core contract while suspended.
  The production app registers a short local-only `BGAppRefreshTask` before launch
  completes, requests it no more than daily only after notification authorization,
  retries after expiration/failure, and never starts sync. Portable policy tests and
  iOS 26.5/27 production builds/isolated launches pass. Simulator cannot launch the
  system task, so execution while suspended remains a physical acceptance case.
- [ ] Permission denial/recovery, edits, cancellation, restart, capacity and horizon handling.
  Shipping-app tests use real notification authorization on iOS 26.5/27: denial shows
  `Off in iOS Settings` plus the Open Settings recovery action, while an independent
  fresh grant shows `Allowed` plus the planning horizon. Portable coordinator/core
  coverage verifies edit coalescing, denied cancellation, explicit cancellation,
  restart/readback recovery, the seven-day horizon and 59/60/61-request capacity edges.
  The Simulator does not expose notification authorization through `simctl privacy`,
  and its Settings switch did not accept the cross-app XCTest tap, so the uninterrupted
  denied-to-enabled Settings transition remains a manual/physical acceptance case.
- [x] Notification Done respects auto-archive; Snooze 1 hour re-arms; body opens the task.
  The final system suite passes 3/3 on each runtime, opens the exact task from a cold
  notification body, completes it with cold Done and keeps it open after resident Snooze.
  Bundles are `/tmp/momentum-notification-actions26-r1.xcresult` and
  `/tmp/momentum-notification-actions27-r1.xcresult`. The 36-test portable lane verifies
  the one-hour replacement plan, retry/deduplication and shared auto-archive behavior.
- [x] Cold and resident notification actions do not duplicate mutations or store owners.
  2026-09-18 dirty `task/ios-app`: real iOS 26.5/27 simulator notifications route
  cold body activation, cold Done and resident Snooze through the isolated shared engine. Done completes
  once after termination; Snooze clears delivery and keeps the task open. Package
  regressions verify one engine owner, callback deduplication and the one-hour plan.
  B-057 fixes the cold callback completion crash. Locked/physical acceptance remains
  user-deferred. Background replenishment is implemented with its system-execution
  acceptance tracked separately above.
- [x] Morning summary defaults off/08:00; time persists; disabled means no pending summaries.
  Preferences/package checks cover default/reset and cancellation. Focused native Settings
  tests verify opt-in and persistence, and real summaries at a custom saved time deliver
  on both simulator runtimes. B-058 stages the wheel value until an atomic Save.
- [x] Summary counts/at-most-once behavior survive restart, time-zone/DST and preference edits.
  Shared ledger regressions cover restart, edits and DST. Real SpringBoard delivery on
  iOS 26.5/27 shows `Today: 1 · Morning: 0 · Tonight: 0`, routes to Today and retains
  the chosen time after a cold relaunch. System background execution remains separate.
- [ ] Actual device delivery/background acceptance is recorded separately from simulator tests.

## Task actions, selection and transfer — F-009–015, F-017

- [x] Complete/reopen sets appropriate core state; auto-archive includes the task family.
  Shipping UI acceptance completes, reopens and auto-archives through the shared engine,
  then finds the archived child with its parent context and restores the whole family.
- [x] Archive Completed is disabled when inapplicable and undo restores the whole batch.
  Focused native acceptance checks both menu states and the single Undo result on iOS
  26.5/27; shared-core family and multi-task regressions cover the complete restore batch.
- [x] Archive remains read-only, includes both tiers, sorts newest completion first and pages 100.
  Native Archive/Search rows expose no edit controls. Core regressions combine young/old
  tiers in completion order, enforce read-only detail/mutations and verify 100-row paging.
- [x] Select all, clear/cancel selection and view switching do not include archived rows.
  B-048 verifies the separate Actions control above the tab bar on iOS 26.5/27
  phone simulators, including true maximum text, rendered white Add ink and menu
  completion. Shipping touch acceptance now also covers Select All, Done Selecting,
  independent list selection and Archive's complete exclusion from selection.
- [x] Bulk complete, delete, plan Today, Morning/Evening, Tomorrow and next Monday each undo once.
  Touch completion/deletion and the existing touch-selected native planning matrix pass
  on both runtimes; shared-core regressions verify each mutation is one undo batch.
- [x] Bulk move to project preserves families; Add Tag supports existing and new tags.
  Native family move/Undo and existing/new-tag assignment pass on iOS 26.5/27. B-062
  makes New Tag clear selection consistently after the shared engine applies it.
- [x] Plan/remove Today and Morning/Evening toggles use core decisions, including dual-tag input.
  Native selection actions and keyboard routes call the same organization commands;
  core regressions cover single/bulk direction, dual-tag precedence and one-step Undo.
- [x] Single/multiple reorder exercises the real native task-transfer payload.
  A UIKit drag source keeps the SF Symbol handle available while SwiftUI List owns
  selection mode. Shipping gestures move one task and a two-task selection, preserve
  relative order and Undo once on iOS 26.5/27 (F-012/B-063).
- [x] Task-transfer drag to project/tag/day exercises each native destination.
  The selection-mode destination strip accepts the same native payload on iOS 26.5/27:
  one gesture moves a two-task selection to a project, then separate gestures reach a tag
  and Morning. Shared-core regressions cover every project/tag/day-period route (F-011/F-013).
- [x] Reorder respects Manual Order, selected group, descending order and hidden neighbors.
  The native payload proves the platform boundary; focused shared-core regressions prove
  the projection-specific positions and one-step restoration (F-012/B-063).
- [ ] Archived drag is rejected; accessible move/reorder alternatives reach the same behavior.
  Archive omits drag/edit controls and the accessible Move action is verified. Named
  Move Up/Down actions are implemented, but their assistive native invocation remains open.
- [x] Multiline and URL paste create tasks through shared parsing and keep source context.
  App-owned system clipboard input creates two separate Today tasks and a URL task whose
  Notes retain the original URL on iOS 26.5/27. B-064 fixes URL/paragraph imports that
  previously discarded Today/project/tag/day-period context.
- [ ] External text/URL drop exercises a real cross-app native payload.
  Production `Transferable` decoding now passes real `NSItemProvider` text and URL unit
  boundaries; the spatial drag gesture from another app remains manual.
- [x] Ordinary paste replaces selection, preserves the insertion point and does not read
  the pasteboard when the task editor opens. Shipping acceptance passes on iOS 26.5/27.
- [x] Marked-text/IME composition preserves its range through suggestion and paste updates.
  The production `UITextView` now defers SwiftUI text-storage reconciliation while a marked
  range exists; a native simulator unit test retains the composition range and cursor.
- [ ] Undo remains reachable after feedback disappears; failures offer actionable recovery.

## Organization, search and presentation — F-004, F-016, F-018–023, F-037, F-042

- [x] Create/rename/recolor/delete project/tag; Inbox deletion is prevented; deletion confirms scope.
  The complete shipping flow verifies custom colors, parent/subtask preservation, exact
  two-task project deletion scope, tag deletion without task loss, Inbox protection and
  cold-relaunch persistence on iOS 26.5/27. A case-insensitive duplicate retains the
  existing spelling/color and creates no second tag (B-066).
- [x] Five grouping choices: Morning & Night, None, Project, first Tag and Time Estimate.
- [x] Grouping persists, preserves family placement and uses one exclusive grouping layer.
- [x] All six estimate intervals, missing tags/projects, duplicate names and fallback colors.
- [x] Manual/title/due/estimate/created sorting in both directions; preference survives restart.
- [x] Search matches every query word across title/notes/project/tags and includes subtasks/archive.
- [x] Search result project/tag links navigate; task/archive caps and truncation notes are accurate.
- [x] Rapid query changes cannot publish stale results; cleared search shows its normal prompt.
- [x] Empty, all-done, loading, offline, persistent error and retry states work in compact layouts.
- [x] The first Today load uses an adaptive skeleton without replaying during later refreshes.
  The real navigation and four-tab hierarchy stays mounted, redacted and disabled; five
  semantic-fill placeholder rows scale through AX5, and the loaded hierarchy returns once
  the first valid Today snapshot is assigned. Cold Spotlight/reminder routes permanently
  bypass the skeleton, a summary-to-Today remains eligible, store failure takes precedence,
  retry re-enters startup, and every non-Today task screen retains its ordinary spinner.
  Hosted lifecycle, route, recovery and visual-render tests pass on iOS 26.5/27 (F-042).
- [x] Row metadata preserves project/estimate/date/time/repeat/tags and notes/reminder indicators.
  Shared Rust tests own grouping/search projection rules, including estimate boundaries,
  family placement, missing/duplicate contexts, caps and notes. Shipping iOS flows exercise
  every grouping, every sort in both directions, preference relaunch, project/tag links,
  stale-query rejection and Archive family behavior on 26.5/27. Existing native state,
  compact-layout and row-semantics suites cover the presentation-only cases; exact bundles
  and limits are recorded in PROGRESS.
- [ ] Colorful labels toggle, #FF6600 exact default in both appearances and AsNeeded’s nine DHFlatUIColors custom accents without country groupings;
  saved selection, light/dark/increased contrast and non-color selection cues.
- [x] Opaque icon fills its complete square with no inset frame or baked-in corner mask.
  2026-09-16 resource readback and iOS 26.5 Home Screen inspection, dirty `task/ios-app`;
  `/tmp/momentum-ios26-home-icon.png`.
- [ ] Native font choice plus independent relative content/interface sizes and reset/persistence.
- [ ] Dynamic Type through accessibility sizes, VoiceOver labels/actions/order and keyboard focus.
  B-012's editor Done action now has a verified 44-point frame above the keyboard on
  iOS 26.5/27 with no invalid-frame warning. Voice Control/Switch Control have visible
  button alternatives for drag/reorder and stable labels; live system activation remains
  manual. Spoken VoiceOver is excluded from the current user scope.
- [ ] Completion/transition animation respects Reduce Motion; meaning never depends on color.
- [ ] English/German UI, dates, pluralization and long strings work with keyboard/landscape/iPad.
- [ ] Hardware keyboard commands cover all task, selection, navigation and search actions.
  F-023/B-045 record passing global navigation, capture/save, modifier, Settings,
  Help and sync checks on iOS 26.5/27. Search passes on iOS 26.5; the latest isolated physical iOS 27 run also fails cold
  Search after an earlier device pass. iOS 27 simulator/device acceptance remains open. Task selection,
  deselection, completion/reopening and undo pass on iOS 26.5; cold Select All
  fails in iOS 27 simulator and physical-device automation (B-047). Text editing and modifier/tab/editor
  isolation pass on both. Open, duplicate, copy title and archive shortcuts are now
  implemented with native regression coverage. Delete acceptance remains open under
  B-046. B-073 adds a direct SwiftUI `onKeyPress` fallback for cold Search, Select All
  and configured-modifier Delete, with unit-tested action routing and editor isolation;
  manual external-key dispatch is still required to close B-045–B-047. Today/slot/day planning and Undo pass on iOS 26.5/27 phone simulators using
  touch selection; per-runtime evidence is in PROGRESS. Move/repeat/reorder/new-project are implemented with native regression coverage;
  B-050 tracks the project sheet audit, and manual external-keyboard acceptance remains. See the
  dated runtime/device results in [PROGRESS.md](PROGRESS.md).
- [ ] Command/Control/Option modifier preference updates applicable commands and help consistently.
- [ ] System-reserved shortcuts retain native behavior; every essential action has a touch path.

## Sync, backup and integrations — F-024–029, F-031–032, F-040

2026-09-18 scope update: the user resumed the deferred parity items. The active
provider set is Off, Nextcloud and LibreSync. LibreSync production UI and transport
are implemented; separate-device discovery and native interaction remain acceptance
boundaries.

- [x] Fresh install defaults to Off; demo/preview never starts a transport or writes real secrets.
  2026-09-18 dirty `task/ios-app`: isolated package and native-boundary tests prove
  fresh Off state, no transport creation and no production Keychain access.
- [x] Off/Nextcloud/LibreSync selection retains saved connection and option values across switching and cold relaunch.
  2026-09-17 dirty `task/ios-app`: production-source setup/recovery UI passes on
  iOS 26.5 and 27, using an isolated WebDAV fixture. See PROGRESS.md for bundles.
- [x] Switching providers during an active exchange preserves tasks and queued edits for the active Off/Nextcloud scope.
  Package admission/drain tests and real HTTP cancellation preserve concurrent local edits;
  LibreSync start/stop/provider transitions are serialized against Nextcloud and restore.
- [x] Nextcloud server/user/folder, app password, encryption, compression and automatic-sync controls.
  2026-09-18 dirty `task/ios-app`: both OS setup flows save and reopen these values,
  correct failed credentials, retry successfully and verify encrypted/compressed output.
  Package validation now normalizes URL/user/folder input without changing secrets,
  requires HTTPS outside loopback, rejects embedded login/query/fragment data and does
  not replace a saved Keychain record when validation fails (B-069).
- [ ] Nextcloud setup passes native accessibility and hosted-server acceptance.
  Compact light/dark navigation and final English-light/German-dark AX5 matrices pass
  on iOS 26.5/27; B-067 gives Connection/Options primary ink and accepts the native
  normal-size contrast report only after an independent 7:1 rendered-pixel check.
  An opt-in hosted TLS test is implemented but skipped without disposable server credentials.
  Spoken VoiceOver is excluded from the current user scope; hosted acceptance stays open.
- [x] Credentials use Keychain; denial/storage failure is reported without false configuration success.
  Atomic device-only Keychain round trips, corrupt reads, read denial and write denial pass
  package/native boundary coverage without enabling sync or replacing the saved connection.
- [x] Isolated WebDAV tests prove upload/download, offline edits, conflict retry and error recovery.
  2026-09-17 dirty `task/ios-app`: real iOS 26.5/27 Swift/FFI/Rust HTTP exchanges
  with loopback WebDAV, disposable Keychain records and stores; plain/encrypted
  compression, two-peer convergence, missing-folder creation (B-037), failed credentials
  and password correction, cancellation and on-disk reopen. See PROGRESS.md for bundles.
  This check covers the isolated protocol scope, not hosted Nextcloud or physical devices.
- [x] Local edits during sync survive; import/provider switch/expiration reject stale commits.
  The iOS 26.5/27 loopback transport suite preserves a concurrent local edit during
  cancellation; package/view tests serialize restore and provider changes and prove a
  late expiration cannot cancel a newer exchange.
- [ ] LibreSync discovery, code expiry/closure, pairing, certificate rejection and unlink behavior.
  Production pairing/discovery/unlink UI, six-digit codes, Keychain identity and retry are
  implemented. Real socket tests cover pairing, certificate reuse and unlink on each runtime;
  separate-device Bonjour discovery, code-expiry timing and rejection UI remain manual.
- [x] An isolated linked peer exchanges both directions; durable state and iOS UI agree.
  Real Swift/UniFFI/Rust loopback peers pass on iOS 26.5/27, including two-way task state,
  callback publication, foreground stop/restart, durable linked state and certificate reuse.
- [x] Background suspension cancels/drains Nextcloud work; foreground resume retries without silent data loss.
  Nextcloud passes deterministic lifecycle plus real HTTP cancellation with queued local edits.
  LibreSync also stops/drains on background/provider/restore transitions and restarts serially.
- [x] Nextcloud provider/progress/last success/errors/details/retry are visible and reachable on empty and populated lists.
  2026-09-17 dirty `task/ios-app`: native real-HTTP status/recovery flows plus production
  RootView/TaskScreen empty-to-populated Today, Upcoming, keyboard-open Search, title
  pixels and AX5 scrolling pass on iOS 26.5/27 (B-043). Compact/German AX5 light/dark
  component checks and final app build pass. Status scrolls with the list. LibreSync
  exposes progress, last success, pending changes, linked devices, retry and manual sync;
  spoken VoiceOver is excluded from the current user scope.
- [x] JSON document export/import uses security-scoped access and explicit replacement confirmation.
  2026-09-17 dirty `task/ios-app`: native local Files round trips, cancelled/confirmed
  replacement, cold persistence, picker cancellation and scoped contrast audits pass
  on iOS 26.5/27. Cloud/third-party providers remain unverified.
- [x] Invalid/cancelled import preserves data; valid import clears/rebuilds dependent UI and indexes.
  2026-09-18 dirty `task/ios-app`: eight package tests, four native integration tests
  per runtime and two shipping Files picker tests per runtime pass. Invalid, cancelled
  and draft-conflicting restores preserve state; confirmed restore replaces tasks,
  refreshes the selected Settings view and persists after cold relaunch.
- [x] URL create/complete entry preserves desktop parameter semantics without changing another tab unexpectedly.
  2026-09-18 dirty `task/ios-app`: shared parser/real-engine tests plus native iOS
  26.5 and 27 Safari warm handoff/cold launch, tab preservation, complete, undo and
  persistence pass. B-068 prevents the installed sync QA host from claiming production
  URL schemes; corrected bundles are `/tmp/momentum-url26-collision-fix-r2.xcresult`
  and `/tmp/momentum-url27-collision-fix-r1.xcresult`.
- [x] Spotlight indexing, deletion and cold/warm task activation use the intended isolated store.
  2026-09-18 dirty `task/ios-app`: protected-index add/update/exact-delete passes 1/1,
  isolated cold/warm activation passes 2/2 and shipping SpringBoard Spotlight passes 1/1
  on each runtime. Both handoffs select Search, restore the exact current query and retain
  an open capture draft; disposable domains/tasks are removed. B-068 keeps the fixture's
  task UTType while removing its production URL ownership, and the corrected activation
  bundles contain no runtime warnings.
- [x] App Intents create/find/complete-or-reopen/plan Today/open match current macOS parameters/results.
  Five production Shortcuts tiles pass on iOS 27. The iOS 27 `AppIntentsTesting` suite
  passes 1/1 and explicitly executes Create, Find, Complete, Reopen, Plan Today and Open
  in `/tmp/momentum-intents27-r1.xcresult`. B-056 supplies all-active Plan suggestions.
  On a clean iOS 26.5 Customer OS install, `linkd` rejects the locally signed simulator
  client with `requiresValidBundle`; this prevents parameter UI after tile selection and
  is recorded as an Apple framework/signing boundary, not a reproduced app failure.
- [x] Intents and notification actions share the app engine on cold/warm invocation and concurrent edits.
  Eleven focused automation/URL/notification tests, ten automation view tests and both
  notification system suites pass. Cold Find uses a Debug-only isolated store marker;
  callbacks share one actor/store owner, stale identifiers do nothing, duplicate Done is
  consumed once and Snooze produces one exact one-hour plan. Locked-device behavior and
  physical testing remain explicitly user-deferred.
- [x] Badge modes: none, due-or-scheduled today, Today including overdue; zero hides the badge.
  Shared-core/coordinator tests cover every mode. Fresh-install iOS 26.5/27 simulator
  acceptance renders `1 new item` on Momentum's Home Screen icon and removes it when
  the count reaches zero.
- [x] Badge counts include subtasks according to core rules; notification permission is respected.
  Family-count tests cover parent/subtask semantics; native acceptance grants permission
  through the real system alert before the OS write. The isolated Debug QA mode never
  enables sync or uses the normal app store.

## Efficiency and completion evidence

- [x] Profile launch, large-list scrolling, search and edits with representative isolated fixtures.
  B-055's immutable snapshot projection is verified. Opt-in QA measurements with
  1,000 persisted tasks pass on iOS 26.5/27 for launch, scrolling, search and a
  representative short-title edit; dated metrics, screenshots and simulator/debug-Rust
  limits are in PROGRESS. Simulator output does not establish physical energy use.
- [x] Record device/OS/dataset/results; verify blocking Rust/network work stays off the main actor.
  2026-09-17 dirty `task/ios-app`: simulator runtimes, dataset and metrics are recorded
  in PROGRESS. Rust store/query work stays behind EngineWorker, and Nextcloud exchange
  runs in a utility detached task. Physical-device profiling is user-deferred.
- [x] No desktop polling/CLI socket or unbounded background activity; refresh/index work is coalesced.
  Source audit confirms iOS has no desktop socket/poll loop; search, sync, notification,
  badge and Spotlight refreshes are cancellable or coalesced, and foreground timers stop
  outside the active scene.
- [x] Exercise Low Power policy, interrupted/background sync and scheduled-work expiration for the approved simulator scope.
  F-041 implements and unit-tests the injected policy on iOS 26.5/27: automatic
  Nextcloud, Spotlight and discretionary notification refresh pause; manual sync,
  local work and an admitted exchange remain available; automatic work resumes when
  the mode turns off. Existing cancellation/expiration unit coverage passes. The user
  explicitly excluded a real Low Power Mode toggle on 2026-09-18. System-launched
  background execution remains manual because Simulator cannot originate that wake.
- [x] Bound startup shimmer work and stop it for accessibility, power and lifecycle states.
  F-042 caps its short-lived `TimelineView` schedule at 30 fps and pauses it under Reduce
  Motion, Low Power Mode and inactive scenes. The static skeleton remains visible; Reduce
  Motion also removes the 0.2-second content fade. Pure policy plus hosted SwiftUI tests pass
  on both supported simulator runtimes; physical energy profiling remains user-deferred.
- [x] Rust workspace/CLI, generated FFI, portable Apple support and macOS regression checks pass.
  2026-09-17 dirty `task/ios-app`: Rust workspace/CLI tests pass after allowing
  the Unix-socket tests outside the filesystem sandbox; generated Apple core preparation,
  136 MomentumKit tests and the macOS Debug app build passed in their recorded scope.
  The 2026-09-19 iOS gate passes 152 portable tests in 23 suites and 89 hosted native
  tests with two intentional skips on each runtime. Simulator-only iOS verification is
  recorded in PROGRESS; no physical device was used.
- [x] FEATURES, BUGS, PROGRESS and relevant specs reflect actual verified scope and remaining gaps.
- [ ] Every case is evidenced or has a user-approved, explicitly scoped disposition; no unqualified
  parity-complete claim while required native/device verification is missing.

## Source discrepancies and native equivalents

- F-009's word “restoration” must not invent a general archive-restore button: current
  macOS Archive rows are read-only; restoration is supported through operation undo.
- MVP's older independent sync switches and automatic-summary prose are superseded by
  F-027 and NOTIFICATIONS.md. Persistent sync errors/status also apply to empty lists.
- iOS uses native lifecycle/background scheduling instead of the Mac keep-running
  switch, menu-bar window or login item; global hotkeys are desktop-only. F-030 desktop
  CLI distribution is Not applicable. System-search and automation remain required.
- Show in Finder becomes native document import/export; do not expose internal storage
  or credentials through Files just to reproduce a desktop folder-opening action.
- Future F-033–036 (format independence, billing and tips) are not implemented desktop
  capabilities and do not authorize new dependencies, migration or commercial features.

Source: docs/MVP.md, GROUPING.md, NOTIFICATIONS.md, FEATURES.md; macOS Commands,
TaskListView, SettingsView, Sheets, TaskFormSheet, TaskIntents, CreateTaskIntent,
NotificationManager; MomentumKit Preferences and TaskAutomation. The implementation
plan must associate cases with named tests and native workflows after design approval.
