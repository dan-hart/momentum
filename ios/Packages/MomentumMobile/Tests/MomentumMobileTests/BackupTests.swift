// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import Testing
@testable import MomentumMobile

@Suite @MainActor struct BackupTests {
    @Test func exportAndRestoreUseTheSharedFormatAndPreserveIdentity() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: root) }
        let source = try await EngineWorker.openChecked(directory: root.appendingPathComponent("source"))
        _ = await source.perform(.add("Portable task #Work 30m", .today))
        let original = await source.snapshot(view: .today)
        let data = try await source.exportBackup()
        #expect(await source.snapshot(view: .today).sync.pendingOps == original.sync.pendingOps)
        let target = try await EngineWorker.openChecked(directory: root.appendingPathComponent("target"))
        _ = await target.perform(.add("Replaced", .today))
        let result = try await target.restoreBackup(BackupFile(name: "fixture.json", data: data))
        #expect(result.changed)
        let restored = await target.snapshot(view: .today)
        #expect(restored.tasks.map(\.id) == original.tasks.map(\.id))
        #expect(restored.tasks.map(\.title) == ["Portable task"])
        #expect(restored.sync.pendingOps == 0)
        #expect(!restored.canUndo)
    }

    @Test func selectingAndCancellingNeverRestoresWithoutConfirmation() async {
        let state = BackupState()
        let file = BackupFile(name: "fixture.json", data: Data("{}".utf8))
        state.select(file)
        #expect(state.selection == file)
        #expect(state.completion == nil)
        state.discardSelection()
        var calls = 0
        #expect(await state.restore { _ in calls += 1 } == false)
        #expect(calls == 0)
    }

    @Test func malformedRestorePreservesTasksAndRetainsSelection() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: root) }
        let worker = try await EngineWorker.openChecked(directory: root)
        _ = await worker.perform(.add("Keep me", .today))
        let before = try Data(contentsOf: root.appendingPathComponent("state.json"))
        let state = BackupState()
        let file = BackupFile(name: "invalid.json", data: Data("{}".utf8))
        state.select(file)
        #expect(await state.restore { _ = try await worker.restoreBackup($0) } == false)
        #expect(state.selection == file)
        #expect(state.error != nil)
        #expect(state.completion == nil)
        #expect(try Data(contentsOf: root.appendingPathComponent("state.json")) == before)
        #expect(await worker.snapshot(view: .today).tasks.map(\.title) == ["Keep me"])
    }

    @Test func failedWriteRetainsSelectedBytesAndCanRetry() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: root) }
        let worker = try await EngineWorker.openChecked(directory: root)
        _ = await worker.perform(.add("Backup task", .today))
        let file = BackupFile(name: "backup.json", data: try await worker.exportBackup())
        _ = await worker.perform(.add("Current task", .today))
        let state = BackupState()
        state.select(file)
        let blocked = root.appendingPathComponent("pending.json.tmp")
        try FileManager.default.createDirectory(at: blocked, withIntermediateDirectories: false)
        #expect(await state.restore { _ = try await worker.restoreBackup($0) } == false)
        #expect(state.selection == file)
        #expect(await worker.snapshot(view: .today).tasks.count == 2)
        try FileManager.default.removeItem(at: blocked)
        #expect(await state.restore { _ = try await worker.restoreBackup($0) })
        #expect(state.selection == nil)
        #expect(state.error == nil)
        #expect(state.completion == .restored)
        #expect(await worker.snapshot(view: .today).tasks.map(\.title) == ["Backup task"])
    }

    @Test func restoreRejectsOverlappingOperations() async {
        let state = BackupState()
        state.select(BackupFile(name: "fixture.json", data: Data()))
        var calls = 0
        let restored = await state.restore { _ in
            #expect(state.activity == .restoring)
            #expect(await state.restore { _ in calls += 1 } == false)
            #expect(await state.prepareExport { calls += 1; return Data() } == false)
        }
        #expect(restored)
        #expect(calls == 0)
    }

    @Test func exportSuccessIsReportedOnlyAfterTheSystemSaves() async {
        let state = BackupState()
        #expect(await state.prepareExport { Data("fixture".utf8) })
        #expect(state.completion == nil)
        #expect(state.preparedExport != nil)
        state.finishExport(nil)
        #expect(state.preparedExport == nil)
        #expect(state.completion == nil)
        #expect(state.error == nil)
        #expect(await state.prepareExport { Data() })
        state.finishExport(.failure(CocoaError(.fileWriteOutOfSpace)))
        #expect(state.error != nil)
        #expect(state.completion == nil)
        #expect(await state.prepareExport { Data() })
        state.finishExport(.success(URL(fileURLWithPath: "/fixture.json")))
        #expect(state.error == nil)
        #expect(state.completion == .exported)
    }

    @Test func loadedFileCapturesStableBytesAndMissingFileClearsSelection() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: false)
        defer { try? FileManager.default.removeItem(at: root) }
        let url = root.appendingPathComponent("fixture.json")
        let data = Data("original".utf8)
        try data.write(to: url)
        let state = BackupState()
        let reader: @Sendable (URL) async throws -> BackupFile = {
            BackupFile(name: $0.lastPathComponent, data: try Data(contentsOf: $0))
        }
        #expect(await state.load(url, reader: reader))
        try Data("changed externally".utf8).write(to: url)
        #expect(state.selection?.data == data)
        #expect(state.selection?.name == "fixture.json")
        #expect(await state.load(root.appendingPathComponent("missing.json"), reader: reader) == false)
        #expect(state.selection == nil)
        #expect(state.error != nil)
    }

    @Test func readCancellationIsNotPresentedAsFailure() async {
        let state = BackupState()
        #expect(await state.load(URL(fileURLWithPath: "/fixture")) { _ in throw CancellationError() } == false)
        #expect(state.error == nil)
        #expect(state.selection == nil)
        #expect(state.activity == nil)
    }
}
