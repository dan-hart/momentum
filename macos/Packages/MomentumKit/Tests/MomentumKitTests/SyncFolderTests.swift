// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import Testing
@testable import MomentumKit

@MainActor @Suite struct SyncFolderTests {
    @Test(arguments: [false, true])
    func cancellationBeforeTheVerifiedResultIsPresentedKeepsTheOriginalFolder(cancelTask: Bool) async {
        let fixture = FolderFixture()
        defer { fixture.clean() }
        let operation = DelayedVerifiedCopy()
        let state = AppState(engine: Engine.open(dir: fixture.dir.url.path), defaults: fixture.defaults,
            keychain: fixture.keychain, services: false, folderCopyFactory: { _, _ in operation })
        let copying = Task { await state.copyToMomentumFolder() }
        await operation.waitUntilRunning()
        if cancelTask { copying.cancel() } else { state.cancelFolderCopy() }
        await operation.finish()
        #expect(!(await copying.value))
        #expect(fixture.defaults.string(forKey: PrefKey.nextcloudFolder) == "previous-tasks")
        #expect(!state.folderCopyInProgress && !state.syncInProgress)
        #expect(state.folderCopyMessage != nil)
    }

    @Test func verifiedCopyChangesOnlyTheFolderAfterVerification() async throws {
        let fixture = FolderFixture()
        defer { fixture.clean() }
        let state = fixture.state(verified: true)
        let outcome = state.engine.addTask(text: "Keep pending", view: .today)
        #expect(outcome.changed)
        let before = state.engine.allTasks().map(\.id)
        let pending = state.engine.syncStatus().pendingOps
        #expect(await state.copyToMomentumFolder())
        #expect(fixture.defaults.string(forKey: PrefKey.nextcloudFolder) == "momentum")
        #expect(fixture.defaults.string(forKey: PrefKey.nextcloudServer) == "https://fixture.invalid")
        #expect(fixture.keychain.get(Keychain.nextcloud) == "fixture-only-password")
        #expect(Preferences(fixture.defaults).syncMethod == .nextcloud)
        #expect(state.engine.allTasks().map(\.id) == before)
        #expect(state.engine.syncStatus().pendingOps == pending)
    }

    @Test(arguments: [false, true])
    func unverifiedOrFailedCopyRetainsTheSavedFolder(failure: Bool) async {
        let fixture = FolderFixture()
        defer { fixture.clean() }
        let state = fixture.state(verified: false, failure: failure)
        #expect(!(await state.copyToMomentumFolder()))
        #expect(fixture.defaults.string(forKey: PrefKey.nextcloudFolder) == "previous-tasks")
        #expect(!state.folderCopyInProgress && !state.syncInProgress)
        #expect(state.folderCopyMessage != nil)
    }
}

@MainActor private final class FolderFixture {
    let name = "momentum-folder-\(UUID())"
    let dir = TempDir()
    let defaults: UserDefaults
    let keychain = Keychain(service: "momentum-folder-test-\(UUID())")
    init() {
        defaults = UserDefaults(suiteName: name)!
        Preferences.register(defaults, domain: name)
        defaults.set("nextcloud", forKey: PrefKey.syncMethod)
        defaults.set("https://fixture.invalid", forKey: PrefKey.nextcloudServer)
        defaults.set("fixture", forKey: PrefKey.nextcloudUser)
        defaults.set("previous-tasks", forKey: PrefKey.nextcloudFolder)
        keychain.set(Keychain.nextcloud, "fixture-only-password")
    }
    func state(verified: Bool, failure: Bool = false) -> AppState {
        AppState(engine: Engine.open(dir: dir.url.path), defaults: defaults, keychain: keychain, services: false,
                 folderCopyFactory: { _, settings in
            FixtureFolderCopy(source: settings.folder, verified: verified, failure: failure)
        })
    }
    func clean() { defaults.removePersistentDomain(forName: name); keychain.delete(Keychain.nextcloud) }
}

private struct FixtureFolderCopy: NextcloudFolderCopying {
    let source: String
    let verified: Bool
    let failure: Bool
    func cancel() -> Bool { false }
    func run() async throws -> NextcloudFolderMigrationReport {
        if failure { throw CocoaError(.fileWriteNoPermission) }
        return NextcloudFolderMigrationReport(sourceFolder: source, targetFolder: "momentum", copiedFiles: 1,
                                             targetFiles: ["sync-data.json"], verified: verified)
    }
}

private actor DelayedVerifiedCopy: NextcloudFolderCopying {
    private var completion: CheckedContinuation<NextcloudFolderMigrationReport, Never>?
    private var started: CheckedContinuation<Void, Never>?
    nonisolated func cancel() -> Bool { false } // Transport already verified; native switch is still pending.
    func run() async throws -> NextcloudFolderMigrationReport {
        await withCheckedContinuation { completion = $0; started?.resume(); started = nil }
    }
    func waitUntilRunning() async {
        if completion == nil { await withCheckedContinuation { started = $0 } }
    }
    func finish() {
        completion?.resume(returning: NextcloudFolderMigrationReport(sourceFolder: "previous-tasks", targetFolder: "momentum",
            copiedFiles: 1, targetFiles: ["sync-data.json"], verified: true))
        completion = nil
    }
}
