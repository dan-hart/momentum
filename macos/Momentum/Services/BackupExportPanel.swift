// SPDX-License-Identifier: GPL-3.0-or-later
import AppKit
import UniformTypeIdentifiers

@MainActor
enum BackupExportPanel {
    static func choose(filename: String, title: String? = nil, export: (URL) -> Void) {
        #if MOMENTUM_TESTFLIGHT
        // The core verifies a sibling temporary file before atomically replacing
        // the destination. A sandbox grant for one file doesn't cover its sibling.
        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.canCreateDirectories = true
        panel.title = String(localized: "Choose a folder for the export")
        guard panel.runModal() == .OK, let folder = panel.url else { return }
        defer { folder.stopAccessingSecurityScopedResource() }
        let basename = (filename as NSString).deletingPathExtension
        let suffix = UUID().uuidString.prefix(8)
        export(folder.appendingPathComponent("\(basename)-\(suffix).json"))
        #else
        let panel = NSSavePanel()
        panel.allowedContentTypes = [.json]
        panel.nameFieldStringValue = filename
        if let title { panel.title = title }
        if panel.runModal() == .OK, let url = panel.url { export(url) }
        #endif
    }
}
