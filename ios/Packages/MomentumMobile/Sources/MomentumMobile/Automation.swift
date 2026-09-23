// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import MomentumKit

extension EngineWorker {
    public func automationCreate(title: String, notes: String = "", planning: TaskPlanning = .today,
                                 projectId: String? = nil, dueDate: Date? = nil) throws -> (AutomationTask, Outcome) {
        try EngineAutomation(engine: engine).create(title: title, notes: notes, planning: planning,
                                                   projectId: projectId, dueDate: dueDate)
    }
    public func automationFind(titleContains: String = "", scope: TaskScope = .all,
                               includeCompleted: Bool = false) -> [AutomationTask] {
        EngineAutomation(engine: engine).find(titleContains: titleContains, scope: scope,
                                             includeCompleted: includeCompleted)
    }
    public func automationResolveTasks(ids: [String]) -> [AutomationTask] {
        EngineAutomation(engine: engine).resolveTasks(ids: ids)
    }
    public func automationTasks(ids: [String]) throws -> [AutomationTask] {
        try EngineAutomation(engine: engine).tasks(ids: ids)
    }
    public func automationSetCompleted(ids: [String], completed: Bool) throws -> (Int, Outcome?) {
        try EngineAutomation(engine: engine).setCompleted(ids: ids, completed: completed)
    }
    public func automationPlanToday(ids: [String]) throws -> (Int, Outcome?) {
        try EngineAutomation(engine: engine).planToday(ids: ids)
    }
    public func automationProjects() -> [ProjectRef] { EngineAutomation(engine: engine).projects() }
}
