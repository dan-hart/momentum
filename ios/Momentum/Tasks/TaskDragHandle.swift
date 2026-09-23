// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumKit
import SwiftUI
import UniformTypeIdentifiers
import UIKit

/// A UIKit drag source keeps multi-selection dragging available while SwiftUI's List
/// owns edit-mode gestures. It publishes the same TaskTransfer JSON and plain-text
/// proxy as the shared Transferable used by the SwiftUI drop destinations.
struct TaskDragHandle: UIViewRepresentable {
    let transfer: TaskTransfer
    let label: String

    func makeCoordinator() -> Coordinator { Coordinator(transfer: transfer) }

    func makeUIView(context: Context) -> UIImageView {
        let view = UIImageView(image: UIImage(systemName: "line.3.horizontal"))
        view.contentMode = .center
        view.tintColor = .secondaryLabel
        view.preferredSymbolConfiguration = UIImage.SymbolConfiguration(textStyle: .body,
                                                                         scale: .medium)
        view.isUserInteractionEnabled = true
        view.isAccessibilityElement = true
        view.accessibilityTraits = .image
        view.accessibilityLabel = label
        view.addInteraction(UIDragInteraction(delegate: context.coordinator))
        return view
    }

    func updateUIView(_ view: UIImageView, context: Context) {
        context.coordinator.transfer = transfer
        view.accessibilityLabel = label
    }

    func sizeThatFits(_ proposal: ProposedViewSize, uiView: UIImageView,
                      context: Context) -> CGSize? {
        CGSize(width: 44, height: 44)
    }

    final class Coordinator: NSObject, UIDragInteractionDelegate {
        var transfer: TaskTransfer

        init(transfer: TaskTransfer) { self.transfer = transfer }

        func dragInteraction(_ interaction: UIDragInteraction,
                             itemsForBeginning session: any UIDragSession) -> [UIDragItem] {
            guard let data = try? JSONEncoder().encode(transfer) else { return [] }
            let provider = NSItemProvider()
            provider.registerDataRepresentation(forTypeIdentifier: UTType.momentumTasks.identifier,
                                                visibility: .all) { completion in
                completion(data, nil)
                return nil
            }
            provider.registerObject(transfer.ids.joined(separator: "\n") as NSString,
                                    visibility: .all)
            return [UIDragItem(itemProvider: provider)]
        }

        func dragInteraction(_ interaction: UIDragInteraction,
                             sessionAllowsMoveOperation session: any UIDragSession) -> Bool {
            true
        }
    }
}
