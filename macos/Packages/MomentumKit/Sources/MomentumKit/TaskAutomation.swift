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
    public var errorDescription: String? {
        switch self {
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
}

/// The testable boundary for system automation. Every write goes through AppState's
/// ordinary refresh/index/undo/sync path. Never changes the user's selected list.
@MainActor public struct TaskAutomation {
    public let state: AppState
    public init(state: AppState) { self.state = state }

    public func create(title: String, notes: String = "", planning: TaskPlanning = .today,
                       projectId: String? = nil, dueDate: Date? = nil) throws -> AutomationTask {
        let parsed = parseQuickAdd(text: title)
        guard !parsed.title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { throw AutomationError.emptyTitle }
        if let projectId, !state.engine.projects().contains(where: { $0.id == projectId }) {
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
        let creation = state.engine.createTaskWithId(draft: draft, view: view)
        state.apply(creation.outcome)
        guard creation.outcome.changed, let id = creation.id, let task = snapshot(id) else {
            throw AutomationError.unavailable
        }
        return task
    }

    /// The filter intentionally says Title Contains; view membership stays in the core.
    public func find(titleContains: String = "", scope: TaskScope = .all,
                     includeCompleted: Bool = false) -> [AutomationTask] {
        let query = titleContains.trimmingCharacters(in: .whitespacesAndNewlines)
        let ids: [String]
        if scope == .upcoming {
            ids = state.engine.upcomingTaskIds(includeCompleted: includeCompleted)
        } else if let view = scope.view {
            ids = state.engine.listing(view: view, archiveLimit: 0).sections.flatMap { section in
                section.rows.compactMap { row in
                    if case .task(let task) = row { return task.id }
                    return nil
                }
            }
        } else {
            ids = state.engine.allTasks().map(\.id)
        }
        var seen = Set<String>()
        return ids.filter { seen.insert($0).inserted }.compactMap(snapshot).filter {
            (includeCompleted || !$0.isCompleted) && (query.isEmpty || $0.title.localizedCaseInsensitiveContains(query))
        }
    }

    public func tasks(ids: [String]) throws -> [AutomationTask] {
        var seen = Set<String>()
        return try ids.filter { seen.insert($0).inserted }.map {
            guard let task = snapshot($0) else { throw AutomationError.taskUnavailable }
            return task
        }
    }

    public func setCompleted(ids: [String], completed: Bool) throws -> Int {
        let changed = try tasks(ids: ids).filter { $0.isCompleted != completed }.map(\.id)
        if completed {
            state.toggleDone(changed)
        } else if !changed.isEmpty {
            state.apply(state.engine.reopenTasks(ids: changed))
        }
        return changed.count
    }

    public func planToday(ids: [String]) throws -> Int {
        let changed = try tasks(ids: ids).filter {
            guard let detail = state.engine.taskDetail(id: $0.id) else { return false }
            return detail.dueDay != today() || detail.time != nil
        }.map(\.id)
        if !changed.isEmpty { state.apply(state.engine.planForToday(ids: changed)) }
        return changed.count
    }

    private func snapshot(_ id: String) -> AutomationTask? {
        guard let detail = state.engine.taskDetail(id: id) else { return nil }
        var date = detail.dueDay.flatMap(Strings.date(fromDay:))
        if let time = detail.time, let day = date {
            date = Calendar.current.date(bySettingHour: Int(time.hour), minute: Int(time.minute), second: 0, of: day)
        }
        return AutomationTask(id: id, title: detail.title, notes: detail.notes,
                              project: state.engine.projects().first(where: { $0.id == detail.projectId })?.title ?? "",
                              dueDate: date, isCompleted: detail.isDone)
    }
}
