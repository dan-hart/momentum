import Foundation
import MomentumCore
import Testing
@testable import MomentumKit

@MainActor @Suite struct AutomationTests {
    @Test func cliChangesRefreshTasksIndexAndSharedSettings() throws {
        let h = Harness()
        let config = #"{"server":"https://example.test","user":"terminal","folder":"tasks","compress":false}"#
        try Data(config.utf8).write(to: h.state.dataDir.appendingPathComponent("cli-config.json"))
        let task = try TaskAutomation(state: h.state).create(title: "From terminal")
        _ = h.engine.setDone(id: task.id, done: true)
        h.state.cliStoreChanged()
        #expect(h.state.row(task.id)?.isDone == true)
        #expect(h.indexer.indexed.last?.first(where: { $0.id == task.id })?.isDone == true)
        #expect(h.defaults.string(forKey: PrefKey.nextcloudUser) == "terminal")
        #expect(!h.state.prefs.syncEnabled)

        // A CLI configuration saved while the app is closed survives the next launch.
        h.defaults.set("old-user", forKey: PrefKey.nextcloudUser)
        let reopened = AppState(engine: h.engine, defaults: h.defaults, services: false)
        #expect(reopened.defaults.string(forKey: PrefKey.nextcloudUser) == "terminal")
    }

    @Test func createReturnsStableTaskAndIgnoresCurrentWindowContext() throws {
        let h = Harness()
        h.state.go(to: .tag(id: h.tagId("urgent")))
        let automation = TaskAutomation(state: h.state)
        let task = try automation.create(title: "Prepare notes #work 1h 30m", notes: "For the meeting", planning: .tomorrow)
        let detail = try #require(h.engine.taskDetail(id: task.id))
        #expect(task.title == "Prepare notes")
        #expect(detail.notes == "For the meeting")
        #expect(detail.estimateMs == 5_400_000)
        #expect(detail.dueDay == dayOffset(day: today(), days: 1))
        #expect(!detail.tagIds.contains(h.tagId("urgent")))
        #expect(h.state.view == .tag(id: h.tagId("urgent")))
        #expect(h.indexer.indexed.last?.contains(where: { $0.id == task.id }) == true)
        #expect(try automation.tasks(ids: [task.id]).first?.id == task.id)
    }

    @Test func creationValidatesBeforeWritingAndSupportsProjectAndCustomDate() throws {
        let h = Harness()
        let automation = TaskAutomation(state: h.state)
        let before = h.engine.allTasks().count
        #expect(throws: AutomationError.emptyTitle) { try automation.create(title: "#onlytag 30m") }
        #expect(throws: AutomationError.projectUnavailable) { try automation.create(title: "Invalid", projectId: "missing") }
        #expect(h.engine.allTasks().count == before)
        let date = try #require(Strings.date(fromDay: "2030-02-03"))
        let task = try automation.create(title: "Dated", planning: .unscheduled, projectId: h.projectId("Home"), dueDate: date)
        #expect(task.dueDate == date)
        #expect(task.project == "Home")
    }

    @Test func queriesUseCoreViewMembershipAndExplicitCompletionFiltering() throws {
        let h = Harness()
        let automation = TaskAutomation(state: h.state)
        let tasks = automation.find(scope: .today)
        #expect(tasks.contains { $0.title == "Renew library books" })
        let id = h.id("Write release notes for 0.1")
        #expect(try automation.setCompleted(ids: [id, id], completed: true) == 1)
        #expect(automation.find(titleContains: "release").isEmpty)
        #expect(automation.find(titleContains: "RELEASE", includeCompleted: true).first?.isCompleted == true)
        #expect(try automation.setCompleted(ids: [id], completed: false) == 1)
        #expect(automation.find(titleContains: "release").first?.isCompleted == false)
    }

    @Test func staleBatchDoesNotPartiallyMutateAndPlanningIsIdempotent() throws {
        let h = Harness()
        let automation = TaskAutomation(state: h.state)
        let task = try automation.create(title: "Unscheduled", planning: .unscheduled)
        #expect(throws: AutomationError.taskUnavailable) { try automation.setCompleted(ids: [task.id, "missing"], completed: true) }
        #expect(h.engine.taskDetail(id: task.id)?.isDone == false)
        #expect(try automation.planToday(ids: [task.id, task.id]) == 1)
        #expect(try automation.planToday(ids: [task.id]) == 0)
        #expect(h.engine.taskDetail(id: task.id)?.dueDay == today())
    }

    @Test func automationHonorsAutoArchiveAndRejectsArchivedInputs() throws {
        let h = Harness()
        h.defaults.set(true, forKey: PrefKey.autoArchive)
        h.state.preferencesChanged()
        let automation = TaskAutomation(state: h.state)
        let id = h.id("Write release notes for 0.1")
        #expect(try automation.setCompleted(ids: [id], completed: true) == 1)
        #expect(h.engine.taskDetail(id: id) == nil)
        #expect(throws: AutomationError.taskUnavailable) { try automation.setCompleted(ids: [id], completed: false) }
        h.state.undo()
        #expect(h.engine.taskDetail(id: id)?.isDone == false)
    }

    @Test func reopeningSeveralTasksDoesNotCompleteOrArchiveThem() throws {
        let h = Harness()
        let automation = TaskAutomation(state: h.state)
        let ids = try ["First", "Second"].map { try automation.create(title: $0).id }
        #expect(try automation.setCompleted(ids: ids, completed: true) == 2)
        h.defaults.set(true, forKey: PrefKey.autoArchive)
        h.state.preferencesChanged()
        #expect(try automation.setCompleted(ids: ids, completed: false) == 2)
        for id in ids { #expect(h.engine.taskDetail(id: id)?.isDone == false) }
        h.state.undo()
        for id in ids { #expect(h.engine.taskDetail(id: id)?.isDone == true) }
    }

    @Test func planningTimedTaskRemovesTimeAndUpcomingCanIncludeCompleted() throws {
        let h = Harness()
        let automation = TaskAutomation(state: h.state)
        let task = try automation.create(title: "Scheduled")
        let draft = TaskDraft(title: task.title, projectId: "", dueDay: today(),
                              time: ClockTime(hour: 15, minute: 0), reminderMinutesBefore: 10,
                              estimateMs: 0, notes: "", tagIds: [], newTags: [])
        _ = h.engine.saveTask(id: task.id, draft: draft)
        #expect(try automation.planToday(ids: [task.id]) == 1)
        #expect(h.engine.taskDetail(id: task.id)?.time == nil)
        #expect(h.engine.taskDetail(id: task.id)?.reminderMinutesBefore == nil)
        let future = try automation.create(title: "Upcoming completed", planning: .tomorrow)
        _ = try automation.setCompleted(ids: [future.id], completed: true)
        #expect(!automation.find(scope: .upcoming).contains { $0.id == future.id })
        #expect(automation.find(scope: .upcoming, includeCompleted: true).contains { $0.id == future.id })
    }
}
