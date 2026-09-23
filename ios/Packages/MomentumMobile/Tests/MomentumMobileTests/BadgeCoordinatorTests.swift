// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumKit
import Testing
@testable import MomentumMobile

private actor BadgeCore: NotificationBadgeCore {
    var queries: [Bool] = []
    func badgeCount(includeOverdue: Bool) -> UInt32 {
        queries.append(includeOverdue)
        return includeOverdue ? 5 : 2
    }
}
private actor BadgeCenter: NotificationBadgeCenter {
    var allowed = true
    var counts: [UInt32] = []
    var fails = false
    private var gate: CheckedContinuation<Void, Never>?
    private var paused = false
    private var waiting: [CheckedContinuation<Void, Never>] = []
    var shouldPause = false
    func badgePermission() -> Bool { allowed }
    func setBadgeCount(_ count: UInt32) async throws {
        if fails { throw CocoaError(.fileWriteNoPermission) }
        if shouldPause {
            shouldPause = false
            await withCheckedContinuation { continuation in
                gate = continuation; paused = true
                waiting.forEach { $0.resume() }; waiting = []
            }
        }
        counts.append(count)
    }
    func deny() { allowed = false }
    func fail() { fails = true }
    func pauseNext() { shouldPause = true }
    func waitForPause() async {
        if paused { return }
        await withCheckedContinuation { waiting.append($0) }
    }
    func resume() { gate?.resume(); gate = nil }
}

@Suite struct BadgeCoordinatorTests {
    @Test func modesUseSharedCountsAndNoneClearsWithoutAQuery() async {
        let core = BadgeCore(), center = BadgeCenter()
        let subject = BadgeCoordinator(core: core, center: center)
        #expect(await subject.refresh(mode: .dueToday) == .updated(2))
        #expect(await subject.refresh(mode: .todayIncludingOverdue) == .updated(5))
        #expect(await subject.refresh(mode: .none) == .updated(0))
        #expect(await core.queries == [false, true])
        #expect(await center.counts == [2, 5, 0])
    }
    @Test func deniedPermissionDoesNotWriteOrQuery() async {
        let core = BadgeCore(), center = BadgeCenter()
        await center.deny()
        let subject = BadgeCoordinator(core: core, center: center)
        #expect(await subject.refresh(mode: .dueToday) == .blockedBySystem)
        #expect(await core.queries.isEmpty)
        #expect(await center.counts.isEmpty)
    }
    @Test func failureIsSanitizedAndNeverReportedAsUpdated() async {
        let center = BadgeCenter()
        await center.fail()
        let subject = BadgeCoordinator(core: BadgeCore(), center: center)
        #expect(await subject.refresh(mode: .dueToday) == .failed)
    }
    @Test func preferenceChangeDuringWriteFinishesWithTheNewestMode() async {
        let core = BadgeCore(), center = BadgeCenter()
        await center.pauseNext()
        let subject = BadgeCoordinator(core: core, center: center)
        let first = Task { await subject.refresh(mode: .dueToday) }
        await center.waitForPause()
        await subject.requestRefresh(mode: .none)
        await center.resume()
        #expect(await first.value == .updated(0))
        #expect(await center.counts == [2, 0])
    }
}
