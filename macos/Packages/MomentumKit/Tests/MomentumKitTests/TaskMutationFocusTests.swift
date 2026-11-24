// SPDX-License-Identifier: GPL-3.0-or-later
import Testing
@testable import MomentumKit

@Test func taskMutationFocusUsesOldNeighborsAndSkipsAffectedRows() {
    #expect(TaskMutationFocus.replacement(before: ["a", "b", "c", "d"], after: ["d", "a", "b"], affected: ["b", "c"]) == "d")
    #expect(TaskMutationFocus.replacement(before: ["a", "b", "c"], after: ["a"], affected: ["b", "c"]) == "a")
    #expect(TaskMutationFocus.replacement(before: ["a"], after: ["a"], affected: ["a"]) == "a")
    #expect(TaskMutationFocus.replacement(before: ["a"], after: [], affected: ["a"]) == nil)
    #expect(TaskMutationFocus.replacement(before: ["a"], after: ["a"], affected: ["offscreen"]) == nil)
}
