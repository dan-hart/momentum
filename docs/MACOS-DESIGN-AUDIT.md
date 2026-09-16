# macOS design audit — September 16, 2026

## Direction and scope

Native, calm and encouraging: the quality of Apple's productivity apps, with fast
capture and satisfying completion. Preserve system accent, custom typography,
native selection/keyboard operation, drag/drop and the handoff's MVP requirements.
This is a macOS presentation pass; shared Rust task semantics and Linux stay intact.

Reviewed the main list, sidebar, quick-add, task editor, empty/all-done states,
Settings and feedback against current Apple guidance:

- [Designing for macOS](https://developer.apple.com/design/human-interface-guidelines/designing-for-macos)
- [Design principles](https://developer.apple.com/design/human-interface-guidelines/design-principles)
- [Typography](https://developer.apple.com/design/human-interface-guidelines/typography)
- [Accessibility](https://developer.apple.com/design/human-interface-guidelines/accessibility)
- [Materials](https://developer.apple.com/design/human-interface-guidelines/materials)
- [Motion](https://developer.apple.com/design/human-interface-guidelines/motion)
- [Toolbars](https://developer.apple.com/design/human-interface-guidelines/toolbars)

## Verdict

The foundation is recognizably native, without generic dashboard/card decoration.
The weakness is an underdeveloped hierarchy: compressed rows, low-emphasis task
entry, small metadata, and an editor where titles compete with secondary fields.
Preserve the split view, native controls, system colors and contextual commands.
Avoid decorative glass on task content, perpetual animation, sounds or streaks.

## Findings and changes

No new critical blocker found in the reviewed flows. Two high-priority interaction
or accessibility findings and eight medium-priority design findings:

| Priority | Location | Finding and impact | Change |
|---|---|---|---|
| High | ContentView / ToastStack | Every Undo toast claims default Return, competing with task entry and editor actions. | Remove that shortcut; preserve Edit → Undo and the explicit button. |
| High | Typography | Caption scaling bottoms out at 9 pt, below Apple's recommended 10 pt macOS minimum. | Enforce a 10 pt floor; regression test first reproduced the failure. |
| Medium | ContentView | The small window title is the main orientation cue; content starts abruptly. | Add a scalable list title and a localized date for daily views. |
| Medium | QuickAddField | Plain field has a weak focus cue; syntax crowds its placeholder; submission has little local feedback. | Clear accent focus outline, shorter prompt, explicit Add action, syntax help and a brief checkmark transition. |
| Medium | TaskRowView / TaskListView | Dense rows and single-line metadata impede scanning, especially at large sizes. | More row spacing, stronger section headings, wrapped metadata and completed-title strikethrough. Preserve native checkboxes and selection. |
| Medium | TaskFormSheet | Title looks like a minor field; narrow form and inset notes background add friction. | Wider editor, multiline title with stronger hierarchy, integrated notes background. |
| Medium | TaskFormSheet / TagChips | Selected chips depend heavily on background color; colored small labels can be hard to read. | Explicit selected checkmark, semantic text color and a separate project-defined color dot. |
| Medium | SidebarView / empty states | New Project is difficult to discover; empty views explain actions without directly offering one. | Add an accessible sidebar New Project control and an empty-state button that focuses quick-add. |
| Medium | Feedback | Completion feels abrupt; default auto-archive bypasses the all-done panel. | Brief list transitions, completion toast checkmark, one all-done symbol pulse, and more adaptable toast shape. |
| Medium | Task list callbacks | Live completion reproduced an AppKit reentrant table-mutation warning. | Defer checkbox, keyboard completion and deletion mutations until the table callback finishes. Completion was repeated without the warning. |

## Motion and accessibility requirements

- No delayed task mutations, waiting for animation, or perpetual effects.
- Capture feedback: brief native symbol replacement; its reset task cancels safely.
- Completion: 200 ms list transition; native checkboxes remain native.
- All done: one SF Symbol pulse; no confetti or repeated bouncing.
- Reduce Motion disables custom movement/symbol effects.
- Reduce Transparency gives toasts an opaque background with an outline.
- Custom font family and independent content/UI sizes continue to apply.
- Use semantic text/background colors; keep destructive actions red and named.
- Verify Return-to-add with Undo visible, completion/Undo, tag selection, empty-state
  focus, small windows, light/dark appearance, and larger typography.

## Acceptance boundaries

This audit is not a declaration that every sheet, accessibility mode or physical
device has passed. The existing native drag/drop, Spotlight, VoiceOver and physical
Mac/Linux acceptance work remains recorded in MACOS-MVP-CHECKLIST.md. Full contrast
measurement and animation frame-time profiling have not been performed.

## Verification

- Debug build and strict signature verification passed.
- Swift tests: 108 reported across 25 suites in 1.405 seconds; 107 executed,
  one opt-in Nearby integration test skipped. Typography regression and completion/
  Undo feedback assertions passed.
- English/German catalogs complete: app 253, package 144, permissions two.
- Native checks passed: Return creates while Undo is visible; Delete followed by
  Command-Z restores the task; empty-state Add focuses the composer; editor title,
  selected tag and notes styling; light/dark main views; all-done panel; 24 pt
  content with 20 pt UI; roughly 961 × 493 short-window layout and restoration.
- Temporary appearance/font/auto-archive overrides were process-only. Debug demo
  builds now accept MOMENTUM_PREVIEW_APPEARANCE=light or dark for repeatable checks
  without changing system settings. Normal launch continues to follow the system.
- The automation accessibility snapshot folds the all-done action into its list
  row. Its full VoiceOver interaction remains unverified, along with live Reduce
  Motion/Transparency settings, exact minimum-width fit and contrast measurements.
- No commits, pushes, shared Rust changes or Linux changes in this pass.
