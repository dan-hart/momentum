// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// What a dragged task row carries: the ids of the whole selection when a selected row is
// dragged, newline-separated as plain text for other apps.
import CoreTransferable
import UniformTypeIdentifiers

public extension UTType {
    static let momentumTasks = UTType(exportedAs: "com.codedbydan.momentum.tasks")
}

public struct TaskTransfer: Codable, Transferable, Sendable {
    public var ids: [String]

    public init(ids: [String]) { self.ids = ids }

    public static var transferRepresentation: some TransferRepresentation {
        CodableRepresentation(contentType: .momentumTasks)
        ProxyRepresentation(exporting: { $0.ids.joined(separator: "\n") })
    }
}
