// SPDX-License-Identifier: GPL-3.0-or-later
import DHFlatUIColors
import Foundation

public struct AccentChoice: Identifiable, Equatable, Sendable {
    public static let preferenceKey = "ios-accent-color"
    public static let momentum = AccentChoice(id: "momentum", name: "Momentum", hex: "#FF6600")
    public let id: String
    public let name: String
    public let hex: String

    /// Keep the default orange exact in every appearance, as requested.
    public func renderedColor(on backgrounds: [RGBColor], isDark: Bool,
                              increasedContrast: Bool) -> RGBColor {
        let base = RGBColor(hex: hex) ?? RGBColor(hex: Self.momentum.hex)!
        if id == Self.momentum.id { return base }
        let ratio = increasedContrast ? 7.05 : 4.55
        return base.accessible(on: backgrounds, minimumRatio: ratio)
    }

    public static func resolve(_ id: String) -> Self {
        AccentPalette.all.lazy.flatMap(\.colors).first { $0.id == id } ?? .momentum
    }
    public static func load(_ defaults: UserDefaults) -> Self {
        resolve(defaults.string(forKey: preferenceKey) ?? "momentum")
    }
}

public struct AccentPalette: Identifiable, Sendable {
    public let id: String
    public let name: String
    public let colors: [AccentChoice]

    /// AsNeeded's nine-color selection, in spectrum order. DHFlatUIColors remains
    /// the source of names/values; retain existing IDs for saved selections.
    public static let all: [Self] = {
        let palette = DHFlatUIColors.Palette.flatUiV1
        let curated: [DHFlatUIColors.Flatuiv1Palette] = [
            .alizarin, .carrot, .orange, .emerald, .turquoise,
            .peterRiver, .amethyst, .pomegranate, .greenSea
        ]
        return [Self(id: palette.name, name: palette.name, colors: curated.map(\.info).map { color in
            AccentChoice(id: palette.name + ":" + color.name, name: color.name, hex: color.hex.uppercased())
        })]
    }()
}

/// WCAG sRGB luminance and contrast, independent of SwiftUI and presentation state.
public struct RGBColor: Equatable, Sendable {
    public let red: Double
    public let green: Double
    public let blue: Double
    public static let black = Self(red: 0, green: 0, blue: 0)
    public static let white = Self(red: 1, green: 1, blue: 1)

    public init(red: Double, green: Double, blue: Double) {
        self.red = Self.clamp(red); self.green = Self.clamp(green); self.blue = Self.clamp(blue)
    }
    public init?(hex: String) {
        let value = hex.hasPrefix("#") ? String(hex.dropFirst()) : hex
        guard value.count == 6, let number = UInt32(value, radix: 16) else { return nil }
        self.init(red: Double((number >> 16) & 255) / 255,
                  green: Double((number >> 8) & 255) / 255, blue: Double(number & 255) / 255)
    }
    private static func clamp(_ value: Double) -> Double { value.isFinite ? min(1, max(0, value)) : 0 }
    public var luminance: Double {
        func linear(_ value: Double) -> Double {
            value <= 0.04045 ? value / 12.92 : pow((value + 0.055) / 1.055, 2.4)
        }
        return 0.2126 * linear(red) + 0.7152 * linear(green) + 0.0722 * linear(blue)
    }
    public func contrast(with other: Self) -> Double {
        (max(luminance, other.luminance) + 0.05) / (min(luminance, other.luminance) + 0.05)
    }
    /// Black or white is always legible on an opaque selected-color swatch/fill.
    public var onColor: Self { contrast(with: .black) >= contrast(with: .white) ? .black : .white }

    /// Preserve the selected base and change only its rendered tone when contrast requires it.
    /// Native surfaces within one appearance share a light or dark family.
    public func accessible(on backgrounds: [Self], minimumRatio: Double = 4.5) -> Self {
        guard !backgrounds.isEmpty else { return self }
        func acceptable(_ color: Self) -> Bool { backgrounds.allSatisfy { color.contrast(with: $0) >= minimumRatio } }
        if acceptable(self) { return self }
        var candidates: [(Double, Self)] = []
        for end in [Self.black, .white] where acceptable(end) {
            var lower = 0.0, upper = 1.0
            for _ in 0..<28 {
                let fraction = (lower + upper) / 2
                if acceptable(mixed(with: end, fraction: fraction)) { upper = fraction }
                else { lower = fraction }
            }
            candidates.append((upper, mixed(with: end, fraction: upper)))
        }
        return candidates.min(by: { $0.0 < $1.0 })?.1 ?? onColor
    }

    private func mixed(with other: Self, fraction: Double) -> Self {
        Self(red: red + (other.red - red) * fraction,
             green: green + (other.green - green) * fraction,
             blue: blue + (other.blue - blue) * fraction)
    }
}
