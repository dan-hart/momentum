// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore
import MomentumKit

/// What importing one shared item did.
public enum SharedTaskImport: Equatable, Sendable {
    /// A task was created.
    case created
    /// Nothing to do: the item was imported before, or has no title.
    case skipped
}

public enum SharedTaskImportIssue: Error, Equatable {
    /// The store could not be saved; the item stays in the inbox for the next try.
    case save
}

extension EngineWorker {
    /// Creates the task for a shared item exactly once, in Inbox, with the shared notes.
    public func importSharedTask(_ item: SharedTaskItem) throws -> SharedTaskImport {
        let result = engine.importTaskOnce(sourceId: item.sourceID, draft: TaskDraft(
            title: item.title, projectId: "", dueDay: nil, time: nil, reminderMinutesBefore: nil,
            estimateMs: 0, notes: item.notes, tagIds: [], newTags: []), sourceAliases: [])
        if case .saveFailed = result.outcome.message { throw SharedTaskImportIssue.save }
        return result.id == nil ? .skipped : .created
    }
}
