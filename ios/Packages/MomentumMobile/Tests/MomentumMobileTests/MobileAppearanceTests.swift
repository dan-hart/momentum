// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import Testing
@testable import MomentumMobile

@Suite struct MobileAppearanceTests {
    @Test func nativeDefaultsAndIndependentScalesSurviveRestart() throws {
        let suite = "momentum-appearance-\(UUID())"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let initial = MobileAppearance(defaults: defaults)
        #expect(initial.contentScale == 1)
        #expect(initial.interfaceScale == 1)
        #expect(initial.haptics)
        defaults.set(1.4, forKey: MobileAppearance.contentScaleKey)
        defaults.set(0.9, forKey: MobileAppearance.interfaceScaleKey)
        defaults.set(false, forKey: MobileAppearance.hapticsKey)
        let restarted = MobileAppearance(defaults: try #require(UserDefaults(suiteName: suite)))
        #expect(restarted.contentScale == 1.4)
        #expect(restarted.interfaceScale == 0.9)
        #expect(!restarted.haptics)
    }

    @Test func corruptPreferencesCannotProduceInvalidTextSizes() {
        #expect(MobileAppearance(contentScale: .nan, interfaceScale: .infinity).contentScale == 1)
        #expect(MobileAppearance(contentScale: .nan, interfaceScale: .infinity).interfaceScale == 1)
        #expect(MobileAppearance(contentScale: -2, interfaceScale: 100).contentScale == 0.8)
        #expect(MobileAppearance(contentScale: -2, interfaceScale: 100).interfaceScale == 1.6)
    }
}
