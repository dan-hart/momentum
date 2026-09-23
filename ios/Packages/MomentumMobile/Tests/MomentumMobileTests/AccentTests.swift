// SPDX-License-Identifier: GPL-3.0-or-later
import DHFlatUIColors
import Foundation
import Testing
@testable import MomentumMobile

@Suite struct AccentTests {
    @Test func customChoicesComeOnlyFromPinnedPalette() throws {
        let allowed = Set(DHFlatUIColors.Palette.allCases.flatMap(\.colors).map { $0.hex.uppercased() })
        #expect(AccentPalette.all.count == 1)
        // AsNeeded's curated spectrum order, independently checked against its catalog.
        #expect(AccentPalette.all.flatMap(\.colors).map(\.hex) == [
            "#E74C3C", "#E67E22", "#F39C12", "#2ECC71", "#1ABC9C",
            "#3498DB", "#9B59B6", "#C0392B", "#16A085"
        ])
        for choice in AccentPalette.all.flatMap(\.colors) { #expect(allowed.contains(choice.hex)) }
        #expect(AccentChoice.resolve("made-up-value") == .momentum)
        #expect(AccentChoice.momentum.hex == "#FF6600")
    }

    @Test func savedCuratedChoicesKeepTheirIdentityAndRetiredChoicesUseDefault() {
        #expect(AccentChoice.resolve("Flat UI v1:TURQUOISE").hex == "#1ABC9C")
        #expect(AccentChoice.resolve("Flat UI v1:ALIZARIN").hex == "#E74C3C")
        #expect(AccentChoice.resolve("Flat UI v1:WET ASPHALT") == .momentum)
        for palette in DHFlatUIColors.Palette.allCases {
            for color in palette.colors {
                let id = palette.name + ":" + color.name
                if !AccentPalette.all.flatMap(\.colors).contains(where: { $0.id == id }) {
                    #expect(AccentChoice.resolve(id) == .momentum)
                }
            }
        }
    }

    @Test func everyPaletteMeetsTextContrastInEachAppearance() throws {
        let light = ["#FFFFFF", "#F2F2F7"].map { RGBColor(hex: $0)! }
        let dark = ["#000000", "#1C1C1E", "#2C2C2E"].map { RGBColor(hex: $0)! }
        for choice in [AccentChoice.momentum] + AccentPalette.all.flatMap(\.colors) {
            let base = try #require(RGBColor(hex: choice.hex))
            for surfaces in [light, dark] {
                for ratio in [4.5, 7.0] {
                    let rendered = base.accessible(on: surfaces, minimumRatio: ratio)
                    for surface in surfaces { #expect(rendered.contrast(with: surface) >= ratio - 0.001) }
                    #expect(rendered.onColor.contrast(with: rendered) >= 4.5)
                }
            }
        }
    }

    @Test func preferencePersistsAndRejectsArbitraryHex() throws {
        let suite = "momentum-accent-\(UUID())"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let chosen = try #require(AccentPalette.all.first?.colors.first)
        defaults.set(chosen.id, forKey: AccentChoice.preferenceKey)
        #expect(AccentChoice.load(defaults) == chosen)
        defaults.set("#ABCDEF", forKey: AccentChoice.preferenceKey)
        #expect(AccentChoice.load(defaults) == .momentum)
    }

    @Test func defaultOrangeStaysExactInDarkAppearance() {
        let surfaces = ["#000000", "#1C1C1E", "#2C2C2E", "#3A3A3C"].map { RGBColor(hex: $0)! }
        for increased in [false, true] {
            #expect(AccentChoice.momentum.renderedColor(on: surfaces, isDark: true,
                increasedContrast: increased) == RGBColor(hex: "#FF6600"))
        }
    }

    @Test func defaultLightOrangeStaysExactWithContrastingActionInk() {
        let surfaces = ["#FFFFFF", "#F2F2F7"].map { RGBColor(hex: $0)! }
        for increased in [false, true] {
            let color = AccentChoice.momentum.renderedColor(on: surfaces, isDark: false,
                increasedContrast: increased)
            #expect(color == RGBColor(hex: "#FF6600"))
            #expect(color.onColor == .black)
            #expect(color.contrast(with: color.onColor) >= 4.5)
        }
    }
    @Test func renderedCustomChoicesMeetContrastAndFilledActionForegrounds() {
        let light = ["#FFFFFF", "#F2F2F7"].map { RGBColor(hex: $0)! }
        let dark = ["#000000", "#1C1C1E", "#2C2C2E", "#3A3A3C"].map { RGBColor(hex: $0)! }
        for choice in AccentPalette.all.flatMap(\.colors) {
            for isDark in [false, true] {
                for increased in [false, true] {
                    let backgrounds = isDark ? dark : light
                    let rendered = choice.renderedColor(on: backgrounds, isDark: isDark,
                        increasedContrast: increased)
                    let minimum = increased ? 7.05 : 4.55
                    #expect(backgrounds.allSatisfy { rendered.contrast(with: $0) >= minimum - 0.000001 })
                    #expect(rendered.contrast(with: isDark ? .black : .white) >= 4.5)
                }
            }
        }
    }

}
