// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
import Foundation
import MomentumCore
import Testing
@testable import MomentumKit

@MainActor @Suite struct GroupingTests {
    @Test func hidingDayPeriodsRefreshesTodayAndReconcilesSelection() throws {
        let h = Harness()
        let morning = try #require(h.state.listing.sections.first { $0.group == .morning })
        let hidden = morning.rows.compactMap { row -> String? in
            if case .task(let task) = row { return task.id }; return nil
        }
        try #require(!hidden.isEmpty)
        let task = try #require(h.state.listing.sections.first { $0.group == .today }?.rows.compactMap { row -> TaskRow? in
            if case .task(let task) = row { return task }; return nil
        }.first)
        h.state.selection = Set(hidden + [task.id])
        h.defaults.set(false, forKey: "show-day-period-tasks-in-today")
        h.state.preferencesChanged()
        #expect(Set(h.rows).isDisjoint(with: hidden))
        #expect(h.state.selection == [task.id])
        h.state.go(to: .morning)
        #expect(Set(h.rows) == Set(hidden))
        h.state.go(to: .today)
        h.defaults.set(true, forKey: "show-day-period-tasks-in-today")
        h.state.preferencesChanged()
        #expect(Set(hidden).isSubset(of: Set(h.rows)))
    }

    @Test func dayPeriodBoundaryQueryHandlesEmptyAndOutOfRangeBorrowedListings() {
        let h = Harness()
        let listing = h.state.listing
        #expect(!dayPeriodSeparatorBefore(listing: listing, sectionIndex: UInt32.max, groupBy: .morningNight))
        var empty = listing
        empty.sections = []
        #expect(!dayPeriodSeparatorBefore(listing: empty, sectionIndex: 0, groupBy: .morningNight))
        #expect(!dayPeriodSeparatorBefore(listing: empty, sectionIndex: UInt32.max, groupBy: .morningNight))
    }

    @Test func changingGroupingRefreshesTheListAndPreservesTaskIdentity() {
        let h = Harness()
        let original = h.state.listing
        let ids = h.rows
        let pending = h.engine.pendingCount()
        for choice in ["project", "tag", "estimate"] {
            h.defaults.set(choice, forKey: PrefKey.groupBy)
            h.state.preferencesChanged()
            #expect(h.rows.sorted() == ids.sorted())
            #expect(Set(h.rows).count == h.rows.count)
            #expect(h.state.listing.sections.allSatisfy { $0.group != nil })
            #expect(h.state.listing.sections.allSatisfy { Strings.sectionTitle($0)?.isEmpty == false })
            #expect(h.state.listing.sections.allSatisfy { $0.kind == .plain })
        }
        #expect(h.engine.pendingCount() == pending)
        h.defaults.set("none", forKey: PrefKey.groupBy)
        h.state.preferencesChanged()
        #expect(h.state.listing.sections.count == 1)
        #expect(h.state.listing.sections.first?.group == nil)
        #expect(h.rows.sorted() == ids.sorted())
        h.defaults.set("morning-night", forKey: PrefKey.groupBy)
        h.state.preferencesChanged()
        #expect(h.state.listing == original)
    }

    @Test func groupedHeadingsUseLiteralUserNamesAndLocalizedDayPeriods() {
        let group = TaskGroup.project(id: "p", title: "R&D <Home>", color: "#33CCAA")
        let section = MomentumCore.Section(group: group, kind: .searchTasks, count: 2, rows: [], note: nil)
        #expect(Strings.sectionTitle(section) == "Tasks (2) · R&D <Home>")
        #expect(Strings.sectionTitle(MomentumCore.Section(group: group, kind: .plain, count: 2, rows: [], note: nil)) == "R&D <Home>")
        #expect(Strings.groupTitle(.tag(id: "urgent", title: "Urgent", color: "#FF6600")) == "#Urgent")
        #expect(Strings.groupTitle(.tag(id: "literal", title: "R&D <Home>", color: nil)) == "#R&D <Home>")
        #expect([TaskGroup.today, .morning, .evening].map(Strings.groupTitle) == ["Today", "Morning", "Evening"])
        let ranges: [EstimateRange] = [.upTo15Minutes, .upTo30Minutes, .upTo60Minutes, .upTo2Hours, .over2Hours, .noEstimate]
        let labels = ranges.map { Strings.groupTitle(.estimate(range: $0)) }
        #expect(Set(labels).count == 6)
        #expect(labels == ["Up to 15 min", "16–30 min", "31–60 min", "1–2 hours", "Over 2 hours", "No estimate"])
    }
}

@MainActor @Suite struct DayCompletionPresentationTests {
    @Test func autoArchivedMorningCompletionKeepsTheCurrentDestination() throws {
        let h = Harness()
        h.defaults.set(true, forKey: PrefKey.autoArchive)
        h.state.preferencesChanged()
        h.state.go(to: .morning)
        let tasks = h.rows
        try #require(!tasks.isEmpty)
        h.state.toggleDone(tasks)
        #expect(h.state.view == .morning)
        #expect(h.state.dayStatus.completion == .periodComplete || h.state.dayStatus.completion == .dayComplete)
    }
    @Test func hiddenCountsAndVisibleRowsRefreshTogetherAfterPreferencesChange() {
        let h = Harness()
        h.defaults.set(false, forKey: PrefKey.showDayPeriodTasksInToday)
        h.state.preferencesChanged()
        #expect(h.state.dayStatus.hiddenMorning > 0)
        #expect(h.state.dayStatus.completion == .pending)
        #expect(!h.state.listing.sections.contains { $0.group == .morning || $0.group == .evening })
        h.defaults.set(true, forKey: PrefKey.showDayPeriodTasksInToday)
        h.state.preferencesChanged()
        #expect(h.state.dayStatus.hiddenMorning == 0)
        #expect(h.state.dayStatus.hiddenEvening == 0)
        #expect(h.state.listing.sections.contains { $0.group == .morning })
    }
}
