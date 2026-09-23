// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import MomentumKit
import MomentumMobile
import Observation

/// Native presentation state; the coordinator and Rust own scheduling decisions.
@MainActor @Observable final class MobileNotifications {
    private var coordinator: NotificationCoordinator?
    private var badgeCoordinator: BadgeCoordinator?
    private let defaults: UserDefaults
    private let center: any NotificationSchedulingCenter
    private let liveCenter: SystemNotificationCenter?
    let isolated: Bool
    private(set) var status: NotificationSchedulingStatus?
    private(set) var requestingPermission = false
    private(set) var permissionFailed = false
    private(set) var badgeStatus: NotificationBadgeStatus = .idle
    private var refreshes = 0
    var isRefreshing: Bool { refreshes > 0 }

    init(isolated: Bool, defaults: UserDefaults) {
        self.isolated = isolated
        self.defaults = defaults
        if isolated {
            liveCenter = nil
            center = IsolatedNotificationCenter()
        } else {
            let live = SystemNotificationCenter()
            liveCenter = live
            center = live
        }
    }

    func connect(worker: EngineWorker) {
        guard coordinator == nil else { return }
        coordinator = NotificationCoordinator(core: worker, center: center,
            nowMs: { UInt64(max(0, Date().timeIntervalSince1970 * 1_000)) },
            formatContent: Self.text)
        if let liveCenter {
            badgeCoordinator = BadgeCoordinator(core: worker, center: liveCenter)
        }
    }

    func refresh() async {
        guard let coordinator else { return }
        refreshes += 1
        defer { refreshes -= 1 }
        status = await coordinator.refresh()
        if let badgeCoordinator {
            badgeStatus = await badgeCoordinator.refresh(mode: Preferences(defaults).dockBadgeMode)
        }
    }

    func requestPermission() async {
        guard let liveCenter, !requestingPermission else { return }
        requestingPermission = true
        permissionFailed = false
        defer { requestingPermission = false }
        do { _ = try await liveCenter.requestAuthorization() }
        catch { permissionFailed = true }
        await refresh()
    }

    private nonisolated static func text(_ content: NotificationContent) -> NotificationText {
        switch content {
        case .reminder(_, let title, let dueAt):
            let body: String
            if let dueAt {
                let time = Date(timeIntervalSince1970: Double(dueAt) / 1_000)
                    .formatted(date: .omitted, time: .shortened)
                body = String(localized: "Due at \(time)")
            } else { body = String(localized: "Reminder") }
            return NotificationText(title: title, body: body)
        case .summary(_, let counts, _):
            return NotificationText(title: String(localized: "Good morning"),
                body: String(localized: "Today: \(counts.total) · Morning: \(counts.morning) · Tonight: \(counts.tonight)"))
        }
    }
}

/// Preview and UI-test stores never touch the user's OS notification collection.
private actor IsolatedNotificationCenter: NotificationSchedulingCenter {
    func authorization() async throws -> NotificationAuthorization { .notDetermined }
    func pendingRequests() async throws -> [NotificationCenterRecord] { [] }
    func deliveredRequests() async throws -> [NotificationCenterRecord] { [] }
    func add(_ request: NotificationSubmission) async throws {}
    func removePending(identifiers: [String]) async {}
    func removeDelivered(identifiers: [String]) async {}
}
