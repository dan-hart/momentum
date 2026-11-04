// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
import Foundation
import MomentumCore
import Observation

/// A read-only connection check, independent of task-sync success and pending edits.
/// Cancel invalidates the result immediately, even if a transport returns late.
@MainActor @Observable public final class NextcloudConnectionTest {
    public private(set) var isRunning = false
    public private(set) var succeeded: Bool?
    public private(set) var error: String?
    private var cancellation: SyncCancellation?
    private var generation = UUID()
    private let check: @Sendable (Engine, NextcloudSettings, SyncCancellation) throws -> Void

    public init(check: @escaping @Sendable (Engine, NextcloudSettings, SyncCancellation) throws -> Void = { engine, settings, cancellation in
        try engine.testNextcloudConnectionCancellable(settings: settings, cancellation: cancellation, timeoutMs: 15_000)
    }) {
        self.check = check
    }

    public func start(engine: Engine, settings: NextcloudSettings) {
        cancel()
        let token = generation
        let cancellation = SyncCancellation()
        self.cancellation = cancellation
        isRunning = true
        let check = self.check
        Task { [weak self] in
            let result = await Task.detached(priority: .utility) {
                Result { try check(engine, settings, cancellation) }
            }.value
            guard let self, generation == token else { return }
            isRunning = false
            self.cancellation = nil
            switch result {
            case .success: succeeded = true
            case .failure(let failure):
                succeeded = false
                error = Strings.error(failure)
            }
        }
    }

    public func cancel() {
        _ = cancellation?.cancel()
        cancellation = nil
        generation = UUID()
        isRunning = false
        succeeded = nil
        error = nil
    }
}
