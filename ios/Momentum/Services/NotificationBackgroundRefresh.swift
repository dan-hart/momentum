// SPDX-License-Identifier: GPL-3.0-or-later
import BackgroundTasks
import Foundation
import MomentumMobile

/// Requests a short, discretionary wake to keep the seven-day local notification
/// plan replenished. This never starts network sync.
enum NotificationBackgroundRefresh {
    static let identifier = "com.codedbydan.Momentum.ios.notifications.refresh"

    /// BGTask is an Objective-C lifecycle token that iOS may invoke from a
    /// background queue. The wrapper transfers that one token into actor work.
    private struct TaskHandle: @unchecked Sendable {
        let task: BGAppRefreshTask

        func expire(using handler: @escaping @Sendable () -> Void) {
            task.expirationHandler = handler
        }

        func complete(success: Bool) {
            task.setTaskCompleted(success: success)
        }
    }

    static func register() {
        BGTaskScheduler.shared.register(forTaskWithIdentifier: identifier, using: nil) { task in
            guard let refreshTask = task as? BGAppRefreshTask else {
                task.setTaskCompleted(success: false)
                return
            }
            let handle = TaskHandle(task: refreshTask)
            Task { @MainActor in
                await execute(
                    installExpiration: { handle.expire(using: $0) },
                    refresh: { await MobileAppModel.shared.refreshNotificationsInBackground() },
                    shouldSchedule: { MobileAppModel.shared.shouldScheduleNotificationBackgroundRefresh },
                    updateSchedule: { await updateSchedule(enabled: $0) },
                    complete: { handle.complete(success: $0) }
                )
            }
        }
    }

    /// The system registration and the refresh lifecycle stay separate so expiry,
    /// rescheduling and completion are covered by fast unit tests. Completion is
    /// reported only after the model has reconciled and the next request is queued.
    @MainActor static func execute(
        installExpiration: (@escaping @Sendable () -> Void) -> Void,
        refresh: @escaping @MainActor @Sendable () async -> Bool,
        shouldSchedule: @escaping @MainActor @Sendable () -> Bool,
        updateSchedule: @escaping @MainActor @Sendable (Bool) async -> Void,
        complete: @escaping @Sendable (Bool) -> Void
    ) async {
        let work = Task { @MainActor in await refresh() }
        installExpiration { work.cancel() }
        let success = await work.value
        await updateSchedule(shouldSchedule())
        complete(success)
    }

    static func updateSchedule(enabled: Bool, now: Date = Date()) async {
        guard enabled else {
            BGTaskScheduler.shared.cancel(taskRequestWithIdentifier: identifier)
            return
        }
        let request = BGAppRefreshTaskRequest(identifier: identifier)
        request.earliestBeginDate = NotificationBackgroundRefreshPolicy.earliestBeginDate(after: now)
        do {
            if #available(iOS 27.0, *) {
                try await BGTaskScheduler.shared.submitTaskRequest(request)
            } else {
                try submitLegacy(request)
            }
        } catch {
            // A future foreground/background transition retries. Background
            // refresh is opportunistic and must not affect task editing.
        }
    }

    @available(iOS, introduced: 13.0, obsoleted: 27.0)
    private static func submitLegacy(_ request: BGTaskRequest) throws {
        try BGTaskScheduler.shared.submit(request)
    }
}
