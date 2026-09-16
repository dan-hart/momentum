// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import Testing
@testable import MomentumKit

@Suite struct PreferenceMigrationTests {
    @Test func migrationPreservesExistingChoicesAndLeavesLegacyDomainIntact() throws {
        let legacy = "momentum-legacy-test-\(UUID().uuidString)"
        let target = "momentum-new-test-\(UUID().uuidString)"
        let defaults = try #require(UserDefaults(suiteName: target))
        defer {
            defaults.removePersistentDomain(forName: legacy)
            defaults.removePersistentDomain(forName: target)
        }
        defaults.setPersistentDomain([PrefKey.appFontName: "Helvetica", PrefKey.contentFontSize: 19.0,
                                      PrefKey.interfaceFontSize: 17.0], forName: legacy)
        defaults.set(22.0, forKey: PrefKey.contentFontSize)
        Preferences.migrateLegacyDomain(defaults, from: legacy, to: target)
        #expect(defaults.string(forKey: PrefKey.appFontName) == "Helvetica")
        #expect(defaults.double(forKey: PrefKey.contentFontSize) == 22)
        #expect(defaults.double(forKey: PrefKey.interfaceFontSize) == 17)
        #expect(defaults.persistentDomain(forName: legacy)?[PrefKey.contentFontSize] as? Double == 19)
        AppTypography.reset(defaults: defaults)
        Preferences.migrateLegacyDomain(defaults, from: legacy, to: target)
        #expect(AppTypography(defaults: defaults) == AppTypography(), "reset choices must not be reimported")
    }

    @Test func freshInstallsDoNotImportALegacyDomainThatAppearsLater() throws {
        let legacy = "momentum-legacy-test-\(UUID().uuidString)"
        let target = "momentum-new-test-\(UUID().uuidString)"
        let defaults = try #require(UserDefaults(suiteName: target))
        defer {
            defaults.removePersistentDomain(forName: legacy)
            defaults.removePersistentDomain(forName: target)
        }
        Preferences.migrateLegacyDomain(defaults, from: legacy, to: target)
        defaults.setPersistentDomain([PrefKey.appFontName: "Helvetica"], forName: legacy)
        Preferences.migrateLegacyDomain(defaults, from: legacy, to: target)
        #expect(defaults.persistentDomain(forName: target)?[PrefKey.appFontName] == nil)
    }
}
