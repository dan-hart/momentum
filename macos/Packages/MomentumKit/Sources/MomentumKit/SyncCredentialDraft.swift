// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
import Foundation
import Observation

/// Keeps a failed credential edit visible without claiming the saved connection
/// matches it. A failed initial read requires a reload before replacement is allowed.
@MainActor @Observable public final class SyncCredentialDraft {
    public private(set) var value = ""
    public private(set) var hasError = false
    private var savedValue: String?
    private let account: String

    public init(account: String) { self.account = account }
    public var isLoaded: Bool { savedValue != nil }
    public var canUseSavedValue: Bool { isLoaded && !hasError && value == savedValue }

    public func load(from keychain: Keychain) {
        do {
            let data = try keychain.readData(account)
            if let data {
                guard let decoded = String(data: data, encoding: .utf8) else { throw KeychainFailure.invalidData }
                value = decoded
            } else {
                value = ""
            }
            savedValue = value
            hasError = false
        } catch {
            savedValue = nil
            hasError = true
        }
    }

    public func save(_ newValue: String, to keychain: Keychain) {
        guard isLoaded else { return }
        value = newValue
        if keychain.set(account, newValue) {
            savedValue = newValue
            hasError = false
        } else {
            hasError = true
        }
    }

    public func retry(using keychain: Keychain) {
        if isLoaded { save(value, to: keychain) }
        else { load(from: keychain) }
    }
}
