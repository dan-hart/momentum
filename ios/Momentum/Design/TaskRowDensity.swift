// SPDX-License-Identifier: GPL-3.0-or-later
import SwiftUI

/// How much room each task row takes. Regular trims the system inset a little so more
/// tasks fit; Compact keeps only what a 44-point touch target and two lines of text need.
enum TaskRowDensity: String, CaseIterable, Identifiable {
    case regular, compact
    var id: String { rawValue }

    static func from(_ raw: String) -> TaskRowDensity { TaskRowDensity(rawValue: raw) ?? .regular }

    /// The List row's own insets (the system default is about 11 points top and bottom).
    var rowInsets: EdgeInsets {
        switch self {
        case .regular: EdgeInsets(top: 7, leading: 16, bottom: 7, trailing: 16)
        case .compact: EdgeInsets(top: 2, leading: 16, bottom: 2, trailing: 16)
        }
    }
    /// Space between the title and its metadata line.
    var subtitleSpacing: CGFloat {
        switch self {
        case .regular: 3
        case .compact: 1
        }
    }
    var titleLines: Int {
        switch self {
        case .regular: 3
        case .compact: 2
        }
    }
    /// The row's touch target: the full 44 points normally; Compact accepts a shorter row,
    /// the whole width stays tappable and the checkbox keeps a 44-point hit area.
    var minimumRowHeight: CGFloat {
        switch self {
        case .regular: 44
        case .compact: 36
        }
    }
    var subtitleLines: Int {
        switch self {
        case .regular: 2
        case .compact: 1
        }
    }
}
