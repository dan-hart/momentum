// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// The smaller sheets: the repeat editor, the two bulk pickers, project and tag editing,
// the nearby-devices screen and the shortcut reference.
import MomentumCore
import MomentumKit
import SwiftUI

// MARK: - Repeat

struct RepeatSheet: SwiftUI.View {
    @Environment(AppState.self) private var state
    @Environment(\.dismiss) private var dismiss
    let taskId: String

    @State private var draft: RepeatDraft?

    private static let weekdayOrder: [UInt32] = [1, 2, 3, 4, 5, 6, 0] // Monday first

    var body: some SwiftUI.View {
        VStack(spacing: 0) {
            if let d = draft {
                Form {
                    Section {
                        Text(Strings.repeatText(state.engine.describeRepeatDraft(draft: d)))
                            .foregroundStyle(.secondary)
                    }
                    Section {
                        Picker(String(localized: "Repeats"), selection: binding(\.cycle)) {
                            Text(String(localized: "Daily")).tag(RepeatCycle.daily)
                            Text(String(localized: "Weekly")).tag(RepeatCycle.weekly)
                            Text(String(localized: "Monthly")).tag(RepeatCycle.monthly)
                            Text(String(localized: "Yearly")).tag(RepeatCycle.yearly)
                        }
                        Stepper(value: binding(\.every), in: 1...99) {
                            LabeledContent(String(localized: "Every"), value: unitLabel(d))
                        }
                        if d.cycle == .weekly {
                            LabeledContent(String(localized: "On")) {
                                HStack(spacing: 4) {
                                    ForEach(Self.weekdayOrder, id: \.self) { wd in
                                        let on = d.weekdays[Int(wd)]
                                        Button {
                                            var next = d
                                            next.weekdays[Int(wd)].toggle()
                                            draft = next
                                        } label: {
                                            Text(Strings.shortWeekdayName(wd).prefix(2))
                                                .appFont(.caption)
                                                .frame(width: 26, height: 20)
                                                .background(on ? Color.accentColor.opacity(0.3) : Color.secondary.opacity(0.12),
                                                            in: Capsule())
                                        }
                                        .buttonStyle(.plain)
                                    }
                                }
                            }
                        }
                        if d.cycle == .monthly {
                            Picker(String(localized: "Monthly on"), selection: monthlyKind) {
                                Text(String(localized: "The same date")).tag(0)
                                Text(String(localized: "The last day of the month")).tag(1)
                                Text(String(localized: "A weekday of the month")).tag(2)
                            }
                            if case .nthWeekday(let week, let weekday) = d.monthly {
                                Picker(String(localized: "Which"), selection: nthWeek) {
                                    Text(String(localized: "First")).tag(Int32(1))
                                    Text(String(localized: "Second")).tag(Int32(2))
                                    Text(String(localized: "Third")).tag(Int32(3))
                                    Text(String(localized: "Fourth")).tag(Int32(4))
                                    Text(String(localized: "Last")).tag(Int32(-1))
                                }
                                Picker(String(localized: "Weekday"), selection: nthWeekday) {
                                    ForEach(Self.weekdayOrder, id: \.self) { wd in
                                        Text(Strings.weekdayName(wd)).tag(wd)
                                    }
                                }
                                .onAppear { _ = (week, weekday) }
                            }
                        }
                        DatePicker(String(localized: "Starts"), selection: startDate, displayedComponents: .date)
                        Toggle(String(localized: "Paused"), isOn: binding(\.paused))
                    } footer: {
                        Text(String(localized: "A paused schedule keeps its settings but creates no tasks."))
                    }
                    if d.existing {
                        Section {
                            Button(String(localized: "Stop Repeating"), role: .destructive) {
                                state.stopRepeat(taskId)
                                dismiss()
                            }
                        }
                    }
                }
                .formStyle(.grouped)
            } else {
                ProgressView()
            }
        }
        .frame(width: 440, height: 520)
        .toolbar {
            ToolbarItem(placement: .cancellationAction) {
                Button(String(localized: "Cancel")) { dismiss() }
            }
            ToolbarItem(placement: .confirmationAction) {
                Button(draft?.existing == true ? String(localized: "Save") : String(localized: "Repeat")) {
                    if let d = draft, state.saveRepeat(taskId, d) { dismiss() }
                }
                .disabled(draft == nil)
            }
        }
        .onAppear { draft = state.engine.repeatDraft(taskId: taskId) }
    }

    private func binding<T>(_ key: WritableKeyPath<RepeatDraft, T>) -> Binding<T> {
        Binding(
            get: { draft?[keyPath: key] ?? RepeatDraft(cycle: .weekly, every: 1, weekdays: Array(repeating: false, count: 7), monthly: .sameDay, startDate: nil, paused: false, existing: false)[keyPath: key] },
            set: { draft?[keyPath: key] = $0 })
    }
    private var monthlyKind: Binding<Int> {
        Binding(
            get: {
                switch draft?.monthly {
                case .lastDay: return 1
                case .nthWeekday: return 2
                default: return 0
                }
            },
            set: { kind in
                switch kind {
                case 1: draft?.monthly = .lastDay
                case 2: draft?.monthly = .nthWeekday(week: 1, weekday: 1)
                default: draft?.monthly = .sameDay
                }
            })
    }
    private var nthWeek: Binding<Int32> {
        Binding(
            get: { if case .nthWeekday(let w, _) = draft?.monthly { return w } else { return 1 } },
            set: { w in if case .nthWeekday(_, let d) = draft?.monthly { draft?.monthly = .nthWeekday(week: w, weekday: d) } })
    }
    private var nthWeekday: Binding<UInt32> {
        Binding(
            get: { if case .nthWeekday(_, let d) = draft?.monthly { return d } else { return 1 } },
            set: { d in if case .nthWeekday(let w, _) = draft?.monthly { draft?.monthly = .nthWeekday(week: w, weekday: d) } })
    }
    private var startDate: Binding<Date> {
        Binding(
            get: { draft?.startDate.flatMap(Strings.date(fromDay:)) ?? Date() },
            set: { draft?.startDate = Strings.day(fromDate: $0) })
    }
    private func unitLabel(_ d: RepeatDraft) -> String {
        switch d.cycle {
        case .daily: return String(localized: "\(d.every) days")
        case .weekly: return String(localized: "\(d.every) weeks")
        case .monthly: return String(localized: "\(d.every) months")
        case .yearly: return String(localized: "\(d.every) years")
        }
    }
}

// MARK: - Bulk pickers

struct MoveToProjectSheet: SwiftUI.View {
    @Environment(AppState.self) private var state
    @Environment(\.dismiss) private var dismiss
    let ids: [String]
    @State private var projectId = ""

    var body: some SwiftUI.View {
        SheetFrame(title: ids.count == 1
            ? String(localized: "Move to Project")
            : String(localized: "Move \(ids.count) Tasks to Project"),
            confirm: String(localized: "Move"),
            canConfirm: !projectId.isEmpty) {
            state.moveToProject(ids, projectId)
        } content: {
            Picker(String(localized: "Project"), selection: $projectId) {
                ForEach(state.engine.projects(), id: \.id) { p in
                    Text(p.title).tag(p.id)
                }
            }
            .labelsHidden()
            .pickerStyle(.inline)
            .frame(height: 160)
        }
        .onAppear { projectId = state.engine.projects().first?.id ?? "" }
    }
}

struct AddTagSheet: SwiftUI.View {
    @Environment(AppState.self) private var state
    @Environment(\.dismiss) private var dismiss
    let ids: [String]
    @State private var tagId = ""
    @State private var newName = ""

    var body: some SwiftUI.View {
        SheetFrame(title: ids.count == 1
            ? String(localized: "Add Tag")
            : String(localized: "Add Tag to \(ids.count) Tasks"),
            confirm: String(localized: "Add"),
            canConfirm: !tagId.isEmpty || !newName.trimmingCharacters(in: .whitespaces).isEmpty) {
            if newName.trimmingCharacters(in: .whitespaces).isEmpty {
                state.addTag(ids, tagId: tagId)
            } else {
                state.addTag(ids, name: newName)
            }
        } content: {
            Picker(String(localized: "Tag"), selection: $tagId) {
                ForEach(state.engine.tags(), id: \.id) { t in
                    Text(t.title).tag(t.id)
                }
            }
            TextField(String(localized: "Or a new tag name"), text: $newName)
        }
        .onAppear { tagId = state.engine.tags().first?.id ?? "" }
    }
}

struct NewProjectSheet: SwiftUI.View {
    @Environment(AppState.self) private var state
    @State private var name = ""

    var body: some SwiftUI.View {
        SheetFrame(title: String(localized: "New Project"),
                   confirm: String(localized: "Add"),
                   canConfirm: !name.trimmingCharacters(in: .whitespaces).isEmpty) {
            state.addProject(name)
        } content: {
            TextField(String(localized: "Project name"), text: $name)
        }
    }
}

struct EditContextSheet: SwiftUI.View {
    @Environment(AppState.self) private var state
    let view: MomentumCore.View
    @State private var name = ""
    @State private var color = Color.accentColor
    @State private var hasColor = false

    private var isProject: Bool {
        if case .project = view { return true }
        return false
    }

    var body: some SwiftUI.View {
        SheetFrame(title: isProject ? String(localized: "Edit Project") : String(localized: "Edit Tag"),
                   confirm: String(localized: "Save"),
                   canConfirm: !name.trimmingCharacters(in: .whitespaces).isEmpty) {
            state.updateContext(view, title: name, color: color.hexString)
        } content: {
            TextField(String(localized: "Name"), text: $name)
            ColorPicker(String(localized: "Color"), selection: $color, supportsOpacity: false)
        }
        .onAppear {
            switch view {
            case .project(let id):
                if let p = state.engine.project(id: id) {
                    name = p.title
                    if let c = p.color.flatMap(Color.init(hex:)) { color = c; hasColor = true }
                }
            case .tag(let id):
                if let t = state.engine.tag(id: id) {
                    name = t.title
                    if let c = t.color.flatMap(Color.init(hex:)) { color = c; hasColor = true }
                }
            default: break
            }
        }
    }
}

/// One shape for the small sheets: a title, some fields, Cancel and a confirm button.
struct SheetFrame<Content: SwiftUI.View>: SwiftUI.View {
    @Environment(\.dismiss) private var dismiss
    let title: String
    let confirm: String
    let canConfirm: Bool
    let action: () -> Void
    @ViewBuilder let content: Content

    init(title: String, confirm: String, canConfirm: Bool,
         action: @escaping () -> Void, @ViewBuilder content: () -> Content) {
        self.title = title
        self.confirm = confirm
        self.canConfirm = canConfirm
        self.action = action
        self.content = content()
    }

    var body: some SwiftUI.View {
        VStack(alignment: .leading, spacing: 14) {
            Text(title).appFont(.headline)
            content
            HStack {
                Spacer()
                Button(String(localized: "Cancel"), role: .cancel) { dismiss() }
                    .keyboardShortcut(.cancelAction)
                Button(confirm) {
                    action()
                    dismiss()
                }
                .keyboardShortcut(.defaultAction)
                .disabled(!canConfirm)
            }
        }
        .padding(20)
        .frame(width: 380)
    }
}

extension Color {
    /// `#rrggbb` for the sync data, which stores colours as CSS strings.
    var hexString: String? {
        guard let c = NSColor(self).usingColorSpace(.sRGB) else { return nil }
        return String(format: "#%02x%02x%02x",
                      Int(round(c.redComponent * 255)),
                      Int(round(c.greenComponent * 255)),
                      Int(round(c.blueComponent * 255)))
    }
}

// MARK: - Nearby devices

struct DevicesSheet: SwiftUI.View {
    @Environment(AppState.self) private var state
    @Environment(\.dismiss) private var dismiss
    @State private var linking: NearbyDevice?
    @State private var code = ""

    var body: some SwiftUI.View {
        VStack(alignment: .leading, spacing: 0) {
            Form {
                Section(String(localized: "This Device")) {
                    LabeledContent(state.p2pInfo?.deviceName ?? String(localized: "This Mac")) {
                        Text(String(localized: "Listening on port \(state.p2pInfo?.port ?? 0)"))
                            .foregroundStyle(.secondary)
                    }
                    if let code = state.pairingCode {
                        LabeledContent {
                            Text(code.prefix(3) + " " + code.suffix(3))
                                .appFont(.title2).monospacedDigit()
                                .textSelection(.enabled)
                        } label: {
                            VStack(alignment: .leading) {
                                Text(String(localized: "Pairing code"))
                                Text(String(localized: "Enter it on the other device within five minutes"))
                                    .appFont(.caption).foregroundStyle(.secondary)
                            }
                        }
                    }
                }
                Section {
                    if state.nearbyDiscovered.isEmpty {
                        HStack {
                            ProgressView().controlSize(.small)
                            Text(String(localized: "Searching… open this screen on the other device too"))
                                .foregroundStyle(.secondary)
                        }
                    }
                    ForEach(state.nearbyDiscovered, id: \.deviceId) { d in
                        LabeledContent(d.name) {
                            HStack {
                                Text(d.address ?? "").foregroundStyle(.secondary).appFont(.caption)
                                Button(String(localized: "Link…")) {
                                    code = ""
                                    linking = d
                                }
                            }
                        }
                    }
                } header: {
                    Text(String(localized: "Nearby"))
                } footer: {
                    Text(String(localized: "Devices running Momentum on this network. Link with the code shown on the other device."))
                }
                Section(String(localized: "Linked Devices")) {
                    if state.nearbyLinked.isEmpty {
                        Text(String(localized: "None yet")).foregroundStyle(.secondary)
                    }
                    ForEach(state.nearbyLinked, id: \.deviceId) { d in
                        LabeledContent {
                            Button(String(localized: "Unlink"), role: .destructive) { state.unlink(d) }
                        } label: {
                            VStack(alignment: .leading) {
                                Text(d.name)
                                Text(d.lastSeenMs.map { String(localized: "Last synced \(Strings.ago($0))") }
                                    ?? String(localized: "Not synced yet"))
                                    .appFont(.caption).foregroundStyle(.secondary)
                            }
                        }
                    }
                }
            }
            .formStyle(.grouped)
        }
        .frame(width: 460, height: 560)
        .toolbar {
            ToolbarItem(placement: .confirmationAction) {
                Button(String(localized: "Done")) { dismiss() }
            }
        }
        .onDisappear { state.devicesClosed() }
        .sheet(item: $linking) { device in
            SheetFrame(title: String(localized: "Link with \(device.name)"),
                       confirm: String(localized: "Link"),
                       canConfirm: code.count >= 6) {
                state.link(device, code: code)
            } content: {
                Text(String(localized: "Type the pairing code shown in Nearby Devices on that device."))
                    .appFont(.caption).foregroundStyle(.secondary)
                TextField(String(localized: "Pairing code"), text: $code, prompt: Text(verbatim: "000000"))
            }
        }
    }
}

extension NearbyDevice: @retroactive Identifiable {
    public var id: String { deviceId }
}

// MARK: - Shortcuts

struct ShortcutsSheet: SwiftUI.View {
    @Environment(AppState.self) private var state
    @Environment(\.dismiss) private var dismiss

    private func keys(_ shortcuts: AppShortcut...) -> String {
        shortcuts.map { $0.display(modifier: state.modifier) }.joined(separator: " / ")
    }

    private var groups: [(String, [(String, String)])] {
        [
            (String(localized: "General"), [
                (String(localized: "New Task"), keys(.newTask)),
                (String(localized: "New Project"), keys(.newProject)),
                (String(localized: "Quick Add"), keys(.quickAdd)),
                (String(localized: "Search"), keys(.search)),
                (String(localized: "Focus Quick Add"), keys(.focusQuickAdd)),
                (String(localized: "Preferences"), keys(.settings)),
                (String(localized: "Undo"), keys(.undo)),
                (String(localized: "Keyboard Shortcuts"), keys(.shortcuts)),
                (String(localized: "Close Window"), keys(.closeWindow)),
                (String(localized: "Quit"), keys(.quit)),
            ]),
            (String(localized: "Tasks"), [
                (String(localized: "Open Task"), keys(.openTask)),
                (String(localized: "Mark as Done"), keys(.markDone)),
                (String(localized: "Delete Task"), keys(.deleteTask)),
                (String(localized: "Duplicate Task"), keys(.duplicateTask)),
                (String(localized: "Copy Task Title"), keys(.copyTitle)),
                (String(localized: "Plan for Today"), keys(.planToday)),
                (String(localized: "Move to Morning"), keys(.morning)),
                (String(localized: "Move to Tonight"), keys(.tonight)),
                (String(localized: "Move to Tomorrow"), keys(.tomorrow)),
                (String(localized: "Move to Next Week"), keys(.nextWeek)),
                (String(localized: "Move to Project"), keys(.moveToProject)),
                (String(localized: "Repeat Schedule"), keys(.repeatSchedule)),
                (String(localized: "Move Up / Down"), keys(.moveUp, .moveDown)),
                (String(localized: "Archive Completed"), keys(.archive)),
            ]),
            (String(localized: "Navigation"), [
                (String(localized: "Go to Sidebar Entry 1–9"), keys(.sidebar(1)) + " … " + keys(.sidebar(9))),
                (String(localized: "Next / Previous View"), keys(.nextView, .previousView)),
                (String(localized: "Toggle Sidebar"), keys(.toggleSidebar)),
                (String(localized: "Sync Now"), keys(.sync, .syncFunction)),
                (String(localized: "Select All / Deselect All"), keys(.selectAll, .deselectAll)),
            ]),
            (String(localized: "System-wide"), [
                (String(localized: "Add a Task from Anywhere"), keys(.globalQuickAdd)),
                (String(localized: "Show Momentum"), keys(.globalShow)),
            ]),
        ]
    }

    var body: some SwiftUI.View {
        VStack(spacing: 0) {
            Form {
                ForEach(groups, id: \.0) { group in
                    Section(group.0) {
                        ForEach(group.1, id: \.0) { name, keys in
                            LabeledContent(name) {
                                Text(keys).monospaced().foregroundStyle(.secondary)
                            }
                            .accessibilityElement(children: .ignore)
                            .accessibilityLabel(Text(name))
                            .accessibilityValue(Text(keys))
                            .accessibilityAddTraits(.isStaticText)
                        }
                    }
                }
            }
            .formStyle(.grouped)
        }
        .frame(width: 460, height: 600)
        .toolbar {
            ToolbarItem(placement: .confirmationAction) {
                Button(String(localized: "Done")) { dismiss() }
            }
        }
    }
}
