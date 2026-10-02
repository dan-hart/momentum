// SPDX-License-Identifier: GPL-3.0-or-later
// Opt-in real-framework regression. Use only a fresh, empty simulator.
import EventKit
import MomentumMobile
import XCTest

@MainActor final class EventKitRemindersIntegrationTests: XCTestCase {
    func testActualEventKitFetchWithSyntheticReminder() async throws {
        #if !targetEnvironment(simulator)
        throw XCTSkip("Synthetic EventKit integration is simulator-only")
        #else
        guard ProcessInfo.processInfo.environment["MOMENTUM_REMINDERS_NATIVE_FIXTURE"] == "1" else {
            throw XCTSkip("Opt in only on a fresh, empty test simulator")
        }
        guard EKEventStore.authorizationStatus(for: .reminder) == .fullAccess else {
            throw XCTSkip("Requires the dedicated test simulator's Reminders grant")
        }
        let store = EKEventStore()
        let calendar = EKCalendar(for: .reminder, eventStore: store)
        calendar.title = "Momentum Synthetic Callback Test " + UUID().uuidString
        let source = try XCTUnwrap(store.sources.first { $0.sourceType == .local })
        calendar.source = source
        try store.saveCalendar(calendar, commit: true)
        defer { try? store.removeCalendar(calendar, commit: true) }
        let reminder = EKReminder(eventStore: store)
        reminder.calendar = calendar
        reminder.title = "Synthetic EventKit reminder"
        reminder.notes = "No personal data"
        reminder.dueDateComponents = DateComponents(year: 2026, month: 10, day: 3)
        try store.save(reminder, commit: true)
        let adapter = EventKitRemindersSource()
        let items = try await adapter.incompleteReminders(in: calendar.calendarIdentifier)
        XCTAssertEqual(items.count, 1)
        XCTAssertEqual(items.first?.title, "Synthetic EventKit reminder")
        XCTAssertEqual(items.first?.notes, "No personal data")
        XCTAssertEqual(items.first?.dueDay, "2026-10-03")
        let repeated = try await adapter.incompleteReminders(in: calendar.calendarIdentifier)
        XCTAssertEqual(repeated, items)
        #endif
    }
}
