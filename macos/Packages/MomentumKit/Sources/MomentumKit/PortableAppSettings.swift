// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import CoreFoundation
import MomentumCore

/// Native key mapping only. Rust validates, persists and reconciles the portable document.
@MainActor public enum PortableAppSettings {
    public struct Summary: Identifiable {
        public let id: String
        public let label: String
        public let value: String
    }
    private static let strings = ["grouping": PrefKey.groupBy, "sort": PrefKey.taskSort,
        "direction": PrefKey.sortDirection, "rowDensity": PrefKey.rowDensity,
        "completionSoundChoice": PrefKey.completionSoundChoice]
    private static let booleans = ["autoArchive": PrefKey.autoArchive,
        "colorfulLabels": PrefKey.colorfulLabels, "completionSoundEnabled": PrefKey.completionSound]

    public static func encode(_ defaults: UserDefaults) throws -> String {
        let preferences = Preferences(defaults)
        let core = preferences.core
        let grouping: String = switch core.groupBy {
        case .morningNight: "morning-night"
        case .none: "none"
        case .project: "project"
        case .tag: "tag"
        case .estimate: "estimate"
        }
        let sort: String = switch core.sort {
        case .manual: "manual"
        case .title: "title"
        case .due: "due"
        case .estimate: "estimate"
        case .created: "created"
        }
        let values: [String: Any] = ["grouping": grouping, "sort": sort,
            "direction": core.direction == .descending ? "descending" : "ascending",
            "upcomingDays": core.upcomingDays >= 30 ? 30 : 7, "autoArchive": core.autoArchive,
            "rowDensity": defaults.string(forKey: PrefKey.rowDensity) == "compact" ? "compact" : "regular",
            "colorfulLabels": preferences.colorful, "completionSoundEnabled": preferences.completionSound,
            "completionSoundChoice": defaults.string(forKey: PrefKey.completionSoundChoice) ?? CompletionSoundChoice.defaultChoice.id]
        let data = try JSONSerialization.data(withJSONObject: ["version": 1, "values": values], options: [.sortedKeys])
        return String(decoding: data, as: UTF8.self)
    }

    /// Apply only keys returned by the core. Missing and unsupported keys leave local choices intact.
    public static func apply(_ json: String, to defaults: UserDefaults) throws {
        guard let document = try JSONSerialization.jsonObject(with: Data(json.utf8)) as? [String: Any],
              let version = document["version"] as? NSNumber,
              CFGetTypeID(version) != CFBooleanGetTypeID(), version.intValue == 1,
              let values = document["values"] as? [String: Any] else { throw invalid() }
        var native: [String: Any] = [:]
        for (key, value) in values {
            if let target = strings[key] {
                guard let text = value as? String else { throw invalid() }
                native[target] = text
            } else if let target = booleans[key] {
                guard let number = value as? NSNumber, CFGetTypeID(number) == CFBooleanGetTypeID() else { throw invalid() }
                native[target] = number.boolValue
            } else if key == "upcomingDays" {
                guard let number = value as? NSNumber, CFGetTypeID(number) != CFBooleanGetTypeID(),
                      number.doubleValue == Double(number.intValue) else { throw invalid() }
                native[PrefKey.upcomingRange] = number.stringValue
            }
        }
        let effective = try JSONSerialization.jsonObject(with: Data(encode(defaults).utf8)) as? [String: Any]
        let effectiveValues = effective?["values"] as? [String: Any] ?? [:]
        // Validate the entire mapping before publishing. Equivalent fallbacks do
        // not overwrite an unsupported native choice written by a newer app.
        for (key, value) in native where !equal(defaults.object(forKey: key), value) {
            let portable = strings.first(where: { $0.value == key })?.key
                ?? booleans.first(where: { $0.value == key })?.key
                ?? (key == PrefKey.upcomingRange ? "upcomingDays" : nil)
            if let portable, let supplied = values[portable], equal(effectiveValues[portable], supplied) { continue }
            defaults.set(value, forKey: key)
        }
    }

    public static func initialize(engine: Engine, defaults: UserDefaults) throws {
        try apply(engine.portableSettingsJson(), to: defaults)
        try stage(engine: engine, defaults: defaults)
    }

    public static func summary(_ json: String) -> [Summary] {
        guard let document = try? JSONSerialization.jsonObject(with: Data(json.utf8)) as? [String: Any],
              let values = document["values"] as? [String: Any] else { return [] }
        let labels: [(String, String)] = [
            ("grouping", String(localized: "Group By", bundle: .module)),
            ("sort", String(localized: "Sort By", bundle: .module)),
            ("direction", String(localized: "Sort direction", bundle: .module)),
            ("upcomingDays", String(localized: "Upcoming range", bundle: .module)),
            ("autoArchive", String(localized: "Archive completed tasks immediately", bundle: .module)),
            ("rowDensity", String(localized: "Row density", bundle: .module)),
            ("colorfulLabels", String(localized: "Colorful labels", bundle: .module)),
            ("completionSoundEnabled", String(localized: "Completion sounds", bundle: .module)),
            ("completionSoundChoice", String(localized: "Completion sound", bundle: .module))]
        return labels.compactMap { key, label in
            guard let value = values[key] else { return nil }
            let text: String
            if let boolean = value as? NSNumber, CFGetTypeID(boolean) == CFBooleanGetTypeID() {
                text = boolean.boolValue ? String(localized: "On", bundle: .module) : String(localized: "Off", bundle: .module)
            } else if key == "upcomingDays", let number = value as? NSNumber {
                let days = number.intValue
                text = String(localized: "\(days) days", bundle: .module)
            } else if let slug = value as? String {
                text = switch slug {
                case "morning-night": String(localized: "Morning & Night", bundle: .module)
                case "none": String(localized: "None", bundle: .module)
                case "project": String(localized: "Project", bundle: .module)
                case "tag": String(localized: "Tag", bundle: .module)
                case "estimate": String(localized: "Estimate", bundle: .module)
                case "manual": String(localized: "Manual", bundle: .module)
                case "title": String(localized: "Title", bundle: .module)
                case "due": String(localized: "Due date", bundle: .module)
                case "created": String(localized: "Created", bundle: .module)
                case "ascending": String(localized: "Ascending", bundle: .module)
                case "descending": String(localized: "Descending", bundle: .module)
                case "regular": String(localized: "Regular", bundle: .module)
                case "compact": String(localized: "Compact", bundle: .module)
                default: CompletionSoundChoice(rawValue: slug)?.label ?? String(localized: "Unknown", bundle: .module)
                }
            } else { return nil }
            return Summary(id: key, label: label, value: text)
        }
    }

    @discardableResult public static func stage(engine: Engine, defaults: UserDefaults, baseline: String? = nil) throws -> String {
        let previous = try engine.portableSettingsJson()
        let current = try encode(defaults)
        do { _ = try engine.stagePortableSettings(json: changes(from: baseline, to: current)); return current }
        catch { try apply(previous, to: defaults); throw error }
    }

    /// A native edit carries only the user's changed keys. An unseen remote edit
    /// to a different key must not be overwritten by an old native snapshot.
    public static func changes(from baseline: String?, to current: String) throws -> String {
        guard let document = try JSONSerialization.jsonObject(with: Data(current.utf8)) as? [String: Any],
              let values = document["values"] as? [String: Any] else { throw invalid() }
        let previous = try baseline.flatMap { try JSONSerialization.jsonObject(with: Data($0.utf8)) as? [String: Any] }
        let before = previous?["values"] as? [String: Any] ?? [:]
        let changed = values.filter { !equal(before[$0.key], $0.value) }
        let data = try JSONSerialization.data(withJSONObject: ["version": 1, "values": changed], options: [.sortedKeys])
        return String(decoding: data, as: UTF8.self)
    }

    private static func equal(_ lhs: Any?, _ rhs: Any) -> Bool {
        guard let lhs = lhs as? NSObject, let rhs = rhs as? NSObject else { return false }
        return lhs == rhs
    }
    private static func invalid() -> CocoaError { CocoaError(.coderReadCorrupt) }
}
