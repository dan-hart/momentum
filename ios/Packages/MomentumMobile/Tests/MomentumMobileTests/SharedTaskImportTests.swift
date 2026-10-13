// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import MomentumKit
import Testing
@testable import MomentumMobile

@Suite struct SharedTaskImportTests {
    @Test func aShareKeepsTheDayCapturedAtAddAcrossDelayedImportAndRelaunch() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: directory) }
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        let item = try decoder.decode(SharedTaskItem.self, from: Data(#"{"id":"delayed","title":"Late share","notes":"Complete notes","sharedAt":"2026-10-12T04:59:00Z","plannedDay":"2026-10-11"}"#.utf8))
        let worker = await EngineWorker.open(directory: directory)
        #expect(try await worker.importSharedTask(item) == .created)
        let task = try #require(await worker.snapshot(view: .search, query: "Late share").tasks.first)
        let detail = try #require(await worker.taskDetail(task.id))
        #expect(detail.dueDay == "2026-10-11", "opening after midnight or travelling must not change the captured day")
        let reopened = await EngineWorker.open(directory: directory)
        #expect(try await reopened.importSharedTask(item) == .skipped)
        #expect(await reopened.taskDetail(task.id)?.dueDay == "2026-10-11")
    }
    @Test func aSharedLinkBecomesOneInboxTaskWithTheLinkInNotesAndNeverTwice() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: directory) }
        let worker = await EngineWorker.open(directory: directory)
        let draft = sharedTaskDraft(title: " Interface Sounds · Kenney ", text: nil, url: "https://kenney.nl/assets/interface-sounds")
        let item = SharedTaskItem(title: draft.title, notes: draft.notes, url: draft.url)
        #expect(try await worker.importSharedTask(item) == .created)
        #expect(try await worker.importSharedTask(item) == .skipped, "a repeated drain is harmless")
        let results = await worker.snapshot(view: .search, query: "Interface Sounds")
        #expect(results.tasks.count == 1)
        let task = try #require(results.tasks.first)
        let detail = try #require(await worker.taskDetail(task.id))
        #expect(detail.title == "Interface Sounds · Kenney")
        #expect(detail.notes == "https://kenney.nl/assets/interface-sounds")
        #expect(detail.dueDay == nil, "older queued shares preserve their unscheduled day")
        let reopened = await EngineWorker.open(directory: directory)
        #expect(try await reopened.importSharedTask(item) == .skipped, "the identity survives a relaunch")
        #expect(try await reopened.importSharedTask(SharedTaskItem(title: "", notes: "", url: nil)) == .skipped)
    }
}
