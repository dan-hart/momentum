// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import MomentumKit
import Observation

public protocol NearbyRuntime: Sendable {
    func start(events: @escaping @Sendable (P2pEvent) async -> Void) async throws -> P2pInfo
    func stop() async
    func snapshot() async -> NearbySnapshot
    func beginPairing() async -> String?
    func endPairing() async
    func link(address: String, code: String) async throws
    func unlink(id: String) async
    func syncNow() async throws -> UInt64
}

public enum ProviderConnectionTestResult: Equatable, Sendable {
    case success
    case failure(String)
}

/// Serial foreground ownership of one core runtime. This controller never owns
/// task rules, timers for sync, discovery or a second task store.
@MainActor @Observable public final class NearbyLifecycle {
    public enum Phase: Equatable, Sendable { case stopped, starting, running, stopping, failed }
    public private(set) var phase: Phase = .stopped
    public private(set) var info: P2pInfo?
    public private(set) var pairingCode: String?
    public private(set) var linked: [NearbyDevice] = []
    public private(set) var discovered: [NearbyDevice] = []
    public private(set) var status: SyncStatus?
    public private(set) var isSyncing = false
    public private(set) var transportError: String?
    public enum Failure: Equatable, Sendable { case credentials, transport }
    public private(set) var failure: Failure?
    public let allowed: Bool
    @ObservationIgnored public var onEvent: (@MainActor (P2pEvent) -> Void)?
    @ObservationIgnored public var didStop: (@MainActor () -> Void)?
    @ObservationIgnored private let runtime: any NearbyRuntime
    @ObservationIgnored private var selected = false
    @ObservationIgnored private var foreground = false
    @ObservationIgnored private var suspended = false
    @ObservationIgnored private var running = false
    @ObservationIgnored private var revision = 0
    @ObservationIgnored private var session: UUID?
    @ObservationIgnored private var active: Task<Void, Never>?
    @ObservationIgnored private let sleep: @Sendable (Duration) async throws -> Void
    @ObservationIgnored private var connectionResults: [UInt64: ProviderConnectionTestResult] = [:]
    @ObservationIgnored private var connectionWaiters: [UInt64: CheckedContinuation<ProviderConnectionTestResult, Never>] = [:]
    @ObservationIgnored private var connectionTimeouts: [UInt64: Task<Void, Never>] = [:]

    public init(runtime: any NearbyRuntime, allowed: Bool,
                sleep: @escaping @Sendable (Duration) async throws -> Void = { try await Task.sleep(for: $0) }) {
        self.runtime = runtime
        self.allowed = allowed
        self.sleep = sleep
    }
    public func setSelected(_ selected: Bool) {
        guard self.selected != selected else { return }
        self.selected = selected
        changed()
    }
    public func setForeground(_ foreground: Bool) {
        guard self.foreground != foreground else { return }
        self.foreground = foreground
        changed()
    }
    public func suspend() async {
        suspended = true
        changed()
        await drain()
    }
    public func resume() {
        guard suspended else { return }
        suspended = false
        changed()
    }
    public func retry() { if phase == .failed { changed() } }
    public func drain() async { await active?.value }

    @discardableResult public func beginPairing() async -> String? {
        guard phase == .running else { return nil }
        pairingCode = await runtime.beginPairing()
        await refresh()
        return pairingCode
    }
    public func endPairing() async {
        await runtime.endPairing()
        pairingCode = nil
        await refresh()
    }
    public func link(address: String, code: String) async throws {
        do {
            try await runtime.link(address: address, code: code)
            failure = nil
            transportError = nil
            await refresh()
        } catch {
            failure = error is KeychainFailure ? .credentials : .transport
            transportError = String(describing: error)
            throw error
        }
    }
    public func unlink(id: String) async {
        await runtime.unlink(id: id)
        await refresh()
    }
    public func syncNow() async {
        guard phase == .running, !isSyncing else { return }
        do {
            _ = try await runtime.syncNow()
        } catch {
            failure = .transport
            transportError = String(describing: error)
        }
    }
    public func testConnection() async -> ProviderConnectionTestResult {
        guard allowed, phase == .running, !linked.isEmpty else {
            return .failure("Link a nearby device before testing the connection.")
        }
        do {
            let cycleID = try await runtime.syncNow()
            if let result = connectionResults.removeValue(forKey: cycleID) { return result }
            return await withTaskCancellationHandler {
                await withCheckedContinuation { continuation in
                    connectionWaiters[cycleID] = continuation
                    connectionTimeouts[cycleID] = Task { [weak self, sleep] in
                        do { try await sleep(.seconds(30)) } catch { return }
                        guard !Task.isCancelled else { return }
                        self?.finishConnectionTest(
                            cycleID,
                            with: .failure("The connection test timed out. Try again when both devices are available.")
                        )
                    }
                }
            } onCancel: { [weak self] in
                Task { @MainActor in
                    self?.finishConnectionTest(cycleID, with: .failure("The connection test was cancelled."))
                }
            }
        } catch is CancellationError {
            return .failure("The connection test was cancelled.")
        } catch {
            return .failure(String(describing: error))
        }
    }
    public func refresh() async {
        guard running else {
            linked = []
            discovered = []
            status = nil
            return
        }
        let snapshot = await runtime.snapshot()
        linked = snapshot.linked
        discovered = snapshot.discovered
        status = snapshot.status
    }

    private var wanted: Bool { allowed && selected && foreground && !suspended }

    private func changed() {
        revision += 1
        // Revoke old callbacks immediately, even when a blocking FFI start/stop
        // has not returned yet. A later foreground request must use a new session.
        if !wanted {
            session = nil
            info = nil
            pairingCode = nil
            isSyncing = false
            finishAllConnectionTests(with: .failure("The connection test was cancelled."))
        }
        guard active == nil else { return }
        active = Task { await reconcile() }
    }

    private func reconcile() async {
        while true {
            if running && (!wanted || session == nil) {
                phase = .stopping
                session = nil
                info = nil
                await runtime.stop()
                running = false
                phase = .stopped
                linked = []
                discovered = []
                status = nil
                // A commit may beat cancellation while its callback is queued.
                // Refresh the owner after the core's complete stop/commit barrier.
                didStop?()
            } else if wanted && !running {
                phase = .starting
                let attempt = revision
                let id = UUID()
                session = id
                do {
                    let started = try await runtime.start { [weak self] event in
                        await self?.receive(event, session: id)
                    }
                    running = true
                    if wanted && session == id {
                        info = started
                        phase = .running
                        failure = nil
                        transportError = nil
                        await refresh()
                    }
                } catch {
                    session = nil
                    info = nil
                    failure = error is KeychainFailure ? .credentials : .transport
                    transportError = String(describing: error)
                    phase = wanted ? .failed : .stopped
                    // Retry only a newer request, never spin on a failed start.
                    if attempt == revision || !wanted { break }
                }
            } else {
                if !wanted { phase = .stopped }
                break
            }
        }
        active = nil
    }

    private func receive(_ event: P2pEvent, session id: UUID) async {
        guard session == id, wanted else { return }
        switch event {
        case .started(let port):
            guard var updated = info else { break }
            updated.port = port
            info = updated
        case .syncStarted:
            isSyncing = true
            transportError = nil
        case .syncCompleted(let cycleID, let error):
            isSyncing = false
            transportError = error
            if let cycleID {
                let result: ProviderConnectionTestResult = error.map { .failure($0) } ?? .success
                if connectionWaiters[cycleID] != nil {
                    finishConnectionTest(cycleID, with: result)
                } else {
                    connectionResults[cycleID] = result
                    if connectionResults.count > 8 {
                        connectionResults.removeValue(forKey: connectionResults.keys.min()!)
                    }
                }
            }
            await refresh()
        case .storeChanged, .statusChanged, .devicesChanged, .discoveryUpdated:
            await refresh()
        case .notice:
            break
        }
        onEvent?(event)
    }

    private func finishConnectionTest(_ cycleID: UInt64, with result: ProviderConnectionTestResult) {
        connectionTimeouts.removeValue(forKey: cycleID)?.cancel()
        connectionWaiters.removeValue(forKey: cycleID)?.resume(returning: result)
        connectionResults.removeValue(forKey: cycleID)
    }

    private func finishAllConnectionTests(with result: ProviderConnectionTestResult) {
        let ids = Array(connectionWaiters.keys)
        ids.forEach { finishConnectionTest($0, with: result) }
        connectionResults.removeAll()
    }
}
