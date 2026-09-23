// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// The edges of the app that reach outside it: notifications, the Spotlight index, and the
// file the `mo` command line writes when the app is not listening. Each is a protocol so
// the state object can be built without any of them — which is what the unit tests do,
// and why they need no running app.
import Foundation
import MomentumCore

/// Reminders, the morning summary, and the Dock badge.
@MainActor
public protocol Notifier: AnyObject {
    func reminder(_ due: ReminderDue)
    func morningSummary(_ summary: MorningSummary)
    func updateBadge(_ count: UInt32)
}

/// Tasks offered to the system's own search.
@MainActor
public protocol SearchIndexer: AnyObject {
    func reindex(_ tasks: [TaskBrief])
}

#if os(macOS)
/// Watches one file for writes by another process (`mo` when the app's socket is not up).
/// Atomic writes replace the file rather than changing it, so the watch is re-armed after
/// a rename or a delete.
public final class FileWatcher: @unchecked Sendable {
    private var source: DispatchSourceFileSystemObject?
    private var retry: DispatchWorkItem?
    private let path: String
    private let onChange: @Sendable () -> Void
    private let lock = NSLock()

    public init(file: URL, onChange: @escaping @Sendable () -> Void) {
        self.path = file.path
        self.onChange = onChange
        start()
    }

    private func start() {
        let fd = open(path, O_EVTONLY)
        guard fd >= 0 else {
            // The store file is created on the first change; look again in a while.
            let item = DispatchWorkItem { [weak self] in self?.start() }
            lock.withLock { retry = item }
            DispatchQueue.global().asyncAfter(deadline: .now() + 5, execute: item)
            return
        }
        let s = DispatchSource.makeFileSystemObjectSource(
            fileDescriptor: fd, eventMask: [.write, .rename, .delete], queue: .global())
        s.setEventHandler { [weak self] in
            guard let self else { return }
            let replaced = s.data.contains(.rename) || s.data.contains(.delete)
            self.onChange()
            if replaced {
                s.cancel()
                self.start()
            }
        }
        s.setCancelHandler { close(fd) }
        s.resume()
        lock.withLock { source = s }
    }

    deinit {
        lock.withLock {
            retry?.cancel()
            source?.cancel()
        }
    }
}

// MARK: - Bridges from the core's background threads
//
// The core calls these from whatever thread it is on. Each hops to the main actor and
// touches the state there. They are `@unchecked Sendable` because the only stored value
// is a weak reference that is never read off the main actor.

public final class P2pBridge: P2pDelegate, @unchecked Sendable {
    private weak var state: AppState?
    public init(state: AppState) { self.state = state }
    public func onEvent(event: P2pEvent) {
        Task { @MainActor [weak self] in self?.state?.handle(event) }
    }
}

public final class CliBridge: CliDelegate, @unchecked Sendable {
    private weak var state: AppState?
    public init(state: AppState) { self.state = state }
    public func storeChanged() {
        Task { @MainActor [weak self] in self?.state?.cliStoreChanged() }
    }
    public func syncRequested() {
        Task { @MainActor [weak self] in self?.state?.sync() }
    }
}
#endif
