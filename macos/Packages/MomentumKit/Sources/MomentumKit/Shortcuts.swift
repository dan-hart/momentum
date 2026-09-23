// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
#if os(macOS)
import AppKit
#endif
import SwiftUI

/// The menu bindings and their displayed equivalents share this definition.
public enum AppShortcut: Hashable, Sendable {
    case newTask, newProject, quickAdd, search, focusQuickAdd, undo, shortcuts
    case settings, closeWindow, quit, selectAll, deselectAll, toggleSidebar
    case openTask, markDone, deleteTask, duplicateTask, copyTitle, planToday
    case morning, tonight, tomorrow, nextWeek, moveToProject, repeatSchedule
    case moveUp, moveDown, archive, sidebar(Int), nextView, previousView
    case sync, syncFunction, globalQuickAdd, globalShow

    public static let all: [AppShortcut] = [
        .newTask, .newProject, .quickAdd, .search, .focusQuickAdd, .undo, .shortcuts,
        .settings, .closeWindow, .quit, .selectAll, .deselectAll, .toggleSidebar,
        .openTask, .markDone, .deleteTask, .duplicateTask, .copyTitle, .planToday,
        .morning, .tonight, .tomorrow, .nextWeek, .moveToProject, .repeatSchedule,
        .moveUp, .moveDown, .archive, .nextView, .previousView, .sync, .syncFunction,
        .globalQuickAdd, .globalShow,
    ] + (1...9).map { .sidebar($0) }

    public func binding(modifier: ModifierKey) -> KeyboardShortcut {
        let base = modifier.modifiers
        let shifted = base.union(.shift)
        let option = base.union(.option)
        switch self {
        case .newTask: return KeyboardShortcut("n", modifiers: base)
        case .newProject: return KeyboardShortcut("n", modifiers: shifted)
        // Option+Option is still Option; use Control to keep Quick Add distinct.
        case .quickAdd: return KeyboardShortcut("n", modifiers: modifier == .option ? [.control, .option] : option)
        case .search: return KeyboardShortcut("f", modifiers: base)
        case .focusQuickAdd: return KeyboardShortcut("l", modifiers: base)
        case .undo: return KeyboardShortcut("z", modifiers: base)
        case .shortcuts: return KeyboardShortcut("/", modifiers: base)
        case .settings: return KeyboardShortcut(",", modifiers: .command)
        case .closeWindow: return KeyboardShortcut("w", modifiers: .command)
        case .quit: return KeyboardShortcut("q", modifiers: .command)
        case .selectAll: return KeyboardShortcut("a", modifiers: .command)
        case .deselectAll: return KeyboardShortcut("a", modifiers: shifted)
        case .toggleSidebar: return KeyboardShortcut("s", modifiers: [.control, .command])
        case .openTask: return KeyboardShortcut("o", modifiers: base)
        case .markDone: return KeyboardShortcut("d", modifiers: base)
        case .deleteTask: return KeyboardShortcut(.delete, modifiers: base)
        case .duplicateTask: return KeyboardShortcut("d", modifiers: shifted)
        case .copyTitle: return KeyboardShortcut("c", modifiers: shifted)
        case .planToday: return KeyboardShortcut("t", modifiers: base)
        case .morning: return KeyboardShortcut("m", modifiers: shifted)
        case .tonight: return KeyboardShortcut("t", modifiers: shifted)
        case .tomorrow: return KeyboardShortcut(.rightArrow, modifiers: shifted)
        case .nextWeek: return KeyboardShortcut(.downArrow, modifiers: shifted)
        case .moveToProject: return KeyboardShortcut("m", modifiers: base)
        case .repeatSchedule: return KeyboardShortcut("r", modifiers: shifted)
        case .moveUp: return KeyboardShortcut(.upArrow, modifiers: option)
        case .moveDown: return KeyboardShortcut(.downArrow, modifiers: option)
        case .archive: return KeyboardShortcut("e", modifiers: base)
        case .sidebar(let number): return KeyboardShortcut(KeyEquivalent(Character(String(number))), modifiers: base)
        case .nextView: return KeyboardShortcut("]", modifiers: shifted)
        case .previousView: return KeyboardShortcut("[", modifiers: shifted)
        case .sync: return KeyboardShortcut("r", modifiers: base)
        // Apple function-key character for F5 (NSF5FunctionKey on macOS).
        case .syncFunction: return KeyboardShortcut(KeyEquivalent("\u{F708}"), modifiers: [])
        case .globalQuickAdd: return KeyboardShortcut("t", modifiers: [.control, .option])
        case .globalShow: return KeyboardShortcut("m", modifiers: [.control, .option])
        }
    }

    public func display(modifier: ModifierKey) -> String {
        let shortcut = binding(modifier: modifier)
        let symbols: [(EventModifiers, String)] = [(.control, "⌃"), (.option, "⌥"), (.shift, "⇧"), (.command, "⌘")]
        let prefix = symbols.filter { shortcut.modifiers.contains($0.0) }.map(\.1).joined()
        let key: String
        switch shortcut.key {
        case .delete: key = "⌫"
        case .upArrow: key = "↑"
        case .downArrow: key = "↓"
        case .rightArrow: key = "→"
        default: key = self == .syncFunction ? "F5" : String(shortcut.key.character).uppercased()
        }
        return prefix + key
    }
}

#if os(macOS)
extension ModifierKey {
    public var shortcutWarning: String {
        switch self {
        case .command:
            return String(localized: "Shortcuts reserved by macOS may not reach Momentum.", bundle: .module)
        case .control:
            return String(localized: "Control shortcuts can conflict with text editing and macOS navigation. Shortcuts reserved by macOS may not reach Momentum.", bundle: .module)
        case .option:
            return String(localized: "Option shortcuts can conflict with accented characters and alternate symbols. Shortcuts reserved by macOS may not reach Momentum. Quick Add uses Control–Option–N to stay distinct from New Task.", bundle: .module)
        }
    }
}

/// A focused text editor keeps native Select All; a task action exists only while its
/// list owns focus. This also prevents selection changes in another window or sheet.
@MainActor public func performSelectAll(firstResponder: NSResponder?, selectTasks: (() -> Void)?) {
    if let text = firstResponder as? NSTextView {
        text.selectAll(nil)
    } else {
        selectTasks?()
    }
}
#endif
