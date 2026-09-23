// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore

/// Value snapshots cross the actor boundary; Rust remains the sole owner of task rules.
public struct TaskSnapshot: Sendable {
    public let listing: Listing
    public let sidebar: Sidebar
    public let projects: [ProjectRef]
    public let tags: [TagRef]
    public let sync: SyncStatus
    public let canUndo: Bool
    public let viewExists: Bool
    /// Native presentation context, including live or archived parents omitted by search.
    public let parentTitles: [String: String]

    /// Reuse the ordered projection made on the worker, rather than rebuilding it
    /// for each selection, keyboard or drag calculation during a view update.
    public let tasks: [TaskRow]
}

public enum TaskCommand: Sendable {
    case add(String, MomentumCore.View)
    case complete([String], Bool)
    case delete([String])
    case archive
    case undo
}

/// Serializes local queries and mutations off the main actor. Network exchanges must
/// use a separate utility executor and the core commit barrier, never this actor queue.
public actor EngineWorker {
    let engine: Engine
    var nearbyTransport: CoreNearbyRuntime?
    var handledNotificationActions: [String: Set<TaskNotificationAction>] = [:]

    private init(engine: Engine) { self.engine = engine }

    public static func open(directory: URL, demo: Bool = false) async -> EngineWorker {
        await Task.detached(priority: .userInitiated) {
            let engine = demo ? Engine.demo(dir: directory.path) : Engine.open(dir: directory.path)
            _ = engine.spawnRepeats()
            return EngineWorker(engine: engine)
        }.value
    }

    public static func openChecked(directory: URL, demo: Bool = false) async throws -> EngineWorker {
        try await Task.detached(priority: .userInitiated) {
            let engine = try demo ? Engine.demo(dir: directory.path) : Engine.openChecked(dir: directory.path)
            _ = engine.spawnRepeats()
            return EngineWorker(engine: engine)
        }.value
    }

    public func snapshot(view: MomentumCore.View, query: String = "", archiveLimit: UInt32 = 100) -> TaskSnapshot {
        let listing = view == .search ? engine.search(query: query)
            : engine.listing(view: view, archiveLimit: archiveLimit)
        let rows = listing.sections.flatMap(\.rows).compactMap { row -> TaskRow? in
            if case .task(let task) = row { return task }
            return nil
        }
        let visibleTitles = Dictionary(rows.map { ($0.id, $0.title) }, uniquingKeysWith: { first, _ in first })
        var parentTitles: [String: String] = [:]
        let parentIDs = Set(rows.compactMap(\.parentId))
        let hiddenIDs = parentIDs.filter { visibleTitles[$0] == nil }
        // One read-only core query for hidden parents, including both archive tiers.
        let hiddenTitles = hiddenIDs.isEmpty ? [:] : engine.taskReferenceTitles(ids: Array(hiddenIDs))
        for parentID in parentIDs {
            parentTitles[parentID] = visibleTitles[parentID] ?? hiddenTitles[parentID]
        }
        return TaskSnapshot(listing: listing,
                     sidebar: engine.sidebar(), projects: engine.projects(), tags: engine.tags(),
                     sync: engine.syncStatus(), canUndo: engine.canUndo(),
                     viewExists: engine.viewExists(view: view), parentTitles: parentTitles, tasks: rows)
    }

    @discardableResult
    public func perform(_ command: TaskCommand) -> Outcome {
        switch command {
        case .add(let text, let view): engine.addTask(text: text, view: view)
        case .complete(let ids, let done): done ? engine.bulkDone(ids: ids) : engine.reopenTasks(ids: ids)
        case .delete(let ids): engine.bulkDelete(ids: ids)
        case .archive: engine.archiveDone()
        case .undo: engine.undo()
        }
    }

    public func undo(batchID: UInt64) -> Outcome {
        engine.undoBatch(id: batchID)
    }

    public func setPreferences(_ preferences: MomentumCore.Preferences) {
        engine.setPreferences(prefs: preferences)
    }

    public func mobileSearchDocuments() -> [MobileSearchDocument] {
        MobileSearchDocument.project(engine.allTasks())
    }

    public func taskTitle(_ id: String) -> String? {
        engine.taskTitle(id: id)
    }
}
