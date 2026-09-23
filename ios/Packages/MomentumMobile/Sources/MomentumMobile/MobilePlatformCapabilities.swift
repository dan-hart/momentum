// SPDX-License-Identifier: GPL-3.0-or-later

public struct MobilePlatformCapabilities: Equatable, Sendable {
    public let isPad: Bool

    public init(isPad: Bool) {
        self.isPad = isPad
    }

    public var showsKeyboardShortcuts: Bool { isPad }
}
