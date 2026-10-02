// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore
import MomentumKit

extension EngineWorker {
    public func importReminder(_ item: ReminderImportItem) throws -> Bool {
        let result = engine.importTaskOnce(sourceId: item.sourceID, draft: TaskDraft(
            title: item.title, projectId: "", dueDay: item.dueDay, time: nil,
            reminderMinutesBefore: nil, estimateMs: 0, notes: item.notes, tagIds: [], newTags: []), sourceAliases: item.sourceAliases)
        if case .saveFailed = result.outcome.message { throw RemindersImportIssue.save }
        return result.id != nil
    }
    public func reconcileReminder(_ item: ReminderImportItem) throws -> Bool {
        let result = engine.reconcileImportSource(sourceId: item.sourceID, sourceAliases: item.sourceAliases)
        if case .saveFailed = result.message { throw RemindersImportIssue.save }
        return result.changed
    }
}
