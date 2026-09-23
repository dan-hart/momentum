// SPDX-License-Identifier: GPL-3.0-or-later
@testable import MomentumMobile
import Testing

@Suite struct MobilePlatformCapabilitiesTests {
    @Test func phoneOmitsKeyboardShortcutDiscovery() {
        let capabilities = MobilePlatformCapabilities(isPad: false)

        #expect(!capabilities.showsKeyboardShortcuts)
    }

    @Test func padIncludesKeyboardShortcutDiscovery() {
        let capabilities = MobilePlatformCapabilities(isPad: true)

        #expect(capabilities.showsKeyboardShortcuts)
    }
}
