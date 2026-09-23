// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import Testing
@testable import MomentumMobile

@Suite struct LifecycleTests {
    @Test func nextRefreshFollowsLocalMidnightAcrossDST() throws {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = try #require(TimeZone(identifier: "America/Chicago"))
        for (month, day, hours) in [(3, 8, 23.0), (11, 1, 25.0)] {
            let start = try #require(calendar.date(from: DateComponents(year: 2026, month: month, day: day)))
            let next = try #require(MobileLifecycle.nextDay(after: start, calendar: calendar))
            #expect(next.timeIntervalSince(start) == hours * 3600)
            #expect(calendar.component(.hour, from: next) == 0)
        }
    }

    @Test func concurrentBootstrapUsesOneEngineOwner() async throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("momentum-bootstrap-\(UUID())")
        defer { try? FileManager.default.removeItem(at: dir) }
        let session = EngineSession()
        async let first = session.open(directory: dir)
        async let second = session.open(directory: dir)
        let (a, b) = try await (first, second)
        #expect(a === b)
        _ = await a.perform(.add("One process owner", .today))
        #expect(await b.snapshot(view: .today).tasks.count == 1)
    }
    @Test func failedBootstrapPreservesDataAndAllowsRetry() async throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("momentum-recovery-\(UUID())")
        defer { try? FileManager.default.removeItem(at: dir) }
        let seed = try await EngineWorker.openChecked(directory: dir)
        _ = await seed.perform(.add("Preserved task", .today))
        let before = try Data(contentsOf: dir.appendingPathComponent("state.json"))
        let journal = dir.appendingPathComponent(".momentum-transaction")
        try FileManager.default.createDirectory(at: journal, withIntermediateDirectories: false)
        try Data("{}".utf8).write(to: journal.appendingPathComponent("manifest.json"))
        let session = EngineSession()
        do {
            _ = try await session.open(directory: dir)
            Issue.record("Unreadable recovery record should fail startup")
        } catch { }
        #expect(try Data(contentsOf: dir.appendingPathComponent("state.json")) == before)
        // Remove only the injected fault; production never deletes a failed recovery record.
        try FileManager.default.removeItem(at: journal)
        let reopened = try await session.open(directory: dir)
        #expect(await reopened.snapshot(view: .today).tasks.map(\.title) == ["Preserved task"])
        let again = try await session.open(directory: dir)
        #expect(reopened === again)
    }

}
