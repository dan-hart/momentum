// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
import Foundation

/// Device-local feedback choices. Stable slugs match the other native clients.
public enum CompletionSoundChoice: String, CaseIterable, Identifiable, Sendable {
    case woodPop = "wood-pop"
    case doubleCheck = "double-check"
    case roundedLift = "rounded-lift"
    case pocketPiano = "pocket-piano"
    case stringFlick = "string-flick"
    case brightFinish = "bright-finish"
    case harpSnap = "harp-snap"
    case clickAndGlow = "click-and-glow"
    case tinyTaDa = "tiny-ta-da"
    case warmSpark = "warm-spark"

    case softKnock = "soft-knock"
    case littleLift = "little-lift"
    case feltFinish = "felt-finish"
    case warmThumb = "warm-thumb"
    case gentlePair = "gentle-pair"
    case smallBloom = "small-bloom"
    case sunnyTap = "sunny-tap"
    case pocketHarp = "pocket-harp"
    case easyLanding = "easy-landing"
    case quietCheer = "quiet-cheer"

    public var isLegacy: Bool { Self.allCases.firstIndex(of: self)! < 10 }
    public static var candidates: [Self] { allCases.filter { !$0.isLegacy } }
    public static let defaultChoice: Self = .littleLift
    public var id: String { rawValue }

    /// Present unknown future values as the default without overwriting the saved slug.
    public static func resolve(_ rawValue: String?) -> Self {
        rawValue.flatMap(Self.init(rawValue:)) ?? defaultChoice
    }

    public var label: String { label(locale: .current) }

    /// An explicit locale also supports previews and native hosted language checks.
    public func label(locale: Locale) -> String {
        let language = locale.language.languageCode?.identifier ?? "en"
        let bundle = Bundle.module.path(forResource: language, ofType: "lproj")
            .flatMap(Bundle.init(path:)) ?? .module
        return switch self {
        case .softKnock: String(localized: "Soft Knock", bundle: bundle, locale: locale)
        case .littleLift: String(localized: "Little Lift", bundle: bundle, locale: locale)
        case .feltFinish: String(localized: "Felt Finish", bundle: bundle, locale: locale)
        case .warmThumb: String(localized: "Warm Thumb", bundle: bundle, locale: locale)
        case .gentlePair: String(localized: "Gentle Pair", bundle: bundle, locale: locale)
        case .smallBloom: String(localized: "Small Bloom", bundle: bundle, locale: locale)
        case .sunnyTap: String(localized: "Sunny Tap", bundle: bundle, locale: locale)
        case .pocketHarp: String(localized: "Pocket Harp", bundle: bundle, locale: locale)
        case .easyLanding: String(localized: "Easy Landing", bundle: bundle, locale: locale)
        case .quietCheer: String(localized: "Quiet Cheer", bundle: bundle, locale: locale)
        case .woodPop: String(localized: "1 Wood Pop", bundle: bundle, locale: locale)
        case .doubleCheck: String(localized: "2 Double Check", bundle: bundle, locale: locale)
        case .roundedLift: String(localized: "3 Rounded Lift", bundle: bundle, locale: locale)
        case .pocketPiano: String(localized: "4 Pocket Piano", bundle: bundle, locale: locale)
        case .stringFlick: String(localized: "5 String Flick", bundle: bundle, locale: locale)
        case .brightFinish: String(localized: "6 Bright Finish", bundle: bundle, locale: locale)
        case .harpSnap: String(localized: "7 Harp Snap", bundle: bundle, locale: locale)
        case .clickAndGlow: String(localized: "8 Click and Glow", bundle: bundle, locale: locale)
        case .tinyTaDa: String(localized: "9 Tiny Ta-Da", bundle: bundle, locale: locale)
        case .warmSpark: String(localized: "10 Warm Spark", bundle: bundle, locale: locale)
        }
    }

    public var url: URL? {
        Bundle.module.url(forResource: rawValue, withExtension: "wav")
    }
}
