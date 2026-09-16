// SPDX-License-Identifier: GPL-3.0-or-later
import AppIntents
import Foundation
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
        let automation = try MomentumAutomationRuntime.automation()
        return identifiers.compactMap { id in
            guard let task = try? automation.tasks(ids: [id]).first else { return nil }
            return MomentumTaskEntity(task)
        }
    }
    @MainActor func entities(matching string: String) async throws -> [MomentumTaskEntity] {
        try MomentumAutomationRuntime.automation().find(titleContains: string, includeCompleted: true).map(MomentumTaskEntity.init)
    }
    @MainActor func suggestedEntities() async throws -> [MomentumTaskEntity] {
        try MomentumAutomationRuntime.automation().find(scope: .today).map(MomentumTaskEntity.init)
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
        try MomentumAutomationRuntime.state().engine.projects().map { .init(id: $0.id, title: $0.title) }
    }
}

@MainActor enum MomentumAutomationRuntime {
    static func state() throws -> AppState {
        guard let state = AppDelegate.shared else { throw AutomationError.unavailable }
        return state
    }
    static func automation() throws -> TaskAutomation { .init(state: try state()) }
}

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
