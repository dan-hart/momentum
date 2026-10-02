// SPDX-License-Identifier: GPL-3.0-or-later
import CryptoKit
import EventKit
import Foundation

@MainActor public final class EventKitRemindersSource: RemindersSource {
    private let store = EKEventStore()
    private var observer: NSObjectProtocol?
    public var didChange: (@MainActor () -> Void)?

    public init() {
        observer = NotificationCenter.default.addObserver(forName: .EKEventStoreChanged, object: store, queue: .main) { [weak self] _ in
            Task { @MainActor [weak self] in self?.didChange?() }
        }
    }

    isolated deinit { if let observer { NotificationCenter.default.removeObserver(observer) } }

    public var permission: RemindersPermission {
        switch EKEventStore.authorizationStatus(for: .reminder) {
        case .fullAccess: .allowed
        case .notDetermined: .notDetermined
        default: .denied
        }
    }

    public func requestPermission() async throws -> Bool {
        guard permission == .notDetermined else { return permission == .allowed }
        return try await store.requestFullAccessToReminders()
    }

    public func lists() -> [ReminderList] {
        guard permission == .allowed else { return [] }
        return store.calendars(for: .reminder).map { ReminderList(id: $0.calendarIdentifier, title: $0.title) }
            .sorted { $0.title.localizedStandardCompare($1.title) == .orderedAscending }
    }

    public func incompleteReminders(in listID: String) async throws -> [ReminderImportItem] {
        guard permission == .allowed else { throw RemindersImportIssue.permission }
        guard let list = store.calendar(withIdentifier: listID), list.allowedEntityTypes.contains(.reminder) else {
            throw RemindersImportIssue.missingList
        }
        // EventKit changes invalidate fetched objects. Resolve the list and predicate
        // afresh each time; never infer changed reminder IDs from notification.userInfo.
        let predicate = store.predicateForIncompleteReminders(withDueDateStarting: nil, ending: nil, calendars: [list])
        let items = await EventKitReminderFetch.snapshots { completion in
            store.fetchReminders(matching: predicate, completion: completion)
        }
        guard permission == .allowed else { throw RemindersImportIssue.permission }
        guard store.calendar(withIdentifier: listID) != nil else { throw RemindersImportIssue.missingList }
        guard let items else { throw RemindersImportIssue.fetch }
        return items
    }
}

/// EventKit's legacy Objective-C block has no actor guarantee. A Sendable callback
/// must project framework objects on the delivering queue, before resuming the
/// MainActor caller with value snapshots. Do not inherit MainActor onto this block.
@MainActor enum EventKitReminderFetch {
    static func snapshots(
        _ start: (@escaping @Sendable ([EKReminder]?) -> Void) -> Void
    ) async -> [ReminderImportItem]? {
        await withCheckedContinuation { continuation in
            start { @Sendable reminders in
                continuation.resume(returning: reminders?.map(ReminderSnapshotProjection.item))
            }
        }
    }
}

private enum ReminderSnapshotProjection {
    nonisolated static func item(_ reminder: EKReminder) -> ReminderImportItem {
        // Prefer the server's identifier, which survives a full database sync.
        // Preserve the local identifier as an alias for newly synchronized items.
        let external = reminder.calendarItemExternalIdentifier.flatMap { $0.isEmpty ? nil : $0 }
        let identity = external.map { "external:" + $0 } ?? "local:" + reminder.calendarItemIdentifier
        func sourceKey(_ value: String) -> String {
            let digest = SHA256.hash(data: Data(value.utf8))
            return "apple-reminders:v1:" + digest.map { String(format: "%02x", $0) }.joined()
        }
        let sourceID = sourceKey(identity)
        let aliases = external == nil ? [] : [sourceKey("local:" + reminder.calendarItemIdentifier)]
        let day = ReminderDateProjection.dueDay(reminder.dueDateComponents)
        let notes = [reminder.notes, reminder.url?.absoluteString].compactMap { $0 }.filter { !$0.isEmpty }.joined(separator: "\n")
        return ReminderImportItem(sourceID: sourceID, title: reminder.title ?? "", notes: notes, dueDay: day, sourceAliases: aliases)
    }
}
