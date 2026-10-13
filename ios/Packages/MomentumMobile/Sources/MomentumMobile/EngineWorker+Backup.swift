// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore

extension EngineWorker {
    /// Local file work stays on the engine actor; Rust owns serialization/validation.
    public func exportBackup(includeSettings: Bool = false) throws -> Data {
        try Task.checkCancellation()
        return try withBackupFile { url in
            _ = try engine.exportBackupWithSettings(path: url.path, includeSettings: includeSettings)
            let data = try Data(contentsOf: url)
            try Task.checkCancellation()
            return data
        }
    }

    public func restoreBackup(_ file: BackupFile, restoreSettings: Bool = false) throws -> Outcome {
        try Task.checkCancellation()
        return try withBackupFile { url in
            try file.data.write(to: url, options: .atomic)
            try Task.checkCancellation()
            return try engine.importBackupWithSettings(path: url.path, restoreSettings: restoreSettings)
        }
    }

    public func portableSettingsJSON() throws -> String { try engine.portableSettingsJson() }
    public func stagePortableSettings(_ json: String) throws { _ = try engine.stagePortableSettings(json: json) }
    public func appSettingsSyncStatus() -> SettingsSyncStatus { engine.settingsSyncStatus() }
    public func setAppSettingsSync(_ enabled: Bool) throws -> SettingsSyncStatus {
        try engine.setSettingsSyncEnabled(enabled: enabled)
    }
    public func chooseAppSettingsBootstrap(_ choice: SettingsBootstrapChoice, reviewID: String) throws -> SettingsSyncStatus {
        try engine.setSettingsBootstrapChoice(choice: choice, expectedReviewId: reviewID)
    }
    public func exportAppSettings() throws -> Data {
        try withBackupFile { url in
            _ = try engine.exportSettings(path: url.path)
            return try Data(contentsOf: url)
        }
    }
    public func importAppSettings(_ file: BackupFile) throws -> Outcome {
        try withBackupFile { url in
            try file.data.write(to: url, options: .atomic)
            return try engine.importSettings(path: url.path)
        }
    }

    private func withBackupFile<T>(_ operation: (URL) throws -> T) throws -> T {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent("momentum-backup-\(UUID())")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: false)
        defer { try? FileManager.default.removeItem(at: directory) }
        return try operation(directory.appendingPathComponent("backup.json"))
    }
}
