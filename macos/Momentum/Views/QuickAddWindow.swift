// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// The one-line window the system-wide shortcut opens: type, Return, done. Escape closes
// it. Whatever you add lands in Today, wherever you were.
import MomentumCore
import MomentumKit
import SwiftUI

struct QuickAddWindow: SwiftUI.View {
    @Environment(AppState.self) private var state
    @Environment(\.dismiss) private var dismiss
    @State private var text = ""
    @FocusState private var focused: Bool

    var body: some SwiftUI.View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Image(systemName: "plus.circle").foregroundStyle(.secondary)
                TextField(String(localized: "Add a task…  #tag  1h 30m"), text: $text)
                    .textFieldStyle(.plain)
                    .appFont(.title3, area: .content)
                    .focused($focused)
                    .onSubmit(add)
                    .accessibilityLabel(String(localized: "Add a task"))
            }
            Text(String(localized: "Added to Today"))
                .appFont(.caption)
                .foregroundStyle(.secondary)
        }
        .padding(16)
        .frame(width: 520)
        .onAppear { focused = true }
        .onExitCommand { dismiss() }
    }

    private func add() {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            dismiss()
            return
        }
        state.addTaskForToday(trimmed)
        text = ""
        dismiss()
    }
}

/// The menu bar item: what is left today, and the things worth reaching without the window.
struct MenuBarView: SwiftUI.View {
    @Environment(AppState.self) private var state
    @Environment(\.openWindow) private var openWindow

    var body: some SwiftUI.View {
        Text(state.todayOpenCount == 0
            ? String(localized: "Nothing left today")
            : String(localized: "\(state.todayOpenCount) tasks today"))
        Divider()
        ForEach(todaysTasks, id: \.id) { row in
            Button {
                state.setDone(row.id, true)
            } label: {
                Text(row.title)
            }
        }
        Divider()
        Button(String(localized: "Quick Add…")) { openWindow(id: "quick-add") }
        Button(String(localized: "Open Momentum")) { AppDelegate.showMainWindow() }
        Button(String(localized: "Sync Now")) { state.sync() }
            .disabled(!state.syncAvailable)
        Divider()
        Button(String(localized: "Quit Momentum")) { NSApp.terminate(nil) }
            .keyboardShortcut("q")
    }

    /// Today's open tasks, capped so the menu stays a menu.
    private var todaysTasks: [TaskRow] {
        let listing = state.engine.listing(view: .today, archiveLimit: 0)
        return listing.sections
            .flatMap(\.rows)
            .compactMap { if case .task(let r) = $0 { return r } else { return nil } }
            .filter { !$0.isDone && !$0.isSubtask }
            .prefix(8)
            .map { $0 }
    }
}
