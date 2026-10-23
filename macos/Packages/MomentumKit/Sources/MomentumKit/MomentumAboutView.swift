// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import SwiftUI
#if os(macOS)
import AppKit
#else
import UIKit
#endif

/// The displayed and copied metadata deliberately excludes store, account and sync information.
public enum MomentumAppDetails {
    public static func text(bundle: Bundle = .main, platform: String) -> String {
        var lines = ["Momentum", platform]
        if let version = bundle.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String { lines.append(String(localized: "Version", bundle: .module) + ": " + version) }
        if let build = bundle.object(forInfoDictionaryKey: "CFBundleVersion") as? String { lines.append(String(localized: "Build", bundle: .module) + ": " + build) }
        return lines.joined(separator: "\n")
    }
}

public struct MomentumAboutView: View {
    public init() {}
    private var platform: String {
        #if os(macOS)
        "macOS"
        #else
        "iOS"
        #endif
    }
    public var body: some View {
        Form {
            Section {
                VStack(alignment: .leading, spacing: 12) {
                    appIcon.frame(width: 72, height: 72).accessibilityHidden(true)
                    Text("Momentum").font(.largeTitle.bold())
                    Text(String(localized: "Make room for what matters.", bundle: .module))
                    Text(MomentumAppDetails.text(platform: platform)).font(.caption).foregroundStyle(.secondary).textSelection(.enabled)
                }.padding(.vertical, 16)
            }.settingsTarget("about.app")
            Section(String(localized: "Our values", bundle: .module)) {
                Label(String(localized: "Offline, without an account", bundle: .module), systemImage: "wifi.slash")
                Text(String(localized: "Your tasks stay usable wherever you are.", bundle: .module))
                Label(String(localized: "Privacy and optional sync", bundle: .module), systemImage: "lock.shield")
                Text(String(localized: "Sync is your choice. No advertising or tracking.", bundle: .module))
                Label(String(localized: "Free and open source", bundle: .module), systemImage: "chevron.left.forwardslash.chevron.right")
                Text(String(localized: "GPL-3.0-or-later. Free to build, use and self-host.", bundle: .module))
            }
            Section(String(localized: "Developer & Project", bundle: .module)) {
                Text("Dan Hart")
                Link(SettingsDiscovery.localized("Source Code"), destination: URL(string: "https://github.com/dan-hart/momentum")!).settingsTarget("about.source")
                Link(SettingsDiscovery.localized("Report an Issue"), destination: URL(string: "https://github.com/dan-hart/momentum/issues")!).settingsTarget("about.issues")
                Button(String(localized: "Copy app details", bundle: .module)) {
                    #if os(macOS)
                    NSPasteboard.general.clearContents()
                    NSPasteboard.general.setString(MomentumAppDetails.text(platform: platform), forType: .string)
                    #else
                    UIPasteboard.general.string = MomentumAppDetails.text(platform: platform)
                    #endif
                }
            }
            Section(SettingsDiscovery.localized("License")) {
                notice("Momentum-LICENSE", title: "GPL-3.0-or-later")
            }.settingsTarget("about.license")
            Section(SettingsDiscovery.localized("Acknowledgments")) {
                notice("Momentum-NOTICE.md", title: SettingsDiscovery.localized("Acknowledgments"))
                notice("MIT", title: "MIT")
                #if os(macOS)
                notice("Sparkle-LICENSE", title: "Sparkle · MIT and bundled licenses")
                #endif
                notice("AGPL-3.0-only", title: "LibreSync · AGPL-3.0")
                notice("CC0-1.0", title: "VCSL · CC0-1.0")
            }.settingsTarget("about.acknowledgments")
        }
        .formStyle(.grouped)
    }
    @ViewBuilder private var appIcon: some View {
        #if os(macOS)
        Image(nsImage: NSImage(named: NSImage.applicationIconName) ?? NSImage()).resizable().scaledToFit()
        #else
        if let icons = Bundle.main.infoDictionary?["CFBundleIcons"] as? [String: Any],
           let primary = icons["CFBundlePrimaryIcon"] as? [String: Any],
           let names = primary["CFBundleIconFiles"] as? [String],
           let name = names.last, let icon = UIImage(named: name) {
            Image(uiImage: icon).resizable().scaledToFit().clipShape(RoundedRectangle(cornerRadius: 16))
        } else {
            Image(systemName: "checkmark.circle.fill").resizable().scaledToFit().foregroundStyle(.tint)
        }
        #endif
    }
    private func notice(_ resource: String, title: String) -> some View {
        DisclosureGroup(title) {
            Text((Bundle.module.url(forResource: resource, withExtension: "txt").flatMap { try? String(contentsOf: $0, encoding: .utf8) }) ?? title)
                .font(.caption).textSelection(.enabled)
        }
    }
}
