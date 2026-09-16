// SPDX-License-Identifier: GPL-3.0-or-later
import AppKit
import Testing
@testable import MomentumKit

@Suite struct TypographyTests {
    @Test func systemDefaultsAndIndependentSizes() {
        let typography = AppTypography()
        #expect(typography.fontName.isEmpty)
        #expect(typography.contentSize == 13)
        #expect(typography.interfaceSize == 13)
        let enlarged = AppTypography(contentSize: 22, interfaceSize: 15)
        #expect(enlarged.resolvedFont(in: .content).pointSize == 22)
        #expect(enlarged.resolvedFont(in: .interface).pointSize == 15)
        #expect(enlarged.resolvedFont(in: .content, role: .caption).pointSize < 22)
        #expect(enlarged.resolvedFont(in: .interface, role: .title2).pointSize > 15)
    }

    @Test func missingFontAndInvalidSizesRemainReadable() {
        let typography = AppTypography(fontName: "Missing-Momentum-Test-Font", contentSize: .nan, interfaceSize: .infinity)
        #expect(typography.resolvedFont(in: .content).fontName == NSFont.systemFont(ofSize: 13).fontName)
        #expect(typography.contentSize == 13)
        #expect(typography.interfaceSize == 13)
        #expect(AppTypography(contentSize: -20, interfaceSize: 900).contentSize == 10)
        #expect(AppTypography(contentSize: -20, interfaceSize: 900).interfaceSize == 32)
        #expect(AppTypography(contentSize: 10).resolvedFont(in: .content, role: .caption).pointSize >= 10,
                "secondary text must respect the macOS minimum even at the smallest preference")
    }

    @Test func fontPanelSelectionPersistsWithoutChangingInterfaceSizeAndResetRestoresDefaults() throws {
        let name = "momentum-typography-\(UUID().uuidString)"
        let defaults = try #require(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name) }
        Preferences.register(defaults)
        defaults.set(18.0, forKey: PrefKey.interfaceFontSize)
        defaults.set("option", forKey: PrefKey.modifierKey)
        let font = try #require(NSFont(name: "Helvetica-Bold", size: 21))
        AppTypography.saveSelection(font, defaults: defaults)
        let saved = AppTypography(defaults: defaults)
        #expect(saved.fontName == font.fontName)
        #expect(saved.resolvedFont(in: .content).fontName == font.fontName)
        #expect(saved.resolvedFont(in: .interface).fontName == font.fontName)
        #expect(saved.contentSize == 21)
        #expect(saved.interfaceSize == 18)
        AppTypography.reset(defaults: defaults)
        #expect(AppTypography(defaults: defaults) == AppTypography())
        #expect(defaults.string(forKey: PrefKey.modifierKey) == "option")
    }
}
