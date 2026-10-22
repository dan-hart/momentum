// SPDX-License-Identifier: GPL-3.0-or-later
import SwiftUI

/// One native List row per period keeps each navigation action independently accessible.
public struct HiddenDayTaskHint: SwiftUI.View {
    public let count: UInt32
    public let morning: Bool
    public let usesEveningName: Bool
    public let navigate: () -> Void
    public init(count: UInt32, morning: Bool, usesEveningName: Bool = false, navigate: @escaping () -> Void) {
        self.count = count; self.morning = morning
        self.usesEveningName = usesEveningName; self.navigate = navigate
    }
    public var body: some SwiftUI.View {
        Button(action: navigate) {
            Text(label)
                .multilineTextAlignment(.center).fixedSize(horizontal: false, vertical: true)
                .frame(maxWidth: .infinity, minHeight: minimumActionHeight)
                .contentShape(Rectangle())
        }
        .font(.caption).foregroundStyle(.secondary)
        .buttonStyle(.borderless)
        .accessibilityLabel(Text(label))
        .accessibilityIdentifier(morning ? "hidden-tasks-morning" : "hidden-tasks-evening")
    }
    private var minimumActionHeight: CGFloat {
        #if os(iOS)
        44
        #else
        20
        #endif
    }
    private var label: String {
        let key = morning ? (count == 1 ? "One task in Morning" : "%u tasks in Morning")
                          : (count == 1 ? "One task in Evening" : "%u tasks in Evening")
        let format = Bundle.module.localizedString(forKey: key, value: nil, table: nil)
        return String(format: format, count) + " · " + Bundle.module.localizedString(
            forKey: morning ? "Go to Morning" : usesEveningName ? "Go to Evening" : "Go to Tonight", value: nil, table: nil)
    }
}
