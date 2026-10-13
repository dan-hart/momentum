// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore
import MomentumKit
import MomentumMobile
import SwiftUI
import XCTest

@MainActor final class SyncSettingsIntegrationTests: XCTestCase {
    func testFailedInitialPortableSeedRetriesFullSnapshotOnExportAndEdit() async throws {
        for recovery in ["export", "edit", "retry", "import"] {
            let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
            let name = "momentum-portable-seed-retry-\(UUID())"
            let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
            Preferences.register(defaults)
            defaults.set("compact", forKey: PrefKey.rowDensity)
            defaults.set("title", forKey: PrefKey.taskSort)
            defaults.set(true, forKey: PrefKey.morningSummaryEnabled)
            defaults.set(9, forKey: PrefKey.morningSummaryHour)
            defaults.set(45, forKey: PrefKey.morningSummaryMinute)
            let blocked = directory.appendingPathComponent("portable-settings.json.tmp")
            try FileManager.default.createDirectory(at: blocked, withIntermediateDirectories: true)
            let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
            await model.start()
            let worker = try XCTUnwrap(model.worker)
            let failure = try XCTUnwrap(model.appSettingsError)
            await model.chooseAppSettingsBootstrap(.useLocal, reviewID: "fixture-review")
            XCTAssertEqual(model.appSettingsError, failure)
            XCTAssertTrue(model.canRetryAppSettings)
            await model.refreshAppSettings(force: true)
            XCTAssertEqual(defaults.string(forKey: PrefKey.rowDensity), "compact")
            XCTAssertEqual(defaults.string(forKey: PrefKey.taskSort), "title")
            try FileManager.default.removeItem(at: blocked)
            if recovery == "export" { _ = try await model.exportAppSettings() }
            else if recovery == "retry" { await model.retryAppSettings() }
            else if recovery == "import" { try await model.importAppSettings(BackupFile(name: "partial.json", data: Data(#"{"version":1,"values":{"sort":"due"}}"#.utf8))) }
            else { defaults.set(true, forKey: PrefKey.autoArchive); await model.preferencesChanged() }
            let json = try await worker.portableSettingsJSON()
            let document = try XCTUnwrap(JSONSerialization.jsonObject(with: Data(json.utf8)) as? [String: Any])
            let values = try XCTUnwrap(document["values"] as? [String: Any])
            XCTAssertEqual(values.count, 9)
            XCTAssertEqual(values["rowDensity"] as? String, "compact")
            XCTAssertEqual(values["sort"] as? String, recovery == "import" ? "due" : "title")
            XCTAssertNil(model.appSettingsError)
            defaults.removePersistentDomain(forName: name)
            try FileManager.default.removeItem(at: directory)
        }
    }

    func testLocalSettingsRetryKeepsRejectedIntentAndUnseenRemoteKeys() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let name = "momentum-portable-retry-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name); try? FileManager.default.removeItem(at: directory) }
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
        await model.start()
        let worker = try XCTUnwrap(model.worker)
        defaults.set("title", forKey: PrefKey.taskSort)
        await model.preferencesChanged()
        let blocked = directory.appendingPathComponent("portable-settings.json.tmp")
        try FileManager.default.createDirectory(at: blocked, withIntermediateDirectories: false)
        defaults.set("created", forKey: PrefKey.taskSort)
        await model.preferencesChanged()
        let failure = try XCTUnwrap(model.appSettingsError)
        XCTAssertEqual(defaults.string(forKey: PrefKey.taskSort), "title")
        await model.refreshAppSettings(force: true)
        await model.preferencesChanged()
        XCTAssertEqual(model.appSettingsError, failure)
        XCTAssertTrue(model.canRetryAppSettings)
        try FileManager.default.removeItem(at: blocked)
        await model.chooseAppSettingsBootstrap(.useLocal, reviewID: "fixture-review")
        XCTAssertEqual(model.appSettingsError, failure)
        XCTAssertTrue(model.canRetryAppSettings)
        try await worker.stagePortableSettings(#"{"version":1,"values":{"grouping":"project"}}"#)
        await model.retryAppSettings()
        let json = try await worker.portableSettingsJSON()
        let document = try XCTUnwrap(JSONSerialization.jsonObject(with: Data(json.utf8)) as? [String: Any])
        let values = try XCTUnwrap(document["values"] as? [String: Any])
        XCTAssertEqual(values["sort"] as? String, "created")
        XCTAssertEqual(values["grouping"] as? String, "project")
        XCTAssertEqual(defaults.string(forKey: PrefKey.taskSort), "created")
        XCTAssertEqual(defaults.string(forKey: PrefKey.groupBy), "project")
        XCTAssertNil(model.appSettingsError)
        XCTAssertFalse(model.canRetryAppSettings)
    }

    func testLocalSettingsAndExportPreserveAnUnseenSyncedChoice() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let name = "momentum-portable-intent-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name); try? FileManager.default.removeItem(at: directory) }
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
        await model.foreground()
        let worker = try XCTUnwrap(model.worker)
        try await worker.stagePortableSettings(#"{"version":1,"values":{"sort":"title"}}"#)
        defaults.set(false, forKey: PrefKey.colorfulLabels)
        await model.preferencesChanged()
        let data = try await model.exportAppSettings()
        let document = try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [String: Any])
        let values = try XCTUnwrap(document["values"] as? [String: Any])
        XCTAssertEqual(values["sort"] as? String, "title")
        XCTAssertEqual(values["colorfulLabels"] as? Bool, false)
        XCTAssertEqual(defaults.string(forKey: PrefKey.taskSort), "title")
        XCTAssertFalse(model.appSettingsSync?.enabled ?? true)
    }

    func testCombinedRestoreSupersedesFailedNativeSeedAndKeepsTaskOnlyRestoreIndependent() async throws {
        for initialSeed in [true, false] {
            let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
            let name = "momentum-portable-restore-retry-\(UUID())"
            let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
            defer { defaults.removePersistentDomain(forName: name); try? FileManager.default.removeItem(at: directory) }
            Preferences.register(defaults)
            defaults.set("title", forKey: PrefKey.taskSort)
            let source = Engine.open(dir: directory.appendingPathComponent("source").path)
            _ = source.addTask(text: "Restored fixture", view: .today)
            _ = try source.stagePortableSettings(json: #"{"version":1,"values":{"sort":"created","rowDensity":"compact"}}"#)
            let backup = directory.appendingPathComponent("combined.json")
            _ = try source.exportBackupWithSettings(path: backup.path, includeSettings: true)
            let file = BackupFile(name: "combined.json", data: try Data(contentsOf: backup))
            let target = directory.appendingPathComponent("target")
            let blocked = target.appendingPathComponent("portable-settings.json.tmp")
            if initialSeed { try FileManager.default.createDirectory(at: blocked, withIntermediateDirectories: true) }
            let model = MobileAppModel(isolatedDirectory: target, defaults: defaults)
            await model.start()
            let worker = try XCTUnwrap(model.worker)
            _ = await worker.perform(.add("Existing fixture", .today))
            if !initialSeed {
                try FileManager.default.createDirectory(at: blocked, withIntermediateDirectories: false)
                defaults.set("due", forKey: PrefKey.taskSort)
                await model.preferencesChanged()
            }
            XCTAssertTrue(model.canRetryAppSettings)
            do { try await model.restoreBackup(file, restoreSettings: true); XCTFail("Blocked combined restore should fail") }
            catch { }
            let retained = await worker.snapshot(view: .today)
            XCTAssertEqual(retained.tasks.map(\.title), ["Existing fixture"])
            try await model.restoreBackup(file, restoreSettings: false)
            let restored = await worker.snapshot(view: .today)
            XCTAssertEqual(restored.tasks.map(\.title), ["Restored fixture"])
            XCTAssertTrue(model.canRetryAppSettings)
            XCTAssertEqual(defaults.string(forKey: PrefKey.taskSort), "title")
            try FileManager.default.removeItem(at: blocked)
            try await model.restoreBackup(file, restoreSettings: true)
            XCTAssertEqual(defaults.string(forKey: PrefKey.taskSort), "created")
            XCTAssertEqual(defaults.string(forKey: PrefKey.rowDensity), "compact")
            XCTAssertFalse(model.canRetryAppSettings)
            XCTAssertNil(model.appSettingsError)
            await model.retryAppSettings()
            let json = try await worker.portableSettingsJSON()
            let document = try XCTUnwrap(JSONSerialization.jsonObject(with: Data(json.utf8)) as? [String: Any])
            let values = try XCTUnwrap(document["values"] as? [String: Any])
            XCTAssertEqual(values["sort"] as? String, "created")
            XCTAssertEqual(values["rowDensity"] as? String, "compact")
        }
    }

    func testFailedSettingsImportsDoNotCommitRejectedLocalIntent() async throws {
        for combined in [false, true] {
            let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
            let name = "momentum-portable-import-atomic-\(UUID())"
            let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
            defer { defaults.removePersistentDomain(forName: name); try? FileManager.default.removeItem(at: directory) }
            let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
            await model.start()
            let worker = try XCTUnwrap(model.worker)
            _ = await worker.perform(.add("Keep this task", .today))
            defaults.set("title", forKey: PrefKey.taskSort)
            defaults.set("compact", forKey: PrefKey.rowDensity)
            await model.preferencesChanged()
            let blocked = directory.appendingPathComponent("portable-settings.json.tmp")
            try FileManager.default.createDirectory(at: blocked, withIntermediateDirectories: false)
            defaults.set("created", forKey: PrefKey.taskSort)
            await model.preferencesChanged()
            XCTAssertTrue(model.canRetryAppSettings)
            try FileManager.default.removeItem(at: blocked)
            let before = try await worker.portableSettingsJSON()
            let stateFile = directory.appendingPathComponent("state.json")
            let pendingFile = directory.appendingPathComponent("pending.json")
            let portableFile = directory.appendingPathComponent("portable-settings.json")
            let taskBytes = try Data(contentsOf: stateFile)
            let pendingBytes = try Data(contentsOf: pendingFile)
            let portableBytes = try Data(contentsOf: portableFile)
            let invalid = BackupFile(name: "invalid.json", data: Data(#"{"version":1,"values":{"sort":17}}"#.utf8))
            if combined {
                let backupData = try await model.exportBackup()
                var body = try XCTUnwrap(JSONSerialization.jsonObject(with: backupData) as? [String: Any])
                body["momentumSettings"] = ["version": 1, "values": ["sort": 17]]
                let file = BackupFile(name: "invalid-combined.json", data: try JSONSerialization.data(withJSONObject: body))
                do { try await model.restoreBackup(file, restoreSettings: true); XCTFail("Invalid settings must fail") } catch { }
            } else {
                do { try await model.importAppSettings(invalid); XCTFail("Invalid settings must fail") } catch { }
            }
            let after = try await worker.portableSettingsJSON()
            XCTAssertEqual(after, before)
            XCTAssertEqual(defaults.string(forKey: PrefKey.taskSort), "title")
            XCTAssertEqual(defaults.string(forKey: PrefKey.rowDensity), "compact")
            XCTAssertTrue(model.canRetryAppSettings)
            XCTAssertEqual(try Data(contentsOf: stateFile), taskBytes)
            XCTAssertEqual(try Data(contentsOf: pendingFile), pendingBytes)
            XCTAssertEqual(try Data(contentsOf: portableFile), portableBytes)
            await model.refreshAppSettings(force: true)
            XCTAssertTrue(model.canRetryAppSettings)
            try await model.importAppSettings(BackupFile(name: "partial.json", data: Data(#"{"version":1,"values":{"sort":"due"}}"#.utf8)))
            XCTAssertEqual(defaults.string(forKey: PrefKey.taskSort), "due")
            XCTAssertEqual(defaults.string(forKey: PrefKey.rowDensity), "compact")
            XCTAssertFalse(model.canRetryAppSettings)
            await model.retryAppSettings()
            let current = try await worker.portableSettingsJSON()
            XCTAssertTrue(current.contains("due"))
        }
    }

    func testBackupSettingsRequireIndependentExportAndRestoreOptIns() async throws {
        for include in [false, true] {
            for restore in [false, true] {
                let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
                let name = "momentum-portable-backup-\(UUID())"
                let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
                let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
                await model.foreground()
                let worker = try XCTUnwrap(model.worker)
                _ = await worker.perform(.add("Portable task fixture", .today))
                defaults.set("compact", forKey: PrefKey.rowDensity)
                defaults.set("private-source-sentinel", forKey: PrefKey.nextcloudServer)
                defaults.set(true, forKey: PrefKey.morningSummaryEnabled)
                await model.preferencesChanged()
                let data = try await model.exportBackup(includeSettings: include)
                let document = try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [String: Any])
                XCTAssertEqual(document["momentumSettings"] != nil, include)
                XCTAssertFalse(String(decoding: data, as: UTF8.self).contains("source-sentinel"))
                defaults.set("regular", forKey: PrefKey.rowDensity)
                defaults.set(false, forKey: PrefKey.morningSummaryEnabled)
                defaults.set("local-device-sentinel", forKey: PrefKey.nextcloudServer)
                await model.preferencesChanged()
                _ = await worker.perform(.add("Task that should be replaced", .today))
                try await model.restoreBackup(BackupFile(name: "fixture.json", data: data), restoreSettings: restore)
                let snapshot = await worker.snapshot(view: .today)
                XCTAssertEqual(snapshot.tasks.map(\.title), ["Portable task fixture"])
                XCTAssertEqual(defaults.string(forKey: PrefKey.rowDensity), include && restore ? "compact" : "regular")
                XCTAssertFalse(defaults.bool(forKey: PrefKey.morningSummaryEnabled))
                XCTAssertEqual(defaults.string(forKey: PrefKey.nextcloudServer), "local-device-sentinel")
                XCTAssertFalse(model.appSettingsSync?.enabled ?? true)
                defaults.removePersistentDomain(forName: name)
                try? FileManager.default.removeItem(at: directory)
            }
        }
    }

    func testSyncScreenPresentationDoesNotOwnStateLoading() async throws {
        let name = "momentum-sync-presentation-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        let probe = PresentationSyncProbe()
        let state = NextcloudSyncState(defaults: defaults, persistence: probe)
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
        state.connect(makeOperation: { _ in RenderOperation() }, status: { await probe.readStatus() })
        let appeared = expectation(description: "Sync settings appeared")
        let mounted = try HostingFixture.mount(
            NavigationStack { SyncSettings(state: state).onAppear { appeared.fulfill() } }.environment(model),
            defaults: defaults,
            defaultsName: name
        )
        defer { mounted.unmount(); try? FileManager.default.removeItem(at: directory) }

        await fulfillment(of: [appeared], timeout: 5)
        for _ in 0..<10 { await Task.yield() }

        let counts = await probe.counts()
        XCTAssertEqual(counts.loads, 0, "Foreground lifecycle, not navigation, owns credential loading")
        XCTAssertEqual(counts.statusReads, 0, "Foreground lifecycle, not navigation, owns status refresh")
    }

    func testIsolatedModelsCannotStartSyncOrReadProductionCredentials() async throws {
        let name = "momentum-sync-isolation-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { defaults.removePersistentDomain(forName: name); try? FileManager.default.removeItem(at: directory) }
        defaults.set("nextcloud", forKey: PrefKey.syncMethod)
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
        await model.foreground()
        XCTAssertFalse(model.sync.allowed)
        XCTAssertEqual(model.sync.provider, .off)
        XCTAssertFalse(model.sync.loaded)
        XCTAssertFalse(model.sync.isSyncing)
        XCTAssertNil(model.sync.failure)
        _ = await model.perform(.add("Pending fixture", .today))
        await model.sync.refreshStatus()
        XCTAssertEqual(model.sync.status?.pendingOps, 1)
    }

    func testModelPropagatesLowPowerModeWithoutDisablingManualWork() throws {
        let name = "momentum-power-mode-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { defaults.removePersistentDomain(forName: name); try? FileManager.default.removeItem(at: directory) }

        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults,
                                   initialLowPowerMode: true)
        XCTAssertTrue(model.lowPowerMode)
        XCTAssertTrue(model.sync.lowPowerMode)

        model.setLowPowerMode(false)
        XCTAssertFalse(model.lowPowerMode)
        XCTAssertFalse(model.sync.lowPowerMode)
    }

    func testSettingsFitCompactWidthsAndGermanAccessibilityText() async throws {
        let name = "momentum-sync-render-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name) }
        let store = RenderConnection()
        let state = NextcloudSyncState(defaults: defaults, persistence: store)
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
        defer { try? FileManager.default.removeItem(at: directory) }
        state.connect(makeOperation: { _ in RenderOperation() }, status: {
            SyncStatus(syncing: false, pendingOps: 3, lastNextcloudMs: 0,
                       lastNearbyMs: 0, nearbyRunning: false, linkedDevices: 0)
        })
        await state.load()
        await state.select(.nextcloud)
        for scheme in [ColorScheme.light, .dark] {
            for large in [false, true] {
                let render = try HostingFixture.render(
                    NavigationStack { SyncSettings(state: state) }.environment(model).frame(height: 700), width: 320,
                    size: large ? .accessibility5 : .large, scheme: scheme, locale: large ? "de" : "en")
                XCTAssertLessThanOrEqual(render.size.width, 320)
                let attachment = XCTAttachment(image: render.image)
                attachment.name = "Sync-\(scheme)-\(large ? "German-AX5" : "English")"
                attachment.lifetime = .keepAlways
                add(attachment)
            }
        }
        let german = try XCTUnwrap(Bundle.main.path(forResource: "de", ofType: "lproj"))
        let strings = try XCTUnwrap(Bundle(path: german))
        XCTAssertEqual(strings.localizedString(forKey: "Save Connection", value: nil, table: nil), "Verbindung speichern")
    }

    func testTaskListStatusExpandsForGermanLargestTextInBothAppearances() async throws {
        let name = "momentum-sync-status-render-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name) }
        let state = NextcloudSyncState(defaults: defaults, persistence: RenderConnection())
        state.connect(makeOperation: { _ in RenderOperation() }, status: {
            SyncStatus(syncing: false, pendingOps: 123, lastNextcloudMs: 1_789_660_000_000,
                       lastNearbyMs: 0, nearbyRunning: false, linkedDevices: 0)
        })
        await state.load()
        await state.select(.nextcloud)
        for scheme in [ColorScheme.light, .dark] {
            let normal = try HostingFixture.render(SyncStatusLink(state: state), scheme: scheme)
            let large = try HostingFixture.render(SyncStatusLink(state: state), size: .accessibility5, scheme: scheme, locale: "de")
            XCTAssertGreaterThanOrEqual(normal.size.height, 44)
            XCTAssertGreaterThan(large.size.height, normal.size.height)
            XCTAssertLessThan(large.size.width, 321, "Allow only pixel rounding at the viewport edge")
            XCTAssertLessThan(large.size.height, 600, "Metadata must stack, not squeeze into narrow columns")
            for (name, render) in [("English", normal), ("German AX5", large)] {
                let attachment = XCTAttachment(image: render.image)
                attachment.name = "Nextcloud status \(scheme) \(name)"
                attachment.lifetime = .keepAlways
                add(attachment)
            }
        }
    }

    func testToolbarBadgeShowsTheStatusInTheAccentColorWithoutScrolling() async throws {
        let name = "momentum-sync-badge-render-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name) }
        let state = NextcloudSyncState(defaults: defaults, persistence: RenderConnection())
        state.connect(makeOperation: { _ in RenderOperation() }, status: {
            SyncStatus(syncing: false, pendingOps: 0, lastNextcloudMs: UInt64(Date().timeIntervalSince1970 * 1000) - 5 * 60_000,
                       lastNearbyMs: 0, nearbyRunning: false, linkedDevices: 0)
        })
        await state.load()
        await state.select(.nextcloud)
        let presentation = SyncStatusPresentation(state: state)
        XCTAssertEqual(presentation.text(now: .now), "Last synced 5 minutes ago")
        XCTAssertEqual(presentation.symbol, "checkmark.circle")
        // Rendered bare, as the list does inside its own NavigationStack; a stack wrapper
        // would fill the fixture's probe height instead of reporting the control's size.
        for scheme in [ColorScheme.light, .dark] {
            let badge = try HostingFixture.render(SyncStatusToolbarButton(state: state).tint(.red), scheme: scheme)
            XCTAssertGreaterThan(badge.size.height, 0)
            XCTAssertTrue(containsRedInk(badge.image),
                          "the checkmark takes the accent tint, not the primary label color")
            let attachment = XCTAttachment(image: badge.image)
            attachment.name = "Sync status badge \(scheme)"
            attachment.lifetime = .keepAlways
            add(attachment)
        }
        await state.select(.off)
        XCTAssertThrowsError(try HostingFixture.render(SyncStatusToolbarButton(state: state)),
                             "Off shows no badge, so there is nothing to size")
    }

    func testADamagedServerCopyIsExplainedInPlainWordsWithReplaceAndDetails() async throws {
        let name = "momentum-sync-damaged-render-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name) }
        let state = NextcloudSyncState(defaults: defaults, persistence: RenderConnection())
        let damaged = CoreError.Actionable(kind: .remoteFileDamaged,
                                           message: "the sync file on the server could not be read: Invalid input length: 4849657")
        state.connect(makeOperation: { _ in FailingOperation(error: damaged) },
                      makeReplaceOperation: { _ in RenderOperation() },
                      status: { SyncStatus(syncing: false, pendingOps: 9, lastNextcloudMs: 1_789_660_000_000,
                                           lastNearbyMs: 0, nearbyRunning: false, linkedDevices: 0) })
        await state.load()
        state.setForeground(true)
        await state.select(.nextcloud)
        await state.syncNow()
        XCTAssertEqual(state.failure, .connection)
        XCTAssertEqual(state.failureKind, .remoteFileDamaged)
        XCTAssertTrue(state.canReplaceServerCopy)
        XCTAssertNotNil(state.lastAttemptAt, "Try Again is acknowledged even when the same failure returns")

        let headline = Strings.syncFailureHeadline(.remoteFileDamaged)
        XCTAssertEqual(headline, "The copy on the server is damaged")
        XCTAssertFalse(Strings.syncFailureRemedy(.remoteFileDamaged).contains("Invalid input length"),
                       "the decoder's words stay behind Details")
        for size in [DynamicTypeSize.large, .accessibility5] {
            // Tall enough for the whole form, so the last section is laid out and exposed.
            // The section on its own: the whole settings form only materializes the rows
            // that fit its viewport, and this one sits last.
            let mounted = try HostingFixture.mount(NavigationStack { Form { SyncFailureSection(state: state, failure: .connection) } },
                                                   defaults: defaults, defaultsName: name,
                                                   size: CGSize(width: 390, height: size == .large ? 1200 : 3000),
                                                   dynamicTypeSize: size)
            defer { mounted.unmount() }
            try await mounted.wait(until: "the failure section is exposed") {
                mounted.hasAccessibilityElement(label: headline)
            }
            let labels = mounted.accessibilityLabels()
            XCTAssertTrue(labels.contains { $0.hasPrefix("Replace Server Copy") }, "a damaged copy offers the replacement: \(labels)")
            XCTAssertTrue(labels.contains { $0.hasPrefix("Try Again") }, labels.joined(separator: " | "))
            XCTAssertTrue(labels.contains { $0.hasPrefix("Details") }, "the core's sentence waits behind Details: \(labels)")
        }
    }

    func testConfigurationErrorWrapsAtLargestTextOnPhoneAndPadWidths() throws {
        for width: CGFloat in [320] {
            let render = try HostingFixture.render(
                NextcloudConfigurationError(issue: .secureServerRequired),
                width: width,
                size: .accessibility5
            )
            XCTAssertLessThanOrEqual(render.size.width, width)
            XCTAssertGreaterThan(render.size.height, 44)
            XCTAssertLessThan(render.size.height, 500)
        }
    }

    func testLibreSyncDeviceManagementFitsPhoneAndPadAtLargestText() async throws {
        let state = NearbyLifecycle(runtime: RenderNearbyRuntime(), allowed: true)

        for width: CGFloat in [320, 744] {
            let render = try HostingFixture.render(
                LibreSyncDeviceList(state: state,
                                    linking: .constant(nil),
                                    unlinking: .constant(nil))
                    .frame(height: 900),
                width: width,
                size: .accessibility5
            )
            XCTAssertLessThanOrEqual(render.size.width, width)
            XCTAssertGreaterThan(render.size.height, 44)
            XCTAssertLessThan(render.size.height, 2_000)
        }
    }

    func testLateExpirationCannotCancelANewerExchange() {
        var expires: (@MainActor @Sendable () -> Void)?
        var cancellations = 0
        let allowance = SyncExecutionAllowance(expiration: { cancellations += 1 }, begin: { handler in
            expires = handler
            return UIBackgroundTaskIdentifier(rawValue: 44)
        }, end: { _ in })
        allowance.finish()
        expires?()
        XCTAssertEqual(cancellations, 0)
    }

    func testImmediateExpirationStillBalancesReturnedExecutionToken() {
        var ended: [UIBackgroundTaskIdentifier] = []
        let token = UIBackgroundTaskIdentifier(rawValue: 43)
        let allowance = SyncExecutionAllowance(expiration: {}, begin: { handler in
            handler()
            return token
        }, end: { ended.append($0) })
        XCTAssertEqual(ended, [token])
        allowance.finish()
        XCTAssertEqual(ended, [token])
    }

    func testExecutionAllowanceEndsExactlyOnceOnExpiryOrCompletion() {
        var expires: (@MainActor @Sendable () -> Void)?
        var ended: [UIBackgroundTaskIdentifier] = []
        var cancellations = 0
        let token = UIBackgroundTaskIdentifier(rawValue: 42)
        let allowance = SyncExecutionAllowance(expiration: { cancellations += 1 }, begin: { handler in
            expires = handler
            return token
        }, end: { ended.append($0) })
        expires?()
        allowance.finish()
        XCTAssertEqual(cancellations, 1)
        XCTAssertEqual(ended, [token])
        let denied = SyncExecutionAllowance(expiration: {}, begin: { _ in .invalid }, end: { ended.append($0) })
        denied.finish()
        XCTAssertEqual(ended, [token])
    }
}

private actor PresentationSyncProbe: NextcloudConnectionPersistence {
    private var loadCount = 0
    private var statusCount = 0

    func load() -> NextcloudConnection? {
        loadCount += 1
        return nil
    }

    func save(_ connection: NextcloudConnection) {}

    func readStatus() -> SyncStatus {
        statusCount += 1
        return SyncStatus(syncing: false, pendingOps: 0, lastNextcloudMs: 0,
                          lastNearbyMs: 0, nearbyRunning: false, linkedDevices: 0)
    }

    func counts() -> (loads: Int, statusReads: Int) {
        (loadCount, statusCount)
    }
}

private actor RenderConnection: NextcloudConnectionPersistence {
    func load() -> NextcloudConnection? {
        var value = NextcloudConnection()
        value.serverURL = "https://fixture.invalid"
        value.userName = "fixture"
        value.password = "fixture-only"
        value.automatic = false
        return value
    }
    func save(_ connection: NextcloudConnection) {}
}

/// True when any pixel is clearly red: the fixture tints the badge red so the
/// accent-following symbol is distinguishable from black or white label ink.
private func containsRedInk(_ image: UIImage) -> Bool {
    guard let cgImage = image.cgImage else { return false }
    let width = cgImage.width, height = cgImage.height
    var bytes = [UInt8](repeating: 0, count: width * height * 4)
    let drew = bytes.withUnsafeMutableBytes { buffer in
        guard let context = CGContext(
            data: buffer.baseAddress, width: width, height: height, bitsPerComponent: 8,
            bytesPerRow: width * 4, space: CGColorSpaceCreateDeviceRGB(),
            bitmapInfo: CGBitmapInfo.byteOrder32Big.rawValue | CGImageAlphaInfo.premultipliedLast.rawValue
        ) else { return false }
        context.draw(cgImage, in: CGRect(x: 0, y: 0, width: width, height: height))
        return true
    }
    guard drew else { return false }
    return stride(from: 0, to: bytes.count, by: 4).contains { i in
        bytes[i] > 150 && bytes[i + 1] < 90 && bytes[i + 2] < 90
    }
}

private struct RenderOperation: NextcloudRunningOperation {
    func run() async throws -> SyncReport { throw CancellationError() }
    func cancel() -> Bool { true }
}

private struct FailingOperation: NextcloudRunningOperation {
    let error: Error
    func run() async throws -> SyncReport { throw error }
    func cancel() -> Bool { true }
}

private actor RenderNearbyRuntime: NearbyRuntime {
    func start(events: @escaping @Sendable (P2pEvent) async -> Void) -> P2pInfo {
        P2pInfo(deviceName: "Fixture iPhone", deviceId: "fixture-phone", port: 42424)
    }
    func stop() {}
    func snapshot() -> NearbySnapshot {
        NearbySnapshot(
            linked: [NearbyDevice(deviceId: "linked", name: "Kitchen Mac", address: nil,
                                  lastSeenMs: 1_789_660_000_000, linked: true)],
            discovered: [NearbyDevice(deviceId: "new", name: "Office iPad",
                                      address: "127.0.0.1:42425", lastSeenMs: nil, linked: false)],
            status: SyncStatus(syncing: false, pendingOps: 2, lastNextcloudMs: 0,
                               lastNearbyMs: 1_789_660_000_000, nearbyRunning: true, linkedDevices: 1)
        )
    }
    func beginPairing() -> String? { "123456" }
    func endPairing() {}
    func link(address: String, code: String) throws {}
    func unlink(id: String) {}
    func syncNow() -> UInt64 { 1 }
}
