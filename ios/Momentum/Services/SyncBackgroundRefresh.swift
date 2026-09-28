// SPDX-License-Identifier: GPL-3.0-or-later
import BackgroundTasks
import Foundation
import MomentumMobile

/// Requests a discretionary wake to run one bounded Nextcloud exchange while the app
/// is not on screen, when Settings › Sync › "Sync in the background" allows it. The
/// lifecycle (expiry cancels, reschedule, then complete) is shared with the
/// notification refresh so both are covered by the same fast tests.
enum SyncBackgroundRefresh {
    static let identifier = "com.codedbydan.Momentum.ios.sync.refresh"

    private struct TaskHandle: @unchecked Sendable {
        let task: BGAppRefreshTask
        func expire(using handler: @escaping @Sendable () -> Void) { task.expirationHandler = handler }
        func complete(success: Bool) { task.setTaskCompleted(success: success) }
    }

    static func register() {
        BGTaskScheduler.shared.register(forTaskWithIdentifier: identifier, using: nil) { task in
            guard let refreshTask = task as? BGAppRefreshTask else {
                task.setTaskCompleted(success: false)
                return
            }
            let handle = TaskHandle(task: refreshTask)
            Task { @MainActor in
                await NotificationBackgroundRefresh.execute(
                    installExpiration: { handle.expire(using: $0) },
                    refresh: { await MobileAppModel.shared.syncInBackground() },
                    shouldSchedule: { MobileAppModel.shared.shouldScheduleBackgroundSync },
                    updateSchedule: { await updateSchedule(enabled: $0) },
                    complete: { handle.complete(success: $0) }
                )
            }
        }
    }

    static func updateSchedule(enabled: Bool, now: Date = Date()) async {
        guard enabled else {
            BGTaskScheduler.shared.cancel(taskRequestWithIdentifier: identifier)
            return
        }
        let request = BGAppRefreshTaskRequest(identifier: identifier)
        request.earliestBeginDate = BackgroundSyncPolicy.earliestBeginDate(after: now)
        do {
            if #available(iOS 27.0, *) {
                try await BGTaskScheduler.shared.submitTaskRequest(request)
            } else {
                try submitLegacy(request)
            }
        } catch {
            // Background sync is opportunistic; the next transition to the background retries.
        }
    }

    @available(iOS, introduced: 13.0, obsoleted: 27.0)
    private static func submitLegacy(_ request: BGTaskRequest) throws {
        try BGTaskScheduler.shared.submit(request)
    }
}
