// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumMobile
import SwiftUI
import UIKit

extension RGBColor {
    var color: Color { Color(.sRGB, red: red, green: green, blue: blue) }
    var uiColor: UIColor { UIColor(red: red, green: green, blue: blue, alpha: 1) }
}

/// Resolve against actual native surfaces, including elevated/grouped dark backgrounds.
/// A small margin protects the minimum against rendering quantization.
enum AccentTheme {
    static func ink(hex: String) -> Color {
        let base = RGBColor(hex: hex) ?? RGBColor(hex: AccentChoice.momentum.hex)!
        return readable(base.uiColor)
    }

    static func accent(_ choice: AccentChoice) -> Color {
        readable((RGBColor(hex: choice.hex) ?? .black).uiColor, choice: choice)
    }

    static func onAccent(_ choice: AccentChoice) -> Color {
        readable((RGBColor(hex: choice.hex) ?? .black).uiColor, choice: choice, foreground: true)
    }

    static var secondaryText: Color { readable(.secondaryLabel) }
    static var errorText: Color { readable(.systemRed) }

    private static func readable(_ base: UIColor, choice: AccentChoice? = nil,
                                 foreground: Bool = false) -> Color {
        return Color(uiColor: UIColor { traits in
            let surfaces: [UIColor] = [.systemBackground, .secondarySystemBackground,
                .tertiarySystemBackground, .systemGroupedBackground,
                .secondarySystemGroupedBackground, .tertiarySystemGroupedBackground]
            func resolved(_ color: UIColor, over background: RGBColor = .black) -> RGBColor {
                var red: CGFloat = 0, green: CGFloat = 0, blue: CGFloat = 0, alpha: CGFloat = 0
                color.resolvedColor(with: traits).getRed(&red, green: &green, blue: &blue, alpha: &alpha)
                return RGBColor(red: red * alpha + background.red * (1 - alpha),
                                green: green * alpha + background.green * (1 - alpha),
                                blue: blue * alpha + background.blue * (1 - alpha))
            }
            let backgrounds = surfaces.map { resolved($0) }
            if let choice {
                let rendered = choice.renderedColor(on: backgrounds, isDark: traits.userInterfaceStyle == .dark,
                    increasedContrast: traits.accessibilityContrast == .high)
                // White brand-button ink, except when Increase Contrast requests
                // the calculated foreground. Resolve dynamically as settings change.
                if foreground, choice == .momentum, traits.accessibilityContrast != .high {
                    return UIColor.white
                }
                return (foreground ? rendered.onColor : rendered).uiColor
            }
            return resolved(base, over: resolved(.systemBackground)).accessible(on: backgrounds,
                minimumRatio: traits.accessibilityContrast == .high ? 7.05 : 5.5).uiColor
        })
    }
}

private struct AccentThemeModifier: ViewModifier {
    @AppStorage(AccentChoice.preferenceKey) private var selected = AccentChoice.momentum.id
    func body(content: Content) -> some View {
        let ink = AccentTheme.accent(AccentChoice.resolve(selected))
        content.tint(ink).accentColor(ink)
    }
}

extension View {
    func momentumAccent() -> some View { modifier(AccentThemeModifier()) }
}

/// Apply the contrasting ink to the symbol itself: native toolbar promotion can
/// override a foreground style placed on the enclosing Button.
struct SheetCommitButton: View {
    let title: String
    let symbol: String
    let action: () -> Void
    @AppStorage(AccentChoice.preferenceKey) private var selected = AccentChoice.momentum.id

    var body: some View {
        Button(action: action) {
            Image(systemName: symbol)
                // Let the native toolbar size its symbol independently of the
                // app's scaled content font; task text keeps Dynamic Type.
                .font(nil)
                .symbolRenderingMode(.palette)
                .foregroundStyle(AccentTheme.onAccent(AccentChoice.resolve(selected)))
                .accessibilityHidden(true)
        }
        .accessibilityLabel(Text(title))
        .buttonStyle(.borderedProminent)
    }
}
