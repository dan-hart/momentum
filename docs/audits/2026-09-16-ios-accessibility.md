# Momentum iOS accessibility audit

Status: source/automated audit recorded; remediation and manual acceptance remain
open. **The app is not yet verified as fully accessible**.
Scope: implemented native iOS/iPadOS UI in dirty `task/ios-app`, based on `7ec82e9`.
This audits the current app, not planned sync, import/export, automation or other
unimplemented parity features. macOS/Linux have separate accessibility evidence.
No personal task store, account, or device settings are used for test fixtures.

**Testing update, 2026-09-18:** the user retired every iOS XCUITest target and
source. The XCTest audit inventory below remains historical evidence for the recorded
dirty revision. Current automated regressions use unit tests that host and render the
production SwiftUI source with explicit appearance, contrast, locale and Dynamic Type
traits. Spoken output, focus traversal, gestures, system prompts and alternate-input
operation remain manual acceptance boundaries.

## Assessment and remediation order

**Not ready for a full accessibility-support claim.** Two high-priority issues
are default foreground contrast (B-013) and largest-text truncation (B-014).
Four medium-priority remediation/investigation items cover task-target geometry
(B-015), expiring feedback (B-016), subtask context (B-017) and error-text contrast
(B-018). Four additional candidates below require interaction verification before
being promoted to confirmed defects. There is no evidence of a critical issue that
blocks every user; coverage gaps must not be read as passes.

1. Resolve foreground roles while preserving exact orange brand/fills; fix large
   text truncation and semantic error contrast.
2. Enlarge/group task actions, expose parent context and make feedback discoverable
   without a five-second reading deadline.
3. Reproduce/resolve the disabled-state, input/error association and keyboard/caret
   candidates with assistive technology; then perform the acceptance matrix below.
4. Rerun automated inventories and compare each remediated finding to its original
   screenshot/tree. Only then advance scoped F-019/F-020 acceptance statuses.

## Method and evidence boundaries

- Inspect all native view sources, presentation state, feedback, typography and
  accent policy; trace controls to their SwiftUI accessibility representation.
- Historical audit runs collected Apple's XCTest `.all` findings with screenshot and
  hierarchy attachments. Those bundles are retained as dated evidence; the harness is
  no longer part of the project.
- Audit ordinary and largest accessibility text layouts; compare appearance/runtime
  samples and distinguish product defects from covered/offscreen UIKit cells.
- A successful diagnostic test means navigation completed. It is **not** an
  accessibility pass. The attached JSON inventory is authoritative for raw findings.
- Source labels and XCUITest trees do not prove spoken VoiceOver order or completion
  with Voice Control, Switch Control, Full Keyboard Access, braille or assistive input.
  Those require explicit interaction acceptance before claiming support.
- The local `xcdocs` search failed with an embedding-service error. Consulted the
  current official Apple guidance and installed Xcode 27 XCTest headers instead.

## Scope coverage and acceptance limits

| Area | Evidence collected | Acceptance still required |
|---|---|---|
| Today, Upcoming, Search | Empty-state native audits; populated/completed/selected Today; source review of shared row/search presentation | Populated search/Upcoming, search announcements and focus restoration with assistive input |
| Task capture/editing | Quick Add success/rejection/long draft; editor scheduling, invalid estimate, notes, subtask creation; native context menu | Assisted completion of every menu action, duplication/delete/undo, long-field correction and focus |
| Recurrence/reminders | Weekly form, lower controls, scheduled-time state, disabled reminder semantics | All recurrence modes, real permission prompts, delivery/actions and spoken disabled-state explanation |
| Lists/archive | Native project/tag editors, custom-color control, lists and archive | Assisted project/tag deletion, move/reorder/drop alternatives and archived browsing |
| Settings | All implemented pages, accent choice, font/feedback, notification summary/badge/recovery | Every custom font at accessibility sizes, slider adjustment, all color choices rendered with assistive settings |
| Vision | Actual AX5 English/German layouts, screenshots, contrast audits/calculation; source color-independent labels | Intermediate sizes, Bold Text, Zoom, Reduce Transparency, real Differentiate Without Color behavior |
| Motion/hearing | Source Reduce Motion guards; visual equivalents to haptics; no essential audio/video | Native Reduce Motion gesture/transition observations; no captions/audio-description claim needed for absent media |
| Speech/motor/cognition | Native labels/roles/states, custom reorder actions, touch-target geometry, feedback timing | Spoken VoiceOver, Voice Control, Switch Control, Full Keyboard Access, braille and actual coarse pointing |
| Platform variants | iOS 27 baseline; representative iOS 26.5, iPad and landscape attempts below | Full device/window/appearance cross-product and physical-device assistive acceptance |

These are explicitly bounded audit samples of all implemented view families, not
an assertion that every configuration or assisted workflow has passed. There is no
physical-device reinstall, personal-store mutation or live sync/notification test.

## Native run ledger

All runs use Xcode 27, Debug, isolated fixtures, and the dirty revision above.
Result bundles and complete attachments are local `/tmp` artifacts; selected
inventories/screenshots are retained in `ios-accessibility-2026-09-16/` beside this
report. The app code was not modified in this audit; harness fixes address traversal
and evidence collection only.

| Run | Scope/result | Local bundle |
|---|---|---|
| iPhone, iOS 27, light, ordinary text | 31 states / 121 raw findings; traversal completed | `/tmp/momentum-a11y-inventory27-v2.xcresult` |
| iPhone, iOS 27, light, AX5 | 12 states / 18 raw findings; traversal completed | Same bundle |
| iPhone, iOS 27, German AX5 | 8 states / 18 raw findings; traversal completed; landscape screenshot has an unresolved capture anomaly | `/tmp/momentum-a11y-supplement27.xcresult` |
| iPhone, iOS 27, subtask/invalid estimate | 3 states / 24 raw findings; traversal completed | Same supplemental bundle |
| iPhone, iOS 26.5, dark, Increase Contrast configured | AX5: 12 states / 18 raw findings; subtask/validation: 3 states / 27 findings; both traversals completed, including actual Notifications bottom | `/tmp/momentum-a11y-dark26.xcresult` |
| iPad mini, iPadOS 27, AX5, first attempt | Harness used iPhone-only tab query; failed before inventory | `/tmp/momentum-a11y-ipad27.xcresult` |
| iPad mini, iPadOS 27, AX5, second attempt | Today screenshot/tree captured; runner terminated with signal term during audit; not an app crash finding | `/tmp/momentum-a11y-ipad27-v2.xcresult` |
| iPad mini, iPadOS 27, AX5, final retry | 12 states / 50 raw findings; traversal completed after harness query hardening | `/tmp/momentum-a11y-ipad27-v3.xcresult` |
| iPhone, iOS 27, German AX5, landscape retry | 8 states / 16 raw findings; traversal completed; screenshot anomaly persists after settle delay, so landscape visual acceptance remains open | `/tmp/momentum-a11y-landscape27.xcresult` |

Initial compilation failed on an invalid XCTest query and was corrected before
recording the successful baseline. The original result bundle is retained as
`/tmp/momentum-a11y-inventory27.xcresult`. The iOS 26.5 simulator was configured with Increase Contrast before testing; no
per-view trait dump was captured. Screenshots verify dark appearance. The setting
was restored to disabled afterward. These failed attempts do not count as
passed accessibility checks. The initial iPhone AX5 "notifications-bottom" capture
was actually mid-page; the harness now reveals Refresh Schedule before capture.

## Guidance

- [Apple Accessibility HIG](https://developer.apple.com/design/human-interface-guidelines/accessibility)
- [VoiceOver evaluation](https://developer.apple.com/help/app-store-connect/manage-app-accessibility/voiceover-evaluation-criteria)
- [Voice Control evaluation](https://developer.apple.com/help/app-store-connect/manage-app-accessibility/voice-control-evaluation-criteria)
- [Larger Text evaluation](https://developer.apple.com/help/app-store-connect/manage-app-accessibility/larger-text-evaluation-criteria)
- [Sufficient Contrast evaluation](https://developer.apple.com/help/app-store-connect/manage-app-accessibility/sufficient-contrast-evaluation-criteria)
- [Reduced Motion evaluation](https://developer.apple.com/help/app-store-connect/manage-app-accessibility/reduced-motion-evaluation-criteria)
- [Differentiate Without Color evaluation](https://developer.apple.com/help/app-store-connect/manage-app-accessibility/differentiate-without-color-alone-evaluation-criteria)
- [Automated audits](https://developer.apple.com/videos/play/wwdc2023/10035/)

## Findings

### A-01 — exact orange cannot satisfy text contrast on light surfaces

**High; confirmed by color calculation and current rendered source (B-013).**
`AccentChoice.renderedColor` deliberately returns #FF6600 for Momentum Default,
including Increase Contrast, following the user's most recent color decision.
Contrast is 2.936:1 against white and 2.631:1 against #F2F2F7; those values fall
below Apple's text guidance. Against #3A3A3C elevated dark, it is 3.865:1, which
also falls below the normal small-text target. Black on the orange fill is 7.153:1.

Affected roles include tint-colored text and native selected controls. Neutral
body text, adaptive custom accents, and black symbols on filled orange actions
are separate and must not be described as failing by association.

**Recommendation:** keep exact orange as the brand/fill color with black content,
and use a readable semantic foreground for text and essential line icons. At
minimum, a system Increase Contrast response is needed. Do not silently restore
the darker orange the user explicitly rejected. A consistent foreground policy
must cover native tab selections, links, menus, buttons and form controls.

### A-02 — undersized task-row accessibility targets

**Medium; native measurement plus source evidence (B-015).** The native audit flags the
metadata element `Inbox · ~15m · #Access` at 130.3 × 14.3 points in populated,
selected and completed states. Native hierarchy also exposes the parent and child
Open buttons as 128 × 20.3 and 117.7 × 20.3 points. Completion has a 44-point
minimum; adjacent title buttons do not. The row attaches custom actions and drag
behavior to separately exposed content. The audit does not prove that every point
outside these frames is untappable; effective touch slop needs interaction testing.

**Recommendation:** group title/metadata into a meaningful, adequately sized Open
action, retaining a separate 44-point completion control and accessible reorder
alternatives. Verify actual hit targets, VoiceOver actions and Voice Control names.
Source: `ios/Momentum/Tasks/TaskScreen.swift`, `TaskRowContent` and `taskRow`.

**2026-09-17 follow-up:** B-015 combines live-row details into the 44-point Open
button while retaining separate completion and an accessible metadata value.
Native phone checks on iOS 26.5/27 pass for geometry, edge taps, completion/reopen
and the unfiltered hit-region audit. See BUGS/PROGRESS for bundles and remaining
assistive-technology checks; this does not retroactively clear the whole audit.

### A-03 — transient feedback expires without accessible announcement

**Medium; source-confirmed timing risk (B-016).** `TaskScreen` dismisses every feedback
message after five seconds, regardless of assistive-technology use. An
`updatesFrequently` trait does not itself enqueue an announcement. A person reading
other content can miss success/error feedback before reaching the banner.

**Recommendation:** keep errors/recovery messages available until dismissed;
announce relevant results without moving focus unnecessarily. Provide persistent
Undo through the toolbar (already implemented) and verify focus after row removal.

**2026-09-17 follow-up:** B-016 removes the dismissal timer and posts each result
once from the app model using Apple's [announcement API](https://developer.apple.com/documentation/accessibility/accessibilitynotification/announcement).
Low priority avoids interrupting ongoing assistive speech. The model-boundary test
verifies requests and unchanged presentation, not delivered speech or VoiceOver
focus. Native persistence/dismissal/Undo and AX5 error checks are recorded in
PROGRESS. Actual spoken acceptance remains separate; further dip17pm tests are
user-deferred.
Spoken delivery still needs VoiceOver testing.

### A-04 — subtask relationship is represented by indentation only

**Medium; confirmed in source and native accessibility hierarchy (B-017).**
`TaskRowContent` uses `isSubtask` for leading padding, but the Open/Complete labels
only name the task. No child/parent relationship is exposed by that path. The isolated fixture exposes `Open Child audit task` and `Complete Child audit task`
without its parent context; reading an isolated child control does not carry the
relationship apparent visually. See [native hierarchy](ios-accessibility-2026-09-16/subtask-hierarchy.txt).

**Recommendation:** expose a concise localized subtask/parent description and
meaningful grouping without hiding separate completion and edit actions. Verify
real spoken output, including duplicate child titles under different parents.

**2026-09-17 follow-up:** B-017 now resolves parent titles in mobile snapshots and
adds localized parent context to independent Open/Complete/Reopen controls. Two
identically named children remain distinguishable in Today and filtered Search;
parent renames are covered by worker tests. English/German native iOS 26.5/27 checks
pass alongside the 44-point target regression. A subsequent core batch lookup
also resolves hidden archived parents from both archive tiers. English/German
archived Search checks pass on iOS 26.5/27, exposing parent labels as StaticText
and preserving read-only rows; result bundles are tracked in PROGRESS.md.
Generic subtask wording remains only for missing parent records. Actual VoiceOver
speech/focus remains open. See B-017 and PROGRESS.md for current evidence; the
original finding above describes the audited revision.

### A-05 — errors do not have explicit announcement/focus handling

**Candidate; source-confirmed missing handling, spoken impact unverified.**
Quick Add, task editing and recurrence show error text while retaining ordinary
input focus. Task/recurrence errors are placed at the form's top, potentially above
the current scroll position. There is no accessibility announcement or error focus.

**Recommendation:** announce failed saves and validation context, keep drafts,
associate invalid fields with their errors, and reveal offscreen error content.
Keep input focus where continued correction is useful. Avoid color-only errors.

**2026-09-18 follow-up:** B-070 adds one semantic error component that adapts at
accessibility text sizes. Async Quick Add, task, recurrence and project-move failures
move accessibility focus to the new error; estimate correction stays associated with
the active field as a hint. Phone/iPad AX5 rendering and both iOS unit matrices pass.
Actual VoiceOver speech, focus order and alternate-input operation remain manual and
the candidate is not closed from source/unit evidence alone.

### A-06 — disabled Reminder trait inconsistency

**Candidate, medium if spoken state is wrong.** XCTest reports missing
`UIAccessibilityTraitNotEnabled` for `Reminder, None` when Time is off. However,
the same captured hierarchy explicitly marks the picker `Disabled`. These are
conflicting signals, not proof that a user can activate it or VoiceOver announces
it incorrectly. Source correctly calls `.disabled(...)`.

**Recommendation:** check actual VoiceOver announcement and activation on both OS
versions. Only add an app workaround if a real semantic defect is reproduced;
retain native disabled behavior and consider a concise reason/help message.

### A-07 — Settings values and iPad empty-state title truncate at largest text

**High; screenshot-confirmed in English and German (B-014).** In Task Lists, menu picker
values render as `Morni…Night` / `Manu…rder`, with analogous German truncation.
Labels wrap but the selected value remains a single truncated line. This makes
current grouping/sorting harder to identify for people relying on large text.
The iPad mini Today empty-state title also truncates to `Nothing planned for to…`
at AX5 despite available vertical space. Its Settings values fit at full width,
so the phone picker truncation is not reproduced there. The iPad editor also
shows an oversized Save checkmark extending beyond its orange fill at AX5; the
button retains its accessible Save name. Native menu accessibility labels retain
the full string; this is a visual
large-text defect, not evidence of absent VoiceOver values.

**Recommendation:** use an accessibility-size layout with a full-width multiline
value or a native navigation selection page; retain scalable text and full labels.
Constrain toolbar symbol geometry without shrinking task text or hit targets.
Sources: `ios/Momentum/Settings/SettingsScreen.swift`, `TaskListSettings`, and
`ios/Momentum/Tasks/TaskScreen.swift`, empty state.
Evidence: [English](ios-accessibility-2026-09-16/english-largest-text-settings.png),
[German](ios-accessibility-2026-09-16/german-largest-text-settings.png),
[iPad empty state](ios-accessibility-2026-09-16/ipad-largest-text-empty-state.png),
[dark iOS 26.5](ios-accessibility-2026-09-16/dark26-largest-text-settings.png),
[iPad editor](ios-accessibility-2026-09-16/ipad-largest-text-editor.png).

### A-08 — long Quick Add editing visibility needs an interaction check

**Candidate.** At largest text, the long draft extends behind the keyboard in the
captured viewport; Create remains reachable and creation succeeded. The screenshot
does not prove scrolling/caret recovery is impossible. Test insertion, correction,
selection and dismissal using touch and VoiceOver before classifying a blocker.
The same draft fits visibly above the iPad keyboard; that does not establish
iPhone editing acceptance.
Source: `ios/Momentum/Tasks/QuickAddSheet.swift`.

### A-09 — field identity and error association need spoken verification

**Candidate.** The populated estimate field appears in the native hierarchy with
value `invalid`, placeholder `1h 30m`, and no explicit label. SwiftUI receives an
`Estimate` title in source, so the tree alone is insufficient to conclude spoken
identity is lost. The visible validation sentence is separate from the field;
Save is disabled. Verify that assistive users hear both field purpose and the
reason saving is unavailable. Audit title/new-tag fields by the same criterion.
Source: `ios/Momentum/Tasks/TaskEditor.swift`, `scheduleSection`.

### A-10 — validation error text misses the small-text contrast target

**Medium; native audit and visible error confirmed (B-018).** After entering `invalid` as
an estimate, the normal-size red sentence is visible on a white form row. Apple's
audit reports “Contrast nearly passed,” explaining that the contrast requires a
larger font. This is not a pass for the current regular body text. Task and repeat
errors use `.red` directly rather than a surface-aware semantic error foreground.
This concern is independent of the user-selected orange accent.

**Recommendation:** use a contrast-compliant semantic error foreground and an
error symbol/description; preserve the message and its field association. Check
both appearances and Increase Contrast after remediation. Source:
`ios/Momentum/Tasks/TaskEditor.swift` and `RepeatEditor.swift`.

**2026-09-17 follow-up:** task/estimate/recurrence errors now use surface-aware
semantic red and an SF Symbol; Estimate also has an explicit localized field name.
The native trait test covers normal/high contrast, light/dark, base/elevated levels
and six system surfaces. Native estimate correction/save/reopen and recurrence
rejection/recovery pass on iOS 26.5/27, including German AX5. B-039's sheet canvas now
covers scrolled content. The German test documents one covered `Fällig` false
positive with pixel/geometry guards; recurrence re-audits partly-offscreen `Starts`
after making it fully visible. Original findings remain attached. See B-018 and
PROGRESS.md for final runtime evidence and limits; A-05 spoken error handling and
B-014's separate Settings findings remain open; its toolbar follow-up is recorded below.

## Raw finding triage

Keep the original inventories, including warnings on covered/offscreen elements.
Counts describe findings across screen visits, not distinct defects. The audit
can retain background rows behind sheets and report content beneath navigation
or tab bars. Screenshots plus element frames must distinguish these from actual
clipping. Several Dynamic Type warnings occur on controls that visibly scale in
the explicit AX5 run; they remain investigation items, not proof that Dynamic Type
is globally unsupported. Unknown-element warnings in native date/repeat controls
require manual inspector/assistive testing. No warning was silently excluded.

Both landscape captures contain black/cropped areas, including the retry after a
settle delay. The native hierarchy reports an 874 × 402-point landscape window,
while the PNG carries orientation-8 metadata and a 2622 × 1206 raster. The cause
is unresolved; do not infer an app layout defect or visual acceptance from this
capture. The [original retry image](ios-accessibility-2026-09-16/german-largest-text-landscape.png)
is retained unchanged. A fresh device-level capture/physical-device check is needed;
the successful navigation back to portrait does not close this visual gap.

## Positive source findings to preserve

- Native tabs, navigation, forms, toggles, menus, date controls and dialogs provide
  standard accessibility semantics instead of custom gesture-only widgets.
- Completion and Add/Save/Cancel controls have task/action labels; decorative icons
  are generally hidden. Notes and reminder indicators carry descriptions.
- Named Move Up/Move Down accessibility actions provide a reorder alternative.
  The native action exposure and actual assisted use still require verification.
- Task text uses scaled metrics/relative custom fonts with no maximum Dynamic Type
  clamp. Settings descriptions permit multiline layout.
- Custom palette choice includes a checkmark and selected trait; labels contain
  color names. Task/project/tag text remains available without relying on hue.
- Increased Contrast and Differentiate Without Color remove task-label color coding.
- List, suggestion and editor transitions check Reduce Motion; completion symbol
  animation is explicitly disabled when Reduce Motion is on. No continuous animation.
- Haptic success is accompanied by a visible task-state change. There is no essential
  audio/video content requiring captions or audio description in implemented UI.
- Quick Add retains rejected drafts; destructive project/tag actions use native
  confirmation with scope text, while task deletion has Undo.
- Native keyboard Save/Create shortcuts exist. Full keyboard navigation/action
  coverage is a separate, still-open parity requirement.

## Required acceptance before a full accessibility claim

- Spoken VoiceOver: all common workflows, reading order, custom actions, focus after
  sheets/errors/deletion, field state/value, notifications and permission changes.
- Voice Control and Switch Control: create/edit/complete/delete, scheduling,
  reorder/move, navigation and settings using only the respective input method.
- Full Keyboard Access: tab/arrow order, menus, date controls, editor traversal,
  escape/cancel, selection, search and focus restoration on iPhone/iPad.
- Largest text and other intermediate accessibility sizes in compact/landscape/iPad
  layouts, plus actual Bold Text, Reduce Motion and Reduce Transparency settings.
- Real-device notification delivery and actions with assistive technology, using a
  dedicated test store/account. The user's dip17pm is not seeded or changed here.
- Recheck every finding after remediation; green unit tests alone do not close them.

### A-07 toolbar follow-up — 2026-09-17

The app's concrete scaled body font was reaching custom toolbar Image labels.
`SheetCommitButton` now clears that symbol font override, restoring native toolbar
sizing without capping task text. A new pixel regression fails for the original
AX5 checkmark and passes the corrected phone rendering; Save/Create actions are
also exercised. See the dated B-014 PROGRESS entry for final iPhone/iPad runtime
evidence. These checks do not establish spoken feedback or whole-screen compliance.
