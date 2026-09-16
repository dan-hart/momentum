<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# macOS MVP completion checklist

The acceptance baseline is **MOMENTUM-MACOS-HANDOFF.md (2026-09-15)**, especially
sections 10, 15 and 16. `docs/MVP.md` remains the product specification. A successful
preview build is not MVP signoff. This checklist records the continuation of the
uncommitted cross-platform implementation; it does not replace the handoff.

## Current acceptance status — 2026-09-16

The MVP feature implementation and handoff's local verification sweep are complete.
The user confirmed native single/multiple-task project moves, manual reorder with
Undo, and external text/URL drops. The normal app is running on the existing user
store, with Today selected and Nextcloud reporting a successful sync.

**Acceptance exceptions:** the user explicitly deferred physical LibreSync and live
Spotlight checks. Spoken VoiceOver navigation remains unverified because VoiceOver
did not stay enabled. Native accessibility-tree, keyboard, and contrast checks passed.
This is not an unconditional signoff of every platform/device acceptance case.
Earlier dated entries below are historical; this status and the final continuation
supersede their pending-check descriptions.

## Handoff inventory

| Handoff item | Implementation and automated evidence | Live acceptance |
|---|---|---|
| D1 Select All | Fixed Command-A; focused list action, native NSTextView selection; ShortcutsTests; explicit list focus on row click | Live: Command-A selected all nine demo rows after clicking a row; text-field replacement and Escape deselection verified |
| D2 Shortcut help matches bindings | AppShortcut definitions drive menu bindings and help; all three modifiers tested for collisions and duplicate symbols | Help menu and rendered guide inspected; each guide row now exposes its action and keys separately to accessibility |
| D3 Archive has no drag | Archived rows expose no TaskTransfer; mixed selection filters archived rows | Archive payload exclusion covered by tests; separate native archive gesture not verified |
| D4 Subtitle order | TaskSubtitle: project, estimate, day/time, repeat, tags; tested separately from rendering | Sample rows observed in preview |
| D5 F5 Sync | Secondary F5 menu accelerator uses the same sync action and enablement | Live: F5 with sync enabled and intentionally incomplete process-only settings displayed the expected configuration warning; normal settings restored afterward |
| M1 Localization | English/German catalogs in app and package; package bundle lookup, compiler extraction, non-fuzzy gettext import, plural/type validation, compiled resource checks | German main window, shortcut guide, task/repeat forms, and General/Fonts/Sync/Backup settings inspected. Corrected Morning from “Morgen früh” to “Heute Morgen”; independent language review not performed |
| M2 System search creation | Native macOS 26 Create Task App Intent, required title in parameter summary, background creation through AppState returning a Task entity; separate Open Task action; extracted metadata verified discoverable | Deferred at user request; discovery and warm/cold execution not claimed |
| M3 System search activation | Resolves identifier to current title and opens Search; editor save finishes first, new drafts retained until user finishes; stale ID ignored | Deferred at user request; result activation not claimed |
| M4 Modifier warning | Command/Control/Option-specific explanation, tested; Option Quick Add uses Control-Option-N to avoid collision | Live: all three preferences and their corresponding warnings inspected; Command restored after checks |
| M5 Accessibility | Named notes/date/dismiss controls; high contrast/differentiate-without-color overrides custom colors; reduced motion; read-only audit script with self-test | Native tree audit: 0 unnamed controls in main window and task editor; keyboard, Increase Contrast and Differentiate Without Color passed. Spoken VoiceOver navigation remains unverified |

Core Spotlight indexes existing task content; it does not expose GNOME's callback for
arbitrary search query results. The macOS equivalent of “Create task…” is a discoverable
App Intent: choose Create Task in Spotlight, supply its title, and run it. See Apple's
[Shortcuts and Spotlight session](https://developer.apple.com/videos/play/wwdc2025/260/).
Do not count metadata extraction as a successful live invocation.

Native list multi-selection and the existing New Task / Sync / View Options toolbar
are retained. They were visible in the preview the user accepted. Mac-specific keyboard
choices remain as documented in handoff section 15.6.

## Drag and drop is required for acceptance

- [x] A selected row exports the selection in list order; an unselected row exports itself.
- [x] Project drops move parents and their children, with one undo for the batch.
- [x] Tag and Today/Morning/Tonight drops go through the shared engine.
- [x] Future tasks already carrying a slot tag always move **into** the drop destination.
- [x] Scheduled day moves clear obsolete times/reminders and undo restores them.
- [x] Selection reordering is one core operation/undo batch on both platforms.
- [x] Manual descending order follows visual ordering; slot undo preserves hidden neighbors.
- [x] Invalid IDs, child-to-top-level reorder, and drops within the selection emit no move.
- [x] Archived rows do not export drag payloads.
- [x] Live single-task and multi-selection project drops (user confirmed; single move also verified in saved data).
- [x] Live manual reordering and Command-Z restoration (user confirmed).
- [x] Live external plain-text/URL drop creates tasks (user confirmed).
- [ ] Separate native tag/day, descending-order and archive gestures: not individually verified; shared routing, ordering and archive exclusion are covered by automated tests.

## Handoff section 15.4: host-less test mapping

| Named behavior gap | Swift coverage |
|---|---|
| Tonight/tag creation | MVPParity.addingFromTonightAppliesTodayAndEvening / addingFromATagViewAppliesItsTag |
| Resulting sort order | MVPParity.sortingByTitleEstimateAndDirectionChangesTheVisibleOrder |
| Context-menu choices | MVPParity.contextMenuFactsFollowMovesCompletionAndSubtaskIdentity (facts; native menu rendering remains live acceptance) |
| Task form draft | TaskFormMapping.formParsesTimeEstimateAndNewTagsAndDisarmsInvalidTime |
| Existing form prefill | TaskFormMapping.editedFormRoundTripsTimeReminderAndTagOrderWithoutWritingOps |
| Repeat once / catch-up | MVPParity.repeatStartupAndRepeatedSameDayTicksCreateOnlyOneInstance / repeatStartupCatchesUpTheNewestMissedDayAndShowsItOverdue |
| Duplicate content | MVPParity.duplicatePreservesEditableContentButHasFreshOpenIdentity |
| Preferences binding | PreferenceDefaults, Changes.autoArchive, MVPParity.preferencesReadbackUpdatesColorsAndEveryShortcutModifier (state; native controls remain live acceptance) |
| Colorful labels | MVPParity preference readback + AccessibilityPreferences.systemAccessibilityOverridesCustomLabelColors |
| Quick-add window creates Today | TaskPresentation.systemCreationPlansTodayOutsideTheCurrentViewAndRejectsEmptyInput |
| System search creation/activation | TaskPresentation system creation + spotlight navigation/draft tests; compiled AppIntent metadata |
| Modifier/bindings/help | ShortcutsTests shared shortcut definitions + preference readback |

Tests use isolated stores/defaults, no app host, and no XCUITest. Repeat tests cover
startup, catch-up and same-day ticks; they do not alter the system clock to cross midnight.

## Verification recorded 2026-09-15

- Baseline before changes: 121 portable Rust tests, 70 Swift tests, GTK compile/test
  compilation, localization checks and Xcode build passed.
- Current portable Rust suite: **129 passed**, including **55 shared core tests**.
  Default and all-features/all-targets workspace variants passed.
- Current host-less Swift suite: **100 passed in 0.604 seconds**, 22 suites.
- Rust formatting, GTK compilation, GTK test compilation, POTFILES completeness,
  every PO file compilation, and LINGUAS consistency passed.
- German resources: **233 app + 141 package + 2 permission entries**, no missing entries;
  **33 plural entries**; seven importer tests passed. Built app/package German lookups,
  typed interpolation and singular/plural checks passed.
- Xcode Debug app build succeeded; CreateTaskIntent metadata is discoverable and includes
  the required title parameter. Local XCFramework is arm64, matching the installed target.
- Accessibility audit script compiles and its self-test passes.
- App launched over demo data; Today/Morning/Tonight/sample rows observed. User approved
  the preview. The Mac subsequently locked, preventing the remaining live interactions.

GTK UI tests are compiled locally but must run on Linux/Broadway as described in the
handoff. No remote CI run, commit, push, release, or signing change was performed.
The macOS workflow now uses the [macOS 26 runner](https://github.com/actions/runner-images),
matching the package deployment target, and runs the localization and audit-tool checks.
That workflow has not been executed remotely.

The old underscore warning was resolved by changing the macOS bundle identifier to
`com.codedbydan.Momentum`. The pre-existing empty Apple Events usage-description warning remains.

**Historical checkpoint:** MVP completion was pending the live acceptance items above. Re-run the handoff's
section 10 sweep after any fixes found there. Keep the preview's temporary demo data
separate from the real store.

## Live continuation — 2026-09-16

The unlocked Mac allowed native UI inspection of the current demo build. A real focus
defect surfaced: selecting a row left quick-add as first responder, so Command-A did
not select tasks. Explicit list focus on a row click fixed this; nine selected rows and
Escape deselection were observed in the accessibility tree. Command-O opened the task
editor, whose title, date, notes, tags and action controls exposed accessible names.

The shortcut guide initially collapsed its action names into one accessibility element
per section. Each entry now exposes its name and key combination separately; the live
German tree showed all entries. The German main window fit without visible clipping.
Its misleading Morning translation was corrected in app/package resources.

Automated drag attempts did not move tasks, and a temporary native item-provider
diagnostic received no source callback. The diagnostic was removed; this is **not**
evidence that drag-and-drop passes, nor enough to attribute the failure to the app.
A manual drag check was requested and remains unanswered. The production drag source
still uses Transferable; core drag/drop coverage remains green.

Spotlight could not be launched through the available UI surface (Launchd spawn error
162). Its warm/cold invocation and result activation remain unverified. The Mac locked
again before the remaining editor, drag, F5 and accessibility checks could finish.

### Empty-list layout refinement

At the user's request, empty lists now fill the detail column so the quick-add field
stays at the same top position as populated lists. The empty message uses a compact,
left-aligned symbol, heading and constrained-width guidance instead of a collapsing
status view. Guidance remains localized and scrolls in short windows. Xcode Debug
build passed; visual inspection at normal/small sizes is pending an unlocked Mac.

### Font customization

Settings → Fonts now uses the native NSFontPanel/NSFontManager to choose an app font
family and face. Content and interface sizes are independently persisted (10–32 pt),
with live preview, immediate updates, reset and missing-font fallback. Content includes
task titles/subtitles, notes, subtasks and quick-add. UI labels, app controls, helper text
and headings use interface sizing; OS-managed menu bars/title bars remain native.
Settings is resizable for larger text. Three new host-less typography tests cover
independent sizes, font persistence, invalid sizes, unavailable fonts and scoped reset;
all **103 Swift tests in 23 suites** passed. All 16 new German strings are translated.
Native font-panel interaction and large-text visual acceptance remain pending because
the Mac is locked; passing model tests does not verify that panel's live callback.

### macOS bundle identity

The app now builds and signs as `com.codedbydan.Momentum`. XcodeGen, generated Info.plist,
URL registration, Spotlight domain, the exported drag type and accessibility lookup
use the new namespace. Existing preferences migrate once, without overwriting new-domain
choices or changing the old domain. Task storage and the shared Keychain service are
unchanged. LibreSync's protocol app ID remains shared with Linux.
The Debug build, strict code-signature validation and all **105 Swift tests in 24 suites**
passed, including two migration tests. The built Info.plist and code signature were read
back to verify the exact requested identifier.

### LibreSync bridge integration

The opt-in `NearbySyncTests.swiftBridgePairsAndRefreshesTasksInBothDirections` passed
locally in 9.422 seconds. Two isolated Engine instances paired over loopback using
file-backed test keys; the receiving Swift state and search index updated after a
task was created, completion synced back, and unlinking refreshed the device list.
This exercises the real UniFFI transport and `P2pBridge` event delivery without an app
host. The macOS workflow now runs this separately from the fast, network-free suite;
that remote workflow has not been executed. Physical Mac-to-Linux discovery/sync and
native UI acceptance remain unverified. The Mac was still locked at this checkpoint.

### Native font and empty-state acceptance — 2026-09-16

The system font panel initially did not become the accessible active window after
Choose Font. Explicitly making the panel key after ordering it forward resolved this:
the native family, face and size controls became available. Atkinson Hyperlegible
Regular was selected, content size changed to 20 pt in the panel, and interface size
independently changed to 16 pt in Settings. All three preferences survived an app
restart. Reset restored System Font and both sizes to 13 pt. The font label's explicit
accessibility value now updates with the chosen face; live readback verified it.
The final Xcode Debug build and `git diff --check` passed.

The main task list and empty Inbox were visually inspected at 20 pt content / 16 pt UI.
The empty Inbox keeps quick-add pinned at the top with a compact message beneath it.
Small-window and maximum-size checks remain pending. Command, Control and Option
preference warnings were inspected live, then Command was restored.

A further task drag did not change its project. An automated native window-edge drag
also did not resize the window. This leaves gesture delivery uncertain; neither the
drag/drop MVP requirement nor compact-window acceptance is signed off. UI connection
timeouts also occurred during this pass. The preview remains on isolated demo data.

### Keyboard, German forms and large typography — 2026-09-16

Live keyboard checks completed a sample task with Space, restored it with Command-Z,
opened its editor with Return, advanced to Project with Tab and dismissed with Escape.
German new-task fields and General/Fonts settings exposed translated labels. This is
partial keyboard/localization acceptance, not a full VoiceOver or language review.

Large typography revealed that native List styles overrode inherited navigation and
section-header fonts. These labels now apply the selected font directly. Sidebar
width scales within bounds, and an opaque footer prevents scrolling labels from
overlapping New Task. The task sheet now widens with the chosen text sizes, capped at
760 points and the screen's available width, with vertically scrollable content.
Live checks observed enlarged sidebar labels, separate footer content, and a wider
task sheet with both content and interface set to 32 pt. Notes remained reachable by
scrolling. The preview was returned to English and System Font / 13 pt for both sizes.

The final Debug build and whitespace validation passed. The host-less regression run
reported 106 tests across 25 suites in 1.373 seconds (105 executed; the opt-in Nearby
integration test was skipped). No shared Rust or Linux code changed in this pass.
Drag/drop, Spotlight invocation, full assistive-technology checks,
compact-window checks and physical Mac/Linux sync remain pending.

### F5 and native paste — 2026-09-16

F5 invoked Sync Now with a process-only `sync-enabled` override, automatic/nearby sync
disabled, and empty server/user overrides. The expected configuration warning appeared
without a server connection. Relaunching without those arguments restored normal settings.

Native multi-line paste exposed an actual defect: NSTextField's editor consumed Paste
before SwiftUI's `onPasteCommand`, leaving newlines in the input instead of creating tasks.
Quick-add now observes multi-line text changes and routes them through `addFromText`;
single-line input retains the native editor's cursor/selection semantics. Live readback
verified two separate tasks from two pasted lines and replacement of selected text by
ordinary paste. The focused import regression test and Debug build passed; whitespace
and strict signature checks passed. This verifies paste, not external drag/drop.

Another selected-row drag produced no project change. A manual check was requested.
Spotlight's system-menu UI surface timed out. Docker is installed but no daemon socket
is present, so Linux runtime/physical cross-platform acceptance is still unavailable.

### Compact-window acceptance — 2026-09-16

The native Window → Move & Resize → Top Left command produced a roughly 961 × 493
preview. The empty Inbox kept quick-add at the top, with its heading and guidance
visible and unclipped. The native Return to Previous Size command restored the window.
Both temporary paste-test tasks were removed through the native Delete command, with
undo feedback observed. This covers a short window; the exact 680-point minimum-width
layout has not been exercised.

Command-Space did not expose a Spotlight surface to the UI tool. Manual drag feedback
is pending, as is permission to temporarily enable VoiceOver/Increase Contrast and
restore the original settings afterward. These are unresolved acceptance gates, not
failed or passing product claims. Linux runtime/device availability is also required.

### Dock badge preferences and destructive styling — 2026-09-16

Settings → General → Desktop now offers Due or scheduled today (the existing default),
Today including overdue, and None. Counts include only unfinished top-level tasks;
zero hides the badge. The Today option uses the core's Today listing even while another
view is selected. Preference changes update the badge immediately and at service startup;
task changes, undo, sync and the existing refresh cycle keep it current. Menu-bar counts
remain independent. The task editor's Delete Task button is visibly red and retains its
destructive role; the menu command now has the same role.

The Swift run reported 108 tests across 25 suites (107 executed, one opt-in Nearby test
skipped). Badge tests cover mode changes, overdue inclusion, completion/undo, disabled
badges, invalid preference fallback, and parent/subtask counting. Debug build, strict
signature validation, whitespace checks and complete English/German catalogs passed.
Native UI readback verified all three choices and the red Delete Task button. The saved
badge preference was returned to dueToday. Dock UI automation timed out, so the badge's
rendered appearance itself remains unverified; its update counts are tested.

### HIG design pass — 2026-09-16

See MACOS-DESIGN-AUDIT.md for the source-backed audit, design decisions, fixes and
verification boundaries. Refined list hierarchy, row spacing/wrapping, quick-add
focus and feedback, editor typography, tag selection, empty/all-done states and
completion feedback. Kept native controls, system accent, custom fonts and shared
task semantics. Removed Return from Undo toasts and enforced a 10 pt caption floor.
Live completion exposed an AppKit reentrant table callback; deferring its mutation
removed the reproduced warning.

Debug build, signature check, complete catalogs and 107 executed Swift tests passed
(one opt-in test skipped). Native checks covered dark/light, larger fonts, short
window, capture with Undo visible, Delete/Undo, empty-state focus and the task editor.
This improves the presentation; it does not close the earlier drag/drop, Spotlight,
assistive-technology or physical Mac/Linux MVP acceptance gates.

### CLI parity and Shortcuts.app

See [MACOS-AUTOMATION.md](MACOS-AUTOMATION.md) for the implementation, automated tests, native Shortcuts chains, bundled CLI verification and remaining platform checks. This addition does not close the outstanding handoff acceptance items.


### Single sync service and visible status (2026-09-16)

- Settings → Sync selects Off, Nextcloud, or LibreSync; the separate Nearby switch/tab
  is removed. Automatic sync and `mo sync` respect the selected transport. Connections
  and linked devices are preserved when switching.
- Main-window status is visible on empty and populated lists. Both transports expose
  progress; failed LibreSync cycles surface error details and Retry instead of a silent
  debug log. Successful unchanged exchanges update the last-sync time.
- Initial LibreSync bootstrap remains pending until a valid snapshot arrives. Later
  snapshots still cannot resurrect deleted tasks. An isolated regression covers empty
  exchange, invalid snapshot, eventual import, and no resurrection.
- Demo mode cannot access sync; its disposable store is no longer allowed to pair with
  real devices. The existing user data was backed up and moved to the previously empty
  normal store before relaunching without demo mode.
- Validation: 120 Swift tests reported (one opt-in network test skipped in the default
  run), 62 core tests, 22 CLI integration tests. The opt-in two-node Swift test also
  passed, covering bidirectional changes, visible progress, unchanged success, and
  no-linked-device failure. Signed Debug build and English/German catalog audit passed.
- Live UI: the selector offers all three methods; error details surfaced the linked
  Linux peer's connection failure. Local-network permission was already enabled.
  The user selected Nextcloud during verification; its successful sync and progress
  state were observed and the choice preserved. Physical Linux → Mac LibreSync task
  verification remains pending; the linked peer timed out over LAN and its hostname.


### MVP acceptance continuation — 2026-09-16

The user requested finishing MVP and explicitly deferred LibreSync and Spotlight
testing for this pass. These are recorded acceptance exceptions, not passes.

- Fixed sidebar selection: Today ignores the stored task-data color and uses the
  user's system accent. Selected rows use the accent background and the system's
  contrasting selected text/icon color; Today remains the launch/default view.
- User verified a native single-task drag into Home. Saved-store readback confirmed
  the sample task's project changed to Home.
- User then confirmed all three native checks passed: dragging a multi-selection,
  manual reordering with Command-Z restoration, and dropping external text/a URL to
  create a task. Automated native gesture delivery was the limitation; the existing
  working SwiftUI drag implementation was retained. Core tests cover tag/day routes,
  descending order, batch undo, and archived rows refusing payloads.
- Full current handoff local sweep passed: 152 portable Rust tests per workspace
  variant (default and all-features/all-targets), GTK compilation and GTK test
  compilation, formatting, POTFILES/PO/LINGUAS checks, seven localization-tool tests,
  and accessibility-audit self-test. Swift reported 120 tests (one opt-in network
  test skipped); no additional LibreSync test run was requested after the exception.
  Compiled German app/package resource validation and the signed Debug build passed.
- Spotlight UI automation reports macOS launch error 162. The user subsequently
  requested skipping Spotlight; no successful invocation or activation is claimed.
- Native accessibility audit passed with 22 controls in the main window and 36 in
  the task editor, with zero unnamed controls. The audit recognizes AppKit's semantic
  window-control subroles while still rejecting unnamed custom buttons; self-tests
  cover both. Keyboard completion/Undo, editor focus and dismissal passed.
- Increase Contrast and Differentiate Without Color visibly updated labels/borders.
  Both settings were restored and read back. VoiceOver did not remain enabled and
  its spoken navigation could not be verified; the original off state was verified.
- German task and repeat forms, General/Fonts/Sync/Backup settings and the main window
  were inspected. Checked labels fit; task content remains in its original language.
  The German locale was a process-only override and ended with the preview.
- Final strict code-signature validation and whitespace checks passed. Closed the
  isolated preview and launched the signed normal app without demo/locale flags.
  Live readback showed Today selected, the system orange accent behind its contrasting
  white star/text, and Nextcloud reporting “Last synced just now”. No commit or push.
