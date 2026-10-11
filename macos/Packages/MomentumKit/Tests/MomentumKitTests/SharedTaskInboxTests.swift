// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
import Foundation
import Testing

@testable import MomentumKit

@Suite struct SharedTaskInboxTests {
    @Test func itemsRoundTripInOrderAndLeaveWhenRemoved() throws {
        let dir = TempDir().url.appendingPathComponent("SharedTasks")
        #expect(SharedTaskInbox.pending(in: dir).isEmpty, "a missing folder is an empty inbox")
        let later = SharedTaskItem(id: "b", title: "Second", notes: "", url: nil, sharedAt: Date(timeIntervalSince1970: 200))
        let first = SharedTaskItem(id: "a", title: "First", notes: "https://example.com\n\nSome text", url: "https://example.com",
                                   sharedAt: Date(timeIntervalSince1970: 100))
        try SharedTaskInbox.write(later, in: dir)
        try SharedTaskInbox.write(first, in: dir)
        let pending = SharedTaskInbox.pending(in: dir)
        #expect(pending.map(\.id) == ["a", "b"], "oldest first")
        #expect(pending.first == first)
        #expect(first.sourceID == "share:v1:a")
        SharedTaskInbox.remove(first, in: dir)
        #expect(SharedTaskInbox.pending(in: dir).map(\.id) == ["b"])
    }

    @Test func anIDThatIsNotAPlainFileNameIsRefusedEverywhere() throws {
        let dir = TempDir().url.appendingPathComponent("SharedTasks")
        let outside = TempDir()
        let marker = outside.url.appendingPathComponent("marker.json")
        try FileManager.default.createDirectory(at: outside.url, withIntermediateDirectories: true)
        try Data("keep".utf8).write(to: marker)
        let escaping = SharedTaskItem(id: "../../" + outside.url.lastPathComponent + "/marker", title: "x", notes: "", url: nil)
        #expect(!escaping.hasSafeID)
        #expect(throws: SharedTaskInboxError.unsafeID) { try SharedTaskInbox.write(escaping, in: dir) }
        SharedTaskInbox.remove(escaping, in: dir)
        #expect(FileManager.default.fileExists(atPath: marker.path), "remove never follows an escaping id")
        // A file whose content claims another id is ignored rather than trusted.
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let honest = SharedTaskItem(id: "honest", title: "Fine", notes: "", url: nil)
        try SharedTaskInbox.write(honest, in: dir)
        let forged = try Data(contentsOf: dir.appendingPathComponent("honest.json"))
        try forged.write(to: dir.appendingPathComponent("other.json"))
        #expect(SharedTaskInbox.pending(in: dir).map(\.id) == ["honest"])
    }

    @Test func unreadableFilesAreSkippedButKept() throws {
        let dir = TempDir().url.appendingPathComponent("SharedTasks")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let junk = dir.appendingPathComponent("junk.json")
        try Data("not json".utf8).write(to: junk)
        try SharedTaskInbox.write(SharedTaskItem(id: "ok", title: "Fine", notes: "", url: nil), in: dir)
        #expect(SharedTaskInbox.pending(in: dir).map(\.id) == ["ok"])
        #expect(FileManager.default.fileExists(atPath: junk.path), "a newer app may understand it")
    }
}
