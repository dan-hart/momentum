// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore
import MomentumKit
import MomentumMobile
import SwiftUI
import UIKit
import XCTest

@MainActor final class NotesCopyIntegrationTests: XCTestCase {
    func testCopySuccessIsVisibleInTheActiveEditorAndKeepsCompleteNotes() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let name = "momentum-copy-editor-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
        await model.foreground()
        let worker = try XCTUnwrap(model.worker)
        let notes = "Complete notes 📝\nSecond line — Grüße"
        _ = await worker.createTask(TaskDraft(title: "Copy fixture", projectId: "", dueDay: nil,
            time: nil, reminderMinutesBefore: nil, estimateMs: 0, notes: notes, tagIds: [], newTags: []), view: .today)
        let createdID = await worker.lastAddedTaskID()
        let taskID = try XCTUnwrap(createdID)
        let mounted = try HostingFixture.mount(
            TaskEditor(worker: worker, view: .today, taskID: taskID, onChange: {})
                .environment(model).environment(model.sync), defaults: defaults, defaultsName: name
        )
        defer {
            mounted.unmount()
            UIPasteboard.general.string = nil
            try? FileManager.default.removeItem(at: directory)
        }
        do {
            try await mounted.wait(until: "loaded editor") { mounted.accessibilityElement(identifier: "task-editor-scheduled") != nil }
        } catch {
            print("Synthetic editor accessibility:", mounted.accessibilitySnapshot())
            let image = XCTAttachment(image: mounted.renderedImage())
            image.name = "Synthetic notes editor"
            image.lifetime = .keepAlways
            add(image)
            throw error
        }
        try await mounted.wait(until: "copy notes control") {
            mounted.scrollToBottom()
            return mounted.hasAccessibilityElement(label: String(localized: "Copy the note"))
        }
        XCTAssertTrue(mounted.activateAccessibilityElement(label: String(localized: "Copy the note")))
        XCTAssertEqual(UIPasteboard.general.string, notes)
        try await mounted.wait(until: "copy toast inside editor") {
            mounted.hasAccessibilityElement(label: String(localized: "Notes copied"))
        }
        let saved = await worker.taskDetail(taskID)
        XCTAssertEqual(saved?.notes, notes)
    }
}
