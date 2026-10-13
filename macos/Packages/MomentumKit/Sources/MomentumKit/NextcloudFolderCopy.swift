// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore

public protocol NextcloudFolderCopying: Sendable {
    func run() async throws -> NextcloudFolderMigrationReport
    @discardableResult func cancel() -> Bool
}

/// Native scheduling/cancellation only; Rust owns safe copying and verification.
public struct NextcloudFolderCopy: NextcloudFolderCopying {
    private let engine: Engine
    private let settings: NextcloudSettings
    private let cancellation = SyncCancellation()

    public init(engine: Engine, settings: NextcloudSettings) {
        self.engine = engine
        self.settings = settings
    }

    public func cancel() -> Bool { cancellation.cancel() }

    public func run() async throws -> NextcloudFolderMigrationReport {
        do {
            return try await withTaskCancellationHandler {
                try await Task.detached(priority: .utility) {
                    try engine.migrateNextcloudFolder(settings: settings, targetFolder: "momentum",
                                                     cancellation: cancellation, timeoutMs: 180_000)
                }.value
            } onCancel: { _ = cancellation.cancel() }
        } catch {
            if cancellation.isCancelled() { throw CancellationError() }
            throw error
        }
    }
}
