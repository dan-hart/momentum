// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore
import MomentumKit
import MomentumMobile
import SwiftUI
import XCTest

@MainActor private final class HostedRemindersFixture: RemindersSource {
    var permission = RemindersPermission.allowed
    var didChange: (@MainActor () -> Void)?
    var error: RemindersImportIssue?
    var fetches = 0
    var items = [ReminderImportItem(sourceID: "hosted-source", title: "Synthetic imported task", notes: "Synthetic note")]
    func requestPermission() async throws -> Bool { true }
    func lists() -> [ReminderList] { [ReminderList(id: "hosted-list", title: "Synthetic Reminders List")] }
    func incompleteReminders(in listID: String) async throws -> [ReminderImportItem] {
        fetches += 1
        if let error { throw error }
        return items
    }
}

@MainActor final class RemindersIntegrationTests: XCTestCase {
    func testImportRefreshesAppAndNotificationRechecksSource() async throws {
        try await withModel { model, source, _, _ in
            await model.reminders.loadLists()
            model.reminders.selectList("hosted-list")
            let before = model.revision
            await model.reminders.importNow()
            XCTAssertEqual(model.reminders.lastImportCount, 1)
            XCTAssertGreaterThan(model.revision, before)
            let worker = try XCTUnwrap(model.worker)
            let tasks = await worker.snapshot(view: .search, query: "Synthetic imported task")
            XCTAssertEqual(tasks.tasks.count, 1)
            model.reminders.setAutomatic(true)
            await model.reminders.reactivate()
            source.items.append(ReminderImportItem(sourceID: "second-source", title: "Second imported task"))
            source.didChange?()
            let clock = ContinuousClock()
            let deadline = clock.now.advanced(by: .seconds(5))
            while await worker.snapshot(view: .search, query: "Second imported task").tasks.isEmpty && clock.now < deadline {
                await Task.yield()
            }
            let second = await worker.snapshot(view: .search, query: "Second imported task")
            XCTAssertEqual(second.tasks.count, 1)
            model.reminders.setAutomatic(false)
        }
    }

    func testSettingsShowAccessibleControlsAndWrapInGerman() async throws {
        try await withModel { model, _, defaults, name in
            await model.reminders.loadLists()
            model.reminders.selectList("hosted-list")
            let mounted = try HostingFixture.mount(
                NavigationStack { RemindersSettings() }.environment(model), defaults: defaults,
                defaultsName: name, size: CGSize(width: 320, height: 900), locale: "de", dynamicTypeSize: .large)
            defer { mounted.unmount() }
            await mounted.settle()
            for label in ["Erinnerungsliste", "Jetzt importieren"] {
                let element = try XCTUnwrap(mounted.accessibilityElement(label: label))
                XCTAssertTrue(element.traits.contains(.button))
                XCTAssertGreaterThanOrEqual(element.frame.minX, 0)
                XCTAssertLessThanOrEqual(element.frame.maxX, 320)
                XCTAssertGreaterThan(element.frame.height, 0)
            }
            for scheme in [ColorScheme.light, .dark] {
                let rendered = try HostingFixture.render(RemindersSettings().environment(model).frame(height: 900),
                    width: 320, size: .accessibility5, scheme: scheme, locale: "de")
                XCTAssertLessThanOrEqual(rendered.size.width, 320)
                let attachment = XCTAttachment(image: rendered.image)
                attachment.name = "Reminders-German-AX5-\(scheme)"
                attachment.lifetime = .keepAlways
                self.add(attachment)
            }
        }
    }

    func testSettingsAnnounceRepeatedFailuresAndRetrySuccessfulImport() async throws {
        try await withModel { model, source, defaults, name in
            await model.reminders.loadLists()
            model.reminders.selectList("hosted-list")
            var announcements: [String] = []
            let mounted = try HostingFixture.mount(
                NavigationStack { RemindersSettings(announce: { announcements.append($0) }) }.environment(model),
                defaults: defaults, defaultsName: name, size: CGSize(width: 320, height: 900))
            defer { mounted.unmount() }
            await mounted.settle()
            source.error = .fetch
            await model.reminders.importNow()
            await mounted.settle()
            XCTAssertEqual(announcements.count, 1)
            let first = try XCTUnwrap(announcements.first)
            XCTAssertEqual(first, String(localized: "Reminders could not be loaded. Try again."))
            XCTAssertTrue(mounted.activateAccessibilityElement(label: "Try Again"))
            let deadline = ContinuousClock().now.advanced(by: .seconds(5))
            while (source.fetches < 2 || model.reminders.busy) && ContinuousClock().now < deadline { await Task.yield() }
            XCTAssertFalse(model.reminders.busy)
            await mounted.settle()
            XCTAssertEqual(announcements, [first, first], "Repeated failures must announce again without a success announcement")
            XCTAssertEqual(source.fetches, 2)
            source.error = nil
            XCTAssertTrue(mounted.activateAccessibilityElement(label: "Try Again"))
            let recoveryDeadline = ContinuousClock().now.advanced(by: .seconds(5))
            while (source.fetches < 3 || model.reminders.busy) && ContinuousClock().now < recoveryDeadline { await Task.yield() }
            XCTAssertFalse(model.reminders.busy)
            await mounted.settle()
            XCTAssertEqual(model.reminders.lastImportCount, 1)
            XCTAssertEqual(source.fetches, 3)
            XCTAssertEqual(announcements.last, String(localized: "Imported \(1) new reminders."))
            XCTAssertEqual(announcements.count, 3)
        }
    }

    private func withModel(_ body: @MainActor (MobileAppModel, HostedRemindersFixture, UserDefaults, String) async throws -> Void) async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let name = "momentum-hosted-reminders-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name); try? FileManager.default.removeItem(at: directory) }
        let source = HostedRemindersFixture()
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults, remindersSource: source)
        model.setForeground(true)
        await model.foreground()
        try await body(model, source, defaults, name)
    }
}
