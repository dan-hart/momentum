// SPDX-License-Identifier: GPL-3.0-or-later
import Accessibility
import Foundation
import MomentumCore
import MomentumKit
import MomentumMobile
import Observation
import UIKit

extension OrganizationCommand {
    var emitsMovementSuccessFeedback: Bool {
        switch self {
        case .today, .toggleToday, .removeToday, .slot, .tomorrow, .nextWeek,
             .project, .drop, .reorder, .nudge:
            true
        case .tag, .importText, .updateProject, .updateTag, .deleteProject, .deleteTag:
            false
        }
    }
}

enum TaskCommandFeedback: Equatable {
    case completionSuccess
    case selectionExit
    case silent

    static func after(_ command: TaskCommand, outcome: Outcome?) -> Self {
        guard outcome?.changed == true else { return .silent }
        switch command {
        case .add, .archive, .undo:
            return .silent
        case .complete(_, true):
            return .completionSuccess
        case .complete(_, false), .delete:
            return .selectionExit
        }
    }
}

struct FeedbackToast: Identifiable, Equatable {
    enum Kind: Equatable { case information, error }
    let id = UUID()
    let message: String
    let undoBatchID: UInt64?
    let kind: Kind
    var isPersistent: Bool { kind == .error }
}

@MainActor @Observable final class MobileAppModel {
    static let shared = MobileAppModel()
    private let engineSession = EngineSession()
    private var startup: Task<Void, Never>?
    private(set) var startupFailure: String?
    private(set) var isOpeningStore = false
    private(set) var isRestoringBackup = false
    private(set) var worker: EngineWorker?
    private(set) var revision = 0
    private(set) var feedbackToast: FeedbackToast?
    var feedback: String? { feedbackToast?.message }
    private(set) var completionFeedback = 0
    private(set) var selectionFeedback = 0
    private(set) var interactionSuccessFeedback = 0
    private(set) var lowPowerMode: Bool
    var showingAdd = false
    var showingNewProject = false
    var showingKeyboardHelp = false
    var searchFocusRevision = 0
    var selectedTab = 0
    struct SystemSearchRequest: Equatable {
        let taskID: String
        let title: String
        let revision: Int
    }
    private(set) var systemSearchRequest: SystemSearchRequest?
    private var systemSearchRevision = 0
    struct NotificationTask: Identifiable { let id: String }
    var notificationTask: NotificationTask?
    private var pendingNotificationRoute: NotificationRoute?
    var activeTaskEditors = 0 {
        didSet {
            if activeTaskEditors == 0 { presentPendingNotification() }
        }
    }
    var creationView: MomentumCore.View = .today
    let defaults: UserDefaults
    let testing: Bool
    let platformCapabilities: MobilePlatformCapabilities
    let allowsBackgroundNotificationRefresh: Bool
    let notifications: MobileNotifications
    let sync: NextcloudSyncState
    private let isolatedDirectory: URL?
    private let spotlight: (any MobileSpotlightIndexing)?
    private let spotlightDebounce: Duration
    private var spotlightRefresh: Task<Void, Never>?
    private var foregroundActive = false
    private let announceFeedback: @MainActor (AttributedString) -> Void
    private let feedbackSleep: @Sendable (Duration) async throws -> Void
    private var feedbackDismissal: Task<Void, Never>?

    struct LaunchMode: Equatable {
        let testing: Bool
        let demo: Bool
        let allowsSystemNotifications: Bool
        let allowsSystemSpotlight: Bool
        let allowsBackgroundNotificationRefresh: Bool
    }

    #if DEBUG
    private static let systemAppIntentsTestingKey = "momentum.debug.system-app-intents-testing"
    private static let systemNotificationTestingKey = "momentum.debug.system-notification-testing"

    private static func prepareSystemAppIntentsTesting(arguments: [String]) -> Bool {
        if arguments.contains("--clear-system-app-intents-testing") {
            UserDefaults.standard.removeObject(forKey: systemAppIntentsTestingKey)
        } else if arguments.contains("--system-app-intents-testing") {
            UserDefaults.standard.set(true, forKey: systemAppIntentsTestingKey)
        }
        return UserDefaults.standard.bool(forKey: systemAppIntentsTestingKey)
    }

    private static func prepareSystemNotificationTesting(arguments: [String]) -> Bool {
        if arguments.contains("--clear-system-notification-testing") {
            UserDefaults.standard.removeObject(forKey: systemNotificationTestingKey)
        } else if arguments.contains("--system-notification-testing") {
            UserDefaults.standard.set(true, forKey: systemNotificationTestingKey)
        }
        return UserDefaults.standard.bool(forKey: systemNotificationTestingKey)
    }
    #endif

    static func launchMode(hasIsolatedDirectory: Bool, arguments: [String],
                           hasSystemAppIntentsTestingMarker: Bool = false,
                           hasSystemNotificationTestingMarker: Bool = false) -> LaunchMode {
        let demo = arguments.contains("--demo")
        #if DEBUG
        let systemBadgeTesting = arguments.contains("--system-badge-testing")
        let systemSpotlightTesting = arguments.contains("--system-spotlight-testing")
        let systemAppIntentsTesting = hasSystemAppIntentsTestingMarker
            || arguments.contains("--system-app-intents-testing")
            || arguments.contains("--clear-system-app-intents-testing")
        let systemNotificationTesting = hasSystemNotificationTestingMarker
            || arguments.contains("--system-notification-testing")
            || arguments.contains("--clear-system-notification-testing")
        #else
        let systemBadgeTesting = false
        let systemSpotlightTesting = false
        let systemAppIntentsTesting = false
        let systemNotificationTesting = false
        #endif
        let testing = hasIsolatedDirectory || systemBadgeTesting || systemSpotlightTesting || systemAppIntentsTesting
            || systemNotificationTesting
        return LaunchMode(testing: testing, demo: demo,
                          allowsSystemNotifications: !demo && (!testing || systemBadgeTesting || systemNotificationTesting),
                          allowsSystemSpotlight: !demo && (!testing || systemSpotlightTesting),
                          allowsBackgroundNotificationRefresh: !demo && !testing)
    }

    init(isolatedDirectory: URL? = nil, defaults isolatedDefaults: UserDefaults? = nil,
         spotlight injectedSpotlight: (any MobileSpotlightIndexing)? = nil,
         spotlightDebounce: Duration = .seconds(2),
         initialLowPowerMode: Bool = ProcessInfo.processInfo.isLowPowerModeEnabled,
         platformCapabilities injectedPlatformCapabilities: MobilePlatformCapabilities? = nil,
         feedbackSleep: @escaping @Sendable (Duration) async throws -> Void = { try await Task.sleep(for: $0) },
         announceFeedback: @escaping @MainActor (AttributedString) -> Void = {
             AccessibilityNotification.Announcement($0).post()
         }) {
        precondition((isolatedDirectory == nil) == (isolatedDefaults == nil), "An isolated store must have isolated preferences")
        self.isolatedDirectory = isolatedDirectory
        self.announceFeedback = announceFeedback
        self.feedbackSleep = feedbackSleep
        platformCapabilities = injectedPlatformCapabilities
            ?? MobilePlatformCapabilities(isPad: UIDevice.current.userInterfaceIdiom == .pad)
        lowPowerMode = initialLowPowerMode
        let arguments = ProcessInfo.processInfo.arguments
        #if DEBUG
        let hasSystemAppIntentsTestingMarker = Self.prepareSystemAppIntentsTesting(arguments: arguments)
        let hasSystemNotificationTestingMarker = Self.prepareSystemNotificationTesting(arguments: arguments)
        #else
        let hasSystemAppIntentsTestingMarker = false
        let hasSystemNotificationTestingMarker = false
        #endif
        let mode = Self.launchMode(hasIsolatedDirectory: isolatedDirectory != nil,
                                   arguments: arguments,
                                   hasSystemAppIntentsTestingMarker: hasSystemAppIntentsTestingMarker,
                                   hasSystemNotificationTestingMarker: hasSystemNotificationTestingMarker)
        #if DEBUG
        self.spotlightDebounce = ProcessInfo.processInfo.arguments.contains("--system-spotlight-testing")
            ? .zero : spotlightDebounce
        #else
        self.spotlightDebounce = spotlightDebounce
        #endif
        let testing = mode.testing
        self.testing = testing
        let demo = mode.demo
        spotlight = injectedSpotlight ?? (mode.allowsSystemSpotlight ? MobileSpotlightIndexer() : nil)
        defaults = isolatedDefaults ?? (testing ? UserDefaults(suiteName: "momentum-ios-ui-tests")!
            : demo ? UserDefaults(suiteName: "momentum-ios-preview")! : .standard)
        if testing && ProcessInfo.processInfo.arguments.contains("--reset-test-store") {
            defaults.removePersistentDomain(forName: "momentum-ios-ui-tests")
        }
        Preferences.register(defaults)
        notifications = MobileNotifications(isolated: !mode.allowsSystemNotifications, defaults: defaults)
        allowsBackgroundNotificationRefresh = mode.allowsBackgroundNotificationRefresh
        let syncState = NextcloudSyncState(defaults: defaults, persistence: NextcloudConnectionStore(),
                                           allowed: !testing && !demo)
        sync = syncState
        syncState.setLowPowerMode(initialLowPowerMode)
    }

    func start() async {
        guard worker == nil else { return }
        if let startup { await startup.value; return }
        let task = Task { await openStore() }
        startup = task
        await task.value
        startup = nil
    }

    private func openStore() async {
        isOpeningStore = true
        defer { isOpeningStore = false }
        let demo = isolatedDirectory == nil && ProcessInfo.processInfo.arguments.contains("--demo")
        let root = DataDirectory.url
        let directory = isolatedDirectory ?? (testing ? root.appendingPathComponent("ui-tests")
            : demo ? root.appendingPathComponent("preview") : root)
        if testing && ProcessInfo.processInfo.arguments.contains("--reset-test-store") {
            await Task.detached { try? FileManager.default.removeItem(at: directory) }.value
        }
        do {
            let opened = try await engineSession.open(directory: directory, demo: demo)
            await opened.setPreferences(Preferences(defaults).core)
            worker = opened
            startupFailure = nil
            notifications.connect(worker: opened)
            sync.connect(makeOperation: { await opened.nextcloudOperation(settings: $0) },
                         makeConnectionTest: { await opened.nextcloudConnectionTestOperation(settings: $0) },
                         status: { await opened.syncStatus() })
            let nearbyRuntime = await opened.nearbyRuntime(
                keychain: Keychain(),
                deviceName: UIDevice.current.name
            )
            sync.connectNearby(NearbyLifecycle(runtime: nearbyRuntime, allowed: sync.allowed))
            sync.didCommit = { [weak self] in
                guard let self else { return }
                revision += 1
                Task { await notifications.refresh() }
                scheduleSpotlight()
            }
            sync.beginExecution = { [weak sync] in
                let allowance = SyncExecutionAllowance(expiration: { sync?.setForeground(false) })
                return { allowance.finish() }
            }
        } catch {
            startupFailure = String(describing: error)
        }
    }

    func foreground() async {
        await start()
        await worker?.refreshForForeground()
        await sync.load()
        await sync.refreshStatus()
        revision += 1
        await notifications.refresh()
        scheduleSpotlight(immediate: true)
    }

    var shouldScheduleNotificationBackgroundRefresh: Bool {
        allowsBackgroundNotificationRefresh
            && NotificationBackgroundRefreshPolicy.shouldSchedule(
                status: notifications.status,
                isLowPowerModeEnabled: lowPowerMode
            )
    }

    func refreshNotificationsInBackground() async -> Bool {
        guard allowsBackgroundNotificationRefresh, !lowPowerMode, !Task.isCancelled else { return false }
        await start()
        guard worker != nil, !Task.isCancelled else { return false }
        await notifications.refresh()
        guard !Task.isCancelled else { return false }
        return NotificationBackgroundRefreshPolicy.completedSuccessfully(status: notifications.status)
    }

    @discardableResult func perform(_ command: TaskCommand) async -> Outcome? {
        guard let worker else { return nil }
        let result = await worker.perform(command)
        accept(result)
        if TaskCommandFeedback.after(command, outcome: result) == .completionSuccess {
            completionFeedback += 1
        }
        return result
    }

    @discardableResult func organize(_ command: OrganizationCommand) async -> Outcome? {
        guard let worker else { return nil }
        let result = await worker.organize(command)
        accept(result)
        if result.changed, command.emitsMovementSuccessFeedback { interactionSuccessFeedback += 1 }
        return result
    }

    func createTaskFromText(_ text: String, view: MomentumCore.View) async -> TaskCreationResult? {
        guard let worker else { return nil }
        let result = await worker.createTaskFromText(text, view: view)
        accept(result.outcome)
        if result.outcome.changed { interactionSuccessFeedback += 1 }
        return result
    }

    func importTasksFromText(_ text: String, view: MomentumCore.View) async -> TaskCreationResult? {
        guard let worker else { return nil }
        let result = await worker.importTasksFromText(text, view: view)
        accept(result.outcome)
        if result.outcome.changed { interactionSuccessFeedback += 1 }
        return result
    }

    func accept(_ outcome: Outcome) {
        if let message = outcome.message {
            let kind: FeedbackToast.Kind
            if case .saveFailed = message { kind = .error } else { kind = .information }
            showFeedback(Strings.message(message), undoBatchID: outcome.undo, kind: kind)
        }
        if outcome.changed { refreshAfterEdit(immediateSync: outcome.syncNow) }
    }

    func exportBackup() async throws -> Data {
        guard let worker else { throw AutomationError.unavailable }
        return try await worker.exportBackup()
    }

    func restoreBackup(_ file: BackupFile) async throws {
        guard let worker else { throw AutomationError.unavailable }
        guard !isRestoringBackup, !showingAdd, !showingNewProject, activeTaskEditors == 0 else {
            throw CancellationError()
        }
        isRestoringBackup = true
        await sync.suspendForRestore()
        defer { isRestoringBackup = false; sync.resumeAfterRestore() }
        let outcome = try await worker.restoreBackup(file)
        accept(outcome) // Refresh visible tasks and request normal notification reconciliation.
    }

    func showFeedback(_ message: String, undoBatchID: UInt64? = nil,
                      kind: FeedbackToast.Kind = .information) {
        feedbackDismissal?.cancel()
        let toast = FeedbackToast(message: message, undoBatchID: undoBatchID, kind: kind)
        feedbackToast = toast
        // Post once at the event boundary, not from each visible/retained list.
        // Low priority lets ongoing assistive speech finish without moving focus.
        var announcement = AttributedString(message)
        announcement.accessibilitySpeechAnnouncementPriority = .low
        announceFeedback(announcement)
        guard !toast.isPersistent else { return }
        feedbackDismissal = Task { @MainActor [weak self, feedbackSleep] in
            do { try await feedbackSleep(undoBatchID == nil ? .milliseconds(3500) : .seconds(5)) }
            catch { return }
            guard !Task.isCancelled, self?.feedbackToast?.id == toast.id else { return }
            self?.feedbackToast = nil
        }
    }

    func dismissFeedback() {
        feedbackDismissal?.cancel()
        feedbackDismissal = nil
        feedbackToast = nil
    }

    func undoFeedback() async {
        guard let batchID = feedbackToast?.undoBatchID, let worker else { return }
        dismissFeedback()
        var outcome = await worker.undo(batchID: batchID)
        if outcome.changed, outcome.message == nil { outcome.message = .undone }
        accept(outcome)
    }

    func refreshAfterEdit() { refreshAfterEdit(immediateSync: false) }

    func refreshAfterTaskCreation() {
        interactionSuccessFeedback += 1
        refreshAfterEdit()
    }

    private func refreshAfterEdit(immediateSync: Bool) {
        revision += 1
        sync.localChanges(immediate: immediateSync)
        Task { await notifications.refresh(); await sync.refreshStatus() }
        scheduleSpotlight()
    }

    func setForeground(_ foreground: Bool) {
        foregroundActive = foreground
        if !foreground {
            spotlightRefresh?.cancel()
            spotlightRefresh = nil
        }
    }

    func setLowPowerMode(_ enabled: Bool) {
        guard lowPowerMode != enabled else { return }
        lowPowerMode = enabled
        sync.setLowPowerMode(enabled)
        if enabled {
            spotlightRefresh?.cancel()
            spotlightRefresh = nil
        } else if foregroundActive {
            scheduleSpotlight()
        }
    }

    private func scheduleSpotlight(immediate: Bool = false) {
        guard !lowPowerMode, let spotlight, let worker else { return }
        spotlightRefresh?.cancel()
        let delay = spotlightDebounce
        spotlightRefresh = Task {
            if !immediate {
                do { try await Task.sleep(for: delay) }
                catch { return }
            }
            guard !Task.isCancelled else { return }
            let documents = await worker.mobileSearchDocuments()
            guard !Task.isCancelled else { return }
            await spotlight.replace(documents)
        }
    }

    func preferencesChanged() async {
        await worker?.setPreferences(Preferences(defaults).core)
        revision += 1
        await notifications.refresh()
    }

    func add(in view: MomentumCore.View) {
        guard !showingAdd else { return }
        creationView = view
        showingAdd = true
        selectionFeedback += 1
    }

    func sidebarSelectionDidChange(from oldValue: MomentumCore.View, to newValue: MomentumCore.View) {
        guard oldValue != newValue else { return }
        selectionFeedback += 1
    }

    func accentSelectionDidChange(from oldValue: String, to newValue: String) {
        guard AccentChoice.resolve(oldValue).id != AccentChoice.resolve(newValue).id else { return }
        selectionFeedback += 1
    }

    func selectionModeDidChange(from oldValue: Bool, to newValue: Bool) {
        guard oldValue != newValue else { return }
        selectionFeedback += 1
    }

    func handle(url: URL) async {
        guard let action = TaskURLAction(url: url) else { return }
        await start()
        guard let worker else { return }
        if await worker.refreshForForeground() { refreshAfterEdit() }
        accept(await worker.handle(action))
        await notifications.refresh()
    }

    func openAutomationTask(_ id: String) async throws {
        await start()
        guard let worker else { throw AutomationError.unavailable }
        _ = try await worker.automationTasks(ids: [id])
        pendingNotificationRoute = .reminder(taskID: id)
        presentPendingNotification()
    }

    /// Resolve the current title so a stale system-search result never routes to
    /// deleted content or relies on metadata cached outside the task store.
    @discardableResult func openSpotlightTask(_ id: String) async -> Bool {
        await start()
        guard let title = await worker?.taskTitle(id) else { return false }
        systemSearchRevision += 1
        systemSearchRequest = SystemSearchRequest(taskID: id, title: title, revision: systemSearchRevision)
        selectedTab = 2
        return true
    }

    func handleNotification(_ route: NotificationRoute, action: TaskNotificationAction?, deliveryID: String) async -> Bool {
        await start()
        guard let worker else { return false }
        switch route {
        case .summary:
            guard action == nil else { return false }
            pendingNotificationRoute = route
            presentPendingNotification()
        case .reminder(let taskID):
            if let action {
                if let outcome = await worker.performNotificationAction(action, taskID: taskID, deliveryID: deliveryID) {
                    accept(outcome)
                    if !outcome.changed, outcome.message != nil { return false }
                }
            } else {
                pendingNotificationRoute = route
                presentPendingNotification()
            }
        }
        await notifications.refresh()
        return true
    }

    /// A notification must not discard an in-progress capture or editor draft.
    func presentPendingNotification() {
        guard !showingAdd, !showingNewProject, !showingKeyboardHelp, activeTaskEditors == 0,
              notificationTask == nil, let pendingNotificationRoute else { return }
        self.pendingNotificationRoute = nil
        switch pendingNotificationRoute {
        case .summary: selectedTab = 0
        case .reminder(let taskID): notificationTask = NotificationTask(id: taskID)
        }
    }
}
