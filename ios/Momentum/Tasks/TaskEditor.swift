// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore
import MomentumKit
import MomentumMobile
import SwiftUI
import UIKit

/// Drafts persist on Save and before repeat/duplicate actions through EngineWorker.
struct TaskEditor: SwiftUI.View {
    let worker: EngineWorker
    let view: MomentumCore.View
    var taskID: String? = nil
    let onChange: @MainActor () -> Void

    @Environment(\.dismiss) private var dismiss
    @Environment(MobileAppModel.self) private var model
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var form = TaskFormModel()
    @State private var detail: TaskDetail?
    @State private var menu: TaskMenuInfo?
    @State private var projects: [ProjectRef] = []
    @State private var tags: [TagRef] = []
    @State private var loaded = false
    @State private var busy = false
    @State private var error: String?
    @State private var subtaskTitle = ""
    @State private var showRepeat = false
    @State private var confirmDelete = false
    private enum Field: Hashable { case title, estimate, tags, notes, subtask }
    @FocusState private var focusedField: Field?
    @AccessibilityFocusState private var errorFocused: Bool

    private var editable: Bool { taskID == nil || menu != nil }
    private var estimateInvalid: Bool { form.mobileEstimateInvalid }
    private var valid: Bool { form.mobileCanSave }

    var body: some SwiftUI.View {
        NavigationStack {
            Group {
                if loaded {
                    editorForm
                } else {
                    ProgressView(String(localized: "Loading task…"))
                }
            }
            .navigationTitle(taskID == nil ? String(localized: "New Task") : String(localized: "Edit Task"))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(String(localized: "Cancel"), systemImage: "xmark") { dismiss() }.disabled(busy)
                }
                ToolbarItem(placement: .confirmationAction) {
                    SheetCommitButton(title: taskID == nil ? String(localized: "Create") : String(localized: "Save"), symbol: taskID == nil ? "plus" : "checkmark") { save() }
                        .disabled(!loaded || !editable || !valid || busy)
                        .keyboardShortcut(.return, modifiers: .command)
                }
            }
            .safeAreaInset(edge: .bottom, spacing: 0) {
                if focusedField != nil {
                    HStack {
                        Spacer()
                        Button {
                            focusedField = nil
                        } label: {
                            Label(String(localized: "Done"), systemImage: "keyboard.chevron.compact.down")
                                .frame(minHeight: 44)
                        }
                        .buttonStyle(.bordered)
                        .accessibilityIdentifier("task-editor-dismiss-keyboard")
                    }
                    .frame(minHeight: 44)
                    .padding(.horizontal)
                    .background(.bar)
                }
            }
            .task { await load() }
            .onChange(of: error) { _, value in if value != nil { errorFocused = true } }
            .onAppear { model.activeTaskEditors += 1 }
            .onDisappear { model.activeTaskEditors = max(0, model.activeTaskEditors - 1) }
            .interactiveDismissDisabled(busy)
            .sheet(isPresented: $showRepeat) {
                if let taskID {
                    RepeatEditor(worker: worker, taskID: taskID) {
                        onChange()
                        Task { detail = await worker.taskDetail(taskID) }
                    }
                }
            }
            .confirmationDialog(String(localized: "Delete Task?"), isPresented: $confirmDelete,
                                titleVisibility: .visible) {
                Button(String(localized: "Delete Task"), role: .destructive) {
                    guard let taskID else { return }
                    mutate(closes: true) { await worker.deleteTask(taskID) }
                }
            }
        }
    }

    private var editorForm: some SwiftUI.View {
        Form {
            if let error {
                Section {
                    AccessibleErrorMessage(error)
                        .accessibilityFocused($errorFocused)
                        .accessibilityIdentifier("task-editor-error")
                }
            }
            if !editable {
                Section { Text(String(localized: "This task is archived or no longer available.")) }
            }
            Group {
                Section {
                    TextField(String(localized: "Title"), text: $form.title, axis: .vertical)
                        .lineLimit(1...5).focused($focusedField, equals: .title)
                        .accessibilityIdentifier("task-editor-title")
                    Picker(String(localized: "Project"), selection: $form.projectId) {
                        ForEach(projects, id: \.id) { Text($0.title).tag($0.id) }
                    }
                    .accessibilityIdentifier("task-editor-project")
                    .accessibilityValue(projects.first(where: { $0.id == form.projectId })?.title ?? "")
                    .disabled(detail?.parentId != nil)
                }
                scheduleSection
                Section {
                    ForEach(tags, id: \.id) { tag in
                        Toggle(tag.title, isOn: Binding(
                            get: { form.tagIds.contains(tag.id) },
                            set: { selected in
                                if selected { form.tagIds.insert(tag.id) } else { form.tagIds.remove(tag.id) }
                            }))
                    }
                    TextField(String(localized: "New tags, comma separated"), text: $form.newTags)
                        .autocorrectionDisabled()
                        .focused($focusedField, equals: .tags)
                } header: {
                    Label(String(localized: "Tags"), systemImage: "tag")
                        .foregroundStyle(AccentTheme.secondaryText)
                }
                if let taskID, let detail {
                    taskActions(taskID, detail: detail)
                }
                Section {
                    TextEditor(text: $form.notes)
                        .frame(minHeight: 140)
                        .focused($focusedField, equals: .notes)
                        .accessibilityLabel(String(localized: "Notes"))
                    Button(String(localized: "Copy the note"), systemImage: "doc.on.doc") {
                        UIPasteboard.general.string = form.notes
                    }
                    .disabled(form.notes.isEmpty)
                } header: {
                    Label(String(localized: "Notes"), systemImage: "note.text")
                        .foregroundStyle(AccentTheme.secondaryText)
                }
            }
            .disabled(!editable || busy)
        }
        .momentumNavigationCanvas()
        .accessibilityIdentifier("task-editor-form")
        .scrollDismissesKeyboard(.interactively)
        .animation(reduceMotion ? nil : .easeOut(duration: 0.2), value: form.dueDay != nil)
        .animation(reduceMotion ? nil : .easeOut(duration: 0.2), value: form.timeText.isEmpty)
        .overlay { if busy { ProgressView().padding().background(.regularMaterial, in: Capsule()) } }
    }

    private var scheduleSection: some SwiftUI.View {
        Section {
            Toggle(String(localized: "Scheduled"), isOn: Binding(
                get: { form.dueDay != nil },
                set: { scheduled in
                    form.dueDay = scheduled ? today() : nil
                    if !scheduled {
                        form.timeText = ""
                        form.reminder = nil
                    }
                }))
            .accessibilityIdentifier("task-editor-scheduled")
            if form.dueDay != nil {
                DatePicker(String(localized: "Due"), selection: Binding(
                    get: { form.dueDay.flatMap(Strings.date(fromDay:)) ?? Date() },
                    set: { form.dueDay = Strings.day(fromDate: $0) }), displayedComponents: .date)
                    .accessibilityIdentifier("task-editor-due-date")
            }
            Toggle(String(localized: "Time"), isOn: Binding(
                get: { !form.timeText.isEmpty },
                set: { form.timeText = $0 ? "09:00" : "" }))
            .disabled(form.dueDay == nil)
            .accessibilityIdentifier("task-editor-time-toggle")
            if !form.timeText.isEmpty {
                DatePicker(String(localized: "Time"), selection: time, displayedComponents: .hourAndMinute)
                    .disabled(form.dueDay == nil)
                    .accessibilityIdentifier("task-editor-time-picker")
            }
            Picker(String(localized: "Reminder"), selection: $form.reminder) {
                Text(String(localized: "None")).tag(nil as UInt32?)
                Text(String(localized: "At the scheduled time")).tag(UInt32(0) as UInt32?)
                Text(String(localized: "5 minutes before")).tag(UInt32(5) as UInt32?)
                Text(String(localized: "10 minutes before")).tag(UInt32(10) as UInt32?)
                Text(String(localized: "15 minutes before")).tag(UInt32(15) as UInt32?)
                Text(String(localized: "30 minutes before")).tag(UInt32(30) as UInt32?)
                Text(String(localized: "1 hour before")).tag(UInt32(60) as UInt32?)
                Text(String(localized: "1 day before")).tag(UInt32(1440) as UInt32?)
                if let offset = form.reminder, ![0, 5, 10, 15, 30, 60, 1440].contains(offset) {
                    Text(String(localized: "\(offset) minutes before")).tag(offset as UInt32?)
                }
            }
            .disabled(form.parsedTime == nil || form.dueDay == nil)
            .accessibilityIdentifier("task-editor-reminder")
            TextField(String(localized: "Estimate"), text: $form.estimateText, prompt: Text(verbatim: "1h 30m"))
                .accessibilityLabel(String(localized: "Estimate"))
                .accessibilityHint(estimateInvalid ? String(localized: "Enter an estimate such as 1h 30m.") : "")
                .autocorrectionDisabled().textInputAutocapitalization(.never)
                .focused($focusedField, equals: .estimate)
            if estimateInvalid {
                AccessibleErrorMessage(String(localized: "Enter an estimate such as 1h 30m."))
                    .accessibilityIdentifier("task-editor-estimate-error")
            }
        } header: {
            Label(String(localized: "Schedule"), systemImage: "calendar")
                .foregroundStyle(AccentTheme.secondaryText)
        }
    }

    @ViewBuilder private func taskActions(_ id: String, detail: TaskDetail) -> some SwiftUI.View {
        if menu?.topLevel == true {
            Section {
                Button { saveDraftBefore(id) { showRepeat = true } } label: {
                    LabeledContent {
                        Text(detail.repeat.map(Strings.repeatText) ?? String(localized: "Does not repeat"))
                            .foregroundStyle(AccentTheme.secondaryText)
                    } label: {
                        Label(String(localized: "Repeat"), systemImage: "repeat")
                    }
                }
                .disabled(!valid)
                TextField(String(localized: "Add subtask"), text: $subtaskTitle)
                    .focused($focusedField, equals: .subtask)
                    .onSubmit { addSubtask(id) }
                Button(String(localized: "Add subtask"), systemImage: "plus") { addSubtask(id) }
                    .disabled(subtaskTitle.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            } footer: {
                Text(String(localized: "Repeat saves your current edits first."))
                    .foregroundStyle(AccentTheme.secondaryText)
            }
        }
        if !detail.subTasks.isEmpty {
            Section {
                ForEach(detail.subTasks, id: \.id) { subtask in
                    Toggle(subtask.title, isOn: Binding(
                        get: { subtask.isDone },
                        set: { done in mutate { await worker.perform(.complete([subtask.id], done)) } }))
                }
            } header: {
                Label(String(localized: "Subtasks"), systemImage: "checklist")
                    .foregroundStyle(AccentTheme.secondaryText)
            }
        }
        Section {
            Button(String(localized: "Duplicate Task"), systemImage: "plus.square.on.square") {
                saveDraftBefore(id) {
                    mutate(closes: true) { await worker.duplicateTask(id) }
                }
            }
            .disabled(!valid)
            Button(String(localized: "Delete Task"), systemImage: "trash", role: .destructive) { confirmDelete = true }
        } footer: {
            Text(String(localized: "Duplicate saves your current edits first."))
                .foregroundStyle(AccentTheme.secondaryText)
        }
    }

    private var time: Binding<Date> {
        Binding(get: {
            let clock = form.parsedTime
            return Calendar.current.date(from: DateComponents(year: 2001, month: 1, day: 1,
                hour: Int(clock?.hour ?? 9), minute: Int(clock?.minute ?? 0))) ?? Date()
        }, set: {
            let components = Calendar.current.dateComponents([.hour, .minute], from: $0)
            form.timeText = String(format: "%02d:%02d", components.hour ?? 0, components.minute ?? 0)
        })
    }

    private func load() async {
        guard !loaded else { return }
        projects = await worker.editingProjects()
        tags = await worker.editingTags()
        if let taskID {
            detail = await worker.taskDetail(taskID)
            menu = await worker.taskMenu(taskID)
            if let detail { form = TaskFormModel(detail: detail) }
        } else {
            form = TaskFormModel(view: view, projects: projects)
            focusedField = .title
        }
        loaded = true
    }

    private func save() {
        guard valid else { return }
        focusedField = nil
        let draft = form.draft
        mutate(closes: true, acceptsUnchanged: true) {
            if let taskID { return await worker.saveTask(taskID, draft: draft) }
            return await worker.createTask(draft, view: view)
        }
    }

    private func addSubtask(_ id: String) {
        let title = subtaskTitle
        guard !title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return }
        mutate { await worker.addSubtask(id, title: title) }
    }

    private func saveDraftBefore(_ id: String, action: @escaping @MainActor () -> Void) {
        guard valid, !busy else { return }
        let draft = form.draft
        focusedField = nil
        busy = true
        error = nil
        Task { @MainActor in
            let outcome = await worker.saveTask(id, draft: draft)
            if !outcome.changed, let message = outcome.message {
                error = Strings.message(message)
                busy = false
                return
            }
            if outcome.changed { onChange() }
            guard let saved = await worker.taskDetail(id),
                  let savedMenu = await worker.taskMenu(id) else {
                error = String(localized: "This task is archived or no longer available.")
                busy = false
                return
            }
            detail = saved
            menu = savedMenu
            form = TaskFormModel(detail: saved)
            tags = await worker.editingTags()
            busy = false
            action()
        }
    }

    private func mutate(closes: Bool = false, acceptsUnchanged: Bool = false,
                        operation: @escaping @MainActor () async -> Outcome) {
        guard !busy else { return }
        busy = true
        error = nil
        Task { @MainActor in
            let outcome = await operation()
            busy = false
            if outcome.changed {
                onChange()
                if closes { dismiss() } else if let taskID {
                    detail = await worker.taskDetail(taskID)
                    subtaskTitle = ""
                }
            } else if acceptsUnchanged && outcome.message == nil {
                dismiss()
            } else {
                error = outcome.message.map(Strings.message) ?? String(localized: "The task could not be changed.")
            }
        }
    }
}
