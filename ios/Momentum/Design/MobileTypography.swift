// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumKit
import MomentumMobile
import SwiftUI
import UIKit

/// Preserve semantic text sizes while applying the selected family and independent
/// app scale. SwiftUI's relativeTo scaling continues to follow Dynamic Type.
enum MobileTextStyle: CaseIterable {
    case body, caption, footnote, callout, headline, title, title2, title3

    var textStyle: Font.TextStyle {
        switch self {
        case .body: .body
        case .caption: .caption
        case .footnote: .footnote
        case .callout: .callout
        case .headline: .headline
        case .title: .title
        case .title2: .title2
        case .title3: .title3
        }
    }

    var uiStyle: UIFont.TextStyle {
        switch self {
        case .body: .body
        case .caption: .caption1
        case .footnote: .footnote
        case .callout: .callout
        case .headline: .headline
        case .title: .title1
        case .title2: .title2
        case .title3: .title3
        }
    }

    var baseSize: Double {
        Double(UIFont.preferredFont(forTextStyle: uiStyle,
            compatibleWith: UITraitCollection(preferredContentSizeCategory: .large)).pointSize)
    }
}

enum MobileTypography {
    static func resolvedName(_ name: String) -> String? {
        BundledAppFont.register()
        return name.isEmpty || UIFont(name: name, size: 17) == nil ? nil : name
    }


}

private struct MobileFont: ViewModifier {
    @AppStorage(PrefKey.appFontName) private var fontName = ""
    @AppStorage(MobileAppearance.contentScaleKey) private var contentScale = 1.0
    @AppStorage(MobileAppearance.interfaceScaleKey) private var interfaceScale = 1.0
    @ScaledMetric private var scaledSize: Double
    let contentArea: Bool
    let style: MobileTextStyle

    init(contentArea: Bool, style: MobileTextStyle) {
        self.contentArea = contentArea
        self.style = style
        _scaledSize = ScaledMetric(wrappedValue: style.baseSize, relativeTo: style.textStyle)
    }

    func body(content: Content) -> some View {
        let preferences = MobileAppearance(contentScale: contentScale, interfaceScale: interfaceScale)
        let scale = contentArea ? preferences.contentScale : preferences.interfaceScale
        let font: Font = if let name = MobileTypography.resolvedName(fontName) {
            .custom(name, size: style.baseSize * scale, relativeTo: style.textStyle)
        } else {
            .system(size: scaledSize * scale)
        }
        content.font(style == .headline ? font.weight(.semibold) : font)
    }
}

extension View {
    func mobileFont(content: Bool = false, caption: Bool = false) -> some View {
        modifier(MobileFont(contentArea: content, style: caption ? .caption : .body))
    }

    func mobileFont(_ style: MobileTextStyle, content: Bool = false) -> some View {
        modifier(MobileFont(contentArea: content, style: style))
    }
}

struct NativeFontPicker: UIViewControllerRepresentable {
    @Binding var fontName: String
    @Environment(\.dismiss) private var dismiss

    func makeCoordinator() -> Coordinator { Coordinator(parent: self) }
    func makeUIViewController(context: Context) -> UIFontPickerViewController {
        BundledAppFont.register()
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
