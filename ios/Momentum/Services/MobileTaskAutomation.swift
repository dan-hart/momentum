// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import MomentumKit
import MomentumMobile

/// App-hosted automation uses the UI's existing engine and refresh path, including
/// cold background invocations. No extension or second store owner is created.
@MainActor struct MobileTaskAutomation {
    let model: MobileAppModel

    private func worker() async throws -> EngineWorker {
        await model.start()
        guard let worker = model.worker else { throw AutomationError.unavailable }
        if await worker.refreshForForeground() { model.refreshAfterEdit() }
        return worker
    }

    func create(title: String, notes: String = "", planning: TaskPlanning = .today,
                projectId: String? = nil, dueDate: Date? = nil) async throws -> AutomationTask {
        let worker = try await worker()
        let (task, outcome) = try await worker.automationCreate(title: title, notes: notes, planning: planning,
                                                               projectId: projectId, dueDate: dueDate)
        await accept(outcome)
        return task
    }
    func find(titleContains: String = "", scope: TaskScope = .all,
              includeCompleted: Bool = false) async throws -> [AutomationTask] {
        let worker = try await worker()
        return await worker.automationFind(titleContains: titleContains, scope: scope, includeCompleted: includeCompleted)
    }
    func resolveTasks(ids: [String]) async throws -> [AutomationTask] {
        let worker = try await worker()
        return await worker.automationResolveTasks(ids: ids)
    }
    func tasks(ids: [String]) async throws -> [AutomationTask] {
        let worker = try await worker()
        return try await worker.automationTasks(ids: ids)
    }
    func projects() async throws -> [ProjectRef] {
        let worker = try await worker()
        return await worker.automationProjects()
    }
    func setCompleted(ids: [String], completed: Bool) async throws -> Int {
        let worker = try await worker()
        let (count, outcome) = try await worker.automationSetCompleted(ids: ids, completed: completed)
        if let outcome { await accept(outcome) }
        return count
    }
    func planToday(ids: [String]) async throws -> Int {
        let worker = try await worker()
        let (count, outcome) = try await worker.automationPlanToday(ids: ids)
        if let outcome { await accept(outcome) }
        return count
    }
    func openTask(id: String) async throws { try await model.openAutomationTask(id) }

    private func accept(_ outcome: Outcome) async {
        model.accept(outcome)
        // Finish the coalesced scheduling drain before a background intent returns.
        await model.notifications.refresh()
    }
}

@MainActor enum MomentumAutomationRuntime {
    static var automation: MobileTaskAutomation { .init(model: .shared) }
    static func create(title: String, notes: String, planning: TaskPlanning,
                       projectId: String?, dueDate: Date?) async throws -> AutomationTask {
        try await automation.create(title: title, notes: notes, planning: planning, projectId: projectId, dueDate: dueDate)
    }
    static func find(titleContains: String = "", scope: TaskScope = .all,
                     includeCompleted: Bool = false) async throws -> [AutomationTask] {
        try await automation.find(titleContains: titleContains, scope: scope, includeCompleted: includeCompleted)
    }
    static func resolveTasks(ids: [String]) async throws -> [AutomationTask] { try await automation.resolveTasks(ids: ids) }
    static func tasks(ids: [String]) async throws -> [AutomationTask] { try await automation.tasks(ids: ids) }
    static func projects() async throws -> [ProjectRef] { try await automation.projects() }
    static func setCompleted(ids: [String], completed: Bool) async throws -> Int {
        try await automation.setCompleted(ids: ids, completed: completed)
    }
    static func planToday(ids: [String]) async throws -> Int { try await automation.planToday(ids: ids) }
    static func openTask(id: String) async throws { try await automation.openTask(id: id) }
}
