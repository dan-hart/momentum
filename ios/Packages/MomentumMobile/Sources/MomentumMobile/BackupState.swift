// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumKit
import Observation

/// Immutable bytes prevent an external file change between selection and confirmation.
public struct BackupFile: Equatable, Sendable {
    public let name: String
    public let data: Data
    public init(name: String, data: Data) { self.name = name; self.data = data }
}

@MainActor @Observable public final class BackupState {
    public enum Activity: Equatable { case reading, preparingExport, restoring }
    public enum Completion: Equatable { case exported, restored }
    public private(set) var selection: BackupFile?
    public private(set) var preparedExport: Data?
    public private(set) var activity: Activity?
    public private(set) var error: String?
    public private(set) var completion: Completion?
    public var canStart: Bool { activity == nil && preparedExport == nil }

    public init() {}

    public func select(_ file: BackupFile) {
        guard canStart else { return }
        selection = file
        error = nil
        completion = nil
    }
    public func discardSelection() {
        guard canStart else { return }
        selection = nil
        error = nil
    }
    public func report(_ failure: Error) {
        guard !(failure is CancellationError),
              !((failure as NSError).domain == NSCocoaErrorDomain && (failure as NSError).code == NSUserCancelledError)
        else { return }
        error = Strings.error(failure)
        completion = nil
    }
    private func begin(_ next: Activity) -> Bool {
        guard canStart, !Task.isCancelled else { return false }
        activity = next
        error = nil
        completion = nil
        return true
    }
    @discardableResult public func load(
        _ url: URL,
        reader: @Sendable (URL) async throws -> BackupFile = BackupFileReader.read
    ) async -> Bool {
        guard begin(.reading) else { return false }
        selection = nil
        defer { activity = nil }
        do {
            let file = try await reader(url)
            try Task.checkCancellation()
            selection = file
            return true
        } catch { report(error); return false }
    }
    @discardableResult public func prepareExport(operation: @MainActor () async throws -> Data) async -> Bool {
        guard begin(.preparingExport) else { return false }
        defer { activity = nil }
        do {
            let data = try await operation()
            try Task.checkCancellation()
            preparedExport = data
            return true
        } catch { report(error); return false }
    }
    /// nil means the system picker was cancelled; preparing bytes is not a saved export.
    public func finishExport(_ result: Result<URL, Error>?) {
        preparedExport = nil
        if let result {
            switch result {
            case .success: completion = .exported
            case .failure(let failure): report(failure)
            }
        }
    }
    @discardableResult public func restore(operation: @MainActor (BackupFile) async throws -> Void) async -> Bool {
        guard let file = selection, begin(.restoring) else { return false }
        defer { activity = nil }
        do {
            try Task.checkCancellation()
            try await operation(file)
            // Once the core committed, report that outcome even if the caller's task
            // was subsequently cancelled. Cancellation cannot undo a successful restore.
            selection = nil
            completion = .restored
            return true
        } catch { report(error); return false }
    }
}

public enum BackupFileReader {
    public static func read(_ url: URL) async throws -> BackupFile {
        let read = Task.detached(priority: .userInitiated) {
            try Task.checkCancellation()
            let scoped = url.startAccessingSecurityScopedResource()
            defer { if scoped { url.stopAccessingSecurityScopedResource() } }
            var coordinationError: NSError?
            var contents: Result<Data, Error>?
            NSFileCoordinator().coordinate(readingItemAt: url, options: [], error: &coordinationError) { coordinated in
                contents = Result { try Data(contentsOf: coordinated) }
            }
            if let coordinationError { throw coordinationError }
            let data = try (contents ?? .failure(CocoaError(.fileReadUnknown))).get()
            try Task.checkCancellation()
            return BackupFile(name: url.lastPathComponent, data: data)
        }
        return try await withTaskCancellationHandler { try await read.value } onCancel: { read.cancel() }
    }
}
