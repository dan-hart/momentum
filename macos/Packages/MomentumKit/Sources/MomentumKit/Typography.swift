// SPDX-License-Identifier: GPL-3.0-or-later
import AppKit
import SwiftUI

public enum FontArea: Sendable { case content, interface }

public enum FontRole: Sendable {
    case body, caption, headline, title, title2, title3

    var scale: Double {
        switch self {
        case .body, .headline: return 1
        case .caption: return 11.0 / 13
        case .title: return 24.0 / 13
        case .title2: return 17.0 / 13
        case .title3: return 15.0 / 13
        }
    }
}

/// Local typography choices, independent of task data and sync. Store the PostScript
/// name so the selected face (including italic/bold) survives an app relaunch.
public struct AppTypography: Equatable, Sendable {
    public static let sizeRange = 10.0...32.0
    public let fontName: String
    public let contentSize: Double
    public let interfaceSize: Double

    public init(fontName: String = "", contentSize: Double = 13, interfaceSize: Double = 13) {
        self.fontName = fontName
        self.contentSize = Self.validSize(contentSize)
        self.interfaceSize = Self.validSize(interfaceSize)
    }

    public init(defaults: UserDefaults) {
        self.init(fontName: defaults.string(forKey: PrefKey.appFontName) ?? "",
                  contentSize: (defaults.object(forKey: PrefKey.contentFontSize) as? NSNumber)?.doubleValue ?? 13,
                  interfaceSize: (defaults.object(forKey: PrefKey.interfaceFontSize) as? NSNumber)?.doubleValue ?? 13)
    }

    private static func validSize(_ size: Double) -> Double {
        size.isFinite ? min(sizeRange.upperBound, max(sizeRange.lowerBound, size)) : 13
    }

    public func resolvedFont(in area: FontArea, role: FontRole = .body) -> NSFont {
        let size = max(10, (area == .content ? contentSize : interfaceSize) * role.scale)
        return NSFont(name: fontName, size: size) ?? NSFont.systemFont(ofSize: size)
    }

    public func font(in area: FontArea, role: FontRole = .body) -> Font {
        let font = Font(resolvedFont(in: area, role: role))
        switch role {
        case .headline, .title, .title2: return font.weight(.semibold)
        default: return font
        }
    }

    public static func saveSelection(_ font: NSFont, defaults: UserDefaults = .standard) {
        defaults.set(font.fontName, forKey: PrefKey.appFontName)
        defaults.set(validSize(font.pointSize), forKey: PrefKey.contentFontSize)
    }

    public static func reset(defaults: UserDefaults = .standard) {
        for key in [PrefKey.appFontName, PrefKey.contentFontSize, PrefKey.interfaceFontSize] {
            defaults.removeObject(forKey: key)
        }
    }
}

private struct AppTypographyKey: EnvironmentKey {
    static let defaultValue = AppTypography()
}

public extension EnvironmentValues {
    var appTypography: AppTypography {
        get { self[AppTypographyKey.self] }
        set { self[AppTypographyKey.self] = newValue }
    }
}

private struct TypographyEnvironment: ViewModifier {
    @AppStorage(PrefKey.appFontName) private var name = ""
    @AppStorage(PrefKey.contentFontSize) private var contentSize = 13.0
    @AppStorage(PrefKey.interfaceFontSize) private var interfaceSize = 13.0

    func body(content: Content) -> some View {
        let typography = AppTypography(fontName: name, contentSize: contentSize, interfaceSize: interfaceSize)
        content
            .environment(\.appTypography, typography)
            .font(typography.font(in: .interface))
    }
}

private struct AppFont: ViewModifier {
    @Environment(\.appTypography) private var typography
    let role: FontRole
    let area: FontArea

    func body(content: Content) -> some View {
        content.font(typography.font(in: area, role: role))
    }
}

public extension View {
    /// Install at each scene root so changes update all open windows immediately.
    func momentumTypography() -> some View { modifier(TypographyEnvironment()) }

    func appFont(_ role: FontRole = .body, area: FontArea = .interface) -> some View {
        modifier(AppFont(role: role, area: area))
    }
}
