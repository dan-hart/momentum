// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
import Foundation
import MomentumCore

/// The editable form and its mapping to the shared core, independent of any window.
public struct TaskFormModel {
    public var title = ""
    public var projectId = ""
    public var dueDay: String?
    public var timeText = ""
    public var reminder: UInt32?
    public var estimateText = ""
    public var notes = ""
    public var tagIds: Set<String> = []
    public var newTags = ""
    private var originalTagIds: [String] = []

    public init() {}

    public init(view: MomentumCore.View, projects: [ProjectRef]) {
        projectId = projects.first?.id ?? ""
        if case .project(let id) = view { projectId = id }
        if view == .today || view == .morning || view == .tonight { dueDay = today() }
        if case .tag(let id) = view { tagIds = [id] }
    }

    public init(detail: TaskDetail) {
        title = detail.title
        projectId = detail.projectId
        dueDay = detail.dueDay
        // An editable clock must round-trip through the core parser in every locale.
        // Localized clock formatting remains appropriate for read-only row subtitles.
        timeText = detail.time.map { String(format: "%02d:%02d", $0.hour, $0.minute) } ?? ""
        reminder = detail.reminderMinutesBefore
        estimateText = detail.estimateMs > 0 ? formatEstimate(ms: detail.estimateMs) : ""
        notes = detail.notes
        tagIds = Set(detail.tagIds)
        originalTagIds = detail.tagIds
    }

    public var parsedTime: ClockTime? { parseTime(text: timeText) }
    public var timeInvalid: Bool {
        !timeText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty && parsedTime == nil
    }
    public var canSave: Bool { !title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }
    public var draft: TaskDraft {
        let retained = originalTagIds.filter { tagIds.contains($0) }
        let added = tagIds.subtracting(originalTagIds).sorted()
        return TaskDraft(
            title: title, projectId: projectId, dueDay: dueDay, time: parsedTime,
            reminderMinutesBefore: parsedTime == nil ? nil : reminder,
            estimateMs: parseEstimate(text: estimateText) ?? 0, notes: notes,
            tagIds: retained + added,
            newTags: newTags.split(separator: ",")
                .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }.filter { !$0.isEmpty })
    }
}
