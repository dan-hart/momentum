// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation

/// When the app asks iOS for a background wake to run one Nextcloud exchange. The
/// person's "Sync in the background" choice, the saved connection's automatic switch and
/// Low Power Mode all gate it; iOS decides when, or whether, a request actually runs.
public enum BackgroundSyncPolicy {
    /// Nextcloud is polled every five minutes in the foreground; in the background a
    /// half-hour floor keeps the app off the discretionary budget's bad side.
    public static let minimumCadence: TimeInterval = 30 * 60

    public static func shouldSchedule(enabled: Bool, exchangePossible: Bool) -> Bool {
        enabled && exchangePossible
    }

    public static func earliestBeginDate(after now: Date) -> Date {
        now.addingTimeInterval(minimumCadence)
    }
}
