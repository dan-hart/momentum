// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
import Foundation
import MomentumCore
import Testing

@testable import MomentumKit

struct InboxCleanupIntelligenceTests {
    private let baseline = InboxCleanupChange(
        taskId: "fixture", sourceRevision: "fixture-revision",
        originalTitle: "read docs https://example.test/device-setup?fixture=secret",
        title: "Read docs",
        links: ["https://example.test/device-setup?fixture=secret"],
        tagIds: ["reading"]
    )

    @Test func localModelCanOnlyAddSuppliedTagsAndRefineTheTitle() {
        let result = InboxCleanupIntelligence.refined(
            baseline, title: "Read the device setup documentation", numbers: [1, 2, 4], tags: [
                TagRef(id: "reading", title: "Reading", color: nil),
                TagRef(id: "devices", title: "Devices", color: nil),
            ]
        )
        #expect(result.title == "Read the device setup documentation")
        #expect(result.tagIds == ["reading", "devices"])
        #expect(result.links == baseline.links)
        #expect(result.taskId == baseline.taskId)
        #expect(result.sourceRevision == baseline.sourceRevision)
        #expect(result.originalTitle == baseline.originalTitle)
    }

    @Test(arguments: ["", "Look at https://example.test", "Line 1\nLine 2"])
    func retainsOfflineSuggestionsWhenOutputIsUnusable(_ title: String) {
        let result = InboxCleanupIntelligence.refined(baseline, title: title, numbers: [-1, 0, 1], tags: [])
        #expect(result == baseline)
    }

    @Test(.enabled(if: ProcessInfo.processInfo.environment["MOMENTUM_INBOX_MODEL_SMOKE"] == "1"))
    func supportedSystemModelCanSuggestAnExistingTag() async throws {
        #expect(InboxCleanupIntelligence.isAvailable)
        let store = TempDir()
        let engine = Engine.open(dir: store.url.path)
        let health = try #require(engine.addTag(title: "Health"))
        _ = engine.addTag(title: "Software development")
        _ = engine.addTask(text: "call dentist to reschedule my cleaning", view: .project(id: "INBOX_PROJECT"))
        let input = InboxCleanupInput(engine: engine)
        let original = try #require(input.changes.first)
        #expect(original.tagIds.isEmpty)
        let suggestions = await InboxCleanupIntelligence.refine(input)
        let suggestion = try #require(suggestions.first)
        #expect(suggestion.tagIds == [health])
        #expect(suggestion.links == original.links)
        #expect(suggestion.sourceRevision == original.sourceRevision)
    }

}
