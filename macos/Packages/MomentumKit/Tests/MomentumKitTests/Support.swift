// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// What every test here gets: an engine over a throwaway copy of the demo data, defaults
// and a Keychain service of its own, and no services running. Nothing touches the real
// store, the real preferences, the login Keychain or the network, so the suite is safe to
// run anywhere and finishes in about a second.
import Foundation
import MomentumCore
import Testing

@testable import MomentumKit

/// A data directory that deletes itself when the test ends.
final class TempDir {
    let url: URL
    init() {
        url = FileManager.default.temporaryDirectory
            .appendingPathComponent("momentum-tests-\(UUID().uuidString)")
        try? FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
    }
    deinit {
        try? FileManager.default.removeItem(at: url)
    }
}

/// One test's world: the engine, its state object, and the throwaway defaults behind them.
@MainActor
final class Harness {
    let dir = TempDir()
    let defaults: UserDefaults
    let suiteName: String
    let state: AppState
    let notifier = RecordingNotifier()
    let indexer = RecordingIndexer()

    init(demo: Bool = true) {
        suiteName = "momentum-tests-\(UUID().uuidString)"
        defaults = UserDefaults(suiteName: suiteName)!
        let path = dir.url.appendingPathComponent("store").path
        let engine = demo ? Engine.demo(dir: path) : Engine.open(dir: path)
        state = AppState(engine: engine, defaults: defaults,
                         keychain: Keychain(service: "momentum-tests-\(UUID().uuidString)"),
                         services: false)
        state.notifier = notifier
        state.indexer = indexer
    }

    deinit {
        // Only the suite name is touched here: a @MainActor type's deinit is nonisolated,
        // so it must not reach for the stored UserDefaults.
        let name = suiteName
        UserDefaults().removePersistentDomain(forName: name)
    }

    var engine: Engine { state.engine }

    /// The id of the demo task with this title.
    func id(_ title: String) -> String {
        guard let t = engine.allTasks().first(where: { $0.title == title }) else {
            Issue.record("no task titled “\(title)”")
            return ""
        }
        return t.id
    }
    func tagId(_ title: String) -> String {
        engine.tags().first { $0.title == title }?.id ?? ""
    }
    func projectId(_ title: String) -> String {
        engine.projects().first { $0.title == title }?.id ?? ""
    }
    /// The task ids the current view lists, in order.
    var rows: [String] { state.allRowIds }
    var sectionKinds: [SectionKind] { state.listing.sections.map(\.kind) }
    var toastTexts: [String] { state.toasts.map(\.text) }
}

@MainActor
final class RecordingNotifier: Notifier {
    var reminders: [ReminderDue] = []
    var summaries: [MorningSummary] = []
    var badges: [UInt32] = []
    func reminder(_ due: ReminderDue) { reminders.append(due) }
    func morningSummary(_ summary: MorningSummary) { summaries.append(summary) }
    func updateBadge(_ count: UInt32) { badges.append(count) }
}

@MainActor
final class RecordingIndexer: SearchIndexer {
    var indexed: [[TaskBrief]] = []
    func reindex(_ tasks: [TaskBrief]) { indexed.append(tasks) }
}

extension SectionKind {
    /// Section kinds compare by shape, ignoring a Day's label, so tests can say
    /// "there is a day section" without naming the day.
    var isDay: Bool {
        if case .day = self { return true }
        return false
    }
}
