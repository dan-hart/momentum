// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import MomentumKit
import MomentumMobile
import SwiftUI
import XCTest

@MainActor final class AutomationIntegrationTests: XCTestCase {
    func testSystemBadgeAcceptanceModeUsesTestDataWithLiveNotifications() {
        let mode = MobileAppModel.launchMode(
            hasIsolatedDirectory: false,
            arguments: ["Momentum", "--system-badge-testing"]
        )
        XCTAssertTrue(mode.testing)
        XCTAssertFalse(mode.demo)
        XCTAssertTrue(mode.allowsSystemNotifications)
        XCTAssertFalse(mode.allowsSystemSpotlight)

        let isolatedTests = MobileAppModel.launchMode(
            hasIsolatedDirectory: true,
            arguments: ["Momentum"]
        )
        XCTAssertTrue(isolatedTests.testing)
        XCTAssertFalse(isolatedTests.allowsSystemNotifications)
        XCTAssertFalse(isolatedTests.allowsSystemSpotlight)
    }

    func testSystemSpotlightAcceptanceModeUsesTestDataWithOnlyLiveSpotlight() {
        let mode = MobileAppModel.launchMode(
            hasIsolatedDirectory: false,
            arguments: ["Momentum", "--system-spotlight-testing"]
        )
        XCTAssertTrue(mode.testing)
        XCTAssertFalse(mode.demo)
        XCTAssertFalse(mode.allowsSystemNotifications)
        XCTAssertTrue(mode.allowsSystemSpotlight)

        let production = MobileAppModel.launchMode(
            hasIsolatedDirectory: false,
            arguments: ["Momentum"]
        )
        XCTAssertFalse(production.testing)
        XCTAssertTrue(production.allowsSystemSpotlight)
    }

    func testSystemAppIntentsAcceptanceModeKeepsColdLaunchesIsolated() {
        let armed = MobileAppModel.launchMode(
            hasIsolatedDirectory: false,
            arguments: ["Momentum", "--system-app-intents-testing"]
        )
        XCTAssertTrue(armed.testing)
        XCTAssertFalse(armed.allowsSystemNotifications)
        XCTAssertFalse(armed.allowsSystemSpotlight)

        let coldLaunch = MobileAppModel.launchMode(
            hasIsolatedDirectory: false,
            arguments: ["Momentum"],
            hasSystemAppIntentsTestingMarker: true
        )
        XCTAssertTrue(coldLaunch.testing)
        XCTAssertFalse(coldLaunch.allowsSystemNotifications)
        XCTAssertFalse(coldLaunch.allowsSystemSpotlight)

        let cleanup = MobileAppModel.launchMode(
            hasIsolatedDirectory: false,
            arguments: ["Momentum", "--clear-system-app-intents-testing"]
        )
        XCTAssertTrue(cleanup.testing, "Cleanup must never open the production store")
    }

    func testSystemNotificationAcceptanceModeKeepsColdLaunchesIsolatedWithLiveNotifications() {
        let armed = MobileAppModel.launchMode(
            hasIsolatedDirectory: false,
            arguments: ["Momentum", "--system-notification-testing"]
        )
        XCTAssertTrue(armed.testing)
        XCTAssertFalse(armed.demo)
        XCTAssertTrue(armed.allowsSystemNotifications)
        XCTAssertFalse(armed.allowsSystemSpotlight)
        XCTAssertFalse(armed.allowsBackgroundNotificationRefresh)

        let coldLaunch = MobileAppModel.launchMode(
            hasIsolatedDirectory: false,
            arguments: ["Momentum"],
            hasSystemNotificationTestingMarker: true
        )
        XCTAssertTrue(coldLaunch.testing)
        XCTAssertTrue(coldLaunch.allowsSystemNotifications)
        XCTAssertFalse(coldLaunch.allowsSystemSpotlight)
        XCTAssertFalse(coldLaunch.allowsBackgroundNotificationRefresh)

        let cleanup = MobileAppModel.launchMode(
            hasIsolatedDirectory: false,
            arguments: ["Momentum", "--clear-system-notification-testing"]
        )
        XCTAssertTrue(cleanup.testing, "Cleanup must never open the production store")
        XCTAssertTrue(cleanup.allowsSystemNotifications)
        XCTAssertFalse(cleanup.allowsBackgroundNotificationRefresh)

        let production = MobileAppModel.launchMode(
            hasIsolatedDirectory: false,
            arguments: ["Momentum"]
        )
        XCTAssertTrue(production.allowsBackgroundNotificationRefresh)

        let preview = MobileAppModel.launchMode(
            hasIsolatedDirectory: false,
            arguments: ["Momentum", "--demo"]
        )
        XCTAssertFalse(preview.allowsBackgroundNotificationRefresh)
    }

    func testColdSearchKeyFallbackRoutesDirectlyWithoutWarmingAnotherCommand() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let name = "momentum-key-fallback-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name); try? FileManager.default.removeItem(at: directory) }
        let model = MobileAppModel(
            isolatedDirectory: directory,
            defaults: defaults,
            platformCapabilities: MobilePlatformCapabilities(isPad: true)
        )
        await model.start()

        let result = MobileKeyPressFallback.global(
            key: KeyEquivalent("f"), modifiers: .command,
            configuredModifier: .command, model: model
        )

        XCTAssertEqual(result, .handled)
        XCTAssertEqual(model.selectedTab, MobileTab.search.rawValue)
        XCTAssertEqual(model.searchFocusRevision, 1)
    }

    func testTaskKeyFallbackHonorsSelectionAndConfiguredDeleteModifier() {
        var actions: [MobileTaskShortcut] = []
        let context = MobileTaskCommandContext(
            view: .today, taskIDs: ["one"], selectedIDs: ["one"],
            selectedTitle: "One", singleTaskMenu: nil,
            enabled: [.selectAll, .deleteTask], reopensTask: false,
            perform: { actions.append($0) }
        )

        XCTAssertEqual(MobileKeyPressFallback.task(
            key: KeyEquivalent("a"), modifiers: .command,
            configuredModifier: .option, context: context,
            capabilities: MobilePlatformCapabilities(isPad: true)
        ), .handled)
        XCTAssertEqual(MobileKeyPressFallback.task(
            key: .delete, modifiers: .option,
            configuredModifier: .option, context: context,
            capabilities: MobilePlatformCapabilities(isPad: true)
        ), .handled)
        XCTAssertEqual(MobileKeyPressFallback.task(
            key: .delete, modifiers: .command,
            configuredModifier: .option, context: context,
            capabilities: MobilePlatformCapabilities(isPad: true)
        ), .ignored)
        XCTAssertEqual(actions, [.selectAll, .deleteTask])
    }

    func testBackgroundRefreshExecutionInstallsExpirationReschedulesAndCompletes() async {
        let probe = BackgroundRefreshExecutionProbe()
        await NotificationBackgroundRefresh.execute(
            installExpiration: { probe.install($0) },
            refresh: { true },
            shouldSchedule: { true },
            updateSchedule: { probe.schedule($0) },
            complete: { probe.complete($0) }
        )
        XCTAssertTrue(probe.hasExpiration)
        XCTAssertEqual(probe.scheduled, [true])
        XCTAssertEqual(probe.completions, [true])
    }

    func testBackgroundRefreshExpirationCancelsWorkAndReportsFailure() async {
        let probe = BackgroundRefreshExecutionProbe()
        let execution = Task { @MainActor in
            await NotificationBackgroundRefresh.execute(
                installExpiration: { probe.install($0) },
                refresh: {
                    while !Task.isCancelled { await Task.yield() }
                    return false
                },
                shouldSchedule: { true },
                updateSchedule: { probe.schedule($0) },
                complete: { probe.complete($0) }
            )
        }
        while !probe.hasExpiration { await Task.yield() }
        probe.expire()
        await execution.value
        XCTAssertEqual(probe.scheduled, [true])
        XCTAssertEqual(probe.completions, [false])
    }

    func testColdConcurrentEntryPointsShareWorkerAndPreserveSelectedTab() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let name = "momentum-automation-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name); try? FileManager.default.removeItem(at: directory) }
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
        model.selectedTab = 3
        let automation = MobileTaskAutomation(model: model)
        async let created = automation.create(title: "From shortcut", planning: .unscheduled)
        async let routed: Void = model.handle(url: URL(string: "momentum://add?title=From%20URL")!)
        let task = try await created
        await routed
        XCTAssertEqual(model.selectedTab, 3)
        let tasks = try await automation.find(includeCompleted: true)
        XCTAssertEqual(Set(tasks.map(\.title)), ["From shortcut", "From URL"])
        let resolved = try await automation.resolveTasks(ids: [task.id, "missing", task.id])
        XCTAssertEqual(resolved.map(\.id), [task.id])
        XCTAssertEqual(model.revision, 2)
        let worker = try XCTUnwrap(model.worker)
        await model.start()
        XCTAssertTrue(model.worker === worker)
        let changed = try await automation.setCompleted(ids: [task.id], completed: true)
        XCTAssertEqual(changed, 1)
        XCTAssertEqual(model.revision, 3)
    }

    func testOpenTaskWaitsForDraftAndDoesNotChangeTab() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let name = "momentum-automation-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name); try? FileManager.default.removeItem(at: directory) }
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
        let automation = MobileTaskAutomation(model: model)
        let task = try await automation.create(title: "Open me")
        model.selectedTab = 1
        model.showingAdd = true
        try await automation.openTask(id: task.id)
        XCTAssertNil(model.notificationTask)
        XCTAssertTrue(model.showingAdd)
        model.showingAdd = false
        model.presentPendingNotification()
        XCTAssertEqual(model.notificationTask?.id, task.id)
        XCTAssertEqual(model.selectedTab, 1)
        model.notificationTask = nil
        do { try await automation.openTask(id: "missing"); XCTFail("Expected stale entity rejection") }
        catch { XCTAssertEqual(error as? AutomationError, .taskUnavailable) }
        XCTAssertNil(model.notificationTask)
    }

    func testOpenTaskWaitsForKeyboardHelpToDismiss() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let name = "momentum-keyboard-help-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name); try? FileManager.default.removeItem(at: directory) }
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
        let automation = MobileTaskAutomation(model: model)
        let task = try await automation.create(title: "Open after help")
        model.showingKeyboardHelp = true
        try await automation.openTask(id: task.id)
        XCTAssertNil(model.notificationTask, "Do not present a second sheet over keyboard help")
        model.showingKeyboardHelp = false
        model.presentPendingNotification()
        XCTAssertEqual(model.notificationTask?.id, task.id, "The queued route must survive dismissal")
    }

    func testOpenTaskWaitsForProjectCreationToDismiss() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let name = "momentum-project-route-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name); try? FileManager.default.removeItem(at: directory) }
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
        let automation = MobileTaskAutomation(model: model)
        let task = try await automation.create(title: "Open after project")
        model.showingNewProject = true
        try await automation.openTask(id: task.id)
        XCTAssertNil(model.notificationTask)
        model.showingNewProject = false
        model.presentPendingNotification()
        XCTAssertEqual(model.notificationTask?.id, task.id)
    }

    func testSpotlightActivationUsesCurrentTaskTitleAndPreservesCaptureDraft() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let name = "momentum-spotlight-route-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name); try? FileManager.default.removeItem(at: directory) }
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
        let automation = MobileTaskAutomation(model: model)
        let task = try await automation.create(title: "Current searchable title")
        model.selectedTab = 0
        model.showingAdd = true

        let handled = await model.openSpotlightTask(task.id)
        XCTAssertTrue(handled)
        XCTAssertEqual(model.selectedTab, 2)
        XCTAssertEqual(model.systemSearchRequest?.taskID, task.id)
        XCTAssertEqual(model.systemSearchRequest?.title, "Current searchable title")
        XCTAssertTrue(model.showingAdd, "System search must not discard an in-progress capture")
    }

    func testStaleSpotlightActivationDoesNotChangePresentation() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let name = "momentum-stale-spotlight-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name); try? FileManager.default.removeItem(at: directory) }
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
        model.selectedTab = 1

        let handled = await model.openSpotlightTask("removed-task")
        XCTAssertFalse(handled)
        XCTAssertEqual(model.selectedTab, 1)
        XCTAssertNil(model.systemSearchRequest)
    }

    func testColdSpotlightPermanentlySuppressesStartupSkeletonWithoutReplay() async throws {
        let fixture = try await startupRouteFixture()
        defer { fixture.cleanup() }
        let task = try await MobileTaskAutomation(model: fixture.model).create(title: "Find from Spotlight")
        let opened = await fixture.model.openSpotlightTask(task.id)
        XCTAssertTrue(opened)
        let mounted = try mountStartupRoute(fixture)
        defer { mounted.unmount() }

        await mounted.settle()
        XCTAssertFalse(mounted.hasVisibleView(identifier: "today-loading-skeleton"))
        fixture.model.selectedTab = MobileTab.today.rawValue
        await mounted.settle()
        XCTAssertFalse(mounted.hasVisibleView(identifier: "today-loading-skeleton"))
        await fixture.gate.releaseAll()
    }

    func testColdReminderPermanentlySuppressesStartupSkeletonWithoutReplay() async throws {
        let fixture = try await startupRouteFixture()
        defer { fixture.cleanup() }
        let automation = MobileTaskAutomation(model: fixture.model)
        let task = try await automation.create(title: "Open from reminder")
        try await automation.openTask(id: task.id)
        XCTAssertEqual(fixture.model.notificationTask?.id, task.id)
        let mounted = try mountStartupRoute(fixture)
        defer { mounted.unmount() }

        await mounted.settle()
        XCTAssertFalse(mounted.hasVisibleView(identifier: "today-loading-skeleton"))
        fixture.model.notificationTask = nil
        fixture.model.selectedTab = MobileTab.today.rawValue
        await mounted.settle()
        XCTAssertFalse(mounted.hasVisibleView(identifier: "today-loading-skeleton"))
        await fixture.gate.releaseAll()
    }

    func testSummaryRouteKeepsTodayStartupEligible() async throws {
        let fixture = try await startupRouteFixture()
        defer { fixture.cleanup() }
        await fixture.model.handle(url: try XCTUnwrap(URL(string: "momentum://add?title=URL%20task")))
        let handled = await fixture.model.handleNotification(
            .summary(day: "2026-09-19"), action: nil, deliveryID: "summary"
        )
        XCTAssertTrue(handled)
        let mounted = try mountStartupRoute(fixture)
        defer { mounted.unmount() }

        await fixture.gate.waitForArrival(1)
        XCTAssertEqual(fixture.model.selectedTab, MobileTab.today.rawValue)
        XCTAssertTrue(mounted.hasVisibleView(identifier: "today-loading-skeleton"))
        await fixture.gate.releaseAll()
    }

    private func startupRouteFixture() async throws -> (
        model: MobileAppModel,
        gate: AutomationSnapshotGate,
        defaults: UserDefaults,
        defaultsName: String,
        cleanup: () -> Void
    ) {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let name = "momentum-startup-route-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
        await model.foreground()
        let gate = AutomationSnapshotGate()
        return (model, gate, defaults, name, {
            defaults.removePersistentDomain(forName: name)
            try? FileManager.default.removeItem(at: directory)
        })
    }

    private func mountStartupRoute(_ fixture: (
        model: MobileAppModel,
        gate: AutomationSnapshotGate,
        defaults: UserDefaults,
        defaultsName: String,
        cleanup: () -> Void
    )) throws -> HostingFixture.MountedView {
        try HostingFixture.mount(
            RootView(snapshotLoader: { worker, view, query, archiveLimit in
                await fixture.gate.wait()
                return await worker.snapshot(view: view, query: query, archiveLimit: archiveLimit)
            })
            .environment(fixture.model)
            .environment(fixture.model.sync),
            defaults: fixture.defaults,
            defaultsName: fixture.defaultsName
        )
    }

}

private actor AutomationSnapshotGate {
    private var arrivals = 0
    private var arrivalWaiters: [(Int, CheckedContinuation<Void, Never>)] = []
    private var pending: [CheckedContinuation<Void, Never>] = []

    func wait() async {
        arrivals += 1
        let ready = arrivalWaiters.filter { $0.0 <= arrivals }
        arrivalWaiters.removeAll { $0.0 <= arrivals }
        ready.forEach { $0.1.resume() }
        await withCheckedContinuation { pending.append($0) }
    }

    func waitForArrival(_ count: Int) async {
        guard arrivals < count else { return }
        await withCheckedContinuation { arrivalWaiters.append((count, $0)) }
    }

    func releaseAll() {
        let waiters = pending
        pending.removeAll()
        waiters.forEach { $0.resume() }
    }
}

private final class BackgroundRefreshExecutionProbe: @unchecked Sendable {
    private let lock = NSLock()
    private var expiration: (@Sendable () -> Void)?
    private var scheduledValues: [Bool] = []
    private var completionValues: [Bool] = []
    var hasExpiration: Bool { lock.withLock { expiration != nil } }
    var scheduled: [Bool] { lock.withLock { scheduledValues } }
    var completions: [Bool] { lock.withLock { completionValues } }
    func install(_ action: @escaping @Sendable () -> Void) { lock.withLock { expiration = action } }
    func expire() { lock.withLock { expiration }?() }
    func schedule(_ value: Bool) { lock.withLock { scheduledValues.append(value) } }
    func complete(_ value: Bool) { lock.withLock { completionValues.append(value) } }
}
