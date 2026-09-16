// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// The one-line add box at the top of every view: Enter creates, `#` opens a tag
// completion popover (arrows, Return or Tab to accept, Escape to close), a multi-line
// paste becomes several tasks.
import MomentumCore
import MomentumKit
import SwiftUI

struct QuickAddField: SwiftUI.View {
    @Environment(AppState.self) private var state
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.colorSchemeContrast) private var contrast
    @State private var text = ""
    @State private var completions: [TagCompletion] = []
    @State private var selected = 0
    @State private var added = false
    @FocusState private var focused: Bool

    private var popoverShown: Binding<Bool> {
        Binding(get: { !completions.isEmpty && focused }, set: { if !$0 { completions = [] } })
    }

    var body: some SwiftUI.View {
        HStack(spacing: 10) {
            Image(systemName: added ? "checkmark.circle.fill" : "plus.circle")
                .foregroundStyle(focused || added ? Color.accentColor : .secondary)
                .contentTransition(reduceMotion ? .identity : .symbolEffect(.replace))
                .accessibilityHidden(true)
            TextField(placeholder, text: $text)
                .textFieldStyle(.plain)
                .appFont(.body, area: .content)
                .focused($focused)
                .onSubmit(submit)
                .onChange(of: text) { _, value in
                    // The native field editor handles Paste before onPasteCommand.
                    // Observe its resulting text so multi-line input reaches the shared
                    // import parser, while single-line paste keeps native selection rules.
                    if value.contains(where: \.isNewline) {
                        state.addFromText(value)
                        added = true
                        text = ""
                        completions = []
                    } else {
                        updateCompletions()
                    }
                }
                .onChange(of: focused) { _, f in if !f { completions = [] } }
                .onKeyPress(.downArrow) { move(1) }
                .onKeyPress(.upArrow) { move(-1) }
                .onKeyPress(.tab) { accept() }
                .onKeyPress(.escape) {
                    guard !completions.isEmpty else { return .ignored }
                    completions = []
                    return .handled
                }
                .accessibilityLabel(String(localized: "Add a task"))
            Button(action: submit) {
                Image(systemName: "arrow.turn.down.left")
            }
            .buttonStyle(.borderless)
            .tint(.accentColor)
            .disabled(text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            .accessibilityLabel(String(localized: "Add Task"))
            .help(String(localized: "Add Task"))
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 10)
        .background(Color(nsColor: .controlBackgroundColor), in: RoundedRectangle(cornerRadius: 10))
        .overlay {
            RoundedRectangle(cornerRadius: 10)
                .strokeBorder(focused ? Color.accentColor : Color.primary.opacity(contrast == .increased ? 0.5 : 0.12),
                              lineWidth: focused ? 1.5 : 1)
                .allowsHitTesting(false)
        }
        .animation(reduceMotion ? nil : .easeOut(duration: 0.18), value: focused)
        .animation(reduceMotion ? nil : .easeOut(duration: 0.18), value: added)
        .help(String(localized: "Use #tag to add a tag and 1h 30m to set an estimate. Press Return to add."))
        .task(id: added) {
            guard added else { return }
            try? await Task.sleep(for: .milliseconds(900))
            if !Task.isCancelled { added = false }
        }
        .popover(isPresented: popoverShown, attachmentAnchor: .rect(.bounds), arrowEdge: .top) {
            CompletionList(completions: completions, selected: selected) { i in
                selected = i
                _ = accept()
            }
        }
        .onChange(of: state.quickAddFocusRequest) { _, _ in focused = true }
        .onChange(of: state.view) { _, _ in completions = [] }
    }

    private var placeholder: String {
        String(localized: "Add a task…")
    }

    private func submit() {
        if !completions.isEmpty {
            _ = accept()
            return
        }
        guard !text.trimmingCharacters(in: .whitespaces).isEmpty else { return }
        state.addTask(text)
        added = true
        text = ""
    }

    private func updateCompletions() {
        guard let word = hashWordAt(text: text, cursor: UInt32(text.count)) else {
            completions = []
            return
        }
        completions = state.engine.tagCompletions(prefix: word.prefix)
        selected = 0
    }

    private func move(_ delta: Int) -> KeyPress.Result {
        guard !completions.isEmpty else { return .ignored }
        selected = ((selected + delta) % completions.count + completions.count) % completions.count
        return .handled
    }

    private func accept() -> KeyPress.Result {
        guard !completions.isEmpty, selected < completions.count else { return .ignored }
        let name = completions[selected].title
        if let done = completeHashWord(text: text, cursor: UInt32(text.count), name: name) {
            text = done.text
        }
        completions = []
        return .handled
    }
}

private struct CompletionList: SwiftUI.View {
    @Environment(AppState.self) private var state
    @LabelColors private var colorful: Bool
    let completions: [TagCompletion]
    let selected: Int
    let pick: (Int) -> Void

    var body: some SwiftUI.View {
        VStack(alignment: .leading, spacing: 2) {
            ForEach(Array(completions.enumerated()), id: \.offset) { i, c in
                HStack {
                    Image(systemName: "tag")
                        .foregroundStyle(colorful ? (c.color.flatMap(Color.init(hex:)) ?? .secondary) : .secondary)
                    Text(c.title)
                    Spacer()
                    Text("\(c.taskCount)").appFont(.caption).foregroundStyle(.secondary)
                }
                .padding(.horizontal, 8)
                .padding(.vertical, 4)
                .background(i == selected ? Color.accentColor.opacity(0.2) : .clear, in: RoundedRectangle(cornerRadius: 4))
                .contentShape(Rectangle())
                .onTapGesture { pick(i) }
                .accessibilityElement(children: .combine)
                .accessibilityAddTraits(.isButton)
                .accessibilityAction { pick(i) }
            }
        }
        .padding(6)
        .frame(minWidth: 220)
    }
}
