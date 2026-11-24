// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation

/// Reuses the old visible order after a task action moves or removes rows.
public enum TaskMutationFocus {
    public static func replacement(before: [String], after: [String], affected: [String]) -> String? {
        let changed = Set(affected)
        guard let anchor = before.firstIndex(where: changed.contains) else { return nil }
        let eligible = Set(after).subtracting(changed)
        return before.dropFirst(anchor + 1).first(where: eligible.contains)
            ?? before.prefix(anchor).last(where: eligible.contains)
            ?? after.first(where: eligible.contains)
            ?? after.first(where: changed.contains)
    }
}
