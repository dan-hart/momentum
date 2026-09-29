// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore

public enum TaskWorkspaceHorizontalSizeClass: Sendable {
    case compact
    case regular
}

public enum TaskWorkspacePresentation: Equatable, Sendable {
    case detailOnly
    case all
}

public enum TaskWorkspaceColumn: Equatable, Sendable {
    case sidebar
    case detail
}

public enum TaskWorkspacePolicy {
    public static let minimumSidebarWidth = 280.0
    public static let minimumAccessibleDetailWidth = 420.0

    /// Both columns fit whenever the width allows an accessible detail beside the
    /// sidebar. That includes an iPhone in landscape whose size class is still
    /// compact (2026-09-29); the workspace draws those columns itself, because a
    /// split view collapses in a compact class regardless of visibility.
    public static func presentation(
        horizontalSizeClass: TaskWorkspaceHorizontalSizeClass,
        availableWidth: Double
    ) -> TaskWorkspacePresentation {
        _ = horizontalSizeClass
        guard availableWidth >= minimumSidebarWidth + minimumAccessibleDetailWidth else {
            return .detailOnly
        }
        return .all
    }
}

public struct TaskWorkspaceState: Equatable, Sendable {
    public private(set) var selection: MomentumCore.View
    public private(set) var columnVisibility: TaskWorkspacePresentation
    public private(set) var preferredCompactColumn: TaskWorkspaceColumn

    public init(selection: MomentumCore.View = .today) {
        self.selection = selection
        columnVisibility = .detailOnly
        preferredCompactColumn = .detail
    }

    public mutating func applyLayout(_ layout: TaskWorkspacePresentation) {
        columnVisibility = layout
        if layout == .detailOnly {
            preferredCompactColumn = .detail
        }
    }

    public mutating func revealSidebar() {
        columnVisibility = .all
        preferredCompactColumn = .sidebar
    }

    public mutating func select(
        _ view: MomentumCore.View,
        layout: TaskWorkspacePresentation
    ) {
        selection = view
        preferredCompactColumn = .detail
        columnVisibility = layout
    }

    public mutating func reconcile(availableViews: Set<MomentumCore.View>) {
        guard !availableViews.contains(selection) else { return }
        selection = .today
        preferredCompactColumn = .detail
    }
}
