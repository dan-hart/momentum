// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import Testing
@testable import MomentumKit

@MainActor @Suite struct DemoSyncTests {
    @Test func previewCannotSyncEvenWhenRealPreferencesEnableIt() {
        let h = Harness()
        h.defaults.set(true, forKey: PrefKey.syncEnabled)
        h.defaults.set(true, forKey: PrefKey.p2pEnabled)
        let preview = AppState(engine: h.engine, defaults: h.defaults, keychain: h.state.keychain,
                               services: false, isDemo: true)
        #expect(!preview.syncAvailable)
        preview.applyP2pSetting()
        #expect(!h.engine.p2pRunning())
        preview.sync()
        #expect(!preview.isSyncing)
        #expect(preview.banner == nil)
        #expect(preview.toasts.last?.text.contains("preview") == true)
        preview.showDevices()
        #expect(preview.sheet == nil)
        #expect(h.defaults.bool(forKey: PrefKey.p2pEnabled), "preview must not change real settings")
    }
}
