// SPDX-License-Identifier: GPL-3.0-or-later
import Accessibility
import Foundation
import MomentumCore
import MomentumMobile
import SwiftUI
import XCTest

@MainActor final class TaskMutationIntegrationTests: XCTestCase {
    func testFeedbackToastTimesOutDeterministicallyAndSaveFailurePersists() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let name = "momentum-toast-timeout-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name); try? FileManager.default.removeItem(at: directory) }
        let delay = FeedbackDelay()
        let model = MobileAppModel(
            isolatedDirectory: directory,
            defaults: defaults,
            feedbackSleep: { try await delay.sleep($0) }
        )

        model.accept(Outcome(changed: false, message: .taskDeleted, undo: 42, syncNow: false))
        await delay.waitForCount(1)
        let duration = await delay.lastDuration
        XCTAssertEqual(duration, .seconds(5))
        XCTAssertEqual(model.feedbackToast?.undoBatchID, 42)
        await delay.advanceLatest()
        for _ in 0..<10 where model.feedbackToast != nil { await Task.yield() }
        XCTAssertNil(model.feedbackToast)

        model.accept(Outcome(changed: false, message: .saveFailed(error: "fixture"), undo: nil, syncNow: false))
        XCTAssertEqual(model.feedbackToast?.kind, .error)
        XCTAssertTrue(try XCTUnwrap(model.feedbackToast).isPersistent)
        let delayCount = await delay.count
        XCTAssertEqual(delayCount, 1, "Persistent errors do not start an auto-dismiss timer")
    }

    func testFeedbackUndoTargetsTheExactMutationBatch() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let name = "momentum-toast-undo-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name); try? FileManager.default.removeItem(at: directory) }
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
        await model.foreground()
        _ = await model.perform(.add("Undo toast fixture", .today))
        let worker = try XCTUnwrap(model.worker)
        let created = await worker.snapshot(view: .today)
        let taskID = try XCTUnwrap(created.tasks.first?.id)
        _ = await model.perform(.delete([taskID]))
        XCTAssertNotNil(model.feedbackToast?.undoBatchID)

        await model.undoFeedback()

        let restored = await worker.snapshot(view: .today)
        XCTAssertTrue(restored.tasks.contains { $0.id == taskID })
        XCTAssertEqual(model.feedback, "Undone")
    }

    func testInteractionFeedbackTriggers() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let name = "momentum-interaction-feedback-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name); try? FileManager.default.removeItem(at: directory) }
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
        await model.foreground()

        XCTAssertEqual(model.selectionFeedback, 0)
        XCTAssertEqual(model.interactionSuccessFeedback, 0)

        model.add(in: .today)
        XCTAssertEqual(model.selectionFeedback, 1)
        model.add(in: .today)
        XCTAssertEqual(model.selectionFeedback, 1, "An already-open Add Task sheet is unchanged")
        model.showingAdd = false

        model.sidebarSelectionDidChange(from: .today, to: .today)
        XCTAssertEqual(model.selectionFeedback, 1)
        model.sidebarSelectionDidChange(from: .today, to: .archive)
        XCTAssertEqual(model.selectionFeedback, 2)

        model.accentSelectionDidChange(from: AccentChoice.momentum.id, to: AccentChoice.momentum.id)
        XCTAssertEqual(model.selectionFeedback, 2)
        model.accentSelectionDidChange(from: AccentChoice.momentum.id, to: AccentPalette.all[0].colors[0].id)
        XCTAssertEqual(model.selectionFeedback, 3)

        let createdResult = await model.createTaskFromText("Feedback task", view: .today)
        let created = try XCTUnwrap(createdResult)
        let taskID = try XCTUnwrap(created.createdTaskID)
        XCTAssertEqual(model.interactionSuccessFeedback, 1)

        let blocked = directory.appendingPathComponent("pending.json.tmp")
        try FileManager.default.createDirectory(at: blocked, withIntermediateDirectories: false)
        let failedResult = await model.createTaskFromText("Must not buzz", view: .today)
        let failed = try XCTUnwrap(failedResult)
        XCTAssertFalse(failed.outcome.changed)
        XCTAssertEqual(model.interactionSuccessFeedback, 1)
        try FileManager.default.removeItem(at: blocked)

        let movedResult = await model.organize(.tomorrow([taskID]))
        let moved = try XCTUnwrap(movedResult)
        XCTAssertTrue(moved.changed)
        XCTAssertEqual(model.interactionSuccessFeedback, 2)
        let unchangedResult = await model.organize(.tomorrow([taskID]))
        let unchanged = try XCTUnwrap(unchangedResult)
        XCTAssertFalse(unchanged.changed)
        XCTAssertEqual(model.interactionSuccessFeedback, 2)

        XCTAssertTrue(InteractionFeedbackPolicy.shouldPresent(enabled: true, oldValue: 1, newValue: 2))
        XCTAssertFalse(InteractionFeedbackPolicy.shouldPresent(enabled: false, oldValue: 1, newValue: 2))
        XCTAssertFalse(InteractionFeedbackPolicy.shouldPresent(enabled: true, oldValue: 2, newValue: 2))
    }

    func testTaskScreenSelectionFeedbackUsesProductionCommands() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let name = "momentum-selection-feedback-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name); try? FileManager.default.removeItem(at: directory) }
        let model = MobileAppModel(
            isolatedDirectory: directory,
            defaults: defaults,
            platformCapabilities: MobilePlatformCapabilities(isPad: true)
        )
        await model.foreground()
        _ = await model.createTaskFromText("Keyboard selection", view: .today)

        let commandProbe = TaskCommandProbe()
        let mounted = try HostingFixture.mount(
            TaskCommandCapture(probe: commandProbe)
            .environment(model)
            .environment(model.sync),
            defaults: defaults,
            defaultsName: name
        )
        defer { mounted.unmount() }
        try await mounted.wait(until: "task selection commands") {
            commandProbe.commands?.enabled.contains(.selectAll) == true
        }

        let initial = model.selectionFeedback
        let selected = MobileKeyPressFallback.task(
            key: KeyEquivalent("a"), modifiers: .command, configuredModifier: .command,
            context: commandProbe.commands, capabilities: model.platformCapabilities
        )
        XCTAssertEqual(selected, .handled)
        try await mounted.wait(until: "keyboard selection feedback") {
            model.selectionFeedback == initial + 1
                && commandProbe.commands?.enabled.contains(.deselectAll) == true
        }

        commandProbe.commands?.perform(.deselectAll)
        try await mounted.wait(until: "keyboard deselection feedback") {
            model.selectionFeedback == initial + 2
                && commandProbe.commands?.enabled.contains(.deselectAll) == false
        }

        commandProbe.commands?.perform(.selectAll)
        try await mounted.wait(until: "bulk selection feedback") {
            model.selectionFeedback == initial + 3
                && commandProbe.commands?.enabled.contains(.markDone) == true
        }
        commandProbe.commands?.perform(.markDone)
        try await mounted.wait(until: "bulk completion leaves selection") {
            model.completionFeedback == 1
                && model.selectionFeedback == initial + 3
                && commandProbe.commands?.enabled.contains(.deselectAll) == false
        }
        XCTAssertEqual(model.completionFeedback + model.selectionFeedback,
                       1 + initial + 3,
                       "Bulk completion must advance only its success trigger")

        try await mounted.wait(until: "completed task snapshot") {
            mounted.hasAccessibilityElement(label: "Reopen Keyboard selection")
        }
        commandProbe.commands?.perform(.selectAll)
        try await mounted.wait(until: "completed task selection feedback") {
            model.selectionFeedback == initial + 4
                && commandProbe.commands?.enabled.contains(.markDone) == true
                && commandProbe.commands?.reopensTask == true
        }
        let beforeReopenCompletion = model.completionFeedback
        let beforeReopenSelection = model.selectionFeedback
        commandProbe.commands?.perform(.markDone)
        try await mounted.wait(until: "bulk reopen leaves selection") {
            model.selectionFeedback == beforeReopenSelection + 1
                && commandProbe.commands?.enabled.contains(.deselectAll) == false
        }
        XCTAssertEqual(model.completionFeedback, beforeReopenCompletion)
        XCTAssertEqual((model.completionFeedback - beforeReopenCompletion)
                       + (model.selectionFeedback - beforeReopenSelection), 1,
                       "Reopening must advance only its selection-exit trigger")

        commandProbe.commands?.perform(.selectAll)
        try await mounted.wait(until: "delete selection feedback") {
            model.selectionFeedback == beforeReopenSelection + 2
                && commandProbe.commands?.enabled.contains(.deleteTask) == true
        }
        let beforeDeleteCompletion = model.completionFeedback
        let beforeDeleteSelection = model.selectionFeedback
        commandProbe.commands?.perform(.deleteTask)
        try await mounted.wait(until: "bulk delete leaves selection") {
            model.selectionFeedback == beforeDeleteSelection + 1
                && commandProbe.commands?.enabled.contains(.deselectAll) == false
        }
        XCTAssertEqual(model.completionFeedback, beforeDeleteCompletion)
        XCTAssertEqual((model.completionFeedback - beforeDeleteCompletion)
                       + (model.selectionFeedback - beforeDeleteSelection), 1,
                       "Delete must advance only its selection-exit trigger")

        XCTAssertTrue(TaskSelectionTransitionFeedback.selection.emitsSelectionFeedback)
        XCTAssertFalse(TaskSelectionTransitionFeedback.coveredBySuccess.emitsSelectionFeedback)
        XCTAssertFalse(TaskSelectionTransitionFeedback.silent.emitsSelectionFeedback)
        let changed = Outcome(changed: true, message: nil, undo: nil, syncNow: false)
        let unchanged = Outcome(changed: false, message: nil, undo: nil, syncNow: false)
        XCTAssertEqual(TaskCommandFeedback.after(.add("Task", .today), outcome: changed), .silent)
        XCTAssertEqual(TaskCommandFeedback.after(.complete([], true), outcome: changed), .completionSuccess)
        XCTAssertEqual(TaskCommandFeedback.after(.complete([], false), outcome: changed), .selectionExit)
        XCTAssertEqual(TaskCommandFeedback.after(.delete([]), outcome: changed), .selectionExit)
        XCTAssertEqual(TaskCommandFeedback.after(.archive, outcome: changed), .silent)
        XCTAssertEqual(TaskCommandFeedback.after(.undo, outcome: changed), .silent)
        XCTAssertEqual(TaskCommandFeedback.after(.complete([], true), outcome: unchanged), .silent)
        XCTAssertEqual(TaskCommandFeedback.after(.delete([]), outcome: nil), .silent)
        XCTAssertEqual(TaskSelectionTransitionFeedback.afterTaskCommand(.complete([], true), outcome: changed),
                       .coveredBySuccess)
        XCTAssertEqual(TaskSelectionTransitionFeedback.afterTaskCommand(.complete([], false), outcome: changed),
                       .selection)
        XCTAssertEqual(TaskSelectionTransitionFeedback.afterTaskCommand(.delete([]), outcome: changed), .selection)
        XCTAssertEqual(TaskSelectionTransitionFeedback.afterTaskCommand(.complete([], true), outcome: unchanged),
                       .silent)
        XCTAssertEqual(TaskSelectionTransitionFeedback.afterTaskCommand(.delete([]), outcome: nil), .silent)
        XCTAssertEqual(TaskSelectionTransitionFeedback.afterOrganization(.tag([], "Tag"), outcome: changed), .selection)
        XCTAssertEqual(TaskSelectionTransitionFeedback.afterOrganization(.tag([], "Tag"), outcome: unchanged), .silent)
        XCTAssertEqual(TaskSelectionTransitionFeedback.afterOrganization(.tomorrow([]), outcome: changed),
                       .coveredBySuccess)
        XCTAssertEqual(TaskSelectionTransitionFeedback.afterOrganization(.tomorrow([]), outcome: unchanged), .silent)
    }

    func testOrganizationFeedbackOnlyTracksChangedTaskMovements() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let name = "momentum-organization-feedback-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name); try? FileManager.default.removeItem(at: directory) }
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
        await model.foreground()
        let worker = try XCTUnwrap(model.worker)

        var taskIDs: [String] = []
        for title in ["Move one", "Move two", "Order one", "Order two"] {
            let result = await model.createTaskFromText(title, view: .today)
            taskIDs.append(try XCTUnwrap(result?.createdTaskID))
        }
        let createdProjectID = await worker.createProject("Feedback Project")
        let projectID = try XCTUnwrap(createdProjectID)
        let createdOrderProjectID = await worker.createProject("Order Project")
        let orderProjectID = try XCTUnwrap(createdOrderProjectID)

        func expectMovement(_ command: OrganizationCommand, _ label: String) async throws {
            let before = model.interactionSuccessFeedback
            let result = await model.organize(command)
            let outcome = try XCTUnwrap(result)
            XCTAssertTrue(outcome.changed, label)
            XCTAssertEqual(model.interactionSuccessFeedback, before + 1, label)
        }

        try await expectMovement(.tomorrow([taskIDs[0]]), "tomorrow")
        // On Sunday, Tomorrow and Next Week both resolve to Monday. Use a fresh task
        // so this remains a real movement on every weekday.
        try await expectMovement(.nextWeek([taskIDs[1]]), "next week")
        try await expectMovement(.today([taskIDs[0]]), "today")
        try await expectMovement(.removeToday(taskIDs[0]), "remove today")
        try await expectMovement(.slot([taskIDs[0]], .morning), "slot")
        try await expectMovement(.project([taskIDs[0]], projectID), "project")

        var before = model.interactionSuccessFeedback
        let unchangedResult = await model.organize(.project([taskIDs[0]], projectID))
        let unchanged = try XCTUnwrap(unchangedResult)
        XCTAssertFalse(unchanged.changed)
        XCTAssertEqual(model.interactionSuccessFeedback, before)

        try await expectMovement(.drop([taskIDs[1]], .project(id: projectID)), "drop")
        let orderSetup = await worker.organize(.project([taskIDs[2], taskIDs[3]], orderProjectID))
        XCTAssertTrue(orderSetup.changed)
        let orderView = MomentumCore.View.project(id: orderProjectID)
        var ordered = await worker.snapshot(view: orderView).tasks.map(\.id)
        let last = try XCTUnwrap(ordered.last)
        let first = try XCTUnwrap(ordered.first)
        try await expectMovement(.reorder([last], before: first, view: orderView), "reorder")
        ordered = await worker.snapshot(view: orderView).tasks.map(\.id)
        try await expectMovement(.nudge(try XCTUnwrap(ordered.first), 1, orderView), "nudge")

        func expectExcluded(_ command: OrganizationCommand, _ label: String) async throws {
            let before = model.interactionSuccessFeedback
            let result = await model.organize(command)
            let outcome = try XCTUnwrap(result)
            XCTAssertTrue(outcome.changed, label)
            XCTAssertEqual(model.interactionSuccessFeedback, before, label)
        }

        try await expectExcluded(.tag([taskIDs[0]], "Feedback Tag"), "tag assignment")
        let tagSnapshot = await worker.snapshot(view: .today)
        let tagID = try XCTUnwrap(tagSnapshot.tags.first { $0.title == "Feedback Tag" }?.id)
        try await expectExcluded(.updateProject(projectID, "Renamed Project", "#FF6600"), "project metadata")
        try await expectExcluded(.updateTag(tagID, "Renamed Tag", "#FF6600"), "tag metadata")
        try await expectExcluded(.importText("Imported without movement haptic", .today), "text import")
        try await expectExcluded(.deleteTag(tagID), "tag deletion")
        try await expectExcluded(.deleteProject(projectID), "project deletion")

        before = model.interactionSuccessFeedback
        let blocked = directory.appendingPathComponent("pending.json.tmp")
        try FileManager.default.createDirectory(at: blocked, withIntermediateDirectories: false)
        let failedResult = await model.organize(.tomorrow([taskIDs[2]]))
        let failed = try XCTUnwrap(failedResult)
        XCTAssertFalse(failed.changed)
        XCTAssertEqual(model.interactionSuccessFeedback, before)
        try FileManager.default.removeItem(at: blocked)
    }

    func testFeedbackAnnouncesEachResultOnceWithoutFocusChangesOrDismissalAnnouncement() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let name = "momentum-feedback-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name) }
        var announcements: [AttributedString] = []
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults,
                                   announceFeedback: { announcements.append($0) })
        model.selectedTab = 2
        model.accept(Outcome(changed: false, message: .saveFailed(error: "Isolated save failure"), undo: nil, syncNow: false))
        XCTAssertTrue(try XCTUnwrap(model.feedback).contains("Isolated save failure"))
        XCTAssertEqual(announcements.count, 1)
        XCTAssertEqual(String(announcements[0].characters), model.feedback)
        XCTAssertEqual(announcements[0].accessibilitySpeechAnnouncementPriority, .low)
        XCTAssertEqual(model.selectedTab, 2)
        XCTAssertNil(model.notificationTask)
        XCTAssertFalse(model.showingAdd)
        model.dismissFeedback()
        XCTAssertNil(model.feedback)
        XCTAssertEqual(announcements.count, 1)

        // Repeating an intentional action is a new result, even if its text matches.
        model.showFeedback("Title copied")
        model.showFeedback("Title copied")
        XCTAssertEqual(announcements.map { String($0.characters) },
                       ["Could not save changes: Isolated save failure", "Title copied", "Title copied"])
        model.accept(Outcome(changed: false, message: nil, undo: nil, syncNow: false))
        XCTAssertEqual(announcements.count, 3)
        XCTAssertEqual(model.feedback, "Title copied")
    }

    func testDurableFailureKeepsCaptureDraftAndSkipsSuccessInvalidationUntilRetry() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let name = "momentum-mutation-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name); try? FileManager.default.removeItem(at: directory) }
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
        await model.foreground()
        let worker = try XCTUnwrap(model.worker)
        let tags = await worker.snapshot(view: .today).tags.map(\.id)
        let rejected = await model.createTaskFromText("#orphan 30m", view: .today)
        XCTAssertNotEqual(rejected?.outcome.changed, true)
        XCTAssertNil(rejected?.createdTaskID)
        let afterRejection = await worker.snapshot(view: .today)
        XCTAssertEqual(afterRejection.tags.map(\.id), tags)
        let revision = model.revision
        let blocked = directory.appendingPathComponent("pending.json.tmp")
        try FileManager.default.createDirectory(at: blocked, withIntermediateDirectories: false)
        let failed = await model.createTaskFromText("Retain this draft #newtag", view: .today)
        XCTAssertNotEqual(failed?.outcome.changed, true)
        XCTAssertNil(failed?.createdTaskID)
        XCTAssertEqual(model.revision, revision)
        XCTAssertEqual(model.completionFeedback, 0)
        let unsaved = await worker.snapshot(view: .today)
        XCTAssertTrue(unsaved.tasks.isEmpty)
        XCTAssertEqual(unsaved.tags.map(\.id), tags)
        try FileManager.default.removeItem(at: blocked)
        let saved = await model.createTaskFromText("Retain this draft #newtag", view: .today)
        XCTAssertNotNil(saved?.createdTaskID)
        XCTAssertEqual(model.revision, revision + 1)
        let committed = await worker.snapshot(view: .today)
        XCTAssertEqual(committed.tasks.map(\.title), ["Retain this draft"])
    }

    func testAddPresentsTaskEditorAndDefersQueuedNotification() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let name = "momentum-editor-routing-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name); try? FileManager.default.removeItem(at: directory) }
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
        await model.foreground()
        let mounted = try HostingFixture.mount(
            RootView().environment(model).environment(model.sync),
            defaults: defaults, defaultsName: name
        )
        defer { mounted.unmount() }
        model.add(in: .today)
        try await mounted.wait(until: "task editor presentation") {
            model.showingAdd && mounted.presentedControllers.count == 1 && model.activeTaskEditors == 1
        }
        let handled = await model.handleNotification(.reminder(taskID: "reminder-task"), action: nil,
                                                     deliveryID: "queued-reminder")
        XCTAssertTrue(handled)
        XCTAssertNil(model.notificationTask)
        XCTAssertEqual(mounted.presentedControllers.count, 1)

        model.showingAdd = false
        try await mounted.wait(until: "queued notification editor presentation") {
            model.notificationTask?.id == "reminder-task" && mounted.presentedControllers.count == 1
        }
        XCTAssertEqual(model.notificationTask?.id, "reminder-task")
        XCTAssertEqual(mounted.maximumPresentationDepth, 1)
    }
}

private actor FeedbackDelay {
    private var durations: [Duration] = []
    private var pending: [UUID: CheckedContinuation<Void, Error>] = [:]
    private var latest: UUID?
    private var countWaiter: (Int, CheckedContinuation<Void, Never>)?
    var count: Int { durations.count }
    var lastDuration: Duration? { durations.last }

    func sleep(_ duration: Duration) async throws {
        let id = UUID()
        try await withTaskCancellationHandler {
            try await withCheckedThrowingContinuation { continuation in
                durations.append(duration)
                pending[id] = continuation
                latest = id
                if let (target, waiter) = countWaiter, durations.count >= target {
                    countWaiter = nil
                    waiter.resume()
                }
            }
        } onCancel: { Task { await self.cancel(id) } }
    }

    func waitForCount(_ count: Int) async {
        if durations.count < count { await withCheckedContinuation { countWaiter = (count, $0) } }
    }

    func advanceLatest() {
        if let latest { pending.removeValue(forKey: latest)?.resume() }
    }

    private func cancel(_ id: UUID) {
        pending.removeValue(forKey: id)?.resume(throwing: CancellationError())
    }
}

@MainActor private final class TaskCommandProbe {
    var commands: MobileTaskCommandContext?
}

private struct TaskCommandCapture: SwiftUI.View {
    let probe: TaskCommandProbe
    @FocusedValue(\.mobileTaskCommands) private var commands

    var body: some SwiftUI.View {
        NavigationStack { TaskScreen(view: .today) }
            .onChange(of: commands, initial: true) { _, value in
                probe.commands = value
            }
    }
}
