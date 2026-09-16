// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
import MomentumCore
import MomentumKit
import SwiftUI

struct TaskRowView: SwiftUI.View {
    @Environment(AppState.self) private var state
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.appTypography) private var typography
    @LabelColors private var colorful: Bool
    let row: TaskRow
    @State private var targeted = false

    var body: some SwiftUI.View {
        HStack(alignment: .center, spacing: 10) {
            if row.archived {
                Image(systemName: "checkmark")
                    .foregroundStyle(.secondary)
                    .frame(width: 16)
            } else {
                Toggle(isOn: Binding(get: { row.isDone }, set: { done in
                    // Finish AppKit's table callback before moving/removing this row.
                    Task { @MainActor in
                        withAnimation(reduceMotion ? nil : .easeInOut(duration: 0.2)) {
                            state.setDone(row.id, done)
                        }
                    }
                })) {
                    EmptyView()
                }
                .toggleStyle(.checkbox)
                .labelsHidden()
                .accessibilityLabel(String(localized: "Done: \(row.title)"))
            }
            VStack(alignment: .leading, spacing: 4) {
                Text(row.title)
                    .foregroundStyle(row.isDone ? .secondary : .primary)
                    .strikethrough(row.isDone, color: .secondary)
                    .lineLimit(2)
                    .fixedSize(horizontal: false, vertical: true)
                if let subtitle = subtitle {
                    subtitle
                        .appFont(.caption, area: .content)
                        .foregroundStyle(.secondary)
                        .lineLimit(typography.contentSize > 18 ? 3 : 2)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }
            Spacer(minLength: 0)
            badges
        }
        .appFont(.body, area: .content)
        .padding(.leading, row.isSubtask ? 28 : 0)
        .padding(.vertical, 6)
        .contentShape(Rectangle())
        .listRowBackground(targeted ? Color.accentColor.opacity(0.15) : nil)
        .modifier(TaskDragSource(id: row.id))
        .modifier(ReorderTarget(row: row, targeted: $targeted))
        .accessibilityElement(children: .combine)
    }

    /// Project · ~estimate · day time · repeat · #tags, the order every list uses.
    /// One attributed run per part, so the project dot and each tag keep their own colour.
    private var subtitle: Text? {
        func tinted(_ text: String, _ hex: String?) -> AttributedString {
            var run = AttributedString(text)
            run.foregroundColor = color(hex)
            return run
        }
        let parts = TaskSubtitle.parts(for: row).map { part -> AttributedString in
            switch part.kind {
            case .project: return tinted("●", part.color) + AttributedString(" " + part.text)
            case .tag: return tinted(part.text, part.color)
            default: return AttributedString(part.text)
            }
        }
        guard let first = parts.first else { return nil }
        let separator = AttributedString("  ·  ")
        return Text(parts.dropFirst().reduce(first) { $0 + separator + $1 })
    }

    @ViewBuilder
    private var badges: some SwiftUI.View {
        HStack(spacing: 8) {
            if let notes = row.notesPreview {
                Image(systemName: "note.text")
                    .help(notes)
                    .accessibilityLabel(String(localized: "Has notes"))
            }
            if let r = row.reminder {
                Image(systemName: "bell")
                    .help(String(localized: "Reminder at \(Strings.time(r))"))
                    .accessibilityLabel(String(localized: "Reminder at \(Strings.time(r))"))
            }
            if let rep = row.repeat {
                Image(systemName: "repeat")
                    .help(Strings.repeatText(rep))
                    .accessibilityLabel(Strings.repeatText(rep))
            }
        }
        .foregroundStyle(.secondary)
        .appFont(.caption, area: .content)
    }

    private func color(_ hex: String?) -> Color {
        if colorful, let h = hex, let c = Color(hex: h) { return c }
        return .secondary
    }
}

/// Archived rows expose no drag interaction, including archived results in Search.
private struct TaskDragSource: ViewModifier {
    @Environment(AppState.self) private var state
    let id: String
    func body(content: Content) -> some SwiftUI.View {
        if let transfer = state.dragTransfer(for: id) {
            content.draggable(transfer)
        } else {
            content
        }
    }
}

/// Dropping tasks on a top-level row places them before it (Manual Order only).
private struct ReorderTarget: ViewModifier {
    @Environment(AppState.self) private var state
    let row: TaskRow
    @Binding var targeted: Bool

    func body(content: Content) -> some SwiftUI.View {
        if row.archived || row.isSubtask {
            content
        } else {
            content.dropDestination(for: TaskTransfer.self) { items, _ in
                state.reorder(items.flatMap(\.ids), before: row.id)
            } isTargeted: { targeted = $0 }
        }
    }
}
