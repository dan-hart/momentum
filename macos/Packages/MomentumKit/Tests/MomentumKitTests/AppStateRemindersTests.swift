// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import Testing
@testable import MomentumKit

@MainActor private final class MacReminderSource: RemindersSource {
    var permission = RemindersPermission.allowed
    var didChange: (@MainActor () -> Void)?
    var prompts = 0
    var fetches = 0
    var items = [ReminderImportItem(sourceID: "local-source", title: "Synthetic reminder", notes: "Synthetic note")]
    func requestPermission() async throws -> Bool { prompts += 1; return permission == .allowed }
    func lists() -> [ReminderList] { [ReminderList(id: "synthetic-list", title: "Synthetic List")] }
    func incompleteReminders(in listID: String) async throws -> [ReminderImportItem] {
        fetches += 1
        return items
    }
}

@MainActor private final class MacReminderHarness {
    let dir = TempDir()
    let suite = "momentum-mac-reminders-\(UUID())"
    let defaults: UserDefaults
    let source = MacReminderSource()
    let state: AppState
    let indexer = RecordingIndexer()
    init(demo: Bool = false) {
        defaults = UserDefaults(suiteName: suite)!
        let engine = Engine.open(dir: dir.url.appendingPathComponent("store").path)
        state = AppState(engine: engine, defaults: defaults,
                         keychain: Keychain(service: "momentum-mac-reminders-\(UUID())"),
                         services: false, isDemo: demo, remindersSource: source)
        state.indexer = indexer
    }
    deinit { UserDefaults().removePersistentDomain(forName: suite) }
    func select() async {
        await state.reminders.loadLists()
        state.reminders.selectList("synthetic-list")
    }
}

@Suite @MainActor struct AppStateRemindersTests {
    @Test func importingRefreshesSearchSidebarAndSpotlightAndIsDurable() async throws {
        let h = MacReminderHarness()
        h.state.searchText = "Synthetic reminder"
        h.state.go(to: .search)
        #expect(h.state.allRowIds.isEmpty)
        await h.select()
        await h.state.reminders.importNow()
        #expect(h.state.reminders.lastImportCount == 1)
        #expect(h.state.allRowIds.count == 1)
        #expect(h.state.sidebar == h.state.engine.sidebar())
        #expect(h.indexer.indexed.last?.count == 1)
        let reopened = try Engine.openChecked(dir: h.state.dataDir.path)
        #expect(reopened.allTasks().map(\.title) == ["Synthetic reminder"])
        let indexed = h.indexer.indexed.count
        await h.state.reminders.importNow()
        #expect(h.state.reminders.lastImportCount == 0)
        #expect(h.state.engine.allTasks().count == 1)
        #expect(h.indexer.indexed.count == indexed)
    }

    @Test func aliasReconciliationRefreshesWithoutCreatingOrCountingTask() async {
        let h = MacReminderHarness()
        await h.select()
        await h.state.reminders.importNow()
        let id = h.state.engine.allTasks().first?.id
        let indexed = h.indexer.indexed.count
        h.source.items = [ReminderImportItem(sourceID: "server-source", title: "Synthetic reminder", sourceAliases: ["local-source"])]
        await h.state.reminders.importNow()
        #expect(h.state.reminders.lastImportCount == 0)
        #expect(h.state.engine.allTasks().map(\.id) == [id].compactMap { $0 })
        #expect(h.indexer.indexed.count == indexed + 1)
        let result = h.state.engine.importTaskOnce(sourceId: "server-source", draft: TaskDraft(
            title: "Duplicate must not be created", projectId: "", dueDay: nil, time: nil,
            reminderMinutesBefore: nil, estimateMs: 0, notes: "", tagIds: [], newTags: []), sourceAliases: [])
        #expect(result.id == nil)
        #expect(!result.outcome.changed)
    }

    @Test func saveFailureKeepsExistingTasksAndRetryCreatesExactlyOnce() async throws {
        let h = MacReminderHarness()
        h.state.addTask("Existing task")
        await h.select()
        let blocked = h.state.dataDir.appendingPathComponent("pending.json.tmp")
        try FileManager.default.createDirectory(at: blocked, withIntermediateDirectories: false)
        await h.state.reminders.importNow()
        #expect(h.state.reminders.issue == .save)
        #expect(h.state.reminders.feedback == .issue(.save))
        #expect(h.state.engine.allTasks().map(\.title) == ["Existing task"])
        #expect(h.defaults.stringArray(forKey: "momentum.reminders.imported") == nil)
        try FileManager.default.removeItem(at: blocked)
        await h.state.reminders.retry()
        #expect(h.state.reminders.issue == nil)
        #expect(h.state.reminders.lastImportCount == 1)
        #expect(Set(h.state.engine.allTasks().map(\.title)) == ["Existing task", "Synthetic reminder"])
        await h.state.reminders.importNow()
        #expect(h.state.reminders.lastImportCount == 0)
        #expect(h.state.engine.allTasks().count == 2)
    }

    @Test func deletionUndoAndRelaunchRetainHistoryWithoutResurrection() async throws {
        let h = MacReminderHarness()
        await h.select()
        await h.state.reminders.importNow()
        let id = try #require(h.state.engine.allTasks().first?.id)
        h.state.delete([id])
        #expect(h.state.engine.allTasks().isEmpty)
        h.state.undo()
        #expect(h.state.engine.allTasks().map(\.id) == [id])
        h.state.delete([id])
        #expect(h.state.engine.allTasks().isEmpty)
        let reopened = try Engine.openChecked(dir: h.state.dataDir.path)
        let state = AppState(engine: reopened, defaults: h.defaults, services: false, remindersSource: h.source)
        #expect(state.reminders.selectedListID == "synthetic-list")
        #expect(!state.reminders.automatic)
        await state.reminders.importNow()
        #expect(state.reminders.lastImportCount == 0)
        #expect(reopened.allTasks().isEmpty)
    }

    @Test func automaticImportRunsOnlyWhileApplicationActiveAndNeverPromptsAtLaunch() async {
        let h = MacReminderHarness()
        await h.state.setApplicationActive(true)?.value
        #expect(!h.state.reminders.automatic)
        #expect(h.source.prompts == 0)
        #expect(h.source.fetches == 0)
        await h.select()
        h.state.setApplicationActive(false)
        h.state.reminders.setAutomatic(true)
        h.source.didChange?()
        await Task.yield()
        #expect(h.source.fetches == 0)
        await h.state.setApplicationActive(true)?.value
        #expect(h.state.engine.allTasks().count == 1)
        #expect(!h.state.reminders.busy)
        #expect(h.source.prompts == 0)
        let before = h.source.fetches
        h.state.setApplicationActive(false)
        h.source.items.append(ReminderImportItem(sourceID: "second", title: "Second synthetic reminder"))
        h.source.didChange?()
        await Task.yield()
        #expect(h.source.fetches == before)
        #expect(h.state.engine.allTasks().count == 1)
        await h.state.setApplicationActive(true)?.value
        #expect(h.state.engine.allTasks().count == 2)
        #expect(!h.state.reminders.busy)
        h.state.stopServices()
        let stopped = h.source.fetches
        h.source.didChange?()
        await Task.yield()
        #expect(h.source.fetches == stopped)
        h.state.reminders.setAutomatic(false)
    }

    @Test func servicesFreeDefaultAndDemoSuppressAccessEvenWithInjectedSource() async {
        let h = MacReminderHarness(demo: true)
        h.state.setApplicationActive(true)
        await h.state.reminders.connect()
        await h.state.reminders.importNow()
        #expect(h.state.reminders.issue == .unavailable)
        #expect(h.source.prompts == 0)
        #expect(h.source.fetches == 0)
        #expect(h.source.didChange == nil)
        let state = AppState(engine: h.state.engine, defaults: h.defaults, services: false)
        await state.reminders.connect()
        #expect(state.reminders.issue == .unavailable)
    }

}
