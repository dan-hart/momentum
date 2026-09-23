# iOS sync, feedback, and list polish design

**Date:** 2026-09-19  
**Scope:** Momentum iOS 26 and 27, with shared-core changes where task or transport truth belongs  
**Tracking:** F-043 and B-084 through B-088  
**Decision authority:** the user asked for the five changes in this scope and previously approved the assistant's best design/spec recommendations for this iOS work.

## Problem

The current iOS app exposes sync configuration but no explicit connection test, gives sync status too much prominence in task lists, reserves a full-width bottom strip for one floating action, leaves operation feedback visible until dismissed, and presents saved project/tag colors as thin outline accents without task counts. These details make the app feel heavier and less informative than intended.

## Considered approaches

### 1. Presentation-only patch

Move the existing status row, remove the bottom background, add a delay to feedback, and calculate list counts in Swift. This is the smallest diff, but it duplicates task rules in the client and cannot honestly test a Nextcloud connection without performing a real sync.

### 2. Native presentation with shared truth — selected

Keep layout, animation, accessibility, and provider-specific messaging in SwiftUI. Add a read-only Nextcloud transport probe and project/tag counts to the Rust core, while LibreSync uses its existing real peer-exchange completion event. This keeps task and network truth in shared code, preserves native iOS behavior, and gives fast unit seams for every state.

### 3. Full sync-health subsystem

Create a durable multi-provider health history, latency measurements, diagnostics, and automatic monitoring. This exceeds the request, adds background work, and would create privacy and battery costs without a current product need.

## Product behavior

### Test Connection

Sync Settings shows a **Test Connection** action for the selected configured provider.

- **Nextcloud:** validate the current saved connection and perform a read-only authenticated WebDAV probe off the main actor. The probe verifies the account endpoint and configured collection when present; a missing Momentum folder is acceptable because the first sync can create it. It never uploads, downloads into the task store, changes sync timestamps, or consumes pending operations. Unsaved or invalid fields disable the action and explain that the connection must be saved first.
- **LibreSync:** require a running provider and at least one linked device, start or join an on-demand outbound exchange, and await that exact exchange's completion. The shared core returns the outbound cycle ID and includes it in started/completed events; events without that ID, inbound work, state-save failures, and later cycles cannot satisfy the test. Success means at least one linked peer completed that attributed exchange. An unavailable peer, missing linked device, timeout, or transport error produces actionable failure copy without claiming connectivity.
- **Off:** no test action is shown.

The action has one observable state: idle, testing, success, or failure. While testing it shows a progress indicator and cannot be triggered twice. A new test replaces the previous result. Provider changes clear stale results. Results use text and SF Symbols, receive accessibility focus, and do not rely on color alone.

## Task-list sync footer

Remove the large sync row from the top of task lists. When sync is enabled, add one compact, tappable footer at the bottom of the list:

- normal: `Last synced just now`, then system-localized relative minutes/hours/days;
- never synced: `Not synced yet`;
- active: `Syncing…`;
- error: `Sync needs attention`;
- pending work may be included in the accessibility value, without adding another visible row.

The footer opens Sync Settings and uses caption/secondary styling with a minimum 44-point hit region. System relative date formatting owns efficient refresh scheduling; Momentum adds no high-frequency timer. Empty task lists still place this status after the empty state so it reads as metadata rather than primary content.

## Floating Add Task control

Replace the full-width bottom safe-area inset with a bottom-trailing overlay containing only the Add Task capsule. The list keeps an invisible scroll-content bottom margin so its last row can scroll above the control, but the visible canvas remains continuous and accepts taps everywhere outside the capsule. The capsule retains its stable identity, safe-area/tab-bar spacing, white default-orange label in normal contrast, calculated Increase Contrast foreground, accessibility layout, press animation, and existing haptic behavior.

Selection controls continue to use their existing full-width safe-area layout because they contain multiple actions and must remain reachable at large text sizes.

## Transient Liquid Glass feedback

Move operation feedback from each retained task screen to one root-owned overlay, preventing duplicate or stale banners between tabs. Present it above the tab bar and floating action as a compact rounded Liquid Glass toast with a symbol and readable semantic text.

- ordinary informational messages dismiss after 3.5 seconds;
- mutation messages that carry the core's `Outcome.undo` batch ID show a separate, localized **Undo** button and dismiss after 5 seconds;
- Undo invokes that exact batch through `Engine.undo_batch`, even if a newer mutation has occurred, then dismisses the originating toast;
- `Message::SaveFailed` becomes a persistent root-owned error glass with accessibility focus, the error text, a **Dismiss** button, and guidance to retry the original action. It does not enter an automatic timeout because the failed mutation is not retained safely enough for a generic retry;
- a replacement message restarts the deadline;
- the timeout is implemented by one cancellable task, with no polling;
- Reduce Motion uses an immediate or opacity-only transition;
- Reduce Transparency and Increase Contrast retain a solid readable fallback behind the glass;
- the existing low-priority accessibility announcement remains. A message-only toast is one accessibility element; an actionable toast exposes the message followed by a distinct Undo button.

The root-owned presentation value carries a unique ID, message, symbol, severity, and optional undo batch ID so repeated identical messages restart their deadlines. The timer is presentation state. Task-store persistence failures use the persistent root error presentation; validation, credential, transport, and recovery errors remain in their existing focused inline views until resolved. None are hidden by the toast timeout. Mutation results and undo behavior remain in `MobileAppModel` and the shared engine.

## Colorful projects, tags, and counts

Project rows always use `folder.fill`; tag rows always use `tag.fill`. The saved DHFlatUIColors value fills the complete symbol. The text and row outline remain semantic system colors. When saved colors are disabled or unavailable, the symbol uses tint. Differentiate Without Color adds no new color-only meaning because the symbol shape and label already identify the context.

Each project and tag row shows a trailing count of unfinished task families assigned to that context. Every direct context member maps to its root parent; unique roots are counted once when that root is live and unfinished. This means a tag found only on a subtask still counts its family, parent-plus-child membership does not double count, and a completed root contributes zero even if a child has inconsistent completion state. This matches the app badge's family-count rule. Zero remains visible so an empty context is explicit.

The Rust sidebar projection computes the counts once from the canonical store and exposes them in `SidebarEntry`. `ListPicker` switches its project/tag iteration to `snapshot.sidebar.projects` and `.tags`, deriving the context ID from each entry's typed `View`; `snapshot.projects` and `.tags` remain the editing/autocomplete projections. SwiftUI only formats the number. Existing Linux and macOS consumers continue to compile and may adopt the field without changing their present layout in this scope.

At accessibility text sizes, the count remains trailing when it fits and moves below the label when needed. It uses monospaced digits, secondary text, and an accessibility label such as `3 tasks` with localized pluralization.

## Components and data flow

- `sp-sync`: a read-only guarded Nextcloud probe using the existing URL/auth/deadline/error mapping.
- `momentum-core`: exposes the probe through UniFFI, gives outbound LibreSync cycles stable IDs with explicit join semantics, and adds canonical unfinished-family counts to `SidebarEntry`.
- `MomentumMobile`: owns cancellable connection-test state, bridges the Nextcloud probe off actor, and awaits only the matching LibreSync completion event with a bounded timeout.
- `SyncSettings`: provider-specific Test Connection action/result presentation.
- `TaskScreen`: compact sync footer and floating-button overlay; no feedback ownership.
- `RootView` plus a focused toast view: one timed feedback presentation across tabs.
- `ListPicker`: filled colored project/tag symbols and shared-core counts.

Generated UniFFI sources are regenerated by project tooling and never hand-edited.

## Error handling and lifecycle

Connection-test errors reuse existing provider error classification without overwriting the persistent automatic-sync failure unless a real sync changes it. Cancellation from navigation, provider changes, backgrounding, or replacement tests returns to idle and does not show a false failure. A test joining an already active LibreSync outbound cycle waits for that cycle ID; an unrelated or unattributed completion is ignored. A completed mutation feedback toast may expire while another tab is visible; replacement messages are deterministic.

No test uses personal credentials, task stores, or peers. The live Nextcloud test remains opt-in with disposable credentials; ordinary coverage uses the existing mock WebDAV transport. LibreSync tests use isolated local runtimes.

## Accessibility, localization, and energy

All new controls have localized labels, hints, 44-point targets, logical focus, Dynamic Type layouts, and non-color status cues. Toast motion respects Reduce Motion, its surface respects transparency/contrast settings, and text retains semantic foreground colors. Relative sync text uses system localization. Filled context symbols retain the established secondary-tint fallback when Colorful Labels is off, Increase Contrast is enabled, or Differentiate Without Color is enabled. Connection work runs only after an explicit tap, is cancellable and bounded, and uses no polling. Feedback owns one short sleep task only while visible.

## Verification

Use test-first coverage and observe each regression fail before implementation:

1. Rust tests for read-only Nextcloud probe success, authentication failure, missing optional collection, cancellation/deadline mapping, and no store mutation.
2. Rust sidebar tests for project/tag counts, unfinished family semantics, archive/completion changes, and zero.
3. Shared-core and MomentumMobile tests for LibreSync cycle attribution, joining an active outbound cycle, ignoring inbound/unattributed events, completion/failure/timeout, cancellation, double-tap exclusion, and provider-change reset.
4. MomentumMobile tests for Nextcloud idle/testing/success/failure states and cancellation.
5. Hosted SwiftUI tests for the compact footer, provider result states, root-owned timed toast and exact-batch Undo, persistent/focused `SaveFailed` error presentation, reduced-motion/contrast variants, filled symbols/counts, and the floating Add Task hit/layout region at phone, iPad, and accessibility text sizes.
6. Complete `unit` and `transport` lanes on iOS 26.5 and iOS 27. Transport coverage includes authenticated WebDAV success, collection 404 accepted after an authenticated account probe, authentication failure, deadline, cancellation, and unchanged store/status.
7. Shared Rust workspace checks, MomentumKit tests, generated-binding freshness, app builds, localization checks, Markdown links, and diff hygiene.

Update `docs/FEATURES.md` (F-043), `docs/BUGS.md` (B-084 through B-088), `docs/PROGRESS.md`, and the iOS parity checklist with actual evidence and remaining physical/system limits.

Visual simulator inspection will cover Today with empty/populated lists, all four tabs, light/dark, Increase Contrast, Reduce Transparency, and largest accessibility text. Final physical-device installation on dip17pm is allowed by the standing request, but no personal sync connection or task data will be inspected.
