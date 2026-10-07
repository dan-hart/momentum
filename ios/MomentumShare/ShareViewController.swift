// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// The share extension's root. It reads what the sending app offered, asks the core for a
// normalized draft, shows the compose sheet, and on Add leaves one file in the App Group
// inbox for the app to import. The task store itself is never opened here: an extension
// has a small memory budget and the app may be writing the store at the same time.
import MomentumCore
import MomentumKit
import SwiftUI
import UIKit

final class ShareViewController: UIViewController {
    private var hosting: UIHostingController<ShareComposeView>?

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = .systemGroupedBackground
        Task { await present() }
    }

    @MainActor
    private func present() async {
        let items = (extensionContext?.inputItems as? [NSExtensionItem]) ?? []
        let payload = await SharedPayload.load(from: items)
        let draft = sharedTaskDraft(title: payload.title, text: payload.text, url: payload.url)
        let failure: String?
        if payload.isEmpty || (draft.title.isEmpty && draft.notes.isEmpty) {
            failure = String(localized: "Nothing here can become a task.")
        } else if SharedTaskInbox.defaultDirectory() == nil {
            failure = String(localized: "Momentum can’t receive shared items on this device.")
        } else {
            failure = nil
        }
        let compose = ShareComposeView(
            title: draft.title, notes: draft.notes, url: draft.url, failure: failure,
            add: { [weak self] title in self?.add(title: title, draft: draft) },
            cancel: { [weak self] in self?.cancel() })
        let hosting = UIHostingController(rootView: compose)
        addChild(hosting)
        hosting.view.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(hosting.view)
        NSLayoutConstraint.activate([
            hosting.view.topAnchor.constraint(equalTo: view.topAnchor),
            hosting.view.bottomAnchor.constraint(equalTo: view.bottomAnchor),
            hosting.view.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            hosting.view.trailingAnchor.constraint(equalTo: view.trailingAnchor),
        ])
        hosting.didMove(toParent: self)
        self.hosting = hosting
    }

    private func add(title: String, draft: SharedTaskDraft) {
        guard let directory = SharedTaskInbox.defaultDirectory() else { return cancel() }
        let trimmed = title.trimmingCharacters(in: .whitespacesAndNewlines)
        let item = SharedTaskItem(title: trimmed.isEmpty ? draft.title : trimmed, notes: draft.notes, url: draft.url)
        do {
            try SharedTaskInbox.write(item, in: directory)
            extensionContext?.completeRequest(returningItems: nil)
        } catch {
            extensionContext?.cancelRequest(withError: error)
        }
    }

    private func cancel() {
        extensionContext?.cancelRequest(withError: NSError(domain: NSCocoaErrorDomain, code: NSUserCancelledError))
    }
}
