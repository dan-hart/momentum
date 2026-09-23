// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import MomentumKit
import Testing
@testable import MomentumMobile

@Suite @MainActor struct NextcloudSyncStateTests {
    @Test func connectionTestUsesSavedConfigurationAndPublishesSuccess() async {
        let fixture = SyncFixture()
        defer { fixture.clean() }
        await fixture.ready()
        #expect(fixture.state.canTestConnection)

        let testing = Task { await fixture.state.testConnection() }
        await fixture.connectionTest.waitUntilRunning()
        #expect(fixture.state.connectionTest == .testing)
        fixture.connectionTest.finish(.success(()))
        await testing.value

        #expect(fixture.state.connectionTest == .success)
        #expect(await fixture.connectionTestFactory.count == 1)
        fixture.state.draft.password = "unsaved"
        #expect(!fixture.state.canTestConnection)
        await fixture.state.select(.off)
        #expect(fixture.state.connectionTest == .idle)
    }

    @Test func connectionTestFailureAndBackgroundCancellationAreNotSyncFailures() async {
        let fixture = SyncFixture()
        defer { fixture.clean() }
        await fixture.ready()
        let failing = Task { await fixture.state.testConnection() }
        await fixture.connectionTest.waitUntilRunning()
        fixture.connectionTest.finish(.failure(CoreError.Transient(message: "offline fixture")))
        await failing.value
        guard case .failure(let message) = fixture.state.connectionTest else {
            Issue.record("Expected connection-test failure")
            return
        }
        #expect(message.contains("offline fixture"))
        #expect(fixture.state.failure == nil)

        let cancelled = Task { await fixture.state.testConnection() }
        await fixture.connectionTest.waitForRunCount(2)
        fixture.state.setForeground(false)
        await fixture.connectionTest.waitUntilCancelled()
        fixture.connectionTest.finish(.failure(CancellationError()))
        await cancelled.value
        #expect(fixture.state.connectionTest == .idle)
        #expect(fixture.state.failure == nil)
    }

    @Test func foregroundReturnRetriesImmediatelyAfterCancelledExchangeDrains() async {
        let fixture = SyncFixture(automatic: true)
        defer { fixture.clean() }
        await fixture.ready()
        await fixture.clock.waitForCount(1)
        await fixture.clock.advanceLatest()
        await fixture.operation.waitUntilRunning()
        fixture.state.setForeground(false)
        fixture.state.setForeground(true)
        fixture.operation.finish(.failure(CancellationError()))
        await fixture.state.drain()
        await fixture.clock.waitForCount(2)
        #expect(await fixture.clock.lastDelay == .zero)
    }

    @Test func automaticEditsCoalesceAndBackgroundCancelsScheduledWork() async {
        let fixture = SyncFixture(automatic: true)
        defer { fixture.clean() }
        await fixture.ready()
        await fixture.clock.waitForCount(1)
        #expect(await fixture.clock.lastDelay == .zero)
        fixture.state.localChanges()
        await fixture.clock.waitForCount(2)
        fixture.state.localChanges()
        await fixture.clock.waitForCount(3)
        #expect(await fixture.clock.lastDelay == .seconds(20))
        await fixture.clock.advanceLatest()
        await fixture.operation.waitUntilRunning()
        fixture.state.localChanges()
        fixture.operation.finish(.success(SyncReport(downloaded: false, uploaded: true, opsUploaded: 1)))
        await fixture.state.drain()
        await fixture.clock.waitForCount(4)
        #expect(await fixture.clock.lastDelay == .seconds(20))
        fixture.state.setForeground(false)
        await fixture.clock.advanceLatest()
        await fixture.state.drain()
        #expect(await fixture.factory.count == 1)
    }

    @Test func lowPowerModePausesAndThenResumesAutomaticSync() async {
        let fixture = SyncFixture(automatic: true)
        defer { fixture.clean() }
        await fixture.ready()
        await fixture.clock.waitForCount(1)

        fixture.state.setLowPowerMode(true)
        await fixture.clock.advanceLatest()
        await fixture.state.drain()
        #expect(fixture.state.lowPowerMode)
        #expect(await fixture.factory.count == 0)

        fixture.state.setLowPowerMode(false)
        await fixture.clock.waitForCount(2)
        #expect(await fixture.clock.lastDelay == .zero)
    }

    @Test func lowPowerModeKeepsUserInitiatedSyncAvailable() async {
        let fixture = SyncFixture(automatic: true)
        defer { fixture.clean() }
        fixture.state.setLowPowerMode(true)
        await fixture.ready()
        #expect(fixture.state.canSync)

        let syncing = Task { await fixture.state.syncNow() }
        await fixture.operation.waitUntilRunning()
        fixture.operation.finish(.success(SyncReport(downloaded: false, uploaded: false, opsUploaded: 0)))
        await syncing.value

        #expect(await fixture.factory.count == 1)
        #expect(fixture.state.failure == nil)
    }

    @Test func savingWaitsForTheOldConnectionToDrain() async {
        let fixture = SyncFixture()
        defer { fixture.clean() }
        await fixture.ready()
        let syncing = Task { await fixture.state.syncNow() }
        await fixture.operation.waitUntilRunning()
        fixture.state.draft.password = "replacement"
        let saving = Task { await fixture.state.save() }
        await fixture.operation.waitUntilCancelled()
        #expect(await fixture.store.saves == 0)
        #expect(fixture.state.saving)
        fixture.operation.finish(.failure(CancellationError()))
        #expect(await saving.value)
        await syncing.value
        #expect(await fixture.store.value?.password == "replacement")
        #expect(!fixture.state.saving)
    }

    @Test func freshInstallAndIsolatedModeNeverStartTransport() async throws {
        let fixture = SyncFixture()
        defer { fixture.clean() }
        await fixture.state.load()
        fixture.state.setForeground(true)
        await fixture.state.syncNow()
        #expect(fixture.state.provider == .off)
        #expect(await fixture.factory.count == 0)
        let isolated = fixture.makeState(allowed: false)
        await isolated.load()
        await isolated.select(.nextcloud)
        isolated.setForeground(true)
        await isolated.syncNow()
        #expect(isolated.provider == .off)
        #expect(await fixture.factory.count == 0)
    }

    @Test func unreadableCredentialsCannotBeReplacedWithAnEmptyDraft() async {
        let fixture = SyncFixture()
        defer { fixture.clean() }
        await fixture.store.failLoad(true)
        await fixture.state.load()
        #expect(fixture.state.failure == .credentialsRead)
        #expect(!fixture.state.loaded)
        #expect(await fixture.state.save() == false)
        #expect(await fixture.store.saves == 0)
        await fixture.store.failLoad(false)
        await fixture.state.load()
        #expect(fixture.state.loaded)
        #expect(fixture.state.draft == fixture.connection)
    }

    @Test func failedSaveRetainsDraftAndSavedConnectionWithoutEnablingSync() async {
        let fixture = SyncFixture()
        defer { fixture.clean() }
        await fixture.state.load()
        fixture.state.draft.password = "changed secret"
        await fixture.store.failSave(true)
        #expect(await fixture.state.save() == false)
        #expect(fixture.state.draft.password == "changed secret")
        #expect(fixture.state.hasUnsavedChanges)
        #expect(fixture.state.failure == .credentialsWrite)
        #expect(fixture.state.provider == .off)
        #expect(await fixture.store.value == fixture.connection)
        await fixture.store.failSave(false)
        #expect(await fixture.state.save())
        #expect(!fixture.state.hasUnsavedChanges)
        #expect(fixture.state.provider == .off)
    }

    @Test func invalidDraftDoesNotReplaceSavedConnection() async {
        let fixture = SyncFixture()
        defer { fixture.clean() }
        await fixture.state.load()
        fixture.state.draft.serverURL = "http://cloud.example.test"

        #expect(await fixture.state.save() == false)
        #expect(fixture.state.configurationIssue == .secureServerRequired)
        #expect(fixture.state.failure == .configuration)
        #expect(await fixture.store.saves == 0)
        #expect(await fixture.store.value == fixture.connection)
        #expect(fixture.state.hasUnsavedChanges)
    }

    @Test func invalidSavedConnectionNeverStartsTransport() async {
        let fixture = SyncFixture(automatic: true)
        defer { fixture.clean() }
        var invalid = fixture.connection
        invalid.serverURL = "http://cloud.example.test"
        await fixture.store.replace(invalid)

        await fixture.state.load()
        fixture.state.setForeground(true)
        await fixture.state.select(.nextcloud)

        #expect(fixture.state.failure == .configuration)
        #expect(fixture.state.configurationIssue == .secureServerRequired)
        #expect(fixture.state.canSync == false)
        #expect(await fixture.factory.count == 0)
    }

    @Test func validDraftIsNormalizedBeforePersistenceAndSync() async {
        let fixture = SyncFixture()
        defer { fixture.clean() }
        await fixture.state.load()
        fixture.state.draft.serverURL = " https://cloud.example.test/nextcloud/ "
        fixture.state.draft.userName = " fixture-user "
        fixture.state.draft.folder = " /Momentum/ "

        #expect(await fixture.state.save())
        #expect(fixture.state.draft.serverURL == "https://cloud.example.test/nextcloud")
        #expect(fixture.state.draft.userName == "fixture-user")
        #expect(fixture.state.draft.folder == "Momentum")
        #expect(await fixture.store.value == fixture.state.draft)
        #expect(fixture.state.configurationIssue == nil)
        #expect(fixture.state.failure == nil)
        #expect(!fixture.state.hasUnsavedChanges)
    }

    @Test func offWaitsForDrainAndPreservesSavedConnection() async {
        let fixture = SyncFixture()
        defer { fixture.clean() }
        await fixture.ready()
        let syncing = Task { await fixture.state.syncNow() }
        await fixture.operation.waitUntilRunning()
        let off = Task { await fixture.state.select(.off) }
        await fixture.operation.waitUntilCancelled()
        #expect(fixture.state.provider == .off)
        #expect(fixture.state.isSyncing)
        await fixture.state.syncNow()
        #expect(await fixture.factory.count == 1)
        fixture.operation.finish(.failure(CancellationError()))
        await off.value
        await syncing.value
        #expect(!fixture.state.isSyncing)
        #expect(fixture.state.failure == nil)
        #expect(await fixture.store.value == fixture.connection)
    }

    @Test func commitThatWinsCancellationStillRefreshesTheApp() async {
        let fixture = SyncFixture()
        defer { fixture.clean() }
        var leases = 0
        fixture.state.beginExecution = { leases += 1; return { leases -= 1 } }
        var commits = 0
        fixture.state.didCommit = { commits += 1 }
        await fixture.ready()
        let syncing = Task { await fixture.state.syncNow() }
        await fixture.operation.waitUntilRunning()
        fixture.state.setForeground(false)
        fixture.operation.finish(.success(SyncReport(downloaded: true, uploaded: true, opsUploaded: 1)))
        await syncing.value
        #expect(commits == 1)
        #expect(leases == 0)
        #expect(fixture.state.failure == nil)
        #expect(!fixture.state.isSyncing)
    }

    @Test func cancellationDuringFactoryAdmissionPreventsRun() async {
        let fixture = SyncFixture()
        defer { fixture.clean() }
        await fixture.factory.pause()
        await fixture.ready()
        let syncing = Task { await fixture.state.syncNow() }
        await fixture.factory.waitUntilEntered()
        fixture.state.setForeground(false)
        await fixture.factory.release()
        await syncing.value
        #expect(fixture.operation.runs == 0)
        #expect(fixture.operation.cancellations == 1)
        #expect(fixture.state.failure == nil)
    }

    @Test func restoreHoldsAdmissionUntilExplicitlyResumed() async {
        let fixture = SyncFixture()
        defer { fixture.clean() }
        await fixture.ready()
        let syncing = Task { await fixture.state.syncNow() }
        await fixture.operation.waitUntilRunning()
        let suspension = Task { await fixture.state.suspendForRestore() }
        await fixture.operation.waitUntilCancelled()
        fixture.operation.finish(.failure(CancellationError()))
        await suspension.value
        await syncing.value
        await fixture.state.syncNow()
        #expect(await fixture.factory.count == 1)
        fixture.state.resumeAfterRestore()
        #expect(!fixture.state.isSuspended)
    }

    @Test func failedExchangePersistsRecoveryAndNeverReportsSuccess() async {
        let fixture = SyncFixture()
        defer { fixture.clean() }
        await fixture.ready()
        var commits = 0
        fixture.state.didCommit = { commits += 1 }
        let syncing = Task { await fixture.state.syncNow() }
        await fixture.operation.waitUntilRunning()
        fixture.operation.finish(.failure(CoreError.Transient(message: "offline fixture")))
        await syncing.value
        #expect(fixture.state.failure == .network)
        #expect(commits == 0)
        #expect(fixture.makeState().failure == .network)
    }

    @Test func providerSelectionSerializesNextcloudAndLibreSyncOwnership() async {
        let fixture = SyncFixture()
        defer { fixture.clean() }
        let runtime = SyncNearbyProbe()
        let nearby = NearbyLifecycle(runtime: runtime, allowed: true)
        fixture.state.connectNearby(nearby)
        await fixture.state.load()
        fixture.state.setForeground(true)

        await fixture.state.select(.libresync)
        #expect(fixture.state.provider == .libresync)
        #expect(nearby.phase == .running)
        #expect(await runtime.starts == 1)
        #expect(fixture.defaults.string(forKey: PrefKey.syncMethod) == SyncMethod.libresync.rawValue)

        await fixture.state.select(.nextcloud)
        #expect(nearby.phase == .stopped)
        #expect(await runtime.stops == 1)
        #expect(fixture.state.provider == .nextcloud)
    }

    @Test func nearbyEditsCoalesceIntoOneDiscretionaryExchange() async {
        let fixture = SyncFixture()
        defer { fixture.clean() }
        let runtime = SyncNearbyProbe()
        let nearby = NearbyLifecycle(runtime: runtime, allowed: true)
        fixture.state.connectNearby(nearby)
        await fixture.state.load()
        fixture.state.setForeground(true)
        await fixture.state.select(.libresync)

        fixture.state.localChanges()
        await fixture.clock.waitForCount(1)
        fixture.state.localChanges()
        await fixture.clock.waitForCount(2)
        #expect(await fixture.clock.lastDelay == .seconds(20))

        await fixture.clock.advanceLatest()
        await runtime.waitForSyncCount(1)
        #expect(await runtime.syncs == 1)
    }
}

@MainActor private final class SyncFixture {
    let name = "momentum-sync-state-\(UUID())"
    let defaults: UserDefaults
    let store: ConnectionMemory
    let operation = ControlledSyncOperation()
    let connectionTest = ControlledConnectionTestOperation()
    let clock = ControlledDelay()
    let factory: SyncFactory
    let connectionTestFactory: ConnectionTestFactory
    let connection: NextcloudConnection
    lazy var state = makeState()
    init(automatic: Bool = false) {
        defaults = UserDefaults(suiteName: name)!
        var connection = NextcloudConnection()
        connection.serverURL = "https://fixture.invalid"
        connection.userName = "fixture"
        connection.password = "fixture-only"
        connection.automatic = automatic
        self.connection = connection
        store = ConnectionMemory(connection)
        factory = SyncFactory(operation: operation)
        connectionTestFactory = ConnectionTestFactory(operation: connectionTest)
    }
    func makeState(allowed: Bool = true) -> NextcloudSyncState {
        let state = NextcloudSyncState(defaults: defaults, persistence: store, allowed: allowed, sleep: { [clock] in try await clock.sleep($0) })
        state.connect(makeOperation: { [factory] _ in await factory.make() },
                      makeConnectionTest: { [connectionTestFactory] _ in await connectionTestFactory.make() },
                      status: {
            SyncStatus(syncing: false, pendingOps: 1, lastNextcloudMs: 0,
                       lastNearbyMs: 0, nearbyRunning: false, linkedDevices: 0)
        })
        return state
    }
    func ready() async {
        await state.load()
        state.setForeground(true)
        await state.select(.nextcloud)
    }
    func clean() { state.setForeground(false); defaults.removePersistentDomain(forName: name) }
}

private final class ControlledConnectionTestOperation: NextcloudConnectionTestOperation, @unchecked Sendable {
    private let lock = NSLock()
    private var result: CheckedContinuation<Void, Error>?
    private var runWaiters: [(Int, CheckedContinuation<Void, Never>)] = []
    private var cancelled: CheckedContinuation<Void, Never>?
    private var runCount = 0
    private var cancelCount = 0
    func run() async throws {
        try await withCheckedThrowingContinuation { continuation in
            lock.withLock {
                result = continuation
                runCount += 1
                let ready = runWaiters.filter { runCount >= $0.0 }
                runWaiters.removeAll { runCount >= $0.0 }
                ready.forEach { $0.1.resume() }
            }
        }
    }
    func cancel() -> Bool {
        lock.withLock {
            cancelCount += 1
            cancelled?.resume()
            cancelled = nil
            return true
        }
    }
    func waitUntilRunning() async { await waitForRunCount(1) }
    func waitForRunCount(_ count: Int) async {
        await withCheckedContinuation { continuation in
            lock.withLock {
                if runCount >= count { continuation.resume() }
                else { runWaiters.append((count, continuation)) }
            }
        }
    }
    func waitUntilCancelled() async {
        await withCheckedContinuation { continuation in
            lock.withLock {
                if cancelCount > 0 { continuation.resume() }
                else { cancelled = continuation }
            }
        }
    }
    func finish(_ outcome: Result<Void, Error>) {
        lock.withLock { result?.resume(with: outcome); result = nil }
    }
}

private actor ConnectionTestFactory {
    let operation: ControlledConnectionTestOperation
    var count = 0
    init(operation: ControlledConnectionTestOperation) { self.operation = operation }
    func make() -> ControlledConnectionTestOperation { count += 1; return operation }
}

private actor ConnectionMemory: NextcloudConnectionPersistence {
    var value: NextcloudConnection?
    var saves = 0
    var loadFailure = false
    var saveFailure = false
    init(_ value: NextcloudConnection?) { self.value = value }
    func replace(_ value: NextcloudConnection?) { self.value = value }
    func failLoad(_ value: Bool) { loadFailure = value }
    func failSave(_ value: Bool) { saveFailure = value }
    func load() throws -> NextcloudConnection? {
        if loadFailure { throw CocoaError(.fileReadNoPermission) }
        return value
    }
    func save(_ value: NextcloudConnection) throws {
        saves += 1
        if saveFailure { throw CocoaError(.fileWriteNoPermission) }
        self.value = value
    }
}

/// The transport boundary is controlled explicitly; there are no wall-clock sleeps.
private final class ControlledSyncOperation: NextcloudRunningOperation, @unchecked Sendable {
    private let lock = NSLock()
    private var result: CheckedContinuation<SyncReport, Error>?
    private var started: CheckedContinuation<Void, Never>?
    private var cancelled: CheckedContinuation<Void, Never>?
    private var runCount = 0
    private var cancelCount = 0
    var runs: Int { lock.withLock { runCount } }
    var cancellations: Int { lock.withLock { cancelCount } }
    func run() async throws -> SyncReport {
        try await withCheckedThrowingContinuation { continuation in
            lock.withLock { result = continuation; runCount += 1; started?.resume(); started = nil }
        }
    }
    func cancel() -> Bool {
        lock.withLock { cancelCount += 1; cancelled?.resume(); cancelled = nil; return true }
    }
    func waitUntilRunning() async {
        await withCheckedContinuation { continuation in
            lock.withLock { if runCount > 0 { continuation.resume() } else { started = continuation } }
        }
    }
    func waitUntilCancelled() async {
        await withCheckedContinuation { continuation in
            lock.withLock { if cancelCount > 0 { continuation.resume() } else { cancelled = continuation } }
        }
    }
    func finish(_ outcome: Result<SyncReport, Error>) {
        lock.withLock { result?.resume(with: outcome); result = nil }
    }
}

private actor SyncFactory {
    let operation: ControlledSyncOperation
    var count = 0
    private var paused = false
    private var gate: CheckedContinuation<Void, Never>?
    private var entered: CheckedContinuation<Void, Never>?
    init(operation: ControlledSyncOperation) { self.operation = operation }
    func pause() { paused = true }
    func make() async -> ControlledSyncOperation {
        count += 1
        entered?.resume(); entered = nil
        if paused { await withCheckedContinuation { gate = $0 } }
        return operation
    }
    func waitUntilEntered() async {
        if count == 0 { await withCheckedContinuation { entered = $0 } }
    }
    func release() { paused = false; gate?.resume(); gate = nil }
}

private actor ControlledDelay {
    var delays: [Duration] = []
    var lastDelay: Duration? { delays.last }
    private var pending: [UUID: CheckedContinuation<Void, Error>] = [:]
    private var latest: UUID?
    private var countWaiter: (Int, CheckedContinuation<Void, Never>)?
    func sleep(_ duration: Duration) async throws {
        let id = UUID()
        try await withTaskCancellationHandler {
            try Task.checkCancellation()
            try await withCheckedThrowingContinuation { continuation in
                pending[id] = continuation
                latest = id
                delays.append(duration)
                if let (target, waiter) = countWaiter, delays.count >= target {
                    countWaiter = nil; waiter.resume()
                }
            }
        } onCancel: { Task { await self.cancel(id) } }
    }
    func waitForCount(_ count: Int) async {
        if delays.count < count { await withCheckedContinuation { countWaiter = (count, $0) } }
    }
    func advanceLatest() { if let latest { pending.removeValue(forKey: latest)?.resume() } }
    private func cancel(_ id: UUID) { pending.removeValue(forKey: id)?.resume(throwing: CancellationError()) }
}

private actor SyncNearbyProbe: NearbyRuntime {
    var starts = 0
    var stops = 0
    var syncs = 0
    private var syncWaiter: (Int, CheckedContinuation<Void, Never>)?
    func start(events: @escaping @Sendable (P2pEvent) async -> Void) -> P2pInfo {
        starts += 1
        return P2pInfo(deviceName: "Fixture", deviceId: "fixture", port: 12345)
    }
    func stop() { stops += 1 }
    func snapshot() -> NearbySnapshot {
        NearbySnapshot(linked: [], discovered: [],
                       status: SyncStatus(syncing: false, pendingOps: 0, lastNextcloudMs: 0,
                                          lastNearbyMs: 0, nearbyRunning: starts > stops, linkedDevices: 0))
    }
    func beginPairing() -> String? { "123456" }
    func endPairing() {}
    func link(address: String, code: String) throws {}
    func unlink(id: String) {}
    func syncNow() -> UInt64 {
        syncs += 1
        if let (target, waiter) = syncWaiter, syncs >= target {
            syncWaiter = nil
            waiter.resume()
        }
        return UInt64(syncs)
    }
    func waitForSyncCount(_ count: Int) async {
        if syncs < count { await withCheckedContinuation { syncWaiter = (count, $0) } }
    }
}
