// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumKit
import MomentumMobile
import Security
import XCTest

@MainActor final class SyncBoundaryIntegrationTests: XCTestCase {
    func testConnectionIsStoredWithDeviceOnlyBackgroundProtection() async throws {
        let service = "momentum-native-connection-tests-\(UUID())"
        let keychain = Keychain(service: service)
        defer { try? keychain.deleteChecked(NextcloudConnectionStore.account) }
        let store = NextcloudConnectionStore(keychain: keychain)
        var connection = NextcloudConnection()
        connection.serverURL = "https://fixture.invalid"
        connection.userName = "fixture"
        connection.password = "fixture-only"
        connection.encryptionPassword = "fixture-encryption-only"
        try await store.save(connection)
        let reopened = try await store.load()
        XCTAssertEqual(reopened, connection)
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: NextcloudConnectionStore.account,
            kSecReturnAttributes as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]
        var result: CFTypeRef?
        XCTAssertEqual(SecItemCopyMatching(query as CFDictionary, &result), errSecSuccess)
        let attributes = try XCTUnwrap(result as? [String: Any])
        XCTAssertEqual(attributes[kSecAttrAccessible as String] as? String,
                       kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly as String)
        try keychain.deleteChecked(NextcloudConnectionStore.account)
        let deleted = try await store.load()
        XCTAssertNil(deleted)
    }

    func testNearbyBridgePreservesMalformedStoredIdentity() throws {
        let keychain = Keychain(service: "momentum-nearby-key-tests-\(UUID())")
        let account = "device-cert-der"
        defer { try? keychain.deleteChecked(account) }
        let original = Data([0xff, 0x00])
        try keychain.writeData(account, original)
        let bridge = KeychainSecretStore(keychain: keychain, accessibility: .afterFirstUnlockThisDeviceOnly)
        XCTAssertNil(bridge.get(name: account))
        XCTAssertEqual(bridge.failure, .invalidData)
        XCTAssertFalse(bridge.set(name: account, value: Data("replacement".utf8).base64EncodedString()))
        bridge.delete(name: account)
        XCTAssertEqual(try keychain.readData(account), original)
    }

    func testNearbyBridgeCreatesDeviceOnlyKeysAndAllowsFreshSessionRecovery() throws {
        let service = "momentum-nearby-key-tests-\(UUID())"
        let keychain = Keychain(service: service)
        let account = "app-key"
        defer { try? keychain.deleteChecked(account) }
        let bridge = KeychainSecretStore(keychain: keychain, accessibility: .afterFirstUnlockThisDeviceOnly)
        XCTAssertNil(bridge.get(name: account))
        XCTAssertTrue(bridge.set(name: account, value: "Zml4dHVyZQ=="))
        let query: [String: Any] = [kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service, kSecAttrAccount as String: account,
            kSecReturnAttributes as String: true, kSecMatchLimit as String: kSecMatchLimitOne]
        var result: CFTypeRef?
        XCTAssertEqual(SecItemCopyMatching(query as CFDictionary, &result), errSecSuccess)
        let attributes = try XCTUnwrap(result as? [String: Any])
        XCTAssertEqual(attributes[kSecAttrAccessible as String] as? String,
                       kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly as String)
        let reopened = KeychainSecretStore(keychain: keychain, accessibility: .afterFirstUnlockThisDeviceOnly)
        XCTAssertEqual(reopened.get(name: account), "Zml4dHVyZQ==")
        XCTAssertNil(reopened.failure)
    }

    func testNativeQueuedSyncCancellationPreservesTheAppStore() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let name = "momentum-native-sync-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name); try? FileManager.default.removeItem(at: directory) }
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
        await model.foreground()
        _ = await model.perform(.add("Keep local task", .today))
        let worker = try XCTUnwrap(model.worker)
        let operation = await worker.nextcloudOperation(settings: NextcloudConnection().settings)
        XCTAssertTrue(operation.cancel())
        do {
            _ = try await operation.run()
            XCTFail("A cancelled queued operation must never start")
        } catch is CancellationError {} catch { XCTFail("Unexpected cancellation mapping: \(error)") }
        let snapshot = await worker.snapshot(view: .today)
        XCTAssertEqual(snapshot.tasks.map(\.title), ["Keep local task"])
        XCTAssertEqual(snapshot.sync.pendingOps, 1)
        XCTAssertFalse(snapshot.sync.syncing)
        XCTAssertEqual(snapshot.sync.lastNextcloudMs, 0)
    }
}
