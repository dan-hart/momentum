// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import MomentumKit
import Testing
@testable import MomentumMobile

/// Opt-in: real sockets, disposable file-backed keys, no personal data or Keychain.
@Suite(.enabled(if: ProcessInfo.processInfo.environment["MOMENTUM_TEST_NEARBY"] == "1"))
@MainActor struct CoreNearbyRuntimeTests {
    @Test(.timeLimit(.minutes(1))) func mobileRuntimePairsExchangesAndStopsAcrossForegroundChanges() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent("nearby-mobile-\(UUID())")
        defer { try? FileManager.default.removeItem(at: directory) }
        let leftWorker = try await EngineWorker.openChecked(directory: directory.appendingPathComponent("left"))
        let rightWorker = try await EngineWorker.openChecked(directory: directory.appendingPathComponent("right"))
        let left = CoreNearbyRuntime(engine: await leftWorker.engine, keychain: Keychain(service: "unused-fixture"),
                                     deviceName: "Fixture left", demoKeyDirectory: directory.appendingPathComponent("left-keys").path)
        let right = CoreNearbyRuntime(engine: await rightWorker.engine, keychain: Keychain(service: "unused-fixture"),
                                      deviceName: "Fixture right", demoKeyDirectory: directory.appendingPathComponent("right-keys").path)
        let leftState = NearbyLifecycle(runtime: left, allowed: true)
        let rightState = NearbyLifecycle(runtime: right, allowed: true)
        var remoteChanges = 0
        var exchangeErrors: [String] = []
        var cycleStarts = 0
        var cycleEnds = 0
        leftState.onEvent = {
            if case .syncStarted = $0 { cycleStarts += 1 }
            if case .syncCompleted(_, let error) = $0 { cycleEnds += 1; if let error { exchangeErrors.append(error) } }
        }
        rightState.onEvent = {
            if $0 == .storeChanged { remoteChanges += 1 }
            if case .syncStarted = $0 { cycleStarts += 1 }
            if case .syncCompleted = $0 { cycleEnds += 1 }
            if case .syncCompleted(_, let error) = $0, let error { exchangeErrors.append(error) }
        }
        do {
            for state in [leftState, rightState] {
                state.setSelected(true); state.setForeground(true); await state.drain()
                try #require(state.phase == .running)
            }
            let port = try #require(rightState.info?.port)
            let code = try #require(await right.beginPairing())
            try await left.link(address: "127.0.0.1:\(port)", code: code)
            try await eventually("both peers linked") {
                let a = await left.snapshot(); let b = await right.snapshot()
                return a.linked.count == 1 && b.linked.count == 1
            }
            await left.endPairing(); await right.endPairing()
            _ = await leftWorker.perform(.add("Shared mobile fixture", .today))
            let original = await leftWorker.snapshot(view: .today)
            let id = try #require(original.tasks.first?.id)
            _ = try await left.syncNow()
            try await eventually("task and callback reached the right peer") { await rightWorker.snapshot(view: .today).tasks.contains { $0.id == id } && remoteChanges > 0 }
            #expect(remoteChanges > 0)
            let completion = await rightWorker.perform(.complete([id], true))
            try #require(completion.changed)
            _ = try await right.syncNow()
            try await eventually("completion reached the left peer") { await leftWorker.snapshot(view: .today).tasks.first { $0.id == id }?.isDone == true }
            leftState.setForeground(false); await leftState.drain()
            #expect(await leftWorker.engine.p2pRunning() == false)
            #expect(await left.beginPairing() == nil)
            leftState.setForeground(true); await leftState.drain()
            #expect(leftState.phase == .running)
            let restarted = await left.snapshot()
            #expect(restarted.linked.count == 1)
            let peer = try #require(restarted.linked.first)
            await left.unlink(id: peer.deviceId)
            #expect(await left.snapshot().linked.isEmpty)
        } catch {
            let leftTasks = await leftWorker.snapshot(view: .search, query: "Shared mobile fixture")
            let rightTasks = await rightWorker.snapshot(view: .search, query: "Shared mobile fixture")
            print("Isolated peer diagnostic: left done=\(leftTasks.tasks.map(\.isDone)), right done=\(rightTasks.tasks.map(\.isDone)), failures=\(exchangeErrors)")
            for (label, runtime) in [("left", left), ("right", right)] {
                let snapshot = await runtime.snapshot()
                let root = directory.appendingPathComponent(label)
                let pending = (try? JSONSerialization.jsonObject(with: Data(contentsOf: root.appendingPathComponent("pending.json")))) as? [Any]
                let journal = (try? JSONSerialization.jsonObject(with: Data(contentsOf: root.appendingPathComponent("p2p/journal.json")))) as? [String: Any]
                print("Fixture \(label): pending=\(pending?.count ?? -1), own=\((journal?["own"] as? [Any])?.count ?? -1), inbox=\((journal?["inbox"] as? [Any])?.count ?? -1), missingAddress=\(snapshot.linked.map { $0.address == nil }), cycles=\(cycleStarts)/\(cycleEnds)")
            }
            await left.stop(); await right.stop()
            throw error
        }
        for state in [leftState, rightState] { state.setSelected(false); await state.drain() }
    }

    private func eventually(_ description: String, _ condition: @MainActor () async -> Bool) async throws {
        let deadline = ContinuousClock.now + .seconds(15)
        while !(await condition()), ContinuousClock.now < deadline { try await Task.sleep(for: .milliseconds(25)) }
        try #require(await condition(), Comment(rawValue: description))
    }
}
