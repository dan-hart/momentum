// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
import AppIntents
import Foundation

/// Keep the original intent and title parameter identifiers for existing shortcuts.
struct CreateTaskIntent: AppIntent {
    static let title: LocalizedStringResource = "Create Task"
    static let description = IntentDescription("Create a Momentum task and return it to the next action. Supports #tags and estimates. An optional due date overrides the plan's date.")
    static var supportedModes: IntentModes { .background }
    @Parameter(title: "Title") var taskTitle: String
    @Parameter(title: "Notes") var notes: String?
    @Parameter(title: "Plan", default: .today) var planning: ShortcutTaskPlanning
    @Parameter(title: "Project") var project: MomentumProjectEntity?
    @Parameter(title: "Due Date", kind: .date) var dueDate: Date?
    static var parameterSummary: some ParameterSummary {
        Summary("Create \(\.$taskTitle) for \(\.$planning)") {
            \.$notes
            \.$project
            \.$dueDate
        }
    }
    @MainActor func perform() async throws -> some IntentResult & ReturnsValue<MomentumTaskEntity> {
        let task = try MomentumAutomationRuntime.automation().create(title: taskTitle, notes: notes ?? "",
            planning: planning.model, projectId: project?.id, dueDate: dueDate)
        return .result(value: MomentumTaskEntity(task))
    }
}
