// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import Testing
@testable import MomentumMobile

@Suite struct EngineWorkerTests {
    func directory() -> URL {
        FileManager.default.temporaryDirectory.appendingPathComponent("momentum-mobile-test-\(UUID())")
    }

    @Test func offlineTaskPersistsAcrossReopen() async throws {
        let dir = directory()
        defer { try? FileManager.default.removeItem(at: dir) }
        let worker = await EngineWorker.open(directory: dir)
        let result = await worker.perform(.add("Plan the week #Work 30m", .today))
        #expect(result.changed)
        let first = await worker.snapshot(view: .today)
        let task = try #require(first.tasks.first)
        #expect(task.title == "Plan the week")
        let reopened = await EngineWorker.open(directory: dir)
        let saved = await reopened.snapshot(view: .today)
        #expect(saved.tasks.map(\.id) == first.tasks.map(\.id))
    }

    @Test func publishedTaskSnapshotRemainsStableAcrossLaterMutations() async throws {
        let dir = directory()
        defer { try? FileManager.default.removeItem(at: dir) }
        let worker = try await EngineWorker.openChecked(directory: dir)
        _ = await worker.perform(.add("Original task", .today))
        let original = await worker.snapshot(view: .today)
        let id = try #require(original.tasks.first?.id)
        #expect(original.tasks.map(\.title) == ["Original task"])
        _ = await worker.perform(.complete([id], true))
        let completed = await worker.snapshot(view: .today)
        #expect(completed.tasks.first?.isDone == true)
        #expect(original.tasks.first?.isDone == false)
        _ = await worker.perform(.delete([id]))
        #expect(await worker.snapshot(view: .today).tasks.isEmpty)
        #expect(original.tasks.map(\.id) == [id])
        #expect(completed.tasks.map(\.id) == [id])
        _ = await worker.perform(.undo)
        let restored = await worker.snapshot(view: .today)
        #expect(restored.tasks.map(\.id) == [id])
        #expect(restored.tasks.first?.isDone == true)
        #expect(original.tasks.first?.isDone == false)
    }

    @Test func filteredSubtasksRetainDistinctCurrentParentTitles() async throws {
        let dir = directory()
        defer { try? FileManager.default.removeItem(at: dir) }
        let worker = await EngineWorker.open(directory: dir)
        _ = await worker.perform(.add("Alpha", .today))
        let alpha = try #require(await worker.lastAddedTaskID())
        _ = await worker.addSubtask(alpha, title: "Review")
        _ = await worker.perform(.add("Beta", .today))
        let beta = try #require(await worker.lastAddedTaskID())
        _ = await worker.addSubtask(beta, title: "Review")

        let today = await worker.snapshot(view: .today)
        #expect(today.parentTitles == [alpha: "Alpha", beta: "Beta"])
        let search = await worker.snapshot(view: .search, query: "Review")
        #expect(search.tasks.count == 2)
        #expect(search.tasks.allSatisfy { $0.title == "Review" })
        #expect(search.parentTitles == [alpha: "Alpha", beta: "Beta"])

        let parent = try #require(await worker.taskDetail(alpha))
        let draft = TaskDraft(title: "Renamed Alpha", projectId: parent.projectId,
            dueDay: parent.dueDay, time: parent.time, reminderMinutesBefore: parent.reminderMinutesBefore,
            estimateMs: parent.estimateMs, notes: parent.notes, tagIds: parent.tagIds, newTags: [])
        #expect(await worker.saveTask(alpha, draft: draft).changed)
        let updated = await worker.snapshot(view: .search, query: "Review")
        #expect(updated.parentTitles == [alpha: "Renamed Alpha", beta: "Beta"])
        let empty = await worker.snapshot(view: .search, query: "No match")
        #expect(empty.parentTitles.isEmpty)
    }

    @Test func filteredArchivedSubtasksRetainParentTitlesAndStayReadOnly() async throws {
        let dir = directory()
        defer { try? FileManager.default.removeItem(at: dir) }
        let worker = await EngineWorker.open(directory: dir)
        _ = await worker.perform(.add("Alpha", .today))
        let alpha = try #require(await worker.lastAddedTaskID())
        _ = await worker.addSubtask(alpha, title: "Review")
        _ = await worker.perform(.add("Beta", .today))
        let beta = try #require(await worker.lastAddedTaskID())
        _ = await worker.addSubtask(beta, title: "Review")
        _ = await worker.perform(.complete([alpha, beta], true))
        _ = await worker.perform(.archive)

        let search = await worker.snapshot(view: .search, query: "Review")
        #expect(search.tasks.count == 2)
        #expect(search.tasks.allSatisfy { $0.archived && $0.isSubtask })
        #expect(search.parentTitles == [alpha: "Alpha", beta: "Beta"])
        for child in search.tasks {
            #expect(await worker.taskDetail(child.id) == nil)
            #expect(!(await worker.perform(.complete([child.id], false))).changed)
        }
        _ = await worker.perform(.undo)
        let restored = await worker.snapshot(view: .search, query: "Review")
        #expect(restored.tasks.allSatisfy { !$0.archived })
        #expect(restored.parentTitles == search.parentTitles)
    }

    @Test func completionAndUndoRefreshAllViews() async throws {
        let dir = directory()
        defer { try? FileManager.default.removeItem(at: dir) }
        let worker = await EngineWorker.open(directory: dir)
        _ = await worker.perform(.add("Ship the app", .today))
        let before = await worker.snapshot(view: .today)
        let task = try #require(before.tasks.first)
        _ = await worker.perform(.complete([task.id], true))
        let done = await worker.snapshot(view: .today)
        #expect(done.tasks.first?.isDone == true)
        #expect(done.canUndo)
        _ = await worker.perform(.undo)
        let restored = await worker.snapshot(view: .project(id: try #require(task.project?.id)))
        #expect(restored.tasks.first?.isDone == false)
    }

    @Test func searchIncludesArchiveAndRejectsArchivedEdits() async throws {
        let dir = directory()
        defer { try? FileManager.default.removeItem(at: dir) }
        let worker = await EngineWorker.open(directory: dir)
        _ = await worker.perform(.add("Prepare launch #Release", .today))
        let first = await worker.snapshot(view: .today)
        let task = try #require(first.tasks.first)
        _ = await worker.perform(.complete([task.id], true))
        _ = await worker.perform(.archive)
        let archived = await worker.snapshot(view: .archive)
        #expect(archived.tasks.first?.archived == true)
        let ignored = await worker.perform(.complete([task.id], false))
        #expect(!ignored.changed)
        let matches = await worker.snapshot(view: .search, query: "launch")
        #expect(matches.tasks.first?.id == task.id)
    }
}
