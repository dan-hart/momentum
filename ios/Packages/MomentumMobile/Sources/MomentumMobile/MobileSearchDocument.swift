// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore

/// A platform-neutral projection for private on-device system search adapters.
/// The shared Rust engine remains responsible for task state and supplies TaskBrief.
public struct MobileSearchDocument: Equatable, Sendable {
    public let id: String
    public let title: String
    public let project: String?
    public let dueDay: DayLabel?
    public let notes: String?

    public init(id: String, title: String, project: String?, dueDay: DayLabel?, notes: String?) {
        self.id = id
        self.title = title
        self.project = project
        self.dueDay = dueDay
        self.notes = notes
    }

    public static func project(_ tasks: [TaskBrief]) -> [Self] {
        tasks.lazy
            .filter { !$0.isDone }
            .map { Self(id: $0.id, title: $0.title, project: $0.project,
                        dueDay: $0.dueDay, notes: $0.notes) }
            .sorted { $0.id < $1.id }
    }
}
