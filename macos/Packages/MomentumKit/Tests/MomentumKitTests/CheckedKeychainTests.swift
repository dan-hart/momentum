// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import Security
import Testing
@testable import MomentumKit

@Suite struct CheckedKeychainTests {
    @Test func lockedReadIsNotMistakenForMissingCredentials() throws {
        let denied = Keychain(service: "fixture", access: StubKeychainAccess(readStatus: errSecInteractionNotAllowed))
        #expect(throws: KeychainFailure.status(errSecInteractionNotAllowed)) { try denied.readData("connection") }
        let missing = Keychain(service: "fixture", access: StubKeychainAccess(readStatus: errSecItemNotFound))
        #expect(try missing.readData("connection") == nil)
    }

    @Test func failedDeletionCannotReportSuccessfulPasswordRemoval() {
        let access = StubKeychainAccess(deleteStatus: errSecAuthFailed)
        let keychain = Keychain(service: "fixture", access: access)
        #expect(!keychain.set("password", ""))
        #expect(throws: KeychainFailure.status(errSecAuthFailed)) { try keychain.deleteChecked("password") }
    }

    @Test func deniedUpdateDoesNotFallBackToAddingAnotherItem() {
        let access = StubKeychainAccess(updateStatus: errSecInteractionNotAllowed)
        let keychain = Keychain(service: "fixture", access: access)
        #expect(throws: KeychainFailure.status(errSecInteractionNotAllowed)) {
            try keychain.writeData("connection", Data("new secret".utf8), accessibility: .afterFirstUnlockThisDeviceOnly)
        }
        #expect(access.adds == 0)
    }

    @Test func newConnectionUsesRequestedDeviceProtectionAndSurfacesAddFailure() {
        let access = StubKeychainAccess(updateStatus: errSecItemNotFound, addStatus: errSecNotAvailable)
        let keychain = Keychain(service: "fixture", access: access)
        #expect(throws: KeychainFailure.status(errSecNotAvailable)) {
            try keychain.writeData("connection", Data("secret".utf8), accessibility: .afterFirstUnlockThisDeviceOnly)
        }
        #expect(access.adds == 1)
        #expect(access.protection == kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly as String)
    }

    @Test func nearbyReadFailureCannotRegenerateOrDeleteAnExistingIdentity() {
        let access = StubKeychainAccess(readStatus: errSecInteractionNotAllowed)
        let store = KeychainSecretStore(keychain: Keychain(service: "fixture", access: access))
        #expect(store.get(name: "p2p-app-key") == nil)
        #expect(!store.set(name: "p2p-app-key", value: "bmV3"))
        store.delete(name: "p2p-app-key")
        #expect(access.updates == 0)
        #expect(access.deletes == 0)
    }

    @Test func malformedNearbySecretCannotBeTreatedAsAnAbsentKey() {
        for bytes in [Data([0xff]), Data("not-base64!".utf8), Data()] {
            let access = StubKeychainAccess(readStatus: errSecSuccess, readData: bytes)
            let store = KeychainSecretStore(keychain: Keychain(service: "fixture", access: access))
            _ = store.get(name: "p2p-app-key")
            #expect(!store.set(name: "p2p-app-key", value: "bmV3"))
            #expect(access.updates == 0)
        }
    }

    @Test func genuinelyMissingNearbyIdentityCanBeCreated() {
        let access = StubKeychainAccess()
        let store = KeychainSecretStore(keychain: Keychain(service: "fixture", access: access))
        #expect(store.get(name: "p2p-app-key") == nil)
        #expect(store.set(name: "p2p-app-key", value: "bmV3"))
        #expect(access.updates == 1)
    }

    @Test func actualCheckedDataRoundTripUsesAnIsolatedService() throws {
        let keychain = Keychain(service: "momentum-checked-tests-\(UUID())")
        defer { try? keychain.deleteChecked("connection") }
        #expect(try keychain.readData("connection") == nil)
        let original = Data([0, 1, 2, 255])
        try keychain.writeData("connection", original)
        #expect(try keychain.readData("connection") == original)
        try keychain.writeData("connection", Data("replacement".utf8))
        #expect(try keychain.readData("connection") == Data("replacement".utf8))
        try keychain.deleteChecked("connection")
        try keychain.deleteChecked("connection")
        #expect(try keychain.readData("connection") == nil)
    }
}

// Each test owns its synchronous stub; no live keychain item or credential is used.
private final class StubKeychainAccess: KeychainAccess, @unchecked Sendable {
    let readStatus: OSStatus
    let readData: Data?
    let updateStatus: OSStatus
    let addStatus: OSStatus
    let deleteStatus: OSStatus
    private(set) var adds = 0
    private(set) var updates = 0
    private(set) var deletes = 0
    private(set) var protection: String?
    init(readStatus: OSStatus = errSecItemNotFound, readData: Data? = nil, updateStatus: OSStatus = errSecSuccess,
         addStatus: OSStatus = errSecSuccess, deleteStatus: OSStatus = errSecSuccess) {
        self.readStatus = readStatus; self.readData = readData; self.updateStatus = updateStatus
        self.addStatus = addStatus; self.deleteStatus = deleteStatus
    }
    func copy(_ query: [String: Any]) -> (OSStatus, Data?) { (readStatus, readData) }
    func update(_ query: [String: Any], _ attributes: [String: Any]) -> OSStatus { updates += 1; return updateStatus }
    func add(_ query: [String: Any]) -> OSStatus {
        adds += 1
        protection = query[kSecAttrAccessible as String] as? String
        return addStatus
    }
    func delete(_ query: [String: Any]) -> OSStatus { deletes += 1; return deleteStatus }
}
