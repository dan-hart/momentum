// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation

public enum MobileLifecycle {
    public static func nextDay(after now: Date, calendar: Calendar = .autoupdatingCurrent) -> Date? {
        calendar.date(byAdding: .day, value: 1, to: calendar.startOfDay(for: now))
    }
}

/// UI, notification actions and app-hosted intents share one bootstrap task.
/// A session owns one immutable store configuration; use separate sessions for test stores.
public actor EngineSession {
    private var opening: Task<EngineWorker, Error>?
    public init() {}

    public func open(directory: URL, demo: Bool = false) async throws -> EngineWorker {
        if let opening { return try await opening.value }
        let task = Task { try await EngineWorker.openChecked(directory: directory, demo: demo) }
        opening = task
        do { return try await task.value }
        catch {
            // Only the creator clears this attempt; attached callers cannot erase a newer retry.
            opening = nil
            throw error
        }
    }
}

extension EngineWorker {
    @discardableResult public func refreshForForeground() -> Bool {
        let dayChanged = engine.dayChanged()
        let spawned = engine.spawnRepeats()
        return dayChanged || spawned > 0
    }
}
