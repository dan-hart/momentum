// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumMobile
import UserNotifications

/// The only adapter that touches the live OS notification center. App composition
/// must give demo/test stores a fake center instead of constructing this actor.
actor SystemNotificationCenter: NotificationSchedulingCenter, NotificationBadgeCenter {
    static let reminderCategory = "momentum.notification.reminder.v1"
    static let summaryCategory = "momentum.notification.summary.v1"

    enum MetadataKey {
        static let route = "momentum.route"
        static let taskID = "momentum.task-id"
        static let day = "momentum.day"
    }

    /// Fixed cases deliberately omit NSError descriptions, userInfo and task data.
    enum Failure: Error, Sendable {
        case authorizationRequest, invalidRequest, scheduling, badgeUpdate
    }

    private let center = UNUserNotificationCenter.current()

    init() {}

    func authorization() async throws -> NotificationAuthorization {
        await withCheckedContinuation { continuation in
            center.getNotificationSettings { settings in
                let value: NotificationAuthorization = switch settings.authorizationStatus {
                case .notDetermined: .notDetermined
                case .denied: .denied
                case .authorized: .authorized
                case .provisional: .provisional
                case .ephemeral: .ephemeral
                @unknown default: .unavailable
                }
                continuation.resume(returning: value)
            }
        }
    }

    /// Only call from an explicit user action. Scheduling never invokes this method.
    func requestAuthorization() async throws -> NotificationAuthorization {
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            center.requestAuthorization(options: [.alert, .sound, .badge]) { _, error in
                if error != nil { continuation.resume(throwing: Failure.authorizationRequest) }
                else { continuation.resume() }
            }
        }
        return try await authorization()
    }

    func pendingRequests() async throws -> [NotificationCenterRecord] {
        await withCheckedContinuation { continuation in
            center.getPendingNotificationRequests { requests in
                continuation.resume(returning: requests.map(Self.record))
            }
        }
    }

    func deliveredRequests() async throws -> [NotificationCenterRecord] {
        await withCheckedContinuation { continuation in
            center.getDeliveredNotifications { notifications in
                continuation.resume(returning: notifications.map { Self.record($0.request) })
            }
        }
    }

    func add(_ request: NotificationSubmission) async throws {
        guard request.timeInterval.isFinite, request.timeInterval > 0,
              NotificationIdentity.decode(identifier: request.identifier, metadata: request.metadata) != nil else {
            throw Failure.invalidRequest
        }
        let content = UNMutableNotificationContent()
        content.title = request.title
        content.body = request.body
        content.sound = .default
        var metadata = request.metadata
        switch request.route {
        case .reminder(let taskID):
            guard !taskID.isEmpty else { throw Failure.invalidRequest }
            content.categoryIdentifier = Self.reminderCategory
            metadata[MetadataKey.route] = "reminder"
            metadata[MetadataKey.taskID] = taskID
            metadata.removeValue(forKey: MetadataKey.day)
        case .summary(let day):
            guard !day.isEmpty else { throw Failure.invalidRequest }
            content.categoryIdentifier = Self.summaryCategory
            metadata[MetadataKey.route] = "summary"
            metadata[MetadataKey.day] = day
            metadata.removeValue(forKey: MetadataKey.taskID)
        }
        content.userInfo = metadata
        let trigger = UNTimeIntervalNotificationTrigger(timeInterval: request.timeInterval, repeats: false)
        let native = UNNotificationRequest(identifier: request.identifier, content: content, trigger: trigger)
        // A cancelled caller still waits for Apple's actual completion. Returning
        // early could orphan a successful late add that the coordinator cannot undo.
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            center.add(native) { error in
                if error != nil { continuation.resume(throwing: Failure.scheduling) }
                else { continuation.resume() }
            }
        }
    }

    /// Removal has no completion callback. The coordinator confirms it by readback;
    /// these methods intentionally remain usable while its drain task is cancelled.
    func removePending(identifiers: [String]) async {
        center.removePendingNotificationRequests(withIdentifiers: owned(identifiers))
    }

    func removeDelivered(identifiers: [String]) async {
        center.removeDeliveredNotifications(withIdentifiers: owned(identifiers))
    }

    func badgePermission() async throws -> Bool {
        await withCheckedContinuation { continuation in
            center.getNotificationSettings { settings in
                let allowed = [.authorized, .provisional, .ephemeral].contains(settings.authorizationStatus)
                continuation.resume(returning: allowed && settings.badgeSetting == .enabled)
            }
        }
    }

    func setBadgeCount(_ count: UInt32) async throws {
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            center.setBadgeCount(Int(count)) { error in
                if error != nil { continuation.resume(throwing: Failure.badgeUpdate) }
                else { continuation.resume() }
            }
        }
    }

    /// Callers must separately validate NotificationIdentity before trusting routes.
    nonisolated static func decodeRoute(metadata: [String: String]) -> NotificationRoute? {
        switch metadata[MetadataKey.route] {
        case "reminder":
            guard let taskID = metadata[MetadataKey.taskID], !taskID.isEmpty else { return nil }
            return .reminder(taskID: taskID)
        case "summary":
            guard let day = metadata[MetadataKey.day], !day.isEmpty else { return nil }
            return .summary(day: day)
        default:
            return nil
        }
    }

    private func owned(_ identifiers: [String]) -> [String] {
        identifiers.filter { $0.hasPrefix(NotificationIdentity.prefix) }
    }

    private nonisolated static func record(_ request: UNNotificationRequest) -> NotificationCenterRecord {
        var metadata: [String: String] = [:]
        for (key, value) in request.content.userInfo {
            guard let key = key as? String, key.hasPrefix("momentum."), let value = value as? String else { continue }
            metadata[key] = value
        }
        return NotificationCenterRecord(identifier: request.identifier, metadata: metadata)
    }
}
