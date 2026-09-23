// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import Testing
@testable import MomentumMobile

@Suite struct NotificationBackgroundRefreshPolicyTests {
    @Test(arguments: [
        NotificationAuthorization.notDetermined,
        .denied,
        .unavailable
    ])
    func unavailableAuthorizationDoesNotRequestBackgroundWork(_ authorization: NotificationAuthorization) {
        var status = NotificationSchedulingStatus()
        status.authorization = authorization
        #expect(!NotificationBackgroundRefreshPolicy.shouldSchedule(status: status))
    }

    @Test(arguments: [
        NotificationAuthorization.authorized,
        .provisional,
        .ephemeral
    ])
    func permittedAuthorizationKeepsTheBoundedPlanReplenished(_ authorization: NotificationAuthorization) {
        var status = NotificationSchedulingStatus()
        status.authorization = authorization
        status.acceptedCount = 0
        #expect(NotificationBackgroundRefreshPolicy.shouldSchedule(status: status))
    }

    @Test func lowPowerModeSuppressesDiscretionaryBackgroundRefresh() {
        var status = NotificationSchedulingStatus()
        status.authorization = .authorized
        #expect(!NotificationBackgroundRefreshPolicy.shouldSchedule(
            status: status,
            isLowPowerModeEnabled: true
        ))
        #expect(NotificationBackgroundRefreshPolicy.shouldSchedule(
            status: status,
            isLowPowerModeEnabled: false
        ))
    }

    @Test func missingStatusDoesNotSpeculativelyWakeTheApp() {
        #expect(!NotificationBackgroundRefreshPolicy.shouldSchedule(status: nil))
    }

    @Test func nextRequestUsesOneDayMinimumCadence() {
        let now = Date(timeIntervalSince1970: 1_800_000_000)
        #expect(NotificationBackgroundRefreshPolicy.earliestBeginDate(after: now)
            == now.addingTimeInterval(24 * 60 * 60))
    }

    @Test func onlyACompleteIssueFreeReconciliationReportsSuccess() {
        var status = NotificationSchedulingStatus()
        status.authorization = .authorized
        #expect(NotificationBackgroundRefreshPolicy.completedSuccessfully(status: status))

        status.wasCancelled = true
        #expect(!NotificationBackgroundRefreshPolicy.completedSuccessfully(status: status))
        status.wasCancelled = false
        status.issues.insert(.schedulingFailed)
        #expect(!NotificationBackgroundRefreshPolicy.completedSuccessfully(status: status))
        #expect(!NotificationBackgroundRefreshPolicy.completedSuccessfully(status: nil))
    }
}
