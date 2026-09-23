// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore
import MomentumKit

public protocol NotificationBadgeCore: Sendable {
    func badgeCount(includeOverdue: Bool) async -> UInt32
}

public protocol NotificationBadgeCenter: Sendable {
    func badgePermission() async throws -> Bool
    func setBadgeCount(_ count: UInt32) async throws
}

public enum NotificationBadgeStatus: Sendable, Equatable {
    case idle, blockedBySystem, updated(UInt32), failed
}

/// One native write at a time; a preference/mutation arriving while suspended
/// always gets a fresh core count before the drain ends. Never prompts for access.
public actor BadgeCoordinator {
    private let core: any NotificationBadgeCore
    private let center: any NotificationBadgeCenter
    private var mode: DockBadgeMode = .dueToday
    private var dirty = false
    private var running: Task<Void, Never>?
    public private(set) var status: NotificationBadgeStatus = .idle

    public init(core: any NotificationBadgeCore, center: any NotificationBadgeCenter) {
        self.core = core
        self.center = center
    }

    public func requestRefresh(mode: DockBadgeMode) {
        self.mode = mode
        dirty = true
        if running == nil { running = Task { await drain() } }
    }

    public func refresh(mode: DockBadgeMode) async -> NotificationBadgeStatus {
        requestRefresh(mode: mode)
        while let running { await running.value }
        return status
    }

    private func drain() async {
        while dirty {
            dirty = false
            let requestedMode = mode
            do {
                guard try await center.badgePermission() else {
                    status = .blockedBySystem
                    continue
                }
                let count: UInt32 = requestedMode == .none ? 0
                    : await core.badgeCount(includeOverdue: requestedMode == .todayIncludingOverdue)
                try await center.setBadgeCount(count)
                status = .updated(count)
            } catch { status = .failed }
        }
        running = nil
    }
}

extension EngineWorker: NotificationBadgeCore {
    public func badgeCount(includeOverdue: Bool) -> UInt32 {
        engine.taskCount(mode: includeOverdue ? .todayIncludingOverdue : .dueToday)
    }
}
