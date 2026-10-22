// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore

/// A manual celebration is available only for genuine completed scope.
public enum CelebrationFeedback: Equatable, Sendable {
    case hidden, staticSymbol, burst
    public static func presentation(dynamic: Bool, completion: DayCompletion, reduceMotion: Bool) -> Self {
        guard dynamic, completion == .periodComplete || completion == .dayComplete else { return .hidden }
        return reduceMotion ? .staticSymbol : .burst
    }
}
