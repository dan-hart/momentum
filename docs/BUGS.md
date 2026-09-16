# Momentum bug ledger

Updated: 2026-09-16. Use the platform status vocabulary in [AGENTS.md](../AGENTS.md).
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
