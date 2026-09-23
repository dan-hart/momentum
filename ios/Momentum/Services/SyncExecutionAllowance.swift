// SPDX-License-Identifier: GPL-3.0-or-later
import UIKit

/// Requests bounded time before transport starts. Expiration always ends the OS
/// assertion promptly; Rust retains its cancellation/commit barrier while draining.
@MainActor final class SyncExecutionAllowance {
    private var identifier = UIBackgroundTaskIdentifier.invalid
    private var expired = false
    private var completed = false
    private let end: (UIBackgroundTaskIdentifier) -> Void
    init(expiration: @escaping @MainActor () -> Void,
         begin: (@escaping @MainActor @Sendable () -> Void) -> UIBackgroundTaskIdentifier = {
             UIApplication.shared.beginBackgroundTask(withName: "Finish Momentum sync", expirationHandler: $0)
         },
         end: @escaping (UIBackgroundTaskIdentifier) -> Void = { UIApplication.shared.endBackgroundTask($0) }) {
        self.end = end
        identifier = begin { [weak self] in
            guard let self, !expired, !completed else { return }
            expired = true
            expiration()
            finish()
        }
        if expired { finish() }
    }
    func finish() {
        completed = true
        guard identifier != .invalid else { return }
        let finished = identifier
        identifier = .invalid
        end(finished)
    }
}
