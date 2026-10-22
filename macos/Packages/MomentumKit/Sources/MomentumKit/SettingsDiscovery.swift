// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import SwiftUI

public enum SettingsDiscovery {
    public struct Category: Identifiable, Hashable, Sendable {
        public let id: String
        public let title: String
        public let symbol: String
        public var subtitle: String {
            let subtitles = [
                "tasks": "Grouping, sorting, and completed tasks",
                "appearance": "Colors, text, and list density",
                "sounds": "Completion sounds, previews, and feedback",
                "notifications": "Permissions, reminders, and morning summary",
                "sync": "Connections, devices, and recovery",
                "data": "Import, export, and restore",
                "integrations": "Shortcuts and Apple Reminders",
                "about": "Project, version, and licenses"
            ]
            return SettingsDiscovery.localized(subtitles[id] ?? "")
        }
    }
    public static func localized(_ key: String) -> String {
        Bundle.module.localizedString(forKey: key, value: key, table: "Settings")
    }
    public static var categories: [Category] { [
        .init(id: "tasks", title: localized("Tasks & Planning"), symbol: "checklist"),
        .init(id: "appearance", title: localized("Appearance"), symbol: "paintpalette"),
        .init(id: "sounds", title: localized("Sounds & Feedback"), symbol: "speaker.wave.2"),
        .init(id: "notifications", title: localized("Notifications"), symbol: "bell"),
        .init(id: "sync", title: localized("Sync & Devices"), symbol: "arrow.triangle.2.circlepath"),
        .init(id: "data", title: localized("Data & Backups"), symbol: "externaldrive"),
        .init(id: "integrations", title: localized("App & Integrations"), symbol: "app.connected.to.app.below.fill"),
        .init(id: "about", title: localized("About & Help"), symbol: "info.circle")
    ] }
    private static let metadata: [String: (String, String)] = [
        "tasks.grouping": ("Group By", ""),
        "tasks.sort": ("Sort By", ""),
        "tasks.sort-direction": ("Sort direction", ""),
        "tasks.upcoming": ("Upcoming range", ""),
        "tasks.day-period-visibility": ("Show Morning and Evening tasks in Today", "night evening morning Nacht Abend Morgen"),
        "tasks.archive": ("Archive completed tasks immediately", ""),
        "tasks.dynamic-messages": ("Dynamic empty-state messages", ""),
        "appearance.colors": ("Color-code projects and tags", ""),
        "appearance.accent": ("Accent Color", ""),
        "appearance.compact": ("Compact rows", ""),
        "appearance.font": ("Font", "text size Schrift Textgröße"),
        "appearance.content-size": ("Content size", ""),
        "appearance.interface-size": ("Interface size", ""),
        "appearance.reset": ("Reset Typography", ""),
        "sounds.enabled": ("Completion sound", ""),
        "sounds.selection": ("Choose completion sound", ""),
        "sounds.volume": ("Completion sound volume", ""),
        "sounds.haptics": ("Haptic feedback", ""),
        "notifications.badge": ("App Badge", "badge count Anzahl"),
        "notifications.permission": ("Notification permission", ""),
        "notifications.reminders": ("Task reminders", ""),
        "notifications.morning-summary": ("Morning summary", ""),
        "sync.provider": ("Sync method", ""),
        "sync.nextcloud": ("Nextcloud connection", ""),
        "sync.libresync": ("LibreSync", ""),
        "sync.automatic": ("Sync automatically", ""),
        "sync.background": ("Sync in the background", "background Hintergrund"),
        "sync.compression": ("Compress the sync file", ""),
        "sync.devices": ("Linked devices", ""),
        "sync.status": ("Sync status and retry", ""),
        "sync.settings": ("Sync app settings", ""),
        "sync.folder-copy": ("Copy to Momentum folder", ""),
        "data.tasks-import": ("Import Backup", "backup restore Sicherung wiederherstellen"),
        "data.tasks-export": ("Export Backup", "backup save Sicherung speichern"),
        "data.settings-import": ("Import app settings", ""),
        "data.settings-export": ("Export app settings", ""),
        "data.summary": ("Data storage", ""),
        "integrations.shortcuts": ("Keyboard Shortcuts", ""),
        "integrations.startup": ("Keep running in the background", ""),
        "integrations.count": ("Dock badge", ""),
        "integrations.dock": ("Dock badge", ""),
        "integrations.menu-bar": ("Show in the menu bar", ""),
        "integrations.reminders-import": ("Apple Reminders", ""),
        "about.app": ("About Momentum", ""),
        "about.source": ("Source Code", ""),
        "about.issues": ("Report an Issue", ""),
        "about.license": ("License", ""),
        "about.acknowledgments": ("Acknowledgments", "")
    ]
    public static func entries(platform: SettingsPlatform) -> [SettingsEntry] {
        settingsDirectory().compactMap { destination in
            let unavailable: Set<String> = platform == .macos ? ["appearance.theme", "integrations.system-search", "integrations.count"] : ["appearance.theme", "integrations.system-search", "data.summary"]
            guard !unavailable.contains(destination.id), destination.platforms.contains(platform), let metadata = metadata[destination.id],
                  let category = categories.first(where: { $0.id == destination.category }) else { return nil }
            // The directory contains labels only, never preference values or user data.
            return SettingsEntry(id: destination.id, category: category.title, title: localized(metadata.0),
                                 description: category.title, aliases: metadata.1.isEmpty ? [] : [metadata.1])
        }
    }
    public static func results(_ query: String, platform: SettingsPlatform) -> [SettingsEntry] {
        searchSettings(query: query, entries: entries(platform: platform))
    }
    public static func category(for destination: String) -> String { String(destination.split(separator: ".").first ?? "tasks") }
}

private struct SettingsTargetKey: EnvironmentKey { static let defaultValue: String? = nil }
extension EnvironmentValues {
    public var settingsDestination: String? {
        get { self[SettingsTargetKey.self] }
        set { self[SettingsTargetKey.self] = newValue }
    }
}
private struct SettingsTarget: ViewModifier {
    let id: String
    @Environment(\.settingsDestination) private var destination
    @AccessibilityFocusState private var focused: Bool
    func body(content: Content) -> some SwiftUI.View {
        content.id(id).accessibilityFocused($focused)
            .overlay(RoundedRectangle(cornerRadius: 6).stroke(destination == id ? Color.accentColor : .clear, lineWidth: 2).allowsHitTesting(false))
            .task(id: destination) { if destination == id { await Task.yield(); focused = true } }
    }
}
extension SwiftUI.View {
    public func settingsTarget(_ id: String) -> some SwiftUI.View { modifier(SettingsTarget(id: id)) }
}

/// Settings owns its own search/navigation state; task search never enters this value.
public struct SettingsRouteState: Equatable, Sendable {
    public var category: String? = "tasks"
    public var destination: String?
    public var query = ""
    public init() {}
    public mutating func open(_ id: String?, query: String? = nil, platform: SettingsPlatform) {
        if let query { self.query = query }
        else if id != nil { self.query = "" }
        guard let id else { destination = nil; return }
        let requestedCategory = SettingsDiscovery.category(for: id)
        category = SettingsDiscovery.categories.contains { $0.id == requestedCategory } ? requestedCategory : "tasks"
        // Runtime capability can disappear between matching and activation. Keep the
        // category useful without targeting a nonexistent or unsupported control.
        destination = SettingsDiscovery.entries(platform: platform).contains { $0.id == id } ? id : nil
    }
}
