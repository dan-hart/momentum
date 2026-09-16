// SPDX-License-Identifier: GPL-3.0-or-later
import AppKit
import MomentumKit

/// The native font panel edits the app's typography, rather than a text editor's
/// selection. Keep its target alive while Settings is open.
@MainActor
final class AppFontPanel: NSObject, NSFontChanging {
    static let shared = AppFontPanel()

    func show() {
        let manager = NSFontManager.shared
        manager.target = self
        manager.action = #selector(changeFont(_:))
        manager.setSelectedFont(AppTypography(defaults: .standard).resolvedFont(in: .content), isMultiple: false)
        manager.orderFrontFontPanel(nil)
        manager.fontPanel(false)?.makeKeyAndOrderFront(nil)
    }

    func changeFont(_ sender: NSFontManager?) {
        guard let manager = sender else { return }
        let current = AppTypography(defaults: .standard).resolvedFont(in: .content)
        AppTypography.saveSelection(manager.convert(current))
        refresh()
    }

    func validModesForFontPanel(_ fontPanel: NSFontPanel) -> NSFontPanel.ModeMask {
        [.face, .size, .collection]
    }

    func refresh() {
        let manager = NSFontManager.shared
        guard manager.target === self else { return }
        manager.setSelectedFont(AppTypography(defaults: .standard).resolvedFont(in: .content), isMultiple: false)
    }

    func close() {
        let manager = NSFontManager.shared
        guard manager.target === self else { return }
        manager.fontPanel(false)?.close()
        manager.target = nil
    }
}
