// SPDX-License-Identifier: GPL-3.0-or-later
import SwiftUI

extension View {
    /// One canvas color, with an opaque navigation surface so scrolled text
    /// Automatic visibility preserves the large title at the top of the scroll view.
    func momentumNavigationCanvas() -> some View {
        // The navigation bar is Liquid Glass (2026-09-29): content scrolls underneath and
        // shows through, and the system's soft scroll-edge effect keeps the title
        // readable instead of an opaque surface that covered the list. The bottom edge
        // stays continuous behind the floating Add task control and the tab bar.
        scrollEdgeEffectStyle(.soft, for: .top)
            .scrollEdgeEffectHidden(true, for: .bottom)
            .toolbarBackgroundVisibility(.automatic, for: .navigationBar)
    }
}
