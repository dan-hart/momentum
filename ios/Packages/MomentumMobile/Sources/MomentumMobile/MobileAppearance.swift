// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation

/// Relative iOS presentation preferences stay separate from desktop point sizes.
public struct MobileAppearance: Equatable, Sendable {
    public static let contentScaleKey = "ios-content-scale"
    public static let interfaceScaleKey = "ios-interface-scale"
    public static let hapticsKey = "ios-haptics"
    public static let scaleRange = 0.8...1.6
    public let contentScale: Double
    public let interfaceScale: Double
    public let haptics: Bool

    public init(contentScale: Double = 1, interfaceScale: Double = 1, haptics: Bool = true) {
        self.contentScale = Self.validScale(contentScale)
        self.interfaceScale = Self.validScale(interfaceScale)
        self.haptics = haptics
    }

    public init(defaults: UserDefaults) {
        self.init(contentScale: defaults.object(forKey: Self.contentScaleKey) as? Double ?? 1,
                  interfaceScale: defaults.object(forKey: Self.interfaceScaleKey) as? Double ?? 1,
                  haptics: defaults.object(forKey: Self.hapticsKey) as? Bool ?? true)
    }

    private static func validScale(_ value: Double) -> Double {
        value.isFinite ? min(scaleRange.upperBound, max(scaleRange.lowerBound, value)) : 1
    }
}
