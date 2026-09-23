// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore
import MomentumKit
import SwiftUI

/// Keeps native task destinations reachable during a phone drag. Each chip also
/// provides the same move as a tap so Switch Control, Voice Control and VoiceOver
/// users do not need to reproduce a spatial drag gesture.
struct TaskDropDestinations: SwiftUI.View {
    let selectedIDs: [String]
    let projects: [ProjectRef]
    let tags: [TagRef]
    let move: ([String], MomentumCore.View) -> Void

    var body: some SwiftUI.View {
        VStack(alignment: .leading, spacing: 6) {
            Label("Move selected tasks", systemImage: "arrowshape.turn.up.right")
                .font(.caption.weight(.semibold))
                .foregroundStyle(AccentTheme.secondaryText)
                .accessibilityHidden(true)
            ScrollView(.horizontal) {
                HStack(spacing: 8) {
                    destination(String(localized: "Today"), symbol: "star",
                                id: "task-drop-today", view: .today)
                    destination(String(localized: "Morning"), symbol: "sun.max",
                                id: "task-drop-morning", view: .morning)
                    destination(String(localized: "Evening"), symbol: "moon.stars",
                                id: "task-drop-evening", view: .tonight)
                    ForEach(projects, id: \.id) { project in
                        destination(project.title, symbol: "folder",
                                    id: "task-drop-project-\(project.title)",
                                    view: .project(id: project.id))
                    }
                    ForEach(tags, id: \.id) { tag in
                        destination("#" + tag.title, symbol: "tag",
                                    id: "task-drop-tag-\(tag.title)",
                                    view: .tag(id: tag.id))
                    }
                }
            }
            .scrollIndicators(.hidden)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .accessibilityIdentifier("task-drop-destinations")
    }

    private func destination(_ title: String, symbol: String, id: String,
                             view: MomentumCore.View) -> some SwiftUI.View {
        TaskDropDestination(title: title, symbol: symbol, identifier: id,
                            selectedIDs: selectedIDs, destination: view, move: move)
    }
}

private struct TaskDropDestination: SwiftUI.View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    let title: String
    let symbol: String
    let identifier: String
    let selectedIDs: [String]
    let destination: MomentumCore.View
    let move: ([String], MomentumCore.View) -> Void
    @State private var targeted = false

    var body: some SwiftUI.View {
        Button { move(selectedIDs, destination) } label: {
            Label(title, systemImage: symbol)
                .lineLimit(1)
                .fixedSize(horizontal: true, vertical: false)
                .frame(minHeight: 30)
        }
        .buttonStyle(.bordered)
        .buttonBorderShape(.capsule)
        .controlSize(.regular)
        .background(targeted ? Color.accentColor.opacity(0.18) : Color.clear,
                    in: Capsule())
        .scaleEffect(targeted && !reduceMotion ? 1.04 : 1)
        .animation(reduceMotion ? nil : .smooth(duration: 0.16), value: targeted)
        .accessibilityLabel("Move selected tasks to \(title)")
        .accessibilityIdentifier(identifier)
        .dropDestination(for: TaskTransfer.self) { transfers, _ in
            let ids = transfers.flatMap(\.ids)
            guard !ids.isEmpty else { return false }
            move(ids, destination)
            return true
        } isTargeted: { targeted = $0 }
    }
}
