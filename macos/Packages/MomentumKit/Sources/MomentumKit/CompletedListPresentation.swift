// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore

/// Legacy visible-done rows cannot prove that today's work was completed today.
public enum CompletedListPresentation: Equatable, Sendable {
    case dayCompletion, neutralDay, legacyList
    public static func resolve(view: MomentumCore.View, completion: DayCompletion) -> Self {
        guard view == .today || view == .morning || view == .tonight else { return .legacyList }
        return completion == .dayComplete || completion == .periodComplete ? .dayCompletion : .neutralDay
    }
}
