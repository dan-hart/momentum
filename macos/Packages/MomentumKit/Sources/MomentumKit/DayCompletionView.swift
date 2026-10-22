// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore
import SwiftUI

/// Eligibility comes exclusively from the atomic core listing snapshot.
public struct DayCompletionView: SwiftUI.View {
    public let view: MomentumCore.View
    public let completion: DayCompletion
    @AppStorage(PrefKey.dynamicEmptyMessages) private var dynamic = true
    @State private var message = CompletionMessageSelection()
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.locale) private var locale
    public init(view: MomentumCore.View, completion: DayCompletion) {
        self.view = view; self.completion = completion
    }
    public var body: some SwiftUI.View {
        VStack(spacing: 12) {
            Image(systemName: view == .morning ? "sun.max" : view == .tonight ? "moon" : "checkmark.circle")
                .font(.largeTitle).foregroundStyle(.tint).accessibilityHidden(true)
            Text(title).font(.title2).accessibilityAddTraits(.isHeader)
            if dynamic, let id = message.id {
                Text(CompletionMessageSelection.text(id: id, locale: locale))
                    .multilineTextAlignment(.center).foregroundStyle(.secondary)
            }
            if !dynamic {
                Text(String(localized: "Take a moment to enjoy the space.", bundle: .module))
                    .multilineTextAlignment(.center).foregroundStyle(.secondary)
            }
            if CelebrationFeedback.presentation(dynamic: dynamic, completion: completion, reduceMotion: reduceMotion) != .hidden {
                CompletionCelebration()
            }
        }
        .frame(maxWidth: .infinity).padding(24)
        .task(id: "\(view)-\(completion)") {
            message.beginVisit(view: view, completion: completion)
            while !Task.isCancelled {
                try? await Task.sleep(for: .seconds(30))
                guard !Task.isCancelled else { return }
                message.update(view: view, completion: completion)
            }
        }
        .onChange(of: completion) { _, _ in message.update(view: view, completion: completion) }
    }
    private var title: String {
        let key = view == .morning ? "Morning list complete" : view == .tonight ? "Evening list complete" : "Today's list is complete"
        return Bundle.module.localizedString(forKey: key, value: nil, table: nil)
    }
}
