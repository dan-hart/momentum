// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore
import MomentumKit
import MomentumMobile
import SwiftUI
import UIKit

/// Native scene commands and in-app help use the same desktop binding definitions.
enum MobileGlobalShortcut: String, CaseIterable, Identifiable {
    case newTask, newProject, search, today, upcoming, searchTab, settingsTab
    case nextTab, previousTab, settings, sync, help
    var id: String { rawValue }

    var shortcut: MomentumKit.AppShortcut {
        switch self {
        case .newTask: .newTask
        case .newProject: .newProject
        case .search: .search
        case .today: .sidebar(1)
        case .upcoming: .sidebar(2)
        case .searchTab: .sidebar(3)
        case .settingsTab: .sidebar(4)
        case .nextTab: .nextView
        case .previousTab: .previousView
        case .settings: .settings
        case .sync: .sync
        case .help: .shortcuts
        }
    }

    var title: LocalizedStringKey {
        switch self {
        case .newTask: "New Task"
        case .newProject: "New Project"
        case .search, .searchTab: "Search"
        case .today: "Today"
        case .upcoming: "Upcoming"
        case .settings, .settingsTab: "Settings"
        case .nextTab: "Next Tab"
        case .previousTab: "Previous Tab"
        case .sync: "Sync Now"
        case .help: "Keyboard Shortcuts"
        }
    }
}

private struct MobileTaskViewKey: FocusedValueKey {
    typealias Value = MomentumCore.View
}

enum MobileTaskShortcut: String, CaseIterable, Identifiable {
    case selectAll, deselectAll, openTask, markDone, duplicateTask, copyTitle, deleteTask, archive, undo
    case planToday, morning, tonight, tomorrow, nextWeek
    case moveToProject, repeatSchedule, moveUp, moveDown
    var id: String { rawValue }
    var shortcut: MomentumKit.AppShortcut {
        switch self {
        case .selectAll: .selectAll
        case .deselectAll: .deselectAll
        case .openTask: .openTask
        case .markDone: .markDone
        case .duplicateTask: .duplicateTask
        case .copyTitle: .copyTitle
        case .archive: .archive
        case .planToday: .planToday
        case .morning: .morning
        case .tonight: .tonight
        case .tomorrow: .tomorrow
        case .nextWeek: .nextWeek
        case .moveToProject: .moveToProject
        case .repeatSchedule: .repeatSchedule
        case .moveUp: .moveUp
        case .moveDown: .moveDown
        case .deleteTask: .deleteTask
        case .undo: .undo
        }
    }
    var title: LocalizedStringKey {
        switch self {
        case .selectAll: "Select All"
        case .deselectAll: "Deselect All"
        case .openTask: "Open Task"
        case .markDone: "Mark as Done"
        case .duplicateTask: "Duplicate Task"
        case .copyTitle: "Copy Title"
        case .archive: "Archive Completed"
        case .planToday: "Plan for Today"
        case .morning: "Morning"
        case .tonight: "Evening"
        case .tomorrow: "Tomorrow"
        case .nextWeek: "Next Week"
        case .moveToProject: "Move to Project"
        case .repeatSchedule: "Repeat"
        case .moveUp: "Move Up"
        case .moveDown: "Move Down"
        case .deleteTask: "Delete Task"
        case .undo: "Undo"
        }
    }
}

/// The visible list supplies actions; neither another tab nor a text field is a task target.
struct MobileTaskCommandContext: Equatable {
    let view: MomentumCore.View
    let taskIDs: [String]
    let selectedIDs: [String]
    let selectedTitle: String?
    let singleTaskMenu: TaskMenuInfo?
    let enabled: Set<MobileTaskShortcut>
    let reopensTask: Bool
    let perform: (MobileTaskShortcut) -> Void

    // Compare the action's inputs, not its newly created closure, so publishing the
    // scene value does not continually invalidate the list that supplies it.
    static func == (lhs: Self, rhs: Self) -> Bool {
        lhs.view == rhs.view && lhs.taskIDs == rhs.taskIDs && lhs.selectedIDs == rhs.selectedIDs
            && lhs.selectedTitle == rhs.selectedTitle && lhs.singleTaskMenu == rhs.singleTaskMenu && lhs.enabled == rhs.enabled && lhs.reopensTask == rhs.reopensTask
    }
}

private struct MobileTaskCommandsKey: FocusedValueKey {
    typealias Value = MobileTaskCommandContext
}

extension FocusedValues {
    var mobileTaskView: MomentumCore.View? {
        get { self[MobileTaskViewKey.self] }
        set { self[MobileTaskViewKey.self] = newValue }
    }
    var mobileTaskCommands: MobileTaskCommandContext? {
        get { self[MobileTaskCommandsKey.self] }
        set { self[MobileTaskCommandsKey.self] = newValue }
    }
}

/// Direct key-event fallback for the three commands that iOS 27 can omit from the
/// scene command responder on a cold launch. It uses the same action contexts and
/// availability gates as the menus; text inputs receive their events first and an
/// unavailable context returns `.ignored` to preserve native editing behavior.
@MainActor enum MobileKeyPressFallback {
    static func global(key: KeyEquivalent, modifiers: EventModifiers,
                       configuredModifier: ModifierKey,
                       model: MobileAppModel) -> KeyPress.Result {
        guard model.platformCapabilities.showsKeyboardShortcuts,
              key == KeyEquivalent("f"), modifiers == configuredModifier.modifiers,
              available(model) else { return .ignored }
        model.selectedTab = MobileTab.search.rawValue
        model.searchFocusRevision += 1
        return .handled
    }

    static func task(key: KeyEquivalent, modifiers: EventModifiers,
                     configuredModifier: ModifierKey,
                     context: MobileTaskCommandContext?,
                     capabilities: MobilePlatformCapabilities) -> KeyPress.Result {
        guard capabilities.showsKeyboardShortcuts, let context else { return .ignored }
        let action: MobileTaskShortcut
        if key == KeyEquivalent("a"), modifiers == .command {
            action = .selectAll
        } else if key == .delete, modifiers == configuredModifier.modifiers {
            action = .deleteTask
        } else {
            return .ignored
        }
        guard context.enabled.contains(action) else { return .ignored }
        context.perform(action)
        return .handled
    }

    static func available(_ model: MobileAppModel) -> Bool {
        model.worker != nil && !model.showingAdd && !model.showingNewProject
            && model.activeTaskEditors == 0 && model.notificationTask == nil
            && !model.showingKeyboardHelp && !model.isRestoringBackup
    }
}

struct MobileKeyboardCommands: Commands {
    let model: MobileAppModel
    private let sync: NextcloudSyncState
    @AppStorage private var modifierRaw: String
    @FocusedValue(\.mobileTaskView) private var taskView
    @FocusedValue(\.mobileTaskCommands) private var taskCommands

    init(model: MobileAppModel, sync: NextcloudSyncState? = nil) {
        self.model = model
        self.sync = sync ?? model.sync
        _modifierRaw = AppStorage(wrappedValue: "command", PrefKey.modifierKey, store: model.defaults)
    }

    private var modifier: ModifierKey { ModifierKey(rawValue: modifierRaw) ?? .command }
    private var available: Bool { MobileKeyPressFallback.available(model) }
    var includesMenus: Bool { model.platformCapabilities.showsKeyboardShortcuts }

    @CommandsBuilder
    var body: some Commands {
        if includesMenus {
            CommandGroup(replacing: .newItem) {
                command(.newTask)
                command(.newProject)
            }
            CommandMenu("Navigate") {
                ForEach([MobileGlobalShortcut.search, .today, .upcoming, .searchTab, .settingsTab, .nextTab, .previousTab]) {
                    command($0)
                }
            }
            // Keep the standard Settings shortcut in its native group. Placing it in
            // Navigate prevents that menu's shortcuts from dispatching on iOS.
            CommandGroup(replacing: .appSettings) { command(.settings) }
            // Keep these groups stable as focus changes. Replacing the native Undo
            // group avoids a duplicate Command-Z; text editing keeps its native group.
            CommandGroup(replacing: .undoRedo) { taskCommand(.undo) }
            CommandMenu("Task") {
                taskCommand(.deselectAll)
                Divider()
                taskCommand(.openTask)
                taskCommand(.markDone)
                taskCommand(.duplicateTask)
                taskCommand(.copyTitle)
                Divider()
                taskCommand(.planToday)
                taskCommand(.morning)
                taskCommand(.tonight)
                taskCommand(.tomorrow)
                taskCommand(.nextWeek)
                taskCommand(.moveToProject)
                taskCommand(.repeatSchedule)
                Divider()
                taskCommand(.moveUp)
                taskCommand(.moveDown)
                Divider()
                taskCommand(.archive)
                taskCommand(.deleteTask)
            }
            CommandMenu("Sync") { command(.sync) }
            CommandGroup(replacing: .help) { command(.help) }
        }
    }

    private func taskCommand(_ action: MobileTaskShortcut) -> some SwiftUI.View {
        let title: LocalizedStringKey
        switch action {
        case .markDone where taskCommands?.reopensTask == true: title = "Mark as Not Done"
        case .planToday where taskCommands?.singleTaskMenu?.plannedToday == true: title = "Remove from Today"
        case .morning where taskCommands?.singleTaskMenu?.slot == .morning: title = "Move to Today"
        case .tonight where taskCommands?.singleTaskMenu?.slot == .tonight: title = "Move to Today"
        case .repeatSchedule where taskCommands?.singleTaskMenu?.repeats == true: title = "Edit Repeat…"
        default: title = action.title
        }
        return Button(title, role: action == .deleteTask ? .destructive : nil) {
            guard available, let context = taskCommands, context.enabled.contains(action) else { return }
            context.perform(action)
        }
        .keyboardShortcut(action.shortcut.binding(modifier: modifier))
        .disabled(!available || taskCommands?.enabled.contains(action) != true)
    }

    private func command(_ action: MobileGlobalShortcut) -> some SwiftUI.View {
        Button(action.title) { perform(action) }
            .keyboardShortcut(action.shortcut.binding(modifier: modifier))
            .disabled(!available || (action == .sync && !sync.canSync))
    }

    private func perform(_ action: MobileGlobalShortcut) {
        guard available else { return }
        switch action {
        case .newTask:
            // A new task never belongs to the archive; create it in Today instead.
            let target = model.selectedTab < 3 ? taskView ?? .today : .today
            model.add(in: target == .archive ? .today : target)
        case .newProject: model.showingNewProject = true
        case .search:
            model.selectedTab = 2
            model.searchFocusRevision += 1
        case .today: model.selectedTab = 0
        case .upcoming: model.selectedTab = 1
        case .searchTab: model.selectedTab = 2
        case .settings, .settingsTab: model.selectedTab = 3
        case .nextTab: model.selectedTab = (model.selectedTab + 1) % 4
        case .previousTab: model.selectedTab = (model.selectedTab + 3) % 4
        case .sync: Task { await sync.syncNow() }
        case .help: model.showingKeyboardHelp = true
        }
    }
}

/// A list-scoped responder implements UIKit's standard Select All action. Native
/// text inputs take over the same system command when editing.
struct TaskSelectionKeyResponder: UIViewRepresentable {
    let enabled: Bool
    let selectAll: () -> Void

    func makeUIView(context: Context) -> SelectionResponder { SelectionResponder() }
    func updateUIView(_ view: SelectionResponder, context: Context) {
        view.selectTasks = selectAll
        let wasEnabled = view.enabled
        view.enabled = enabled
        if enabled && !wasEnabled && view.window != nil { view.becomeFirstResponder() }
        if !enabled && view.isFirstResponder { view.resignFirstResponder() }
    }
    static func dismantleUIView(_ view: SelectionResponder, coordinator: ()) {
        if view.isFirstResponder { view.resignFirstResponder() }
        view.selectTasks = {}
    }

    final class SelectionResponder: UIView {
        var enabled = false
        var selectTasks: () -> Void = {}
        override var canBecomeFirstResponder: Bool { enabled }
        override func didMoveToWindow() {
            super.didMoveToWindow()
            if window != nil && enabled { becomeFirstResponder() }
        }
        override func canPerformAction(_ action: Selector, withSender sender: Any?) -> Bool {
            if action == #selector(selectAll(_:)) { return enabled }
            return super.canPerformAction(action, withSender: sender)
        }
        override func selectAll(_ sender: Any?) { if enabled { selectTasks() } }
    }
}

struct KeyboardSettings: SwiftUI.View {
    @AppStorage(PrefKey.modifierKey) private var modifierRaw = "command"
    @Environment(\.dynamicTypeSize) private var textSize
    private var modifier: ModifierKey { ModifierKey(rawValue: modifierRaw) ?? .command }

    var body: some SwiftUI.View {
        Form {
            Section {
                Picker("Shortcut Modifier", selection: $modifierRaw) {
                    ForEach(ModifierKey.allCases) { Text($0.label).tag($0.rawValue) }
                }
                .accessibilityIdentifier("keyboard-modifier")
            } footer: {
                Text("App shortcuts use this modifier. Text editing and system shortcuts keep their standard keys.")
                    .foregroundStyle(AccentTheme.secondaryText)
            }
            Section {
                ForEach(MobileGlobalShortcut.allCases) { action in
                    shortcutRow(action.title, action.shortcut, id: action.rawValue)
                }
            }
            Section("Task") {
                ForEach(MobileTaskShortcut.allCases) { action in
                    shortcutRow(action.title, action.shortcut, id: action.rawValue)
                }
            }
        }
        .navigationTitle("Keyboard Shortcuts")
        .navigationBarTitleDisplayMode(.inline)
        .momentumNavigationCanvas()
    }

    private func shortcutRow(_ title: LocalizedStringKey, _ shortcut: AppShortcut, id: String) -> some SwiftUI.View {
        Group {
            if textSize.isAccessibilitySize {
                VStack(alignment: .leading, spacing: 4) {
                    Text(title)
                    Text(verbatim: shortcut.display(modifier: modifier)).monospaced()
                }
            } else {
                LabeledContent(title) {
                    Text(verbatim: shortcut.display(modifier: modifier)).monospaced()
                }
            }
        }
        .fixedSize(horizontal: false, vertical: true)
        .accessibilityElement(children: .combine)
        .accessibilityIdentifier("shortcut-" + id)
    }
}
