// SPDX-License-Identifier: GPL-3.0-or-later
import SwiftUI

extension View {
    /// One canvas color, with an opaque navigation surface so scrolled text
    /// cannot overlap the title after removing the scroll-edge gradient.
    /// Automatic visibility preserves the large title at the top of the scroll view.
    func momentumNavigationCanvas() -> some View {
        scrollEdgeEffectHidden(true, for: [.top, .bottom])
            .toolbarBackground(Color(uiColor: .systemGroupedBackground), for: .navigationBar)
            .toolbarBackgroundVisibility(.automatic, for: .navigationBar)
    }
}
