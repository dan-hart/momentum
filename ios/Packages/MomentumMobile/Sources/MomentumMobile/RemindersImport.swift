// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import Observation

public struct ReminderList: Identifiable, Sendable, Equatable {
    public let id: String
    public let title: String
    public init(id: String, title: String) { self.id = id; self.title = title }
}

public struct ReminderImportItem: Sendable, Equatable {
    public let sourceID: String
    public let sourceAliases: [String]
    public let title: String
    public let notes: String
    public let dueDay: String?
    public init(sourceID: String, title: String, notes: String = "", dueDay: String? = nil, sourceAliases: [String] = []) {
        self.sourceID = sourceID; self.sourceAliases = sourceAliases; self.title = title; self.notes = notes; self.dueDay = dueDay
    }
}

public enum RemindersPermission: Sendable { case notDetermined, allowed, denied }
public enum RemindersImportIssue: Error, Equatable { case permission, missingList, fetch, save, unavailable }

/// Only value snapshots leave the native adapter. The importer never writes to EventKit.
@MainActor public protocol RemindersSource: AnyObject {
    var permission: RemindersPermission { get }
    var didChange: (@MainActor () -> Void)? { get set }
    func requestPermission() async throws -> Bool
    func lists() -> [ReminderList]
    func incompleteReminders(in listID: String) async throws -> [ReminderImportItem]
}

@MainActor @Observable public final class RemindersImport {
    public private(set) var lists: [ReminderList] = []
    public private(set) var selectedListID: String
    public private(set) var automatic: Bool
    public private(set) var connected = false
    public private(set) var busy = false
    public private(set) var issue: RemindersImportIssue?
    public private(set) var lastImportCount: Int?
    public var didImport: (@MainActor () -> Void)?
    private let source: (any RemindersSource)?
    private let defaults: UserDefaults
    private var importer: (@Sendable (ReminderImportItem) async throws -> Bool)?
    private var foreground = false
    private var rerun = false
    private var generation = 0
    private static let listKey = "momentum.reminders.list"
    private static let autoKey = "momentum.reminders.automatic"
    private static let historyKey = "momentum.reminders.imported"

    public init(source: (any RemindersSource)?, defaults: UserDefaults) {
        self.source = source; self.defaults = defaults
        selectedListID = defaults.string(forKey: Self.listKey) ?? ""
        automatic = defaults.bool(forKey: Self.autoKey)
        source?.didChange = { [weak self] in
            guard let self, self.foreground, self.automatic else { return }
            Task { await self.refresh(importNew: true) }
        }
    }

    public func connectImporter(_ importer: @escaping @Sendable (ReminderImportItem) async throws -> Bool) {
        self.importer = importer
    }

    /// The only path that may present the OS prompt. Called by the Connect button.
    public func connect() async {
        guard !busy, let source else { issue = .unavailable; return }
        busy = true
        do {
            let allowed = try await source.requestPermission()
            connected = allowed
            issue = allowed ? nil : .permission
        } catch { issue = .permission }
        busy = false
        await refresh(importNew: false)
    }

    public func selectList(_ id: String) {
        generation += 1
        selectedListID = id
        defaults.set(id, forKey: Self.listKey)
        lastImportCount = nil
        issue = nil
        if automatic && foreground { Task { await refresh(importNew: true) } }
    }

    public func setAutomatic(_ enabled: Bool) {
        generation += 1
        automatic = enabled
        defaults.set(enabled, forKey: Self.autoKey)
        if enabled && foreground { Task { await refresh(importNew: true) } }
    }

    public func setForeground(_ active: Bool) {
        foreground = active
        if !active { generation += 1 }
    }

    public func reactivate() async {
        if automatic { await refresh(importNew: true) }
        else if source?.permission != .allowed && !selectedListID.isEmpty {
            connected = false; lists = []; issue = .permission
        }
    }
    public func loadLists() async { await refresh(importNew: false) }
    public func importNow() async { await refresh(importNew: true) }

    private func refresh(importNew: Bool) async {
        if busy { rerun = rerun || importNew; return }
        guard let source else { issue = .unavailable; return }
        guard source.permission == .allowed else {
            connected = false
            lists = []
            // An untouched, disabled integration stays quiet on launch.
            if !selectedListID.isEmpty || importNew { issue = .permission }
            return
        }
        connected = true
        busy = true
        defer {
            busy = false
            if rerun {
                rerun = false
                Task { await self.refresh(importNew: self.automatic && self.foreground) }
            }
        }
        lists = source.lists()
        guard !selectedListID.isEmpty else { issue = nil; return }
        guard lists.contains(where: { $0.id == selectedListID }) else { issue = .missingList; return }
        issue = nil
        guard importNew, let importer else { return }
        let list = selectedListID
        let version = generation
        var imported = Set(defaults.stringArray(forKey: Self.historyKey) ?? [])
        var count = 0
        lastImportCount = nil
        do {
            let items = try await source.incompleteReminders(in: list)
            for item in items {
                guard generation == version else { return }
                guard source.permission == .allowed else {
                    connected = false; lists = []; issue = .permission; return
                }
                guard !Task.isCancelled else { return }
                let identities = [item.sourceID] + item.sourceAliases
                if identities.contains(where: imported.contains) {
                    imported.formUnion(identities)
                    defaults.set(Array(imported), forKey: Self.historyKey)
                    continue
                }
                guard !item.title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { continue }
                if try await importer(item) { count += 1; didImport?() }
                // Retain history after local deletion/Undo and disabling automatic import.
                // The core's atomic provenance check closes the save/history crash window.
                imported.formUnion(identities)
                defaults.set(Array(imported), forKey: Self.historyKey)
            }
            guard generation == version else { return }
            lastImportCount = count
        } catch let error as RemindersImportIssue {
            lastImportCount = count
            issue = error
            if error == .permission { connected = false; lists = [] }
        } catch { lastImportCount = count; issue = .fetch }
    }
}

/// Keep the reminder's calendar day, including floating/all-day dates, rather than
/// converting its timestamp into the device's potentially different time zone.
public enum ReminderDateProjection {
    public static func dueDay(_ components: DateComponents?) -> String? {
        guard let components, components.year != nil, components.month != nil, components.day != nil else { return nil }
        let zone = components.timeZone ?? TimeZone.current
        var source = components.calendar ?? Calendar(identifier: .gregorian)
        source.timeZone = zone
        guard let date = source.date(from: components) else { return nil }
        var gregorian = Calendar(identifier: .gregorian)
        gregorian.timeZone = zone
        let day = gregorian.dateComponents([.year, .month, .day], from: date)
        guard let year = day.year, let month = day.month, let date = day.day else { return nil }
        return String(format: "%04d-%02d-%02d", year, month, date)
    }
}
