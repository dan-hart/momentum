// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
import Foundation
import MomentumCore
import Testing
@testable import MomentumKit

@MainActor
@Suite struct TaskPresentation {
    @Test func subtitleFollowsProjectEstimateDayRepeatTags() throws {
        let h = Harness()
        var row = try #require(h.engine.taskRow(id: h.id("Dentist appointment")))
        row.day = dayLabel(day: dayOffset(day: today(), days: 1))
        row.repeat = .daily
        let parts = TaskSubtitle.parts(for: row)
        #expect(parts.map(\.kind) == [.project, .estimate, .schedule, .repeatSchedule] + row.tags.map { _ in .tag })
        #expect(parts[2].text.contains(Strings.day(row.day!)))
        #expect(parts[2].text.contains(Strings.time(row.time!)))
    }

    @Test func archiveNeverProvidesADragPayloadIncludingMixedSelections() throws {
        let h = Harness()
        let id = h.id("Dentist appointment")
        h.state.setDone(id, true)
        h.state.archiveDone()
        h.state.showSearch(for: "")
        h.state.go(to: .archive)
        #expect(h.state.dragTransfer(for: id) == nil)
        h.state.go(to: .today)
        let live = try #require(h.rows.first)
        h.state.selection = [live, id]
        #expect(h.state.dragTransfer(for: live)?.ids == [live])
    }

    @Test func spotlightOpensSearchWithTheCurrentTitleAndNoEditor() {
        let h = Harness()
        let id = h.id("Dentist appointment")
        #expect(h.state.showSearchForTask(id))
        #expect(h.state.view == .search)
        #expect(h.state.searchText == "Dentist appointment")
        #expect(h.rows.contains(id))
        #expect(h.state.sheet == nil)
        #expect(!h.state.showSearchForTask("removed-task"))
    }

    @Test func spotlightWaitsForEditorToSaveBeforeResolvingTheTitle() throws {
        let h = Harness()
        let id = h.id("Dentist appointment")
        h.state.sheet = .editTask(id)
        #expect(h.state.showSearchForTask(id))
        #expect(h.state.sheet == nil)
        var form = TaskFormModel(detail: try #require(h.engine.taskDetail(id: id)))
        form.title = "Renamed appointment"
        // SwiftUI runs the editor's onDisappear save before sheet onDismiss.
        h.state.saveTask(id, form.draft)
        h.state.sheetDismissed()
        #expect(h.state.searchText == "Renamed appointment")
        #expect(h.rows.contains(id))
    }

    @Test func spotlightPreservesCreationDraftUntilTheUserFinishesIt() {
        let h = Harness()
        let id = h.id("Dentist appointment")
        h.state.sheet = .newTask
        #expect(h.state.showSearchForTask(id))
        #expect(h.state.sheet == .newTask)
        #expect(h.state.view == .today)
        h.state.sheet = nil
        h.state.sheetDismissed()
        #expect(h.state.view == .search)
        #expect(h.rows.contains(id))
    }

    @Test func systemCreationPlansTodayOutsideTheCurrentViewAndRejectsEmptyInput() {
        let h = Harness(demo: false)
        h.state.go(to: .upcoming)
        #expect(!h.state.addTaskForToday("  "))
        #expect(h.state.addTaskForToday("Call dentist #health 30m"))
        let id = h.id("Call dentist")
        #expect(h.engine.taskDetail(id: id)?.dueDay == today())
        #expect(h.engine.taskDetail(id: id)?.estimateMs == 1_800_000)
        #expect(!h.indexer.indexed.isEmpty)
        #expect(h.engine.todayOpenCount() == 1)
    }
}

@MainActor
@Suite struct TaskFormMapping {
    @Test func newFormUsesCurrentProjectTagAndDayDefaults() {
        let h = Harness()
        let pid = h.projectId("Home")
        #expect(TaskFormModel(view: .project(id: pid), projects: h.engine.projects()).projectId == pid)
        let tag = h.tagId("urgent")
        #expect(TaskFormModel(view: .tag(id: tag), projects: h.engine.projects()).tagIds == [tag])
        for view in [MomentumCore.View.today, .morning, .tonight] {
            #expect(TaskFormModel(view: view, projects: h.engine.projects()).dueDay == today())
        }
        #expect(TaskFormModel(view: .upcoming, projects: h.engine.projects()).dueDay == nil)
    }

    @Test func editedFormRoundTripsTimeReminderAndTagOrderWithoutWritingOps() throws {
        let h = Harness()
        let id = h.id("Dentist appointment")
        h.state.addTag([id], name: "second")
        let detail = try #require(h.engine.taskDetail(id: id))
        let form = TaskFormModel(detail: detail)
        let before = h.engine.pendingCount()
        #expect(form.draft.time == detail.time)
        #expect(form.draft.reminderMinutesBefore == detail.reminderMinutesBefore)
        #expect(form.draft.tagIds == detail.tagIds)
        h.state.saveTask(id, form.draft)
        #expect(h.engine.pendingCount() == before)
    }

    @Test func formParsesTimeEstimateAndNewTagsAndDisarmsInvalidTime() {
        var form = TaskFormModel()
        form.title = "Dentist follow-up"
        form.timeText = "2:30 pm"
        form.reminder = 15
        form.estimateText = "1h 30m"
        form.newTags = " health, urgent , ,"
        #expect(form.canSave)
        #expect(form.draft.time == ClockTime(hour: 14, minute: 30))
        #expect(form.draft.estimateMs == 5_400_000)
        #expect(form.draft.newTags == ["health", "urgent"])
        form.timeText = "bad time"
        #expect(form.timeInvalid)
        #expect(form.draft.time == nil)
        #expect(form.draft.reminderMinutesBefore == nil)
        form.title = " \n "
        #expect(!form.canSave)
    }
}
