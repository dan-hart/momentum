// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore

public enum TaskPlanning: String, CaseIterable, Sendable {
    case today, morning, tonight, tomorrow, unscheduled
}

public enum TaskScope: String, CaseIterable, Sendable {
    case all, today, morning, tonight, upcoming
    var view: MomentumCore.View? {
        switch self {
        case .all: nil
        case .today: .today
        case .morning: .morning
        case .tonight: .tonight
        case .upcoming: .upcoming
        }
    }
}

public enum AutomationError: LocalizedError, Equatable {
    case unavailable, emptyTitle, taskUnavailable, projectUnavailable
    case saveFailed(String)
    public var errorDescription: String? {
        switch self {
        case .saveFailed(let error): Strings.message(.saveFailed(error: error))
        case .unavailable: String(localized: "Momentum is not ready. Open the app and try again.", bundle: .module)
        case .emptyTitle: String(localized: "Enter a task title.", bundle: .module)
        case .taskUnavailable: String(localized: "A task was deleted or archived. Choose a current task and try again.", bundle: .module)
        case .projectUnavailable: String(localized: "This project is no longer available. Choose another project.", bundle: .module)
        }
    }
}

public struct AutomationTask: Sendable, Equatable, Identifiable {
    public let id: String
    public let title: String
    public let notes: String
    public let project: String
    public let dueDate: Date?
    public let isCompleted: Bool

    public init(id: String, title: String, notes: String, project: String, dueDate: Date?, isCompleted: Bool) {
        self.id = id
        self.title = title
        self.notes = notes
        self.project = project
        self.dueDate = dueDate
        self.isCompleted = isCompleted
    }
}

/// Shared native parameter mapping for automation; Rust owns task rules and writes.
/// Execute on the platform's serialized engine owner, never concurrently across actors.
public struct EngineAutomation {
    private let engine: Engine
    public init(engine: Engine) { self.engine = engine }

    public func create(title: String, notes: String = "", planning: TaskPlanning = .today,
                       projectId: String? = nil, dueDate: Date? = nil) throws -> (AutomationTask, Outcome) {
        let parsed = parseQuickAdd(text: title)
        guard !parsed.title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { throw AutomationError.emptyTitle }
        if let projectId, !engine.projects().contains(where: { $0.id == projectId }) {
            throw AutomationError.projectUnavailable
        }
        let view: MomentumCore.View = switch planning {
        case .today: .today
        case .morning: .morning
        case .tonight: .tonight
        case .tomorrow, .unscheduled: .search
        }
        let day: String? = dueDate.map(Strings.day(fromDate:)) ?? {
            switch planning {
            case .today, .morning, .tonight: today()
            case .tomorrow: dayOffset(day: today(), days: 1)
            case .unscheduled: nil
            }
        }()
        let draft = TaskDraft(title: parsed.title, projectId: projectId ?? "", dueDay: day,
                              time: nil, reminderMinutesBefore: nil, estimateMs: parsed.estimateMs,
                              notes: notes, tagIds: [], newTags: parsed.tags)
        let creation = engine.createTaskWithId(draft: draft, view: view)
        try checkSave(creation.outcome)
        guard creation.outcome.changed, let id = creation.id, let task = snapshot(id) else {
            throw AutomationError.unavailable
        }
        return (task, creation.outcome)
    }

    /// The filter intentionally says Title Contains; view membership stays in the core.
    public func find(titleContains: String = "", scope: TaskScope = .all,
                     includeCompleted: Bool = false) -> [AutomationTask] {
        let query = titleContains.trimmingCharacters(in: .whitespacesAndNewlines)
        let ids: [String]
        if scope == .upcoming {
            ids = engine.upcomingTaskIds(includeCompleted: includeCompleted)
        } else if let view = scope.view {
            ids = engine.listing(view: view, archiveLimit: 0).sections.flatMap { section in
                section.rows.compactMap { row in
                    if case .task(let task) = row { return task.id }
                    return nil
                }
            }
        } else {
            ids = engine.allTasks().map(\.id)
        }
        var seen = Set<String>()
        return ids.filter { seen.insert($0).inserted }.compactMap(snapshot).filter {
            (includeCompleted || !$0.isCompleted) && (query.isEmpty || $0.title.localizedCaseInsensitiveContains(query))
        }
    }

    /// Entity queries omit stale identifiers; mutation validation remains strict.
    public func resolveTasks(ids: [String]) -> [AutomationTask] {
        var seen = Set<String>()
        return ids.filter { seen.insert($0).inserted }.compactMap(snapshot)
    }

    public func tasks(ids: [String]) throws -> [AutomationTask] {
        var seen = Set<String>()
        return try ids.filter { seen.insert($0).inserted }.map {
            guard let task = snapshot($0) else { throw AutomationError.taskUnavailable }
            return task
        }
    }

    public func setCompleted(ids: [String], completed: Bool) throws -> (Int, Outcome?) {
        let changed = try tasks(ids: ids).filter { $0.isCompleted != completed }.map(\.id)
        guard !changed.isEmpty else { return (0, nil) }
        let outcome = completed
            ? (changed.count == 1 ? engine.toggleDone(id: changed[0]) : engine.bulkDone(ids: changed))
            : engine.reopenTasks(ids: changed)
        try checkSave(outcome)
        return (outcome.changed ? changed.count : 0, outcome)
    }

    public func planToday(ids: [String]) throws -> (Int, Outcome?) {
        let changed = try tasks(ids: ids).filter {
            guard let detail = engine.taskDetail(id: $0.id) else { return false }
            return detail.dueDay != today() || detail.time != nil
        }.map(\.id)
        guard !changed.isEmpty else { return (0, nil) }
        let outcome = engine.planForToday(ids: changed)
        try checkSave(outcome)
        return (outcome.changed ? changed.count : 0, outcome)
    }

    public func projects() -> [ProjectRef] { engine.projects() }

    private func checkSave(_ outcome: Outcome) throws {
        if let message = outcome.message, case .saveFailed(let error) = message {
            throw AutomationError.saveFailed(error)
        }
    }

    private func snapshot(_ id: String) -> AutomationTask? {
        guard let detail = engine.taskDetail(id: id) else { return nil }
        var date = detail.dueDay.flatMap(Strings.date(fromDay:))
        if let time = detail.time, let day = date {
            date = Calendar.current.date(bySettingHour: Int(time.hour), minute: Int(time.minute), second: 0, of: day)
        }
        return AutomationTask(id: id, title: detail.title, notes: detail.notes,
                              project: engine.projects().first(where: { $0.id == detail.projectId })?.title ?? "",
                              dueDate: date, isCompleted: detail.isDone)
    }
}
#if os(macOS)
/// Preserve the desktop refresh/index/undo/sync path and public automation API.
@MainActor public struct TaskAutomation {
    public let state: AppState
    public init(state: AppState) { self.state = state }
    private var automation: EngineAutomation { .init(engine: state.engine) }

    public func create(title: String, notes: String = "", planning: TaskPlanning = .today,
                       projectId: String? = nil, dueDate: Date? = nil) throws -> AutomationTask {
        let (task, outcome) = try automation.create(title: title, notes: notes, planning: planning,
                                                  projectId: projectId, dueDate: dueDate)
        state.apply(outcome)
        return task
    }
    public func find(titleContains: String = "", scope: TaskScope = .all,
                     includeCompleted: Bool = false) -> [AutomationTask] {
        automation.find(titleContains: titleContains, scope: scope, includeCompleted: includeCompleted)
    }
    public func tasks(ids: [String]) throws -> [AutomationTask] { try automation.tasks(ids: ids) }
    public func resolveTasks(ids: [String]) -> [AutomationTask] { automation.resolveTasks(ids: ids) }
    public func setCompleted(ids: [String], completed: Bool) throws -> Int {
        let (count, outcome) = try automation.setCompleted(ids: ids, completed: completed)
        if let outcome { state.apply(outcome) }
        return count
    }
    public func planToday(ids: [String]) throws -> Int {
        let (count, outcome) = try automation.planToday(ids: ids)
        if let outcome { state.apply(outcome) }
        return count
    }
}
#endif
