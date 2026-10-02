// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore

extension EngineWorker {
    public func importReminder(_ item: ReminderImportItem) throws -> Bool {
        let result = engine.importTaskOnce(sourceId: item.sourceID, draft: TaskDraft(
            title: item.title, projectId: "", dueDay: item.dueDay, time: nil,
            reminderMinutesBefore: nil, estimateMs: 0, notes: item.notes, tagIds: [], newTags: []), sourceAliases: item.sourceAliases)
        if case .saveFailed = result.outcome.message { throw RemindersImportIssue.save }
        return result.outcome.changed
    }
}
