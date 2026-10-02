// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore
import MomentumMobile
import SwiftUI
import XCTest

@MainActor private final class HostedRemindersFixture: RemindersSource {
    var permission = RemindersPermission.allowed
    var didChange: (@MainActor () -> Void)?
    var items = [ReminderImportItem(sourceID: "hosted-source", title: "Synthetic imported task", notes: "Synthetic note")]
    func requestPermission() async throws -> Bool { true }
    func lists() -> [ReminderList] { [ReminderList(id: "hosted-list", title: "Synthetic Reminders List")] }
    func incompleteReminders(in listID: String) async throws -> [ReminderImportItem] { items }
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
