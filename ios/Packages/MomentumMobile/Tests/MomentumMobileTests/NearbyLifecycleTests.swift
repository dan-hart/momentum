// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import Testing
@testable import MomentumMobile

@Suite @MainActor struct NearbyLifecycleTests {
    @Test func connectionTestCapturesCompletionThatWinsTheCallReturnRace() async throws {
        let runtime = NearbyRuntimeProbe()
        let state = NearbyLifecycle(runtime: runtime, allowed: true)
        state.setSelected(true)
        state.setForeground(true)
        await state.drain()
        try await state.link(address: "127.0.0.1:12346", code: "654321")
        await runtime.configureConnectionTest(cycleID: 17, completeBeforeReturn: true)

        let result = await state.testConnection()

        #expect(result == .success)
        #expect(await runtime.syncs == 1)
    }

    @Test func connectionTestIgnoresUnattributedAndOtherCycleEvents() async throws {
        let runtime = NearbyRuntimeProbe()
        let state = NearbyLifecycle(runtime: runtime, allowed: true)
        state.setSelected(true)
        state.setForeground(true)
        await state.drain()
        try await state.link(address: "127.0.0.1:12346", code: "654321")
        await runtime.configureConnectionTest(cycleID: 23, completeBeforeReturn: false)

        let testing = Task { await state.testConnection() }
        await runtime.waitForSyncCount(1)
        await runtime.emit(.syncCompleted(cycleId: nil, error: "unrelated"), session: 0)
        await runtime.emit(.syncCompleted(cycleId: 22, error: nil), session: 0)
        #expect(state.isSyncing == false)
        await runtime.emit(.syncCompleted(cycleId: 23, error: nil), session: 0)

        #expect(await testing.value == .success)
    }

    @Test func onlyTheSelectedForegroundProviderStarts() async {
        let runtime = NearbyRuntimeProbe()
        let state = NearbyLifecycle(runtime: runtime, allowed: true)
        state.setForeground(true)
        await state.drain()
        #expect(await runtime.starts == 0)
        state.setSelected(true)
        await state.drain()
        #expect(await runtime.starts == 1)
        #expect(state.phase == .running)
        state.setSelected(false)
        await state.drain()
        #expect(await runtime.stops == 1)
        #expect(state.phase == .stopped)
    }

    @Test func previewAndTestsCannotStartEvenWithSavedSelection() async {
        let runtime = NearbyRuntimeProbe()
        let state = NearbyLifecycle(runtime: runtime, allowed: false)
        state.setSelected(true)
        state.setForeground(true)
        state.retry()
        await state.drain()
        #expect(await runtime.starts == 0)
        #expect(state.info == nil)
    }
    @Test func backgroundDuringStartupDrainsTheLateListener() async {
        let gate = NearbyGate()
        let runtime = NearbyRuntimeProbe(startGate: gate)
        let state = NearbyLifecycle(runtime: runtime, allowed: true)
        state.setSelected(true)
        state.setForeground(true)
        await gate.waitUntilEntered()
        state.setForeground(false)
        await gate.release()
        await state.drain()
        #expect(await runtime.starts == 1)
        #expect(await runtime.stops == 1)
        #expect(state.phase == .stopped)
        #expect(state.info == nil)
    }

    @Test func foregroundReturnWaitsForStoppingBeforeRestart() async {
        let gate = NearbyGate()
        let runtime = NearbyRuntimeProbe(stopGate: gate)
        let state = NearbyLifecycle(runtime: runtime, allowed: true)
        state.setSelected(true)
        state.setForeground(true)
        await state.drain()
        state.setForeground(false)
        await gate.waitUntilEntered()
        state.setForeground(true)
        #expect(await runtime.starts == 1)
        await gate.release()
        await state.drain()
        #expect(await runtime.starts == 2)
        #expect(await runtime.stops == 1)
        #expect(state.phase == .running)
    }

    @Test func suspendedRestoreCannotRestartUntilResumed() async {
        let runtime = NearbyRuntimeProbe()
        let state = NearbyLifecycle(runtime: runtime, allowed: true)
        state.setSelected(true)
        state.setForeground(true)
        await state.drain()
        await state.suspend()
        #expect(state.phase == .stopped)
        state.setForeground(false)
        state.setForeground(true)
        await state.drain()
        #expect(await runtime.starts == 1)
        state.resume()
        await state.drain()
        #expect(await runtime.starts == 2)
    }

    @Test func staleEventsCannotPublishIntoANewSession() async {
        let runtime = NearbyRuntimeProbe()
        let state = NearbyLifecycle(runtime: runtime, allowed: true)
        var received: [P2pEvent] = []
        state.onEvent = { received.append($0) }
        state.setSelected(true)
        state.setForeground(true)
        await state.drain()
        state.setSelected(false)
        await state.drain()
        state.setSelected(true)
        await state.drain()
        await runtime.emit(.storeChanged, session: 0)
        await runtime.emit(.statusChanged, session: 1)
        #expect(received == [.statusChanged])
    }

    @Test func failedStartupDoesNotSpinAndCanBeRetried() async {
        let runtime = NearbyRuntimeProbe()
        await runtime.setFailure(true)
        let state = NearbyLifecycle(runtime: runtime, allowed: true)
        state.setSelected(true)
        state.setForeground(true)
        await state.drain()
        #expect(state.phase == .failed)
        #expect(await runtime.starts == 1)
        state.setForeground(true)
        await state.drain()
        #expect(await runtime.starts == 1)
        await runtime.setFailure(false)
        state.retry()
        await state.drain()
        #expect(state.phase == .running)
        #expect(await runtime.starts == 2)
    }

    @Test func pairingLinkingAndUnlinkingRefreshDeviceState() async throws {
        let runtime = NearbyRuntimeProbe()
        let state = NearbyLifecycle(runtime: runtime, allowed: true)
        state.setSelected(true)
        state.setForeground(true)
        await state.drain()

        let code = await state.beginPairing()
        #expect(code == "123456")
        #expect(state.pairingCode == "123456")
        #expect(state.discovered.map(\.name) == ["Fixture peer"])

        try await state.link(address: "127.0.0.1:12346", code: "654321")
        #expect(state.linked.map(\.deviceId) == ["peer"])
        #expect(state.failure == nil)

        await state.unlink(id: "peer")
        #expect(state.linked.isEmpty)
        await state.endPairing()
        #expect(state.pairingCode == nil)
    }

    @Test func transportEventsRefreshStatusAndPublishStoreChanges() async {
        let runtime = NearbyRuntimeProbe()
        let state = NearbyLifecycle(runtime: runtime, allowed: true)
        var storeChanges = 0
        state.onEvent = { if $0 == .storeChanged { storeChanges += 1 } }
        state.setSelected(true)
        state.setForeground(true)
        await state.drain()

        await runtime.setStatus(lastNearbyMs: 42, linkedDevices: 1)
        await runtime.emit(.storeChanged, session: 0)

        #expect(storeChanges == 1)
        #expect(state.status?.lastNearbyMs == 42)
        #expect(state.status?.linkedDevices == 1)
    }
}

private actor NearbyRuntimeProbe: NearbyRuntime {
    var starts = 0
    var stops = 0
    var failing = false
    var sinks: [@Sendable (P2pEvent) async -> Void] = []
    let startGate: NearbyGate?
    let stopGate: NearbyGate?
    var linked: [NearbyDevice] = []
    var discovered = [NearbyDevice(deviceId: "peer", name: "Fixture peer",
                                   address: "127.0.0.1:12346", lastSeenMs: nil, linked: false)]
    var status = SyncStatus(syncing: false, pendingOps: 0, lastNextcloudMs: 0,
                            lastNearbyMs: 0, nearbyRunning: true, linkedDevices: 0)
    var syncs = 0
    var nextCycleID: UInt64 = 1
    var completeBeforeReturn = false
    private var syncWaiter: (Int, CheckedContinuation<Void, Never>)?
    init(startGate: NearbyGate? = nil, stopGate: NearbyGate? = nil) {
        self.startGate = startGate; self.stopGate = stopGate
    }
    func start(events: @escaping @Sendable (P2pEvent) async -> Void) async throws -> P2pInfo {
        starts += 1
        sinks.append(events)
        await startGate?.pause()
        if failing { throw CancellationError() }
        return P2pInfo(deviceName: "Fixture", deviceId: "fixture", port: 12345)
    }
    func stop() async { stops += 1; await stopGate?.pause() }
    func snapshot() -> NearbySnapshot {
        NearbySnapshot(linked: linked, discovered: discovered, status: status)
    }
    func beginPairing() -> String? { "123456" }
    func endPairing() {}
    func link(address: String, code: String) throws {
        linked = [NearbyDevice(deviceId: "peer", name: "Fixture peer", address: address,
                               lastSeenMs: nil, linked: true)]
        discovered = []
        status.linkedDevices = 1
    }
    func unlink(id: String) {
        linked.removeAll { $0.deviceId == id }
        status.linkedDevices = UInt32(linked.count)
    }
    func syncNow() async throws -> UInt64 {
        syncs += 1
        if let (target, waiter) = syncWaiter, syncs >= target {
            syncWaiter = nil
            waiter.resume()
        }
        let cycleID = nextCycleID
        if completeBeforeReturn, let sink = sinks.last {
            await sink(.syncStarted(cycleId: cycleID))
            await sink(.syncCompleted(cycleId: cycleID, error: nil))
        }
        return cycleID
    }
    func configureConnectionTest(cycleID: UInt64, completeBeforeReturn: Bool) {
        nextCycleID = cycleID
        self.completeBeforeReturn = completeBeforeReturn
    }
    func waitForSyncCount(_ count: Int) async {
        if syncs < count { await withCheckedContinuation { syncWaiter = (count, $0) } }
    }
    func setFailure(_ value: Bool) { failing = value }
    func setStatus(lastNearbyMs: UInt64, linkedDevices: UInt32) {
        status.lastNearbyMs = lastNearbyMs
        status.linkedDevices = linkedDevices
    }
    func emit(_ event: P2pEvent, session: Int) async { await sinks[session](event) }
}

private actor NearbyGate {
    private var entered = false
    private var released = false
    private var arrivals: [CheckedContinuation<Void, Never>] = []
    private var pending: CheckedContinuation<Void, Never>?
    func pause() async {
        if released { return }
        await withCheckedContinuation { continuation in
            pending = continuation
            entered = true
            arrivals.forEach { $0.resume() }
            arrivals.removeAll()
        }
    }
    func waitUntilEntered() async {
        if entered { return }
        await withCheckedContinuation { arrivals.append($0) }
    }
    func release() { released = true; pending?.resume(); pending = nil }
}
