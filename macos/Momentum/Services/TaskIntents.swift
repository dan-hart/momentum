// SPDX-License-Identifier: GPL-3.0-or-later
import AppIntents
import MomentumKit

struct FindTasksIntent: AppIntent {
    static let title: LocalizedStringResource = "Find Tasks"
    static let description = IntentDescription("Find current tasks by list and title. Today includes overdue tasks. Archived tasks are excluded.")
    static var supportedModes: IntentModes { .background }
    @Parameter(title: "List", default: .all) var scope: ShortcutTaskScope
    @Parameter(title: "Title Contains") var titleContains: String?
    @Parameter(title: "Include Completed", default: false) var includeCompleted: Bool
    static var parameterSummary: some ParameterSummary {
        Summary("Find tasks in \(\.$scope)") {
            \.$titleContains
            \.$includeCompleted
        }
    }
    @MainActor func perform() async throws -> some IntentResult & ReturnsValue<[MomentumTaskEntity]> {
        let tasks = try MomentumAutomationRuntime.automation().find(titleContains: titleContains ?? "",
            scope: scope.model, includeCompleted: includeCompleted)
        return .result(value: tasks.map(MomentumTaskEntity.init))
    }
}

struct SetTaskCompletedIntent: AppIntent {
    static let title: LocalizedStringResource = "Set Task Completed"
    static let description = IntentDescription("Complete or reopen current tasks and return the number changed. Respects automatic archiving; archived tasks are read-only.")
    static var supportedModes: IntentModes { .background }
    @Parameter(title: "Tasks") var tasks: [MomentumTaskEntity]
    @Parameter(title: "Completed", default: true) var completed: Bool
    static var parameterSummary: some ParameterSummary {
        Summary("Set \(\.$tasks) completed to \(\.$completed)")
    }
    @MainActor func perform() async throws -> some IntentResult & ReturnsValue<Int> {
        .result(value: try MomentumAutomationRuntime.automation().setCompleted(ids: tasks.map(\.id), completed: completed))
    }
}

struct PlanTasksForTodayIntent: AppIntent {
    static let title: LocalizedStringResource = "Plan Tasks for Today"
    static let description = IntentDescription("Plan current tasks for today and return the number changed.")
    static var supportedModes: IntentModes { .background }
    @Parameter(title: "Tasks") var tasks: [MomentumTaskEntity]
    static var parameterSummary: some ParameterSummary { Summary("Plan \(\.$tasks) for today") }
    @MainActor func perform() async throws -> some IntentResult & ReturnsValue<Int> {
        .result(value: try MomentumAutomationRuntime.automation().planToday(ids: tasks.map(\.id)))
    }
}

struct OpenTaskIntent: AppIntent {
    static let title: LocalizedStringResource = "Open Task"
    static let description = IntentDescription("Show a task in Momentum.")
    static var supportedModes: IntentModes { .foreground(.immediate) }
    @Parameter(title: "Task") var task: MomentumTaskEntity
    static var parameterSummary: some ParameterSummary { Summary("Open \(\.$task)") }
    @MainActor func perform() async throws -> some IntentResult {
        let state = try MomentumAutomationRuntime.state()
        guard state.showSearchForTask(task.id) else { throw AutomationError.taskUnavailable }
        AppDelegate.showMainWindow()
        return .result()
    }
}

struct MomentumShortcuts: AppShortcutsProvider {
    static var appShortcuts: [AppIntents.AppShortcut] {
        AppShortcut(intent: CreateTaskIntent(), phrases: ["Create a task in \(.applicationName)"],
                    shortTitle: "Create Task", systemImageName: "plus.circle")
        AppShortcut(intent: FindTasksIntent(), phrases: ["Find tasks in \(.applicationName)"],
                    shortTitle: "Find Tasks", systemImageName: "checklist")
        AppShortcut(intent: SetTaskCompletedIntent(), phrases: ["Complete tasks in \(.applicationName)"],
                    shortTitle: "Complete Tasks", systemImageName: "checkmark.circle")
        AppShortcut(intent: PlanTasksForTodayIntent(), phrases: ["Plan tasks in \(.applicationName)"],
                    shortTitle: "Plan for Today", systemImageName: "calendar")
        AppShortcut(intent: OpenTaskIntent(), phrases: ["Open a task in \(.applicationName)"],
                    shortTitle: "Open Task", systemImageName: "arrow.up.forward.app")
    }
}
