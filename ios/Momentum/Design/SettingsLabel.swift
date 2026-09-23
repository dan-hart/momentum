// SPDX-License-Identifier: GPL-3.0-or-later
import SwiftUI

struct SettingsLabel: View {
    let title: LocalizedStringKey
    let symbol: String
    var subtitle: LocalizedStringKey? = nil
    init(_ title: LocalizedStringKey, symbol: String, subtitle: LocalizedStringKey? = nil) {
        self.title = title
        self.symbol = symbol
        self.subtitle = subtitle
    }
    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: 12) {
            Image(systemName: symbol).symbolRenderingMode(.hierarchical)
                .foregroundStyle(.tint).frame(width: 28)
                .accessibilityHidden(true)
            VStack(alignment: .leading, spacing: 3) {
                Text(title).foregroundStyle(Color.primary)
                if let subtitle {
                    Text(subtitle).font(.footnote).foregroundStyle(AccentTheme.secondaryText)
                }
            }.fixedSize(horizontal: false, vertical: true)
                .padding(.vertical, subtitle == nil ? 0 : 4)
        }
        .fixedSize(horizontal: false, vertical: true)
    }
}
