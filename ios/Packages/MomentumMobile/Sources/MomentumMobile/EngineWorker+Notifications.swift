// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore

extension EngineWorker: NotificationSchedulingCore {
    public func notificationPlan(nowMs: UInt64) throws -> NotificationPlan {
        try engine.notificationPlan(nowMs: nowMs)
    }

    public func reconcileNotifications(nowMs: UInt64, pending: [NotificationObservation],
                                       delivered: [NotificationObservation]) throws -> NotificationPlan {
        try engine.reconcileNotifications(nowMs: nowMs, pending: pending, delivered: delivered)
    }

    public func acceptNotification(request: NotificationRequest, nowMs: UInt64) throws -> NotificationAcceptance {
        try engine.acceptNotification(request: request, nowMs: nowMs)
    }
}
