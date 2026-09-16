// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// The menu bar. Every shortcut uses the configured modifier (⌘ by default) so the app's
// own commands read as one family; system-provided items (Settings, Toggle Sidebar,
// Select All, Quit) keep their platform keys.
import MomentumCore
import MomentumKit
import SwiftUI

struct MomentumCommands: Commands {
    let state: AppState

    @FocusedValue(\.selectAllTasks) private var selectAllTasks

    private func shortcut(_ action: AppShortcut) -> KeyboardShortcut {
        action.binding(modifier: state.modifier)
    }
    private var hasTarget: Bool { !state.targets.isEmpty }
    private var single: Bool { state.target != nil }

    var body: some Commands {
        CommandGroup(replacing: .newItem) {
            Button(String(localized: "New Task…")) { state.newTask() }
                .keyboardShortcut(shortcut(.newTask))
            Button(String(localized: "New Project…")) { state.sheet = .newProject }
                .keyboardShortcut(shortcut(.newProject))
            Button(String(localized: "Quick Add…")) { WindowOpener.shared.open?("quick-add") }
                .keyboardShortcut(shortcut(.quickAdd))
            Divider()
            Button(String(localized: "Import Backup…")) { importBackup() }
            Button(String(localized: "Export Backup…")) { exportBackup() }
        }

        CommandGroup(replacing: .undoRedo) {
            Button(state.undoMenuTitle) { state.undo() }
                .keyboardShortcut(shortcut(.undo))
                .disabled(!state.canUndo)
        }

        CommandGroup(replacing: .textEditing) {
            Button(String(localized: "Select All")) {
                performSelectAll(firstResponder: NSApp.keyWindow?.firstResponder, selectTasks: selectAllTasks)
            }
            .keyboardShortcut(shortcut(.selectAll))
        }

        CommandGroup(after: .pasteboard) {
            Divider()
            Button(String(localized: "Copy Task Title")) { if let id = state.target { state.copyTitle(id) } }
                .keyboardShortcut(shortcut(.copyTitle))
                .disabled(!single)
            Button(String(localized: "Duplicate Task")) { if let id = state.target { state.duplicate(id) } }
                .keyboardShortcut(shortcut(.duplicateTask))
                .disabled(!single)
            Button(String(localized: "Delete Task"), role: .destructive) { state.delete(state.targets) }
                .keyboardShortcut(shortcut(.deleteTask))
                .disabled(!hasTarget)
            Divider()
            Button(String(localized: "Deselect All")) { state.selection.removeAll() }
                .keyboardShortcut(shortcut(.deselectAll))
                .disabled(state.selection.isEmpty)
        }

        CommandMenu(String(localized: "Task")) {
            Button(String(localized: "Open Task")) { if let id = state.target { state.open(id) } }
                .keyboardShortcut(shortcut(.openTask))
                .disabled(!single)
            Button(doneTitle) { state.toggleDone(state.targets) }
                .keyboardShortcut(shortcut(.markDone))
                .disabled(!hasTarget)
            Divider()
            Button(todayTitle) { state.toggleToday(state.targets) }
                .keyboardShortcut(shortcut(.planToday))
                .disabled(!hasTarget)
            Button(slotTitle(.morning)) { state.toggleSlot(state.targets, .morning) }
                .keyboardShortcut(shortcut(.morning))
                .disabled(!hasTarget)
            Button(slotTitle(.tonight)) { state.toggleSlot(state.targets, .tonight) }
                .keyboardShortcut(shortcut(.tonight))
                .disabled(!hasTarget)
            Button(String(localized: "Move to Tomorrow")) { state.moveToTomorrow(state.targets) }
                .keyboardShortcut(shortcut(.tomorrow))
                .disabled(!hasTarget)
            Button(String(localized: "Move to Next Week")) { state.moveToNextWeek(state.targets) }
                .keyboardShortcut(shortcut(.nextWeek))
                .disabled(!hasTarget)
            Button(String(localized: "Move to Project…")) { state.sheet = .moveToProject(state.selectedTopLevel) }
                .keyboardShortcut(shortcut(.moveToProject))
                .disabled(state.selectedTopLevel.isEmpty)
            Button(String(localized: "Add Tag…")) { state.sheet = .addTag(state.targets) }
                .disabled(!hasTarget)
            Divider()
            Button(repeatTitle) { if let id = state.target { state.sheet = .repeatSchedule(id) } }
                .keyboardShortcut(shortcut(.repeatSchedule))
                .disabled(!(single && state.row(state.target ?? "")?.isSubtask == false))
            Divider()
            Button(String(localized: "Move Up")) { state.nudge(-1) }
                .keyboardShortcut(shortcut(.moveUp))
                .disabled(!single)
            Button(String(localized: "Move Down")) { state.nudge(1) }
                .keyboardShortcut(shortcut(.moveDown))
                .disabled(!single)
            Divider()
            Button(String(localized: "Archive Completed Tasks")) { state.archiveDone() }
                .keyboardShortcut(shortcut(.archive))
                .disabled(!state.listing.canArchive)
        }

        CommandGroup(before: .sidebar) {
            Button(String(localized: "Focus Quick Add")) { state.requestQuickAddFocus() }
                .keyboardShortcut(shortcut(.focusQuickAdd))
            Button(String(localized: "Search")) { state.go(to: .search) }
                .keyboardShortcut(shortcut(.search))
            Divider()
            ForEach(Array(state.orderedViews.prefix(9).enumerated()), id: \.offset) { i, v in
                Button(viewName(v)) { state.go(to: v) }
                    .keyboardShortcut(shortcut(.sidebar(i + 1)))
            }
            Divider()
            Button(String(localized: "Next View")) { state.stepView(1) }
                .keyboardShortcut(shortcut(.nextView))
            Button(String(localized: "Previous View")) { state.stepView(-1) }
                .keyboardShortcut(shortcut(.previousView))
            Divider()
            SortMenu(state: state)
            Divider()
            Button(String(localized: "Sync Now")) { state.sync() }
                .keyboardShortcut(shortcut(.sync))
                .disabled(!state.syncAvailable)
            Button(String(localized: "Sync Now")) { state.sync() }
                .keyboardShortcut(shortcut(.syncFunction))
                .disabled(!state.syncAvailable)
            Button(String(localized: "Nearby Devices…")) { state.showDevices() }
        }

        CommandGroup(replacing: .help) {
            Button(String(localized: "Keyboard Shortcuts")) { state.sheet = .shortcuts }
                .keyboardShortcut(shortcut(.shortcuts))
            Link(String(localized: "Momentum Help"), destination: URL(string: "https://github.com/dan-hart/momentum#readme")!)
        }
    }

    private var doneTitle: String {
        if let id = state.target, state.row(id)?.isDone == true {
            return String(localized: "Mark as Not Done")
        }
        return String(localized: "Mark as Done")
    }
    private var todayTitle: String {
        if let id = state.target, state.engine.taskMenu(id: id)?.plannedToday == true {
            return String(localized: "Remove from Today")
        }
        return String(localized: "Plan for Today")
    }
    private func slotTitle(_ slot: Slot) -> String {
        if let id = state.target, state.engine.taskMenu(id: id)?.slot == slot {
            return String(localized: "Move to Today")
        }
        return slot == .morning ? String(localized: "Move to Morning") : String(localized: "Move to Tonight")
    }
    private var repeatTitle: String {
        if let id = state.target, state.engine.taskMenu(id: id)?.repeats == true {
            return String(localized: "Edit Repeat…")
        }
        return String(localized: "Repeat…")
    }
    private func viewName(_ v: MomentumCore.View) -> String {
        switch v {
        case .project, .tag: return Strings.viewTitle(state.engine.viewTitle(view: v))
        default: return Strings.sidebarTitle(v)
        }
    }

    private func importBackup() {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [.json]
        panel.title = String(localized: "Import Super Productivity backup")
        if panel.runModal() == .OK, let url = panel.url {
            state.importBackup(url)
        }
    }
    private func exportBackup() {
        let panel = NSSavePanel()
        panel.allowedContentTypes = [.json]
        panel.nameFieldStringValue = "\(today()).json"
        panel.title = String(localized: "Export backup")
        if panel.runModal() == .OK, let url = panel.url {
            state.exportBackup(url)
        }
    }
}

/// Sort By, Order and Coming Up, bound to the defaults; used by the View menu and the
/// toolbar's View Options.
struct SortMenu: SwiftUI.View {
    let state: AppState
    @AppStorage(PrefKey.groupBy) private var group = "morning-night"
    @AppStorage(PrefKey.taskSort) private var sort = "manual"
    @AppStorage(PrefKey.sortDirection) private var direction = "ascending"
    @AppStorage(PrefKey.upcomingRange) private var range = "7"

    var body: some SwiftUI.View {
        Picker(String(localized: "Group By"), selection: $group) {
            Text(String(localized: "Morning & Night")).tag("morning-night")
            Text(String(localized: "None")).tag("none")
            Text(String(localized: "Project")).tag("project")
            Text(String(localized: "Tag")).tag("tag")
            Text(String(localized: "Time Estimate")).tag("estimate")
        }
        Picker(String(localized: "Sort By"), selection: $sort) {
            Text(String(localized: "Manual Order")).tag("manual")
            Text(String(localized: "Title")).tag("title")
            Text(String(localized: "Due Day")).tag("due")
            Text(String(localized: "Estimate")).tag("estimate")
            Text(String(localized: "Created")).tag("created")
        }
        Picker(String(localized: "Order"), selection: $direction) {
            Text(String(localized: "Ascending")).tag("ascending")
            Text(String(localized: "Descending")).tag("descending")
        }
        Picker(String(localized: "Coming Up"), selection: $range) {
            Text(String(localized: "Next 7 Days")).tag("7")
            Text(String(localized: "Next 30 Days")).tag("30")
        }
    }
}

private struct SelectAllTasksKey: FocusedValueKey {
    typealias Value = () -> Void
}

extension FocusedValues {
    var selectAllTasks: (() -> Void)? {
        get { self[SelectAllTasksKey.self] }
        set { self[SelectAllTasksKey.self] = newValue }
    }
}
