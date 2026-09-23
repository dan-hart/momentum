// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// Secrets in the login Keychain as generic passwords with service "momentum" and the
// purpose as the account, exactly how the `mo` command line's `keyring` crate stores
// them, so the app password entered here is the one `mo sync` uses.
import Foundation
import MomentumCore
import Security

public enum KeychainFailure: Error, Equatable, Sendable {
    case status(OSStatus)
    case invalidData
}

public enum KeychainAccessibility: Sendable {
    case afterFirstUnlockThisDeviceOnly
    var value: CFString { kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly }
}

protocol KeychainAccess: Sendable {
    func copy(_ query: [String: Any]) -> (OSStatus, Data?)
    func update(_ query: [String: Any], _ attributes: [String: Any]) -> OSStatus
    func add(_ query: [String: Any]) -> OSStatus
    func delete(_ query: [String: Any]) -> OSStatus
}

private struct SystemKeychainAccess: KeychainAccess {
    func copy(_ query: [String: Any]) -> (OSStatus, Data?) {
        var output: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &output)
        return (status, output as? Data)
    }
    func update(_ query: [String: Any], _ attributes: [String: Any]) -> OSStatus {
        SecItemUpdate(query as CFDictionary, attributes as CFDictionary)
    }
    func add(_ query: [String: Any]) -> OSStatus { SecItemAdd(query as CFDictionary, nil) }
    func delete(_ query: [String: Any]) -> OSStatus { SecItemDelete(query as CFDictionary) }
}

public struct Keychain: Sendable {
    public static let service = "momentum"
    public static let nextcloud = "nextcloud"
    public static let encryption = "encryption"

    /// A service name override for tests, so they never touch the real items.
    public let service: String
    private let access: any KeychainAccess
    public init(service: String = Keychain.service) {
        self.service = service
        access = SystemKeychainAccess()
    }

    init(service: String, access: any KeychainAccess) { self.service = service; self.access = access }

    private func query(_ account: String) -> [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
        ]
    }

    public func get(_ account: String) -> String? {
        guard let data = try? readData(account) else { return nil }
        return String(data: data, encoding: .utf8)
    }

    /// Only item-not-found means absent. Locked/denied reads must remain recoverable
    /// errors rather than prompting callers to overwrite a supposedly empty item.
    public func readData(_ account: String) throws -> Data? {
        var q = query(account)
        q[kSecReturnData as String] = true
        q[kSecMatchLimit as String] = kSecMatchLimitOne
        let (status, data) = access.copy(q)
        if status == errSecItemNotFound { return nil }
        guard status == errSecSuccess else { throw KeychainFailure.status(status) }
        guard let data else { throw KeychainFailure.invalidData }
        return data
    }

    @discardableResult
    public func set(_ account: String, _ value: String) -> Bool {
        guard let data = value.data(using: .utf8) else { return false }
        do {
            if value.isEmpty { try deleteChecked(account) }
            else { try writeData(account, data) }
            return true
        } catch { return false }
    }

    /// A single record can keep related connection fields and secrets atomic.
    /// nil preserves existing platform defaults and existing item protection.
    public func writeData(_ account: String, _ data: Data,
                          accessibility: KeychainAccessibility? = nil) throws {
        var update: [String: Any] = [kSecValueData as String: data]
        if let accessibility { update[kSecAttrAccessible as String] = accessibility.value }
        let status = access.update(query(account), update)
        if status == errSecSuccess { return }
        guard status == errSecItemNotFound else { throw KeychainFailure.status(status) }
        var add = query(account)
        add.merge(update) { _, new in new }
        add[kSecAttrLabel as String] = "Momentum \(account)"
        var result = access.add(add)
        // Another writer may have inserted the same exact item after our lookup.
        if result == errSecDuplicateItem { result = access.update(query(account), update) }
        guard result == errSecSuccess else { throw KeychainFailure.status(result) }
    }

    public func delete(_ account: String) {
        try? deleteChecked(account)
    }

    public func deleteChecked(_ account: String) throws {
        let status = access.delete(query(account))
        guard status == errSecSuccess || status == errSecItemNotFound else { throw KeychainFailure.status(status) }
    }
}

/// The core's optional-read callback cannot distinguish absent from unreadable.
/// Latch failures for this transport session so a failed read can never authorize
/// replacement keys. A fresh bridge on the next startup allows an explicit retry.
public final class KeychainSecretStore: SecretStore, @unchecked Sendable {
    private let keychain: Keychain
    private let accessibility: KeychainAccessibility?
    private let lock = NSLock()
    private var storedFailure: KeychainFailure?
    public var failure: KeychainFailure? { lock.withLock { storedFailure } }

    public init(keychain: Keychain = Keychain(), accessibility: KeychainAccessibility? = nil) {
        self.keychain = keychain
        self.accessibility = accessibility
    }

    public func get(name: String) -> String? {
        lock.withLock {
            guard storedFailure == nil else { return nil }
            do {
                guard let data = try keychain.readData(name) else { return nil }
                guard let value = String(data: data, encoding: .utf8) else { throw KeychainFailure.invalidData }
                let normalized = value.filter { !$0.isWhitespace }
                guard let decoded = Data(base64Encoded: normalized), !decoded.isEmpty else {
                    throw KeychainFailure.invalidData
                }
                return normalized
            } catch {
                storedFailure = error as? KeychainFailure ?? .invalidData
                return nil
            }
        }
    }

    public func set(name: String, value: String) -> Bool {
        lock.withLock {
            guard storedFailure == nil else { return false }
            do {
                try keychain.writeData(name, Data(value.utf8), accessibility: accessibility)
                return true
            } catch {
                storedFailure = error as? KeychainFailure ?? .invalidData
                return false
            }
        }
    }

    public func delete(name: String) {
        lock.withLock {
            guard storedFailure == nil else { return }
            do { try keychain.deleteChecked(name) }
            catch { storedFailure = error as? KeychainFailure ?? .invalidData }
        }
    }
}
