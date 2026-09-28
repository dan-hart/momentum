// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import Testing
@testable import MomentumMobile

@Suite struct BackgroundSyncPolicyTests {
    @Test func aWakeIsRequestedOnlyWhenTheChoiceAndTheConnectionAllowIt() {
        #expect(BackgroundSyncPolicy.shouldSchedule(enabled: true, exchangePossible: true))
        #expect(!BackgroundSyncPolicy.shouldSchedule(enabled: false, exchangePossible: true))
        #expect(!BackgroundSyncPolicy.shouldSchedule(enabled: true, exchangePossible: false))
    }

    @Test func nextRequestKeepsAHalfHourFloor() {
        let now = Date(timeIntervalSince1970: 1_800_000_000)
        #expect(BackgroundSyncPolicy.earliestBeginDate(after: now) == now.addingTimeInterval(30 * 60))
    }
}
