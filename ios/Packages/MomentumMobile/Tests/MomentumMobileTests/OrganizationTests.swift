// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import Testing
@testable import MomentumMobile

@Suite struct OrganizationTests {
    @Test func todayShortcutTogglesOneTaskButPlansASelection() async throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("momentum-today-shortcut-\(UUID())")
        defer { try? FileManager.default.removeItem(at: dir) }
        let worker = await EngineWorker.open(directory: dir)
        _ = await worker.perform(.add("First", .today))
        let first = try #require(await worker.lastAddedTaskID())
        _ = await worker.organize(.toggleToday([first]))
        #expect(await worker.taskMenu(first)?.plannedToday == false)
        _ = await worker.perform(.undo)
        #expect(await worker.taskMenu(first)?.plannedToday == true)
        _ = await worker.perform(.add("Second", .today))
        let second = try #require(await worker.lastAddedTaskID())
        _ = await worker.organize(.toggleToday([first]))
        _ = await worker.organize(.toggleToday([first, second]))
        #expect(await worker.taskMenu(first)?.plannedToday == true)
        #expect(await worker.taskMenu(second)?.plannedToday == true)
        _ = await worker.perform(.undo)
        #expect(await worker.taskMenu(first)?.plannedToday == false)
        #expect(await worker.taskMenu(second)?.plannedToday == true)
        #expect(await !worker.organize(.toggleToday([])).changed)
    }

    @Test func dayPeriodDestinationsFollowLastTaskMovement() async throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("momentum-period-\(UUID())")
        defer { try? FileManager.default.removeItem(at: dir) }
        let worker = await EngineWorker.open(directory: dir)
        let empty = await worker.snapshot(view: .today)
        #expect(!empty.sidebar.fixed.contains { $0.view == .morning || $0.view == .tonight })
        _ = await worker.perform(.add("Morning task #Morning", .today))
        let morning = await worker.snapshot(view: .morning)
        let task = try #require(morning.tasks.first)
        #expect(morning.viewExists)
        #expect(morning.sidebar.fixed.contains { $0.view == .morning })
        _ = await worker.organize(.slot([task.id], .tonight))
        let moved = await worker.snapshot(view: .morning)
        #expect(!moved.viewExists)
        #expect(!moved.sidebar.fixed.contains { $0.view == .morning })
        #expect(moved.sidebar.fixed.contains { $0.view == .tonight })
    }

    @Test func deletedContextsStopBeingNavigationDestinations() async throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("momentum-context-\(UUID())")
        defer { try? FileManager.default.removeItem(at: dir) }
        let worker = await EngineWorker.open(directory: dir)
        let project = try #require(await worker.createProject("Temporary project"))
        let tag = try #require(await worker.createTag("Temporary tag"))
        #expect(await worker.snapshot(view: .project(id: project)).viewExists)
        #expect(await worker.snapshot(view: .tag(id: tag)).viewExists)
        _ = await worker.organize(.deleteProject(project))
        _ = await worker.organize(.deleteTag(tag))
        #expect(await !worker.snapshot(view: .project(id: project)).viewExists)
        #expect(await !worker.snapshot(view: .tag(id: tag)).viewExists)
        #expect(await worker.snapshot(view: .today).viewExists)
    }

    @Test func removingCustomColorsPersistsForProjectsAndTags() async throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("momentum-context-colors-\(UUID())")
        defer { try? FileManager.default.removeItem(at: dir) }
        let worker = await EngineWorker.open(directory: dir)
        let project = try #require(await worker.createProject("Project"))
        let tag = try #require(await worker.createTag("Tag"))
        _ = await worker.organize(.updateProject(project, "Project", "#FF6600"))
        _ = await worker.organize(.updateTag(tag, "Tag", "#FF6600"))
        let colored = await worker.snapshot(view: .today)
        #expect(colored.projects.first { $0.id == project }?.color == "#ff6600")
        #expect(colored.tags.first { $0.id == tag }?.color == "#ff6600")
        _ = await worker.organize(.updateProject(project, "Project", ""))
        _ = await worker.organize(.updateTag(tag, "Tag", ""))
        let reopened = await EngineWorker.open(directory: dir)
        let cleared = await reopened.snapshot(view: .today)
        #expect(cleared.projects.first { $0.id == project }?.color == nil)
        #expect(cleared.tags.first { $0.id == tag }?.color == nil)
    }

    @Test func moveTagTomorrowAndUndoKeepTaskIdentity() async throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("momentum-organization-\(UUID())")
        defer { try? FileManager.default.removeItem(at: dir) }
        let worker = await EngineWorker.open(directory: dir)
        _ = await worker.perform(.add("Prepare trip", .today))
        let initial = await worker.snapshot(view: .today)
        let task = try #require(initial.tasks.first)
        let project = try #require(await worker.createProject("Travel"))
        _ = await worker.organize(.project([task.id], project))
        _ = await worker.organize(.tag([task.id], "Urgent"))
        let moved = await worker.snapshot(view: .project(id: project))
        #expect(moved.tasks.first?.id == task.id)
        #expect(moved.tasks.first?.tags.first?.title == "Urgent")
        _ = await worker.organize(.tomorrow([task.id]))
        let upcoming = await worker.snapshot(view: .upcoming)
        #expect(upcoming.tasks.first?.id == task.id)
        _ = await worker.perform(.undo)
        let today = await worker.snapshot(view: .today)
        #expect(today.tasks.first?.id == task.id)
    }

    @Test func groupingPreferencesAffectTheSameCoreListing() async throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("momentum-grouping-\(UUID())")
        defer { try? FileManager.default.removeItem(at: dir) }
        let worker = await EngineWorker.open(directory: dir)
        _ = await worker.perform(.add("Short #First #Second 15m", .today))
        let prefs = MomentumCore.Preferences(groupBy: .tag, sort: .manual, direction: .ascending,
            upcomingDays: 7, autoArchive: false, morningSummaryEnabled: false,
            morningSummaryTime: ClockTime(hour: 8, minute: 0))
        await worker.setPreferences(prefs)
        let snapshot = await worker.snapshot(view: .today)
        #expect(snapshot.tasks.count == 1)
        let group = try #require(snapshot.listing.sections.first?.group)
        if case .tag(_, let name, _) = group { #expect(name == "First") }
        else { Issue.record("Expected first-tag group") }
    }

    @Test func creatingDuplicateTagDoesNotMutateExistingMetadata() async throws {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent(
            "momentum-duplicate-tag-\(UUID())"
        )
        defer { try? FileManager.default.removeItem(at: dir) }
        let worker = await EngineWorker.open(directory: dir)
        let tag = try #require(await worker.createTag("outing"))
        _ = await worker.organize(.updateTag(tag, "outing", "#FF6600"))

        let result = try #require(await worker.createTagIfAbsent("OUTING"))
        #expect(result == ContextCreationResult(id: tag, created: false))

        let snapshot = await worker.snapshot(view: .today)
        #expect(snapshot.tags.count == 1)
        #expect(snapshot.tags.first?.title == "outing")
        #expect(snapshot.tags.first?.color == "#ff6600")
    }
}
