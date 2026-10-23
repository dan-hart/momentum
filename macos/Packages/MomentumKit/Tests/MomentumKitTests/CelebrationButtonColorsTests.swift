// SPDX-License-Identifier: GPL-3.0-or-later
import SwiftUI
import Testing
@testable import MomentumKit

@Suite struct CelebrationButtonColorsTests {
    private func luminance(_ color: Color.Resolved) -> Double {
        func linear(_ component: Float) -> Double {
            let value = Double(component)
            return value <= 0.04045 ? value / 12.92 : pow((value + 0.055) / 1.055, 2.4)
        }
        return 0.2126 * linear(color.red) + 0.7152 * linear(color.green) + 0.0722 * linear(color.blue)
    }
    private func contrast(_ a: Color.Resolved, _ b: Color.Resolved) -> Double {
        (max(luminance(a), luminance(b)) + 0.05) / (min(luminance(a), luminance(b)) + 0.05)
    }
    @Test func defaultOrangeNeedsDarkInkForBothSymbolAndText() {
        let orange = Color.Resolved(red: 1, green: 0.4, blue: 0)
        let white = Color.Resolved(red: 1, green: 1, blue: 1)
        #expect(abs(contrast(orange, white) - 2.936) < 0.001)
        let colors = CelebrationButtonColors(accent: orange, isPressed: false)
        #expect(colors.fill == orange)
        #expect(contrast(colors.fill, colors.ink) >= 4.5)
        #expect(colors.ink.red == 0)
    }
    @Test @MainActor func resolvedSystemColorsRespectAppearance() {
        for scheme in [ColorScheme.light, .dark] {
            var environment = EnvironmentValues()
            environment.colorScheme = scheme
            for accent in [Color.accentColor, .red, .orange, .yellow, .green, .mint,
                           .teal, .cyan, .blue, .indigo, .purple, .pink, .gray] {
                for pressed in [false, true] {
                    let colors = CelebrationButtonColors(accent: accent.resolve(in: environment),
                                                         isPressed: pressed)
                    #expect(contrast(colors.fill, colors.ink) >= 4.5)
                }
            }
        }
    }
    @Test func paletteAndCustomColorsRetainReadableInkWhenPressed() {
        // Default, all nine iOS palette values, black/white and a dense arbitrary
        // custom-color grid, including the black/white crossover luminance.
        let palette = [0xFF6600, 0xE74C3C, 0xE67E22, 0xF39C12, 0x2ECC71,
                       0x1ABC9C, 0x3498DB, 0x9B59B6, 0xC0392B, 0x16A085,
                       0x000000, 0xFFFFFF, 0x757575]
        var accents = palette.map { hex in Color.Resolved(red: Float((hex >> 16) & 255) / 255,
            green: Float((hex >> 8) & 255) / 255, blue: Float(hex & 255) / 255) }
        for red in stride(from: Float(0), through: 1, by: 0.1) {
            for green in stride(from: Float(0), through: 1, by: 0.1) {
                for blue in stride(from: Float(0), through: 1, by: 0.1) {
                    accents.append(Color.Resolved(red: red, green: green, blue: blue))
                }
            }
        }
        for accent in accents {
            for pressed in [false, true] {
                let colors = CelebrationButtonColors(accent: accent, isPressed: pressed)
                #expect(colors.fill.opacity == 1)
                #expect(contrast(colors.fill, colors.ink) >= 4.5)
                if !pressed { #expect(colors.fill == accent) }
            }
        }
    }
}
