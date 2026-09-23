// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore
import Testing
@testable import MomentumMobile

@Suite struct NotificationIdentityTests {
    @Test func exactRevisionRoundTripsWithoutExposingContentInIdentifier() {
        let observation = NotificationObservation(id: "reminder:task:1000", sourceRevision: "Private task 🧡")
        let identifier = NotificationIdentity.identifier(for: observation)
        // Independent SHA-256 vector pins the versioned wire identity across releases.
        #expect(identifier == "momentum.notification.v1.f1f593ac2349744e4bf5dbda04655870b54859520dd32609468edfa0ec653383")
        #expect(identifier.hasPrefix(NotificationIdentity.prefix))
        #expect(!identifier.contains("Private"))
        #expect(!identifier.contains("task"))
        #expect(NotificationIdentity.decode(identifier: identifier,
            metadata: NotificationIdentity.metadata(for: observation)) == observation)
    }

    @Test func tupleBoundariesAndUnicodeRemainDistinct() {
        let inputs = [
            NotificationObservation(id: "a", sourceRevision: "bc"),
            NotificationObservation(id: "ab", sourceRevision: "c"),
            NotificationObservation(id: "🧡", sourceRevision: ":1:a"),
            NotificationObservation(id: "🧡:1:", sourceRevision: "a")
        ]
        #expect(Set(inputs.map { NotificationIdentity.identifier(for: $0) }).count == inputs.count)
        for item in inputs {
            #expect(NotificationIdentity.decode(identifier: NotificationIdentity.identifier(for: item),
                metadata: NotificationIdentity.metadata(for: item)) == item)
        }
    }

    @Test func changedSourceHasSeparateNativeIdentity() {
        let before = NotificationObservation(id: "same-logical-request", sourceRevision: "old")
        let after = NotificationObservation(id: before.id, sourceRevision: "new")
        #expect(NotificationIdentity.identifier(for: before) != NotificationIdentity.identifier(for: after))
        #expect(NotificationIdentity.decode(identifier: NotificationIdentity.identifier(for: before),
            metadata: NotificationIdentity.metadata(for: after)) == nil)
    }

    @Test func unrelatedOrMalformedMetadataIsRejected() {
        let observation = NotificationObservation(id: "id", sourceRevision: "revision")
        let identifier = NotificationIdentity.identifier(for: observation)
        let metadata = NotificationIdentity.metadata(for: observation)
        #expect(NotificationIdentity.decode(identifier: "another.app.request", metadata: metadata) == nil)
        #expect(NotificationIdentity.decode(identifier: identifier, metadata: [:]) == nil)
        for key in metadata.keys {
            var missing = metadata
            missing.removeValue(forKey: key)
            #expect(NotificationIdentity.decode(identifier: identifier, metadata: missing) == nil)
            var altered = metadata
            altered[key] = "unsupported-or-changed"
            #expect(NotificationIdentity.decode(identifier: identifier, metadata: altered) == nil)
        }
        for invalid in [NotificationObservation(id: "", sourceRevision: "r"),
                        NotificationObservation(id: "id", sourceRevision: "")] {
            #expect(NotificationIdentity.decode(identifier: NotificationIdentity.identifier(for: invalid),
                metadata: NotificationIdentity.metadata(for: invalid)) == nil)
        }
    }
}
