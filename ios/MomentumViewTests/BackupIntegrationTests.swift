// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumMobile
import SwiftUI
import XCTest

@MainActor final class BackupIntegrationTests: XCTestCase {
    func testRestoreRefreshesTasksAndPreservesSettingsNavigation() async throws {
        try await withModel { model in
            let worker = try XCTUnwrap(model.worker)
            _ = await model.perform(.add("Keep from backup", .today))
            model.selectedTab = 3
            let revision = model.revision
            let bytes = try await model.exportBackup()
            XCTAssertEqual(model.revision, revision, "Preparing an export must not mutate tasks")
            _ = await model.perform(.add("Replace this later task", .today))
            let beforeRestore = model.revision
            try await model.restoreBackup(BackupFile(name: "Tasks.json", data: bytes))
            XCTAssertEqual(model.revision, beforeRestore + 1)
            XCTAssertEqual(model.selectedTab, 3)
            XCTAssertFalse(model.isRestoringBackup)
            let snapshot = await worker.snapshot(view: .today)
            XCTAssertEqual(snapshot.tasks.map(\.title), ["Keep from backup"])
        }
    }

    func testRejectedRestorePreservesTasksAndReleasesBusyState() async throws {
        try await withModel { model in
            _ = await model.perform(.add("Keep existing task", .today))
            let revision = model.revision
            do {
                try await model.restoreBackup(BackupFile(name: "Invalid.json", data: Data("{}".utf8)))
                XCTFail("An unrelated JSON object must not replace the store")
            } catch {}
            XCTAssertEqual(model.revision, revision)
            XCTAssertFalse(model.isRestoringBackup)
            let snapshot = await model.worker?.snapshot(view: .today)
            XCTAssertEqual(snapshot?.tasks.map(\.title), ["Keep existing task"])
        }
    }

    func testRestoreDoesNotDiscardAnActiveCaptureOrEditor() async throws {
        try await withModel { model in
            let file = BackupFile(name: "Empty.json", data: try await model.exportBackup())
            _ = await model.perform(.add("Unsaved editor context", .today))
            let revision = model.revision
            for presentation in 0..<3 {
                model.showingAdd = presentation == 0
                model.activeTaskEditors = presentation == 1 ? 1 : 0
                model.showingNewProject = presentation == 2
                do {
                    try await model.restoreBackup(file)
                    XCTFail("Restore must preserve an open draft")
                } catch is CancellationError {} catch { XCTFail("Unexpected error: \(error)") }
                XCTAssertEqual(model.revision, revision)
                XCTAssertFalse(model.isRestoringBackup)
            }
            let snapshot = await model.worker?.snapshot(view: .today)
            XCTAssertEqual(snapshot?.tasks.count, 1)
        }
    }

    func testBackupsRenderInBothAppearancesAndGermanAccessibleText() async throws {
        try await withModel { model in
            for scheme in [ColorScheme.light, .dark] {
                for large in [false, true] {
                    let render = try HostingFixture.render(
                        BackupSettings().environment(model).frame(height: 700), width: 320,
                        size: large ? .accessibility5 : .large, scheme: scheme, locale: large ? "de" : "en")
                    XCTAssertLessThanOrEqual(render.size.width, 320)
                    let attachment = XCTAttachment(image: render.image)
                    attachment.name = "Backups-\(scheme)-\(large ? "German-AX5" : "English")"
                    attachment.lifetime = .keepAlways
                    self.add(attachment)
                }
            }
        }
    }

    private func withModel(_ body: (MobileAppModel) async throws -> Void) async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let name = "momentum-backup-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name); try? FileManager.default.removeItem(at: directory) }
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
        await model.foreground()
        try await body(model)
    }
}
