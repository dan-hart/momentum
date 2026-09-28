// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// What a dragged task row carries. A drag of several selected rows is one item per task,
// so the system shows every row lifting and other apps receive one line of text each.
import CoreTransferable
import UniformTypeIdentifiers

public extension UTType {
    static let momentumTasks = UTType(exportedAs: "com.codedbydan.momentum.tasks")
}

public struct TaskTransfer: Codable, Transferable, Sendable, Identifiable, Equatable {
    public var ids: [String]

    public init(ids: [String]) { self.ids = ids }

    /// The drag container matches items to the rows they lifted from by this id.
    public var id: String { ids.joined(separator: "\n") }

    public static var transferRepresentation: some TransferRepresentation {
        CodableRepresentation(contentType: .momentumTasks)
        ProxyRepresentation(exporting: { $0.ids.joined(separator: "\n") })
    }
}
