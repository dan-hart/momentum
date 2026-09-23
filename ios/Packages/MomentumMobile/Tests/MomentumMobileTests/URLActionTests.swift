// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import MomentumKit
import Testing
@testable import MomentumMobile

@Suite struct URLActionTests {
    @Test func parsesBothSchemesAndRetainsDesktopParameterSemantics() throws {
        let url = try #require(URL(string: "momentum://add?title=Review%20%23Work%2030m&notes=Line%201%0ALine%202&due=2030-01-02&tags=Home,%20Errands%20,,&title=Ignored"))
        #expect(TaskURLAction(url: url) == .create(text: "Review #Work 30m #Home #Errands", notes: "Line 1\nLine 2", due: "2030-01-02"))
        #expect(TaskURLAction(url: URL(string: "superproductivity://create-task?title=Read")!) == .create(text: "Read", notes: nil, due: nil))
        #expect(TaskURLAction(url: URL(string: "superproductivity://complete-task?title=Read")!) == .complete(title: "Read"))
    }

    @Test func rejectsUnrelatedOrInvalidActionsWithoutMutation() {
        for value in ["https://add?title=No", "momentum://delete-task?title=No", "momentum://add", "momentum://add?title=%20%0A", "momentum://complete-task"] {
            #expect(TaskURLAction(url: URL(string: value)!) == nil)
        }
    }

    @Test func appliesCreateAndCompleteThroughCoreAndPersists() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: directory) }
        let worker = await EngineWorker.open(directory: directory)
        let create = try #require(TaskURLAction(url: URL(string: "momentum://add?title=Read%20%23Books%2030m&notes=Bring%20book&due=2030-01-02")!))
        #expect(await worker.handle(create).changed)
        let results = await worker.snapshot(view: .search, query: "Read")
        let task = try #require(results.tasks.first)
        let detail = try #require(await worker.taskDetail(task.id))
        #expect(detail.notes == "Bring book")
        #expect(detail.dueDay == "2030-01-02")
        #expect(detail.estimateMs == 1_800_000)
        #expect(await worker.handle(.complete(title: "Read")).changed)
        let reopened = await EngineWorker.open(directory: directory)
        let saved = try #require(await reopened.taskDetail(task.id))
        #expect(saved.isDone)
        #expect(saved.notes == "Bring book")
    }
}
