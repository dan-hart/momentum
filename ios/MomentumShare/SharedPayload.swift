// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// What the share sheet handed over, reduced to the three things the core drafts a task
// from: a title (a page title when the sending app offers one), plain text, and a web link.
import Foundation
import UniformTypeIdentifiers

struct SharedPayload: Equatable, Sendable {
    var title: String?
    var text: String?
    var url: String?

    var isEmpty: Bool { title == nil && text == nil && url == nil }

    /// Reads every attachment the sending app provided. Loading is best effort: a provider
    /// that fails is skipped so one odd attachment never blocks the share.
    @MainActor
    static func load(from items: [NSExtensionItem]) async -> SharedPayload {
        var payload = SharedPayload()
        for item in items {
            if payload.title == nil, let t = item.attributedTitle?.string.nilIfBlank { payload.title = t }
            if payload.text == nil, let t = item.attributedContentText?.string.nilIfBlank { payload.text = t }
            for provider in item.attachments ?? [] {
                if payload.url == nil, provider.hasItemConformingToTypeIdentifier(UTType.url.identifier) {
                    if let loaded = try? await provider.loadItem(forTypeIdentifier: UTType.url.identifier),
                       let url = Self.url(from: loaded), !url.isFileURL {
                        payload.url = url.absoluteString
                        continue
                    }
                }
                if payload.text == nil, provider.hasItemConformingToTypeIdentifier(UTType.plainText.identifier) {
                    if let loaded = try? await provider.loadItem(forTypeIdentifier: UTType.plainText.identifier),
                       let text = Self.text(from: loaded)?.nilIfBlank {
                        payload.text = text
                    }
                }
            }
        }
        // Safari offers the page title as the content text; the core treats it as a title.
        if payload.title == nil, let url = payload.url, let text = payload.text, !text.contains(url) {
            payload.title = text
            payload.text = nil
        }
        return payload
    }

    private static func url(from loaded: any NSSecureCoding) -> URL? {
        if let url = loaded as? URL { return url }
        if let data = loaded as? Data, let s = String(data: data, encoding: .utf8) { return URL(string: s) }
        if let s = loaded as? String { return URL(string: s) }
        return nil
    }

    private static func text(from loaded: any NSSecureCoding) -> String? {
        if let s = loaded as? String { return s }
        if let data = loaded as? Data { return String(data: data, encoding: .utf8) }
        if let attributed = loaded as? NSAttributedString { return attributed.string }
        return nil
    }
}

extension String {
    var nilIfBlank: String? { trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? nil : self }
}
