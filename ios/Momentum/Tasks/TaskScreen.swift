// SPDX-License-Identifier: GPL-3.0-or-later
import CoreTransferable
import MomentumCore
import MomentumKit
import MomentumMobile
import SwiftUI
import UIKit

enum TaskSnapshotTransition: Equatable {
    case immediate
    case smooth(duration: TimeInterval)

    static func decision(hasSnapshot: Bool, reduceMotion: Bool, isSearch: Bool) -> Self {
        hasSnapshot && !reduceMotion && !isSearch ? .smooth(duration: 0.2) : .immediate
    }

    var transaction: Transaction {
        let animation: Animation?
        switch self {
        case .immediate: animation = nil
        case .smooth(let duration): animation = .smooth(duration: duration)
        }
        var transaction = Transaction(animation: animation)
        transaction.disablesAnimations = animation == nil
        return transaction
    }
}

enum TaskSelectionTransitionFeedback: Equatable {
    case selection
    case coveredBySuccess
    case silent

    var emitsSelectionFeedback: Bool { self == .selection }

    static func afterOrganization(_ command: OrganizationCommand, outcome: Outcome?) -> Self {
        guard outcome?.changed == true else { return .silent }
        return command.emitsMovementSuccessFeedback ? .coveredBySuccess : .selection
    }

    static func afterTaskCommand(_ command: TaskCommand, outcome: Outcome?) -> Self {
        switch TaskCommandFeedback.after(command, outcome: outcome) {
        case .completionSuccess: return .coveredBySuccess
        case .selectionExit: return .selection
        case .silent: return .silent
        }
    }
}

typealias TaskSnapshotLoader = @MainActor (
    EngineWorker, MomentumCore.View, String, UInt32
) async -> TaskSnapshot

enum TaskLoadingPresentation: Equatable {
    case progress
    case todaySkeleton
}

struct TaskScreen: SwiftUI.View {
    @Environment(MobileAppModel.self) private var model
    @Environment(NextcloudSyncState.self) private var sync
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.colorSchemeContrast) private var contrast
    @Environment(\.accessibilityDifferentiateWithoutColor) private var differentiate
    @Environment(\.dynamicTypeSize) private var textSize
    @Environment(\.dismiss) private var dismiss
    @Environment(\.scenePhase) private var scenePhase
    @AppStorage(PrefKey.colorfulLabels) private var colorful = true
    @AppStorage(PrefKey.modifierKey) private var modifierRaw = ModifierKey.command.rawValue
    let view: MomentumCore.View
    var dismissesWhenViewDisappears = true
    var loadingPresentation = TaskLoadingPresentation.progress
    var onSnapshotChange: (TaskSnapshot) -> Void = { _ in }
    var snapshotLoader: TaskSnapshotLoader = {
        worker, view, query, archiveLimit in
        await worker.snapshot(view: view, query: query, archiveLimit: archiveLimit)
    }
    var onInitialSnapshotReady: @MainActor () -> Void = {}
    var showsSidebarButton = false
    var onSnapshotAssignment: @MainActor (TaskSnapshotTransition, Transaction) -> Void = { _, _ in }
    @State private var snapshot: TaskSnapshot?
    @State private var query = ""
    @State private var archiveLimit: UInt32 = 100
    @State private var selection: Set<String> = []
    @State private var editMode = EditMode.inactive
    @State private var taskSheet: TaskSheet?
    @State private var addingTagTo: [String] = []
    @State private var showingNewTag = false
    @State private var newTag = ""
    @State private var isVisible = false
    @State private var selectedTaskMenu: TaskMenuInfo?
    @State private var reportedInitialSnapshotReady = false
    private struct MenuRequest: Equatable { let id: String; let revision: Int }
    private var menuRequest: MenuRequest? {
        guard isVisible, !searchFocused, taskSheet == nil, model.activeTaskEditors == 0 else { return nil }
        let ids = snapshot?.tasks.filter { !$0.archived && selection.contains($0.id) }.map(\.id) ?? []
        return ids.count == 1 ? MenuRequest(id: ids[0], revision: model.revision) : nil
    }
    @FocusState private var searchFocused: Bool
    private enum TaskSheet: Identifiable {
        case edit(String), repeatSchedule(String), moveToProject([String])
        var id: String {
            switch self {
            case .edit(let id): "edit-" + id
            case .repeatSchedule(let id): "repeat-" + id
            case .moveToProject(let ids): "move-" + ids.joined(separator: ",")
            }
        }
    }


    private var refreshKey: String { "\(view)-\(model.worker != nil)-\(model.revision)-\(query)-\(archiveLimit)" }
    private var title: String {
        view == .upcoming ? String(localized: "Upcoming")
            : snapshot.map { Strings.viewTitle($0.listing.title) } ?? Strings.sidebarTitle(view)
    }

    var body: some SwiftUI.View {
        Group {
            if let snapshot {
                List(selection: $selection) {
                    if let empty = snapshot.listing.empty {
                        let copy = Strings.empty(empty, modifier: "⌘", syncConfigured: false)
                        ContentUnavailableView {
                            Label {
                                Text(copy.title)
                                    .lineLimit(nil)
                                    .fixedSize(horizontal: false, vertical: true)
                            } icon: {
                                Image(systemName: copy.symbol)
                            }
                        } description: {
                            Text(emptyDescription).foregroundStyle(AccentTheme.secondaryText)
                        }
                        .listRowBackground(Color.clear)
                    }
                    if let allDone = snapshot.listing.allDone {
                        let copy = Strings.allDone(allDone)
                        VStack(spacing: 12) {
                            Image(systemName: "checkmark.circle.fill")
                                .font(.largeTitle).foregroundStyle(.tint).accessibilityHidden(true)
                            Text(copy.title).font(.title2.bold())
                            Text(copy.description).foregroundStyle(AccentTheme.secondaryText)
                            Button("Archive Completed") { act(.archive) }
                        }
                        .frame(maxWidth: .infinity).padding(.vertical)
                    }
                    ForEach(Array(snapshot.listing.sections.enumerated()), id: \.offset) { _, section in
                        Section {
                            ForEach(section.rows, id: \.mobileID) { row in
                                switch row {
                                case .task(let task):
                                    taskRow(task, snapshot: snapshot)
                                        .tag(task.id)
                                        .selectionDisabled(task.archived)
                                case .project(let project):
                                    NavigationLink(project.title) { TaskScreen(view: .project(id: project.id)) }
                                case .tag(let tag):
                                    NavigationLink("#" + tag.title) { TaskScreen(view: .tag(id: tag.id)) }
                                }
                            }
                            if let note = section.note {
                                Text(Strings.sectionNote(note)).font(.footnote).foregroundStyle(AccentTheme.secondaryText)
                            }
                        } header: {
                            if let title = Strings.sectionTitle(section) {
                                Text(title).foregroundStyle(sectionColor(section.group))
                            }
                        }
                    }
                    if snapshot.listing.moreAvailable > 0 {
                        Button("Show More") { archiveLimit += 100 }
                    }
                    if sync.provider != .off {
                        SyncStatusLink(state: sync)
                            .listRowBackground(Color.clear)
                            .selectionDisabled(true)
                    }
                }
                .listStyle(.insetGrouped)
                .contentMargins(.bottom, 76, for: .scrollContent)
                .dropDestination(for: ExternalTaskText.self) { items, _ in
                    guard view != .archive else { return false }
                    // A task row also exports its ids as plain text for other apps; a row
                    // released on the list background must not become a task named by its id.
                    let known = Set(snapshot.tasks.map(\.id))
                    let external = items.filter { item in
                        !item.text.split(separator: "\n").allSatisfy { known.contains(String($0)) }
                    }
                    Task {
                        for item in external { await model.organize(.importText(item.text, view)) }
                    }
                    return !external.isEmpty
                }
                .environment(\.editMode, $editMode)
            } else {
                loadingView
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .modifier(TaskKeyboardPresentation(
            capabilities: model.platformCapabilities,
            view: view,
            commands: keyboardCommands,
            modifierRaw: modifierRaw
        ))
        .onAppear { isVisible = true }
        .onDisappear { isVisible = false }
        .navigationTitle(title)
        .navigationBarBackButtonHidden(showsSidebarButton)
        .momentumNavigationCanvas()
        .overlay(alignment: .bottomTrailing) {
            bottomTaskControls
                .padding(.horizontal, 20)
                .padding(.bottom, 12)
        }
        .sheet(item: $taskSheet, onDismiss: model.presentPendingNotification) { sheet in
            taskSheetContent(sheet)
        }
        .alert("Add Tag", isPresented: $showingNewTag) {
            TextField("Tag name", text: $newTag)
            Button("Cancel", role: .cancel) {}
            Button("Add") {
                Task { @MainActor in
                    let command = OrganizationCommand.tag(addingTagTo, newTag)
                    let outcome = await model.organize(command)
                    transitionSelection(
                        to: [],
                        editing: false,
                        feedback: .afterOrganization(command, outcome: outcome)
                    )
                }
            }.disabled(newTag.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
        }
        .modifier(SearchPresentation(enabled: view == .search, query: $query, focused: $searchFocused))
        .toolbar {
            if showsSidebarButton {
                ToolbarItem(placement: .topBarLeading) {
                    Button {
                        dismiss()
                    } label: {
                        Label("Show lists", systemImage: "sidebar.leading")
                    }
                    .accessibilityIdentifier("task-workspace-show-sidebar")
                }
            }
            ToolbarItemGroup(placement: .topBarTrailing) {
                if view != .archive {
                    Menu {
                        Button(editMode.isEditing ? "Done Selecting" : "Select Tasks") {
                            let wasEditing = editMode.isEditing
                            transitionSelection(to: [], editing: !wasEditing, feedback: .selection)
                        }
                        Button("Select All") {
                            transitionSelection(
                                to: Set(snapshot?.tasks.filter { !$0.archived }.map(\.id) ?? []),
                                editing: true,
                                feedback: .selection
                            )
                        }
                        Button("Archive Completed") { act(.archive) }
                            .disabled(snapshot?.listing.canArchive != true)
                    } label: { Label("List options", systemImage: "ellipsis") }
                }
                if snapshot?.canUndo == true {
                    Button { act(.undo) } label: { Label("Undo", systemImage: "arrow.uturn.backward") }
                }
            }
        }
        .task(id: menuRequest) {
            selectedTaskMenu = nil
            guard let request = menuRequest, let worker = model.worker else { return }
            let menu = await worker.taskMenu(request.id)
            guard !Task.isCancelled, menuRequest == request else { return }
            selectedTaskMenu = menu
        }
        .task(id: refreshKey) {
            guard let worker = model.worker else { return }
            if view == .search {
                do { try await Task.sleep(for: .milliseconds(120)) } catch { return }
            }
            let result = await snapshotLoader(worker, view, query, archiveLimit)
            guard !Task.isCancelled else { return }
            onSnapshotChange(result)
            selection.formIntersection(result.tasks.filter { !$0.archived }.map(\.id))
            let transition = TaskSnapshotTransition.decision(
                hasSnapshot: snapshot != nil,
                reduceMotion: reduceMotion,
                isSearch: view == .search
            )
            let startupTransition: TaskSnapshotTransition = if snapshot == nil,
                                                               loadingPresentation == .todaySkeleton,
                                                               !reduceMotion {
                .smooth(duration: StartupSkeletonMotionPolicy.standardFadeDuration)
            } else {
                transition
            }
            let transaction = startupTransition.transaction
            onSnapshotAssignment(startupTransition, transaction)
            withTransaction(transaction) { snapshot = result }
            guard result.viewExists else {
                if dismissesWhenViewDisappears { dismiss() }
                return
            }
            if view == .today, !reportedInitialSnapshotReady {
                reportedInitialSnapshotReady = true
                onInitialSnapshotReady()
            }
        }
    }

    @ViewBuilder private var loadingView: some SwiftUI.View {
        switch loadingPresentation {
        case .progress:
            ProgressView("Loading tasks")
        case .todaySkeleton:
            TodaySkeletonView(motion: StartupSkeletonMotionPolicy(
                isComplete: false,
                reduceMotion: reduceMotion,
                lowPowerMode: model.lowPowerMode,
                sceneIsActive: scenePhase == .active
            ))
        }
    }

    @ViewBuilder private func taskSheetContent(_ sheet: TaskSheet) -> some SwiftUI.View {
        if let worker = model.worker {
            switch sheet {
            case .edit(let id):
                TaskEditor(worker: worker, view: view, taskID: id) { model.refreshAfterEdit() }
            case .repeatSchedule(let id):
                RepeatEditor(worker: worker, taskID: id) { model.refreshAfterEdit() }
                    .onAppear { model.activeTaskEditors += 1 }
                    .onDisappear { model.activeTaskEditors = max(0, model.activeTaskEditors - 1) }
            case .moveToProject(let ids):
                MoveToProjectSheet(ids: ids, projects: snapshot?.projects ?? []) {
                    transitionSelection(to: [], editing: false, feedback: .coveredBySuccess)
                }
                    .onAppear { model.activeTaskEditors += 1 }
                    .onDisappear { model.activeTaskEditors = max(0, model.activeTaskEditors - 1) }
            }
        }
    }

    @ViewBuilder private var bottomTaskControls: some SwiftUI.View {
        let motion = InteractionMotionPolicy.presentation(reduceMotion: reduceMotion)
        VStack(alignment: .trailing, spacing: 12) {
            if !selection.isEmpty, let snapshot {
                TaskDropDestinations(
                    selectedIDs: snapshot.tasks.filter { selection.contains($0.id) }.map(\.id),
                    projects: snapshot.projects,
                    tags: snapshot.tags,
                    move: moveSelection
                )
                .transition(motion.transition)
            }
            if textSize.isAccessibilitySize {
                VStack(alignment: .trailing, spacing: 12) {
                    if !selection.isEmpty {
                        selectionActions.frame(maxWidth: .infinity, alignment: .leading)
                    }
                    if view != .archive { addTaskButton }
                }
            } else {
                HStack(spacing: 12) {
                    if !selection.isEmpty { selectionActions }
                    Spacer(minLength: 0)
                    if view != .archive { addTaskButton }
                }
            }
        }
        .animation(motion.animation, value: selection.isEmpty)
    }

    private var addTaskButton: some SwiftUI.View {
        FloatingAddTaskButton { model.add(in: view) }
            .disabled(model.worker == nil)
    }

    private var selectionActions: some SwiftUI.View {
        Menu {
            TaskActionsMenu(ids: snapshot?.tasks.filter { selection.contains($0.id) }.map(\.id) ?? [],
                            projects: snapshot?.projects ?? [], tags: snapshot?.tags ?? [],
                            onNewTag: { requestTag(selection.sorted()) },
                            onFinished: {
                                transitionSelection(to: [], editing: false, feedback: $0)
                            })
        } label: {
            Label("Actions (\(selection.count))", systemImage: "ellipsis.circle")
                .font(.headline)
                .foregroundStyle(.primary)
                .fixedSize(horizontal: false, vertical: true)
                .frame(minHeight: 32)
        }
        .buttonStyle(.bordered)
        .buttonBorderShape(.capsule)
        .controlSize(.large)
        .tint(.primary)
        .accessibilityIdentifier("task-selection-actions")
    }

    @ViewBuilder private func taskRow(_ task: TaskRow, snapshot: TaskSnapshot) -> some SwiftUI.View {
        let parentTitle = task.parentId.flatMap { snapshot.parentTitles[$0] }
        if task.archived {
            TaskRowContent(task: task, parentTitle: parentTitle, toggle: {}, open: {})
        } else {
            let transfer = TaskTransfer(ids: selection.contains(task.id)
                ? snapshot.tasks.filter { selection.contains($0.id) }.map(\.id) : [task.id])
            let content = TaskRowContent(
                task: task, parentTitle: parentTitle,
                presentation: editMode.isEditing ? .selection : .browsing,
                toggle: { act(.complete([task.id], !task.isDone)) },
                open: { taskSheet = .edit(task.id) }
            )
            Group {
                if editMode.isEditing {
                    HStack(spacing: 0) {
                        content
                        TaskDragHandle(transfer: transfer,
                                       label: String(localized: "Reorder \(task.title)"))
                            .frame(minWidth: 44, minHeight: 44)
                    }
                } else {
                    content.draggable(transfer)
                }
            }
                .dropDestination(for: TaskTransfer.self) { transfers, _ in
                    Task { await model.organize(.reorder(transfers.flatMap(\.ids), before: task.id, view: view)) }
                    return true
                }
                .contextMenu {
                    if !editMode.isEditing {
                        TaskActionsMenu(ids: [task.id], row: task, projects: snapshot.projects,
                                        tags: snapshot.tags,
                                        onEdit: { taskSheet = .edit(task.id) },
                                        onNewTag: { requestTag([task.id]) })
                    }
                }
                .swipeActions(edge: .trailing) {
                    Button("Delete", systemImage: "trash", role: .destructive) { act(.delete([task.id])) }
                }
                .swipeActions(edge: .leading) {
                    Button(task.isDone ? "Mark as Not Done" : "Mark as Done",
                           systemImage: task.isDone ? "arrow.uturn.backward.circle" : "checkmark") {
                        act(.complete([task.id], !task.isDone))
                    }.tint(Color.accentColor)
                    Button("Open", systemImage: "square.and.pencil") { taskSheet = .edit(task.id) }
                        .tint(.secondary)
                }
                .accessibilityAction(named: "Move Up") { Task { await model.organize(.nudge(task.id, -1, view)) } }
                .accessibilityAction(named: "Move Down") { Task { await model.organize(.nudge(task.id, 1, view)) } }
        }
    }

    private func requestTag(_ ids: [String]) {
        addingTagTo = ids
        newTag = ""
        showingNewTag = true
    }

    private var emptyDescription: String {
        switch view {
        case .search: String(localized: "Search tasks, projects, tags, and notes.")
        case .archive: String(localized: "Completed tasks appear here after you archive them.")
        case .today: String(localized: "Add a task or bring one into Today from your lists.")
        case .upcoming: String(localized: "Tasks planned for future dates appear here.")
        default: String(localized: "Add a task to this list, or move one here.")
        }
    }

    private func sectionColor(_ group: TaskGroup?) -> Color {
        guard colorful, contrast != .increased, !differentiate,
              let color = group?.savedColor else { return AccentTheme.secondaryText }
        return AccentTheme.ink(hex: color)
    }

    private func act(_ command: TaskCommand) {
        Task { await model.perform(command) }
    }

    private func organize(_ command: OrganizationCommand) {
        Task { await model.organize(command) }
    }

    private func moveSelection(_ ids: [String], to destination: MomentumCore.View) {
        Task { @MainActor in
            guard await model.organize(.drop(ids, destination))?.changed == true else { return }
            transitionSelection(to: [], editing: false, feedback: .coveredBySuccess)
        }
    }

    private func transitionSelection(to newSelection: Set<String>, editing: Bool,
                                     feedback: TaskSelectionTransitionFeedback) {
        let wasEditing = editMode.isEditing
        selection = newSelection
        editMode = editing ? .active : .inactive
        if wasEditing != editing, feedback.emitsSelectionFeedback {
            model.selectionModeDidChange(from: wasEditing, to: editing)
        }
    }

    private func performSelectionAction(_ command: TaskCommand) {
        Task { @MainActor in
            let outcome = await model.perform(command)
            transitionSelection(to: [], editing: false,
                                feedback: .afterTaskCommand(command, outcome: outcome))
        }
    }

    private var keyboardCommands: MobileTaskCommandContext? {
        guard isVisible, !searchFocused, taskSheet == nil, !showingNewTag,
              !model.showingAdd, !model.showingNewProject, model.activeTaskEditors == 0, model.notificationTask == nil,
              !model.showingKeyboardHelp, !model.isRestoringBackup, let snapshot else { return nil }
        let tasks = snapshot.tasks.filter { !$0.archived }
        let selected = tasks.filter { selection.contains($0.id) }
        let topLevel = selected.filter { !$0.isSubtask }
        let reopens = selected.count == 1 && selected[0].isDone
        var enabled: Set<MobileTaskShortcut> = []
        if !tasks.isEmpty { enabled.insert(.selectAll) }
        if !selected.isEmpty { enabled.formUnion([.deselectAll, .markDone, .deleteTask, .planToday, .morning, .tonight, .tomorrow, .nextWeek]) }
        if selected.count == 1 {
            enabled.formUnion([.openTask, .duplicateTask, .copyTitle, .moveUp, .moveDown])
            if !topLevel.isEmpty { enabled.insert(.repeatSchedule) }
        }
        if !topLevel.isEmpty { enabled.insert(.moveToProject) }
        if snapshot.listing.canArchive { enabled.insert(.archive) }
        if snapshot.canUndo { enabled.insert(.undo) }
        return MobileTaskCommandContext(view: view, taskIDs: tasks.map(\.id), selectedIDs: selected.map(\.id),
                                        selectedTitle: selected.count == 1 ? selected[0].title : nil,
                                        singleTaskMenu: selected.count == 1 && selectedTaskMenu?.id == selected[0].id ? selectedTaskMenu : nil,
                                        enabled: enabled, reopensTask: reopens) { action in
            switch action {
            case .selectAll:
                transitionSelection(to: Set(tasks.map(\.id)), editing: true, feedback: .selection)
            case .deselectAll:
                transitionSelection(to: [], editing: false, feedback: .selection)
            case .openTask:
                if let task = selected.first { taskSheet = .edit(task.id) }
            case .duplicateTask:
                guard let task = selected.first else { return }
                Task {
                    guard let worker = model.worker else { return }
                    let result = await worker.duplicateTaskWithID(task.id)
                    model.accept(result.outcome)
                    if let newID = result.taskID { selection = [newID] }
                }
            case .copyTitle:
                guard let task = selected.first else { return }
                UIPasteboard.general.string = task.title
                model.showFeedback(String(localized: "Title copied"))
            case .planToday: organize(.toggleToday(selected.map(\.id)))
            case .morning: organize(.slot(selected.map(\.id), .morning))
            case .tonight: organize(.slot(selected.map(\.id), .tonight))
            case .tomorrow: organize(.tomorrow(selected.map(\.id)))
            case .nextWeek: organize(.nextWeek(selected.map(\.id)))
            case .moveToProject: taskSheet = .moveToProject(topLevel.map(\.id))
            case .repeatSchedule:
                if let task = topLevel.first { taskSheet = .repeatSchedule(task.id) }
            case .moveUp, .moveDown:
                if let task = selected.first { organize(.nudge(task.id, action == .moveUp ? -1 : 1, view)) }
            case .archive: act(.archive)
            case .markDone: performSelectionAction(.complete(selected.map(\.id), !reopens))
            case .deleteTask: performSelectionAction(.delete(selected.map(\.id)))
            case .undo: act(.undo)
            }
        }
    }
}

private struct TaskKeyboardPresentation: ViewModifier {
    let capabilities: MobilePlatformCapabilities
    let view: MomentumCore.View
    let commands: MobileTaskCommandContext?
    let modifierRaw: String

    @ViewBuilder func body(content: Content) -> some SwiftUI.View {
        if capabilities.showsKeyboardShortcuts {
            content
                .focusedSceneValue(\.mobileTaskView, view)
                .focusedSceneValue(\.mobileTaskCommands, commands)
                .background {
                    TaskSelectionKeyResponder(enabled: commands?.enabled.contains(.selectAll) == true) {
                        commands?.perform(.selectAll)
                    }
                    .frame(width: 0, height: 0)
                    .accessibilityHidden(true)
                }
                .onKeyPress(keys: [KeyEquivalent("a"), .delete], phases: .down) { press in
                    MobileKeyPressFallback.task(
                        key: press.key, modifiers: press.modifiers,
                        configuredModifier: ModifierKey(rawValue: modifierRaw) ?? .command,
                        context: commands,
                        capabilities: capabilities
                    )
                }
        } else {
            content
        }
    }
}

private extension TaskGroup {
    var savedColor: String? {
        switch self {
        case .project(_, _, let color), .tag(_, _, let color): color
        default: nil
        }
    }
}

extension Row {
    var mobileID: String {
        switch self {
        case .task(let task): "task:" + task.id
        case .project(let project): "project:" + project.id
        case .tag(let tag): "tag:" + tag.id
        }
    }
}

enum TaskRowPresentation {
    case browsing
    case selection
}

struct TaskRowContent: SwiftUI.View {
    let task: TaskRow
    var parentTitle: String? = nil
    var presentation: TaskRowPresentation = .browsing
    let toggle: () -> Void
    let open: () -> Void
    @AppStorage(PrefKey.colorfulLabels) private var colorful = true
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.colorSchemeContrast) private var contrast
    @Environment(\.accessibilityDifferentiateWithoutColor) private var differentiate

    var body: some SwiftUI.View {
        HStack(spacing: 10) {
            if !task.archived, presentation == .browsing {
                Button(action: toggle) {
                    Image(systemName: task.isDone ? "checkmark.circle.fill" : "circle")
                        .font(.title2).frame(minWidth: 44, minHeight: 44)
                        .symbolRenderingMode(.hierarchical)
                        .contentTransition(.symbolEffect(.replace, options: .nonRepeating))
                        .symbolEffect(.bounce, options: .nonRepeating, value: reduceMotion ? false : task.isDone)
                        .transaction { if reduceMotion { $0.animation = nil; $0.disablesAnimations = true } }
                }
                .buttonStyle(.borderless)
                .accessibilityLabel(actionLabel(task.isDone ? Text("Reopen \(task.title)") : Text("Complete \(task.title)")))
            } else if task.archived {
                Image(systemName: "checkmark.circle").accessibilityHidden(true)
            }
            if task.archived {
                details
                    .accessibilityElement(children: .combine)
                    .accessibilityLabel(actionLabel(Text(task.title)))
                    .accessibilityValue(accessibleDetails)
            } else {
                Button(action: open) {
                    details
                        .frame(minHeight: 44)
                        .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .accessibilityLabel(actionLabel(Text("Open \(task.title)")))
                .accessibilityValue(accessibleDetails)
            }
        }
        .padding(.leading, task.isSubtask ? 20 : 0)
        .padding(.vertical, 3)
    }

    private var details: some SwiftUI.View {
        HStack(spacing: 10) {
            VStack(alignment: .leading, spacing: 4) {
                Text(task.title).mobileFont(content: true)
                    .strikethrough(task.isDone && !task.archived)
                    .foregroundStyle(task.isDone || task.archived ? AccentTheme.secondaryText : Color.primary)
                let parts = TaskSubtitle.parts(for: task)
                if !parts.isEmpty {
                    parts.enumerated().reduce(Text("")) { result, item in
                        let color = colorful && contrast != .increased && !differentiate
                            ? item.element.color.map(AccentTheme.ink(hex:)) ?? AccentTheme.secondaryText : AccentTheme.secondaryText
                        let part = Text(verbatim: (item.offset == 0 ? "" : " · ") + item.element.text)
                            .foregroundColor(color)
                        return Text("\(result)\(part)")
                    }.mobileFont(content: true, caption: true)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            if task.notesPreview != nil {
                Image(systemName: "note.text").foregroundStyle(AccentTheme.secondaryText)
                    .accessibilityLabel("Has notes")
            }
            if let reminder = task.reminder {
                Image(systemName: "bell.badge").foregroundStyle(AccentTheme.secondaryText)
                    .accessibilityLabel("Reminder at \(Strings.time(reminder))")
            }
        }
    }

    private func actionLabel(_ action: Text) -> Text {
        guard task.isSubtask else { return action }
        if let parentTitle { return Text("\(action), subtask of \(parentTitle)") }
        // Preserve the subtask relationship even if its parent record is unavailable.
        return Text("\(action), subtask")
    }

    // The explicit Open label names the action; retain its visual metadata as
    // a single spoken value rather than separate, undersized row elements.
    private var accessibleDetails: Text {
        var values = TaskSubtitle.parts(for: task).map { Text($0.text) }
        if task.notesPreview != nil { values.append(Text("Has notes")) }
        if let reminder = task.reminder { values.append(Text("Reminder at \(Strings.time(reminder))")) }
        return values.enumerated().reduce(Text("")) { result, item in
            item.offset == 0 ? item.element : Text("\(result), \(item.element)")
        }
    }
}

private struct SearchPresentation: ViewModifier {
    @Environment(MobileAppModel.self) private var model
    let enabled: Bool
    @Binding var query: String
    var focused: FocusState<Bool>.Binding
    func body(content: Content) -> some SwiftUI.View {
        if enabled {
            content.searchable(text: $query, placement: .navigationBarDrawer(displayMode: .always), prompt: "Search tasks and notes")
                .searchFocused(focused)
                .task(id: model.searchFocusRevision) {
                    if model.searchFocusRevision > 0 { focused.wrappedValue = true }
                }
                .task(id: model.systemSearchRequest) {
                    guard let request = model.systemSearchRequest else { return }
                    query = request.title
                    focused.wrappedValue = true
                    model.consumeSystemSearchRequest()
                }
        } else { content }
    }
}

/// Native text and URL transfers feed the existing Rust paste classifier.
struct ExternalTaskText: Transferable {
    let text: String
    static var transferRepresentation: some TransferRepresentation {
        ProxyRepresentation(importing: { (text: String) in Self(text: text) })
        ProxyRepresentation(importing: { (url: URL) in Self(text: url.absoluteString) })
    }
}
