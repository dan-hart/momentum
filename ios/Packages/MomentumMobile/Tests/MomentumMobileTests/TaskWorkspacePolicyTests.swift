// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore
import Testing
@testable import MomentumMobile

@Suite struct TaskWorkspacePolicyTests {
    @Test func regularWideLayoutShowsBothColumns() {
        #expect(TaskWorkspacePolicy.presentation(
            horizontalSizeClass: .regular,
            availableWidth: 844
        ) == .all)
    }

    @Test(arguments: [
        (TaskWorkspaceHorizontalSizeClass.compact, 844.0),
        (.regular, 699.0),
    ])
    func compactOrNarrowLayoutPrefersDetail(
        horizontalSizeClass: TaskWorkspaceHorizontalSizeClass,
        availableWidth: Double
    ) {
        #expect(TaskWorkspacePolicy.presentation(
            horizontalSizeClass: horizontalSizeClass,
            availableWidth: availableWidth
        ) == .detailOnly)
    }

    @Test func selectionSurvivesCompactAndWideLayoutChanges() {
        var state = TaskWorkspaceState(selection: .project(id: "roadmap"))

        state.applyLayout(.detailOnly)
        state.applyLayout(.all)

        #expect(state.selection == .project(id: "roadmap"))
        #expect(state.columnVisibility == .all)
    }

    @Test func compactSidebarSelectionReturnsToDetail() {
        var state = TaskWorkspaceState()
        state.applyLayout(.detailOnly)
        state.revealSidebar()

        state.select(.morning, layout: .detailOnly)

        #expect(state.selection == .morning)
        #expect(state.preferredCompactColumn == .detail)
        #expect(state.columnVisibility == .detailOnly)
    }

    @Test func disappearingMorningFallsBackToToday() {
        assertFallbackToToday(selection: .morning, available: [.today, .tonight, .archive])
    }

    @Test func disappearingEveningFallsBackToToday() {
        assertFallbackToToday(selection: .tonight, available: [.today, .morning, .archive])
    }

    @Test func disappearingProjectFallsBackToToday() {
        assertFallbackToToday(
            selection: .project(id: "deleted-project"),
            available: [.today, .project(id: "kept-project"), .archive]
        )
    }

    @Test func disappearingTagFallsBackToToday() {
        assertFallbackToToday(
            selection: .tag(id: "deleted-tag"),
            available: [.today, .tag(id: "kept-tag"), .archive]
        )
    }

    private func assertFallbackToToday(
        selection: MomentumCore.View,
        available: Set<MomentumCore.View>
    ) {
        var state = TaskWorkspaceState(selection: selection)

        state.reconcile(availableViews: available)

        #expect(state.selection == .today)
        #expect(state.preferredCompactColumn == .detail)
    }
}
