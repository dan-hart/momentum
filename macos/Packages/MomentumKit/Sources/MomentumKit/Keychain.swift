// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// Secrets in the login Keychain as generic passwords with service "momentum" and the
// purpose as the account, exactly how the `mo` command line's `keyring` crate stores
// them, so the app password entered here is the one `mo sync` uses.
import Foundation
import MomentumCore
import Security

public struct Keychain: Sendable {
    public static let service = "momentum"
    public static let nextcloud = "nextcloud"
    public static let encryption = "encryption"

    /// A service name override for tests, so they never touch the real items.
    public let service: String
    public init(service: String = Keychain.service) {
        self.service = service
    }

    private func query(_ account: String) -> [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
        ]
    }

    public func get(_ account: String) -> String? {
        var q = query(account)
        q[kSecReturnData as String] = true
        q[kSecMatchLimit as String] = kSecMatchLimitOne
        var out: CFTypeRef?
        guard SecItemCopyMatching(q as CFDictionary, &out) == errSecSuccess,
              let data = out as? Data, let s = String(data: data, encoding: .utf8) else { return nil }
        return s
    }

    @discardableResult
    public func set(_ account: String, _ value: String) -> Bool {
        guard let data = value.data(using: .utf8) else { return false }
        if value.isEmpty {
            delete(account)
            return true
        }
        let update = [kSecValueData as String: data]
        let status = SecItemUpdate(query(account) as CFDictionary, update as CFDictionary)
        if status == errSecSuccess { return true }
        if status != errSecItemNotFound { return false }
        var add = query(account)
        add[kSecValueData as String] = data
        add[kSecAttrLabel as String] = "Momentum \(account)"
        return SecItemAdd(add as CFDictionary, nil) == errSecSuccess
    }

    public func delete(_ account: String) {
        SecItemDelete(query(account) as CFDictionary)
    }
}

/// The core's key store for nearby sync, over the same Keychain service.
public final class KeychainSecretStore: SecretStore {
    private let keychain: Keychain
    public init(keychain: Keychain = Keychain()) {
        self.keychain = keychain
    }
    public func get(name: String) -> String? { keychain.get(name) }
    public func set(name: String, value: String) -> Bool { keychain.set(name, value) }
    public func delete(name: String) { keychain.delete(name) }
}
