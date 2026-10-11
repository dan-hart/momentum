// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import MomentumKit
import Testing
@testable import MomentumMobile

@Suite struct SharedTaskImportTests {
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
        #expect(detail.dueDay == nil, "shared items land in Inbox, not Today")
        let reopened = await EngineWorker.open(directory: directory)
        #expect(try await reopened.importSharedTask(item) == .skipped, "the identity survives a relaunch")
        #expect(try await reopened.importSharedTask(SharedTaskItem(title: "", notes: "", url: nil)) == .skipped)
    }
}
