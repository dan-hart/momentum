// SPDX-License-Identifier: GPL-3.0-or-later
import AppIntents
import Foundation
import MomentumCore
import MomentumKit

struct MomentumTaskEntity: AppEntity {
    static let typeDisplayRepresentation = TypeDisplayRepresentation(name: "Task")
    static let defaultQuery = MomentumTaskQuery()
    let id: String
    @Property(title: "Title") var title: String
    @Property(title: "Notes") var notes: String
    @Property(title: "Project") var project: String
    @Property(title: "Due Date") var dueDate: Date?
    @Property(title: "Completed") var isCompleted: Bool

    var displayRepresentation: DisplayRepresentation {
        .init(title: "\(title)", subtitle: "\(project)",
              image: .init(systemName: isCompleted ? "checkmark.circle" : "circle"))
    }
    init(_ task: AutomationTask) {
        id = task.id; title = task.title; notes = task.notes; project = task.project
        dueDate = task.dueDate; isCompleted = task.isCompleted
    }
}

struct MomentumTaskQuery: EntityStringQuery {
    @MainActor func entities(for identifiers: [String]) async throws -> [MomentumTaskEntity] {
        try await MomentumAutomationRuntime.resolveTasks(ids: identifiers).map(MomentumTaskEntity.init)
    }
    @MainActor func entities(matching string: String) async throws -> [MomentumTaskEntity] {
        try await MomentumAutomationRuntime.find(titleContains: string, includeCompleted: true).map(MomentumTaskEntity.init)
    }
    @MainActor func suggestedEntities() async throws -> [MomentumTaskEntity] {
        // Planning must offer tasks that are not already in Today; the same broad
        // active-task suggestions also keep Complete and Open useful from Shortcuts.
        try await MomentumAutomationRuntime.find(scope: .all).map(MomentumTaskEntity.init)
    }
}

struct MomentumProjectEntity: AppEntity {
    static let typeDisplayRepresentation = TypeDisplayRepresentation(name: "Project")
    static let defaultQuery = MomentumProjectQuery()
    let id: String
    let title: String
    var displayRepresentation: DisplayRepresentation {
        .init(title: "\(title)", image: .init(systemName: "folder"))
    }
}
struct MomentumProjectQuery: EntityStringQuery {
    @MainActor func entities(for identifiers: [String]) async throws -> [MomentumProjectEntity] {
        let projects = try await suggestedEntities()
        return identifiers.compactMap { id in projects.first { $0.id == id } }
    }
    @MainActor func entities(matching string: String) async throws -> [MomentumProjectEntity] {
        try await suggestedEntities().filter { $0.title.localizedCaseInsensitiveContains(string) }
    }
    @MainActor func suggestedEntities() async throws -> [MomentumProjectEntity] {
        try await MomentumAutomationRuntime.projects().map { .init(id: $0.id, title: $0.title) }
    }
}

#if os(macOS)
@MainActor enum MomentumAutomationRuntime {
    static func state() throws -> AppState {
        guard let state = AppDelegate.shared else { throw AutomationError.unavailable }
        return state
    }
    static func automation() throws -> TaskAutomation { .init(state: try state()) }
    static func create(title: String, notes: String, planning: TaskPlanning,
                       projectId: String?, dueDate: Date?) async throws -> AutomationTask {
        try automation().create(title: title, notes: notes, planning: planning, projectId: projectId, dueDate: dueDate)
    }
    static func find(titleContains: String = "", scope: TaskScope = .all,
                     includeCompleted: Bool = false) async throws -> [AutomationTask] {
        try automation().find(titleContains: titleContains, scope: scope, includeCompleted: includeCompleted)
    }
    static func resolveTasks(ids: [String]) async throws -> [AutomationTask] { try automation().resolveTasks(ids: ids) }
    static func tasks(ids: [String]) async throws -> [AutomationTask] { try automation().tasks(ids: ids) }
    static func projects() async throws -> [ProjectRef] { try state().engine.projects() }
    static func setCompleted(ids: [String], completed: Bool) async throws -> Int {
        try automation().setCompleted(ids: ids, completed: completed)
    }
    static func planToday(ids: [String]) async throws -> Int { try automation().planToday(ids: ids) }
    static func openTask(id: String) async throws {
        guard try state().showSearchForTask(id) else { throw AutomationError.taskUnavailable }
        AppDelegate.showMainWindow()
    }
}
#endif

enum ShortcutTaskPlanning: String, AppEnum {
    case today, morning, tonight, tomorrow, unscheduled
    static let typeDisplayRepresentation = TypeDisplayRepresentation(name: "Plan")
    static var caseDisplayRepresentations: [Self: DisplayRepresentation] {
        [.today: "Today", .morning: "Morning", .tonight: "Tonight", .tomorrow: "Tomorrow", .unscheduled: "Unscheduled"]
    }
    var model: TaskPlanning { TaskPlanning(rawValue: rawValue)! }
}
enum ShortcutTaskScope: String, AppEnum {
    case all, today, morning, tonight, upcoming
    static let typeDisplayRepresentation = TypeDisplayRepresentation(name: "Task List")
    static var caseDisplayRepresentations: [Self: DisplayRepresentation] {
        [.all: "All Tasks", .today: "Today", .morning: "Morning", .tonight: "Tonight", .upcoming: "Upcoming"]
    }
    var model: TaskScope { TaskScope(rawValue: rawValue)! }
}
