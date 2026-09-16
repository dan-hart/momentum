// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// The task list: one inset-grouped section per core section, native multi-selection,
// drag to reorder or to a sidebar entry, a context menu that acts on the selection.
import MomentumCore
import MomentumKit
import SwiftUI

struct TaskListView: SwiftUI.View {
    @Environment(AppState.self) private var state
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @LabelColors private var colorful: Bool
    @FocusState private var listFocused: Bool

    var body: some SwiftUI.View {
        @Bindable var state = state
        Group {
            if let empty = state.listing.empty {
                let copy = Strings.empty(empty, modifier: state.modifier.symbol, syncConfigured: state.prefs.syncConfigured)
                EmptyTaskListView(copy: copy)
            } else {
                List(selection: $state.selection) {
                    if let allDone = state.listing.allDone {
                        AllDoneView(allDone: allDone)
                            .listRowSeparator(.hidden)
                            .selectionDisabled()
                    }
                    ForEach(Array(state.listing.sections.enumerated()), id: \.offset) { _, section in
                        Section {
                            ForEach(section.rows, id: \.rowId) { row in
                                rowView(row)
                            }
                            if let note = section.note {
                                Text(Strings.sectionNote(note))
                                    .appFont(.caption)
                                    .foregroundStyle(.secondary)
                                    .selectionDisabled()
                            }
                        } header: {
                            if let group = section.group, let headingColor = groupColor(group) {
                                HStack(alignment: .firstTextBaseline, spacing: 4) {
                                    if let context = Strings.sectionContextTitle(section) {
                                        Text(context + " ·")
                                            .foregroundStyle(section.kind == .overdue ? Color.red : .secondary)
                                    }
                                    Text(Strings.groupTitle(group))
                                        .foregroundStyle(headingColor)
                                }
                                .appFont(.headline)
                                .accessibilityElement(children: .combine)
                            } else if let title = Strings.sectionTitle(section) {
                                Text(title)
                                    .appFont(.headline)
                                    .foregroundStyle(section.kind == .overdue ? Color.red : .secondary)
                            }
                        }
                    }
                    if state.listing.moreAvailable > 0 {
                        Button(String(localized: "Show More (\(state.listing.moreAvailable) remaining)")) {
                            state.showMoreArchive()
                        }
                        .buttonStyle(.borderless)
                        .frame(maxWidth: .infinity)
                        .selectionDisabled()
                    }
                }
                .listStyle(.inset)
                .focused($listFocused)
                // Selecting a row alone can leave the quick-add field as first responder.
                .simultaneousGesture(TapGesture().onEnded { listFocused = true })
                .focusedValue(\.selectAllTasks, { state.selection = Set(state.allRowIds) })
                .onKeyPress(.escape) {
                    guard !state.selection.isEmpty else { return .ignored }
                    state.selection.removeAll()
                    return .handled
                }
                .contextMenu(forSelectionType: String.self) { ids in
                    TaskContextMenu(ids: orderedIds(ids))
                } primaryAction: { ids in
                    if ids.count == 1, let id = ids.first, state.row(id)?.archived == false {
                        state.open(id)
                    }
                }
                .onDeleteCommand {
                    let ids = state.targets
                    Task { @MainActor in state.delete(ids) }
                }
                .onKeyPress(.return) {
                    guard let id = state.target, state.row(id)?.archived == false else { return .ignored }
                    state.open(id)
                    return .handled
                }
                .onKeyPress(.space) {
                    let ids = state.targets.filter { state.row($0)?.archived == false }
                    guard !ids.isEmpty else { return .ignored }
                    Task { @MainActor in
                        withAnimation(reduceMotion ? nil : .easeInOut(duration: 0.2)) {
                            state.toggleDone(ids)
                        }
                    }
                    return .handled
                }
            }
        }
        // Empty and populated lists occupy the same space, keeping quick-add at the top.
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }

    private func orderedIds(_ ids: Set<String>) -> [String] {
        state.allRowIds.filter { ids.contains($0) }
    }

    @ViewBuilder
    private func rowView(_ row: Row) -> some SwiftUI.View {
        switch row {
        case .task(let task):
            TaskRowView(row: task)
                .tag(task.id)
        case .project(let item):
            Label {
                Text(item.title)
            } icon: {
                Image(systemName: "folder").foregroundStyle(color(item.color))
            }
            .tag("project:\(item.id)")
            .contentShape(Rectangle())
            .onTapGesture { state.go(to: .project(id: item.id)) }
        case .tag(let item):
            Label {
                Text(item.title)
            } icon: {
                Image(systemName: "tag").foregroundStyle(color(item.color))
            }
            .tag("tag:\(item.id)")
            .contentShape(Rectangle())
            .onTapGesture { state.go(to: .tag(id: item.id)) }
        }
    }

    private func groupColor(_ group: TaskGroup) -> Color? {
        switch group {
        case .project(_, _, let hex), .tag(_, _, let hex): return color(hex)
        default: return nil
        }
    }

    private func color(_ hex: String?) -> Color {
        if colorful, let h = hex, let c = Color(hex: h) { return c }
        return .secondary
    }
}

/// A quiet introduction to an empty list, aligned with the task-entry field above it.
/// Scrollable so long project names and translated guidance also fit smaller windows.
private struct EmptyTaskListView: SwiftUI.View {
    @Environment(AppState.self) private var state
    let copy: Strings.EmptyCopy

    var body: some SwiftUI.View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                Image(systemName: copy.symbol)
                    .font(.system(size: 36, weight: .regular))
                    .symbolRenderingMode(.hierarchical)
                    .foregroundStyle(.tint)
                    .accessibilityHidden(true)

                VStack(alignment: .leading, spacing: 8) {
                    Text(copy.title)
                        .appFont(.title2)
                        .foregroundStyle(.primary)
                        .accessibilityAddTraits(.isHeader)
                    Text(copy.description)
                        .appFont()
                        .foregroundStyle(.secondary)
                        .lineSpacing(4)
                        .fixedSize(horizontal: false, vertical: true)
                }
                if !state.isSearching && state.view != .archive {
                    Button {
                        state.requestQuickAddFocus()
                    } label: {
                        Label(String(localized: "Add a task"), systemImage: "plus")
                    }
                    .buttonStyle(.bordered)
                }
            }
            .frame(maxWidth: 460, alignment: .leading)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, 26)
            .padding(.top, 36)
            .padding(.bottom, 24)
        }
    }
}

extension Row {
    var rowId: String {
        switch self {
        case .task(let row): return row.id
        case .project(let item): return "project:\(item.id)"
        case .tag(let item): return "tag:\(item.id)"
        }
    }
}

/// The context menu for one task or the whole selection.
struct TaskContextMenu: SwiftUI.View {
    @Environment(AppState.self) private var state
    let ids: [String]

    private var info: TaskMenuInfo? {
        ids.count == 1 ? state.engine.taskMenu(id: ids[0]) : nil
    }
    private var live: [String] { ids.filter { state.row($0)?.archived == false } }

    var body: some SwiftUI.View {
        if live.isEmpty {
            EmptyView()
        } else {
            if live.count == 1 {
                Button(String(localized: "Open")) { state.open(live[0]) }
            }
            Button(info?.isDone == true ? String(localized: "Mark as Not Done") : String(localized: "Mark as Done")) {
                state.toggleDone(live)
            }
            Divider()
            Button(info?.plannedToday == true ? String(localized: "Remove from Today") : String(localized: "Plan for Today")) {
                state.toggleToday(live)
            }
            Button(info?.slot == .morning ? String(localized: "Move to Today") : String(localized: "Move to Morning")) {
                state.toggleSlot(live, .morning)
            }
            Button(info?.slot == .tonight ? String(localized: "Move to Today") : String(localized: "Move to Tonight")) {
                state.toggleSlot(live, .tonight)
            }
            Button(String(localized: "Move to Tomorrow")) { state.moveToTomorrow(live) }
            Button(String(localized: "Move to Next Week")) { state.moveToNextWeek(live) }
            let topLevel = live.filter { state.row($0)?.isSubtask == false }
            if !topLevel.isEmpty {
                Button(String(localized: "Move to Project…")) { state.sheet = .moveToProject(topLevel) }
            }
            Button(String(localized: "Add Tag…")) { state.sheet = .addTag(live) }
            if let i = info, i.topLevel {
                Divider()
                Button(i.repeats ? String(localized: "Edit Repeat…") : String(localized: "Repeat…")) {
                    state.sheet = .repeatSchedule(i.id)
                }
            }
            if live.count == 1 {
                Button(String(localized: "Duplicate")) { state.duplicate(live[0]) }
                Button(String(localized: "Copy Title")) { state.copyTitle(live[0]) }
            }
            Divider()
            Button(String(localized: "Delete"), role: .destructive) { state.delete(live) }
        }
    }
}

struct AllDoneView: SwiftUI.View {
    @Environment(AppState.self) private var state
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var appeared = false
    let allDone: AllDone

    var body: some SwiftUI.View {
        let copy = Strings.allDone(allDone)
        VStack(spacing: 6) {
            Image(systemName: "checkmark.circle.fill")
                .font(.system(size: 48))
                .symbolRenderingMode(.hierarchical)
                .foregroundStyle(.tint)
                .symbolEffect(.pulse, value: appeared)
                .symbolEffectsRemoved(reduceMotion)
                .accessibilityHidden(true)
                .padding(.bottom, 4)
            Text(copy.title).appFont(.title2).accessibilityAddTraits(.isHeader)
            Text(copy.description)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .fixedSize(horizontal: false, vertical: true)
            Button(String(localized: "Archive Completed")) { state.archiveDone() }
                .buttonStyle(.borderedProminent)
                .padding(.top, 8)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 24)
        .accessibilityElement(children: .contain)
        .onAppear { appeared = true }
    }
}
