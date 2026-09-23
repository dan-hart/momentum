<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# iOS navigation and interaction fixes

Date: 2026-09-18. Baseline: `7ec82e9` with the existing uncommitted iOS parity work.
Status: approved by the user on 2026-09-18. This remains local work; no commit, push,
publication, migration, physical-device installation, or physical-device test is
authorized by this approval.

## Outcome

Fix the ten reported iPhone and iPad behaviors without changing task data formats or
adding dependencies. Preserve the four tabs, native SwiftUI presentation, Rust-owned
task rules, offline behavior, accessibility preferences, and the current unit-only iOS
test architecture. The changes target iOS 26 and 27 and use simulator validation only.

## Tracking and platform impact

| Record | Scope | Affected platforms |
|---|---|---|
| B-075 / F-019 | First task snapshot moves the floating Add Task control | iOS only |
| B-076 / F-003/F-004/F-018 | Sidebar enters from the wrong edge and does not remain visible on capable landscape iPhones | iOS only |
| B-077 / F-005/F-006 | Quick Add does not open the created task detail | iOS only |
| B-078 / F-025/F-028 | Sync screen owns redundant asynchronous loading during back navigation | iOS only |
| B-079 / F-023 | Keyboard shortcut discovery is exposed on iPhone | iOS only |
| B-080 / F-019/F-020 | Accent selection checkmark does not follow the live choice | iOS only |
| F-019/F-020 | Add restrained motion and haptic feedback | iOS only enhancement |
| B-081 / F-010/F-020 | Completion and selection controls compete in selection mode | iOS only |
| B-082 / F-003/F-037 | Morning & Night can label tomorrow tasks as Today | Shared core; Linux, macOS, iOS, and CLI consumers |
| F-039 | Fast regression coverage for the changes | iOS unit architecture only |

These IDs are reserved by this approved design and must be added to the living ledgers
with reproduction, expected behavior, fix, evidence, and remaining validation.

## Considered approaches

1. **Native adaptive workspace (selected).** Use `NavigationSplitView` for the Today
   workspace, let SwiftUI decide whether two columns fit, and keep task detail routing
   and view policies in small testable state types. This supplies the correct leading
   sidebar transition and native wide-landscape behavior with the least custom code.
2. Keep `NavigationStack` and implement a custom leading overlay. This is initially
   smaller, but it duplicates system gestures, accessibility, column restoration, and
   size-class behavior.
3. Wrap `UISplitViewController`. This provides lower-level control but adds a UIKit
   bridge without a behavior that SwiftUI cannot already provide.

## Navigation and sidebar

The Today tab becomes an adaptive task workspace containing a list sidebar and a task
detail column. The detail column hosts the selected `TaskScreen`; `TaskEditor` remains
the standard modal editor. The sidebar contains Today, Morning, Evening, projects,
tags, and Archive using the existing sidebar snapshot and localized labels. Today is
the default selection. Selection survives tab switches and rotation. If Morning,
Evening, a project, or a tag disappears, the workspace falls back to Today.

The layout policy uses the reported horizontal size class and available container
width, never a device-name allowlist. A landscape phone is considered capable of a
persistent sidebar only when it reports regular horizontal size and its measured width
can fit the sidebar minimum plus the accessible detail minimum. That policy requests
`.all` column visibility. Narrow or compact layouts use `.detail` as the preferred
compact column; their toolbar button reveals the collapsed sidebar from the leading
edge. Selecting a list in a collapsed sidebar returns to its task list. A capable
landscape iPhone simulator must visibly show both columns, while a narrow phone must
visibly remain collapsed. Upcoming, Search, and Settings retain their independent tab
navigation state.

The floating Add Task control stays in a stable bottom inset outside snapshot
transitions. The first task snapshot is assigned without animation so the initial
ProgressView-to-list change cannot animate the button's geometry. Later list refreshes
may retain the existing short smooth transition when Reduce Motion is off.

## Task creation and selection

Every successful creation returns its task identity from the serialized `EngineWorker`
operation before another actor caller can mutate the engine. Quick Add stores that ID
in root-owned pending-editor state and dismisses. The root presents the standard
`TaskEditor` only from Quick Add's dismissal boundary, then resumes queued notification
routing after the editor closes. This prevents two sheets from racing. The expanded
More Details path continues to use the same editor. A multiline import that creates
several tasks opens the last created task, matching the engine's existing last-added
ordering. Failed or unchanged outcomes keep the entry sheet visible with its current
error handling. Tests cover single creation, multiline creation, and a queued
notification arriving during the handoff.

Task rows receive an explicit presentation mode. Ordinary rows retain the completion
circle and completion actions. Selection mode hides that control so the native row
selection indicator is the only selection affordance; accessibility completion actions
remain unavailable from the hidden control and the existing bulk Actions menu remains
available.

## Settings and platform capability policy

Sync state continues to be loaded and refreshed by `MobileAppModel` on foreground.
`SyncSettings` becomes a pure observer of that state and no longer launches a duplicate
load/status task when pushed. This removes page-owned async work from the navigation
lifetime and prevents a back transition from competing with a live status refresh.
Provider changes retain their existing drain and cancellation barriers.

Keyboard shortcut discovery is an iPadOS capability. The Settings row, help sheet,
scene command menu, and fallback shortcut handlers are exposed only when the current
interface idiom is iPad. The capability decision is injected into pure policy tests;
text editing behavior and ordinary software-keyboard controls remain unchanged.

Accent rows separate the color swatch from a trailing native selection checkmark. The
checkmark is driven by the resolved selected color ID, updates immediately with the
preview, adds the selected accessibility trait, and remains readable across appearance
and Increase Contrast modes. The approved DHFlatUIColors-only choices and exact default
orange behavior do not change.

## Shared-core grouping correction

Morning & Night grouping applies only to Today-style day views where Today, Morning,
and Evening describe the task's slot. Upcoming, project, tag, and search results return
no group for Morning & Night rather than mapping an untagged task to
`TaskGroup::Today`. `group_listing` treats that result as an instruction to preserve the
original plain section instead of force-unwrapping a group. Those views retain due-day
metadata on each row and eliminate the false Today header for tasks due tomorrow.
Tests cover Upcoming, project, tag, and search, plus preservation of Today, Morning,
Evening, completed, family, ordering, and undo behavior. Because this rule belongs to
shared listing semantics, the Rust correction is consumed by iOS, macOS, Linux, and
the CLI.

## Motion and feedback

Add restrained feedback at actions that confirm a state change: opening Add Task,
choosing a list or accent, entering or leaving selection, and successfully creating or
moving tasks. Use selection or success haptics behind the existing haptics preference.
Use 140–220 ms scale, opacity, or symbol transitions for the floating button, selected
checkmarks, and selection controls. Reduce Motion replaces nonessential movement with
an immediate or short opacity change. No animation blocks input, moves persistent
navigation unexpectedly, or conveys information without another visual/semantic cue.

## Testing and acceptance

Follow fail-first red/green cycles. Add Rust assertions proving tomorrow tasks cannot
receive a Today group outside a Today-style view. Add Swift Testing coverage for atomic
create-with-ID behavior, adaptive sidebar policy, iPad-only keyboard capability, and
selection presentation. Add focused in-process `UIHostingController` checks for the
accent checkmark, hidden completion control, stable first-load overlay, and Sync screen
lifecycle. Do not add or run XCUI tests and do not use ViewInspector.

Build and run the fast shared mobile and native view lanes, then build the app for the
iOS 26.5 and iOS 27 simulators. Inspect the affected flows in isolated/demo data in
light and dark appearance, portrait and landscape, Reduce Motion, and representative
Dynamic Type sizes. Simulator observation can establish the requested navigation and
visual behavior; no physical-device claim is made. Because B-082 changes shared-core
listing behavior, also run focused and workspace Rust tests, the `mo` CLI regression
suite, FFI freshness/regeneration compatibility checks, `swift test` for MomentumKit,
the macOS core build, and the available Linux consumer compile/test checks from
`docs/TESTING.md`. A macOS build does not establish native Linux runtime behavior, which
must remain explicit. Update `docs/FEATURES.md`, `docs/BUGS.md`, `docs/PROGRESS.md`, and
the iOS parity checklist with exact evidence and remaining system-interaction limits.
