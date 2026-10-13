// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import Testing
@testable import MomentumKit

@MainActor @Suite struct PortableAppSettingsTests {
    @Test func refreshingSyncedPortableValuesUpdatesTheVisibleList() throws {
        let h = Harness()
        let ids = h.rows.sorted()
        _ = try h.engine.stagePortableSettings(json: #"{"version":1,"values":{"grouping":"none"}}"#)
        h.state.refreshAppSettings()
        #expect(h.state.listing.sections.count == 1)
        #expect(h.state.listing.sections.first?.group == nil)
        #expect(h.rows.sorted() == ids)
    }

    @Test func aFailedNativeStageStaysVisibleThroughRefreshAndNoOpNotificationUntilRetry() throws {
        let h = Harness(demo: false)
        h.defaults.set("title", forKey: PrefKey.taskSort)
        h.state.preferencesChanged()
        let blocked = h.state.dataDir.appendingPathComponent("portable-settings.json.tmp")
        try FileManager.default.createDirectory(at: blocked, withIntermediateDirectories: false)
        h.defaults.set("created", forKey: PrefKey.taskSort)
        h.state.preferencesChanged()
        let failure = try #require(h.state.appSettingsError)
        #expect(h.defaults.string(forKey: PrefKey.taskSort) == "title")
        #expect(h.engine.preferences().sort == .title)
        h.state.refresh()
        h.state.preferencesChanged() // The notification from the rollback has no new intent.
        #expect(h.state.appSettingsError == failure)
        #expect(h.state.canRetryAppSettings)
        h.state.chooseAppSettingsBootstrap(.useLocal, reviewID: "fixture-review")
        #expect(h.state.appSettingsError == failure)
        #expect(h.state.canRetryAppSettings)
        try FileManager.default.removeItem(at: blocked)
        _ = try h.engine.stagePortableSettings(json: #"{"version":1,"values":{"grouping":"project"}}"#)
        h.state.retryAppSettings()
        #expect(h.state.appSettingsError == nil)
        #expect(h.engine.preferences().sort == .created)
        #expect(h.engine.preferences().groupBy == .project)
        #expect(!h.state.canRetryAppSettings)
    }

    @Test(arguments: ["export", "consent", "edit", "retry"])
    func aFailedInitialSeedRetriesAllNativeValuesWithoutApplyingEmptyCore(recovery: String) throws {
        let directory = TempDir()
        let engine = Engine.open(dir: directory.url.appendingPathComponent("store").path)
        let suite = "momentum-seed-failure-\(UUID())"
        let defaults = UserDefaults(suiteName: suite)!
        defer { UserDefaults().removePersistentDomain(forName: suite) }
        Preferences.register(defaults)
        defaults.set("compact", forKey: PrefKey.rowDensity)
        defaults.set("title", forKey: PrefKey.taskSort)
        let blocked = URL(fileURLWithPath: engine.dataDir()).appendingPathComponent("portable-settings.json.tmp")
        try FileManager.default.createDirectory(at: blocked, withIntermediateDirectories: false)
        let state = AppState(engine: engine, defaults: defaults, services: false)
        let failure = try #require(state.appSettingsError)
        state.chooseAppSettingsBootstrap(.useLocal, reviewID: "fixture-review")
        #expect(state.appSettingsError == failure)
        #expect(state.canRetryAppSettings)
        state.refreshAppSettings(force: true)
        #expect(defaults.string(forKey: PrefKey.rowDensity) == "compact")
        #expect(defaults.string(forKey: PrefKey.taskSort) == "title")
        try FileManager.default.removeItem(at: blocked)
        switch recovery {
        case "export": state.exportAppSettings(directory.url.appendingPathComponent("settings.json"))
        case "consent": state.setAppSettingsSync(true)
        case "retry": state.retryAppSettings()
        default: defaults.set(true, forKey: PrefKey.autoArchive); state.preferencesChanged()
        }
        let document = try #require(JSONSerialization.jsonObject(with: Data(engine.portableSettingsJson().utf8)) as? [String: Any])
        let values = try #require(document["values"] as? [String: Any])
        #expect(values.count == 9)
        #expect(values["rowDensity"] as? String == "compact")
        #expect(values["sort"] as? String == "title")
        #expect(state.appSettingsError == nil)
    }
    @Test(arguments: [false, true], [false, true])
    func backupSettingsRequireBothExportAndRestoreOptIns(include: Bool, restore: Bool) throws {
        let source = Harness(demo: false)
        _ = source.engine.addTask(text: "Portable backup fixture", view: .today)
        source.defaults.set("project", forKey: PrefKey.groupBy)
        source.defaults.set("compact", forKey: PrefKey.rowDensity)
        source.defaults.set("source-private-sentinel", forKey: PrefKey.nextcloudServer)
        source.defaults.set(true, forKey: PrefKey.morningSummaryEnabled)
        source.state.preferencesChanged()
        let file = source.dir.url.appendingPathComponent("portable-backup.json")
        source.state.exportBackup(file, includeSettings: include)
        let data = try Data(contentsOf: file)
        let document = try #require(JSONSerialization.jsonObject(with: data) as? [String: Any])
        #expect((document["momentumSettings"] != nil) == include)
        #expect(!String(decoding: data, as: UTF8.self).contains("private-sentinel"))
        let target = Harness(demo: false)
        target.defaults.set("target-private-sentinel", forKey: PrefKey.nextcloudServer)
        target.state.importBackup(file, restoreSettings: restore)
        #expect(target.engine.allTasks().map(\.title) == ["Portable backup fixture"])
        #expect(target.defaults.string(forKey: PrefKey.groupBy) == (include && restore ? "project" : "morning-night"))
        #expect(target.defaults.string(forKey: PrefKey.rowDensity) == (include && restore ? "compact" : "regular"))
        #expect(target.defaults.string(forKey: PrefKey.nextcloudServer) == "target-private-sentinel")
        #expect(!target.defaults.bool(forKey: PrefKey.morningSummaryEnabled))
        #expect(!target.engine.settingsSyncStatus().enabled)
    }

    @Test(arguments: [true, false])
    func combinedRestoreSupersedesFailedNativeSeedOrDelta(initialSeed: Bool) throws {
        let source = Harness(demo: false)
        _ = source.engine.addTask(text: "Restored fixture", view: .today)
        source.defaults.set("created", forKey: PrefKey.taskSort)
        source.defaults.set("compact", forKey: PrefKey.rowDensity)
        let backup = source.dir.url.appendingPathComponent("combined.json")
        source.state.exportBackup(backup, includeSettings: true)
        let directory = TempDir()
        let engine = Engine.open(dir: directory.url.appendingPathComponent("store").path)
        _ = engine.addTask(text: "Existing fixture", view: .today)
        let suite = "momentum-restore-seed-failure-\(UUID())"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        Preferences.register(defaults)
        defaults.set("title", forKey: PrefKey.taskSort)
        let blocked = URL(fileURLWithPath: engine.dataDir()).appendingPathComponent("portable-settings.json.tmp")
        if initialSeed { try FileManager.default.createDirectory(at: blocked, withIntermediateDirectories: false) }
        let state = AppState(engine: engine, defaults: defaults, services: false)
        if !initialSeed {
            try FileManager.default.createDirectory(at: blocked, withIntermediateDirectories: false)
            defaults.set("due", forKey: PrefKey.taskSort)
            state.preferencesChanged()
        }
        #expect(state.canRetryAppSettings)
        state.importBackup(backup, restoreSettings: true)
        #expect(engine.allTasks().map(\.title) == ["Existing fixture"])
        state.importBackup(backup, restoreSettings: false)
        #expect(engine.allTasks().map(\.title) == ["Restored fixture"])
        #expect(state.canRetryAppSettings)
        #expect(defaults.string(forKey: PrefKey.taskSort) == "title")
        try FileManager.default.removeItem(at: blocked)
        state.importBackup(backup, restoreSettings: true)
        #expect(defaults.string(forKey: PrefKey.taskSort) == "created")
        #expect(defaults.string(forKey: PrefKey.rowDensity) == "compact")
        #expect(!state.canRetryAppSettings)
        #expect(state.appSettingsError == nil)
        state.retryAppSettings()
        #expect(engine.preferences().sort == .created)
        #expect(defaults.string(forKey: PrefKey.rowDensity) == "compact")
    }

    @Test(arguments: [false, true])
    func failedSettingsImportsDoNotCommitRejectedLocalIntent(combined: Bool) throws {
        let h = Harness(demo: false)
        _ = h.engine.addTask(text: "Keep this task", view: .today)
        h.defaults.set("title", forKey: PrefKey.taskSort)
        h.defaults.set("compact", forKey: PrefKey.rowDensity)
        h.state.preferencesChanged()
        let blocked = h.state.dataDir.appendingPathComponent("portable-settings.json.tmp")
        try FileManager.default.createDirectory(at: blocked, withIntermediateDirectories: false)
        h.defaults.set("created", forKey: PrefKey.taskSort)
        h.state.preferencesChanged()
        #expect(h.state.canRetryAppSettings)
        let failure = try #require(h.state.appSettingsError)
        try FileManager.default.removeItem(at: blocked)
        let before = try h.engine.portableSettingsJson()
        let stateFile = h.state.dataDir.appendingPathComponent("state.json")
        let pendingFile = h.state.dataDir.appendingPathComponent("pending.json")
        let portableFile = h.state.dataDir.appendingPathComponent("portable-settings.json")
        let taskBytes = try Data(contentsOf: stateFile)
        let pendingBytes = try Data(contentsOf: pendingFile)
        let portableBytes = try Data(contentsOf: portableFile)
        let invalid = h.dir.url.appendingPathComponent("invalid-settings.json")
        try Data(#"{"version":1,"values":{"sort":17}}"#.utf8).write(to: invalid)
        if combined {
            let source = Harness(demo: false)
            _ = source.engine.addTask(text: "Do not restore this task", view: .today)
            let backup = source.dir.url.appendingPathComponent("invalid-combined.json")
            source.state.exportBackup(backup, includeSettings: true)
            var body = try #require(JSONSerialization.jsonObject(with: Data(contentsOf: backup)) as? [String: Any])
            body["momentumSettings"] = ["version": 1, "values": ["sort": 17]]
            try JSONSerialization.data(withJSONObject: body).write(to: backup)
            h.state.importBackup(backup, restoreSettings: true)
        } else { h.state.importAppSettings(invalid) }
        #expect(try h.engine.portableSettingsJson() == before)
        #expect(h.defaults.string(forKey: PrefKey.taskSort) == "title")
        #expect(h.defaults.string(forKey: PrefKey.rowDensity) == "compact")
        #expect(h.state.canRetryAppSettings)
        #expect(h.state.appSettingsError == failure)
        #expect(try Data(contentsOf: stateFile) == taskBytes)
        #expect(try Data(contentsOf: pendingFile) == pendingBytes)
        #expect(try Data(contentsOf: portableFile) == portableBytes)
        h.state.refreshAppSettings(force: true)
        #expect(h.state.canRetryAppSettings)
        try Data(#"{"version":1,"values":{"sort":"due"}}"#.utf8).write(to: invalid)
        h.state.importAppSettings(invalid)
        #expect(h.defaults.string(forKey: PrefKey.taskSort) == "due")
        #expect(h.defaults.string(forKey: PrefKey.rowDensity) == "compact")
        #expect(!h.state.canRetryAppSettings)
        h.state.retryAppSettings()
        #expect(h.engine.preferences().sort == .due)
    }

    @Test func partialImportAfterFailedSeedPreservesNativeMissingValuesAndSupersedesOldSeed() throws {
        let directory = TempDir()
        let engine = Engine.open(dir: directory.url.appendingPathComponent("store").path)
        let suite = "momentum-import-seed-failure-\(UUID())"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        Preferences.register(defaults)
        defaults.set("compact", forKey: PrefKey.rowDensity)
        defaults.set("title", forKey: PrefKey.taskSort)
        let blocked = URL(fileURLWithPath: engine.dataDir()).appendingPathComponent("portable-settings.json.tmp")
        try FileManager.default.createDirectory(at: blocked, withIntermediateDirectories: false)
        let state = AppState(engine: engine, defaults: defaults, services: false)
        #expect(state.canRetryAppSettings)
        try FileManager.default.removeItem(at: blocked)
        let file = directory.url.appendingPathComponent("partial.json")
        try Data(#"{"version":1,"values":{"sort":"due"}}"#.utf8).write(to: file)
        state.importAppSettings(file)
        #expect(defaults.string(forKey: PrefKey.taskSort) == "due")
        #expect(defaults.string(forKey: PrefKey.rowDensity) == "compact")
        #expect(!state.canRetryAppSettings)
        state.retryAppSettings()
        let document = try #require(JSONSerialization.jsonObject(with: Data(engine.portableSettingsJson().utf8)) as? [String: Any])
        let values = try #require(document["values"] as? [String: Any])
        #expect(values.count == 9)
        #expect(values["sort"] as? String == "due")
        #expect(values["rowDensity"] as? String == "compact")
    }

    @Test func settingsOnlyImportKeepsTasksAndLocalConsent() throws {
        let source = Harness(demo: false)
        source.defaults.set("compact", forKey: PrefKey.rowDensity)
        let file = source.dir.url.appendingPathComponent("app-settings.json")
        source.state.exportAppSettings(file)
        let target = Harness(demo: false)
        _ = target.engine.addTask(text: "Task that must remain", view: .today)
        target.defaults.set("local-font-sentinel", forKey: PrefKey.appFontName)
        target.defaults.set(true, forKey: PrefKey.morningSummaryEnabled)
        target.state.preferencesChanged()
        target.state.importAppSettings(file)
        #expect(target.engine.allTasks().map(\.title) == ["Task that must remain"])
        #expect(target.defaults.string(forKey: PrefKey.rowDensity) == "compact")
        #expect(target.defaults.string(forKey: PrefKey.appFontName) == "local-font-sentinel")
        #expect(target.defaults.bool(forKey: PrefKey.morningSummaryEnabled))
        #expect(!target.engine.settingsSyncStatus().enabled)
    }

    @Test func exportPreservesAnUnseenSyncedChoice() throws {
        let h = Harness(demo: false)
        _ = try h.engine.stagePortableSettings(json: #"{"version":1,"values":{"sort":"title"}}"#)
        let file = h.dir.url.appendingPathComponent("settings-export.json")
        h.state.exportAppSettings(file)
        let document = try #require(JSONSerialization.jsonObject(with: Data(contentsOf: file)) as? [String: Any])
        let values = try #require(document["values"] as? [String: Any])
        #expect(values["sort"] as? String == "title")
        #expect(h.defaults.string(forKey: PrefKey.taskSort) == "title")
    }

    @Test func anUnseenSyncedKeySurvivesALocalChangeToAnotherKey() throws {
        let h = Harness(demo: false)
        _ = try h.engine.stagePortableSettings(json: #"{"version":1,"values":{"sort":"title"}}"#)
        #expect(h.defaults.string(forKey: PrefKey.taskSort) == "manual")
        h.defaults.set(false, forKey: PrefKey.colorfulLabels)
        h.state.preferencesChanged()
        #expect(h.engine.preferences().sort == .title)
        #expect(h.defaults.string(forKey: PrefKey.taskSort) == "title")
        #expect(!h.defaults.bool(forKey: PrefKey.colorfulLabels))
    }
    @Test func onlyApprovedPortableValuesLeaveNativePreferences() throws {
        let h = Harness(demo: false)
        h.defaults.set("private-credential-sentinel", forKey: PrefKey.nextcloudServer)
        h.defaults.set("private-font-sentinel", forKey: PrefKey.appFontName)
        h.defaults.set(true, forKey: PrefKey.morningSummaryEnabled)
        h.defaults.set("compact", forKey: PrefKey.rowDensity)
        let text = try PortableAppSettings.encode(h.defaults)
        let document = try #require(JSONSerialization.jsonObject(with: Data(text.utf8)) as? [String: Any])
        let values = try #require(document["values"] as? [String: Any])
        #expect(Set(values.keys) == ["grouping", "sort", "direction", "upcomingDays", "autoArchive", "rowDensity", "colorfulLabels", "completionSoundEnabled", "completionSoundChoice"])
        #expect(values["rowDensity"] as? String == "compact")
        #expect(!text.contains("sentinel"))
        #expect(!h.engine.settingsSyncStatus().enabled)
    }

    @Test func importedPartialValuesPreserveMissingAndDeviceOnlyChoices() throws {
        let h = Harness(demo: false)
        h.defaults.set("compact", forKey: PrefKey.rowDensity)
        h.defaults.set("local-server-sentinel", forKey: PrefKey.nextcloudServer)
        h.defaults.set(false, forKey: PrefKey.completionSound)
        try PortableAppSettings.stage(engine: h.engine, defaults: h.defaults)
        _ = try h.engine.stagePortableSettings(json: #"{"version":1,"values":{"grouping":"project","completionSoundEnabled":true,"unknownFutureSetting":"ignored"}}"#)
        try PortableAppSettings.apply(try h.engine.portableSettingsJson(), to: h.defaults)
        #expect(h.defaults.string(forKey: PrefKey.groupBy) == "project")
        #expect(h.defaults.bool(forKey: PrefKey.completionSound))
        #expect(h.defaults.string(forKey: PrefKey.rowDensity) == "compact")
        #expect(h.defaults.string(forKey: PrefKey.nextcloudServer) == "local-server-sentinel")
        #expect(!h.engine.settingsSyncStatus().enabled)
    }

    @Test func aDurabilityFailureRollsBackNativeSettingsBeforePublication() throws {
        let h = Harness(demo: false)
        try PortableAppSettings.initialize(engine: h.engine, defaults: h.defaults)
        h.defaults.set("title", forKey: PrefKey.taskSort)
        try PortableAppSettings.stage(engine: h.engine, defaults: h.defaults)
        try FileManager.default.createDirectory(at: h.state.dataDir.appendingPathComponent("portable-settings.json.tmp"), withIntermediateDirectories: false)
        h.defaults.set("created", forKey: PrefKey.taskSort)
        #expect(throws: (any Error).self) { try PortableAppSettings.stage(engine: h.engine, defaults: h.defaults) }
        #expect(h.defaults.string(forKey: PrefKey.taskSort) == "title")
        #expect(h.engine.preferences().sort == .title)
    }
}
