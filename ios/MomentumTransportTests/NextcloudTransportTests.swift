// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore
import MomentumKit
import MomentumMobile
import XCTest

/// Actual iOS Swift -> UniFFI -> Rust -> HTTP -> disk exchanges, not a mocked adapter.
@MainActor final class NextcloudTransportTests: XCTestCase {
    func testConnectionProbeIsReadOnlyAndAcceptsAMissingFolder() async throws {
        let fixture = try await TransportFixture()
        defer { fixture.close() }
        let worker = try await fixture.worker("probe")
        _ = await worker.perform(.add("Keep pending", .today))
        let before = await worker.snapshot(view: .today)

        try await worker.nextcloudConnectionTestOperation(
            settings: fixture.connection.settings,
            timeoutMs: 2_000
        ).run()

        let after = await worker.snapshot(view: .today)
        XCTAssertEqual(after.tasks, before.tasks)
        XCTAssertEqual(after.sync, before.sync)
        XCTAssertEqual(fixture.server.propfinds, 2, "A missing folder falls back to the authenticated account root")
        XCTAssertEqual(fixture.server.gets, 0)
        XCTAssertEqual(fixture.server.puts, 0)
        XCTAssertEqual(fixture.server.collections, 0)
    }

    func testConnectionProbeReportsAuthenticationTimeoutAndCancellation() async throws {
        let fixture = try await TransportFixture()
        defer { fixture.close() }
        let worker = try await fixture.worker("probe-errors")

        fixture.server.rejectAuthentication = true
        do {
            try await worker.nextcloudConnectionTestOperation(
                settings: fixture.connection.settings,
                timeoutMs: 2_000
            ).run()
            XCTFail("Expected rejected credentials")
        } catch {
            XCTAssertTrue(String(describing: error).contains("401"))
        }
        fixture.server.rejectAuthentication = false

        let reachedTimeout = expectation(description: "Timeout probe reached WebDAV")
        fixture.server.onHeldPROPFIND = { reachedTimeout.fulfill() }
        do {
            try await worker.nextcloudConnectionTestOperation(
                settings: fixture.connection.settings,
                timeoutMs: 25
            ).run()
            XCTFail("Expected deadline")
        } catch {
            XCTAssertFalse(error is CancellationError)
        }
        await fulfillment(of: [reachedTimeout], timeout: 2)
        fixture.server.releasePROPFIND()

        let reachedCancellation = expectation(description: "Cancelled probe reached WebDAV")
        fixture.server.onHeldPROPFIND = { reachedCancellation.fulfill() }
        let operation = await worker.nextcloudConnectionTestOperation(
            settings: fixture.connection.settings,
            timeoutMs: 2_000
        )
        let running = Task { try await operation.run() }
        await fulfillment(of: [reachedCancellation], timeout: 2)
        XCTAssertTrue(operation.cancel())
        fixture.server.releasePROPFIND()
        do {
            try await running.value
            XCTFail("Expected cancellation")
        } catch {
            XCTAssertTrue(error is CancellationError)
        }
    }

    func testHostedNextcloudTLSWhenExplicitlyConfigured() async throws {
        let environment = ProcessInfo.processInfo.environment
        guard let serverURL = environment["MOMENTUM_NEXTCLOUD_TEST_URL"],
              let userName = environment["MOMENTUM_NEXTCLOUD_TEST_USERNAME"],
              let password = environment["MOMENTUM_NEXTCLOUD_TEST_PASSWORD"],
              let folder = environment["MOMENTUM_NEXTCLOUD_TEST_FOLDER"],
              !serverURL.isEmpty, !userName.isEmpty, !password.isEmpty, !folder.isEmpty else {
            throw XCTSkip("Set the MOMENTUM_NEXTCLOUD_TEST_* variables for an isolated disposable hosted folder")
        }
        var connection = NextcloudConnection()
        connection.serverURL = serverURL
        connection.userName = userName
        connection.password = password
        connection.folder = folder
        connection.automatic = false
        connection = try connection.validated()
        XCTAssertTrue(connection.serverURL.lowercased().hasPrefix("https://"),
                      "The hosted acceptance lane must exercise TLS")

        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: directory) }
        let publisher = try await hostedWorker(directory.appendingPathComponent("publisher"), fullBackup: true)
        let receiver = try await hostedWorker(directory.appendingPathComponent("receiver"), fullBackup: false)
        let title = "Momentum hosted TLS fixture \(UUID().uuidString)"
        _ = await publisher.perform(.add(title, .today))
        _ = try await publisher.nextcloudOperation(settings: connection.settings).run()
        _ = try await receiver.nextcloudOperation(settings: connection.settings).run()
        let received = await receiver.snapshot(view: .today).tasks.first { $0.title == title }
        XCTAssertNotNil(received)

        if let id = received?.id {
            _ = await receiver.perform(.delete([id]))
            _ = try await receiver.nextcloudOperation(settings: connection.settings).run()
            _ = try await publisher.nextcloudOperation(settings: connection.settings).run()
            let finalSnapshot = await publisher.snapshot(view: .today)
            XCTAssertFalse(finalSnapshot.tasks.contains { $0.id == id })
        }
    }

    func testTwoDevicesConvergeWithOfflineEditsConflictRetryAndEncryptedCompression() async throws {
        for encrypted in [false, true] {
            let fixture = try await TransportFixture()
            defer { fixture.close() }
            var connection = fixture.connection
            connection.compress = encrypted
            connection.encryptionPassword = encrypted ? "fixture-encryption-only" : ""
            let a = try await fixture.worker("a", fullBackup: true)
            let b = try await fixture.worker("b")
            _ = await a.perform(.add("From A", .today))
            let upload = try await fixture.exchange(a, connection)
            XCTAssertTrue(upload.uploaded)
            XCTAssertEqual(fixture.server.collections, 1, "First upload creates the collection")
            let payload = try XCTUnwrap(fixture.server.payload)
            XCTAssertTrue(payload.starts(with: Data((encrypted ? "pf_CE2__" : "pf_2__").utf8)))
            if encrypted { XCTAssertFalse(String(decoding: payload, as: UTF8.self).contains("From A")) }
            let download = try await fixture.exchange(b, connection)
            XCTAssertTrue(download.downloaded)
            XCTAssertFalse(download.uploaded)
            await assertTasks(b, ["From A"], pending: 0)

            // Both peers edit without seeing the other's changes.
            _ = await a.perform(.add("A offline", .today))
            _ = await b.perform(.add("B offline", .today))
            _ = try await fixture.exchange(b, connection)
            fixture.server.conflictNextPUT = true
            _ = try await fixture.exchange(a, connection)
            XCTAssertEqual(fixture.server.conflicts, 1, "The real client retries a rejected ETag")
            _ = try await fixture.exchange(b, connection)
            let expected = ["A offline", "B offline", "From A"]
            await assertTasks(a, expected, pending: 0)
            await assertTasks(b, expected, pending: 0)
            let reopened = try await fixture.worker("b")
            await assertTasks(reopened, expected, pending: 0)
            let uploads = fixture.server.puts
            _ = try await fixture.exchange(b, connection)
            XCTAssertEqual(fixture.server.puts, uploads, "An unchanged exchange must not upload again")
        }
    }

    func testNativeCoordinatorPersistsCredentialsAndRecoversAfterAuthenticationFailure() async throws {
        let fixture = try await TransportFixture()
        defer { fixture.close() }
        let worker = try await fixture.worker("local", fullBackup: true)
        _ = await worker.perform(.add("Keep offline edit", .today))
        let state = fixture.state(worker)
        await state.load()
        XCTAssertEqual(state.provider, .off)
        state.draft = fixture.connection
        let didSave = await state.save()
        XCTAssertTrue(didSave)
        let saved = try await fixture.store.load()
        XCTAssertEqual(saved, fixture.connection)
        await state.select(.nextcloud)
        state.setForeground(true)
        var commits = 0
        state.didCommit = { commits += 1 }
        fixture.server.rejectAuthentication = true
        await state.syncNow()
        XCTAssertNotNil(state.failure)
        XCTAssertEqual(commits, 0)
        XCTAssertEqual(state.status?.lastNextcloudMs, 0)
        await assertTasks(worker, ["Keep offline edit"], pending: 1)
        let reopenedState = fixture.state(worker)
        XCTAssertEqual(reopenedState.failure, state.failure, "Failure remains visible after reopening")
        fixture.server.rejectAuthentication = false
        await state.syncNow()
        XCTAssertNil(state.failure)
        XCTAssertEqual(commits, 1)
        XCTAssertGreaterThan(state.status?.lastNextcloudMs ?? 0, 0)
        await assertTasks(worker, ["Keep offline edit"], pending: 0)
        await state.select(.off)
        let retained = try await fixture.store.load()
        XCTAssertEqual(retained, fixture.connection)
        XCTAssertEqual(state.provider, .off)
        state.setForeground(false)
    }

    func testBackgroundCancellationDrainsHTTPAndKeepsConcurrentLocalEdits() async throws {
        let fixture = try await TransportFixture()
        defer { fixture.close() }
        let worker = try await fixture.worker("local", fullBackup: true)
        _ = await worker.perform(.add("Before sync", .today))
        let state = fixture.state(worker)
        await state.load()
        state.draft = fixture.connection
        let didSave = await state.save()
        XCTAssertTrue(didSave)
        await state.select(.nextcloud)
        state.setForeground(true)
        let arrived = expectation(description: "Actual HTTP GET reached the loopback peer")
        fixture.server.onHeldGET = { arrived.fulfill() }
        let exchange = Task { await state.syncNow() }
        await fulfillment(of: [arrived], timeout: 5)
        // This must finish while the network request is still held: no worker/main-actor blockage.
        _ = await worker.perform(.add("During sync", .today))
        state.setForeground(false)
        XCTAssertTrue(state.isStopping)
        fixture.server.releaseGET()
        await exchange.value
        XCTAssertFalse(state.isSyncing)
        XCTAssertNil(state.failure, "Lifecycle cancellation is not a connection failure")
        XCTAssertEqual(fixture.server.puts, 0)
        XCTAssertEqual(state.status?.lastNextcloudMs, 0)
        await assertTasks(worker, ["Before sync", "During sync"], pending: 2)
        let reopened = try await fixture.worker("local")
        await assertTasks(reopened, ["Before sync", "During sync"], pending: 2)
        state.setForeground(true)
        await state.syncNow()
        await assertTasks(worker, ["Before sync", "During sync"], pending: 0)
        state.setForeground(false)
    }

    func testWrongEncryptionPasswordPreservesLocalDataAndRecoversAfterSavingCorrection() async throws {
        let fixture = try await TransportFixture()
        defer { fixture.close() }
        var connection = fixture.connection
        connection.compress = true
        connection.encryptionPassword = "fixture-encryption-only"
        let publisher = try await fixture.worker("publisher", fullBackup: true)
        _ = await publisher.perform(.add("Encrypted remote task", .today))
        _ = try await fixture.exchange(publisher, connection)
        let local = try await fixture.worker("local")
        _ = await local.perform(.add("Keep local edit", .today))
        let state = fixture.state(local)
        await state.load()
        state.draft = connection
        state.draft.encryptionPassword = "incorrect-fixture-password"
        let savedWrong = await state.save()
        XCTAssertTrue(savedWrong)
        await state.select(.nextcloud)
        state.setForeground(true)
        let before = fixture.server.payload
        await state.syncNow()
        XCTAssertEqual(state.failure, .connection)
        XCTAssertEqual(state.status?.lastNextcloudMs, 0)
        XCTAssertEqual(fixture.server.payload, before, "A failed decode must never overwrite the server")
        await assertTasks(local, ["Keep local edit"], pending: 1)
        state.draft = connection
        let savedCorrected = await state.save()
        XCTAssertTrue(savedCorrected)
        await state.syncNow()
        XCTAssertNil(state.failure)
        await assertTasks(local, ["Encrypted remote task", "Keep local edit"], pending: 0)
        _ = try await fixture.exchange(publisher, connection)
        await assertTasks(publisher, ["Encrypted remote task", "Keep local edit"], pending: 0)
        state.setForeground(false)
    }

    private func assertTasks(_ worker: EngineWorker, _ titles: [String], pending: UInt32,
                             file: StaticString = #filePath, line: UInt = #line) async {
        let snapshot = await worker.snapshot(view: .today)
        XCTAssertEqual(snapshot.tasks.map(\.title).sorted(), titles.sorted(), file: file, line: line)
        XCTAssertEqual(snapshot.sync.pendingOps, pending, file: file, line: line)
        XCTAssertFalse(snapshot.sync.syncing, file: file, line: line)
    }

    private func hostedWorker(_ directory: URL, fullBackup: Bool) async throws -> EngineWorker {
        let worker = try await EngineWorker.openChecked(directory: directory)
        guard fullBackup else { return worker }
        let bytes = try await worker.exportBackup()
        var backup = try XCTUnwrap(JSONSerialization.jsonObject(with: bytes) as? [String: Any])
        var data = try XCTUnwrap(backup["data"] as? [String: Any])
        data["globalConfig"] = [String: String]()
        backup["data"] = data
        _ = try await worker.restoreBackup(BackupFile(
            name: "HostedFixture.json",
            data: try JSONSerialization.data(withJSONObject: backup)
        ))
        return worker
    }
}

@MainActor private final class TransportFixture {
    let server: LoopbackDAV
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    let defaults: UserDefaults
    let suite = "momentum-dav-\(UUID())"
    let keychain = Keychain(service: "momentum-dav-\(UUID())")
    let store: NextcloudConnectionStore
    var connection: NextcloudConnection {
        var value = NextcloudConnection()
        value.serverURL = server.url
        value.userName = "fixture"
        value.password = "fixture-only"
        value.folder = "fixture"
        value.automatic = false
        return value
    }

    init() async throws {
        defaults = try XCTUnwrap(UserDefaults(suiteName: suite))
        store = NextcloudConnectionStore(keychain: keychain)
        server = try LoopbackDAV()
        try await server.start()
    }

    func close() {
        server.stop()
        try? keychain.deleteChecked(NextcloudConnectionStore.account)
        defaults.removePersistentDomain(forName: suite)
        try? FileManager.default.removeItem(at: directory)
    }

    func worker(_ name: String, fullBackup: Bool = false) async throws -> EngineWorker {
        let worker = try await EngineWorker.openChecked(directory: directory.appendingPathComponent(name))
        if fullBackup {
            // Existing desktop/SP contract refuses to bootstrap a server from a partial
            // dataset. Supply a disposable complete backup through the real import API.
            let bytes = try await worker.exportBackup()
            var backup = try XCTUnwrap(JSONSerialization.jsonObject(with: bytes) as? [String: Any])
            var data = try XCTUnwrap(backup["data"] as? [String: Any])
            data["globalConfig"] = [String: String]()
            backup["data"] = data
            _ = try await worker.restoreBackup(BackupFile(name: "Fixture.json", data: JSONSerialization.data(withJSONObject: backup)))
        }
        return worker
    }

    func exchange(_ worker: EngineWorker, _ connection: NextcloudConnection) async throws -> SyncReport {
        let operation = await worker.nextcloudOperation(settings: connection.settings)
        return try await operation.run()
    }

    func state(_ worker: EngineWorker) -> NextcloudSyncState {
        let state = NextcloudSyncState(defaults: defaults, persistence: store)
        state.connect(makeOperation: { await worker.nextcloudOperation(settings: $0) },
                      makeConnectionTest: { await worker.nextcloudConnectionTestOperation(settings: $0) },
                      status: { await worker.syncStatus() })
        return state
    }
}
