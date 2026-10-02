// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import Testing
import MomentumKit
@testable import MomentumMobile

@Suite struct ReminderWorkerTests {
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
