// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
import AppKit
import SwiftUI
import Testing
@testable import MomentumKit

@Suite struct ShortcutsTests {
    @Test func configurableBindingsHaveNoCollisions() {
        for modifier in ModifierKey.allCases {
            let bindings = AppShortcut.all.map { $0.binding(modifier: modifier) }
            for (index, binding) in bindings.enumerated() {
                #expect(!bindings.prefix(index).contains {
                    $0.key == binding.key && $0.modifiers == binding.modifiers
                }, "Duplicate accelerator for \(AppShortcut.all[index]) with \(modifier)")
            }
        }
    }

    @Test func systemBindingsStayFixedWhileAppBindingsFollowPreference() {
        for modifier in ModifierKey.allCases {
            #expect(AppShortcut.newTask.binding(modifier: modifier).modifiers == modifier.modifiers)
            #expect(AppShortcut.selectAll.binding(modifier: modifier).modifiers == .command)
            #expect(AppShortcut.settings.binding(modifier: modifier).modifiers == .command)
            #expect(AppShortcut.quit.binding(modifier: modifier).modifiers == .command)
            #expect(AppShortcut.selectAll.display(modifier: modifier) == "⌘A")
        }
    }

    @Test func optionQuickAddIsDistinctAndSymbolsAreDeduplicated() {
        #expect(AppShortcut.newTask.display(modifier: .option) == "⌥N")
        #expect(AppShortcut.quickAdd.display(modifier: .option) == "⌃⌥N")
        #expect(AppShortcut.moveUp.display(modifier: .option) == "⌥↑")
        for modifier in ModifierKey.allCases {
            for shortcut in AppShortcut.all {
                let display = shortcut.display(modifier: modifier)
                for symbol in ["⌘", "⌃", "⌥", "⇧"] {
                    #expect(display.components(separatedBy: symbol).count <= 2)
                }
            }
        }
    }

    @Test func syncHasUnmodifiedF5AlongsideConfigurableR() {
        for modifier in ModifierKey.allCases {
            let function = AppShortcut.syncFunction.binding(modifier: modifier)
            #expect(function.key.character == Character(UnicodeScalar(NSF5FunctionKey)!))
            #expect(function.modifiers.isEmpty)
            #expect(AppShortcut.syncFunction.display(modifier: modifier) == "F5")
            #expect(AppShortcut.sync.binding(modifier: modifier).key == "r")
            #expect(AppShortcut.sync.binding(modifier: modifier).modifiers == modifier.modifiers)
        }
    }

    @Test func warningsExplainModifierSpecificSystemConflicts() {
        #expect(ModifierKey.command.shortcutWarning.contains("macOS"))
        #expect(ModifierKey.control.shortcutWarning.contains("text"))
        #expect(ModifierKey.option.shortcutWarning.contains("characters"))
        #expect(ModifierKey.control.shortcutWarning != ModifierKey.option.shortcutWarning)
    }

    @MainActor @Test func selectAllKeepsTextSelectionOutOfTheTaskList() {
        let text = NSTextView()
        text.string = "Task title draft"
        text.setSelectedRange(NSRange(location: 4, length: 0))
        var selectedTasks = false
        performSelectAll(firstResponder: text) { selectedTasks = true }
        #expect(text.selectedRange() == NSRange(location: 0, length: text.string.utf16.count))
        #expect(!selectedTasks)
    }

    @MainActor @Test func selectAllUsesOnlyTheFocusedListAction() {
        var selectedTasks = false
        performSelectAll(firstResponder: NSView()) { selectedTasks = true }
        #expect(selectedTasks)
        performSelectAll(firstResponder: nil, selectTasks: nil)
    }
}
