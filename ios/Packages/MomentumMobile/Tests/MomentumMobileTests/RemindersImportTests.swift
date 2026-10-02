// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import Testing
@testable import MomentumMobile

@MainActor private final class FixtureSource: RemindersSource {
    var permission = RemindersPermission.notDetermined
    var didChange: (@MainActor () -> Void)?
    var grant = true
    var prompts = 0
    var fetches = 0
    var available = [ReminderList(id: "fixture-list", title: "Synthetic List")]
    var items = [ReminderImportItem(sourceID: "fixture-source", title: "Synthetic Reminder")]
    var error: RemindersImportIssue?
    var suspended: CheckedContinuation<[ReminderImportItem], any Error>?
    var suspendFetch = false
    func requestPermission() async throws -> Bool {
        prompts += 1; permission = grant ? .allowed : .denied; return grant
    }
    func lists() -> [ReminderList] { available }
    func incompleteReminders(in listID: String) async throws -> [ReminderImportItem] {
        fetches += 1
        if let error { throw error }
        if suspendFetch { return try await withCheckedThrowingContinuation { suspended = $0 } }
        return items
    }
}

private actor ImportSink {
    var ids: [String] = []
    var fail = false
    func importItem(_ item: ReminderImportItem) throws -> Bool {
        if fail { throw RemindersImportIssue.save }
        ids.append(item.sourceID); return true
    }
    func count() -> Int { ids.count }
    func setFailure(_ value: Bool) { fail = value }
}

@Suite @MainActor struct RemindersImportTests {
    private func preferences() -> UserDefaults {
        UserDefaults(suiteName: "reminders-fixture-\(UUID())")!
    }

    @Test func disabledByDefaultAndNoPermissionPromptOnLaunch() async {
        let source = FixtureSource()
        let state = RemindersImport(source: source, defaults: preferences())
        state.setForeground(true)
        await state.reactivate()
        #expect(!state.automatic)
        #expect(source.prompts == 0)
        #expect(source.fetches == 0)
        #expect(state.issue == nil)
        source.grant = false
        await state.connect()
        #expect(source.prompts == 1)
        #expect(!state.connected)
        #expect(state.issue == .permission)
    }

    @Test func repeatedImportPersistsHistoryAndRevocationStopsReads() async throws {
        let source = FixtureSource(), defaults = preferences(), sink = ImportSink()
        let state = RemindersImport(source: source, defaults: defaults)
        state.connectImporter { try await sink.importItem($0) }
        await state.connect()
        state.selectList("fixture-list")
        await state.importNow()
        #expect(state.lastImportCount == 1)
        source.items = [ReminderImportItem(sourceID: "server-source", title: "Synthetic Reminder", sourceAliases: ["fixture-source"])]
        await state.importNow()
        #expect(state.lastImportCount == 0)
        #expect(await sink.count() == 1)
        let reopened = RemindersImport(source: source, defaults: defaults)
        reopened.connectImporter { try await sink.importItem($0) }
        await reopened.importNow()
        #expect(await sink.count() == 1)
        let before = source.fetches
        source.permission = .denied
        await reopened.reactivate()
        #expect(!reopened.connected)
        #expect(reopened.issue == .permission)
        #expect(source.fetches == before)
        #expect(source.prompts == 1)
    }

    @Test func missingEmptyListsAndFailedSaveCanRecover() async {
        let source = FixtureSource(), sink = ImportSink()
        let state = RemindersImport(source: source, defaults: preferences())
        state.connectImporter { try await sink.importItem($0) }
        await state.connect()
        state.selectList("fixture-list")
        source.available = []
        await state.importNow()
        #expect(state.issue == .missingList)
        #expect(source.fetches == 0)
        source.available = [ReminderList(id: "fixture-list", title: "Synthetic List")]
        source.items = []
        await state.importNow()
        #expect(state.lastImportCount == 0)
        source.items = [ReminderImportItem(sourceID: "next-source", title: "New Reminder")]
        await sink.setFailure(true)
        await state.importNow()
        #expect(state.issue == .save)
        await sink.setFailure(false)
        await state.importNow()
        #expect(state.lastImportCount == 1)
    }

    @Test func foregroundReactivationImportsNewItemsButDisabledDoesNot() async {
        let source = FixtureSource(), sink = ImportSink()
        let state = RemindersImport(source: source, defaults: preferences())
        state.connectImporter { try await sink.importItem($0) }
        await state.connect()
        state.selectList("fixture-list")
        state.setAutomatic(true) // inactive: does not start importing
        #expect(source.fetches == 0)
        state.setForeground(true)
        await state.reactivate()
        #expect(await sink.count() == 1)
        state.setForeground(false)
        source.items.append(ReminderImportItem(sourceID: "new-source", title: "New Reminder"))
        source.didChange?()
        #expect(source.fetches == 1)
        state.setForeground(true)
        await state.reactivate()
        #expect(await sink.count() == 2)
        state.setAutomatic(false)
        source.items.append(ReminderImportItem(sourceID: "third-source", title: "Third Reminder"))
        await state.reactivate()
        #expect(await sink.count() == 2)
    }

    @Test func notificationDuringFetchIsCoalescedAndOffCancelsRemainingImports() async throws {
        let source = FixtureSource(), sink = ImportSink()
        let state = RemindersImport(source: source, defaults: preferences())
        state.connectImporter { try await sink.importItem($0) }
        await state.connect()
        state.selectList("fixture-list")
        state.setAutomatic(true)
        state.setForeground(true)
        source.suspendFetch = true
        let pending = Task { await state.reactivate() }
        while source.suspended == nil { await Task.yield() }
        source.didChange?()
        await Task.yield()
        state.setAutomatic(false)
        source.suspendFetch = false
        source.suspended?.resume(returning: source.items)
        await pending.value
        #expect(await sink.count() == 0)
    }

    @Test func previewHasNoSystemSource() async {
        let state = RemindersImport(source: nil, defaults: preferences())
        await state.connect()
        #expect(state.issue == .unavailable)
    }
}

@Suite struct ReminderProjectionTests {
    @Test func dueDayUsesSourceCalendarAndTimezone() {
        let zone = TimeZone(secondsFromGMT: 14 * 3600)!
        #expect(ReminderDateProjection.dueDay(DateComponents(timeZone: zone, year: 2028, month: 2, day: 29, hour: 0)) == "2028-02-29")
        #expect(ReminderDateProjection.dueDay(DateComponents(calendar: Calendar(identifier: .buddhist), timeZone: zone, year: 2569, month: 10, day: 3)) == "2026-10-03")
        #expect(ReminderDateProjection.dueDay(nil) == nil)
        #expect(ReminderDateProjection.dueDay(DateComponents(year: 2026, month: 10)) == nil)
    }

    @Test func realWorkerPreservesDraftAndDeduplicatesAcrossReopen() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent("reminders-engine-\(UUID())")
        defer { try? FileManager.default.removeItem(at: directory) }
        let worker = await EngineWorker.open(directory: directory)
        let item = ReminderImportItem(sourceID: "fixture-source", title: "Fixture task", notes: "Fixture notes", dueDay: "2026-10-03")
        #expect(try await worker.importReminder(item))
        let id = try #require(await worker.lastAddedTaskID())
        let detail = try #require(await worker.taskDetail(id))
        #expect(detail.notes == "Fixture notes")
        #expect(detail.dueDay == "2026-10-03")
        #expect(detail.time == nil)
        #expect(detail.repeat == nil)
        let reopened = await EngineWorker.open(directory: directory)
        #expect(try await !reopened.importReminder(item))
    }
}
