// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import MomentumKit
import Observation

/// Owns presentation and scheduling only. Rust owns transport, conflict resolution,
/// deadlines and the final commit boundary. No polling runs while inactive or Off.
/// Cadence (2026-09-29, twice the desktop's): edits settle for 10 s before an upload,
/// and foreground reconciliation backs off as the app becomes idle.
@MainActor @Observable public final class NextcloudSyncState {
    public enum Failure: String, Sendable {
        case credentialsRead, credentialsWrite, configuration, connection, network, busy
    }
    public enum ConnectionTestState: Equatable, Sendable {
        case idle, testing, success, failure(String)
    }
    public enum FolderCopyState: Equatable, Sendable {
        case idle, copying, saving, copied, failure(String)
    }
    public private(set) var folderCopy: FolderCopyState = .idle
    public var folderCopyInProgress: Bool { folderCopy == .copying || folderCopy == .saving }
    public var canCopyFolder: Bool {
        canSync && provider == .nextcloud && saved?.folder != "momentum" && makeFolderCopy != nil && !folderCopyInProgress
    }
    public var draft = NextcloudConnection() {
        didSet {
            if oldValue != draft { cancelConnectionTest() }
            if oldValue.identity != draft.identity {
                draft.optimized = Preferences(defaults).optimizedConnectionIdentities.contains(draft.identity)
            }
        }
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
    /// The core's classification of the last failed exchange, when it was one.
    public private(set) var failureKind: SyncFailureKind?
    /// The core's own sentence for the last failed exchange, for a Details control.
    public private(set) var failureMessage: String?
    /// When the last exchange or replacement ended, however it ended, so a retry is
    /// visibly acknowledged even when the same failure comes back.
    public private(set) var lastAttemptAt: Date?
    public private(set) var status: SyncStatus?
    public private(set) var connectionTest: ConnectionTestState = .idle
    public private(set) var nearby: NearbyLifecycle?
    @ObservationIgnored public var didCommit: (@MainActor () -> Void)?
    /// Independent settings exchanges can commit even when the task exchange fails.
    @ObservationIgnored public var didFinish: (@MainActor () -> Void)?
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
    @ObservationIgnored private let now: @MainActor @Sendable () -> TimeInterval
    @ObservationIgnored private var lastActivity: TimeInterval
    @ObservationIgnored private var editScheduled = false
    @ObservationIgnored private let pushSleep: @Sendable () async throws -> Void
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
    @ObservationIgnored private var pushTask: Task<Void, Never>?
    @ObservationIgnored private var pushHint: (@Sendable (NextcloudSettings) async -> Bool)?
    @ObservationIgnored private var stopPush: (@Sendable () async -> Void)?
    @ObservationIgnored private var optimizedRequired: (@Sendable (NextcloudSettings) async -> Bool)?
    @ObservationIgnored private var makeOperation: (@Sendable (NextcloudSettings) async -> any NextcloudRunningOperation)?
    @ObservationIgnored private var makeBackgroundOperation: (@Sendable (NextcloudSettings) async -> any NextcloudRunningOperation)?
    @ObservationIgnored private var makeReplaceOperation: (@Sendable (NextcloudSettings) async -> any NextcloudRunningOperation)?
    @ObservationIgnored private var readStatus: (@Sendable () async -> SyncStatus)?
    @ObservationIgnored private var makeConnectionTest: (@Sendable (NextcloudSettings) async -> any NextcloudConnectionTestOperation)?
    @ObservationIgnored private var makeFolderCopy: (@Sendable (NextcloudSettings) async -> any NextcloudFolderCopying)?
    @ObservationIgnored private var folderCopyOperation: (any NextcloudFolderCopying)?
    @ObservationIgnored private var folderCopyCancellationRequested = false
    @ObservationIgnored private var connectionTestOperation: (any NextcloudConnectionTestOperation)?
    @ObservationIgnored private var connectionTestTask: Task<Void, Never>?
    @ObservationIgnored private var connectionTestGeneration = 0
    private static let failureKey = "mobile-nextcloud-failure"
    private static let failureKindKey = "mobile-nextcloud-failure-kind"
    private static let failureMessageKey = "mobile-nextcloud-failure-message"

    public init(defaults: UserDefaults, persistence: any NextcloudConnectionPersistence, allowed: Bool = true,
                sleep: @escaping @Sendable (Duration) async throws -> Void = { try await Task.sleep(for: $0) },
                now: @escaping @MainActor @Sendable () -> TimeInterval = { ProcessInfo.processInfo.systemUptime },
                pushSleep: @escaping @Sendable () async throws -> Void = { try await Task.sleep(for: .seconds(1)) }) {
        self.defaults = defaults
        self.persistence = persistence
        self.allowed = allowed
        self.sleep = sleep
        self.now = now
        self.pushSleep = pushSleep
        self.lastActivity = now()
        provider = allowed ? (SyncMethod(rawValue: defaults.string(forKey: PrefKey.syncMethod) ?? "") ?? .off) : .off
        failure = allowed ? defaults.string(forKey: Self.failureKey).flatMap(Failure.init(rawValue:)) : nil
        if failure != nil {
            failureKind = defaults.string(forKey: Self.failureKindKey).flatMap(SyncFailureKind.init(storageName:))
            failureMessage = defaults.string(forKey: Self.failureMessageKey)
        }
    }

    public func connect(makeOperation: @escaping @Sendable (NextcloudSettings) async -> any NextcloudRunningOperation,
                        makeBackgroundOperation: (@Sendable (NextcloudSettings) async -> any NextcloudRunningOperation)? = nil,
                        makeReplaceOperation: (@Sendable (NextcloudSettings) async -> any NextcloudRunningOperation)? = nil,
                        makeConnectionTest: (@Sendable (NextcloudSettings) async -> any NextcloudConnectionTestOperation)? = nil,
                        makeFolderCopy: (@Sendable (NextcloudSettings) async -> any NextcloudFolderCopying)? = nil,
                        optimizedRequired: (@Sendable (NextcloudSettings) async -> Bool)? = nil,
                        pushHint: (@Sendable (NextcloudSettings) async -> Bool)? = nil,
                        stopPush: (@Sendable () async -> Void)? = nil,
                        status: @escaping @Sendable () async -> SyncStatus) {
        self.makeOperation = makeOperation
        self.makeBackgroundOperation = makeBackgroundOperation
        self.makeReplaceOperation = makeReplaceOperation
        self.makeConnectionTest = makeConnectionTest
        self.makeFolderCopy = makeFolderCopy
        self.optimizedRequired = optimizedRequired
        self.pushHint = pushHint
        self.stopPush = stopPush
        readStatus = status
    }

    public func connectNearby(_ state: NearbyLifecycle) {
        nearby = state
        state.onEvent = { [weak self, weak state] event in
            guard let self else { return }
            status = state?.status
            if case .storeChanged = event { didCommit?() }
            didFinish?()
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
            var connection = try await persistence.load()
            if var candidate = connection, await optimizedRequired?(candidate.settings) == true {
                candidate.optimized = true
                try await persistence.save(candidate)
                connection = candidate
            }
            saved = connection
            if let connection, connection.optimized { Preferences(defaults).rememberOptimizedConnection(connection.identity) }
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
        guard allowed, loaded, !saving, !folderCopyInProgress else { return false }
        guard var candidate = try? draft.validated() else {
            record(.configuration)
            return false
        }
        saving = true
        if !candidate.optimized, await optimizedRequired?(candidate.settings) == true { candidate.optimized = true }
        candidate.optimized = candidate.optimized || Preferences(defaults).optimizedConnectionIdentities.contains(candidate.identity)
        await cancelAndDrain()
        do {
            try await persistence.save(candidate)
            if candidate.optimized { Preferences(defaults).rememberOptimizedConnection(candidate.identity) }
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

    public var optimizedSyncEnabled: Bool { saved?.optimized == true && saved?.identity == draft.identity }

    /// Called only after the person confirms the one-way connection upgrade.
    public func upgradeOptimizedSync(expectedIdentity: String) async {
        guard !hasUnsavedChanges, !draft.optimized, !isSyncing, canSync, draft.identity == expectedIdentity else { return }
        draft.optimized = true
        if await save() { await syncNow() }
    }

    /// Changing the selection disables admission immediately and drains the old
    /// exchange before the selected provider can start. Connections remain saved.
    public func select(_ next: SyncMethod) async {
        guard !folderCopyInProgress else { return }
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
            lastActivity = now()
            if isSyncing { foregroundRetry = true }
            schedule(after: .zero)
        }
        else { cancel(); cancelConnectionTest(); cancelFolderCopy() }
    }

    /// Low Power Mode pauses discretionary automatic work. A person can still
    /// choose Sync Now, and an already-running exchange is allowed to finish.
    public func setLowPowerMode(_ enabled: Bool) {
        guard lowPowerMode != enabled else { return }
        lowPowerMode = enabled
        if enabled {
            stopPushMonitoring()
            scheduled?.cancel()
            scheduled = nil
            editScheduled = false
        } else {
            schedule(after: .zero)
        }
    }

    public func localChanges(immediate: Bool = false) {
        lastActivity = now()
        if provider == .libresync {
            scheduleNearby(after: immediate ? .zero : .seconds(10))
            return
        }
        if isSyncing { editsDuringSync = true }
        else { schedule(after: immediate ? .zero : editDelay, forEdit: !immediate) }
    }

    public func refreshStatus() async {
        if let readStatus { status = await readStatus() }
    }

    public func syncNow() async {
        if provider == .libresync {
            await nearby?.syncNow()
            return
        }
        guard let makeOperation else { return }
        await run(makeOperation)
    }

    @discardableResult public func copyToMomentumFolder() async -> Bool {
        guard canCopyFolder, let source = saved, let makeFolderCopy else { return false }
        folderCopyCancellationRequested = false
        folderCopy = .copying
        await suspendForRestore()
        defer { folderCopyOperation = nil; resumeAfterRestore() }
        do {
            let operation = await makeFolderCopy(source.settings)
            folderCopyOperation = operation
            try Task.checkCancellation()
            if folderCopyCancellationRequested { throw CancellationError() }
            let report = try await withTaskCancellationHandler {
                try await operation.run()
            } onCancel: { _ = operation.cancel() }
            try Task.checkCancellation()
            if folderCopyCancellationRequested { throw CancellationError() }
            guard report.verified, report.sourceFolder == source.folder, report.targetFolder == "momentum",
                  saved == source, draft == source, provider == .nextcloud else {
                throw CocoaError(.fileReadCorruptFile)
            }
            // Verification is the commit boundary. Once credential persistence starts,
            // cancellation cannot turn a saved connection into an unreported success.
            folderCopyOperation = nil
            folderCopy = .saving
            var destination = source
            destination.folder = report.targetFolder
            try await persistence.save(destination)
            if destination.optimized { Preferences(defaults).rememberOptimizedConnection(destination.identity) }
            saved = destination
            draft = destination
            folderCopy = .copied
            return true
        } catch is CancellationError {
            folderCopy = .failure(String(localized: "Folder copy was cancelled. The original folder stays intact."))
        } catch {
            folderCopy = .failure(Strings.error(error))
        }
        return false
    }
    public func cancelFolderCopy() {
        guard folderCopy == .copying else { return }
        folderCopyCancellationRequested = true
        _ = folderCopyOperation?.cancel()
    }

    /// Whether Replace Server Copy is offered: the core said the server copy is
    /// damaged and a replacement operation is wired up.
    public var canReplaceServerCopy: Bool {
        failureKind == .remoteFileDamaged && makeReplaceOperation != nil && provider == .nextcloud
    }

    /// Publish this device's tasks as the server copy after the person confirmed that
    /// this device's tasks should win. Same admission and commit path as an exchange.
    public func replaceServerCopy() async {
        guard canReplaceServerCopy, let makeReplaceOperation else { return }
        await run(makeReplaceOperation)
    }

    private func run(_ make: @escaping @Sendable (NextcloudSettings) async -> any NextcloudRunningOperation) async {
        guard eligible, let saved else { return }
        if let active { await active.value; return }
        scheduled?.cancel(); scheduled = nil
        editScheduled = false
        isSyncing = true
        editsDuringSync = false
        foregroundRetry = false
        let admittedGeneration = generation
        let task = Task {
            let finishExecution = beginExecution?()
            defer { finishExecution?() }
            let next = await make(saved.settings)
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
                    record(Self.classify(error), from: error)
                }
                lastAttemptAt = Date()
            }
            await refreshStatus()
            didFinish?()
            operation = nil
            isSyncing = false
            isStopping = false
            active = nil
            schedule(after: foregroundRetry ? .zero : editsDuringSync ? editDelay : pollDelay)
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

    /// A system background wake may run one exchange: every condition except being on
    /// screen applies, and Low Power Mode still pauses discretionary work.
    public var backgroundSyncPossible: Bool {
        allowed && loaded && saved != nil && saved?.validationIssue == nil && saved?.automatic == true
            && provider == .nextcloud && !saving && transitions == 0 && !isSuspended
            && !lowPowerMode && makeOperation != nil
    }

    /// One bounded exchange from a background wake, without the window. Cancelling the
    /// calling task (the wake expiring) cancels the running operation; a commit that
    /// beats the cancellation is still published. Returns whether an exchange committed.
    public func syncInBackground() async -> Bool {
        guard backgroundSyncPossible, let saved, let makeOperation = makeBackgroundOperation ?? makeOperation else { return false }
        if let active { await active.value }
        guard !Task.isCancelled, backgroundSyncPossible, !isSyncing else { return false }
        scheduled?.cancel(); scheduled = nil
        editScheduled = false
        isSyncing = true
        editsDuringSync = false
        foregroundRetry = false
        let admittedGeneration = generation
        let exchange = Task<Bool, Never> {
            let next = await makeOperation(saved.settings)
            operation = next
            var committed = false
            if Task.isCancelled || admittedGeneration != generation {
                _ = next.cancel()
            } else {
                do {
                    // An expiring wake cancels this task; hand that to the operation
                    // explicitly so the core's cancellation barrier sees it at once.
                    _ = try await withTaskCancellationHandler {
                        try await next.run()
                    } onCancel: {
                        _ = next.cancel()
                    }
                    if failure != .credentialsRead && failure != .credentialsWrite { record(nil) }
                    didCommit?()
                    committed = true
                } catch is CancellationError {
                    // The wake expired or the provider changed; the core kept its state.
                } catch CoreError.Transient(kind: .timedOut, message: _) {
                    // The wake's fixed budget ran out, which a large file over a slow link
                    // does. Not a failure to show: the next foreground exchange gets the
                    // full budget and reports its own result.
                } catch {
                    record(Self.classify(error), from: error)
                }
                lastAttemptAt = Date()
            }
            await refreshStatus()
            didFinish?()
            operation = nil
            isSyncing = false
            isStopping = false
            active = nil
            // Same hand-off as a foreground exchange: a return to the foreground while
            // the wake ran retries at once, edits made meanwhile upload promptly, and
            // otherwise the idle cycle is re-armed (no-op while still in the background).
            schedule(after: foregroundRetry ? .zero : editsDuringSync ? editDelay : pollDelay)
            return committed
        }
        active = Task { _ = await exchange.value }
        return await withTaskCancellationHandler {
            await exchange.value
        } onCancel: {
            exchange.cancel()
        }
    }

    public func suspendForRestore() async {
        isSuspended = true
        await nearby?.suspend()
        await cancelAndDrain()
    }
    public func resumeAfterRestore() {
        isSuspended = false
        nearby?.resume()
        schedule(after: editDelay)
    }
    public func drain() async { await active?.value }

    private var eligible: Bool {
        allowed && loaded && saved != nil && saved?.validationIssue == nil
            && foreground && provider == .nextcloud
            && !saving && transitions == 0 && !isSuspended && makeOperation != nil
    }
    private func cancel() {
        stopPushMonitoring()
        generation += 1
        scheduled?.cancel(); scheduled = nil
        editScheduled = false
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
    private func stopPushMonitoring() {
        pushTask?.cancel(); pushTask = nil
        if let stopPush { Task { await stopPush() } }
    }
    private func updatePushMonitoring() {
        guard eligible, saved?.automatic == true, !lowPowerMode, let saved, let pushHint else {
            stopPushMonitoring()
            return
        }
        guard pushTask == nil else { return }
        pushTask = Task { [weak self, pushSleep] in
            while !Task.isCancelled {
                do { try await pushSleep() } catch { return }
                guard let self, self.eligible, !self.lowPowerMode, self.saved == saved else { return }
                let hinted = await pushHint(saved.settings)
                guard !Task.isCancelled, self.eligible, self.saved == saved else { return }
                if hinted {
                    if self.isSyncing { self.editsDuringSync = true }
                    else { await self.syncNow() }
                }
            }
        }
    }

    private var editDelay: Duration { .milliseconds(Int64(syncEditDelayMs())) }
    private var pollDelay: Duration {
        .milliseconds(Int64(syncPollDelayMs(idleMs: UInt64(max(0, now() - lastActivity) * 1_000))))
    }
    private func schedule(after delay: Duration, forEdit: Bool = false) {
        updatePushMonitoring()
        // Keep the first edit deadline: a continuous burst must not postpone upload.
        if forEdit && editScheduled { return }
        scheduled?.cancel(); scheduled = nil
        editScheduled = false
        guard eligible, saved?.automatic == true, !lowPowerMode, !isSyncing else { return }
        editScheduled = forEdit
        scheduled = Task { [weak self, sleep] in
            do { try await sleep(delay) } catch { return }
            guard !Task.isCancelled else { return }
            await self?.syncNow()
        }
    }
    private func scheduleNearby(after delay: Duration) {
        scheduled?.cancel(); scheduled = nil
        editScheduled = false
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
    private func record(_ value: Failure?, from error: Error? = nil) {
        failure = value
        defaults.set(value?.rawValue, forKey: Self.failureKey)
        let core = error as? CoreError
        failureKind = switch core {
        case .Actionable(let kind, _), .Transient(let kind, _): kind
        default: nil
        }
        failureMessage = switch core {
        case .Actionable(_, let message), .Transient(_, let message), .Io(let message), .Invalid(let message),
             .Nearby(let message): message
        case .NotConfigured, .Busy, .none: error.map { String(describing: $0) }
        }
        if value == nil { failureKind = nil; failureMessage = nil }
        defaults.set(failureKind?.storageName, forKey: Self.failureKindKey)
        defaults.set(failureMessage, forKey: Self.failureMessageKey)
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

extension SyncFailureKind {
    /// A stable name for defaults; the generated enum has no raw value.
    var storageName: String {
        switch self {
        case .unreachable: "unreachable"
        case .unauthorized: "unauthorized"
        case .serverError: "serverError"
        case .timedOut: "timedOut"
        case .encryptionPasswordMissing: "encryptionPasswordMissing"
        case .encryptionPasswordWrong: "encryptionPasswordWrong"
        case .remoteFileDamaged: "remoteFileDamaged"
        case .incompatible: "incompatible"
        case .nothingToStartFrom: "nothingToStartFrom"
        case .conflict: "conflict"
        case .cancelled: "cancelled"
        case .uploadIncomplete: "uploadIncomplete"
        case .other: "other"
        }
    }

    static let allStored: [SyncFailureKind] = [
        .unreachable, .unauthorized, .serverError, .timedOut, .encryptionPasswordMissing, .encryptionPasswordWrong,
        .remoteFileDamaged, .incompatible, .nothingToStartFrom, .conflict, .cancelled, .uploadIncomplete, .other,
    ]

    init?(storageName: String) {
        guard let kind = Self.allStored.first(where: { $0.storageName == storageName }) else { return nil }
        self = kind
    }
}
