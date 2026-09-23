// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore
import Testing
@testable import MomentumMobile

@Suite struct SpotlightProjectionTests {
    @Test func projectionIndexesOnlyOpenTasksWithoutLosingSearchableContext() {
        let tasks = [
            TaskBrief(id: "open", title: "Book train", project: "Travel", dueDay: nil,
                      isDone: false, notes: "Window seat"),
            TaskBrief(id: "done", title: "Renew passport", project: nil, dueDay: nil,
                      isDone: true, notes: nil),
        ]

        let documents = MobileSearchDocument.project(tasks)

        #expect(documents == [
            MobileSearchDocument(id: "open", title: "Book train", project: "Travel",
                                 dueDay: nil, notes: "Window seat"),
        ])
    }

    @Test func projectionIsStableAcrossCoreIterationOrder() {
        let alpha = TaskBrief(id: "a", title: "Alpha", project: nil, dueDay: nil,
                              isDone: false, notes: nil)
        let beta = TaskBrief(id: "b", title: "Beta", project: nil, dueDay: nil,
                             isDone: false, notes: nil)

        #expect(MobileSearchDocument.project([beta, alpha]) ==
                MobileSearchDocument.project([alpha, beta]))
    }
}
