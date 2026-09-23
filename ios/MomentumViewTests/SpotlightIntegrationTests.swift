// SPDX-License-Identifier: GPL-3.0-or-later
import CoreSpotlight
import MomentumMobile
import XCTest

@MainActor final class SpotlightIntegrationTests: XCTestCase {
    func testIndexerMapsPrivateTaskMetadataAndSkipsIdenticalSnapshots() async throws {
        let client = RecordingSystemSearchIndex()
        let indexer = MobileSpotlightIndexer(client: client)
        let documents = [
            MobileSearchDocument(id: "task-1", title: "Book train", project: "Travel",
                                 dueDay: nil, notes: "Window seat"),
        ]

        await indexer.replace(documents)
        await indexer.replace(documents)

        XCTAssertEqual(client.reconciliations.count, 1)
        XCTAssertTrue(client.updates.isEmpty)
        let item = try XCTUnwrap(client.reconciliations.first?.first)
        XCTAssertEqual(item.uniqueIdentifier, "task-1")
        XCTAssertEqual(item.domainIdentifier, MobileSpotlightIndexer.domain)
        XCTAssertEqual(item.attributeSet.title, "Book train")
        XCTAssertEqual(item.attributeSet.displayName, "Book train")
        XCTAssertEqual(item.attributeSet.textContent, "Window seat")
        XCTAssertEqual(item.attributeSet.keywords, ["Travel"])
        XCTAssertEqual(item.expirationDate, .distantFuture)
    }

    func testIndexerReplacesTheDomainWhenAllTasksDisappear() async {
        let client = RecordingSystemSearchIndex()
        let indexer = MobileSpotlightIndexer(client: client)

        await indexer.replace([
            MobileSearchDocument(id: "task-1", title: "One", project: nil, dueDay: nil, notes: nil),
        ])
        await indexer.replace([])

        XCTAssertEqual(client.reconciliations.map(\.count), [1])
        XCTAssertEqual(client.updates.count, 1)
        XCTAssertEqual(client.updates[0].deleted, ["task-1"])
        XCTAssertTrue(client.updates[0].indexed.isEmpty)
    }

    func testIndexerWritesOnlyChangedAndNewDocumentsAfterInitialReconciliation() async {
        let client = RecordingSystemSearchIndex()
        let indexer = MobileSpotlightIndexer(client: client)
        await indexer.replace([
            MobileSearchDocument(id: "a", title: "Alpha", project: nil, dueDay: nil, notes: nil),
            MobileSearchDocument(id: "b", title: "Beta", project: nil, dueDay: nil, notes: nil),
        ])
        await indexer.replace([
            MobileSearchDocument(id: "a", title: "Alpha renamed", project: nil, dueDay: nil, notes: nil),
            MobileSearchDocument(id: "c", title: "Gamma", project: nil, dueDay: nil, notes: nil),
        ])

        XCTAssertEqual(client.reconciliations.count, 1)
        XCTAssertEqual(client.updates.count, 1)
        XCTAssertEqual(client.updates[0].deleted, ["b"])
        XCTAssertEqual(Set(client.updates[0].indexed.map(\.uniqueIdentifier)), ["a", "c"])
    }

    func testIndexerSuppliesCurrentDocumentsForSystemReindexRequests() async throws {
        let client = RecordingSystemSearchIndex()
        let indexer = MobileSpotlightIndexer(client: client)
        await indexer.replace([
            MobileSearchDocument(id: "a", title: "Alpha", project: nil, dueDay: nil, notes: nil),
            MobileSearchDocument(id: "b", title: "Beta", project: nil, dueDay: nil, notes: nil),
        ])

        let all = try XCTUnwrap(client.reindexProvider)(nil)
        let selected = try XCTUnwrap(client.reindexProvider)(["b", "missing"])

        XCTAssertEqual(Set(all.map(\.uniqueIdentifier)), ["a", "b"])
        XCTAssertEqual(selected.map(\.uniqueIdentifier), ["b"])
    }

    func testModelCoalescesEditsAndCancelsPendingIndexWorkWhenInactive() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let name = "momentum-spotlight-scheduling-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name); try? FileManager.default.removeItem(at: directory) }
        let indexer = RecordingMobileSpotlightIndexer()
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults,
                                   spotlight: indexer, spotlightDebounce: .milliseconds(200))
        let automation = MobileTaskAutomation(model: model)

        _ = try await automation.create(title: "One")
        _ = try await automation.create(title: "Two")
        try await Task.sleep(for: .milliseconds(300))
        XCTAssertEqual(indexer.batches.last?.map(\.title).sorted(), ["One", "Two"])
        let settledCount = indexer.batches.count

        _ = try await automation.create(title: "Cancelled")
        model.setForeground(false)
        try await Task.sleep(for: .milliseconds(300))
        XCTAssertEqual(indexer.batches.count, settledCount)
    }

    func testActivityDecoderAcceptsOnlyCoreSpotlightTaskIdentifiers() {
        let activity = NSUserActivity(activityType: CSSearchableItemActionType)
        activity.addUserInfoEntries(from: [CSSearchableItemActivityIdentifier: "task-1"])
        XCTAssertEqual(MobileSpotlightActivity.taskID(from: activity), "task-1")

        let unrelated = NSUserActivity(activityType: "com.codedbydan.Momentum.unrelated")
        unrelated.addUserInfoEntries(from: [CSSearchableItemActivityIdentifier: "task-1"])
        XCTAssertNil(MobileSpotlightActivity.taskID(from: unrelated))
        XCTAssertNil(MobileSpotlightActivity.taskID(from: NSUserActivity(activityType: CSSearchableItemActionType)))
    }
}

@MainActor final class SpotlightSystemTests: XCTestCase {
    func testProtectedCustomIndexAddsUpdatesAndDeletesItems() async throws {
        guard ProcessInfo.processInfo.environment["MOMENTUM_SPOTLIGHT_NATIVE"] == "1" else {
            throw XCTSkip("Real Core Spotlight acceptance is opt-in.")
        }
        guard CSSearchableIndex.isIndexingAvailable() else {
            throw XCTSkip("Core Spotlight indexing is unavailable on this simulator.")
        }

        let client = CoreSpotlightIndexClient()
        let suffix = UUID().uuidString
        let retainedID = "momentum-spotlight-retained-\(suffix)"
        let removedID = "momentum-spotlight-removed-\(suffix)"
        defer { Task { try? await client.reconcile([]) } }

        try await client.reconcile([
            Self.item(id: retainedID, title: "Before \(suffix)"),
            Self.item(id: removedID, title: "Remove \(suffix)"),
        ])
        try await eventuallyFind(id: retainedID, title: "Before \(suffix)")
        try await eventuallyFind(id: removedID, title: "Remove \(suffix)")

        try await client.update(
            deleting: [removedID],
            indexing: [Self.item(id: retainedID, title: "After \(suffix)")]
        )
        try await eventuallyFind(id: retainedID, title: "After \(suffix)")
        try await eventuallyMissing(title: "Remove \(suffix)")

        try await client.reconcile([])
        try await eventuallyMissing(title: "After \(suffix)")
    }

    private static func item(id: String, title: String) -> CSSearchableItem {
        let attributes = CSSearchableItemAttributeSet(contentType: .content)
        attributes.title = title
        return CSSearchableItem(uniqueIdentifier: id,
                                domainIdentifier: MobileSpotlightIndexer.domain,
                                attributeSet: attributes)
    }

    private func eventuallyFind(id: String, title: String) async throws {
        for _ in 0..<50 {
            if try await query(title: title).contains(where: { $0.uniqueIdentifier == id }) { return }
            try await Task.sleep(for: .milliseconds(100))
        }
        XCTFail("Core Spotlight did not return the expected item.")
        throw SpotlightTestError.timedOut
    }

    private func eventuallyMissing(title: String) async throws {
        for _ in 0..<50 {
            if try await query(title: title).isEmpty { return }
            try await Task.sleep(for: .milliseconds(100))
        }
        XCTFail("Core Spotlight did not remove the item.")
        throw SpotlightTestError.timedOut
    }

    private func query(title: String) async throws -> [CSSearchableItem] {
        let results = SearchResults()
        let context = CSSearchQueryContext()
        context.fetchAttributes = ["title", "domainIdentifier", "uniqueIdentifier"]
        context.filterQueries = ["domainIdentifier == \"\(MobileSpotlightIndexer.domain)\""]
        let query = CSSearchQuery(queryString: "title == \"\(title)\"c", queryContext: context)
        query.protectionClasses = [.completeUntilFirstUserAuthentication]
        return try await withCheckedThrowingContinuation { continuation in
            query.foundItemsHandler = { results.append($0) }
            query.completionHandler = { error in
                _ = query
                if let error { continuation.resume(throwing: error) }
                else { continuation.resume(returning: results.value) }
            }
            query.start()
        }
    }
}

private enum SpotlightTestError: Error {
    case timedOut
}

private final class SearchResults: @unchecked Sendable {
    private let lock = NSLock()
    private var items: [CSSearchableItem] = []

    func append(_ newItems: [CSSearchableItem]) {
        lock.withLock { items.append(contentsOf: newItems) }
    }

    var value: [CSSearchableItem] {
        lock.withLock { items }
    }
}

@MainActor private final class RecordingSystemSearchIndex: SystemSearchIndexClient {
    struct Update {
        let deleted: [String]
        let indexed: [CSSearchableItem]
    }
    var reconciliations: [[CSSearchableItem]] = []
    var updates: [Update] = []
    var reindexProvider: SystemSearchReindexProvider?
    func reconcile(_ items: [CSSearchableItem]) async throws { reconciliations.append(items) }
    func update(deleting identifiers: [String], indexing items: [CSSearchableItem]) async throws {
        updates.append(Update(deleted: identifiers, indexed: items))
    }
}

@MainActor private final class RecordingMobileSpotlightIndexer: MobileSpotlightIndexing {
    var batches: [[MobileSearchDocument]] = []
    func replace(_ documents: [MobileSearchDocument]) async { batches.append(documents) }
}
