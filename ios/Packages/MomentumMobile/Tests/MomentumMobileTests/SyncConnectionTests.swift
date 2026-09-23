// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumKit
import Testing
@testable import MomentumMobile

@Suite struct SyncConnectionTests {
    @Test func savedConnectionRoundTripsAsOneKeychainRecord() async throws {
        let keychain = Keychain(service: "momentum-connection-test-\(UUID())")
        defer { try? keychain.deleteChecked(NextcloudConnectionStore.account) }
        let store = NextcloudConnectionStore(keychain: keychain)
        #expect(try await store.load() == nil)
        var connection = NextcloudConnection()
        connection.serverURL = "https://fixture.invalid"
        connection.userName = "fixture"
        connection.password = " keep whitespace "
        connection.encryptionPassword = "other secret"
        connection.compress = true
        connection.automatic = false
        try await store.save(connection)
        #expect(try await store.load() == connection)
        let stored = try keychain.readData(NextcloudConnectionStore.account)
        let bytes = try #require(stored)
        let record = try JSONDecoder().decode(NextcloudConnection.self, from: bytes)
        #expect(record == connection)
        #expect(connection.settings.password == " keep whitespace ")
        #expect(connection.settings.encryptionPassword == "other secret")
        #expect(connection.settings.compress)
    }

    @Test func corruptSavedConnectionIsAnErrorRatherThanAnEmptyDraft() async throws {
        let keychain = Keychain(service: "momentum-connection-test-\(UUID())")
        defer { try? keychain.deleteChecked(NextcloudConnectionStore.account) }
        let corrupt = Data("not a connection".utf8)
        try keychain.writeData(NextcloudConnectionStore.account, corrupt)
        let store = NextcloudConnectionStore(keychain: keychain)
        await #expect(throws: (any Error).self) { try await store.load() }
        #expect(try keychain.readData(NextcloudConnectionStore.account) == corrupt)
    }

    @Test func emptyEncryptionIsOptionalAndPasswordWhitespaceIsPreserved() {
        var connection = NextcloudConnection()
        connection.password = " "
        #expect(connection.settings.password == " ")
        #expect(connection.settings.encryptionPassword == nil)
        #expect(connection.folder == "super-productivity")
    }

    @Test func validationNormalizesAddressAndNamesWithoutChangingSecrets() throws {
        var connection = NextcloudConnection()
        connection.serverURL = "  https://cloud.example.test/nextcloud///  "
        connection.userName = "  dan  "
        connection.password = " keep whitespace "
        connection.folder = " /Momentum/ "
        connection.encryptionPassword = " encryption whitespace "

        let normalized = try connection.validated()

        #expect(normalized.serverURL == "https://cloud.example.test/nextcloud")
        #expect(normalized.userName == "dan")
        #expect(normalized.folder == "Momentum")
        #expect(normalized.password == " keep whitespace ")
        #expect(normalized.encryptionPassword == " encryption whitespace ")
    }

    @Test(arguments: [
        ("", NextcloudConnectionIssue.serverURLRequired),
        ("cloud.example.test", .serverURLInvalid),
        ("ftp://cloud.example.test", .serverURLInvalid),
        ("http://cloud.example.test", .secureServerRequired),
        ("https://user:secret@cloud.example.test", .serverURLInvalid),
        ("https://cloud.example.test/path?token=secret", .serverURLInvalid),
        ("https://cloud.example.test/path#fragment", .serverURLInvalid),
    ])
    func validationRejectsUnsafeOrAmbiguousServerAddresses(
        serverURL: String,
        expected: NextcloudConnectionIssue
    ) {
        var connection = configuredConnection()
        connection.serverURL = serverURL
        #expect(throws: expected) { try connection.validated() }
    }

    @Test func loopbackHTTPRemainsAvailableForIsolatedTransportFixtures() throws {
        for address in ["http://localhost:8080", "http://127.0.0.1:8080", "http://[::1]:8080"] {
            var connection = configuredConnection()
            connection.serverURL = address
            #expect(try connection.validated().serverURL == address)
        }
    }

    @Test(arguments: [
        ("", "fixture", "secret", "Momentum", NextcloudConnectionIssue.serverURLRequired),
        ("https://cloud.example.test", " ", "secret", "Momentum", .userNameRequired),
        ("https://cloud.example.test", "fixture", "", "Momentum", .passwordRequired),
        ("https://cloud.example.test", "fixture", "secret", " / ", .folderRequired),
    ])
    func validationIdentifiesEachRequiredField(
        serverURL: String,
        userName: String,
        password: String,
        folder: String,
        expected: NextcloudConnectionIssue
    ) {
        var connection = NextcloudConnection()
        connection.serverURL = serverURL
        connection.userName = userName
        connection.password = password
        connection.folder = folder
        #expect(connection.validationIssue == expected)
    }

    @Test func cancellationBeforeNativeDispatchDoesNotConsumePendingEdits() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: directory) }
        let worker = try await EngineWorker.openChecked(directory: directory)
        _ = await worker.perform(.add("Keep queued task", .today))
        let operation = await worker.nextcloudOperation(settings: NextcloudConnection().settings)
        #expect(operation.cancel())
        await #expect(throws: CancellationError.self) { try await operation.run() }
        let snapshot = await worker.snapshot(view: .today)
        #expect(snapshot.tasks.map(\.title) == ["Keep queued task"])
        #expect(snapshot.sync.pendingOps == 1)
        #expect(!snapshot.sync.syncing)
        #expect(snapshot.sync.lastNextcloudMs == 0)
    }

    @Test func alreadyCancelledTaskSignalsTheOperationBeforeAdmission() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: directory) }
        let worker = try await EngineWorker.openChecked(directory: directory)
        let operation = await worker.nextcloudOperation(settings: NextcloudConnection().settings)
        let task = Task {
            withUnsafeCurrentTask { $0?.cancel() }
            return try await operation.run()
        }
        await #expect(throws: CancellationError.self) { try await task.value }
        #expect(!operation.cancel())
    }

    @Test func configurationFailureIsNotReportedAsCancellation() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: directory) }
        let worker = try await EngineWorker.openChecked(directory: directory)
        let operation = await worker.nextcloudOperation(settings: NextcloudConnection().settings)
        do {
            _ = try await operation.run()
            Issue.record("Unconfigured sync should fail")
        } catch is CancellationError { Issue.record("Configuration failure was concealed as cancellation") }
        catch {}
        #expect(!operation.cancel(), "completed failure must release the operation")
        _ = await worker.perform(.add("Edits remain available", .today))
        let snapshot = await worker.snapshot(view: .today)
        #expect(snapshot.tasks.map(\.title) == ["Edits remain available"])
    }
}

private func configuredConnection() -> NextcloudConnection {
    var connection = NextcloudConnection()
    connection.serverURL = "https://cloud.example.test"
    connection.userName = "fixture"
    connection.password = "secret"
    connection.folder = "Momentum"
    return connection
}
