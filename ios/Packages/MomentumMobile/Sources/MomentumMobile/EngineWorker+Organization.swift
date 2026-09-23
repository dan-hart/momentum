// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore

public struct ContextCreationResult: Sendable, Equatable {
    public let id: String
    public let created: Bool

    public init(id: String, created: Bool) {
        self.id = id
        self.created = created
    }
}

public enum OrganizationCommand: Sendable {
    case today([String])
    case toggleToday([String])
    case removeToday(String)
    case slot([String], Slot)
    case tomorrow([String])
    case nextWeek([String])
    case project([String], String)
    case tag([String], String)
    case drop([String], MomentumCore.View)
    case reorder([String], before: String, view: MomentumCore.View)
    case nudge(String, Int32, MomentumCore.View)
    case importText(String, MomentumCore.View)
    case updateProject(String, String, String?)
    case updateTag(String, String, String?)
    case deleteProject(String)
    case deleteTag(String)
}

extension EngineWorker {
    public func createProject(_ title: String) -> String? { engine.addProject(title: title) }
    public func createTag(_ title: String) -> String? { engine.addTag(title: title) }
    public func createTagIfAbsent(_ title: String) -> ContextCreationResult? {
        let existingIDs = Set(engine.tags().map(\.id))
        guard let id = engine.addTag(title: title) else { return nil }
        return ContextCreationResult(id: id, created: !existingIDs.contains(id))
    }
    public func projectTaskCount(_ id: String) -> UInt32 { engine.projectTaskCount(id: id) }
    public func tagCompletions(_ prefix: String) -> [TagCompletion] { engine.tagCompletions(prefix: prefix) }
    public func menu(for id: String) -> TaskMenuInfo? { engine.taskMenu(id: id) }

    /// Capture the final imported task before another actor caller can mutate the engine.
    public func importTasksFromText(_ text: String, view: MomentumCore.View) -> TaskCreationResult {
        let outcome = engine.addFromText(text: text, view: view)
        return TaskCreationResult(outcome: outcome, createdTaskID: outcome.changed ? engine.lastAddedId() : nil)
    }

    @discardableResult public func organize(_ command: OrganizationCommand) -> Outcome {
        switch command {
        case .today(let ids): engine.planForToday(ids: ids)
        case .toggleToday(let ids):
            ids.count == 1 ? engine.toggleToday(id: ids[0]) : engine.planForToday(ids: ids)
        case .removeToday(let id): engine.removeFromToday(id: id)
        case .slot(let ids, let slot): engine.toggleSlot(ids: ids, slot: slot)
        case .tomorrow(let ids): engine.moveToTomorrow(ids: ids)
        case .nextWeek(let ids): engine.moveToNextWeek(ids: ids)
        case .project(let ids, let project): engine.moveToProject(ids: ids, projectId: project)
        case .tag(let ids, let name): engine.addTagByName(ids: ids, name: name)
        case .drop(let ids, let view): engine.dropTasks(ids: ids, dest: view)
        case .reorder(let ids, let before, let view): engine.reorderTasks(moved: ids, before: before, view: view)
        case .nudge(let id, let delta, let view): engine.nudge(id: id, delta: delta, view: view)
        case .importText(let text, let view): engine.addFromText(text: text, view: view)
        case .updateProject(let id, let name, let color): engine.updateProject(id: id, title: name, color: color)
        case .updateTag(let id, let name, let color): engine.updateTag(id: id, title: name, color: color)
        case .deleteProject(let id): engine.deleteProject(id: id)
        case .deleteTag(let id): engine.deleteTag(id: id)
        }
    }
}
