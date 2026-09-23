// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import MomentumKit
import Observation

/// Owns presentation and scheduling only. Rust owns transport, conflict resolution,
/// deadlines and the final commit boundary. No polling runs while inactive or Off.
@MainActor @Observable public final class NextcloudSyncState {
    public enum Failure: String, Sendable {
        case credentialsRead, credentialsWrite, configuration, connection, network, busy
    }
    public enum ConnectionTestState: Equatable, Sendable {
        case idle, testing, success, failure(String)
    }
    public var draft = NextcloudConnection() {
        didSet { if oldValue != draft { cancelConnectionTest() } }
    }
    public private(set) var provider: SyncMethod
    public private(set) var loaded = false
    public private(set) var loading = false
    public private(set) var saving = false
    public private(set) var isSyncing = false
    public private(set) var isStopping = false
    public private(set) var isSuspended = false
    public private(set) var lowPowerMode = false
    public private(set) var failure: Failure?
    public private(set) var status: SyncStatus?
    public private(set) var connectionTest: ConnectionTestState = .idle
    public private(set) var nearby: NearbyLifecycle?
    @ObservationIgnored public var didCommit: (@MainActor () -> Void)?
    @ObservationIgnored public var beginExecution: (@MainActor () -> (@MainActor () -> Void))?
    public let allowed: Bool
    public var hasUnsavedChanges: Bool { saved != draft }
    public var canSync: Bool {
        switch provider {
        case .nextcloud: eligible && !isSyncing && !hasUnsavedChanges
        case .libresync: nearby?.phase == .running && nearby?.isSyncing == false
        case .off: false
        }
    }
    public var configurationIssue: NextcloudConnectionIssue? { draft.validationIssue }
    public var canTestConnection: Bool {
        guard allowed, loaded, foreground, !hasUnsavedChanges, connectionTest != .testing else { return false }
        switch provider {
        case .nextcloud: return saved?.validationIssue == nil && makeConnectionTest != nil
        case .libresync: return nearby?.phase == .running && nearby?.linked.isEmpty == false
        case .off: return false
        }
    }

    @ObservationIgnored private let defaults: UserDefaults
    @ObservationIgnored private let sleep: @Sendable (Duration) async throws -> Void
    @ObservationIgnored private let persistence: any NextcloudConnectionPersistence
    @ObservationIgnored private var saved: NextcloudConnection?
    @ObservationIgnored private var foreground = false
    @ObservationIgnored private var transitions = 0
    @ObservationIgnored private var generation = 0
    @ObservationIgnored private var active: Task<Void, Never>?
    @ObservationIgnored private var operation: (any NextcloudRunningOperation)?
    @ObservationIgnored private var scheduled: Task<Void, Never>?
    @ObservationIgnored private var editsDuringSync = false
    @ObservationIgnored private var foregroundRetry = false
    @ObservationIgnored private var makeOperation: (@Sendable (NextcloudSettings) async -> any NextcloudRunningOperation)?
    @ObservationIgnored private var readStatus: (@Sendable () async -> SyncStatus)?
    @ObservationIgnored private var makeConnectionTest: (@Sendable (NextcloudSettings) async -> any NextcloudConnectionTestOperation)?
    @ObservationIgnored private var connectionTestOperation: (any NextcloudConnectionTestOperation)?
    @ObservationIgnored private var connectionTestTask: Task<Void, Never>?
    @ObservationIgnored private var connectionTestGeneration = 0
    private static let failureKey = "mobile-nextcloud-failure"

    public init(defaults: UserDefaults, persistence: any NextcloudConnectionPersistence, allowed: Bool = true,
                sleep: @escaping @Sendable (Duration) async throws -> Void = { try await Task.sleep(for: $0) }) {
        self.defaults = defaults
        self.persistence = persistence
        self.allowed = allowed
        self.sleep = sleep
        provider = allowed ? (SyncMethod(rawValue: defaults.string(forKey: PrefKey.syncMethod) ?? "") ?? .off) : .off
        failure = allowed ? defaults.string(forKey: Self.failureKey).flatMap(Failure.init(rawValue:)) : nil
    }

    public func connect(makeOperation: @escaping @Sendable (NextcloudSettings) async -> any NextcloudRunningOperation,
                        makeConnectionTest: (@Sendable (NextcloudSettings) async -> any NextcloudConnectionTestOperation)? = nil,
                        status: @escaping @Sendable () async -> SyncStatus) {
        self.makeOperation = makeOperation
        self.makeConnectionTest = makeConnectionTest
        readStatus = status
    }

    public func connectNearby(_ state: NearbyLifecycle) {
        nearby = state
        state.onEvent = { [weak self, weak state] event in
            guard let self else { return }
            status = state?.status
            if case .storeChanged = event { didCommit?() }
        }
        state.didStop = { [weak self] in
            guard let self else { return }
            Task { await self.refreshStatus() }
        }
        state.setForeground(foreground)
        state.setSelected(provider == .libresync)
    }

    public func load() async {
        guard allowed, !loaded, !loading else { return }
        loading = true
        defer { loading = false }
        do {
            let connection = try await persistence.load()
            saved = connection
            draft = connection ?? NextcloudConnection()
            loaded = true
            if connection?.validationIssue != nil {
                record(.configuration)
            } else if failure == .credentialsRead || failure == .configuration {
                record(nil)
            }
            await refreshStatus()
            schedule(after: .zero)
        } catch { record(.credentialsRead) }
    }

    @discardableResult public func save() async -> Bool {
        guard allowed, loaded, !saving else { return false }
        guard let candidate = try? draft.validated() else {
            record(.configuration)
            return false
        }
        saving = true
        await cancelAndDrain()
        do {
            try await persistence.save(candidate)
            saved = candidate
            draft = candidate
            saving = false
            if failure == .credentialsRead || failure == .credentialsWrite || failure == .configuration { record(nil) }
            schedule(after: .zero)
            return true
        } catch {
            saving = false
            record(.credentialsWrite)
            return false
        }
    }

    /// Changing the selection disables admission immediately and drains the old
    /// exchange before the selected provider can start. Connections remain saved.
    public func select(_ next: SyncMethod) async {
        guard allowed else { return }
        transitions += 1
        if next != .libresync {
            nearby?.setSelected(false)
            await nearby?.drain()
        }
        provider = next
        cancelConnectionTest()
        defaults.set(next.rawValue, forKey: PrefKey.syncMethod)
        await cancelAndDrain()
        if next == .libresync {
            nearby?.setSelected(true)
            await nearby?.drain()
        }
        transitions -= 1
        schedule(after: .zero)
    }

    public func setForeground(_ active: Bool) {
        foreground = active
        nearby?.setForeground(active)
        if active {
            if isSyncing { foregroundRetry = true }
            schedule(after: .zero)
        }
        else { cancel(); cancelConnectionTest() }
    }

    /// Low Power Mode pauses discretionary automatic work. A person can still
    /// choose Sync Now, and an already-running exchange is allowed to finish.
    public func setLowPowerMode(_ enabled: Bool) {
        guard lowPowerMode != enabled else { return }
        lowPowerMode = enabled
        if enabled {
            scheduled?.cancel()
            scheduled = nil
        } else {
            schedule(after: .zero)
        }
    }

    public func localChanges(immediate: Bool = false) {
        if provider == .libresync {
            scheduleNearby(after: immediate ? .zero : .seconds(20))
            return
        }
        if isSyncing { editsDuringSync = true }
        else { schedule(after: immediate ? .zero : .seconds(20)) }
    }

    public func refreshStatus() async {
        if let readStatus { status = await readStatus() }
    }

    public func syncNow() async {
        if provider == .libresync {
            await nearby?.syncNow()
            return
        }
        guard eligible, let saved, let makeOperation else { return }
        if let active { await active.value; return }
        scheduled?.cancel(); scheduled = nil
        isSyncing = true
        editsDuringSync = false
        foregroundRetry = false
        let admittedGeneration = generation
        let task = Task {
            let finishExecution = beginExecution?()
            defer { finishExecution?() }
            let next = await makeOperation(saved.settings)
            operation = next
            if Task.isCancelled || admittedGeneration != generation || !eligible {
                _ = next.cancel()
            } else {
                do {
                    _ = try await next.run()
                    // Commit may beat cancellation. Publish it even after Off/background.
                    if failure != .credentialsRead && failure != .credentialsWrite { record(nil) }
                    didCommit?()
                } catch is CancellationError {
                    // Preserve prior failures and the core's last successful exchange.
                } catch {
                    record(Self.classify(error))
                }
            }
            await refreshStatus()
            operation = nil
            isSyncing = false
            isStopping = false
            active = nil
            schedule(after: foregroundRetry ? .zero : editsDuringSync ? .seconds(20) : .seconds(300))
        }
        active = task
        await task.value
    }

    public func testConnection() async {
        guard canTestConnection else { return }
        if let connectionTestTask { await connectionTestTask.value; return }
        connectionTestGeneration += 1
        let admittedGeneration = connectionTestGeneration
        connectionTest = .testing
        let provider = provider
        let saved = saved
        let task = Task { [weak self] in
            guard let self else { return }
            let result: ProviderConnectionTestResult
            switch provider {
            case .nextcloud:
                guard let saved, let makeConnectionTest else {
                    result = .failure("Save a valid connection before testing it.")
                    break
                }
                let operation = await makeConnectionTest(saved.settings)
                connectionTestOperation = operation
                if Task.isCancelled || admittedGeneration != connectionTestGeneration {
                    _ = operation.cancel()
                    result = .failure("The connection test was cancelled.")
                    break
                }
                do {
                    try await operation.run()
                    result = .success
                } catch is CancellationError {
                    result = .failure("The connection test was cancelled.")
                } catch {
                    result = .failure(String(describing: error))
                }
                connectionTestOperation = nil
            case .libresync:
                result = await nearby?.testConnection()
                    ?? .failure("LibreSync is not ready yet.")
            case .off:
                result = .failure("Choose a sync provider before testing the connection.")
            }
            guard admittedGeneration == connectionTestGeneration, !Task.isCancelled else {
                connectionTest = .idle
                connectionTestTask = nil
                return
            }
            switch result {
            case .success: connectionTest = .success
            case .failure(let message):
                connectionTest = message.contains("cancelled") ? .idle : .failure(message)
            }
            connectionTestTask = nil
        }
        connectionTestTask = task
        await task.value
    }

    public func suspendForRestore() async {
        isSuspended = true
        await nearby?.suspend()
        await cancelAndDrain()
    }
    public func resumeAfterRestore() {
        isSuspended = false
        nearby?.resume()
        schedule(after: .seconds(20))
    }
    public func drain() async { await active?.value }

    private var eligible: Bool {
        allowed && loaded && saved != nil && saved?.validationIssue == nil
            && foreground && provider == .nextcloud
            && !saving && transitions == 0 && !isSuspended && makeOperation != nil
    }
    private func cancel() {
        generation += 1
        scheduled?.cancel(); scheduled = nil
        if isSyncing { isStopping = true }
        _ = operation?.cancel()
        active?.cancel()
    }
    private func cancelAndDrain() async {
        cancel()
        await active?.value
    }
    private func cancelConnectionTest() {
        connectionTestGeneration += 1
        _ = connectionTestOperation?.cancel()
        connectionTestOperation = nil
        connectionTestTask?.cancel()
        connectionTestTask = nil
        connectionTest = .idle
    }
    private func schedule(after delay: Duration) {
        scheduled?.cancel(); scheduled = nil
        guard eligible, saved?.automatic == true, !lowPowerMode, !isSyncing else { return }
        scheduled = Task { [weak self, sleep] in
            do { try await sleep(delay) } catch { return }
            guard !Task.isCancelled else { return }
            await self?.syncNow()
        }
    }
    private func scheduleNearby(after delay: Duration) {
        scheduled?.cancel(); scheduled = nil
        guard allowed, foreground, provider == .libresync, transitions == 0,
              !isSuspended, !lowPowerMode, nearby?.phase == .running else { return }
        scheduled = Task { [weak self, sleep] in
            do { try await sleep(delay) } catch { return }
            guard !Task.isCancelled, let self,
                  self.foreground, self.provider == .libresync,
                  !self.isSuspended, !self.lowPowerMode else { return }
            await self.nearby?.syncNow()
        }
    }
    private func record(_ value: Failure?) {
        failure = value
        defaults.set(value?.rawValue, forKey: Self.failureKey)
    }
    private static func classify(_ error: Error) -> Failure {
        guard let core = error as? CoreError else { return .network }
        switch core {
        case .NotConfigured: return .configuration
        case .Actionable, .Invalid: return .connection
        case .Busy: return .busy
        default: return .network
        }
    }
}
