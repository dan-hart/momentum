// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore

extension EngineWorker {
    /// Local file work stays on the engine actor; Rust owns serialization/validation.
    public func exportBackup() throws -> Data {
        try Task.checkCancellation()
        return try withBackupFile { url in
            _ = try engine.exportBackup(path: url.path)
            let data = try Data(contentsOf: url)
            try Task.checkCancellation()
            return data
        }
    }

    public func restoreBackup(_ file: BackupFile) throws -> Outcome {
        try Task.checkCancellation()
        return try withBackupFile { url in
            try file.data.write(to: url, options: .atomic)
            try Task.checkCancellation()
            return try engine.importBackup(path: url.path)
        }
    }

    private func withBackupFile<T>(_ operation: (URL) throws -> T) throws -> T {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent("momentum-backup-\(UUID())")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: false)
        defer { try? FileManager.default.removeItem(at: directory) }
        return try operation(directory.appendingPathComponent("backup.json"))
    }
}
