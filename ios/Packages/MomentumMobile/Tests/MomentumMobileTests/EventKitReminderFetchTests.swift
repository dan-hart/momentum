// SPDX-License-Identifier: GPL-3.0-or-later
import Dispatch
import EventKit
import Testing
@testable import MomentumMobile

@Suite @MainActor struct EventKitReminderFetchTests {
    @Test func frameworkCompletionCanArriveOnBackgroundQueue() async {
        let result = await EventKitReminderFetch.snapshots { completion in
            DispatchQueue.global(qos: .userInitiated).async {
                // No authorization, EventKit store, or personal data is accessed.
                completion([])
            }
        }
        #expect(result == [])
    }

    @Test func backgroundFailurePreservesNilResultForRecovery() async {
        let result = await EventKitReminderFetch.snapshots { completion in
            DispatchQueue.global(qos: .userInitiated).async { completion(nil) }
        }
        #expect(result == nil)
    }

    @Test func synchronousCompletionAlsoResumesExactlyOnce() async {
        let result = await EventKitReminderFetch.snapshots { $0([]) }
        #expect(result == [])
    }
}
