// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumKit
import Testing

@Suite struct AutomationTaskTests {
    @Test func publicSnapshotPreservesAdapterValues() {
        let dueDate = Date(timeIntervalSince1970: 1_900_000_000)
        let task = AutomationTask(id: "task-id", title: "Prepare notes", notes: "For the meeting",
                                  project: "Work", dueDate: dueDate, isCompleted: true)
        #expect(task.id == "task-id")
        #expect(task.title == "Prepare notes")
        #expect(task.notes == "For the meeting")
        #expect(task.project == "Work")
        #expect(task.dueDate == dueDate)
        #expect(task.isCompleted)
    }
}
