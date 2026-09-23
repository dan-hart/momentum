// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import MomentumMobile
import XCTest

/// Opt-in through MomentumPerformanceTests; never seeds a large store in the fast lane.
@MainActor final class SnapshotPerformanceTests: XCTestCase {
    func testFirstSnapshotAndRepeatedProjectionForLargeList() async throws {
        guard ProcessInfo.processInfo.environment["MOMENTUM_PERFORMANCE_TESTS"] == "1" else {
            throw XCTSkip("Run the dedicated MomentumPerformanceTests scheme")
        }
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent("momentum-perf-\(UUID())")
        defer { try? FileManager.default.removeItem(at: directory) }
        let count = 1_000
        let titles = (0..<count).map { "Performance task \($0)" }.joined(separator: "\n")
        do {
            let seedingWorker = try await EngineWorker.openChecked(directory: directory)
            let result = await seedingWorker.organize(.importText(titles, .today))
            XCTAssertTrue(result.changed)
        }
        // This is the cold user-visible data boundary: reopen persisted state, run startup
        // recurrence checks, cross the actor and create all metadata for the first render.
        let started = ContinuousClock.now
        let worker = try await EngineWorker.openChecked(directory: directory)
        let snapshot = await worker.snapshot(view: .today)
        let elapsed = started.duration(to: .now)
        let seconds = Double(elapsed.components.seconds)
            + Double(elapsed.components.attoseconds) / 1_000_000_000_000_000_000
        print("Cold 1,000-task Today load: \(seconds * 1_000) ms")
        XCTAssertEqual(snapshot.tasks.count, count)
        XCTAssertLessThan(seconds, 0.1, "The cold ordinary task load should stay below 100 ms")
        let options = XCTMeasureOptions()
        options.iterationCount = 5
        // Store creation, Rust persistence and the actor hop are outside this
        // measurement. It isolates repeated presentation reads on the main actor.
        measure(metrics: [XCTClockMetric(), XCTMemoryMetric()], options: options) {
            var visited = 0
            for _ in 0..<100 { visited += snapshot.tasks.count }
            XCTAssertEqual(visited, count * 100)
        }
    }
}
