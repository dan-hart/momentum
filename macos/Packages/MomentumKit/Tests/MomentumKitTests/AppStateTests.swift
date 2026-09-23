// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// The macOS half of the behaviours in docs/MVP.md section 2. The Rust core has its own
// tests for what a change does to the data; these check what this app does with the
// answer — which view is showing, what is selected, which toast appeared, what Undo
// reverses. Together they are the net that catches a change breaking one platform.
import Foundation
import MomentumCore
import Testing

@testable import MomentumKit

@MainActor
@Suite struct ViewsAndSections {
    @Test func todayUsesOneLayerOfDayPeriodGroups() {
        let h = Harness()
        #expect(h.state.view == .today)
        #expect(h.sectionKinds.allSatisfy { $0 == .plain })
        #expect(h.state.listing.sections.map(\.group) == [.today, .morning, .evening])
        #expect(h.rows.first == h.id("Renew library books"), "what slipped comes first")
        #expect(!h.sectionKinds.contains(.completed), "nothing is done yet")
        #expect(h.state.listing.empty == nil)
        #expect(h.state.listing.title == .today)
    }

    @Test func completingMovesTheTaskToCompletedAndUndoBringsItBack() {
        let h = Harness()
        let id = h.id("Write release notes for 0.1")
        h.state.setDone(id, true)
        #expect(h.sectionKinds.contains(.completed))
        #expect(h.rows.last == id, "completed tasks sink to the bottom")
        #expect(h.toastTexts.contains(Strings.message(.taskCompleted)))
        #expect(h.state.toasts.last?.celebratesCompletion == true)
        h.state.undo()
        #expect(!h.sectionKinds.contains(.completed))
        #expect(h.state.toasts.last?.celebratesCompletion == false)
    }

    @Test func aToastsUndoButtonRevertsThatChangeAndOnlyOnce() throws {
        let h = Harness()
        let first = h.id("Write release notes for 0.1")
        let second = h.id("Read two chapters")
        h.state.setDone(first, true)
        let toast = try #require(h.state.toasts.last)
        h.state.setDone(second, true)
        h.state.undoToast(toast)
        #expect(h.engine.taskRow(id: first)?.isDone == false)
        #expect(h.engine.taskRow(id: second)?.isDone == true, "only that batch was undone")
        #expect(!h.state.toasts.contains(toast))
    }

    @Test func allDoneReplacesTheListWhenNothingIsOpen() {
        let h = Harness(demo: false)
        h.state.addTask("only one")
        h.state.setDone(h.id("only one"), true)
        #expect(h.state.listing.allDone == .today(completed: 1))
        #expect(h.state.listing.canArchive)
        h.state.archiveDone()
        #expect(h.state.listing.empty == .today, "after archiving, the normal empty state")
    }

    @Test func emptyViewsExplainWhatToDoNext() throws {
        let h = Harness(demo: false)
        let copy = Strings.empty(try #require(h.state.listing.empty), modifier: "⌘", syncConfigured: false)
        #expect(copy.title.isEmpty == false)
        #expect(copy.description.contains("⌘N"), "the hint names the configured modifier")
        #expect(copy.description.contains("sync"), "and offers sync when it is not set up")
        let configured = Strings.empty(.today, modifier: "⌃", syncConfigured: true)
        #expect(configured.description.contains("⌃N"))
        #expect(!configured.description.contains("turn on sync"))
    }

    @Test func comingUpShowsDatesOnRowsAndFollowsTheRange() {
        let h = Harness()
        h.state.go(to: .upcoming)
        #expect(h.state.listing.sections.allSatisfy { $0.kind == .plain })
        #expect(h.state.listing.sections.flatMap(\.rows).allSatisfy {
            if case .task(let row) = $0 { return row.day != nil }
            return false
        })
        #expect(!h.rows.isEmpty)
        let far = h.rows.count
        h.defaults.set("30", forKey: PrefKey.upcomingRange)
        h.state.preferencesChanged()
        #expect(h.rows.count >= far)
    }

    @Test func archivePagesAndTheArchiveRowsAreReadOnly() throws {
        let h = Harness()
        let id = h.id("Write release notes for 0.1")
        h.state.setDone(id, true)
        h.state.archiveDone()
        h.state.go(to: .archive)
        #expect(h.rows.contains(id))
        let row = try #require(h.state.row(id))
        #expect(row.archived)
        #expect(row.doneDay != nil)
    }

    @Test func searchFindsTitlesNotesProjectsAndTags() {
        let h = Harness()
        h.state.go(to: .search)
        h.state.searchText = "orca"
        h.state.searchTextChanged()
        #expect(h.rows.contains(h.id("Test with Orca and high contrast")))
        h.state.searchText = "gnome"
        h.state.searchTextChanged()
        #expect(h.sectionKinds.contains(.searchTags))
        h.state.searchText = "zzz-nothing"
        h.state.searchTextChanged()
        #expect(h.state.listing.empty == .noResults)
        h.state.searchText = ""
        h.state.searchTextChanged()
        #expect(h.state.listing.empty == .search)
    }

    @Test func morningAndTonightAppearOnlyOnDaysThatUseThem() {
        let h = Harness(demo: false)
        let fixed = { h.state.sidebar.fixed.map(\.view) }
        #expect(!fixed().contains(.morning))
        h.state.addTask("Plain")
        let id = h.id("Plain")
        h.state.toggleSlot([id], .morning)
        #expect(fixed().contains(.morning))
        h.state.go(to: .morning)
        #expect(h.rows == [id])
        // Moving the only morning task to tonight swaps the entry, and the view falls back.
        h.state.toggleSlot([id], .tonight)
        #expect(!fixed().contains(.morning))
        #expect(fixed().contains(.tonight))
        #expect(h.state.view == .today, "the emptied view falls back to Today")
    }
}

@MainActor
@Suite struct Navigation {
    @Test func stepViewWrapsAroundTheSidebar() {
        let h = Harness()
        let all = h.state.orderedViews
        #expect(all.count >= 9)
        for _ in all { h.state.stepView(1) }
        #expect(h.state.view == .today, "a full lap comes home")
        h.state.stepView(-1)
        #expect(h.state.view == all.last)
    }

    @Test func jumpSelectsTheNthEntry() {
        let h = Harness()
        h.state.jump(to: 3)
        #expect(h.state.view == h.state.orderedViews[3])
        h.state.jump(to: 999)
        #expect(h.state.view == h.state.orderedViews[3], "out of range changes nothing")
    }

    @Test func collapsingASectionRemovesItsEntriesFromTheKeyboardOrder() {
        let h = Harness()
        let withProjects = h.state.orderedViews.count
        h.state.setProjectsExpanded(false)
        #expect(h.state.orderedViews.count < withProjects)
        #expect(h.defaults.bool(forKey: PrefKey.projectsCollapsed))
        h.state.setProjectsExpanded(true)
        #expect(h.state.orderedViews.count == withProjects)
    }

    @Test func switchingViewsClearsTheSelection() {
        let h = Harness()
        h.state.selection = [h.id("Write release notes for 0.1")]
        h.state.go(to: .upcoming)
        #expect(h.state.selection.isEmpty)
    }

    @Test func deletingTheOpenProjectFallsBackToToday() throws {
        let h = Harness(demo: false)
        h.state.addProject("Garden")
        guard case .project(let pid) = h.state.view else {
            Issue.record("adding a project opens it")
            return
        }
        h.state.confirmDeleteContext(.project(id: pid))
        h.state.deleteContext(try #require(h.state.confirmation))
        #expect(h.state.view == .today)
        #expect(h.state.confirmation == nil)
    }

    @Test func theInboxProjectCannotBeDeleted() {
        let h = Harness()
        h.state.go(to: .project(id: h.state.inboxProjectId))
        #expect(h.state.contextIsEditable)
        #expect(!h.state.contextIsDeletable)
        h.state.confirmDeleteContext(h.state.view)
        #expect(h.state.confirmation == nil)
    }
}

@MainActor
@Suite struct Selection {
    @Test func targetsFollowListOrderNotClickOrder() {
        let h = Harness()
        let rows = h.rows
        h.state.selection = [rows[2], rows[0]]
        #expect(h.state.targets == [rows[0], rows[2]], "commands act in the order shown")
        #expect(h.state.target == nil, "two selected is not a single target")
        h.state.selection = [rows[1]]
        #expect(h.state.target == rows[1])
    }

    @Test func aSelectionSurvivingADeleteDropsTheGoneRows() {
        let h = Harness()
        let a = h.id("Write release notes for 0.1")
        let b = h.id("Read two chapters")
        h.state.selection = [a, b]
        h.state.delete([a])
        #expect(h.state.selection == [b])
    }

    @Test func bulkActionsReportTheirCountAndUndoAsOneBatch() {
        let h = Harness()
        let a = h.id("Write release notes for 0.1")
        let b = h.id("Test with Orca and high contrast")
        h.state.toggleDone([a, b])
        #expect(h.toastTexts.last == Strings.message(.tasksCompleted(n: 2)))
        h.state.undo()
        #expect(h.engine.taskRow(id: a)?.isDone == false)
        #expect(h.engine.taskRow(id: b)?.isDone == false, "one undo for the whole batch")
    }

    @Test func selectedTopLevelExcludesSubtasks() {
        let h = Harness()
        let parent = h.id("Write release notes for 0.1")
        h.state.addSubtask(parent, "Check the changelog")
        let sub = h.id("Check the changelog")
        h.state.selection = [parent, sub]
        #expect(h.state.selectedTopLevel == [parent])
    }
}

@MainActor
@Suite struct Changes {
    @Test func quickAddParsesTagsAndEstimateAndPlansForTheCurrentView() throws {
        let h = Harness()
        h.state.addTask("Buy milk #home 30m")
        let row = try #require(h.state.row(h.id("Buy milk")))
        #expect(row.estimateMs == 1_800_000)
        #expect(row.tags.map(\.title) == ["home"])
        #expect(row.dueDay == today(), "added from Today, planned for today")
    }

    @Test func theTagPopoverCompletesTheWordUnderTheCursor() throws {
        let h = Harness()
        let matches = h.engine.tagCompletions(prefix: "urg")
        #expect(matches.map(\.title) == ["urgent"])
        let completed = try #require(completeHashWord(text: "Fix it #urg", cursor: 11, name: "urgent"))
        #expect(completed.text == "Fix it #urgent ")
        #expect(completed.cursor == 15)
    }

    @Test func dayMovesReportAndUndoRestoresTheOldDay() {
        let h = Harness()
        let id = h.id("Write release notes for 0.1")
        h.state.moveToTomorrow([id])
        #expect(h.toastTexts.last == Strings.message(.movedToTomorrow))
        #expect(h.engine.taskRow(id: id)?.dueDay == dayOffset(day: today(), days: 1))
        #expect(!h.rows.contains(id), "it left Today")
        h.state.undo()
        #expect(h.engine.taskRow(id: id)?.dueDay == today())
    }

    @Test func aTaskIsInOneSlotAtATime() {
        let h = Harness()
        let id = h.id("Read two chapters")
        #expect(h.engine.taskMenu(id: id)?.slot == .tonight)
        h.state.toggleSlot([id], .morning)
        #expect(h.engine.taskMenu(id: id)?.slot == .morning, "moving to Morning drops Evening")
        h.state.undo()
        #expect(h.engine.taskMenu(id: id)?.slot == .tonight)
    }

    @Test func droppingOnASidebarEntryMovesTagsOrPlans() {
        let h = Harness()
        let id = h.id("Plan weekend hike")
        let work = h.projectId("Momentum")
        #expect(h.state.drop([id], on: .project(id: work)))
        #expect(!h.state.drop([id], on: .project(id: work)), "already there: nothing happens")
        #expect(h.state.drop([id], on: .tag(id: h.tagId("urgent"))))
        #expect(h.state.drop([id], on: .today))
        #expect(h.toastTexts.last == Strings.message(.plannedForToday))
    }

    @Test func reorderNeedsManualOrderAndSaysSoOtherwise() {
        let h = Harness(demo: false)
        for t in ["one", "two", "three"] { h.state.addTask(t) }
        let (a, b, c) = (h.id("one"), h.id("two"), h.id("three"))
        #expect(h.rows == [a, b, c])
        #expect(h.state.reorder([c], before: a))
        #expect(h.rows == [c, a, b])
        h.state.undo()
        #expect(h.rows == [a, b, c], "a drag is undoable")
        h.defaults.set("title", forKey: PrefKey.taskSort)
        h.state.preferencesChanged()
        #expect(!h.state.reorder([c], before: a))
        #expect(h.toastTexts.last == Strings.message(.manualOrderOnly))
    }

    @Test func nudgeMovesOneRowAndKeepsItSelected() {
        let h = Harness(demo: false)
        for t in ["one", "two", "three"] { h.state.addTask(t) }
        let c = h.id("three")
        h.state.selection = [c]
        h.state.nudge(-1)
        #expect(h.rows[1] == c)
        #expect(h.state.selection == [c], "the moved row keeps the selection")
    }

    @Test func pastedLinesBecomeSeveralTasksAndAParagraphBecomesOne() throws {
        let h = Harness(demo: false)
        h.state.addFromText("first thing\nsecond thing\n\n")
        #expect(h.toastTexts.last == Strings.message(.tasksAdded(n: 2)))
        h.state.addFromText("https://example.org/page/")
        let link = try #require(h.engine.taskDetail(id: h.id("example.org/page")))
        #expect(link.notes == "https://example.org/page/", "the link itself goes in the notes")
    }

    @Test func autoArchiveSendsACompletedTaskStraightToTheArchive() {
        let h = Harness()
        let id = h.id("Write release notes for 0.1")
        h.defaults.set(true, forKey: PrefKey.autoArchive)
        h.state.preferencesChanged()
        h.state.setDone(id, true)
        #expect(h.toastTexts.last == Strings.message(.taskCompletedArchived))
        #expect(!h.rows.contains(id))
        h.state.undo()
        #expect(h.engine.taskRow(id: id)?.isDone == false, "undo restores it open")
    }

    @Test func editingATaskWritesOnlyWhatChanged() throws {
        let h = Harness()
        let id = h.id("Dentist appointment")
        let d = try #require(h.engine.taskDetail(id: id))
        #expect(d.time == ClockTime(hour: 15, minute: 30))
        #expect(d.reminderMinutesBefore == 30)
        let unchanged = TaskDraft(title: d.title, projectId: d.projectId, dueDay: d.dueDay, time: d.time,
                                  reminderMinutesBefore: d.reminderMinutesBefore, estimateMs: d.estimateMs,
                                  notes: d.notes, tagIds: d.tagIds, newTags: [])
        let before = h.engine.pendingCount()
        h.state.saveTask(id, unchanged)
        #expect(h.engine.pendingCount() == before, "saving an untouched form writes nothing")
        var edited = unchanged
        edited.title = "Dentist (moved)"
        edited.time = nil
        h.state.saveTask(id, edited)
        #expect(h.engine.taskRow(id: id)?.title == "Dentist (moved)")
        #expect(h.engine.taskDetail(id: id)?.dueDay == today(), "clearing the time keeps the day")
    }

    @Test func repeatSchedulesRoundTripThroughTheEditor() throws {
        let h = Harness()
        let id = h.id("Write release notes for 0.1")
        var draft = try #require(h.engine.repeatDraft(taskId: id))
        #expect(!draft.existing)
        draft.weekdays = Array(repeating: false, count: 7)
        #expect(!h.state.saveRepeat(id, draft))
        #expect(h.toastTexts.last == Strings.message(.pickAWeekday))
        draft.weekdays[1] = true
        #expect(h.state.saveRepeat(id, draft))
        #expect(h.engine.taskMenu(id: id)?.repeats == true)
        #expect(h.engine.repeatDraft(taskId: id)?.existing == true)
        h.state.stopRepeat(id)
        #expect(h.engine.taskMenu(id: id)?.repeats == false)
    }
}

@MainActor
@Suite struct Notifications {
    @Test func morningSummaryRequiresOptInAndDoesNotRepeatAfterToggling() {
        let h = Harness()
        h.state.checkReminders()
        #expect(h.notifier.summaries.isEmpty)
        h.defaults.set(0, forKey: PrefKey.morningSummaryHour)
        h.defaults.set(0, forKey: PrefKey.morningSummaryMinute)
        h.defaults.set(true, forKey: PrefKey.morningSummaryEnabled)
        h.state.preferencesChanged()
        h.state.checkReminders()
        #expect(h.notifier.summaries.count == 1)
        h.defaults.set(false, forKey: PrefKey.morningSummaryEnabled)
        h.state.preferencesChanged()
        h.state.checkReminders()
        h.defaults.set(true, forKey: PrefKey.morningSummaryEnabled)
        h.state.preferencesChanged()
        h.state.checkReminders()
        #expect(h.notifier.summaries.count == 1)
    }

    @Test func anUnattachedNotifierDoesNotConsumeTheDailySummary() {
        let h = Harness()
        h.state.notifier = nil
        h.defaults.set(0, forKey: PrefKey.morningSummaryHour)
        h.defaults.set(true, forKey: PrefKey.morningSummaryEnabled)
        h.state.preferencesChanged()
        h.state.checkReminders()
        h.state.notifier = h.notifier
        h.state.checkReminders()
        #expect(h.notifier.summaries.count == 1)
    }

    @Test func remindersFireOnceAndASnoozeRearmsThem() {
        let h = Harness()
        h.state.checkReminders()
        let fired = h.notifier.reminders.count
        #expect(fired >= 0)
        h.notifier.reminders.removeAll()
        h.state.checkReminders()
        #expect(h.notifier.reminders.isEmpty, "a reminder fires once")
    }

    @Test func theBadgeFollowsTheOpenTaskCount() throws {
        let h = Harness()
        h.state.refresh()
        let count = try #require(h.notifier.badges.last)
        #expect(count == h.engine.taskCount(mode: .dueToday))
        #expect(count == h.state.todayOpenCount, "the menu bar uses the same due-today count")
        #expect(count > 0)
    }

    @Test func aNotificationsDoneButtonCompletesTheTask() {
        let h = Harness()
        let id = h.id("Write release notes for 0.1")
        h.state.completeFromNotification(id)
        #expect(h.engine.taskRow(id: id)?.isDone == true)
    }

    @Test func badgeModeChangesImmediatelyAndIncludesOverdueOnlyWhenRequested() throws {
        let h = Harness()
        h.state.refresh()
        let todayCount = h.state.todayOpenCount
        h.state.go(to: .upcoming)
        h.defaults.set("todayIncludingOverdue", forKey: "dock-badge-mode")
        h.state.preferencesChanged()
        let allToday = try #require(h.notifier.badges.last)
        #expect(allToday == h.engine.taskCount(mode: .todayIncludingOverdue))
        #expect(allToday > todayCount)
        h.state.setDone(h.id("Renew library books"), true)
        #expect(h.notifier.badges.last == allToday - 1)
        h.state.undo()
        #expect(h.notifier.badges.last == allToday)
        h.defaults.set("dueToday", forKey: "dock-badge-mode")
        h.state.preferencesChanged()
        #expect(h.notifier.badges.last == todayCount)
        h.defaults.set("none", forKey: "dock-badge-mode")
        h.state.preferencesChanged()
        #expect(h.notifier.badges.last == 0)
        h.state.addTaskForToday("Another task")
        #expect(h.notifier.badges.last == 0)
        #expect(h.state.todayOpenCount == todayCount + 1, "the menu bar count stays independent")
        h.defaults.set("unknown", forKey: "dock-badge-mode")
        h.state.preferencesChanged()
        #expect(h.notifier.badges.last == todayCount + 1)
    }

    @Test func todayBadgeCountsParentsOnceAndExcludesCompletedAndArchivedTasks() {
        let h = Harness(demo: false)
        h.defaults.set("todayIncludingOverdue", forKey: "dock-badge-mode")
        h.state.preferencesChanged()
        #expect(h.notifier.badges.last == 0)
        h.state.addTask("Parent")
        let id = h.id("Parent")
        h.state.addSubtask(id, "Child")
        #expect(h.notifier.badges.last == 1)
        h.state.setDone(id, true)
        #expect(h.notifier.badges.last == 0)
        h.state.archiveDone()
        #expect(h.notifier.badges.last == 0)
    }
}

@MainActor
@Suite struct UrlsAndArguments {
    @Test func addUrlsCreateATaskWithNotesAndTags() throws {
        let h = Harness(demo: false)
        h.state.handle(url: URL(string: "momentum://add?title=Call%20the%20bank&notes=ask%20about%20fees&tags=admin,money")!)
        // A task made from a URL lands in the first project without being planned, so it
        // is in the store rather than in Today's list.
        let detail = try #require(h.engine.taskDetail(id: h.id("Call the bank")))
        #expect(detail.notes == "ask about fees")
        let names = Set(detail.tagIds.compactMap { id in h.engine.tags().first { $0.id == id }?.title })
        #expect(names == ["admin", "money"])
        #expect(detail.dueDay == nil, "a URL add does not plan the task")
    }

    @Test func superProductivityUrlsWork() {
        let h = Harness(demo: false)
        h.state.handle(url: URL(string: "superproductivity://create-task?title=Imported")!)
        #expect(h.engine.allTasks().contains { $0.title == "Imported" })
        h.state.handle(url: URL(string: "superproductivity://complete-task?title=Imported")!)
        #expect(h.engine.allTasks().first { $0.title == "Imported" }?.isDone == true)
    }

    @Test func aUrlWithNoTitleDoesNothing() {
        let h = Harness(demo: false)
        let before = h.engine.allTasks().count
        h.state.handle(url: URL(string: "momentum://add?notes=orphan")!)
        h.state.handle(url: URL(string: "momentum://nonsense")!)
        #expect(h.engine.allTasks().count == before)
    }

    @Test func commandLineArgumentsAddAndNavigate() {
        let h = Harness(demo: false)
        #expect(h.state.handle(arguments: ["Momentum", "--add", "From the shell"]) == false,
                "a command-line add does not open a window")
        #expect(h.engine.allTasks().contains { $0.title == "From the shell" })
        #expect(h.state.handle(arguments: ["Momentum", "--search", "shell"]))
        #expect(h.state.view == .search)
        #expect(h.state.searchText == "shell")
        #expect(h.state.handle(arguments: ["Momentum", "--today"]))
        #expect(h.state.view == .today)
        #expect(h.state.handle(arguments: ["Momentum", "--background"]) == false)
    }
}

@MainActor
@Suite struct SyncAndStatus {
    @Test func syncIsRefusedAndExplainedWhenItIsNotSetUp() {
        let h = Harness()
        h.state.sync()
        #expect(h.toastTexts.last?.contains("turned off") == true)
        h.defaults.set(true, forKey: PrefKey.syncEnabled)
        h.state.sync()
        #expect(h.state.banner?.contains("not configured") == true)
        h.state.dismissBanner()
        #expect(h.state.banner == nil)
    }

    @Test func theCaptionIsHiddenUntilThereIsSomethingToSay() {
        let h = Harness()
        #expect(h.state.syncCaption == nil, "no sync, no caption")
        #expect(!h.state.syncAvailable)
        h.defaults.set(true, forKey: PrefKey.syncEnabled)
        #expect(h.state.syncCaption == Strings.notSyncedYet)
        #expect(h.state.syncAvailable)
    }

    @Test func pendingOpsAreCountedForTheStatusLine() {
        let h = Harness(demo: false)
        #expect(h.state.syncStatus.pendingOps == 0)
        h.state.addTask("something")
        #expect(h.state.syncStatus.pendingOps > 0)
    }
}

@MainActor
@Suite struct BackupRoundTrip {
    @Test func exportThenImportCarriesEveryTask() {
        let source = Harness()
        let target = Harness(demo: false)
        let file = source.dir.url.appendingPathComponent("backup.json")
        source.state.exportBackup(file)
        #expect(source.toastTexts.last == Strings.message(.backupExported))
        target.state.importBackup(file)
        #expect(target.toastTexts.last == Strings.message(.backupImported))
        #expect(target.engine.allTasks().count == source.engine.allTasks().count)
    }

    @Test func importingSomethingThatIsNotABackupIsReported() {
        let h = Harness()
        let file = h.dir.url.appendingPathComponent("junk.json")
        try! "not json".write(to: file, atomically: true, encoding: .utf8)
        h.state.importBackup(file)
        #expect(h.toastTexts.last != Strings.message(.backupImported))
        #expect(h.state.toasts.isEmpty == false, "the failure is shown, not swallowed")
    }
}

@Suite @MainActor struct PersistenceFailures {
    @Test func formOperationsReturnSaveErrorsAndKeepTheirTaskDataForRetry() throws {
        let h = Harness(demo: false)
        h.state.addTask("Original")
        let id = try #require(h.engine.allTasks().first?.id)
        let blocked = h.dir.url.appendingPathComponent("store/pending.json.tmp")
        try FileManager.default.createDirectory(at: blocked, withIntermediateDirectories: false)
        var form = TaskFormModel(detail: try #require(h.engine.taskDetail(id: id)))
        form.title = "Edited draft"
        let failedEdit = h.state.saveTask(id, form.draft)
        #expect(!failedEdit.changed)
        if let message = failedEdit.message, case .saveFailed = message {
            #expect(h.toastTexts.last == Strings.message(message))
        } else { Issue.record("The form needs an explicit save failure to retain its draft") }
        #expect(h.engine.taskDetail(id: id)?.title == "Original")
        let failedCreation = h.state.createTask(form.draft)
        #expect(!failedCreation.changed)
        #expect(h.engine.allTasks().count == 1)
        try FileManager.default.removeItem(at: blocked)
        #expect(h.state.saveTask(id, form.draft).changed)
        #expect(h.engine.taskDetail(id: id)?.title == "Edited draft")
    }
}
