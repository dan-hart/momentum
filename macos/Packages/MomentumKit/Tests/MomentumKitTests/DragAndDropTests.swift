// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
import MomentumCore
import Testing
@testable import MomentumKit

@MainActor
@Suite struct DragAndDrop {
    @Test func selectedRowCarriesSelectionInListOrderAndProjectDropMovesChildren() throws {
        let h = Harness(demo: false)
        for title in ["first", "second", "third"] { h.state.addTask(title) }
        let first = h.id("first"), third = h.id("third")
        h.state.addSubtask(first, "child")
        let child = h.id("child")
        let target = try #require(h.engine.addProject(title: "Destination"))
        h.state.refresh()
        h.state.selection = [third, first]
        let transfer = try #require(h.state.dragTransfer(for: third))
        #expect(transfer.ids == [first, third])
        #expect(h.state.drop(transfer.ids, on: .project(id: target)))
        for id in [first, third, child] {
            #expect(h.engine.taskDetail(id: id)?.projectId == target)
        }
        h.state.undo()
        for id in [first, third, child] {
            #expect(h.engine.taskDetail(id: id)?.projectId == "INBOX_PROJECT")
        }
    }

    @Test func selectionReorderIsOneUndoAndManualDescendingFollowsVisibleOrder() throws {
        let h = Harness(demo: false)
        for title in ["first", "second", "third", "fourth"] { h.state.addTask(title) }
        let original = h.rows
        h.state.selection = [original[1], original[3]]
        let transfer = try #require(h.state.dragTransfer(for: original[3]))
        #expect(h.state.reorder(transfer.ids, before: original[0]))
        #expect(h.rows == [original[1], original[3], original[0], original[2]])
        h.state.undo()
        #expect(h.rows == original)
        #expect(!h.state.canUndo)
        h.defaults.set("descending", forKey: PrefKey.sortDirection)
        h.state.preferencesChanged()
        #expect(h.state.reorder([original[0]], before: original[3]))
        #expect(h.rows == [original[0], original[3], original[2], original[1]])
        h.state.undo()
        #expect(h.rows == Array(original.reversed()))
    }

    @Test func slotDropAlwaysMovesIntoDestinationAndKeepsUndoForTheBatch() {
        let h = Harness()
        let first = h.id("Read two chapters"), second = h.id("Prep tomorrow's lunch")
        h.state.moveToTomorrow([first, second])
        let future = h.engine.taskDetail(id: first)?.dueDay
        #expect(h.state.drop([first, second], on: .tonight))
        for id in [first, second] {
            #expect(h.engine.taskMenu(id: id)?.slot == .tonight)
            #expect(h.engine.taskDetail(id: id)?.dueDay == today())
        }
        h.state.undo()
        for id in [first, second] {
            #expect(h.engine.taskDetail(id: id)?.dueDay == future)
        }
    }
}
