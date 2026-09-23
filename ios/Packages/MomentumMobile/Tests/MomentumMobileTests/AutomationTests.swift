// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import MomentumKit
import Testing
@testable import MomentumMobile

@Suite struct MobileAutomationTests {
    @Test func createsQueriesAndMutatesAtomicallyThroughSharedRules() async throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: dir) }
        let worker = await EngineWorker.open(directory: dir)
        let (task, created) = try await worker.automationCreate(title: "Read #Books 30m", notes: "Chapter two", planning: .unscheduled)
        #expect(created.changed)
        #expect(task.notes == "Chapter two")
        #expect(task.dueDate == nil)
        let detail = try #require(await worker.taskDetail(task.id))
        #expect(detail.estimateMs == 1_800_000)
        do {
            _ = try await worker.automationSetCompleted(ids: [task.id, "missing"], completed: true)
            Issue.record("A stale batch must fail before mutation")
        } catch { #expect(error as? AutomationError == .taskUnavailable) }
        #expect(await worker.taskDetail(task.id)?.isDone == false)
        #expect(try await worker.automationPlanToday(ids: [task.id, task.id]).0 == 1)
        #expect(try await worker.automationPlanToday(ids: [task.id]).0 == 0)
        #expect(await worker.automationFind(titleContains: "READ", scope: .today).map(\.id) == [task.id])
        #expect(try await worker.automationSetCompleted(ids: [task.id], completed: true).0 == 1)
        #expect(await worker.automationFind().isEmpty)
        #expect(await worker.automationFind(includeCompleted: true).count == 1)
        #expect(try await worker.automationSetCompleted(ids: [task.id], completed: false).0 == 1)
        #expect(await worker.perform(.undo).changed)
        #expect(await worker.taskDetail(task.id)?.isDone == true)
    }

    @Test func entityResolutionOmitsStaleIDsWithoutWeakeningMutationValidation() async throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: dir) }
        let worker = await EngineWorker.open(directory: dir)
        let (first, _) = try await worker.automationCreate(title: "First")
        let (second, _) = try await worker.automationCreate(title: "Second")
        let resolved = await worker.automationResolveTasks(ids: [second.id, "missing", first.id, second.id])
        #expect(resolved.map(\.id) == [second.id, first.id])
        #expect(await worker.automationResolveTasks(ids: []).isEmpty)
        do {
            _ = try await worker.automationPlanToday(ids: [first.id, "missing"])
            Issue.record("Mutations still reject a stale batch")
        } catch { #expect(error as? AutomationError == .taskUnavailable) }
    }

    @Test func creationRejectsInvalidInputAndDueOverridesPlan() async throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: dir) }
        let worker = await EngineWorker.open(directory: dir)
        for (title, project, expected) in [("#Books 30m", Optional<String>.none, AutomationError.emptyTitle), ("Invalid", "missing", .projectUnavailable)] {
            do { _ = try await worker.automationCreate(title: title, projectId: project); Issue.record("Expected rejected input") }
            catch { #expect(error as? AutomationError == expected) }
        }
        #expect(await worker.automationFind(includeCompleted: true).isEmpty)
        let date = try #require(Strings.date(fromDay: "2030-02-03"))
        let (task, _) = try await worker.automationCreate(title: "Dated", planning: .tomorrow, dueDate: date)
        #expect(task.dueDate == date)
        #expect(await worker.taskDetail(task.id)?.dueDay == "2030-02-03")
    }
    @Test func failedDurableAutomationDoesNotReturnSuccessOrMutateTasks() async throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: dir) }
        let worker = await EngineWorker.open(directory: dir)
        let (task, _) = try await worker.automationCreate(title: "Keep me", planning: .unscheduled)
        try FileManager.default.createDirectory(at: dir.appendingPathComponent("pending.json.tmp"), withIntermediateDirectories: false)
        do {
            _ = try await worker.automationSetCompleted(ids: [task.id], completed: true)
            Issue.record("A failed commit must throw instead of returning a success count")
        } catch {
            if let failure = error as? AutomationError, case .saveFailed(let reason) = failure {
                #expect(!reason.isEmpty)
            } else { Issue.record("Expected a specific save failure, got \(error)") }
        }
        do {
            _ = try await worker.automationPlanToday(ids: [task.id])
            Issue.record("A failed plan must throw instead of returning a success count")
        } catch {
            if let failure = error as? AutomationError, case .saveFailed(let reason) = failure {
                #expect(!reason.isEmpty)
            } else { Issue.record("Expected a specific save failure, got \(error)") }
        }
        #expect(await worker.taskDetail(task.id)?.isDone == false)
        #expect(await worker.taskDetail(task.id)?.dueDay == nil)
    }

}
