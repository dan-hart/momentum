// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore

public protocol NextcloudRunningOperation: Sendable {
    func run() async throws -> SyncReport
    @discardableResult func cancel() -> Bool
}

public protocol NextcloudConnectionTestOperation: Sendable {
    func run() async throws
    @discardableResult func cancel() -> Bool
}

/// A read-only WebDAV probe with an independent cancellation token. Testing a
/// saved connection never starts the task exchange or mutates sync state.
public struct NextcloudConnectionOperation: NextcloudConnectionTestOperation {
    let engine: Engine
    let settings: NextcloudSettings
    let timeoutMs: UInt64
    private let cancellation = SyncCancellation()

    @discardableResult public func cancel() -> Bool { cancellation.cancel() }

    public func run() async throws {
        do {
            try await withTaskCancellationHandler {
                try await Task.detached(priority: .utility) {
                    try engine.testNextcloudConnectionCancellable(
                        settings: settings,
                        cancellation: cancellation,
                        timeoutMs: timeoutMs
                    )
                }.value
            } onCancel: { _ = cancellation.cancel() }
        } catch {
            if cancellation.isCancelled() { throw CancellationError() }
            throw error
        }
    }
}

/// One operation shares the existing engine but never occupies its local actor queue
/// during network I/O. Its core handle also covers cancellation before FFI admission.
public struct NextcloudOperation: NextcloudRunningOperation {
    let engine: Engine
    let settings: NextcloudSettings
    private let cancellation = SyncCancellation()

    @discardableResult public func cancel() -> Bool { cancellation.cancel() }

    public func run() async throws -> SyncReport {
        do {
            return try await withTaskCancellationHandler {
                try await Task.detached(priority: .utility) {
                    try engine.syncNextcloudCancellable(settings: settings, cancellation: cancellation)
                }.value
            } onCancel: { _ = cancellation.cancel() }
        } catch {
            if cancellation.isCancelled() { throw CancellationError() }
            throw error
        }
        // A successful commit wins over a cancellation arriving after its boundary.
        // Never check Task.isCancelled after success or conceal committed task changes.
    }
}

extension EngineWorker {
    public func syncStatus() -> SyncStatus { engine.syncStatus() }

    public func nextcloudOperation(settings: NextcloudSettings) -> NextcloudOperation {
        NextcloudOperation(engine: engine, settings: settings)
    }

    public func nextcloudConnectionTestOperation(
        settings: NextcloudSettings,
        timeoutMs: UInt64 = 30_000
    ) -> NextcloudConnectionOperation {
        NextcloudConnectionOperation(engine: engine, settings: settings, timeoutMs: timeoutMs)
    }
}
