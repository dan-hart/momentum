// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
import MomentumCore
import Testing
@testable import MomentumKit

@MainActor @Suite struct GroupingTests {
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
