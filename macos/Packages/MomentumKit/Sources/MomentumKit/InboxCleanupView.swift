// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
import Foundation
import MomentumCore
import SwiftUI

public struct InboxCleanupInput: Sendable {
    let changes: [InboxCleanupChange]
    private let tags: [TagRef]
    private let assignedTags: [String: Set<String>]

    public init(engine: Engine) {
        let changes = engine.previewInboxCleanup()
        self.changes = changes
        // Reserved planning tags can move a task between day lists. The cleanup
        // adds ordinary organizational tags only.
        let reserved = ["morning", "evening"]
        tags = engine.tags().filter {
            !reserved.contains($0.title.lowercased())
        }
        assignedTags = Dictionary(uniqueKeysWithValues: changes.map {
            ($0.taskId, Set(engine.taskDetail(id: $0.taskId)?.tagIds ?? []))
        })
    }

    func availableTags(for change: InboxCleanupChange) -> [TagRef] {
        let assigned = assignedTags[change.taskId] ?? []
        return tags.filter { !assigned.contains($0.id) }
    }
}

public struct InboxCleanupButton: SwiftUI.View {
    private let load: @MainActor () async -> InboxCleanupInput?
    private let apply: @MainActor ([InboxCleanupChange]) async -> Outcome?
    @State private var isReviewing = false

    public init(load: @escaping @MainActor () async -> InboxCleanupInput?,
                apply: @escaping @MainActor ([InboxCleanupChange]) async -> Outcome?) {
        self.load = load
        self.apply = apply
    }

    public var body: some SwiftUI.View {
        Button { isReviewing = true } label: {
            Label {
                Text("Clean Up Inbox", bundle: .module)
            } icon: {
                Image(systemName: "sparkles")
            }
            #if os(iOS)
            .frame(maxWidth: .infinity, minHeight: 36)
            #endif
        }
        .buttonStyle(.borderedProminent)
        .controlSize(.large)
        .accessibilityIdentifier("inbox-cleanup")
        .sheet(isPresented: $isReviewing) {
            InboxCleanupReview(load: load, apply: apply)
        }
    }
}

private struct InboxCleanupDraft: Identifiable {
    var change: InboxCleanupChange
    var isSelected = true
    var id: String { change.taskId }

    var hasChanges: Bool {
        change.title != change.originalTitle || !change.links.isEmpty || !change.tagIds.isEmpty
    }
}

private struct InboxCleanupReview: SwiftUI.View {
    @Environment(\.dismiss) private var dismiss
    let load: @MainActor () async -> InboxCleanupInput?
    let apply: @MainActor ([InboxCleanupChange]) async -> Outcome?
    @State private var input: InboxCleanupInput?
    @State private var drafts: [InboxCleanupDraft] = []
    @State private var failure: String?
    @State private var isLoading = true
    @State private var isApplying = false
    @State private var refinement: Task<Void, Never>?
    @State private var refresh = 0

    var body: some SwiftUI.View {
        #if os(macOS)
        VStack(spacing: 0) {
            Text("Review Inbox Cleanup", bundle: .module)
                .font(.title2.weight(.semibold))
                .padding()
            content
            Divider()
            HStack {
                cancelButton
                Spacer()
                applyButton
            }
            .padding()
        }
        .frame(minWidth: 460, idealWidth: 560, minHeight: 360, idealHeight: 490)
        .task(id: refresh) { await prepare() }
        .onDisappear { refinement?.cancel() }
        #else
        NavigationStack {
            content
                .navigationTitle(Text("Review Inbox Cleanup", bundle: .module))
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) { cancelButton }
                    ToolbarItem(placement: .confirmationAction) { applyButton }
                }
        }
        .interactiveDismissDisabled(isApplying)
        .task(id: refresh) { await prepare() }
        .onDisappear { refinement?.cancel() }
        #endif
    }

    @ViewBuilder private var content: some SwiftUI.View {
        if isLoading {
            ProgressView {
                Text("Preparing suggestions…", bundle: .module)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else if drafts.isEmpty && failure == nil && refinement == nil {
            ContentUnavailableView {
                Label {
                    Text("No cleanup suggestions", bundle: .module)
                } icon: {
                    Image(systemName: "tray")
                }
            }
        } else {
            List {
                if refinement != nil {
                    Section {
                        ProgressView {
                            Text("Refining suggestions on this device…", bundle: .module)
                        }
                        Button {
                            refinement?.cancel()
                            refinement = nil
                        } label: {
                            Text("Use Basic Suggestions", bundle: .module)
                        }
                    }
                }
                if let failure {
                    Section {
                        Text(failure).foregroundStyle(.secondary)
                        Button { refresh += 1 } label: {
                            Text("Refresh Preview", bundle: .module)
                        }
                    }
                }
                ForEach($drafts) { $draft in
                    Section {
                        Toggle(isOn: $draft.isSelected) {
                            Text(draft.change.originalTitle)
                        }
                        VStack(alignment: .leading, spacing: 8) {
                            TextField(text: $draft.change.title) {
                                Text("Proposed Title", bundle: .module)
                            }
                            if !draft.change.links.isEmpty {
                                Label {
                                    Text("Move to Notes", bundle: .module)
                                } icon: {
                                    Image(systemName: "link")
                                }
                                .font(.subheadline)
                                ForEach(draft.change.links, id: \.self) { link in
                                    Text(verbatim: link)
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                        .textSelection(.enabled)
                                }
                            }
                            if let input {
                                tagChoices(for: $draft, tags: input.availableTags(for: draft.change))
                            }
                        }
                        .disabled(!draft.isSelected)
                    }
                }
                .disabled(refinement != nil || isApplying)
            }
        }
    }

    @ViewBuilder private func tagChoices(for draft: Binding<InboxCleanupDraft>, tags: [TagRef]) -> some SwiftUI.View {
        let proposed = Set(draft.wrappedValue.change.tagIds)
        let suggested = tags.filter { proposed.contains($0.id) }
        if !suggested.isEmpty {
            Text("Tags to Add", bundle: .module)
                .font(.subheadline)
            ForEach(suggested, id: \.id) { tag in
                HStack {
                    Text(verbatim: tag.title)
                    Spacer()
                    Button {
                        draft.wrappedValue.change.tagIds.removeAll { $0 == tag.id }
                    } label: {
                        Image(systemName: "xmark.circle")
                    }
                    .buttonStyle(.borderless)
                    .accessibilityLabel(Text("Remove Tag", bundle: .module))
                    .accessibilityValue(Text(verbatim: tag.title))
                }
            }
        }
        let remaining = tags.filter { !proposed.contains($0.id) }
        if !remaining.isEmpty {
            Menu {
                ForEach(remaining, id: \.id) { tag in
                    Button {
                        draft.wrappedValue.change.tagIds.append(tag.id)
                    } label: {
                        Text(verbatim: tag.title)
                    }
                }
            } label: {
                Text("Add Tag", bundle: .module)
            }
        }
    }

    private var cancelButton: some SwiftUI.View {
        Button(role: .cancel) { dismiss() } label: {
            Text("Cancel", bundle: .module)
        }
        .disabled(isApplying)
        .keyboardShortcut(.cancelAction)
    }

    private var applyButton: some SwiftUI.View {
        Button {
            Task { await applyChanges() }
        } label: {
            Text("Apply Changes", bundle: .module)
        }
        .disabled(isLoading || isApplying || failure != nil || refinement != nil
                  || selected.isEmpty || selected.contains { $0.title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty })
        .keyboardShortcut(.defaultAction)
        .accessibilityIdentifier("inbox-cleanup-apply")
    }

    private var selected: [InboxCleanupChange] {
        drafts.filter { $0.isSelected && $0.hasChanges }.map(\.change)
    }

    private func prepare() async {
        isLoading = true
        failure = nil
        refinement?.cancel()
        refinement = nil
        guard let input = await load() else {
            if !Task.isCancelled {
                isLoading = false
                failure = String(localized: "Could not load Inbox cleanup.", bundle: .module)
            }
            return
        }
        guard !Task.isCancelled else { return }
        self.input = input
        drafts = Self.actionable(input.changes)
        isLoading = false
        if !input.changes.isEmpty, InboxCleanupIntelligence.isAvailable {
            refinement = Task {
                let changes = await InboxCleanupIntelligence.refine(input)
                guard !Task.isCancelled else { return }
                drafts = Self.actionable(changes)
                refinement = nil
            }
        }
    }

    private static func actionable(_ changes: [InboxCleanupChange]) -> [InboxCleanupDraft] {
        changes.map { InboxCleanupDraft(change: $0) }.filter(\.hasChanges)
    }

    private func applyChanges() async {
        guard !isApplying, !selected.isEmpty else { return }
        isApplying = true
        defer { isApplying = false }
        let outcome = await apply(selected)
        if outcome?.changed == true {
            dismiss()
        } else {
            failure = outcome?.message.map(Strings.message)
                ?? String(localized: "Could not apply Inbox cleanup. Refresh the preview and try again.", bundle: .module)
        }
    }
}
