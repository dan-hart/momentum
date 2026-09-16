// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
// Hostless counterparts to the remaining named GTK behavior tests in handoff §15.4.
// These exercise AppState and the real core over isolated stores, never native views.
import Foundation
import MomentumCore
import Testing
@testable import MomentumKit

@MainActor
@Suite struct MVPParity {
    @Test func addingFromTonightAppliesTodayAndEvening() throws {
        let h = Harness()
        h.state.go(to: .tonight)
        #expect(h.state.view == .tonight)
        h.state.addTask("Wind down")
        let id = h.id("Wind down")
        let detail = try #require(h.engine.taskDetail(id: id))
        #expect(detail.dueDay == today())
        #expect(detail.tagIds.contains(h.tagId("Evening")))
        #expect(h.engine.taskMenu(id: id)?.slot == .tonight)
        #expect(h.rows.contains(id))
    }

    @Test func addingFromATagViewAppliesItsTag() throws {
        let h = Harness()
        let urgent = h.tagId("urgent")
        #expect(!urgent.isEmpty)
        h.state.go(to: .tag(id: urgent))
        h.state.addTask("Fire drill")
        let id = h.id("Fire drill")
        let detail = try #require(h.engine.taskDetail(id: id))
        #expect(detail.tagIds.contains(urgent))
        #expect(h.rows.contains(id))
    }

    @Test func sortingByTitleEstimateAndDirectionChangesTheVisibleOrder() {
        let h = Harness(demo: false)
        h.state.addTask("Zulu 10m")
        h.state.addTask("Alpha 30m")
        h.state.addTask("Mike 20m")
        let zulu = h.id("Zulu"), alpha = h.id("Alpha"), mike = h.id("Mike")
        #expect(h.rows == [zulu, alpha, mike])
        for (key, direction, expected) in [
            ("title", "ascending", [alpha, mike, zulu]),
            ("title", "descending", [zulu, mike, alpha]),
            ("estimate", "ascending", [zulu, mike, alpha]),
            ("estimate", "descending", [alpha, mike, zulu]),
            ("manual", "ascending", [zulu, alpha, mike]),
        ] {
            h.defaults.set(key, forKey: PrefKey.taskSort)
            h.defaults.set(direction, forKey: PrefKey.sortDirection)
            h.state.preferencesChanged()
            #expect(h.rows == expected, "\(key), \(direction)")
        }
    }

    @Test func contextMenuFactsFollowMovesCompletionAndSubtaskIdentity() throws {
        let h = Harness(demo: false)
        h.state.addTask("Plan the release")
        let id = h.id("Plan the release")
        var menu = try #require(h.engine.taskMenu(id: id))
        #expect(menu.plannedToday && menu.topLevel && !menu.isDone && !menu.repeats)
        #expect(menu.slot == nil)

        h.state.moveToTomorrow([id])
        menu = try #require(h.engine.taskMenu(id: id))
        #expect(!menu.plannedToday, "the menu now offers Plan for Today")
        h.state.toggleSlot([id], .tonight)
        menu = try #require(h.engine.taskMenu(id: id))
        #expect(menu.plannedToday && menu.slot == .tonight)
        h.state.toggleSlot([id], .morning)
        #expect(h.engine.taskMenu(id: id)?.slot == .morning)
        h.state.setDone(id, true)
        #expect(h.engine.taskMenu(id: id)?.isDone == true)

        h.state.addSubtask(id, "Check notes")
        let child = try #require(h.engine.taskDetail(id: id)?.subTasks.first)
        let childMenu = try #require(h.engine.taskMenu(id: child.id))
        #expect(!childMenu.topLevel, "subtasks cannot independently move to a project or gain a repeat schedule")
        #expect(h.engine.taskMenu(id: "missing-task") == nil)
    }

    @Test func duplicatePreservesEditableContentButHasFreshOpenIdentity() throws {
        let h = Harness(demo: false)
        h.state.addProject("Work")
        let draft = TaskDraft(title: "Dentist appointment", projectId: h.projectId("Work"),
                              dueDay: today(), time: ClockTime(hour: 15, minute: 30),
                              reminderMinutesBefore: 30, estimateMs: 5_400_000,
                              notes: "Bring insurance card\nAsk about the next visit", tagIds: [], newTags: ["health", "important"])
        h.state.createTask(draft)
        let id = h.id("Dentist appointment")
        h.state.setDone(id, true)
        let original = try #require(h.engine.taskDetail(id: id))
        #expect(original.time == ClockTime(hour: 15, minute: 30))
        #expect(original.reminderMinutesBefore == 30)
        h.state.duplicate(id)
        let duplicateID = try #require(h.engine.lastAddedId())
        let copy = try #require(h.engine.taskDetail(id: duplicateID))
        #expect(copy.id != original.id)
        #expect(!copy.isDone && original.isDone)
        #expect(copy.title == original.title)
        #expect(copy.projectId == original.projectId)
        #expect(copy.notes == original.notes)
        #expect(copy.estimateMs == original.estimateMs)
        #expect(copy.tagIds == original.tagIds)
        #expect(copy.dueDay == original.dueDay)
        #expect(copy.time == original.time)
        #expect(copy.reminderMinutesBefore == original.reminderMinutesBefore)
        #expect(copy.parentId == nil && copy.subTasks.isEmpty && copy.repeat == nil)
        #expect(h.state.selection == [duplicateID])
        h.state.undo()
        #expect(h.engine.taskDetail(id: duplicateID) == nil)
        #expect(h.engine.taskDetail(id: id) == original)
    }

    @Test func preferencesReadbackUpdatesColorsAndEveryShortcutModifier() {
        let h = Harness()
        let before = h.engine.projects()
        for enabled in [false, true, false] {
            h.defaults.set(enabled, forKey: PrefKey.colorfulLabels)
            h.state.preferencesChanged()
            #expect(h.state.colorful == enabled)
            #expect(h.state.prefs.colorful == enabled)
            #expect(h.engine.projects() == before, "color preference affects presentation, not saved project colors")
        }
        for modifier in ModifierKey.allCases {
            h.defaults.set(modifier.rawValue, forKey: PrefKey.modifierKey)
            h.state.preferencesChanged()
            #expect(h.state.modifier == modifier)
            #expect(AppShortcut.newTask.binding(modifier: h.state.modifier).modifiers == modifier.modifiers)
        }
    }

    @Test func repeatStartupAndRepeatedSameDayTicksCreateOnlyOneInstance() throws {
        let h = Harness(demo: false)
        let fixture = try repeatFixture(h, cycle: .daily, missed: today())
        let before = h.engine.allTasks().count
        let startup = restartedState(h)
        let instance = "rpt_\(fixture)_\(today())"
        #expect(h.engine.taskDetail(id: instance)?.dueDay == today())
        #expect(startup.allRowIds.contains(instance))
        #expect(h.engine.allTasks().count == before + 1)
        startup.tick()
        startup.tick()
        let again = restartedState(h)
        #expect(again.allRowIds.contains(instance))
        #expect(h.engine.allTasks().count == before + 1)
        #expect(h.engine.spawnRepeats() == 0)
    }

    @Test func repeatStartupCatchesUpTheNewestMissedDayAndShowsItOverdue() throws {
        let h = Harness(demo: false)
        let missed = dayOffset(day: today(), days: -2)
        let config = try repeatFixture(h, cycle: .weekly, missed: missed)
        let startup = restartedState(h)
        let instance = "rpt_\(config)_\(missed)"
        let detail = try #require(h.engine.taskDetail(id: instance))
        #expect(detail.dueDay == missed, "catch-up keeps the missed date instead of moving it to today")
        let first = try #require(startup.listing.sections.first)
        #expect(first.kind == .plain && first.group == .today)
        #expect(first.rows.contains { if case .task(let row) = $0 { return row.id == instance }; return false })
        #expect(h.engine.taskDetail(id: "rpt_\(config)_\(today())") == nil)
        startup.tick()
        #expect(h.engine.allTasks().filter { $0.id == instance }.count == 1)
    }

    private func restartedState(_ h: Harness) -> AppState {
        AppState(engine: h.engine, defaults: h.defaults,
                 keychain: Keychain(service: "momentum-tests-\(UUID().uuidString)"), services: false)
    }

    /// Start from a real saved schedule, then import an isolated backup whose last-run
    /// marker predates today. This models an app that was closed, without a clock hook,
    /// sleeping, or changing the production scheduler's behavior.
    private func repeatFixture(_ h: Harness, cycle: RepeatCycle, missed: String) throws -> String {
        h.state.addTask("Scheduled planning")
        let taskID = h.id("Scheduled planning")
        var draft = try #require(h.engine.repeatDraft(taskId: taskID))
        draft.cycle = cycle
        draft.every = 1
        draft.startDate = dayOffset(day: today(), days: -60)
        let parts = missed.split(separator: "-").compactMap { Int($0) }
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = TimeZone(secondsFromGMT: 0)!
        let date = try #require(calendar.date(from: DateComponents(year: parts[0], month: parts[1], day: parts[2], hour: 12)))
        let weekday = calendar.component(.weekday, from: date) - 1
        draft.weekdays = (0..<7).map { $0 == weekday }
        #expect(h.state.saveRepeat(taskID, draft))
        let file = h.dir.url.appendingPathComponent("repeat-history.json")
        _ = try h.engine.exportBackup(path: file.path)
        var backup = try #require(JSONSerialization.jsonObject(with: Data(contentsOf: file)) as? [String: Any])
        var data = try #require(backup["data"] as? [String: Any])
        var table = try #require(data["taskRepeatCfg"] as? [String: Any])
        var entities = try #require(table["entities"] as? [String: Any])
        let configID = try #require(entities.keys.first)
        var config = try #require(entities[configID] as? [String: Any])
        config["lastTaskCreationDay"] = dayOffset(day: today(), days: -9)
        entities[configID] = config
        table["entities"] = entities
        data["taskRepeatCfg"] = table
        backup["data"] = data
        try JSONSerialization.data(withJSONObject: backup).write(to: file)
        _ = try h.engine.importBackup(path: file.path)
        return configID
    }
}
