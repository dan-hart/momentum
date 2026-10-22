// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
import Foundation
import Testing
@testable import MomentumKit

@Suite struct LocalizationTests {
    @Test func germanCompletionChoicesKeepTheApprovedNumbers() throws {
        let path = try #require(Bundle.module.path(forResource: "de", ofType: "lproj"))
        let german = try #require(Bundle(path: path))
        let labels = [
            ("1 Wood Pop", "1 Holzklang"), ("2 Double Check", "2 Doppeltes Häkchen"),
            ("3 Rounded Lift", "3 Sanfter Aufstieg"), ("4 Pocket Piano", "4 Taschenklavier"),
            ("5 String Flick", "5 Saitenzupfer"), ("6 Bright Finish", "6 Heller Abschluss"),
            ("7 Harp Snap", "7 Harfenzupfer"), ("8 Click and Glow", "8 Klick und Glanz"),
            ("9 Tiny Ta-Da", "9 Kleines Ta-Da"), ("10 Warm Spark", "10 Warmer Funke"),
        ]
        for (key, translated) in labels {
            #expect(german.localizedString(forKey: key, value: nil, table: "Localizable") == translated)
        }
        #expect(CompletionSoundChoice.allCases.filter(\.isLegacy).map { $0.label(locale: Locale(identifier: "de_DE")) } == labels.map { $0.1 })
    }

    @Test func germanResourcesAreAvailableInThePackageBundle() throws {
        let path = try #require(Bundle.module.path(forResource: "de", ofType: "lproj"))
        let german = try #require(Bundle(path: path))
        #expect(german.localizedString(forKey: "Today", value: nil, table: "Localizable") == "Heute")
        #expect(german.localizedString(forKey: "Undo", value: nil, table: "Localizable") == "Rückgängig")
    }

    @Test func germanNumericInterpolationUsesTheCompiledTypedKey() throws {
        // The process runs in English. Select German resources explicitly; locale
        // controls number formatting and does not override a bundle's language.
        let path = try #require(Bundle.module.path(forResource: "de", ofType: "lproj"))
        let german = try #require(Bundle(path: path))
        let count: UInt32 = 3
        let translated = String(localized: "\(count) tasks deleted", bundle: german, locale: Locale(identifier: "de"))
        #expect(translated == "3 Aufgaben gelöscht")
        let one: UInt32 = 1
        #expect(String(localized: "\(one) tasks deleted", bundle: german, locale: Locale(identifier: "de")) == "1 Aufgabe gelöscht")
        let zero: UInt32 = 0
        #expect(String(localized: "\(zero) tasks deleted", bundle: german, locale: Locale(identifier: "de")) == "0 Aufgaben gelöscht")
    }
}
