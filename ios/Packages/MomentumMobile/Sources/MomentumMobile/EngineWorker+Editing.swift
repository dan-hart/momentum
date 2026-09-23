// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore

public struct TaskCreationResult: Sendable, Equatable {
    public let outcome: Outcome
    public let createdTaskID: String?

    public init(outcome: Outcome, createdTaskID: String?) {
        self.outcome = outcome
        self.createdTaskID = createdTaskID
    }
}

extension EngineWorker {
    public func taskDetail(_ id: String) -> TaskDetail? { engine.taskDetail(id: id) }
    public func taskMenu(_ id: String) -> TaskMenuInfo? { engine.taskMenu(id: id) }
    public func editingProjects() -> [ProjectRef] { engine.projects() }
    public func editingTags() -> [TagRef] { engine.tags() }
    public func lastAddedTaskID() -> String? { engine.lastAddedId() }

    public func createTask(_ draft: TaskDraft, view: MomentumCore.View) -> Outcome {
        engine.createTask(draft: draft, view: view)
    }
    /// Capture Quick Add identity before another actor caller can mutate the engine.
    public func createTaskFromText(_ text: String, view: MomentumCore.View) -> TaskCreationResult {
        let outcome = engine.addTask(text: text, view: view)
        return TaskCreationResult(outcome: outcome, createdTaskID: outcome.changed ? engine.lastAddedId() : nil)
    }
    public func saveTask(_ id: String, draft: TaskDraft) -> Outcome {
        engine.saveTask(id: id, draft: draft)
    }
    public func addSubtask(_ id: String, title: String) -> Outcome {
        engine.addSubtask(parentId: id, title: title)
    }
    public func duplicateTask(_ id: String) -> Outcome { engine.duplicateTask(id: id) }
    /// Capture the created identity before another actor caller can add a task.
    public func duplicateTaskWithID(_ id: String) -> (outcome: Outcome, taskID: String?) {
        let outcome = engine.duplicateTask(id: id)
        return (outcome, outcome.changed ? engine.lastAddedId() : nil)
    }
    public func deleteTask(_ id: String) -> Outcome { engine.deleteTask(id: id) }
    public func repeatDraft(_ id: String) -> RepeatDraft? { engine.repeatDraft(taskId: id) }
    public func repeatDescription(_ draft: RepeatDraft) -> RepeatDescription {
        engine.describeRepeatDraft(draft: draft)
    }
    public func saveRepeat(_ id: String, draft: RepeatDraft) -> Outcome {
        engine.saveRepeat(taskId: id, draft: draft)
    }
    public func stopRepeat(_ id: String) -> Outcome { engine.stopRepeat(taskId: id) }
}
