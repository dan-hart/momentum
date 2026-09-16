// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// Tasks in Spotlight, so typing a task name anywhere on the Mac finds it — the macOS
// answer to the GNOME Shell search provider and the KRunner runner. Opening a result
// hands the task back through a `momentum://` URL.
import CoreSpotlight
import Foundation
import MomentumCore
import MomentumKit
import UniformTypeIdentifiers

@MainActor
final class SpotlightIndexer: SearchIndexer {
    private let index = CSSearchableIndex.default()
    private var lastIds: Set<String> = []
    private var pending: Task<Void, Never>?

    /// Replaces the index with the open tasks. Debounced: a burst of edits reindexes once.
    func reindex(_ tasks: [TaskBrief]) {
        pending?.cancel()
        pending = Task { [weak self] in
            try? await Task.sleep(for: .seconds(2))
            guard let self, !Task.isCancelled else { return }
            self.write(tasks.filter { !$0.isDone })
        }
    }

    private func write(_ tasks: [TaskBrief]) {
        let items = tasks.map { t -> CSSearchableItem in
            let attrs = CSSearchableItemAttributeSet(contentType: .content)
            attrs.title = t.title
            attrs.contentDescription = [t.project, t.dueDay.map(Strings.day)]
                .compactMap { $0 }
                .joined(separator: " · ")
            attrs.textContent = t.notes
            attrs.keywords = t.project.map { [$0] }
            let item = CSSearchableItem(uniqueIdentifier: t.id,
                                        domainIdentifier: "com.codedbydan.Momentum.task",
                                        attributeSet: attrs)
            return item
        }
        let ids = Set(tasks.map(\.id))
        let gone = Array(lastIds.subtracting(ids))
        lastIds = ids
        if !gone.isEmpty {
            index.deleteSearchableItems(withIdentifiers: gone) { error in
                if let error { NSLog("spotlight delete: \(error)") }
            }
        }
        guard !items.isEmpty else { return }
        index.indexSearchableItems(items) { error in
            if let error { NSLog("spotlight index: \(error)") }
        }
    }
}
