// SPDX-License-Identifier: GPL-3.0-or-later
import SwiftUI

/// A consistent, readable error presentation. Callers own focus timing so an
/// inline validation hint can remain on its field while async failures move focus.
struct AccessibleErrorMessage: View {
    @Environment(\.dynamicTypeSize) private var dynamicTypeSize
    private let message: Text

    init(_ message: String) { self.message = Text(verbatim: message) }
    init(_ message: LocalizedStringKey) { self.message = Text(message) }

    var body: some View {
        Group {
            if dynamicTypeSize.isAccessibilitySize {
                VStack(alignment: .leading, spacing: 8) {
                    symbol
                    message
                }
            } else {
                Label { message } icon: { symbol }
            }
        }
        .foregroundStyle(AccentTheme.errorText)
        .fixedSize(horizontal: false, vertical: true)
        .frame(minHeight: 44, alignment: .leading)
        .accessibilityElement(children: .combine)
    }

    private var symbol: some View {
        Image(systemName: "exclamationmark.circle.fill").accessibilityHidden(true)
    }
}
