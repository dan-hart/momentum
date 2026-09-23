<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!-- Copyright (C) 2026 Dan Hart -->
# Accessibility

Momentum is built from standard GTK 4 and libadwaita widgets, so it gets the toolkit's
screen-reader support, keyboard navigation, high contrast and large text for free. This
page lists what the app adds on top, how to check it, and what to keep in mind when
changing the UI.

## What the app guarantees

- **Every control has a name.** Icon-only buttons carry an `accessibility { label }` in
  Blueprint or `update_property(Label)` in Rust; a tooltip alone is not enough for Orca.
  Each task's check box is named "Done: <title>" (or "Select: <title>" in selection mode),
  so tabbing through the list reads the task, not "check box".
- **Badges speak.** The notes, repeat and reminder icons on a row have accessible labels
  ("Has notes", "Repeats every Monday", "Reminder at 15:00"), and the same text is in the
  tooltip.
- **Everything works from the keyboard.** Rows are focusable list items; Menu or Shift+F10
  opens the same context menu as a right click; every mouse gesture has a shortcut
  (`Keyboard Shortcuts` in the main menu, Ctrl+?): drag to reorder is Ctrl+Up/Down, drop on
  Today is Ctrl+T, drop on a project is Ctrl+M, multi-select is Ctrl+click or Select Tasks.
- **High contrast disables colour coding.** Project and tag colours come from the sync
  data, so their contrast cannot be guaranteed; when the system high-contrast setting is
  on, labels fall back to plain text and monochrome icons regardless of the "Colorful
  labels" preference. The window refreshes live when the setting changes.
- **Large text and zoom.** No fixed pixel sizes for text; lists reflow, the sidebar
  collapses below 640sp, and the dialogs scroll.
- **Colour is never the only signal.** Overdue tasks sit under an "Overdue" heading with
  the date spelled out; done tasks are dimmed and moved to a "Completed" section, not just
  struck through.
- **Reduced motion.** Nothing animates on its own; the sync spinner is the only motion and
  appears only for syncs longer than a second.

## Checking a change

Automated, on the host, with the app open:

```sh
build-aux/a11y-dump.py          # problems only, exit 1 if a control has no name
build-aux/a11y-dump.py --all    # the whole tree, useful to see what Orca will read
```

The script walks the AT-SPI tree over D-Bus (it needs only `gdbus`) and lists every
button, entry, check box, list item and menu item without a name or description. Run it
against the demo data so the result is reproducible:

```sh
MOMENTUM_DEMO=1 flatpak run io.github.dan_hart.Momentum.Devel &
build-aux/a11y-dump.py
```

Manual, once per release:

1. **Orca**: Super+Alt+S toggles it. Tab through the main window, a task row's context
   menu, the New Task dialog and Preferences; everything should be announced with a
   sensible name, and no control should be read as just its role.
2. **High contrast**: Settings › Accessibility › Seeing › High Contrast. Colour-coded
   labels must vanish; nothing may become unreadable.
3. **Large text**: Settings › Accessibility › Seeing › Large Text, then the "Text scaling
   factor" further up. No clipped labels; the sidebar and dialogs must still fit.
4. **Keyboard only**: unplug the mouse (or don't touch it) and add, edit, complete,
   reorder, move and delete a task, then sync.

## When adding UI

- Icon-only button: add `tooltip-text` and an `accessibility { label }` (Blueprint) or
  `update_property(&[Property::Label(..)])` (Rust). Same text for both.
- Decorative image: leave it unnamed; GTK hides it from the screen reader. Meaningful
  image: give it a label.
- New shortcut: add it to `data/resources/ui/shortcuts.blp` too.
- Colour: only through `color_class`/`hex_color`, which respect the high-contrast switch.


## macOS

The macOS front end uses SwiftUI/AppKit controls and native list multi-selection. Notes,
due-date controls, task checkboxes, reminder/repeat badges and dismiss buttons have
accessible names. Color labels defer to Increase Contrast and Differentiate Without
Color, and toast animations defer to Reduce Motion. The color policy is host-less tested.

With a demo window open, a read-only audit can check the actual AX tree:

```sh
swift macos/scripts/a11y-audit.swift --all
swift macos/scripts/a11y-audit.swift --pid PID
swift macos/scripts/a11y-audit.swift --self-test
```

The audit checks named interactive controls, exits 1 for missing names, and exits 2 if no
readable window/accessibility permission is available. It never enables permissions.
Use a demo instance: `--all` prints visible task/control text. Test New Task, an editor,
Settings, menus, and the main list separately. The checker does not prove useful reading
order, contrast, keyboard operability, or correct VoiceOver speech; review those manually.

The live macOS audit is still pending; see [MACOS-MVP-CHECKLIST.md](MACOS-MVP-CHECKLIST.md).

## iOS and iPadOS

The iOS client uses native SwiftUI controls, semantic system colors, Dynamic Type,
SF Symbols with accessibility labels, non-color selection state, Reduce Motion guards,
and an Increase Contrast exception for calculated foreground ink over accent fills.
Normal-contrast buttons using the default `#FF6600` accent render white text and symbols.

The automated iOS suite is unit-only. Portable Swift tests cover state and rules;
`MomentumViewTests` hosts the real production SwiftUI source in `UIHostingController`
and asserts geometry, pixels, localization and accessibility metadata under explicit
appearance, contrast and text-size traits. The project contains no XCUITest target and
does not use ViewInspector. Run the combined lane on a disposable simulator:

Asynchronous Quick Add, task, recurrence, project-move, backup and sync failures use
semantic error ink and explicit accessibility focus. Inline estimate correction stays
on the active field as an accessibility hint. The shared error presentation stacks at
accessibility text sizes and is rendered at phone and iPad widths in unit tests.

```sh
python3 ios/scripts/test.py unit \
  --destination 'platform=iOS Simulator,id=YOUR_TEST_SIMULATOR'
```

Unit tests cannot prove VoiceOver speech/order, Voice Control, Switch Control, Full
Keyboard Access, gestures, system permission prompts or cross-app behavior. Check those
manually on a disposable simulator when affected and record the exact OS/configuration.
See [the iOS testing guide](../ios/TESTING.md) and the
[dated audit](audits/2026-09-16-ios-accessibility.md).
