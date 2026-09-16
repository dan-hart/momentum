// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
import Foundation
import MomentumCore

public enum TaskSubtitle {
    public enum Kind { case project, estimate, schedule, repeatSchedule, completed, tag }
    public struct Part {
        public let kind: Kind
        public let text: String
        public let color: String?
        init(_ kind: Kind, _ text: String, color: String? = nil) {
            self.kind = kind; self.text = text; self.color = color
        }
    }

    /// The order is shared by visual and accessible row descriptions.
    public static func parts(for row: TaskRow) -> [Part] {
        var parts: [Part] = []
        if let p = row.project { parts.append(Part(.project, p.title, color: p.color)) }
        if row.estimateMs > 0 { parts.append(Part(.estimate, "~" + Strings.estimate(row.estimateMs))) }
        let when = [row.day.map(Strings.day), row.time.map(Strings.time)].compactMap { $0 }.joined(separator: " ")
        if !when.isEmpty { parts.append(Part(.schedule, when)) }
        if let r = row.repeat { parts.append(Part(.repeatSchedule, Strings.repeatText(r))) }
        if row.archived, let done = row.doneDay {
            parts.append(Part(.completed, String(localized: "Done \(Strings.day(done))", bundle: .module)))
        }
        for tag in row.tags { parts.append(Part(.tag, "#" + tag.title, color: tag.color)) }
        return parts
    }
}
