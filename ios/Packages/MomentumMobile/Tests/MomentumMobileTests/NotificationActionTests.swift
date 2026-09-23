// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import Testing
@testable import MomentumMobile

@Suite struct NotificationActionTests {
    @Test func duplicateDoneUsesSingleSharedOwnerAndOneUndo() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: directory) }
        let session = EngineSession()
        async let cold = session.open(directory: directory)
        async let warm = session.open(directory: directory)
        let (first, second) = try await (cold, warm)
        #expect(first === second)
        _ = await first.perform(.add("Finish once", .today))
        let task = try #require(await first.snapshot(view: .today).tasks.first)
        async let firstDone = first.performNotificationAction(.complete, taskID: task.id, deliveryID: "delivery")
        async let secondDone = second.performNotificationAction(.complete, taskID: task.id, deliveryID: "delivery")
        let results = await [firstDone, secondDone]
        #expect(results.compactMap { $0 }.filter(\.changed).count == 1)
        _ = await second.perform(.undo)
        #expect(await first.taskDetail(task.id)?.isDone == false)
        #expect(await first.performNotificationAction(.complete, taskID: task.id, deliveryID: "delivery") == nil)
        #expect(await first.taskDetail(task.id)?.isDone == false)
    }

    @Test func snoozeIsDeduplicatedAndMissingTaskDoesNotSucceed() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: directory) }
        let worker = await EngineWorker.open(directory: directory)
        _ = await worker.perform(.add("Remind once", .today))
        let task = try #require(await worker.snapshot(view: .today).tasks.first)
        #expect(await worker.performNotificationAction(.snooze, taskID: task.id, deliveryID: "delivery")?.changed == true)
        let now = UInt64(Date().timeIntervalSince1970 * 1_000)
        let plan = try await worker.notificationPlan(nowMs: now)
        let reminder = try #require(plan.requests.first)
        #expect(reminder.fireAtMs > now + 59 * 60_000)
        #expect(reminder.fireAtMs <= now + 60 * 60_000)
        #expect(await worker.performNotificationAction(.snooze, taskID: task.id, deliveryID: "delivery") == nil)
        _ = await worker.perform(.complete([task.id], true))
        #expect(await worker.performNotificationAction(.snooze, taskID: task.id, deliveryID: "stale-delivery")?.changed == false)
        let missing = await worker.performNotificationAction(.snooze, taskID: "missing", deliveryID: "missing-delivery")
        #expect(missing?.changed == false)
        #expect(await worker.performNotificationAction(.snooze, taskID: "missing", deliveryID: "missing-delivery") != nil)
    }

    @Test func doneActionHonorsTheSharedAutoArchivePreference() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: directory) }
        let worker = await EngineWorker.open(directory: directory)
        let preferences = MomentumCore.Preferences(groupBy: .morningNight, sort: .manual,
            direction: .ascending, upcomingDays: 7, autoArchive: true,
            morningSummaryEnabled: false, morningSummaryTime: ClockTime(hour: 8, minute: 0))
        await worker.setPreferences(preferences)
        _ = await worker.perform(.add("Archive from notification", .today))
        let task = try #require(await worker.snapshot(view: .today).tasks.first)

        let outcome = await worker.performNotificationAction(.complete, taskID: task.id,
                                                               deliveryID: "auto-archive")

        #expect(outcome?.changed == true)
        #expect(await worker.snapshot(view: .today).tasks.isEmpty)
        #expect(await worker.snapshot(view: .archive).tasks.map(\.id) == [task.id])
    }

    @Test func failedPersistenceDoesNotConsumeNotificationActionRetry() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: directory) }
        let worker = await EngineWorker.open(directory: directory)
        _ = await worker.perform(.add("Retry completion", .today))
        let task = try #require(await worker.snapshot(view: .today).tasks.first)
        let blocked = directory.appendingPathComponent("pending.json.tmp")
        try FileManager.default.createDirectory(at: blocked, withIntermediateDirectories: false)
        let failed = await worker.performNotificationAction(.complete, taskID: task.id, deliveryID: "fixture")
        #expect(failed?.changed == false)
        #expect(failed?.message != nil)
        #expect(await worker.taskDetail(task.id)?.isDone == false)
        try FileManager.default.removeItem(at: blocked)
        #expect(await worker.performNotificationAction(.complete, taskID: task.id, deliveryID: "fixture")?.changed == true)
        #expect(await worker.performNotificationAction(.complete, taskID: task.id, deliveryID: "fixture") == nil)
        #expect(await worker.perform(.undo).changed)
        #expect(await worker.taskDetail(task.id)?.isDone == false)
    }

}
