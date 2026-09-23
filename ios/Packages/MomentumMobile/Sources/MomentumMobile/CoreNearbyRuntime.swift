// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import MomentumKit

public struct NearbySnapshot: Sendable {
    public let linked: [NearbyDevice]
    public let discovered: [NearbyDevice]
    public let status: SyncStatus

    public init(linked: [NearbyDevice], discovered: [NearbyDevice], status: SyncStatus) {
        self.linked = linked
        self.discovered = discovered
        self.status = status
    }
}

/// A separate serial executor keeps listener setup/teardown away from the main
/// actor and the task-editing actor. All transport rules remain in Rust.
public actor CoreNearbyRuntime: NearbyRuntime {
    private let engine: Engine
    private let keychain: Keychain
    private let deviceName: String
    private let demoKeyDirectory: String?
    private var bridge: NearbyEventBridge?

    init(engine: Engine, keychain: Keychain, deviceName: String, demoKeyDirectory: String? = nil) {
        self.engine = engine; self.keychain = keychain
        self.deviceName = deviceName; self.demoKeyDirectory = demoKeyDirectory
    }
    public func start(events: @escaping @Sendable (P2pEvent) async -> Void) throws -> P2pInfo {
        guard bridge == nil, !engine.p2pRunning() else { throw CoreError.Busy }
        let events = NearbyEventBridge(events: events)
        let secrets = KeychainSecretStore(keychain: keychain, accessibility: .afterFirstUnlockThisDeviceOnly)
        bridge = events
        do {
            return try engine.p2pStart(delegate: events, secrets: secrets, deviceName: deviceName,
                                       demoKeyDir: demoKeyDirectory)
        } catch {
            events.finish()
            bridge = nil
            if let failure = secrets.failure { throw failure }
            throw error
        }
    }
    public func stop() {
        guard let bridge else { return }
        engine.p2pStop()
        bridge.finish()
        self.bridge = nil
    }
    deinit {
        guard let bridge else { return }
        bridge.finish()
        let engine = engine
        Task.detached(priority: .utility) { engine.p2pStop() }
    }
    public func snapshot() -> NearbySnapshot {
        NearbySnapshot(linked: engine.p2pDevices(), discovered: engine.p2pDiscovered(), status: engine.syncStatus())
    }
    public func beginPairing() -> String? { engine.p2pBeginPairing() }
    public func endPairing() { engine.p2pEndPairing() }
    public func link(address: String, code: String) throws { try engine.p2pLink(address: address, code: code) }
    public func unlink(id: String) { engine.p2pUnlink(deviceId: id) }
    public func syncNow() throws -> UInt64 { try engine.p2pSyncNow() }
}


extension EngineWorker {
    /// One runtime owner per store; later requests share its original configuration.
    public func nearbyRuntime(keychain: Keychain, deviceName: String) -> CoreNearbyRuntime {
        if let nearbyTransport { return nearbyTransport }
        let runtime = CoreNearbyRuntime(engine: engine, keychain: keychain, deviceName: deviceName)
        nearbyTransport = runtime
        return runtime
    }
}

/// Preserve Rust callback order without blocking its network threads on the main
/// actor. Session checks in NearbyLifecycle reject events already queued at stop.
private final class NearbyEventBridge: P2pDelegate {
    private let continuation: AsyncStream<P2pEvent>.Continuation
    private let consumer: Task<Void, Never>
    init(events: @escaping @Sendable (P2pEvent) async -> Void) {
        let (stream, continuation) = AsyncStream<P2pEvent>.makeStream()
        self.continuation = continuation
        consumer = Task.detached(priority: .utility) {
            for await event in stream {
                guard !Task.isCancelled else { break }
                await events(event)
            }
        }
    }
    func onEvent(event: P2pEvent) { continuation.yield(event) }
    func finish() { continuation.finish(); consumer.cancel() }
    deinit { continuation.finish(); consumer.cancel() }
}
