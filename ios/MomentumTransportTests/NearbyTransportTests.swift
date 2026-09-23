// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore
import MomentumKit
import MomentumMobile
import Security
import XCTest

/// Real iOS Swift -> UniFFI -> LibreSync sockets with isolated Security.framework keys.
/// Loopback proves transport/lifecycle behavior, not Bonjour or physical-device discovery.
@MainActor final class NearbyTransportTests: XCTestCase {
    func testNativePeersExchangeAndPreserveIdentityAcrossForegroundRestart() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let services = (0..<2).map { _ in "momentum-native-nearby-tests-\(UUID())" }
        defer {
            try? FileManager.default.removeItem(at: directory)
            for service in services {
                SecItemDelete([kSecClass as String: kSecClassGenericPassword,
                               kSecAttrService as String: service] as CFDictionary)
            }
        }
        let a = try await EngineWorker.openChecked(directory: directory.appendingPathComponent("a"))
        let b = try await EngineWorker.openChecked(directory: directory.appendingPathComponent("b"))
        let left = await a.nearbyRuntime(keychain: Keychain(service: services[0]), deviceName: "Fixture A")
        let right = await b.nearbyRuntime(keychain: Keychain(service: services[1]), deviceName: "Fixture B")
        let leftState = NearbyLifecycle(runtime: left, allowed: true)
        let rightState = NearbyLifecycle(runtime: right, allowed: true)
        var leftChanges = 0
        var rightChanges = 0
        leftState.onEvent = { if $0 == .storeChanged { leftChanges += 1 } }
        rightState.onEvent = { if $0 == .storeChanged { rightChanges += 1 } }
        do {
            for state in [leftState, rightState] {
                state.setSelected(true)
                state.setForeground(true)
                await state.drain()
                XCTAssertEqual(state.phase, .running)
            }
            let originalIdentity = try XCTUnwrap(leftState.info?.deviceId)
            let leftKeys = Keychain(service: services[0])
            let originalCertificate = try XCTUnwrap(leftKeys.readData("device-cert-der"))
            let port = try XCTUnwrap(rightState.info?.port)
            let code = await right.beginPairing()
            try await left.link(address: "127.0.0.1:\(port)", code: XCTUnwrap(code))
            try await eventually("Both native peers link") {
                let a = await left.snapshot(), b = await right.snapshot()
                return a.linked.count == 1 && b.linked.count == 1
            }
            await right.endPairing()
            _ = await a.perform(.add("Native nearby fixture", .today))
            let tasks = await a.snapshot(view: .today).tasks
            let id = try XCTUnwrap(tasks.first?.id)
            _ = try await left.syncNow()
            try await eventually("The receiving iOS store and callback reflect the task") {
                await b.snapshot(view: .today).tasks.contains { $0.id == id } && rightChanges > 0
            }
            _ = await b.perform(.complete([id], true))
            _ = try await right.syncNow()
            try await eventually("Completion and its inbound callback reach the first iOS peer") {
                await a.snapshot(view: .today).tasks.first { $0.id == id }?.isDone == true && leftChanges > 0
            }
            leftState.setForeground(false)
            await leftState.drain()
            XCTAssertEqual(leftState.phase, .stopped)
            let stoppedCode = await left.beginPairing()
            XCTAssertNil(stoppedCode)
            leftState.setForeground(true)
            await leftState.drain()
            XCTAssertEqual(leftState.phase, .running)
            XCTAssertEqual(leftState.info?.deviceId, originalIdentity)
            XCTAssertTrue(try leftKeys.readData("device-cert-der") == originalCertificate,
                          "Restart must reuse the trusted certificate")
            let restarted = await left.snapshot()
            XCTAssertEqual(restarted.linked.count, 1)
            _ = await a.perform(.complete([id], false))
            _ = try await left.syncNow()
            try await eventually("The existing peer accepts the restarted device without pairing again") {
                await b.snapshot(view: .today).tasks.first { $0.id == id }?.isDone == false
            }
            let peer = try XCTUnwrap(restarted.linked.first)
            await left.unlink(id: peer.deviceId)
            let unlinked = await left.snapshot()
            XCTAssertTrue(unlinked.linked.isEmpty)
        } catch {
            for state in [leftState, rightState] { state.setSelected(false); await state.drain() }
            throw error
        }
        for state in [leftState, rightState] { state.setSelected(false); await state.drain() }
    }

    private func eventually(_ message: String, _ condition: @MainActor () async -> Bool) async throws {
        let deadline = ContinuousClock.now + .seconds(15)
        while !(await condition()), ContinuousClock.now < deadline {
            try await Task.sleep(for: .milliseconds(25))
        }
        guard await condition() else {
            XCTFail(message)
            throw CancellationError()
        }
    }
}
