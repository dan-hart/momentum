// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore
import MomentumMobile
import SwiftUI
import UIKit

struct TaskWorkspace: SwiftUI.View {
    @Environment(MobileAppModel.self) private var model
    @Environment(\.horizontalSizeClass) private var horizontalSizeClass
    @Binding var state: TaskWorkspaceState
    var loadingPresentation = TaskLoadingPresentation.progress
    var snapshotLoader: TaskSnapshotLoader = {
        worker, view, query, archiveLimit in
        await worker.snapshot(view: view, query: query, archiveLimit: archiveLimit)
    }
    var onInitialTodayReady: @MainActor () -> Void = {}
    @State private var snapshot: TaskSnapshot?

    var body: some SwiftUI.View {
        GeometryReader { geometry in
            let layout = TaskWorkspacePolicy.presentation(
                horizontalSizeClass: horizontalSizeClass == .regular ? .regular : .compact,
                availableWidth: geometry.size.width
            )

            NavigationSplitView(
                columnVisibility: columnVisibility(layout: layout),
                preferredCompactColumn: preferredCompactColumn(layout: layout)
            ) {
                ListPicker(snapshot: snapshot, selection: selection(layout: layout))
                    .navigationSplitViewColumnWidth(
                        min: TaskWorkspacePolicy.minimumSidebarWidth,
                        ideal: 300,
                        max: 340
                    )
                    .background(TaskWorkspaceColumnMarker(identifier: "task-workspace-sidebar"))
                    .accessibilityIdentifier("task-workspace-sidebar")
            } detail: {
                NavigationStack {
                    TaskScreen(
                        view: state.selection,
                        dismissesWhenViewDisappears: false,
                        loadingPresentation: state.selection == .today ? loadingPresentation : .progress,
                        onSnapshotChange: receive,
                        snapshotLoader: snapshotLoader,
                        onInitialSnapshotReady: onInitialTodayReady,
                        showsSidebarButton: layout == .detailOnly
                    )
                    .id(state.selection)
                }
                .background(TaskWorkspaceColumnMarker(identifier: "task-workspace-detail"))
                .accessibilityIdentifier("task-workspace-detail")
            }
            .navigationSplitViewStyle(.balanced)
            .onChange(of: layout, initial: true) { _, newLayout in
                state.applyLayout(newLayout)
            }
        }
    }

    private func selection(layout: TaskWorkspacePresentation) -> Binding<MomentumCore.View> {
        Binding(
            get: { state.selection },
            set: { newValue in
                let previous = state.selection
                state.select(newValue, layout: layout)
                model.sidebarSelectionDidChange(from: previous, to: newValue)
            }
        )
    }

    private func columnVisibility(
        layout: TaskWorkspacePresentation
    ) -> Binding<NavigationSplitViewVisibility> {
        Binding(
            get: { state.columnVisibility == .all ? .all : .detailOnly },
            set: { visibility in
                if visibility == .detailOnly {
                    state.applyLayout(.detailOnly)
                } else if layout == .all {
                    state.applyLayout(.all)
                } else {
                    state.revealSidebar()
                }
            }
        )
    }

    private func preferredCompactColumn(
        layout: TaskWorkspacePresentation
    ) -> Binding<NavigationSplitViewColumn> {
        Binding(
            get: { state.preferredCompactColumn == .sidebar ? .sidebar : .detail },
            set: { column in
                if column == .sidebar {
                    state.revealSidebar()
                } else {
                    state.select(state.selection, layout: layout)
                }
            }
        )
    }

    private func receive(_ newSnapshot: TaskSnapshot) {
        snapshot = newSnapshot
        state.reconcile(availableViews: availableViews(in: newSnapshot))
    }

    private func availableViews(in snapshot: TaskSnapshot) -> Set<MomentumCore.View> {
        var available = Set(snapshot.sidebar.fixed.map(\.view))
        available.insert(.today)
        available.insert(.archive)
        available.formUnion(snapshot.projects.map { .project(id: $0.id) })
        available.formUnion(snapshot.tags.map { .tag(id: $0.id) })
        return available
    }
}

private struct TaskWorkspaceColumnMarker: UIViewRepresentable {
    let identifier: String

    func makeUIView(context: Context) -> UIView {
        let view = UIView()
        view.backgroundColor = .clear
        view.isUserInteractionEnabled = false
        view.accessibilityIdentifier = identifier
        return view
    }

    func updateUIView(_ view: UIView, context: Context) {
        view.accessibilityIdentifier = identifier
    }
}
