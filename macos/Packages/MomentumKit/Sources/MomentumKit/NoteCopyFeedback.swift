// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import Observation
#if os(macOS)
import AppKit
#elseif os(iOS)
import UIKit
#endif

/// Acknowledges a successful copy in the active native editor; each copy owns its deadline.
@MainActor @Observable public final class NoteCopyFeedback {
    public private(set) var message: String?
    public private(set) var generation: UInt64 = 0
    @ObservationIgnored private let write: @MainActor (String) -> Bool
    @ObservationIgnored private let success: @MainActor () -> Void
    @ObservationIgnored private let sleep: @Sendable () async -> Void
    @ObservationIgnored private var dismissal: Task<Void, Never>?

    public convenience init() {
        self.init(write: { notes in
            #if os(macOS)
            NSPasteboard.general.clearContents()
            return NSPasteboard.general.setString(notes, forType: .string)
            #elseif os(iOS)
            UIPasteboard.general.string = notes
            return true
            #else
            return false
            #endif
        }, success: {
            #if os(macOS)
            NSHapticFeedbackManager.defaultPerformer.perform(.alignment, performanceTime: .now)
            #endif
        })
    }

    public init(write: @escaping @MainActor (String) -> Bool, success: @escaping @MainActor () -> Void,
                sleep: @escaping @Sendable () async -> Void = { try? await Task.sleep(for: .milliseconds(3500)) }) {
        self.write = write
        self.success = success
        self.sleep = sleep
    }

    @discardableResult public func copy(_ notes: String) -> Bool {
        guard write(notes) else { dismiss(); return false }
        dismissal?.cancel()
        generation &+= 1
        let generation = generation
        message = String(localized: "Notes copied", bundle: .module)
        success()
        dismissal = Task { @MainActor [weak self, sleep] in
            await sleep()
            guard !Task.isCancelled, self?.generation == generation else { return }
            self?.message = nil
        }
        return true
    }

    public func dismiss() {
        dismissal?.cancel()
        dismissal = nil
        message = nil
    }
}
