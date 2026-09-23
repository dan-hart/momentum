// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore

public enum TaskNotificationAction: Sendable, Hashable { case complete, snooze }

extension EngineWorker {
    /// OS callbacks and the UI share this actor. Duplicate callbacks in this process
    /// cannot mutate twice; this is not a durable cross-process delivery receipt.
    public func performNotificationAction(_ action: TaskNotificationAction, taskID: String,
                                          deliveryID: String) -> Outcome? {
        guard handledNotificationActions[deliveryID]?.contains(action) != true else { return nil }
        let result: Outcome = switch action {
        case .complete: engine.bulkDone(ids: [taskID])
        case .snooze: engine.snooze(id: taskID, minutes: 60)
        }
        if result.changed { handledNotificationActions[deliveryID, default: []].insert(action) }
        return result
    }
}
