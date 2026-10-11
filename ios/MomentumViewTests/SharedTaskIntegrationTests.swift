// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import MomentumKit
import MomentumMobile
import XCTest

/// The app side of the share sheet: what the extension left in the inbox becomes Inbox
/// tasks on the next foreground, exactly once, with a toast that says so.
@MainActor
final class SharedTaskIntegrationTests: XCTestCase {
    func testForegroundImportsInboxItemsOnceAndClearsThem() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let inbox = directory.appendingPathComponent("inbox")
        let name = "momentum-hosted-share-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name); try? FileManager.default.removeItem(at: directory) }

        let draft = sharedTaskDraft(title: nil, text: "https://kenney.nl/assets/interface-sounds", url: nil)
        let link = SharedTaskItem(title: draft.title, notes: draft.notes, url: draft.url)
        let selection = SharedTaskItem(title: "Call the clinic", notes: "https://example.com/clinic\n\nCall the clinic, see https://example.com/clinic", url: "https://example.com/clinic")
        try SharedTaskInbox.write(link, in: inbox)
        try SharedTaskInbox.write(selection, in: inbox)

        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults, sharedInbox: inbox)
        model.setForeground(true)
        await model.foreground()

        XCTAssertEqual(model.feedback, String(localized: "Added \(2) shared tasks to Inbox"))
        XCTAssertTrue(SharedTaskInbox.pending(in: inbox).isEmpty, "imported items leave the inbox")
        let worker = try XCTUnwrap(model.worker)
        let found = await worker.snapshot(view: .search, query: "Interface sounds")
        XCTAssertEqual(found.tasks.map(\.title), ["Interface sounds – kenney.nl"])
        let loaded = await worker.taskDetail(found.tasks[0].id)
        let detail = try XCTUnwrap(loaded)
        XCTAssertEqual(detail.notes, "https://kenney.nl/assets/interface-sounds")
        XCTAssertNil(detail.dueDay)

        // A second foreground with the same files (a crash between import and removal) adds nothing.
        try SharedTaskInbox.write(link, in: inbox)
        let created = await model.importSharedTasks()
        XCTAssertEqual(created, 0)
        XCTAssertTrue(SharedTaskInbox.pending(in: inbox).isEmpty)
        let again = await worker.snapshot(view: .search, query: "Interface sounds")
        XCTAssertEqual(again.tasks.count, 1)
    }
}
