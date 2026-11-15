// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumKit
import MomentumMobile
import SwiftUI
import UIKit

/// Keep the native title and its scroll transition while applying app typography
/// to the UIKit bar that owns them. Each stack (including sheets) has its own bar.
private struct NavigationTypography: ViewModifier {
    @AppStorage(PrefKey.appFontName) private var fontName = ""
    @AppStorage(MobileAppearance.interfaceScaleKey) private var interfaceScale = 1.0
    @ScaledMetric(relativeTo: .headline) private var inlineSize = MobileTextStyle.headline.baseSize
    @ScaledMetric(relativeTo: .largeTitle) private var largeSize = MobileTextStyle.largeTitle.baseSize

    func body(content: Content) -> some View {
        let name = MobileTypography.resolvedName(fontName)
        let scale = MobileAppearance(interfaceScale: interfaceScale).interfaceScale
        let fonts = name != nil || scale != 1
        content.background(NavigationFontBridge(
            inlineFont: fonts ? MobileTypography.navigationFont(name: name, size: inlineSize * scale, style: .headline) : nil,
            largeFont: fonts ? MobileTypography.navigationFont(name: name, size: largeSize * scale, style: .largeTitle) : nil
        ))
    }
}

private struct NavigationFontBridge: UIViewControllerRepresentable {
    let inlineFont: UIFont?
    let largeFont: UIFont?

    func makeUIViewController(context: Context) -> Controller { Controller() }

    func updateUIViewController(_ controller: Controller, context: Context) {
        controller.inlineFont = inlineFont
        controller.largeFont = largeFont
        controller.applyFonts()
    }

    final class Controller: UIViewController {
        var inlineFont: UIFont?
        var largeFont: UIFont?

        override func loadView() {
            let view = UIView()
            view.isUserInteractionEnabled = false
            self.view = view
        }

        override func viewWillAppear(_ animated: Bool) {
            super.viewWillAppear(animated)
            applyFonts()
        }

        override func viewDidLayoutSubviews() {
            super.viewDidLayoutSubviews()
            applyFonts()
        }

        func applyFonts() {
            guard let navigationBar = navigationController?.navigationBar else { return }
            if let appearance = updated(navigationBar.standardAppearance) {
                navigationBar.standardAppearance = appearance
            }
            if let scrollEdge = navigationBar.scrollEdgeAppearance, let appearance = updated(scrollEdge) {
                navigationBar.scrollEdgeAppearance = appearance
            }
            if let compact = navigationBar.compactAppearance, let appearance = updated(compact) {
                navigationBar.compactAppearance = appearance
            }
            if let compactEdge = navigationBar.compactScrollEdgeAppearance, let appearance = updated(compactEdge) {
                navigationBar.compactScrollEdgeAppearance = appearance
            }
        }

        private func updated(_ original: UINavigationBarAppearance) -> UINavigationBarAppearance? {
            let inlineSpacing = inlineFont.flatMap { MobileTypography.characterSpacing(for: $0.fontName, size: $0.pointSize) }
            let largeSpacing = largeFont.flatMap { MobileTypography.characterSpacing(for: $0.fontName, size: $0.pointSize) }
            guard original.titleTextAttributes[.font] as? UIFont != inlineFont
                    || original.largeTitleTextAttributes[.font] as? UIFont != largeFont
                    || original.titleTextAttributes[.kern] as? CGFloat != inlineSpacing
                    || original.largeTitleTextAttributes[.kern] as? CGFloat != largeSpacing else { return nil }
            let appearance = original.copy()
            appearance.titleTextAttributes[.font] = inlineFont
            appearance.titleTextAttributes[.kern] = inlineSpacing
            appearance.largeTitleTextAttributes[.font] = largeFont
            appearance.largeTitleTextAttributes[.kern] = largeSpacing
            return appearance
        }
    }
}

extension View {
    func momentumNavigationTypography() -> some View { modifier(NavigationTypography()) }
}
