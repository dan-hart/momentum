// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import MomentumKit
import Testing
@testable import MomentumMobile

@Suite struct TaskEditingTests {
    private func directory() -> URL {
        FileManager.default.temporaryDirectory.appendingPathComponent("momentum-editing-\(UUID())")
    }

    @Test func duplicateSelectionDoesNotFollowAnotherCreation() async throws {
        let dir = directory()
        defer { try? FileManager.default.removeItem(at: dir) }
        let worker = await EngineWorker.open(directory: dir)
        _ = await worker.perform(.add("Original task", .today))
        let original = try #require(await worker.lastAddedTaskID())
        let result = await worker.duplicateTaskWithID(original)
        #expect(result.outcome.changed)
        // Later creation must not replace the identity returned by duplication.
        _ = await worker.perform(.add("Concurrent creation", .today))
        let selected = try #require(result.taskID)
        #expect(selected != original)
        #expect(await worker.taskDetail(selected)?.title == "Original task")
    }

    @Test func concurrentDuplicatesReturnDistinctCopiesAndFailureHasNoIdentity() async throws {
        let dir = directory()
        defer { try? FileManager.default.removeItem(at: dir) }
        let worker = await EngineWorker.open(directory: dir)
        _ = await worker.perform(.add("Original task", .today))
        let original = try #require(await worker.lastAddedTaskID())
        let copies = await withTaskGroup(of: String?.self, returning: [String].self) { group in
            for index in 0..<8 {
                group.addTask { await worker.duplicateTaskWithID(original).taskID }
                group.addTask {
                    _ = await worker.perform(.add("Other creation \(index)", .today))
                    return nil
                }
            }
            var ids: [String] = []
            for await id in group { if let id { ids.append(id) } }
            return ids
        }
        #expect(copies.count == 8)
        #expect(Set(copies).count == 8)
        for id in copies { #expect(await worker.taskDetail(id)?.title == "Original task") }
        let rejected = await worker.duplicateTaskWithID("missing-task")
        #expect(!rejected.outcome.changed)
        #expect(rejected.taskID == nil)
    }

    @Test func concurrentQuickAddsReturnTheirOwnCreatedTaskIDs() async throws {
        let dir = directory()
        defer { try? FileManager.default.removeItem(at: dir) }
        let worker = await EngineWorker.open(directory: dir)
        let created = await withTaskGroup(of: (String, String)?.self, returning: [(String, String)].self) { group in
            for index in 0..<8 {
                group.addTask {
                    let title = "Quick add \(index)"
                    let result = await worker.createTaskFromText(title, view: .today)
                    return result.createdTaskID.map { ($0, title) }
                }
                group.addTask {
                    _ = await worker.perform(.add("Other creation \(index)", .today))
                    return nil
                }
            }
            var results: [(String, String)] = []
            for await result in group { if let result { results.append(result) } }
            return results
        }

        #expect(created.count == 8)
        #expect(Set(created.map(\.0)).count == 8)
        for (id, title) in created {
            #expect(await worker.taskDetail(id)?.title == title)
        }
    }

    @Test func concurrentMultilineImportsReturnTheirLastCreatedTaskIDs() async throws {
        let dir = directory()
        defer { try? FileManager.default.removeItem(at: dir) }
        let worker = await EngineWorker.open(directory: dir)
        let created = await withTaskGroup(of: (String, String)?.self, returning: [(String, String)].self) { group in
            for index in 0..<8 {
                group.addTask {
                    let lastTitle = "Imported last \(index)"
                    let result = await worker.importTasksFromText("Imported first \(index)\n\(lastTitle)", view: .today)
                    return result.createdTaskID.map { ($0, lastTitle) }
                }
                group.addTask {
                    _ = await worker.perform(.add("Other creation \(index)", .today))
                    return nil
                }
            }
            var results: [(String, String)] = []
            for await result in group { if let result { results.append(result) } }
            return results
        }

        #expect(created.count == 8)
        #expect(Set(created.map(\.0)).count == 8)
        for (id, title) in created {
            #expect(await worker.taskDetail(id)?.title == title)
        }
    }

    @Test func fullFormRoundTripsAndPreservesTagOrder() async throws {
        let dir = directory()
        defer { try? FileManager.default.removeItem(at: dir) }
        let worker = await EngineWorker.open(directory: dir)
        let projects = await worker.editingProjects()
        var form = TaskFormModel(view: .today, projects: projects)
        form.title = "Review proposal"
        form.timeText = "14:30"
        form.reminder = 30
        form.estimateText = "1h 30m"
        form.notes = "First line\nSecond line"
        form.newTags = "Zebra, Alpha, Middle"
        let created = await worker.createTask(form.draft, view: .today)
        #expect(created.changed)
        let id = try #require(await worker.lastAddedTaskID())
        let detail = try #require(await worker.taskDetail(id))
        #expect(detail.time == ClockTime(hour: 14, minute: 30))
        #expect(detail.reminderMinutesBefore == 30)
        #expect(detail.estimateMs == 5_400_000)
        #expect(detail.notes == form.notes)
        let tags = await worker.editingTags()
        #expect(detail.tagIds.compactMap { id in tags.first { $0.id == id }?.title } == ["Zebra", "Alpha", "Middle"])
        var edited = TaskFormModel(detail: detail)
        edited.title = "Revised proposal"
        #expect(edited.draft.tagIds == detail.tagIds)
        let saved = await worker.saveTask(id, draft: edited.draft)
        #expect(saved.changed)
        let reopened = await EngineWorker.open(directory: dir)
        let persisted = try #require(await reopened.taskDetail(id))
        #expect(persisted.title == "Revised proposal")
        #expect(persisted.tagIds == detail.tagIds)
        #expect(persisted.dueDay == form.dueDay)
        #expect(persisted.projectId == form.projectId)
    }

    @Test func recurrenceRulesSavePauseAndStopThroughCore() async throws {
        let dir = directory()
        defer { try? FileManager.default.removeItem(at: dir) }
        let worker = await EngineWorker.open(directory: dir)
        _ = await worker.perform(.add("Water plants", .today))
        let id = try #require(await worker.lastAddedTaskID())
        var draft = try #require(await worker.repeatDraft(id))
        for cycle in [RepeatCycle.daily, .weekly, .monthly, .yearly] {
            draft.cycle = cycle
            draft.every = 2
            draft.weekdays = [false, true, false, true, false, false, false]
            draft.monthly = .nthWeekday(week: -1, weekday: 5)
            draft.startDate = today()
            draft.paused = true
            let outcome = await worker.saveRepeat(id, draft: draft)
            #expect(outcome.changed)
            let saved = try #require(await worker.repeatDraft(id))
            #expect(saved.cycle == cycle)
            #expect(saved.every == 2)
            #expect(saved.paused)
            #expect(saved.existing)
            if cycle == .weekly { #expect(saved.weekdays == draft.weekdays) }
            if cycle == .monthly { #expect(saved.monthly == draft.monthly) }
        }
        let stopped = await worker.stopRepeat(id)
        #expect(stopped.changed)
        #expect(await worker.taskDetail(id)?.repeat == nil)
    }

    @Test func subtaskDuplicateDeleteAndArchivedGuardUseCoreRules() async throws {
        let dir = directory()
        defer { try? FileManager.default.removeItem(at: dir) }
        let worker = await EngineWorker.open(directory: dir)
        _ = await worker.perform(.add("Prepare release", .today))
        let id = try #require(await worker.lastAddedTaskID())
        #expect(await worker.taskMenu(id)?.topLevel == true)
        let added = await worker.addSubtask(id, title: "Check notes")
        #expect(added.changed)
        let detail = try #require(await worker.taskDetail(id))
        let child = try #require(detail.subTasks.first)
        #expect(await worker.taskMenu(child.id)?.topLevel == false)
        let duplicate = await worker.duplicateTask(id)
        #expect(duplicate.changed)
        let copiedID = try #require(await worker.lastAddedTaskID())
        #expect(copiedID != id)
        let deleted = await worker.deleteTask(copiedID)
        #expect(deleted.changed)
        #expect(await worker.taskDetail(copiedID) == nil)
        _ = await worker.perform(.complete([id], true))
        _ = await worker.perform(.archive)
        #expect(await worker.taskMenu(id) == nil)
        let blocked = await worker.saveTask(id, draft: TaskFormModel(detail: detail).draft)
        #expect(!blocked.changed)
    }

    @Test func weeklyRepeatWithoutWeekdaysReturnsValidationAndPreservesSchedule() async throws {
        let dir = directory()
        defer { try? FileManager.default.removeItem(at: dir) }
        let worker = await EngineWorker.open(directory: dir)
        _ = await worker.perform(.add("Weekly review", .today))
        let id = try #require(await worker.lastAddedTaskID())
        var draft = try #require(await worker.repeatDraft(id))
        draft.cycle = .weekly
        draft.weekdays = Array(repeating: false, count: 7)
        let rejected = await worker.saveRepeat(id, draft: draft)
        #expect(!rejected.changed)
        #expect(rejected.message == .pickAWeekday)
        #expect(await worker.taskDetail(id)?.repeat == nil)
        draft.weekdays[1] = true
        _ = await worker.saveRepeat(id, draft: draft)
        let before = try #require(await worker.repeatDraft(id))
        draft.weekdays[1] = false
        let rejectedEdit = await worker.saveRepeat(id, draft: draft)
        #expect(!rejectedEdit.changed)
        #expect(rejectedEdit.message == .pickAWeekday)
        #expect(await worker.repeatDraft(id) == before)
    }

    @Test func editingTimeAndClearingSchedulePersistAcrossReopen() async throws {
        let dir = directory()
        defer { try? FileManager.default.removeItem(at: dir) }
        let worker = await EngineWorker.open(directory: dir)
        _ = await worker.perform(.add("Scheduled review", .today))
        let id = try #require(await worker.lastAddedTaskID())
        let detail = try #require(await worker.taskDetail(id))
        var form = TaskFormModel(detail: detail)
        form.timeText = "09:00"
        form.reminder = 30
        _ = await worker.saveTask(id, draft: form.draft)
        let scheduled = try #require(await worker.taskDetail(id))
        #expect(scheduled.time == ClockTime(hour: 9, minute: 0))
        #expect(scheduled.reminderMinutesBefore == 30)
        form.dueDay = nil
        form.timeText = ""
        form.reminder = nil
        _ = await worker.saveTask(id, draft: form.draft)
        let reopened = await EngineWorker.open(directory: dir)
        let unscheduled = try #require(await reopened.taskDetail(id))
        #expect(unscheduled.dueDay == nil)
        #expect(unscheduled.time == nil)
        #expect(unscheduled.reminderMinutesBefore == nil)
    }
}
