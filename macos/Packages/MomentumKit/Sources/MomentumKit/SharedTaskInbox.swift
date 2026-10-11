// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// The hand-off between a share extension and the app. The extension never opens the task
// store: it drops one small JSON file per shared item into a folder both processes can
// reach (an App Group container), and the app imports the files the next time it comes to
// the foreground. Each file carries a stable identity, so an import that is interrupted
// and repeated still creates the task exactly once.
import Foundation

/// One item another app shared, as the extension saved it.
public struct SharedTaskItem: Codable, Equatable, Sendable {
    /// Random per share; also the file name and the import identity.
    public var id: String
    public var title: String
    public var notes: String
    public var url: String?
    public var sharedAt: Date

    public init(id: String = UUID().uuidString, title: String, notes: String, url: String?, sharedAt: Date = .now) {
        self.id = id
        self.title = title
        self.notes = notes
        self.url = url
        self.sharedAt = sharedAt
    }

    /// The import identity the core stores on the task, so a repeated drain cannot duplicate it.
    public var sourceID: String { "share:v1:\(id)" }

    /// An id is also a file name, so it must never be able to leave the inbox folder.
    public var hasSafeID: Bool {
        !id.isEmpty && id.count <= 64
            && id.allSatisfy { $0.isLetter || $0.isNumber || $0 == "-" || $0 == "_" }
    }
}

public enum SharedTaskInboxError: Error, Equatable {
    case unsafeID
}

public enum SharedTaskInbox {
    /// The App Group both the app and its share extension declare.
    public static let appGroup = "group.com.codedbydan.Momentum"
    static let folderName = "SharedTasks"

    /// The inbox inside the App Group container, or nil when the group is not available to
    /// this process (a build without the entitlement, or the tests).
    public static func defaultDirectory() -> URL? {
        FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: appGroup)
            .map { $0.appendingPathComponent(folderName, isDirectory: true) }
    }

    /// Saves one item atomically: a sibling temporary file is written in full and then renamed,
    /// so a reader never sees a partial file.
    public static func write(_ item: SharedTaskItem, in directory: URL) throws {
        guard let url = fileURL(for: item, in: directory) else { throw SharedTaskInboxError.unsafeID }
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let data = try encoder.encode(item)
        try data.write(to: url, options: .atomic)
    }

    /// Items waiting for the app, oldest first. Unreadable files are skipped, not deleted, so
    /// a newer app version can still try them.
    public static func pending(in directory: URL) -> [SharedTaskItem] {
        guard let names = try? FileManager.default.contentsOfDirectory(atPath: directory.path) else { return [] }
        return names
            .filter { $0.hasSuffix(".json") }
            .compactMap { name -> SharedTaskItem? in
                guard let data = try? Data(contentsOf: directory.appendingPathComponent(name)),
                      let item = try? decoder.decode(SharedTaskItem.self, from: data),
                      item.hasSafeID, name == item.id + ".json" else { return nil }
                return item
            }
            .sorted { ($0.sharedAt, $0.id) < ($1.sharedAt, $1.id) }
    }

    /// Forgets an item once the app owns it.
    public static func remove(_ item: SharedTaskItem, in directory: URL) {
        guard let url = fileURL(for: item, in: directory) else { return }
        try? FileManager.default.removeItem(at: url)
    }

    /// Nil for an id that is not a plain file name; such an item is never written or removed.
    static func fileURL(for item: SharedTaskItem, in directory: URL) -> URL? {
        guard item.hasSafeID else { return nil }
        return directory.appendingPathComponent(item.id).appendingPathExtension("json")
    }

    private static let encoder: JSONEncoder = {
        let e = JSONEncoder()
        e.dateEncodingStrategy = .iso8601
        e.outputFormatting = [.sortedKeys]
        return e
    }()
    private static let decoder: JSONDecoder = {
        let d = JSONDecoder()
        d.dateDecodingStrategy = .iso8601
        return d
    }()
}
