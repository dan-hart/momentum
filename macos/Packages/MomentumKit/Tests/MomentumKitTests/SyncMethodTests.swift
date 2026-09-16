import Foundation
import Testing
@testable import MomentumKit

@MainActor @Suite struct SyncMethodTests {
    @Test func legacyPreferencesChooseExactlyOneService() {
        let h = Harness()
        h.defaults.set(true, forKey: PrefKey.syncEnabled)
        h.defaults.set(true, forKey: PrefKey.p2pEnabled)
        #expect(h.state.prefs.syncMethod == .libresync)
        #expect(!h.state.prefs.syncEnabled)
        Preferences.migrateSyncMethod(h.defaults)
        h.defaults.set(false, forKey: PrefKey.p2pEnabled)
        #expect(h.state.prefs.syncMethod == .libresync)
        for method in SyncMethod.allCases {
            h.defaults.set(method.rawValue, forKey: PrefKey.syncMethod)
            #expect(h.state.prefs.syncEnabled == (method == .nextcloud))
            #expect(h.state.prefs.p2pEnabled == (method == .libresync))
        }
        h.defaults.set("future-provider", forKey: PrefKey.syncMethod)
        #expect(h.state.prefs.syncMethod == .off)
    }

    @Test func syncLifecycleShowsProgressAndKeepsFailuresUntilRetry() {
        let h = Harness()
        h.defaults.set("libresync", forKey: PrefKey.syncMethod)
        h.state.handle(.syncStarted)
        #expect(h.state.syncInProgress)
        #expect(h.state.syncCaption?.contains("Syncing") == true)
        h.state.handle(.syncCompleted(error: "Peer unavailable"))
        #expect(!h.state.syncInProgress)
        #expect(h.state.syncError == "Peer unavailable")
        h.state.handle(.syncStarted)
        #expect(h.state.syncError == nil)
        h.state.handle(.syncCompleted(error: nil))
        #expect(!h.state.syncInProgress)
        h.defaults.set("off", forKey: PrefKey.syncMethod)
        h.state.preferencesChanged()
        h.state.handle(.syncStarted)
        #expect(!h.state.syncInProgress, "late events from a stopped provider are ignored")
    }
}
