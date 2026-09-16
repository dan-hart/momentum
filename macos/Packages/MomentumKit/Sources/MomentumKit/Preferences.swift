// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// Non-secret preferences in UserDefaults, under the same key names the GNOME app uses
// in GSettings, so the two configurations read alike. Secrets go to the Keychain.
import Foundation
import MomentumCore
import SwiftUI

public enum PrefKey {
    public static let syncMethod = "sync-method"
    public static let nextcloudServer = "nextcloud-server"
    public static let nextcloudUser = "nextcloud-user"
    public static let nextcloudFolder = "nextcloud-folder"
    public static let autoSync = "auto-sync"
    public static let compress = "compress"
    public static let syncEnabled = "sync-enabled"
    public static let projectsCollapsed = "projects-collapsed"
    public static let tagsCollapsed = "tags-collapsed"
    public static let groupBy = "group-by"
    public static let taskSort = "task-sort"
    public static let sortDirection = "sort-direction"
    public static let upcomingRange = "upcoming-range"
    public static let colorfulLabels = "colorful-labels"
    public static let runInBackground = "run-in-background"
    public static let showInMenuBar = "show-in-menu-bar"
    public static let modifierKey = "modifier-key"
    public static let autoArchive = "auto-archive"
    public static let morningSummaryEnabled = "morning-summary-enabled"
    public static let morningSummaryHour = "morning-summary-hour"
    public static let morningSummaryMinute = "morning-summary-minute"
    public static let p2pEnabled = "p2p-enabled"
    public static let sidebarVisible = "sidebar-visible"
    // Mac-only presentation preferences; never written to the shared task store.
    public static let dockBadgeMode = "dock-badge-mode"
    public static let appFontName = "app-font-name"
    public static let contentFontSize = "content-font-size"
    public static let interfaceFontSize = "interface-font-size"
}

/// Only one transport owns sync. Add future services here rather than separate switches.
public enum SyncMethod: String, CaseIterable, Identifiable {
    case off, nextcloud, libresync
    public var id: String { rawValue }
    public var label: String {
        switch self {
        case .off: String(localized: "Off", bundle: .module)
        case .nextcloud: "Nextcloud"
        case .libresync: "LibreSync"
        }
    }
}

public enum DockBadgeMode: String, CaseIterable, Identifiable {
    case dueToday, todayIncludingOverdue, none
    public var id: String { rawValue }

    public var label: String {
        switch self {
        case .dueToday: String(localized: "Due or scheduled today", bundle: .module)
        case .todayIncludingOverdue: String(localized: "Today, including overdue", bundle: .module)
        case .none: String(localized: "None", bundle: .module)
        }
    }
}

public enum ModifierKey: String, CaseIterable, Identifiable {
    case command, control, option
    public var id: String { rawValue }

    public var label: String {
        switch self {
        case .command: return String(localized: "Command", bundle: .module)
        case .control: return String(localized: "Control", bundle: .module)
        case .option: return String(localized: "Option", bundle: .module)
        }
    }
    /// The symbol shown in hints: "⌘", "⌃", "⌥".
    public var symbol: String {
        switch self {
        case .command: return "⌘"
        case .control: return "⌃"
        case .option: return "⌥"
        }
    }
    public var modifiers: EventModifiers {
        switch self {
        case .command: return .command
        case .control: return .control
        case .option: return .option
        }
    }
}

/// Typed access to the defaults, with the same defaults as the GSettings schema.
public struct Preferences {
    /// Bundle-ID changes move UserDefaults to a new domain. Copy once, preserving
    /// any choices already made in the new app and leaving the old domain untouched.
    public static func migrateLegacyDomain(
        _ defaults: UserDefaults = .standard,
        from legacyDomain: String = "io.github.dan_hart.Momentum",
        to targetDomain: String = "com.codedbydan.Momentum"
    ) {
        let marker = "legacy-bundle-preferences-migrated"
        var current = defaults.persistentDomain(forName: targetDomain) ?? [:]
        guard current[marker] as? Bool != true else { return }
        for (key, value) in defaults.persistentDomain(forName: legacyDomain) ?? [:] where current[key] == nil {
            current[key] = value
        }
        current[marker] = true
        defaults.setPersistentDomain(current, forName: targetDomain)
    }

    public static func register(_ defaults: UserDefaults = .standard) {
        defaults.register(defaults: [
            PrefKey.nextcloudFolder: "super-productivity",
            PrefKey.autoSync: true,
            PrefKey.compress: false,
            PrefKey.syncEnabled: false,
            PrefKey.groupBy: "morning-night",
            PrefKey.taskSort: "manual",
            PrefKey.sortDirection: "ascending",
            PrefKey.upcomingRange: "7",
            PrefKey.colorfulLabels: true,
            PrefKey.runInBackground: false,
            PrefKey.showInMenuBar: false,
            PrefKey.dockBadgeMode: DockBadgeMode.dueToday.rawValue,
            PrefKey.modifierKey: "command",
            PrefKey.autoArchive: false,
            PrefKey.morningSummaryEnabled: false,
            PrefKey.morningSummaryHour: 8,
            PrefKey.morningSummaryMinute: 0,
            PrefKey.p2pEnabled: false,
            PrefKey.sidebarVisible: true,
            PrefKey.appFontName: "",
            PrefKey.contentFontSize: 13.0,
            PrefKey.interfaceFontSize: 13.0,
        ])
    }

    public let defaults: UserDefaults
    public init(_ defaults: UserDefaults = .standard) {
        self.defaults = defaults
    }

    public var modifier: ModifierKey {
        ModifierKey(rawValue: defaults.string(forKey: PrefKey.modifierKey) ?? "command") ?? .command
    }
    public var syncMethod: SyncMethod {
        if let selected = defaults.string(forKey: PrefKey.syncMethod) {
            return SyncMethod(rawValue: selected) ?? .off
        }
        // Existing linked-device users keep their selected transport when both old switches were on.
        if defaults.bool(forKey: PrefKey.p2pEnabled) { return .libresync }
        return defaults.bool(forKey: PrefKey.syncEnabled) ? .nextcloud : .off
    }
    public static func migrateSyncMethod(_ defaults: UserDefaults = .standard) {
        guard defaults.string(forKey: PrefKey.syncMethod) == nil else { return }
        defaults.set(Preferences(defaults).syncMethod.rawValue, forKey: PrefKey.syncMethod)
    }
    public var syncEnabled: Bool { syncMethod == .nextcloud }
    public var autoSync: Bool { defaults.bool(forKey: PrefKey.autoSync) }
    public var p2pEnabled: Bool { syncMethod == .libresync }
    public var colorful: Bool { defaults.bool(forKey: PrefKey.colorfulLabels) }
    public var runInBackground: Bool { defaults.bool(forKey: PrefKey.runInBackground) }
    public var showInMenuBar: Bool { defaults.bool(forKey: PrefKey.showInMenuBar) }
    public var dockBadgeMode: DockBadgeMode {
        DockBadgeMode(rawValue: defaults.string(forKey: PrefKey.dockBadgeMode) ?? "") ?? .dueToday
    }

    public var morningSummaryTime: ClockTime {
        let hour = defaults.object(forKey: PrefKey.morningSummaryHour) as? Int ?? 8
        let minute = defaults.object(forKey: PrefKey.morningSummaryMinute) as? Int ?? 0
        return ClockTime(hour: UInt32((0...23).contains(hour) ? hour : 8),
                         minute: UInt32((0...59).contains(minute) ? minute : 0))
    }

    /// The preferences the core computes with.
    public var core: MomentumCore.Preferences {
        let group: GroupBy
        switch defaults.string(forKey: PrefKey.groupBy) {
        case "none": group = .none
        case "project": group = .project
        case "tag": group = .tag
        case "estimate": group = .estimate
        default: group = .morningNight
        }
        let sort: SortKey
        switch defaults.string(forKey: PrefKey.taskSort) ?? "manual" {
        case "title": sort = .title
        case "due": sort = .due
        case "estimate": sort = .estimate
        case "created": sort = .created
        default: sort = .manual
        }
        let direction: SortDirection = defaults.string(forKey: PrefKey.sortDirection) == "descending" ? .descending : .ascending
        let days = UInt32(defaults.string(forKey: PrefKey.upcomingRange) ?? "7") ?? 7
        return MomentumCore.Preferences(groupBy: group, sort: sort, direction: direction, upcomingDays: days,
                                        autoArchive: defaults.bool(forKey: PrefKey.autoArchive),
                                        morningSummaryEnabled: defaults.bool(forKey: PrefKey.morningSummaryEnabled),
                                        morningSummaryTime: morningSummaryTime)
    }

    /// Nextcloud settings with secrets from the Keychain; nil while the switch is off or
    /// the connection is incomplete.
    public func nextcloud(keychain: Keychain = Keychain()) -> NextcloudSettings? {
        guard syncEnabled else { return nil }
        let s = NextcloudSettings(
            serverUrl: defaults.string(forKey: PrefKey.nextcloudServer) ?? "",
            userName: defaults.string(forKey: PrefKey.nextcloudUser) ?? "",
            password: keychain.get(Keychain.nextcloud) ?? "",
            folder: defaults.string(forKey: PrefKey.nextcloudFolder) ?? "super-productivity",
            compress: defaults.bool(forKey: PrefKey.compress),
            encryptionPassword: keychain.get(Keychain.encryption))
        let complete = !s.serverUrl.trimmingCharacters(in: .whitespaces).isEmpty
            && !s.userName.trimmingCharacters(in: .whitespaces).isEmpty
            && !s.password.isEmpty
            && !s.folder.trimmingCharacters(in: .whitespaces).isEmpty
        return complete ? s : nil
    }
    public var syncConfigured: Bool { nextcloud() != nil }

    /// Import only shared, non-secret settings. Local switches and credentials stay local.
    public func readCliConfig(from dir: URL) {
        guard let data = try? Data(contentsOf: dir.appendingPathComponent("cli-config.json")),
              let cfg = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return }
        for (field, key) in [("server", PrefKey.nextcloudServer), ("user", PrefKey.nextcloudUser),
                             ("folder", PrefKey.nextcloudFolder)] {
            if let value = cfg[field] as? String, defaults.string(forKey: key) != value {
                defaults.set(value, forKey: key)
            }
        }
        if let value = cfg["compress"] as? Bool, defaults.bool(forKey: PrefKey.compress) != value {
            defaults.set(value, forKey: PrefKey.compress)
        }
    }

    /// Writes the non-secret Nextcloud settings where the `mo` command line reads them.
    public func writeCliConfig(to dir: URL) {
        let cfg: [String: Any] = [
            "method": syncMethod.rawValue,
            "server": defaults.string(forKey: PrefKey.nextcloudServer) ?? "",
            "user": defaults.string(forKey: PrefKey.nextcloudUser) ?? "",
            "folder": defaults.string(forKey: PrefKey.nextcloudFolder) ?? "super-productivity",
            "compress": defaults.bool(forKey: PrefKey.compress),
        ]
        if let data = try? JSONSerialization.data(withJSONObject: cfg, options: [.prettyPrinted, .sortedKeys]) {
            let file = dir.appendingPathComponent("cli-config.json")
            if (try? Data(contentsOf: file)) != data { try? data.write(to: file, options: .atomic) }
        }
    }
}

/// Where the store lives: the same directory `mo` uses, so the app and the command line
/// never disagree. `MOMENTUM_DATA_DIR` overrides it (tests, a second profile).
public enum DataDirectory {
    public static var url: URL {
        if let override = ProcessInfo.processInfo.environment["MOMENTUM_DATA_DIR"], !override.isEmpty {
            return URL(fileURLWithPath: override)
        }
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support")
        return base.appendingPathComponent("momentum")
    }
}
