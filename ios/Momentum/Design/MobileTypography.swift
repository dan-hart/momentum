// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumKit
import MomentumMobile
import SwiftUI
import UIKit

private struct MobileFont: ViewModifier {
    @AppStorage(PrefKey.appFontName) private var fontName = ""
    @AppStorage(MobileAppearance.contentScaleKey) private var contentScale = 1.0
    @AppStorage(MobileAppearance.interfaceScaleKey) private var interfaceScale = 1.0
    @ScaledMetric(relativeTo: .body) private var bodySize = 17.0
    @ScaledMetric(relativeTo: .caption) private var captionSize = 12.0
    var contentArea = false
    var caption = false

    func body(content: Content) -> some View {
        let preferences = MobileAppearance(contentScale: contentScale, interfaceScale: interfaceScale)
        let scale = contentArea ? preferences.contentScale : preferences.interfaceScale
        let baseSize = caption ? 12.0 : 17.0
        let font: Font = fontName.isEmpty || UIFont(name: fontName, size: baseSize) == nil
            ? .system(size: (caption ? captionSize : bodySize) * scale)
            : .custom(fontName, size: baseSize * scale, relativeTo: caption ? .caption : .body)
        content.font(font)
    }
}

extension View {
    func mobileFont(content: Bool = false, caption: Bool = false) -> some View {
        modifier(MobileFont(contentArea: content, caption: caption))
    }
}

struct NativeFontPicker: UIViewControllerRepresentable {
    @Binding var fontName: String
    @Environment(\.dismiss) private var dismiss

    func makeCoordinator() -> Coordinator { Coordinator(parent: self) }
    func makeUIViewController(context: Context) -> UIFontPickerViewController {
        let configuration = UIFontPickerViewController.Configuration()
        configuration.includeFaces = true
        let picker = UIFontPickerViewController(configuration: configuration)
        picker.delegate = context.coordinator
        if let font = UIFont(name: fontName, size: 17) { picker.selectedFontDescriptor = font.fontDescriptor }
        return picker
    }
    func updateUIViewController(_ controller: UIFontPickerViewController, context: Context) {
        context.coordinator.parent = self
    }

    final class Coordinator: NSObject, UIFontPickerViewControllerDelegate {
        var parent: NativeFontPicker
        init(parent: NativeFontPicker) { self.parent = parent }
        func fontPickerViewControllerDidPickFont(_ viewController: UIFontPickerViewController) {
            if let descriptor = viewController.selectedFontDescriptor {
                parent.fontName = UIFont(descriptor: descriptor, size: 17).fontName
            }
            parent.dismiss()
        }
        func fontPickerViewControllerDidCancel(_ viewController: UIFontPickerViewController) { parent.dismiss() }
    }
}
