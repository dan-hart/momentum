// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore
import MomentumKit
import MomentumMobile
import SwiftUI
import UIKit

struct TaskActionsMenu: SwiftUI.View {
    @Environment(MobileAppModel.self) private var model
    let ids: [String]
    var row: TaskRow?
    var projects: [ProjectRef] = []
    var tags: [TagRef] = []
    var onEdit: (() -> Void)?
    var onNewTag: (() -> Void)?
    var onFinished: (TaskSelectionTransitionFeedback) -> Void = { _ in }

    /// The clicked row's facts when the menu is for one task: the core's slot rule and
    /// today's date decide which labels toggle back.
    private var plannedToday: Bool { row.map { $0.dueDay == today() } ?? false }
    private var slot: Slot? { row.flatMap { slotOfTagNames(names: $0.tags.map(\.title)) } }

    var body: some SwiftUI.View {
        // Act
        if let onEdit { Button("Open", systemImage: "square.and.pencil", action: onEdit) }
        Button(row?.isDone == true ? "Mark as Not Done" : "Mark as Done", systemImage: row?.isDone == true ? "arrow.uturn.backward.circle" : "checkmark.circle") {
            act(.complete(ids, row?.isDone != true))
        }
        Divider()
        // Plan: Today, then every destination in one Move To submenu.
        if plannedToday, ids.count == 1 {
            Button("Remove from Today", systemImage: "star.slash") { act(.removeToday(ids[0])) }
        } else {
            Button("Plan for Today", systemImage: "star") { act(.today(ids)) }
        }
        Menu("Move To", systemImage: "arrow.turn.down.right") {
            Button(slot == .morning ? "Today" : "Morning", systemImage: slot == .morning ? "star" : "sun.max") {
                act(.slot(ids, .morning))
            }
            Button(slot == .tonight ? "Today" : "Evening", systemImage: slot == .tonight ? "star" : "moon.stars") {
                act(.slot(ids, .tonight))
            }
            Divider()
            Button("Tomorrow", systemImage: "sunrise") { act(.tomorrow(ids)) }
            // The two days after tomorrow, by weekday name.
            ForEach([Int64(2), 3], id: \.self) { days in
                let day = dayOffset(day: today(), days: days)
                Button(Strings.day(dayLabel(day: day)), systemImage: "calendar") { act(.day(ids, day)) }
            }
            Button("Next Week", systemImage: "calendar.badge.clock") { act(.nextWeek(ids)) }
            if !projects.isEmpty {
                Divider()
                Menu("Project", systemImage: "folder") {
                    ForEach(projects, id: \.id) { project in
                        Button(project.title) { act(.project(ids, project.id)) }
                    }
                }
            }
        }
        Divider()
        // Organise
        Menu("Add Tag", systemImage: "tag") {
            ForEach(tags, id: \.id) { tag in
                Button(tag.title) { act(.tag(ids, tag.title)) }
            }
            if let onNewTag {
                if !tags.isEmpty { Divider() }
                Button("New Tag…", systemImage: "tag.badge.plus", action: onNewTag)
            }
        }
        if let row {
            Button("Copy Title", systemImage: "doc.on.doc") {
                UIPasteboard.general.string = row.title
                model.showFeedback(String(localized: "Title copied"))
            }
        }
        Divider()
        Button("Delete", systemImage: "trash", role: .destructive) {
            act(.delete(ids))
        }
    }

    private func act(_ command: TaskCommand) {
        Task {
            let outcome = await model.perform(command)
            onFinished(.afterTaskCommand(command, outcome: outcome))
        }
    }

    private func act(_ command: OrganizationCommand) {
        Task {
            let outcome = await model.organize(command)
            onFinished(.afterOrganization(command, outcome: outcome))
        }
    }
}

/// One tap chooses the destination; task-family rules and Undo stay in the core.
struct MoveToProjectSheet: SwiftUI.View {
    @Environment(MobileAppModel.self) private var model
    @Environment(\.dismiss) private var dismiss
    let ids: [String]
    let projects: [ProjectRef]
    var onMoved: () -> Void = {}
    @State private var movingID: String?
    @State private var error: String?
    @AccessibilityFocusState private var errorFocused: Bool

    var body: some SwiftUI.View {
        NavigationStack {
            List {
                if let error {
                    AccessibleErrorMessage(error)
                        .accessibilityFocused($errorFocused)
                        .accessibilityIdentifier("move-project-error")
                }
                ForEach(projects, id: \.id) { project in
                    Button {
                        guard movingID == nil else { return }
                        movingID = project.id
                        error = nil
                        Task {
                            let outcome = await model.organize(.project(ids, project.id))
                            movingID = nil
                            if let outcome, outcome.changed || outcome.message == nil {
                                if outcome.changed { onMoved() }
                                dismiss()
                            } else {
                                error = outcome?.message.map(Strings.message)
                                    ?? String(localized: "The tasks could not be moved.")
                            }
                        }
                    } label: {
                        HStack(alignment: .firstTextBaseline, spacing: 12) {
                            Image(systemName: "folder").accessibilityHidden(true)
                            Text(project.title)
                                .fixedSize(horizontal: false, vertical: true)
                                .frame(maxWidth: .infinity, alignment: .leading)
                            if movingID == project.id { Spacer(); ProgressView() }
                        }
                        .frame(minHeight: 44)
                    }
                    .foregroundStyle(.primary)
                    .accessibilityLabel(project.title)
                    .disabled(movingID != nil)
                }
            }
            .navigationTitle("Move to Project")
            .navigationBarTitleDisplayMode(.inline)
            .momentumNavigationCanvas()
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel", systemImage: "xmark") { dismiss() }
                        .disabled(movingID != nil)
                }
            }
            .interactiveDismissDisabled(movingID != nil)
            .onChange(of: error) { _, value in if value != nil { errorFocused = true } }
        }
    }
}
