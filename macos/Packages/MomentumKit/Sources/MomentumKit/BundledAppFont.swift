// SPDX-License-Identifier: GPL-3.0-or-later
import CoreText
import Foundation

/// Curated offline choices. Values are compatible with the existing local font-name
/// preference; installed-font selections continue to use their original face names.
public enum BundledAppFont: String, CaseIterable, Identifiable, Sendable {
    case system = ""
    case atkinsonMono = "AtkinsonHyperlegibleMono-Regular"
    case openDyslexic = "OpenDyslexic-Regular"

    public var id: String { rawValue }
    public var label: String {
        switch self {
        case .system: String(localized: "System Font", table: "Settings", bundle: .module)
        case .atkinsonMono: "Atkinson Hyperlegible Mono"
        case .openDyslexic: "OpenDyslexic"
        }
    }

    public static var customLabel: String { String(localized: "Custom Font", table: "Settings", bundle: .module) }

    static let resourceNames = [
        "AtkinsonHyperlegibleMono-Regular.ttf", "AtkinsonHyperlegibleMono-Bold.ttf",
        "AtkinsonHyperlegibleMono-Italic.ttf", "AtkinsonHyperlegibleMono-BoldItalic.ttf",
        "OpenDyslexic-Regular.otf", "OpenDyslexic-Bold.otf",
        "OpenDyslexic-Italic.otf", "OpenDyslexic-BoldItalic.otf",
    ]

    static func resourceURL(_ name: String) -> URL? {
        // SwiftPM processes resources into the module bundle on both Apple platforms.
        Bundle.module.url(forResource: name, withExtension: nil)
    }

    private static let registration: Void = {
        for name in resourceNames {
            if let url = resourceURL(name) {
                // Process-only registration never installs fonts in the user's system.
                // Already registered fonts are harmless; resolution retains its system fallback.
                CTFontManagerRegisterFontsForURL(url as CFURL, .process, nil)
            }
        }
    }()

    public static func register() { _ = registration }
}
