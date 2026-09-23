// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore
import MomentumKit

extension EngineWorker {
    public func handle(_ action: TaskURLAction) -> Outcome {
        switch action {
        case .create(let text, let notes, let due):
            engine.addTaskWithNotes(text: text, notes: notes, due: due)
        case .complete(let title):
            engine.completeByTitle(title: title)
        }
    }
}
