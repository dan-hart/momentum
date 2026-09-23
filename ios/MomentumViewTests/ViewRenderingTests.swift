// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore
import MomentumKit
import MomentumMobile
import Observation
import SwiftUI
import UIKit
import XCTest

@MainActor final class ViewRenderingTests: XCTestCase {
    func testStartupSkeletonMotionPolicyCapsAndPausesAnimation() {
        let animated = StartupSkeletonMotionPolicy(
            isComplete: false,
            reduceMotion: false,
            lowPowerMode: false,
            sceneIsActive: true
        )
        XCTAssertTrue(animated.shimmers)
        XCTAssertEqual(animated.minimumInterval, 1.0 / 30.0, accuracy: 0.000_001)
        XCTAssertEqual(animated.fadeDuration, 0.2, accuracy: 0.000_001)

        for policy in [
            StartupSkeletonMotionPolicy(isComplete: true, reduceMotion: false,
                                        lowPowerMode: false, sceneIsActive: true),
            StartupSkeletonMotionPolicy(isComplete: false, reduceMotion: true,
                                        lowPowerMode: false, sceneIsActive: true),
            StartupSkeletonMotionPolicy(isComplete: false, reduceMotion: false,
                                        lowPowerMode: true, sceneIsActive: true),
            StartupSkeletonMotionPolicy(isComplete: false, reduceMotion: false,
                                        lowPowerMode: false, sceneIsActive: false),
        ] {
            XCTAssertFalse(policy.shimmers)
        }
        XCTAssertEqual(
            StartupSkeletonMotionPolicy(isComplete: false, reduceMotion: true,
                                        lowPowerMode: false, sceneIsActive: true).fadeDuration,
            0
        )
    }

    func testTodaySkeletonRendersAdaptiveSystemPlaceholders() throws {
        let policy = StartupSkeletonMotionPolicy(
            isComplete: false,
            reduceMotion: false,
            lowPowerMode: false,
            sceneIsActive: true
        )
        let name = "momentum-skeleton-structure-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        let mounted = try HostingFixture.mount(
            TodaySkeletonView(motion: policy),
            defaults: defaults,
            defaultsName: name
        )
        defer { mounted.unmount() }

        XCTAssertTrue(mounted.containsView(ofType: UIScrollView.self))
        XCTAssertTrue(mounted.accessibilitySnapshot().isEmpty)
        XCTAssertGreaterThan(try XCTUnwrap(mounted.renderedImage().pngData()).count, 1_000)

        for scheme in [ColorScheme.light, .dark] {
            for contrast in [ColorSchemeContrast.standard, .increased] {
                let render = try HostingFixture.render(
                    TodaySkeletonView(motion: policy).frame(height: 700),
                    width: 390,
                    scheme: scheme,
                    contrast: contrast
                )
                XCTAssertEqual(render.size.width, 390, accuracy: 0.5)
                XCTAssertGreaterThan(try XCTUnwrap(render.image.pngData()).count, 1_000)
                if scheme == .light, contrast == .standard {
                    let attachment = XCTAttachment(image: render.image)
                    attachment.name = "Today skeleton component"
                    attachment.lifetime = .keepAlways
                    add(attachment)
                }
            }
        }
        let accessible = try HostingFixture.render(
            TodaySkeletonView(motion: policy).frame(height: 700),
            width: 320,
            size: .accessibility5
        )
        XCTAssertLessThanOrEqual(accessible.size.width, 320)
        XCTAssertGreaterThan(try XCTUnwrap(accessible.image.pngData()).count, 1_000)
    }

    func testRootStartupSkeletonOwnsLoadingAccessibilityAndKeepsRealTabsMounted() async throws {
        let fixture = try await rootStartupFixture()
        defer { fixture.cleanup(); fixture.mounted.unmount() }
        await fixture.gate.waitForArrival(1)
        await fixture.mounted.settle()

        XCTAssertTrue(fixture.mounted.hasVisibleView(identifier: "today-loading-skeleton"))
        XCTAssertEqual(fixture.mounted.tabBarItemTitles().count, MobileTab.allCases.count)
        XCTAssertEqual(fixture.mounted.accessibilityLabels(), ["Loading Momentum"])
        XCTAssertFalse(fixture.mounted.hasAccessibilityElement(label: "Add task"))
    }

    func testRootStartupSkeletonTransitionsOnceAfterFirstTodaySnapshot() async throws {
        let fixture = try await rootStartupFixture()
        defer { fixture.cleanup(); fixture.mounted.unmount() }
        await fixture.gate.waitForArrival(1)
        await fixture.mounted.settle()

        XCTAssertTrue(fixture.mounted.hasVisibleView(identifier: "today-loading-skeleton"))
        let loadingAttachment = XCTAttachment(image: fixture.mounted.renderedImage())
        loadingAttachment.name = "Momentum Today startup skeleton"
        loadingAttachment.lifetime = .keepAlways
        add(loadingAttachment)
        await fixture.gate.releaseNext()
        try await fixture.mounted.wait(until: "initial Today snapshot") {
            !fixture.mounted.hasVisibleView(identifier: "today-loading-skeleton")
                && fixture.mounted.hasAccessibilityElement(label: "Add task")
        }
        await fixture.mounted.settle()
        let readyAttachment = XCTAttachment(image: fixture.mounted.renderedImage())
        readyAttachment.name = "Momentum Today ready"
        readyAttachment.lifetime = .keepAlways
        add(readyAttachment)
        let readyButtonFrame = try XCTUnwrap(
            fixture.mounted.accessibilityElement(label: "Add task")?.frame
        )

        fixture.model.refreshAfterEdit()
        await fixture.gate.waitForArrival(2)
        XCTAssertFalse(fixture.mounted.hasVisibleView(identifier: "today-loading-skeleton"))
        XCTAssertEqual(fixture.mounted.accessibilityElement(label: "Add task")?.frame, readyButtonFrame)
        await fixture.gate.releaseNext()
    }

    func testNonTodayTaskScreensNeverUseStartupSkeleton() async throws {
        let views: [MomentumCore.View] = [
            .upcoming, .search, .archive, .project(id: "project"), .tag(id: "tag"),
        ]
        for view in views {
            let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
            let defaultsName = "momentum-non-today-loading-tests-\(UUID())"
            let defaults = try XCTUnwrap(UserDefaults(suiteName: defaultsName))
            let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
            await model.foreground()
            let gate = SnapshotDeliveryGate()
            let mounted = try HostingFixture.mount(
                NavigationStack {
                    TaskScreen(
                        view: view,
                        snapshotLoader: { worker, view, query, archiveLimit in
                            await gate.wait()
                            return await worker.snapshot(
                                view: view, query: query, archiveLimit: archiveLimit
                            )
                        }
                    )
                }
                .environment(model)
                .environment(model.sync),
                defaults: defaults,
                defaultsName: defaultsName
            )
            await gate.waitForArrival(1)
            XCTAssertFalse(mounted.hasVisibleView(identifier: "today-loading-skeleton"))
            XCTAssertTrue(
                mounted.accessibilityLabels().contains { $0.contains("Loading tasks") },
                "Mounted \(view) accessibility: \(mounted.accessibilitySnapshot())"
            )
            await gate.releaseNext()
            mounted.unmount()
            try? FileManager.default.removeItem(at: directory)
        }
    }

    func testInteractionMotionRespectsReduceMotion() async throws {
        let standard = InteractionMotionPolicy.presentation(reduceMotion: false)
        XCTAssertEqual(standard.duration, 0.18, accuracy: 0.001)
        XCTAssertEqual(standard.pressedScale, 0.96, accuracy: 0.001)
        XCTAssertTrue(standard.usesScaleTransition)
        XCTAssertTrue(standard.usesSymbolTransition)

        let reduced = InteractionMotionPolicy.presentation(reduceMotion: true)
        XCTAssertLessThanOrEqual(reduced.duration, 0.14)
        XCTAssertEqual(reduced.pressedScale, 1, accuracy: 0.001)
        XCTAssertFalse(reduced.usesScaleTransition)
        XCTAssertFalse(reduced.usesSymbolTransition)

        let addDefaultsName = "momentum-reduced-motion-add-tests-\(UUID())"
        let addDefaults = try XCTUnwrap(UserDefaults(suiteName: addDefaultsName))
        let add = try HostingFixture.mount(
            FloatingAddTaskButton(action: {})
                .environment(\._accessibilityReduceMotion, true),
            defaults: addDefaults,
            defaultsName: addDefaultsName,
            animationsEnabled: true
        )
        let addElement = try XCTUnwrap(add.accessibilityElement(label: "Add task"))
        XCTAssertFalse(addElement.frame.isEmpty)
        add.unmount()

        let accentDefaultsName = "momentum-reduced-motion-accent-tests-\(UUID())"
        let accentDefaults = try XCTUnwrap(UserDefaults(suiteName: accentDefaultsName))
        let choice = try XCTUnwrap(AccentPalette.all.first?.colors.first)
        let accentProbe = AccentSelectionProbe(selected: AccentChoice.momentum.id)
        let accentSelection = Binding(
            get: { accentProbe.selected },
            set: { accentProbe.selected = $0 }
        )
        let accents = try HostingFixture.mount(
            VStack {
                AccentChoiceRow(choice: .momentum, selected: accentSelection)
                AccentChoiceRow(choice: choice, selected: accentSelection)
            }
            .environment(\._accessibilityReduceMotion, true),
            defaults: accentDefaults,
            defaultsName: accentDefaultsName,
            animationsEnabled: true
        )
        let defaultLabel = "\(AccentChoice.momentum.name.localizedCapitalized), \(AccentChoice.momentum.hex)"
        let choiceLabel = "\(choice.name.localizedCapitalized), \(choice.hex)"
        XCTAssertTrue(accents.accessibilityElement(label: defaultLabel)?.traits.contains(.selected) == true)
        accentProbe.selected = choice.id
        try await accents.wait(until: "reduced-motion accent selection") {
            accents.accessibilityElement(label: choiceLabel)?.traits.contains(.selected) == true
                && accents.accessibilityElement(label: defaultLabel)?.traits.contains(.selected) == false
        }
        accents.unmount()

        let workspace = try await taskWorkspaceFixture(
            size: CGSize(width: 844, height: 500),
            horizontalSizeClass: .regular,
            reduceMotion: true
        )
        try await workspace.mounted.wait(until: "reduced-motion selected sidebar list") {
            workspace.mounted.accessibilityElement(label: "Today")?.traits.contains(.selected) == true
        }
        let today = try XCTUnwrap(workspace.mounted.accessibilityElement(label: "Today"))
        XCTAssertFalse(today.frame.isEmpty)
        XCTAssertGreaterThan(try XCTUnwrap(workspace.mounted.renderedImage().pngData()).count, 1_000)
        workspace.mounted.unmount()
        workspace.cleanup()

        let selection = try await taskScreenRowFixture(
            title: "Reduced motion selection",
            isPad: true,
            reduceMotion: true
        )
        defer { selection.mounted.unmount(); selection.cleanup() }
        try await selection.mounted.wait(until: "reduced-motion task row") {
            selection.mounted.hasAccessibilityElement(label: "Complete Reduced motion selection")
        }
        await selection.mounted.settle()
        XCTAssertTrue(selection.mounted.selectAllTasks())
        try await selection.mounted.wait(until: "reduced-motion selection controls") {
            selection.mounted.hasAccessibilityElement(label: "Actions (1)")
                && selection.mounted.hasAccessibilityTrait(.selected)
                && !selection.mounted.hasAccessibilityElement(label: "Complete Reduced motion selection")
        }
        XCTAssertFalse(try XCTUnwrap(selection.mounted.accessibilityElement(label: "Actions (1)"))
            .frame.isEmpty)
    }

    func testInitialTaskSnapshotDoesNotAnimate() async throws {
        let today = try await taskScreenSnapshotFixture(view: .today, reduceMotion: false)
        defer { today.mounted.unmount(); today.cleanup() }

        await today.gate.waitForArrival(1)
        let loadingFrame = try XCTUnwrap(addTaskButtonBounds(in: today.mounted.renderedImage()))
        await today.gate.releaseNext()
        let initial = await today.assignments.next()
        XCTAssertEqual(initial.transition, .immediate)
        XCTAssertTrue(initial.transaction.disablesAnimations)
        XCTAssertNil(initial.transaction.animation)
        let contentFrame = try XCTUnwrap(addTaskButtonBounds(in: today.mounted.renderedImage()))
        XCTAssertEqual(contentFrame.maxY, loadingFrame.maxY, accuracy: 0.5)
        XCTAssertEqual(contentFrame.height, loadingFrame.height, accuracy: 0.5)

        today.model.refreshAfterEdit()
        await today.gate.waitForArrival(2)
        await today.gate.releaseNext()
        let refresh = await today.assignments.next()
        XCTAssertEqual(refresh.transition, .smooth(duration: 0.2))
        XCTAssertFalse(refresh.transaction.disablesAnimations)
        XCTAssertNotNil(refresh.transaction.animation)

        for (view, reduceMotion) in [(MomentumCore.View.search, false), (.today, true)] {
            let fixture = try await taskScreenSnapshotFixture(view: view, reduceMotion: reduceMotion)
            defer { fixture.mounted.unmount(); fixture.cleanup() }
            await fixture.gate.waitForArrival(1)
            await fixture.gate.releaseNext()
            let first = await fixture.assignments.next()
            XCTAssertEqual(first.transition, .immediate)
            XCTAssertTrue(first.transaction.disablesAnimations)
            XCTAssertNil(first.transaction.animation)

            fixture.model.refreshAfterEdit()
            await fixture.gate.waitForArrival(2)
            await fixture.gate.releaseNext()
            let second = await fixture.assignments.next()
            XCTAssertEqual(second.transition, .immediate)
            XCTAssertTrue(second.transaction.disablesAnimations)
            XCTAssertNil(second.transaction.animation)
        }
    }

    func testTaskWorkspaceCollapsesAtPhonePortraitWidth() async throws {
        let fixture = try await taskWorkspaceFixture(
            size: CGSize(width: 390, height: 844),
            horizontalSizeClass: .compact
        )
        defer { fixture.mounted.unmount(); fixture.cleanup() }

        try await fixture.mounted.wait(until: "collapsed portrait split") {
            fixture.mounted.hasVisibleView(identifier: "task-workspace-detail")
        }
        XCTAssertFalse(fixture.mounted.hasVisibleView(identifier: "task-workspace-sidebar"))
    }

    func testCompactTaskWorkspaceUsesSidebarButtonInsteadOfBackButton() async throws {
        let fixture = try await taskWorkspaceFixture(
            size: CGSize(width: 390, height: 844),
            horizontalSizeClass: .compact
        )
        defer { fixture.mounted.unmount(); fixture.cleanup() }

        XCTAssertNotNil(UIImage(systemName: "sidebar.leading"))
        await fixture.mounted.settle()
        let leadingControls = fixture.mounted.visibleControls(
            in: CGRect(x: 0, y: 0, width: 100, height: 160)
        ).filter { $0.allControlEvents.contains(.primaryActionTriggered) }
        let sidebarButton = try XCTUnwrap(leadingControls.first)
        XCTAssertEqual(leadingControls.count, 1)
        XCTAssertFalse(fixture.mounted.hasAccessibilityElement(label: "Back"))
        let attachment = XCTAttachment(image: fixture.mounted.renderedImage())
        attachment.name = "Today compact sidebar button"
        attachment.lifetime = .keepAlways
        add(attachment)

        sidebarButton.sendActions(for: .primaryActionTriggered)
        try await fixture.mounted.wait(until: "sidebar revealed") {
            fixture.probe.state.preferredCompactColumn == .sidebar
                && fixture.mounted.hasVisibleView(identifier: "task-workspace-sidebar")
                && !fixture.mounted.hasVisibleView(identifier: "task-workspace-detail")
        }
    }

    func testCompactSidebarMorningSelectionLoadsItsTasks() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let defaultsName = "momentum-morning-navigation-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: defaultsName))
        let seed = await EngineWorker.open(directory: directory)
        _ = await seed.perform(.add("Morning focus #Morning", .today))
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
        await model.foreground()
        let probe = TaskWorkspaceProbe()
        let mounted = try HostingFixture.mount(
            TaskWorkspaceTestHost(probe: probe)
                .environment(model)
                .environment(model.sync),
            defaults: defaults,
            defaultsName: defaultsName,
            size: CGSize(width: 390, height: 844),
            horizontalSizeClass: .compact
        )
        defer {
            mounted.unmount()
            try? FileManager.default.removeItem(at: directory)
        }

        try await mounted.wait(until: "initial Today task") {
            mounted.hasAccessibilityElement(label: "Open Morning focus")
        }
        let leadingControls = mounted.visibleControls(
            in: CGRect(x: 0, y: 0, width: 100, height: 160)
        ).filter { $0.allControlEvents.contains(.primaryActionTriggered) }
        try XCTUnwrap(leadingControls.first).sendActions(for: .primaryActionTriggered)
        try await mounted.wait(until: "Morning sidebar destination") {
            mounted.hasAccessibilityElement(label: "Morning")
        }
        XCTAssertTrue(mounted.activateAccessibilityElement(label: "Morning"))
        try await mounted.wait(until: "Morning task detail") {
            probe.state.selection == .morning
                && mounted.hasVisibleView(identifier: "task-workspace-detail")
                && mounted.hasAccessibilityElement(label: "Open Morning focus")
                && !mounted.hasAccessibilityElement(label: "Loading tasks")
        }
    }

    func testMissingMorningSnapshotDoesNotRemainLoading() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let defaultsName = "momentum-missing-morning-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: defaultsName))
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
        await model.foreground()
        let assigned = expectation(description: "missing Morning snapshot assigned")
        let mounted = try HostingFixture.mount(
            NavigationStack {
                TaskScreen(
                    view: .morning,
                    dismissesWhenViewDisappears: false,
                    onSnapshotAssignment: { _, _ in assigned.fulfill() }
                )
            }
            .environment(model)
            .environment(model.sync),
            defaults: defaults,
            defaultsName: defaultsName
        )
        defer {
            mounted.unmount()
            try? FileManager.default.removeItem(at: directory)
        }

        await fulfillment(of: [assigned], timeout: 1)
        try await mounted.wait(until: "missing Morning loading completion") {
            !mounted.hasAccessibilityElement(label: "Loading tasks")
        }
    }

    func testTaskWorkspaceShowsBothColumnsAtCapableLandscapeWidth() async throws {
        let fixture = try await taskWorkspaceFixture(
            size: CGSize(width: 844, height: 390),
            horizontalSizeClass: .regular
        )
        defer { fixture.mounted.unmount(); fixture.cleanup() }

        try await fixture.mounted.wait(until: "landscape workspace columns") {
            fixture.mounted.hasVisibleView(identifier: "task-workspace-sidebar")
                && fixture.mounted.hasVisibleView(identifier: "task-workspace-detail")
        }
    }

    func testListsShowColoredFilledSymbolsAndUniqueTaskCounts() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let name = "momentum-list-count-render-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name); try? FileManager.default.removeItem(at: directory) }
        defaults.set(true, forKey: PrefKey.colorfulLabels)
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
        await model.foreground()
        let worker = try XCTUnwrap(model.worker)
        let createdProjectID = await worker.createProject("Color Project")
        let projectID = try XCTUnwrap(createdProjectID)
        let createdTagID = await worker.createTag("Color Tag")
        let tagID = try XCTUnwrap(createdTagID)
        _ = await worker.organize(.updateProject(projectID, "Color Project", "#FF6600"))
        _ = await worker.organize(.updateTag(tagID, "Color Tag", "#1ABC9C"))
        _ = await worker.perform(.add("Counted task", .today))
        let lastAddedTaskID = await worker.lastAddedTaskID()
        let taskID = try XCTUnwrap(lastAddedTaskID)
        _ = await worker.organize(.project([taskID], projectID))
        _ = await worker.organize(.tag([taskID], "Color Tag"))
        let snapshot = await worker.snapshot(view: .today)

        let mounted = try HostingFixture.mount(
            NavigationStack {
                ListPicker(snapshot: snapshot, selection: .constant(.today))
            }
            .environment(model),
            defaults: defaults,
            defaultsName: name
        )
        defer { mounted.unmount() }
        await mounted.settle()

        XCTAssertNotNil(UIImage(systemName: "folder.fill"))
        XCTAssertNotNil(UIImage(systemName: "tag.fill"))
        XCTAssertTrue(mounted.hasAccessibilityElement(label: "Color Project, 1 task"))
        XCTAssertTrue(mounted.hasAccessibilityElement(label: "Color Tag, 1 task"))
    }

    func testTaskWorkspaceCollapsesJustBelowCombinedMinimumWidth() async throws {
        let fixture = try await taskWorkspaceFixture(
            size: CGSize(width: 699, height: 500),
            horizontalSizeClass: .regular
        )
        defer { fixture.mounted.unmount(); fixture.cleanup() }

        try await fixture.mounted.wait(until: "detail below workspace width threshold") {
            fixture.mounted.hasVisibleView(identifier: "task-workspace-detail")
        }
        XCTAssertFalse(fixture.mounted.hasVisibleView(identifier: "task-workspace-sidebar"))
    }

    func testTaskWorkspaceShowsBothColumnsAtCombinedMinimumWidth() async throws {
        let fixture = try await taskWorkspaceFixture(
            size: CGSize(width: 700, height: 500),
            horizontalSizeClass: .regular
        )
        defer { fixture.mounted.unmount(); fixture.cleanup() }

        try await fixture.mounted.wait(until: "both columns at workspace width threshold") {
            fixture.mounted.hasVisibleView(identifier: "task-workspace-sidebar")
                && fixture.mounted.hasVisibleView(identifier: "task-workspace-detail")
        }
    }

    func testTaskWorkspaceSelectionPersistsAcrossTabSwitchAndRotation() async throws {
        let fixture = try await taskWorkspaceLifecycleFixture()
        defer { fixture.mounted.unmount(); fixture.cleanup() }

        try await fixture.mounted.wait(until: "compact archive detail") {
            fixture.mounted.hasVisibleView(identifier: "task-workspace-detail")
        }
        XCTAssertEqual(fixture.probe.workspace.selection, .archive)

        fixture.probe.selectedTab = 1
        try await fixture.mounted.wait(until: "independent tab selection") {
            fixture.mounted.hasVisibleView(identifier: "task-workspace-independent-tab")
        }
        XCTAssertEqual(fixture.probe.workspace.selection, .archive)

        fixture.probe.selectedTab = 0
        fixture.mounted.updateViewport(
            size: CGSize(width: 844, height: 390),
            horizontalSizeClass: .regular
        )
        try await fixture.mounted.wait(until: "wide workspace after tab return") {
            fixture.mounted.hasVisibleView(identifier: "task-workspace-sidebar")
                && fixture.mounted.hasVisibleView(identifier: "task-workspace-detail")
        }
        XCTAssertEqual(fixture.probe.workspace.selection, .archive)

        fixture.mounted.updateViewport(
            size: CGSize(width: 390, height: 844),
            horizontalSizeClass: .compact
        )
        try await fixture.mounted.wait(until: "compact workspace after rotation") {
            fixture.mounted.hasVisibleView(identifier: "task-workspace-detail")
                && !fixture.mounted.hasVisibleView(identifier: "task-workspace-sidebar")
        }
        XCTAssertEqual(fixture.probe.workspace.selection, .archive)
    }

    func testAccessibleErrorMessageWrapsAtLargestTextOnPhoneAndPad() throws {
        let message = String(localized: "The saved connection couldn’t be read. Unlock your device and try again. Your saved details have not been replaced.")
        for width: CGFloat in [320, 744] {
            let render = try HostingFixture.render(
                AccessibleErrorMessage(message),
                width: width,
                size: .accessibility5
            )
            XCTAssertLessThanOrEqual(render.size.width, width)
            XCTAssertGreaterThanOrEqual(render.size.height, 44)
            XCTAssertLessThan(render.size.height, 700)
        }
    }

    func testFourTabContractUsesRequestedOrderSymbolsAndStableIdentifiers() {
        XCTAssertEqual(MobileTab.allCases.map(\.titleKey), ["Today", "Upcoming", "Search", "Settings"])
        XCTAssertEqual(MobileTab.allCases.map(\.rawValue), [0, 1, 2, 3])
        XCTAssertEqual(MobileTab.allCases.map(\.symbol),
                       ["star.fill", "calendar", "magnifyingglass", "gearshape.fill"])
        XCTAssertEqual(MobileTab.allCases.map(\.accessibilityIdentifier),
                       ["tab-today", "tab-upcoming", "tab-search", "tab-settings"])
        XCTAssertEqual(Set(MobileTab.allCases.map(\.symbol)).count, MobileTab.allCases.count)
    }

    func testKeyboardSettingsAndHelpAreAvailableOnlyOnPad() async throws {
        let phone = try platformFixture(isPad: false)
        var mounted = try HostingFixture.mount(
            NavigationStack { SettingsScreen() }
                .environment(phone.model)
                .environment(phone.model.sync),
            defaults: phone.defaults,
            defaultsName: phone.defaultsName
        )
        await mounted.settle()
        XCTAssertFalse(mounted.hasAccessibilityElement(label: "Keyboard Shortcuts"))
        mounted.unmount()
        phone.cleanup()

        let pad = try platformFixture(isPad: true)
        mounted = try HostingFixture.mount(
            NavigationStack { SettingsScreen() }
                .environment(pad.model)
                .environment(pad.model.sync),
            defaults: pad.defaults,
            defaultsName: pad.defaultsName
        )
        try await mounted.wait(until: "iPad keyboard settings row") {
            mounted.hasAccessibilityElement(label: "Keyboard Shortcuts")
        }
        mounted.unmount()

        pad.model.showingKeyboardHelp = true
        mounted = try HostingFixture.mount(
            RootView()
                .environment(pad.model)
                .environment(pad.model.sync),
            defaults: pad.defaults,
            defaultsName: pad.defaultsName
        )
        try await mounted.wait(until: "iPad keyboard help") {
            !mounted.presentedControllers.isEmpty
        }
        mounted.unmount()
        pad.cleanup()

        let phoneHelp = try platformFixture(isPad: false)
        phoneHelp.model.showingKeyboardHelp = true
        mounted = try HostingFixture.mount(
            RootView()
                .environment(phoneHelp.model)
                .environment(phoneHelp.model.sync),
            defaults: phoneHelp.defaults,
            defaultsName: phoneHelp.defaultsName
        )
        await mounted.settle()
        XCTAssertTrue(mounted.presentedControllers.isEmpty)
        mounted.unmount()
        phoneHelp.cleanup()
    }

    func testKeyboardCommandsAndFallbackHandlersAreAvailableOnlyOnPad() async throws {
        let phone = try platformFixture(isPad: false)
        let pad = try platformFixture(isPad: true)
        defer { phone.cleanup(); pad.cleanup() }
        await phone.model.start()
        await pad.model.start()

        XCTAssertFalse(MobileKeyboardCommands(model: phone.model).includesMenus)
        XCTAssertTrue(MobileKeyboardCommands(model: pad.model).includesMenus)
        XCTAssertEqual(MobileKeyPressFallback.global(
            key: KeyEquivalent("f"), modifiers: .command,
            configuredModifier: .command, model: phone.model
        ), .ignored)
        XCTAssertEqual(MobileKeyPressFallback.global(
            key: KeyEquivalent("f"), modifiers: .command,
            configuredModifier: .command, model: pad.model
        ), .handled)

        var actions: [MobileTaskShortcut] = []
        let context = MobileTaskCommandContext(
            view: .today, taskIDs: ["one"], selectedIDs: ["one"],
            selectedTitle: "One", singleTaskMenu: nil,
            enabled: [.selectAll], reopensTask: false,
            perform: { actions.append($0) }
        )
        XCTAssertEqual(MobileKeyPressFallback.task(
            key: KeyEquivalent("a"), modifiers: .command,
            configuredModifier: .command, context: context,
            capabilities: phone.model.platformCapabilities
        ), .ignored)
        XCTAssertEqual(MobileKeyPressFallback.task(
            key: KeyEquivalent("a"), modifiers: .command,
            configuredModifier: .command, context: context,
            capabilities: pad.model.platformCapabilities
        ), .handled)
        XCTAssertEqual(actions, [.selectAll])
    }

    func testTaskSelectionResponderIsAvailableOnlyOnPad() async throws {
        let phone = try await taskScreenRowFixture(title: "Phone task", isPad: false)
        await phone.mounted.settle()
        XCTAssertFalse(phone.mounted.containsView(ofType: TaskSelectionKeyResponder.SelectionResponder.self))
        phone.mounted.unmount()
        phone.cleanup()

        let pad = try await taskScreenRowFixture(title: "Pad task", isPad: true)
        try await pad.mounted.wait(until: "iPad task selection responder") {
            pad.mounted.containsView(ofType: TaskSelectionKeyResponder.SelectionResponder.self)
        }
        pad.mounted.unmount()
        pad.cleanup()
    }

    func testExternalItemProvidersImportTextAndURLsThroughNativeTransferBoundary() async throws {
        let textProvider = NSItemProvider(object: "Plan the release" as NSString)
        let text = try await loadExternalTaskText(from: textProvider)
        XCTAssertEqual(text?.text, "Plan the release")

        let urlProvider = NSItemProvider(object: NSURL(string: "https://example.org/mobile-drop/")!)
        let url = try await loadExternalTaskText(from: urlProvider)
        XCTAssertEqual(url?.text, "https://example.org/mobile-drop/")
    }

    private func loadExternalTaskText(from provider: NSItemProvider) async throws -> ExternalTaskText? {
        try await withCheckedThrowingContinuation { continuation in
            provider.loadTransferable(type: ExternalTaskText.self) { result in
                continuation.resume(with: result)
            }
        }
    }

    func testFloatingAddFitsCompactWidthAndGrowsWithAccessibleText() throws {
        for scheme in [ColorScheme.light, .dark] {
            let button = FloatingAddTaskButton(action: {})
            let normal = try HostingFixture.render(button, width: 280, scheme: scheme)
            let large = try HostingFixture.render(button, width: 280, size: .accessibility5,
                                                  scheme: scheme, locale: "de")
            XCTAssertGreaterThanOrEqual(normal.size.height, 44)
            XCTAssertGreaterThan(large.size.height, normal.size.height)
            XCTAssertLessThan(large.size.height, normal.size.height * 2,
                              "An oversized translated label must yield to the compact symbol")
            XCTAssertLessThanOrEqual(large.size.width, 280)
            let attachment = XCTAttachment(image: normal.image)
            attachment.name = "Add task white foreground \(scheme)"
            attachment.lifetime = .keepAlways
            add(attachment)
        }
    }

    func testSettingsSubtitleAndLargeTextIncreaseRequiredHeight() throws {
        let title = SettingsLabel("Task Lists", symbol: "checklist")
        let subtitle = SettingsLabel("Task Lists", symbol: "checklist", subtitle: "Grouping, sorting, and completed tasks")
        let plain = try HostingFixture.render(title)
        let detailed = try HostingFixture.render(subtitle)
        let accessible = try HostingFixture.render(subtitle, size: .accessibility5)
        XCTAssertGreaterThan(detailed.size.height, plain.size.height)
        XCTAssertGreaterThan(accessible.size.height, detailed.size.height)
        XCTAssertLessThanOrEqual(accessible.size.width, 320)
    }

    func testLongSettingsLabelWrapsOnCompactPhone() throws {
        let label = SettingsLabel("Abgeschlossene Aufgaben sofort archivieren", symbol: "archivebox.fill")
        let wide = try HostingFixture.render(label, width: 744, locale: "de")
        let narrow = try HostingFixture.render(label, width: 280, locale: "de")
        let accessible = try HostingFixture.render(label, width: 744, size: .accessibility5, locale: "de")
        XCTAssertGreaterThan(narrow.size.height, wide.size.height)
        XCTAssertGreaterThan(accessible.size.height, wide.size.height)
        XCTAssertLessThanOrEqual(narrow.size.width, 280)
        XCTAssertLessThanOrEqual(accessible.size.width, 744)
        XCTAssertGreaterThan(try XCTUnwrap(accessible.image.pngData()).count, 1_000)
    }

    func testAccentSelectionCheckmarkFollowsBinding() async throws {
        let name = "momentum-accent-selection-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        let choice = try XCTUnwrap(AccentPalette.all.first?.colors.first)
        let probe = AccentSelectionProbe(selected: AccentChoice.momentum.id)
        let selected = Binding(
            get: { probe.selected },
            set: { probe.selected = $0 }
        )
        let mounted = try HostingFixture.mount(
            VStack {
                AccentChoiceRow(choice: .momentum, selected: selected)
                AccentChoiceRow(choice: choice, selected: selected)
            }
            .padding(),
            defaults: defaults,
            defaultsName: name
        )
        defer { mounted.unmount() }
        let defaultLabel = "\(AccentChoice.momentum.name.localizedCapitalized), \(AccentChoice.momentum.hex)"
        let choiceLabel = "\(choice.name.localizedCapitalized), \(choice.hex)"

        try await mounted.wait(until: "default accent selection") {
            mounted.accessibilityElement(label: defaultLabel)?.traits.contains(.selected) == true
        }
        XCTAssertFalse(mounted.accessibilityElement(label: choiceLabel)?.traits.contains(.selected) == true)

        probe.selected = choice.id

        try await mounted.wait(until: "updated accent selection") {
            mounted.accessibilityElement(label: choiceLabel)?.traits.contains(.selected) == true
                && mounted.accessibilityElement(label: defaultLabel)?.traits.contains(.selected) == false
        }
    }

    func testHapticFeedbackSettingUsesFullScopeLabelInEnglishAndGerman() async throws {
        let germanPath = try XCTUnwrap(Bundle.main.path(forResource: "de", ofType: "lproj"))
        let german = try XCTUnwrap(Bundle(path: germanPath))
        XCTAssertEqual(
            german.localizedString(forKey: "Haptic feedback", value: nil, table: nil),
            "Haptisches Feedback"
        )

        for (locale, expected) in [("en", "Haptic feedback"), ("de", "Haptisches Feedback")] {
            let name = "momentum-haptic-setting-label-tests-\(locale)-\(UUID())"
            let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
            let mounted = try HostingFixture.mount(
                NavigationStack { AppearanceSettings() }
                    .environment(\.locale, Locale(identifier: locale)),
                defaults: defaults,
                defaultsName: name
            )
            defer { mounted.unmount() }
            XCTAssertTrue(
                mounted.hasAccessibilityElement(label: expected),
                "Mounted \(locale) accessibility: \(mounted.accessibilitySnapshot())"
            )
            XCTAssertFalse(mounted.hasAccessibilityElement(label: "Completion haptics"))
        }
    }

    func testRealTaskRowScalesWithDynamicTypeAndUserTextPreference() async throws {
        let task = try await taskRow(title: "Review the release checklist with the team")
        let row = TaskRowContent(task: task, toggle: {}, open: {})
        let normal = try HostingFixture.render(row)
        let large = try HostingFixture.render(row, size: .accessibility5)
        let custom = try HostingFixture.render(row, contentScale: 1.5)
        XCTAssertGreaterThan(large.size.height, normal.size.height)
        XCTAssertGreaterThan(custom.size.height, normal.size.height)
        XCTAssertGreaterThanOrEqual(normal.size.height, 44)
        XCTAssertLessThanOrEqual(large.size.width, 320)
    }

    func testSelectionRowOmitsCompletionControl() async throws {
        let fixture = try await taskScreenRowFixture(title: "Choose release tasks", isPad: true)
        defer { fixture.mounted.unmount(); fixture.cleanup() }

        XCTAssertTrue(
            fixture.mounted.hasAccessibilityElement(label: "Complete Choose release tasks"),
            "Mounted accessibility: \(fixture.mounted.accessibilitySnapshot())"
        )
        XCTAssertTrue(fixture.mounted.selectAllTasks())
        try await fixture.mounted.wait(until: "selected task row") {
            !fixture.mounted.hasAccessibilityElement(label: "Complete Choose release tasks")
                && fixture.mounted.hasAccessibilityTrait(.selected)
                && fixture.mounted.hasAccessibilityElement(label: "Actions (1)")
        }
        XCTAssertFalse(fixture.mounted.hasAccessibilityElement(label: "Complete Choose release tasks"))
        XCTAssertTrue(
            fixture.mounted.hasAccessibilityElement(label: "Open Choose release tasks"),
            "Mounted accessibility: \(fixture.mounted.accessibilitySnapshot())"
        )
    }

    func testBrowsingRowKeepsCompletionControl() async throws {
        let fixture = try await taskScreenRowFixture(title: "Review release tasks", isPad: true)
        defer { fixture.mounted.unmount(); fixture.cleanup() }

        XCTAssertTrue(
            fixture.mounted.hasAccessibilityElement(label: "Complete Review release tasks"),
            "Mounted accessibility: \(fixture.mounted.accessibilitySnapshot())"
        )
        XCTAssertTrue(
            fixture.mounted.hasAccessibilityElement(label: "Open Review release tasks"),
            "Mounted accessibility: \(fixture.mounted.accessibilitySnapshot())"
        )
        XCTAssertFalse(fixture.mounted.hasAccessibilityTrait(.selected))
    }

    func testMountedAccessibilityTraversalUsesReachableVisibleOrder() throws {
        let defaultsName = "momentum-accessibility-traversal-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: defaultsName))
        let mounted = try HostingFixture.mount(
            AccessibilityTraversalProbe(),
            defaults: defaults,
            defaultsName: defaultsName
        )
        defer { mounted.unmount() }

        XCTAssertEqual(
            mounted.accessibilityLabels().filter { $0.hasPrefix("Traversal ") },
            ["Traversal second", "Traversal first"]
        )
    }

    func testTaskRowWrapsLongTitleRatherThanGrowingPastContainer() async throws {
        let task = try await taskRow(title: "Review the release checklist with the entire team before sending the update")
        let row = TaskRowContent(task: task, toggle: {}, open: {})
        let wide = try HostingFixture.render(row, width: 744)
        let narrow = try HostingFixture.render(row, width: 280)
        XCTAssertGreaterThan(narrow.size.height, wide.size.height)
        XCTAssertLessThanOrEqual(narrow.size.width, 280)
    }

    func testCompletionChangesRenderedRowInBothAppearances() async throws {
        var task = try await taskRow(title: "Completed task")
        let unfinishedLight = try HostingFixture.render(TaskRowContent(task: task, toggle: {}, open: {}))
        let unfinishedDark = try HostingFixture.render(TaskRowContent(task: task, toggle: {}, open: {}), scheme: .dark)
        task.isDone = true
        let row = TaskRowContent(task: task, toggle: {}, open: {})
        let light = try HostingFixture.render(row)
        let dark = try HostingFixture.render(row, scheme: .dark)
        XCTAssertEqual(light.size, dark.size)
        XCTAssertNotEqual(light.image.pngData(), dark.image.pngData())
        XCTAssertNotEqual(light.image.pngData(), unfinishedLight.image.pngData())
        XCTAssertNotEqual(dark.image.pngData(), unfinishedDark.image.pngData())
        XCTAssertGreaterThan(try XCTUnwrap(light.image.pngData()).count, 1000)
    }

    func testDefaultAccentAndCommitInkResolveExactlyUnderNativeTraits() {
        for style in [UIUserInterfaceStyle.light, .dark] {
            for contrast in [UIAccessibilityContrast.normal, .high] {
                let traits = UITraitCollection(traitsFrom: [
                    UITraitCollection(userInterfaceStyle: style),
                    UITraitCollection(accessibilityContrast: contrast)
                ])
                let accent = UIColor(AccentTheme.accent(.momentum)).resolvedColor(with: traits)
                var r: CGFloat = 0, g: CGFloat = 0, b: CGFloat = 0, a: CGFloat = 0
                XCTAssertTrue(accent.getRed(&r, green: &g, blue: &b, alpha: &a))
                XCTAssertEqual(r, 1, accuracy: 0.001)
                XCTAssertEqual(g, 0.4, accuracy: 0.001)
                XCTAssertEqual(b, 0, accuracy: 0.001)
                let ink = UIColor(AccentTheme.onAccent(.momentum)).resolvedColor(with: traits)
                XCTAssertTrue(ink.getRed(&r, green: &g, blue: &b, alpha: &a))
                XCTAssertEqual(r + g + b, contrast == .high ? 0 : 3, accuracy: 0.001)
            }
        }
    }

    func testErrorInkMeetsTextContrastAcrossNativeSurfaces() throws {
        let surfaces: [UIColor] = [.systemBackground, .secondarySystemBackground,
            .tertiarySystemBackground, .systemGroupedBackground,
            .secondarySystemGroupedBackground, .tertiarySystemGroupedBackground]
        func rgb(_ color: UIColor, _ traits: UITraitCollection) throws -> RGBColor {
            var r: CGFloat = 0, g: CGFloat = 0, b: CGFloat = 0, a: CGFloat = 0
            XCTAssertTrue(color.resolvedColor(with: traits).getRed(&r, green: &g, blue: &b, alpha: &a))
            XCTAssertEqual(a, 1, accuracy: 0.001)
            return RGBColor(red: r, green: g, blue: b)
        }
        for style in [UIUserInterfaceStyle.light, .dark] {
            for contrast in [UIAccessibilityContrast.normal, .high] {
                for level in [UIUserInterfaceLevel.base, .elevated] {
                    let traits = UITraitCollection(traitsFrom: [UITraitCollection(userInterfaceStyle: style),
                        UITraitCollection(accessibilityContrast: contrast), UITraitCollection(userInterfaceLevel: level)])
                    let foreground = try rgb(UIColor(AccentTheme.errorText), traits)
                    for surface in surfaces {
                        XCTAssertGreaterThanOrEqual(foreground.contrast(with: try rgb(surface, traits)),
                            contrast == .high ? 7 : 4.5,
                            "Error text must remain readable in ordinary and elevated native forms")
                    }
                }
            }
        }
    }

    func testSecondaryInkKeepsNativeRenderingMarginAcrossSurfaces() throws {
        let surfaces: [UIColor] = [.systemBackground, .secondarySystemBackground,
            .tertiarySystemBackground, .systemGroupedBackground,
            .secondarySystemGroupedBackground, .tertiarySystemGroupedBackground]
        func rgb(_ color: UIColor, _ traits: UITraitCollection) throws -> RGBColor {
            var r: CGFloat = 0, g: CGFloat = 0, b: CGFloat = 0, a: CGFloat = 0
            XCTAssertTrue(color.resolvedColor(with: traits).getRed(&r, green: &g, blue: &b, alpha: &a))
            return RGBColor(red: r, green: g, blue: b)
        }
        for style in [UIUserInterfaceStyle.light, .dark] {
            for contrast in [UIAccessibilityContrast.normal, .high] {
                let traits = UITraitCollection(traitsFrom: [UITraitCollection(userInterfaceStyle: style),
                    UITraitCollection(accessibilityContrast: contrast)])
                let foreground = try rgb(UIColor(AccentTheme.secondaryText), traits)
                for surface in surfaces {
                    XCTAssertGreaterThanOrEqual(foreground.contrast(with: try rgb(surface, traits)),
                        contrast == .high ? 7.05 : 5.5)
                }
            }
        }
    }

    func testCreateAndSaveSymbolsProduceDifferentRenderedContent() throws {
        let create = try HostingFixture.render(SheetCommitButton(title: "Create", symbol: "plus", action: {}))
        let save = try HostingFixture.render(SheetCommitButton(title: "Save", symbol: "checkmark", action: {}))
        XCTAssertNotEqual(create.image.pngData(), save.image.pngData())
        XCTAssertGreaterThan(try XCTUnwrap(create.image.pngData()).count, 500)
    }

    func testCommitSymbolRendersWhiteInsideOrangeFill() throws {
        for scheme in [ColorScheme.light, .dark] {
            let rendered = try HostingFixture.render(
                SheetCommitButton(title: "Save", symbol: "checkmark", action: {}), scheme: scheme)
            let image = try XCTUnwrap(rendered.image.cgImage)
            let width = image.width, height = image.height
            var bytes = [UInt8](repeating: 0, count: width * height * 4)
            try bytes.withUnsafeMutableBytes { buffer in
                let context = try XCTUnwrap(CGContext(data: buffer.baseAddress, width: width, height: height,
                    bitsPerComponent: 8, bytesPerRow: width * 4, space: CGColorSpaceCreateDeviceRGB(),
                    bitmapInfo: CGBitmapInfo.byteOrder32Big.rawValue | CGImageAlphaInfo.premultipliedLast.rawValue))
                context.draw(image, in: CGRect(x: 0, y: 0, width: width, height: height))
            }
            var orange: [(Int, Int)] = []
            for y in 0..<height {
                for x in 0..<width {
                    let i = (y * width + x) * 4
                    if bytes[i] > 220 && (80...130).contains(bytes[i + 1]) && bytes[i + 2] < 40 {
                        orange.append((x, y))
                    }
                }
            }
            XCTAssertGreaterThan(orange.count, 100, "The real button must paint an orange fill")
            let minX = try XCTUnwrap(orange.map(\.0).min()), maxX = try XCTUnwrap(orange.map(\.0).max())
            let minY = try XCTUnwrap(orange.map(\.1).min()), maxY = try XCTUnwrap(orange.map(\.1).max())
            var whiteInk = 0
            // The symbol's optical bounds differ between iOS releases. Keep the
            // scan inside the capsule's solid middle while covering nearly its
            // full width so an off-center SF Symbol is still measured.
            let horizontalInset = max(2, (maxX - minX) / 20)
            let verticalInset = max(2, (maxY - minY) / 5)
            for y in (minY + verticalInset)...(maxY - verticalInset) {
                for x in (minX + horizontalInset)...(maxX - horizontalInset) {
                    let i = (y * width + x) * 4
                    if bytes[i] > 230 && bytes[i + 1] > 230 && bytes[i + 2] > 230 { whiteInk += 1 }
                }
            }
            XCTAssertGreaterThan(whiteInk, 5, "The rendered checkmark must use white ink in \(scheme)")
        }
    }

    private func taskRow(title: String) async throws -> TaskRow {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: directory) }
        let worker = await EngineWorker.open(directory: directory)
        _ = await worker.perform(.add(title, .today))
        let snapshot = await worker.snapshot(view: .today)
        return try XCTUnwrap(snapshot.tasks.first)
    }

    private func rootStartupFixture() async throws -> (
        mounted: HostingFixture.MountedView,
        model: MobileAppModel,
        gate: SnapshotDeliveryGate,
        cleanup: () -> Void
    ) {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let defaultsName = "momentum-root-startup-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: defaultsName))
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
        await model.foreground()
        let gate = SnapshotDeliveryGate()
        let mounted = try HostingFixture.mount(
            RootView(snapshotLoader: { worker, view, query, archiveLimit in
                await gate.wait()
                return await worker.snapshot(view: view, query: query, archiveLimit: archiveLimit)
            })
            .environment(model)
            .environment(model.sync),
            defaults: defaults,
            defaultsName: defaultsName,
            animationsEnabled: true
        )
        return (mounted, model, gate, {
            defaults.removePersistentDomain(forName: defaultsName)
            try? FileManager.default.removeItem(at: directory)
        })
    }

    private func taskScreenSnapshotFixture(
        view: MomentumCore.View,
        reduceMotion: Bool
    ) async throws -> (
        mounted: HostingFixture.MountedView,
        model: MobileAppModel,
        gate: SnapshotDeliveryGate,
        assignments: SnapshotAssignmentRecorder,
        cleanup: () -> Void
    ) {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let defaultsName = "momentum-snapshot-transaction-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: defaultsName))
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
        await model.foreground()
        let gate = SnapshotDeliveryGate()
        let assignments = SnapshotAssignmentRecorder()
        let mounted = try HostingFixture.mount(
            TaskScreen(
                view: view,
                snapshotLoader: { worker, view, query, archiveLimit in
                    await gate.wait()
                    return await worker.snapshot(view: view, query: query, archiveLimit: archiveLimit)
                },
                onSnapshotAssignment: { transition, transaction in
                    assignments.record(transition: transition, transaction: transaction)
                }
            )
            .environment(model)
            .environment(model.sync)
            .environment(\._accessibilityReduceMotion, reduceMotion),
            defaults: defaults,
            defaultsName: defaultsName,
            animationsEnabled: true
        )
        return (mounted, model, gate, assignments, {
            try? FileManager.default.removeItem(at: directory)
        })
    }

    private func taskScreenRowFixture(title: String, isPad: Bool, reduceMotion: Bool = false) async throws -> (
        mounted: HostingFixture.MountedView,
        cleanup: () -> Void
    ) {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let defaultsName = "momentum-task-screen-row-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: defaultsName))
        let seed = await EngineWorker.open(directory: directory)
        _ = await seed.perform(.add(title, .today))
        let model = MobileAppModel(
            isolatedDirectory: directory,
            defaults: defaults,
            platformCapabilities: MobilePlatformCapabilities(isPad: isPad)
        )
        await model.foreground()
        let gate = SnapshotDeliveryGate()
        let assignments = SnapshotAssignmentRecorder()
        let mounted = try HostingFixture.mount(
            NavigationStack {
                TaskScreen(
                    view: .today,
                    snapshotLoader: { worker, view, query, archiveLimit in
                        await gate.wait()
                        return await worker.snapshot(view: view, query: query, archiveLimit: archiveLimit)
                    },
                    onSnapshotAssignment: { transition, transaction in
                        assignments.record(transition: transition, transaction: transaction)
                    }
                )
            }
            .environment(model)
            .environment(model.sync)
            .environment(\._accessibilityReduceMotion, reduceMotion),
            defaults: defaults,
            defaultsName: defaultsName
        )
        await gate.waitForArrival(1)
        await gate.releaseNext()
        _ = await assignments.next()
        return (mounted, { try? FileManager.default.removeItem(at: directory) })
    }

    private func platformFixture(isPad: Bool) throws -> (
        model: MobileAppModel,
        defaults: UserDefaults,
        defaultsName: String,
        cleanup: () -> Void
    ) {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let defaultsName = "momentum-platform-capability-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: defaultsName))
        let model = MobileAppModel(
            isolatedDirectory: directory,
            defaults: defaults,
            platformCapabilities: MobilePlatformCapabilities(isPad: isPad)
        )
        return (model, defaults, defaultsName, {
            defaults.removePersistentDomain(forName: defaultsName)
            try? FileManager.default.removeItem(at: directory)
        })
    }

    private func addTaskButtonBounds(in image: UIImage) -> CGRect? {
        guard let cgImage = image.cgImage else { return nil }
        let width = cgImage.width
        let height = cgImage.height
        var bytes = [UInt8](repeating: 0, count: width * height * 4)
        let drew = bytes.withUnsafeMutableBytes { buffer in
            guard let context = CGContext(
                data: buffer.baseAddress,
                width: width,
                height: height,
                bitsPerComponent: 8,
                bytesPerRow: width * 4,
                space: CGColorSpaceCreateDeviceRGB(),
                bitmapInfo: CGBitmapInfo.byteOrder32Big.rawValue | CGImageAlphaInfo.premultipliedLast.rawValue
            ) else { return false }
            context.draw(cgImage, in: CGRect(x: 0, y: 0, width: width, height: height))
            return true
        }
        guard drew else { return nil }
        var minX = width, maxX = -1, minY = height, maxY = -1
        for y in (height / 2)..<height {
            for x in (width / 2)..<width {
                let index = (y * width + x) * 4
                if bytes[index] > 220, (75...135).contains(bytes[index + 1]), bytes[index + 2] < 45 {
                    minX = min(minX, x)
                    maxX = max(maxX, x)
                    minY = min(minY, y)
                    maxY = max(maxY, y)
                }
            }
        }
        guard maxX >= minX, maxY >= minY else { return nil }
        return CGRect(x: minX, y: minY, width: maxX - minX + 1, height: maxY - minY + 1)
    }

    private func taskWorkspaceFixture(
        size: CGSize,
        horizontalSizeClass: UIUserInterfaceSizeClass,
        reduceMotion: Bool = false
    ) async throws -> (
        mounted: HostingFixture.MountedView,
        probe: TaskWorkspaceProbe,
        cleanup: () -> Void
    ) {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let defaultsName = "momentum-workspace-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: defaultsName))
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
        await model.foreground()
        let probe = TaskWorkspaceProbe()
        let mounted = try HostingFixture.mount(
            TaskWorkspaceTestHost(probe: probe)
                .environment(model)
                .environment(model.sync)
                .environment(\._accessibilityReduceMotion, reduceMotion),
            defaults: defaults,
            defaultsName: defaultsName,
            size: size,
            horizontalSizeClass: horizontalSizeClass
        )
        return (mounted, probe, {
            try? FileManager.default.removeItem(at: directory)
        })
    }

    private func taskWorkspaceLifecycleFixture() async throws -> (
        mounted: HostingFixture.MountedView,
        probe: TaskWorkspaceLifecycleProbe,
        cleanup: () -> Void
    ) {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let defaultsName = "momentum-workspace-lifecycle-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: defaultsName))
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
        await model.foreground()
        let probe = TaskWorkspaceLifecycleProbe()
        let mounted = try HostingFixture.mount(
            TaskWorkspaceLifecycleHost(probe: probe)
                .environment(model)
                .environment(model.sync),
            defaults: defaults,
            defaultsName: defaultsName,
            size: CGSize(width: 390, height: 844),
            horizontalSizeClass: .compact
        )
        return (mounted, probe, {
            try? FileManager.default.removeItem(at: directory)
        })
    }
}

private actor SnapshotDeliveryGate {
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

    func releaseNext() {
        pending.removeFirst().resume()
    }
}

@MainActor private final class SnapshotAssignmentRecorder {
    struct Record {
        let transition: TaskSnapshotTransition
        let transaction: Transaction
    }

    private var records: [Record] = []
    private var cursor = 0
    private var waiter: CheckedContinuation<Void, Never>?

    func record(transition: TaskSnapshotTransition, transaction: Transaction) {
        let record = Record(transition: transition, transaction: transaction)
        records.append(record)
        if let waiter {
            self.waiter = nil
            waiter.resume()
        }
    }

    func next() async -> Record {
        if cursor >= records.count {
            await withCheckedContinuation { waiter = $0 }
        }
        defer { cursor += 1 }
        return records[cursor]
    }
}

@MainActor @Observable private final class TaskWorkspaceProbe {
    var state = TaskWorkspaceState()
}

private struct TaskWorkspaceTestHost: SwiftUI.View {
    @Bindable var probe: TaskWorkspaceProbe

    var body: some SwiftUI.View {
        TaskWorkspace(state: $probe.state)
    }
}

@MainActor @Observable private final class AccentSelectionProbe {
    var selected: String

    init(selected: String) {
        self.selected = selected
    }
}

@MainActor @Observable private final class TaskWorkspaceLifecycleProbe {
    var selectedTab = 0
    var workspace = TaskWorkspaceState(selection: .archive)
}

private struct TaskWorkspaceLifecycleHost: SwiftUI.View {
    @Bindable var probe: TaskWorkspaceLifecycleProbe

    var body: some SwiftUI.View {
        TabView(selection: $probe.selectedTab) {
            Tab("Today", systemImage: "star.fill", value: 0) {
                TaskWorkspace(state: $probe.workspace)
            }
            Tab("Other", systemImage: "gearshape.fill", value: 1) {
                NavigationStack { Text("Independent tab") }
                    .background(HostedVisibilityMarker(identifier: "task-workspace-independent-tab"))
            }
        }
    }
}

private struct HostedVisibilityMarker: UIViewRepresentable {
    let identifier: String

    func makeUIView(context: Context) -> UIView {
        let view = UIView()
        view.backgroundColor = .clear
        view.isUserInteractionEnabled = false
        view.accessibilityIdentifier = identifier
        return view
    }

    func updateUIView(_ view: UIView, context: Context) {
        view.accessibilityIdentifier = identifier
    }
}

private struct AccessibilityTraversalProbe: UIViewRepresentable {
    func makeUIView(context: Context) -> UIView {
        let root = UIView()
        let first = accessibleLabel("Traversal first", frame: CGRect(x: 16, y: 72, width: 180, height: 44))
        let second = accessibleLabel("Traversal second", frame: CGRect(x: 16, y: 16, width: 180, height: 44))
        let hidden = accessibleLabel("Traversal hidden", frame: CGRect(x: 16, y: 128, width: 180, height: 44))
        hidden.isHidden = true
        let transparent = accessibleLabel("Traversal transparent", frame: CGRect(x: 16, y: 184, width: 180, height: 44))
        transparent.alpha = 0
        let hiddenContainer = UIView(frame: CGRect(x: 16, y: 240, width: 180, height: 44))
        hiddenContainer.accessibilityElementsHidden = true
        let hiddenChild = accessibleLabel("Traversal hidden child", frame: hiddenContainer.bounds)
        hiddenContainer.addSubview(hiddenChild)
        let offscreen = accessibleLabel("Traversal offscreen", frame: CGRect(x: 16, y: 2_000, width: 180, height: 44))

        [first, second, hidden, transparent, hiddenContainer, offscreen].forEach(root.addSubview)
        root.accessibilityElements = [second, first, hidden, transparent, hiddenContainer, offscreen]
        return root
    }

    func updateUIView(_ view: UIView, context: Context) {}

    private func accessibleLabel(_ text: String, frame: CGRect) -> UILabel {
        let label = UILabel(frame: frame)
        label.text = text
        label.accessibilityLabel = text
        label.isAccessibilityElement = true
        return label
    }
}
