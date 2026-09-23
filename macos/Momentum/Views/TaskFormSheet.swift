// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// New Task and the task editor: the same form either way. Creating has Cancel and Create
// in the toolbar with Create disabled until there is a title; editing applies as you go
// and adds the repeat row, subtasks and Delete.
import MomentumCore
import MomentumKit
import SwiftUI

struct TaskFormSheet: SwiftUI.View {
    @Environment(AppState.self) private var state
    @LabelColors private var colorful: Bool
    @Environment(\.dismiss) private var dismiss
    @Environment(\.appTypography) private var typography
    let taskId: String?

    @State private var form = TaskFormModel()
    @State private var newSubtask = ""
    @State private var loaded = false
    @State private var saveError: String?
    @FocusState private var titleFocused: Bool

    private var isNew: Bool { taskId == nil }
    private var detail: TaskDetail? { taskId.flatMap { state.engine.taskDetail(id: $0) } }
    private var parsedTime: ClockTime? { form.parsedTime }
    private var timeInvalid: Bool { form.timeInvalid }
    private var formWidth: Double {
        let scale = max(1, max(typography.contentSize, typography.interfaceSize) / 13)
        let available = (NSScreen.main?.visibleFrame.width ?? 800) - 40
        return min(max(520, available), min(760, 520 * scale))
    }

    /// Minutes before the scheduled time, in the order the picker lists them.
    private static let reminderChoices: [(UInt32?, String)] = [
        (nil, String(localized: "None")),
        (0, String(localized: "At the scheduled time")),
        (5, String(localized: "5 minutes before")),
        (10, String(localized: "10 minutes before")),
        (15, String(localized: "15 minutes before")),
        (30, String(localized: "30 minutes before")),
        (60, String(localized: "1 hour before")),
        (1440, String(localized: "1 day before")),
    ]

    var body: some SwiftUI.View {
        VStack(spacing: 0) {
            Form {
                Section {
                    TextField(String(localized: "Title"), text: $form.title, axis: .vertical)
                        .appFont(.title2, area: .content)
                        .lineLimit(1...4)
                        .labelsHidden()
                        .accessibilityLabel(String(localized: "Title"))
                        .focused($titleFocused)
                        .onSubmit(save)
                    Picker(String(localized: "Project"), selection: $form.projectId) {
                        ForEach(state.engine.projects(), id: \.id) { p in
                            Label {
                                Text(p.title)
                            } icon: {
                                Image(systemName: "folder")
                                    .foregroundStyle(colorful ? (p.color.flatMap(Color.init(hex:)) ?? .secondary) : .secondary)
                            }
                            .tag(p.id)
                        }
                    }
                    .disabled(detail?.parentId != nil)
                }
                Section {
                    DueRow(dueDay: $form.dueDay)
                    TextField(String(localized: "Time"), text: $form.timeText, prompt: Text(verbatim: "14:30"))
                        .foregroundStyle(timeInvalid ? Color.red : .primary)
                        .accessibilityHint(timeInvalid ? String(localized: "Invalid time. Use a time such as 14:30.") : "")
                    Picker(String(localized: "Reminder"), selection: $form.reminder) {
                        ForEach(Self.reminderChoices, id: \.1) { minutes, label in
                            Text(label).tag(minutes)
                        }
                    }
                    .disabled(parsedTime == nil)
                    TextField(String(localized: "Estimate"), text: $form.estimateText, prompt: Text(verbatim: "1h 30m"))
                }
                Section(String(localized: "Tags")) {
                    TagChips(selected: $form.tagIds)
                    TextField(String(localized: "New tags, comma separated"), text: $form.newTags)
                }
                Section {
                    TextEditor(text: $form.notes)
                        .accessibilityLabel(String(localized: "Notes"))
                        .frame(minHeight: 90)
                        .scrollContentBackground(.hidden)
                        .appFont(.body, area: .content)
                } header: {
                    HStack {
                        Text(String(localized: "Notes"))
                        Spacer()
                        Button {
                            NSPasteboard.general.clearContents()
                            NSPasteboard.general.setString(form.notes, forType: .string)
                        } label: {
                            Image(systemName: "doc.on.doc")
                        }
                        .buttonStyle(.borderless)
                        .help(String(localized: "Copy the note"))
                        .accessibilityLabel(String(localized: "Copy the note"))
                        .disabled(form.notes.isEmpty)
                    }
                }
                if let id = taskId, let d = detail {
                    if d.parentId == nil {
                        Section {
                            Button {
                                save()
                                state.sheet = .repeatSchedule(id)
                            } label: {
                                LabeledContent(String(localized: "Repeat")) {
                                    Text(d.repeat.map(Strings.repeatText) ?? String(localized: "Does not repeat"))
                                        .foregroundStyle(.secondary)
                                }
                            }
                            .buttonStyle(.plain)
                            TextField(String(localized: "Add subtask"), text: $newSubtask)
                                .appFont(.body, area: .content)
                                .onSubmit {
                                    state.addSubtask(id, newSubtask)
                                    newSubtask = ""
                                }
                        }
                    }
                    if !d.subTasks.isEmpty {
                        Section(String(localized: "Subtasks")) {
                            ForEach(d.subTasks, id: \.id) { sub in
                                Toggle(sub.title, isOn: Binding(
                                    get: { sub.isDone },
                                    set: { state.setDone(sub.id, $0) }))
                                    .appFont(.body, area: .content)
                            }
                        }
                    }
                    Section {
                        Button(String(localized: "Delete Task"), role: .destructive) {
                            dismiss()
                            state.delete([id])
                        }
                        .tint(.red)
                        .foregroundStyle(.red)
                    }
                }
            }
            .formStyle(.grouped)
        }
        .frame(width: formWidth)
        .frame(minHeight: 440, idealHeight: 620, maxHeight: 760)
        .toolbar {
            if isNew {
                ToolbarItem(placement: .cancellationAction) {
                    Button(String(localized: "Cancel")) { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button(String(localized: "Create")) { save() }
                        .disabled(!form.canSave)
                }
            } else {
                ToolbarItem(placement: .confirmationAction) {
                    Button(String(localized: "Done")) { save() }
                }
            }
        }
        .alert("Could not save changes", isPresented: Binding(
            get: { saveError != nil }, set: { if !$0 { saveError = nil } }
        )) {
            Button("OK", role: .cancel) { saveError = nil }
        } message: {
            Text(saveError ?? "")
        }
        .onAppear(perform: load)
        .onDisappear { if !isNew { applyEdit() } }
    }

    private func load() {
        guard !loaded else { return }
        loaded = true
        if let d = detail {
            form = TaskFormModel(detail: d)
        } else {
            form = TaskFormModel(view: state.view, projects: state.engine.projects())
            titleFocused = true
        }
    }

    private func save() {
        let outcome: Outcome?
        if isNew {
            guard form.canSave else { return }
            outcome = state.createTask(form.draft)
        } else {
            outcome = applyEdit()
        }
        if let message = outcome?.message, case .saveFailed(let error) = message {
            saveError = error
            return
        }
        dismiss()
    }

    @discardableResult private func applyEdit() -> Outcome? {
        guard let id = taskId, state.engine.taskDetail(id: id) != nil else { return nil }
        return state.saveTask(id, form.draft)
    }
}

/// Due: a calendar plus the two moves people make most.
private struct DueRow: SwiftUI.View {
    @Binding var dueDay: String?

    var body: some SwiftUI.View {
        LabeledContent(String(localized: "Due")) {
            HStack {
                if let day = dueDay, let date = Strings.date(fromDay: day) {
                    DatePicker("", selection: Binding(
                        get: { date },
                        set: { dueDay = Strings.day(fromDate: $0) }
                    ), displayedComponents: .date)
                    .labelsHidden()
                    .accessibilityLabel(String(localized: "Due"))
                } else {
                    Text(String(localized: "Not scheduled")).foregroundStyle(.secondary)
                }
                Spacer()
                Menu(String(localized: "Set")) {
                    Button(String(localized: "Today")) { dueDay = today() }
                    Button(String(localized: "Tomorrow")) { dueDay = dayOffset(day: today(), days: 1) }
                    Button(String(localized: "None")) { dueDay = nil }
                }
                .fixedSize()
            }
        }
    }
}

/// Existing tags as toggle chips, in the tag's own colour.
private struct TagChips: SwiftUI.View {
    @Environment(AppState.self) private var state
    @LabelColors private var colorful: Bool
    @Binding var selected: Set<String>

    var body: some SwiftUI.View {
        let tags = state.engine.tags()
        if tags.isEmpty {
            Text(String(localized: "No tags yet")).foregroundStyle(.secondary)
        } else {
            FlowLayout(spacing: 6) {
                ForEach(tags, id: \.id) { tag in
                    let on = selected.contains(tag.id)
                    Button {
                        if on { selected.remove(tag.id) } else { selected.insert(tag.id) }
                    } label: {
                        HStack(spacing: 4) {
                            if on { Image(systemName: "checkmark").accessibilityHidden(true) }
                            if colorful, let color = tag.color.flatMap(Color.init(hex:)) {
                                Circle().fill(color).frame(width: 6, height: 6).accessibilityHidden(true)
                            }
                            Text("#" + tag.title)
                        }
                            .appFont(.caption)
                            .padding(.horizontal, 8)
                            .padding(.vertical, 3)
                            .background(on ? Color.accentColor.opacity(0.25) : Color.secondary.opacity(0.12),
                                        in: Capsule())
                            .foregroundStyle(.primary)
                    }
                    .buttonStyle(.plain)
                    .accessibilityAddTraits(on ? .isSelected : [])
                }
            }
        }
    }
}

/// Chips wrap onto as many rows as they need.
struct FlowLayout: Layout {
    var spacing: CGFloat = 6

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let width = proposal.width ?? 320
        var x: CGFloat = 0, y: CGFloat = 0, rowHeight: CGFloat = 0
        for view in subviews {
            let size = view.sizeThatFits(.unspecified)
            if x + size.width > width, x > 0 {
                x = 0
                y += rowHeight + spacing
                rowHeight = 0
            }
            x += size.width + spacing
            rowHeight = max(rowHeight, size.height)
        }
        return CGSize(width: width, height: y + rowHeight)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        var x = bounds.minX, y = bounds.minY, rowHeight: CGFloat = 0
        for view in subviews {
            let size = view.sizeThatFits(.unspecified)
            if x + size.width > bounds.maxX, x > bounds.minX {
                x = bounds.minX
                y += rowHeight + spacing
                rowHeight = 0
            }
            view.place(at: CGPoint(x: x, y: y), proposal: ProposedViewSize(size))
            x += size.width + spacing
            rowHeight = max(rowHeight, size.height)
        }
    }
}
