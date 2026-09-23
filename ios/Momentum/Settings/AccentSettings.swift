// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumMobile
import SwiftUI

struct AccentSettings: View {
    @Environment(MobileAppModel.self) private var model
    @AppStorage(AccentChoice.preferenceKey) private var selected = AccentChoice.momentum.id

    var body: some View {
        Form {
            Section {
                AccentChoiceRow(choice: .momentum, selected: $selected,
                                onSelectionChange: model.accentSelectionDidChange)
            } header: { Text("Momentum Default").foregroundStyle(AccentTheme.secondaryText) } footer: {
                Text("Momentum Default stays orange in every appearance. Custom colors adapt for readable text and controls.")
                    .foregroundStyle(AccentTheme.secondaryText)
            }
            Section {
                ForEach(AccentPalette.all.flatMap(\.colors)) { choice in
                    AccentChoiceRow(choice: choice, selected: $selected,
                                    onSelectionChange: model.accentSelectionDidChange)
                }
            } header: {
                Label { Text("DHFlatUIColors") } icon: {
                    Image(systemName: "paintpalette.fill").accessibilityHidden(true)
                }.foregroundStyle(AccentTheme.secondaryText)
            }
            Section {
                Label("Ready for today", systemImage: "checkmark.circle.fill")
                    .foregroundStyle(.tint)
                Text("Body text keeps its system color for readability.").foregroundStyle(Color.primary)
            } header: { Text("Preview").foregroundStyle(AccentTheme.secondaryText) }
        }
        .navigationTitle("Accent Color")
        .momentumNavigationCanvas()
    }
}

struct AccentChoiceRow: View {
    let choice: AccentChoice
    @Binding var selected: String
    var onSelectionChange: (String, String) -> Void = { _, _ in }
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    private var isSelected: Bool { AccentChoice.resolve(selected).id == choice.id }

    var body: some View {
        let rgb = RGBColor(hex: choice.hex)!
        let motion = InteractionMotionPolicy.presentation(reduceMotion: reduceMotion)
        Button {
            let previous = selected
            selected = choice.id
            onSelectionChange(previous, choice.id)
        } label: {
            HStack(spacing: 12) {
                RoundedRectangle(cornerRadius: 9)
                    .fill(rgb.color)
                    .overlay { RoundedRectangle(cornerRadius: 9).stroke(AccentTheme.secondaryText, lineWidth: 1) }
                    .frame(width: 36, height: 36).accessibilityHidden(true)
                VStack(alignment: .leading, spacing: 2) {
                    Text(verbatim: choice.name.localizedCapitalized).foregroundStyle(Color.primary)
                    Text(verbatim: choice.hex).font(.caption.monospaced()).foregroundStyle(AccentTheme.secondaryText)
                }.fixedSize(horizontal: false, vertical: true)
                Spacer(minLength: 0)
                if isSelected {
                    Image(systemName: "checkmark")
                        .foregroundStyle(Color.primary)
                        .accessibilityHidden(true)
                        .transition(motion.transition)
                }
            }
            .frame(minHeight: 44)
            .contentShape(Rectangle())
        }
        .accessibilityIdentifier("accent-" + choice.id)
        .accessibilityLabel("\(choice.name.localizedCapitalized), \(choice.hex)")
        .accessibilityAddTraits(isSelected ? .isSelected : [])
        .animation(motion.animation, value: isSelected)
    }
}
