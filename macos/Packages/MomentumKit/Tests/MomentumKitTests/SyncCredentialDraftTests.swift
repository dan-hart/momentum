// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import Security
import Testing
@testable import MomentumKit

@MainActor @Suite struct SyncCredentialDraftTests {
    @Test func deniedSaveKeepsOriginalSecretAndBlocksSavedConnectionActionsUntilRetry() throws {
        let access = CredentialFixture()
        let keychain = Keychain(service: "fixture", access: access)
        let draft = SyncCredentialDraft(account: Keychain.nextcloud)
        draft.load(from: keychain)
        #expect(draft.canUseSavedValue)
        access.denyWrites = true
        draft.save("edited", to: keychain)
        #expect(draft.value == "edited")
        #expect(draft.hasError)
        #expect(!draft.canUseSavedValue)
        #expect(keychain.get(Keychain.nextcloud) == "original")
        access.denyWrites = false
        draft.retry(using: keychain)
        #expect(draft.canUseSavedValue)
        #expect(!draft.hasError)
        #expect(keychain.get(Keychain.nextcloud) == "edited")
    }
    @Test func unreadableSecretRequiresReloadBeforeAllowingReplacement() {
        let access = CredentialFixture()
        access.denyReads = true
        let keychain = Keychain(service: "fixture", access: access)
        let draft = SyncCredentialDraft(account: Keychain.encryption)
        draft.load(from: keychain)
        #expect(!draft.isLoaded)
        #expect(!draft.canUseSavedValue)
        draft.save("replacement", to: keychain)
        access.denyReads = false
        #expect(keychain.get(Keychain.encryption) == "original")
        draft.retry(using: keychain)
        #expect(draft.value == "original")
        #expect(draft.canUseSavedValue)
    }
}

private final class CredentialFixture: KeychainAccess, @unchecked Sendable {
    var denyWrites = false
    var denyReads = false
    private var data = Data("original".utf8)
    func copy(_ query: [String: Any]) -> (OSStatus, Data?) {
        denyReads ? (errSecInteractionNotAllowed, nil) : (errSecSuccess, data)
    }
    func update(_ query: [String: Any], _ attributes: [String: Any]) -> OSStatus {
        if denyWrites { return errSecInteractionNotAllowed }
        data = attributes[kSecValueData as String] as! Data
        return errSecSuccess
    }
    func add(_ query: [String: Any]) -> OSStatus { errSecInternalError }
    func delete(_ query: [String: Any]) -> OSStatus {
        if denyWrites { return errSecInteractionNotAllowed }
        data = Data()
        return errSecSuccess
    }
}
