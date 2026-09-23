// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore

/// Immutable values keep native notification objects inside the OS adapter.
public enum NotificationAuthorization: Sendable, Equatable {
    case notDetermined, denied, authorized, provisional, ephemeral, unavailable

    public var permitsScheduling: Bool {
        switch self {
        case .authorized, .provisional, .ephemeral: true
        case .notDetermined, .denied, .unavailable: false
        }
    }
}

/// Keeps the core's bounded notification plan populated without asking iOS to
/// wake the app more than once per day. The system may run a request later.
public enum NotificationBackgroundRefreshPolicy {
    public static let minimumCadence: TimeInterval = 24 * 60 * 60

    public static func shouldSchedule(status: NotificationSchedulingStatus?,
                                      isLowPowerModeEnabled: Bool = false) -> Bool {
        !isLowPowerModeEnabled && status?.authorization?.permitsScheduling == true
    }

    public static func earliestBeginDate(after now: Date) -> Date {
        now.addingTimeInterval(minimumCadence)
    }

    public static func completedSuccessfully(status: NotificationSchedulingStatus?) -> Bool {
        guard let status else { return false }
        return !status.isRefreshing && !status.wasCancelled && status.issues.isEmpty
    }
}

public struct NotificationCenterRecord: Sendable, Equatable {
    public let identifier: String
    /// Private metadata: revisions can contain task text. Never log this value.
    public let metadata: [String: String]

    public init(identifier: String, metadata: [String: String]) {
        self.identifier = identifier
        self.metadata = metadata
    }
}

public struct NotificationText: Sendable, Equatable {
    public let title: String
    public let body: String

    public init(title: String, body: String) { self.title = title; self.body = body }
}

/// Typed navigation/category data, separate from opaque scheduling identities.
public enum NotificationRoute: Sendable, Equatable {
    case reminder(taskID: String)
    case summary(day: String)
}

/// A one-shot OS request. The coordinator retains the unchanged core request for
/// acknowledgement; localized strings and trigger clamping never alter core values.
public struct NotificationSubmission: Sendable, Equatable {
    public let route: NotificationRoute
    public let identifier: String
    public let metadata: [String: String]
    public let title: String
    public let body: String
    public let timeInterval: TimeInterval
}

public protocol NotificationSchedulingCore: Sendable {
    func notificationPlan(nowMs: UInt64) async throws -> NotificationPlan
    func reconcileNotifications(nowMs: UInt64, pending: [NotificationObservation],
                                delivered: [NotificationObservation]) async throws -> NotificationPlan
    func acceptNotification(request: NotificationRequest, nowMs: UInt64) async throws -> NotificationAcceptance
}

/// No permission-prompt API or default live center: demos/tests must inject a fake.
/// `add` must finish when the OS operation finishes, even when its caller is cancelled,
/// so a successful late add can be cleaned up. Removal initiates cancellation; the
/// coordinator reads both collections afterward to verify it. Cleanup methods must
/// remain usable from a cancelled task.
public protocol NotificationSchedulingCenter: Sendable {
    func authorization() async throws -> NotificationAuthorization
    func pendingRequests() async throws -> [NotificationCenterRecord]
    func deliveredRequests() async throws -> [NotificationCenterRecord]
    func add(_ request: NotificationSubmission) async throws
    func removePending(identifiers: [String]) async
    func removeDelivered(identifiers: [String]) async
}

/// Safe to expose in UI state. Never include arbitrary OS/core error descriptions,
/// request content, identifiers, or source revisions in status or logs.
public enum NotificationSchedulingIssue: Sendable, Hashable {
    case authorizationUnavailable, observationsUnavailable, planUnavailable
    case schedulingFailed, acceptanceFailed, sourceChanged, cancellationUnconfirmed
}

public struct NotificationSchedulingStatus: Sendable, Equatable {
    public internal(set) var authorization: NotificationAuthorization?
    public internal(set) var horizonEndMs: UInt64?
    public internal(set) var overflowReminders: UInt64 = 0
    public internal(set) var overflowSummaries: UInt64 = 0
    public internal(set) var acceptedCount = 0
    public internal(set) var unconfirmedCancellationCount = 0
    public internal(set) var issues: Set<NotificationSchedulingIssue> = []
    public internal(set) var isRefreshing = false
    public internal(set) var wasCancelled = false
}
