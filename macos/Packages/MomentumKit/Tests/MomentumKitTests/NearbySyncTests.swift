// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import Testing
@testable import MomentumKit

/// Opt-in socket integration; the normal host-less suite remains network-free.
@Suite(.enabled(if: ProcessInfo.processInfo.environment["MOMENTUM_TEST_NEARBY"] == "1"))
@MainActor struct NearbySyncTests {
    @Test func swiftBridgePairsAndRefreshesTasksInBothDirections() async throws {
        let left = Harness(demo: false)
        let right = Harness(demo: false)
        left.defaults.set("libresync", forKey: PrefKey.syncMethod)
        right.defaults.set("libresync", forKey: PrefKey.syncMethod)
        let leftInfo = try left.engine.p2pStart(
            delegate: P2pBridge(state: left.state), secrets: UnusedSecrets(),
            deviceName: "Momentum integration left", demoKeyDir: left.dir.url.appendingPathComponent("keys").path)
        defer { left.engine.p2pStop() }
        let rightInfo = try right.engine.p2pStart(
            delegate: P2pBridge(state: right.state), secrets: UnusedSecrets(),
            deviceName: "Momentum integration right", demoKeyDir: right.dir.url.appendingPathComponent("keys").path)
        defer { right.engine.p2pStop() }
        #expect(leftInfo.port != rightInfo.port)

        let code = try #require(right.engine.p2pBeginPairing())
        try left.engine.p2pLink(address: "127.0.0.1:\(rightInfo.port)", code: code)
        try await eventually("both Swift states receive the linked devices") {
            left.state.nearbyLinked.count == 1 && right.state.nearbyLinked.count == 1
        }
        left.engine.p2pEndPairing()
        right.engine.p2pEndPairing()

        left.state.addTask("Task over the Swift LibreSync bridge")
        let id = try #require(left.engine.allTasks().first?.id)
        _ = try left.engine.p2pSyncNow()
        try await eventually("remote change refreshes the Swift task list and search index") {
            right.state.row(id)?.title == "Task over the Swift LibreSync bridge"
                && right.indexer.indexed.last?.contains(where: { $0.id == id }) == true
        }

        right.state.setDone(id, true)
        _ = try right.engine.p2pSyncNow()
        try await eventually("completion travels back to the first Swift state") {
            left.state.row(id)?.isDone == true
        }
        #expect(left.engine.allTasks().count == 1)
        #expect(right.engine.allTasks().count == 1)

        try await eventually("the completed exchange clears progress") {
            !left.state.syncInProgress && !right.state.syncInProgress
        }
        #expect(left.state.syncError == nil)
        #expect(left.state.syncStatus.lastNearbyMs > 0)
        let previousSync = left.state.syncStatus.lastNearbyMs
        left.state.sync()
        #expect(left.state.syncInProgress, "manual sync shows progress immediately")
        try await eventually("an unchanged exchange still records success") {
            !left.state.syncInProgress && left.state.syncStatus.lastNearbyMs > previousSync
        }
        #expect(left.state.syncError == nil)

        left.engine.p2pUnlink(deviceId: rightInfo.deviceId)
        try await eventually("unlink refreshes the macOS device list") {
            left.state.nearbyLinked.isEmpty
        }
        left.state.sync()
        try await eventually("sync without a linked device ends with an explanation") {
            !left.state.syncInProgress && left.state.syncError != nil
        }
        #expect(left.state.syncError?.contains("No linked devices") == true)
    }

    private func eventually(_ message: String, condition: () -> Bool) async throws {
        let clock = ContinuousClock()
        let deadline = clock.now + .seconds(15)
        while !condition(), clock.now < deadline {
            try await Task.sleep(for: .milliseconds(25))
        }
        try #require(condition(), Comment(rawValue: message))
    }
}

/// File-backed test keys must bypass the login Keychain entirely.
private final class UnusedSecrets: SecretStore, Sendable {
    func get(name: String) -> String? { nil }
    func set(name: String, value: String) -> Bool { false }
    func delete(name: String) {}
}
