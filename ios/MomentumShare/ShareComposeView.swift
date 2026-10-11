// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
import SwiftUI

/// The one-screen compose sheet: the drafted title ready to edit, the link and notes it
/// will carry, and a single Add button.
struct ShareComposeView: View {
    @State var title: String
    let notes: String
    let url: String?
    let failure: String?
    let add: (String) -> Void
    let cancel: () -> Void
    @FocusState private var titleFocused: Bool

    private var canAdd: Bool {
        failure == nil && !title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    var body: some View {
        NavigationStack {
            Form {
                if let failure {
                    Section { Text(failure).foregroundStyle(.secondary) }
                } else {
                    Section {
                        TextField(String(localized: "Title"), text: $title, axis: .vertical)
                            .lineLimit(1...4)
                            .focused($titleFocused)
                            .submitLabel(.done)
                            .onSubmit { if canAdd { add(title) } }
                            .accessibilityIdentifier("share-title")
                    }
                    if !notes.isEmpty {
                        Section(String(localized: "Notes")) {
                            Text(notes)
                                .font(.callout)
                                .foregroundStyle(.secondary)
                                .lineLimit(6)
                                .textSelection(.enabled)
                        }
                    }
                }
            }
            .navigationTitle(String(localized: "New Task"))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(String(localized: "Cancel"), action: cancel)
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button(String(localized: "Add")) { add(title) }
                        .disabled(!canAdd)
                        .accessibilityIdentifier("share-add")
                }
            }
            .safeAreaInset(edge: .bottom) {
                if failure == nil {
                    Text(String(localized: "Momentum adds the task to Inbox when it next opens."))
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                        .multilineTextAlignment(.center)
                        .padding(.horizontal)
                        .padding(.bottom, 8)
                }
            }
        }
        .onAppear { if url != nil, title.isEmpty { titleFocused = true } }
    }
}
