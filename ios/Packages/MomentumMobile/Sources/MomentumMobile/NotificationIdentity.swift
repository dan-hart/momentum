// SPDX-License-Identifier: GPL-3.0-or-later
import CryptoKit
import Foundation
import MomentumCore

/// A revision has its own OS identifier, so delayed cancellation cannot remove a
/// replacement of the same logical reminder. Metadata is private: never log it.
public enum NotificationIdentity {
    public static let prefix = "momentum.notification.v1."
    private static let version = "1"
    private static let versionKey = "momentum.version"
    private static let idKey = "momentum.id"
    private static let revisionKey = "momentum.revision"

    public static func identifier(for observation: NotificationObservation) -> String {
        var payload = Data()
        // UTF-8 byte lengths make tuple boundaries unambiguous, including Unicode
        // and delimiter characters. Fixed-width big-endian lengths are portable.
        for value in [version, observation.id, observation.sourceRevision] {
            let bytes = Array(value.utf8)
            var length = UInt64(bytes.count).bigEndian
            withUnsafeBytes(of: &length) { payload.append(contentsOf: $0) }
            payload.append(contentsOf: bytes)
        }
        return prefix + SHA256.hash(data: payload).map { String(format: "%02x", $0) }.joined()
    }

    public static func metadata(for observation: NotificationObservation) -> [String: String] {
        [versionKey: version, idKey: observation.id, revisionKey: observation.sourceRevision]
    }

    /// Return nil for unrelated or malformed requests. The native adapter must keep
    /// malformed identifiers in our namespace for explicit, scoped cleanup.
    public static func decode(identifier: String, metadata: [String: String]) -> NotificationObservation? {
        guard identifier.hasPrefix(prefix), metadata[versionKey] == version,
              let id = metadata[idKey], !id.isEmpty,
              let revision = metadata[revisionKey], !revision.isEmpty else { return nil }
        let observation = NotificationObservation(id: id, sourceRevision: revision)
        return self.identifier(for: observation) == identifier ? observation : nil
    }
}
