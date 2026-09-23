// SPDX-License-Identifier: GPL-3.0-or-later
import CoreSpotlight
import Foundation
import MomentumKit
import MomentumMobile
import UniformTypeIdentifiers

enum MobileSpotlightActivity {
    static func taskID(from activity: NSUserActivity) -> String? {
        guard activity.activityType == CSSearchableItemActionType,
              let id = activity.userInfo?[CSSearchableItemActivityIdentifier] as? String,
              !id.isEmpty else { return nil }
        return id
    }
}

typealias SystemSearchReindexProvider = @MainActor @Sendable ([String]?) -> [CSSearchableItem]

@MainActor protocol SystemSearchIndexClient: AnyObject {
    var reindexProvider: SystemSearchReindexProvider? { get set }
    func reconcile(_ items: [CSSearchableItem]) async throws
    func update(deleting identifiers: [String], indexing items: [CSSearchableItem]) async throws
}

@MainActor protocol MobileSpotlightIndexing: AnyObject {
    func replace(_ documents: [MobileSearchDocument]) async
}

/// Keep the system Spotlight index behind one main-actor client so deletion and
/// replacement remain one ordered operation. The app-owned domain limits every
/// reconciliation to Momentum's tasks without using a private named index.
@MainActor final class CoreSpotlightIndexClient: SystemSearchIndexClient {
    private let index: CSSearchableIndex
    private let reindexDelegate = CoreSpotlightReindexDelegate()

    var reindexProvider: SystemSearchReindexProvider? {
        get { reindexDelegate.provider }
        set { reindexDelegate.provider = newValue }
    }

    init() {
        index = CSSearchableIndex.default()
        index.indexDelegate = reindexDelegate
    }

    func reconcile(_ items: [CSSearchableItem]) async throws {
        guard CSSearchableIndex.isIndexingAvailable() else { return }
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, any Error>) in
            index.deleteSearchableItems(withDomainIdentifiers: [MobileSpotlightIndexer.domain]) { error in
                if let error { continuation.resume(throwing: error) }
                else { continuation.resume() }
            }
        }
        guard !items.isEmpty else { return }
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, any Error>) in
            index.indexSearchableItems(items) { error in
                if let error { continuation.resume(throwing: error) }
                else { continuation.resume() }
            }
        }
    }

    func update(deleting identifiers: [String], indexing items: [CSSearchableItem]) async throws {
        guard CSSearchableIndex.isIndexingAvailable() else { return }
        if !identifiers.isEmpty {
            try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, any Error>) in
                index.deleteSearchableItems(withIdentifiers: identifiers) { error in
                    if let error { continuation.resume(throwing: error) }
                    else { continuation.resume() }
                }
            }
        }
        guard !items.isEmpty else { return }
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, any Error>) in
            index.indexSearchableItems(items) { error in
                if let error { continuation.resume(throwing: error) }
                else { continuation.resume() }
            }
        }
    }
}

/// Replaces Momentum's protected on-device task domain. Identical projections are
/// ignored so foreground transitions and a burst of edits do not churn the index.
@MainActor final class MobileSpotlightIndexer: MobileSpotlightIndexing {
    static let domain = "com.codedbydan.Momentum.task"
    private let client: any SystemSearchIndexClient
    private var current: [MobileSearchDocument]?

    init(client: any SystemSearchIndexClient = CoreSpotlightIndexClient()) {
        self.client = client
        client.reindexProvider = { [weak self] identifiers in
            self?.items(for: identifiers) ?? []
        }
    }

    func replace(_ documents: [MobileSearchDocument]) async {
        guard documents != current else { return }
        do {
            if let current {
                let oldByID = Dictionary(uniqueKeysWithValues: current.map { ($0.id, $0) })
                let newIDs = Set(documents.map(\.id))
                let removed = oldByID.keys.filter { !newIDs.contains($0) }.sorted()
                let changed = documents.filter { oldByID[$0.id] != $0 }.map(Self.item)
                guard !removed.isEmpty || !changed.isEmpty else { return }
                try await client.update(deleting: removed, indexing: changed)
            } else {
                try await client.reconcile(documents.map(Self.item))
            }
            current = documents
        } catch {
            // Never include task metadata in logs.
            NSLog("spotlight index update failed: %@", String(describing: error))
        }
    }

    private static func item(_ document: MobileSearchDocument) -> CSSearchableItem {
        let attributes = CSSearchableItemAttributeSet(contentType: .content)
        attributes.title = document.title
        attributes.displayName = document.title
        attributes.contentDescription = [document.project, document.dueDay.map(Strings.day)]
            .compactMap { $0 }
            .joined(separator: " · ")
        attributes.textContent = document.notes
        attributes.keywords = document.project.map { [$0] }
        let item = CSSearchableItem(uniqueIdentifier: document.id,
                                    domainIdentifier: domain,
                                    attributeSet: attributes)
        item.expirationDate = .distantFuture
        return item
    }

    private func items(for identifiers: [String]?) -> [CSSearchableItem] {
        guard let current else { return [] }
        guard let identifiers else { return current.map(Self.item) }
        let requested = Set(identifiers)
        return current.filter { requested.contains($0.id) }.map(Self.item)
    }
}

/// Services recovery requests without making normal foreground transitions rewrite
/// an unchanged index. The provider is main-actor isolated with the app snapshot.
@MainActor private final class CoreSpotlightReindexDelegate: NSObject, CSSearchableIndexDelegate {
    var provider: SystemSearchReindexProvider?

    nonisolated func searchableIndex(
        _ searchableIndex: CSSearchableIndex,
        reindexAllSearchableItemsWithAcknowledgementHandler acknowledgementHandler: @escaping () -> Void
    ) {
        let request = SpotlightReindexRequest(index: searchableIndex, acknowledgement: acknowledgementHandler)
        Task { @MainActor [weak self] in
            try? await request.index.indexSearchableItems(self?.provider?(nil) ?? [])
            request.acknowledgement()
        }
    }

    nonisolated func searchableIndex(
        _ searchableIndex: CSSearchableIndex,
        reindexSearchableItemsWithIdentifiers identifiers: [String],
        acknowledgementHandler: @escaping () -> Void
    ) {
        let request = SpotlightReindexRequest(index: searchableIndex, acknowledgement: acknowledgementHandler)
        Task { @MainActor [weak self] in
            try? await request.index.indexSearchableItems(self?.provider?(identifiers) ?? [])
            request.acknowledgement()
        }
    }
}

/// Core Spotlight's Objective-C delegate contract predates Sendable annotations.
/// The system owns both values for the duration of this callback.
private final class SpotlightReindexRequest: @unchecked Sendable {
    let index: CSSearchableIndex
    let acknowledgement: () -> Void

    init(index: CSSearchableIndex, acknowledgement: @escaping () -> Void) {
        self.index = index
        self.acknowledgement = acknowledgement
    }
}
