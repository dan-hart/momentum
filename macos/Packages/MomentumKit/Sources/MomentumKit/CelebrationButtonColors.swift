// SPDX-License-Identifier: GPL-3.0-or-later
import SwiftUI

/// Resolve the actual selected accent once, then paint an opaque sRGB fill and
/// matching ink. Native prominent styles can adjust their fill and foreground
/// independently, so their implicit white ink is unsafe for bright accents.
struct CelebrationButtonColors {
    let fill: Color.Resolved
    let ink: Color.Resolved

    init(accent: Color.Resolved, isPressed: Bool) {
        let factor: Float = isPressed ? 0.88 : 1
        func component(_ value: Float) -> Float { min(1, max(0, value)) * factor }
        if isPressed || [accent.red, accent.green, accent.blue].contains(where: { !(0...1).contains($0) }) {
            fill = Color.Resolved(red: component(accent.red), green: component(accent.green),
                                  blue: component(accent.blue))
        } else {
            var opaque = accent
            opaque.opacity = 1
            fill = opaque
        }
        // Color.Resolved exposes linear sRGB components. Do not linearize them
        // a second time; WCAG luminance is their weighted sum.
        let luminance = 0.2126 * Double(fill.linearRed) + 0.7152 * Double(fill.linearGreen)
            + 0.0722 * Double(fill.linearBlue)
        let blackContrast = (luminance + 0.05) / 0.05
        let whiteContrast = 1.05 / (luminance + 0.05)
        let value: Float = blackContrast >= whiteContrast ? 0 : 1
        ink = Color.Resolved(red: value, green: value, blue: value)
    }
}

/// The action remains a native SwiftUI Button (keyboard, accessibility and focus),
/// with the existing rounded shape and a fill whose pressed state we can measure.
struct CelebrationButtonStyle: ButtonStyle {
    @Environment(\.self) private var environment
    @Environment(\.isEnabled) private var isEnabled

    func makeBody(configuration: Configuration) -> some View {
        let colors = CelebrationButtonColors(accent: Color.accentColor.resolve(in: environment),
                                             isPressed: configuration.isPressed)
        configuration.label
            .symbolRenderingMode(.monochrome)
            .foregroundStyle(Color(colors.ink))
            .padding(.horizontal, 16)
            .padding(.vertical, 10)
            .frame(minHeight: 44)
            .background(Color(colors.fill), in: RoundedRectangle(cornerRadius: 14))
            .overlay {
                if environment.colorSchemeContrast == .increased {
                    RoundedRectangle(cornerRadius: 14).strokeBorder(Color(colors.ink), lineWidth: 2)
                }
            }
            .contentShape(RoundedRectangle(cornerRadius: 14))
            .opacity(isEnabled ? 1 : 0.5)
    }
}
