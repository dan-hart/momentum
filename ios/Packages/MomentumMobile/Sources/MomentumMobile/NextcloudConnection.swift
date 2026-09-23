// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import MomentumKit

public enum NextcloudConnectionIssue: String, Error, Equatable, Sendable {
    case serverURLRequired
    case serverURLInvalid
    case secureServerRequired
    case userNameRequired
    case passwordRequired
    case folderRequired
}

/// Presentation configuration only; protocol, encryption and merging stay in Rust.
public struct NextcloudConnection: Codable, Equatable, Sendable {
    public var serverURL = ""
    public var userName = ""
    public var password = ""
    public var folder = "super-productivity"
    public var encryptionPassword = ""
    public var compress = false
    public var automatic = true
    public init() {}

    public var settings: NextcloudSettings {
        NextcloudSettings(serverUrl: serverURL, userName: userName, password: password,
                          folder: folder, compress: compress,
                          encryptionPassword: encryptionPassword.isEmpty ? nil : encryptionPassword)
    }

    public var validationIssue: NextcloudConnectionIssue? {
        do { _ = try validated(); return nil }
        catch let issue as NextcloudConnectionIssue { return issue }
        catch { return .serverURLInvalid }
    }

    /// Returns the canonical record stored and sent to Rust. Secrets are never trimmed:
    /// whitespace can be a valid app-password or encryption-password character.
    public func validated() throws -> Self {
        var value = self
        value.serverURL = serverURL.trimmingCharacters(in: .whitespacesAndNewlines)
        value.userName = userName.trimmingCharacters(in: .whitespacesAndNewlines)
        value.folder = folder.trimmingCharacters(in: .whitespacesAndNewlines)
            .trimmingCharacters(in: CharacterSet(charactersIn: "/"))

        guard !value.serverURL.isEmpty else { throw NextcloudConnectionIssue.serverURLRequired }
        guard let components = URLComponents(string: value.serverURL),
              let scheme = components.scheme?.lowercased(),
              let host = components.host, !host.isEmpty,
              components.user == nil, components.password == nil,
              components.query == nil, components.fragment == nil,
              components.url != nil,
              scheme == "https" || scheme == "http"
        else { throw NextcloudConnectionIssue.serverURLInvalid }
        let isLoopback = host.caseInsensitiveCompare("localhost") == .orderedSame
            || host == "127.0.0.1" || host == "::1" || host == "[::1]"
        guard scheme == "https" || isLoopback else { throw NextcloudConnectionIssue.secureServerRequired }
        guard !value.userName.isEmpty else { throw NextcloudConnectionIssue.userNameRequired }
        guard !value.password.isEmpty else { throw NextcloudConnectionIssue.passwordRequired }
        guard !value.folder.isEmpty else { throw NextcloudConnectionIssue.folderRequired }

        while value.serverURL.hasSuffix("/") { value.serverURL.removeLast() }
        return value
    }
}

public protocol NextcloudConnectionPersistence: Sendable {
    func load() async throws -> NextcloudConnection?
    func save(_ connection: NextcloudConnection) async throws
}

/// One atomic Keychain item prevents mixing an edited server/user with old secrets.
/// This is a new mobile record; the established desktop/CLI accounts are unchanged.
public actor NextcloudConnectionStore: NextcloudConnectionPersistence {
    public static let account = "nextcloud-mobile-connection-v1"
    private let keychain: Keychain
    public init(keychain: Keychain = Keychain()) { self.keychain = keychain }

    public func load() throws -> NextcloudConnection? {
        guard let data = try keychain.readData(Self.account) else { return nil }
        return try JSONDecoder().decode(NextcloudConnection.self, from: data)
    }

    public func save(_ connection: NextcloudConnection) throws {
        let data = try JSONEncoder().encode(connection)
        try keychain.writeData(Self.account, data, accessibility: .afterFirstUnlockThisDeviceOnly)
    }
}
