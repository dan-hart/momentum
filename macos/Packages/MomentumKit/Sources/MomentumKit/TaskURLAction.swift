// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation

/// Native URL decoding only. Parsing task text, dates and mutations stays in Rust.
public enum TaskURLAction: Sendable, Equatable {
    case create(text: String, notes: String?, due: String?)
    case complete(title: String)

    public init?(url: URL) {
        guard let scheme = url.scheme?.lowercased(),
              ["momentum", "superproductivity"].contains(scheme),
              let components = URLComponents(url: url, resolvingAgainstBaseURL: false) else { return nil }
        func parameter(_ name: String) -> String? {
            components.queryItems?.first { $0.name == name }?.value
        }
        switch components.host {
        case "add", "create-task":
            guard var text = parameter("title"),
                  !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return nil }
            if let tags = parameter("tags") {
                for tag in tags.split(separator: ",").map({ $0.trimmingCharacters(in: .whitespaces) }) where !tag.isEmpty {
                    text += " #\(tag)"
                }
            }
            self = .create(text: text, notes: parameter("notes"), due: parameter("due"))
        case "complete-task":
            guard let title = parameter("title") else { return nil }
            self = .complete(title: title)
        default: return nil
        }
    }
}
