// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
import Foundation
import MomentumCore

public enum MorningSummarySection: UInt8, CaseIterable, Identifiable, Sendable {
    case today = 1
    case morning = 2
    case evening = 4
    case overdue = 8

    public var id: Self { self }

    public var label: String {
        switch self {
        case .today: String(localized: "Today", bundle: .module)
        case .morning: String(localized: "Morning", bundle: .module)
        case .evening: String(localized: "Evening", bundle: .module)
        case .overdue: String(localized: "Overdue", bundle: .module)
        }
    }

    fileprivate func count(in summary: MorningSummary) -> UInt32 {
        switch self {
        case .today: summary.total
        case .morning: summary.morning
        case .evening: summary.tonight
        case .overdue: summary.overdue
        }
    }
}

public extension MorningSummary {
    var displayText: String {
        let selection = MorningSummarySection.allCases.filter { sections & $0.rawValue != 0 }
        guard selection.contains(where: { $0.count(in: self) > 0 }) else {
            return String(localized: "No tasks in the selected sections today.", bundle: .module)
        }
        let rows = selection.map { section in
            String(localized: "\(section.label): \(section.count(in: self).formatted())", bundle: .module)
        }
        return ListFormatter.localizedString(byJoining: rows)
    }
}
