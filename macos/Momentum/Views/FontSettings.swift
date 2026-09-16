// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumKit
import SwiftUI

struct FontSettings: SwiftUI.View {
    @Environment(\.appTypography) private var typography
    @AppStorage(PrefKey.contentFontSize) private var contentSize = 13.0
    @AppStorage(PrefKey.interfaceFontSize) private var interfaceSize = 13.0

    private var fontLabel: String {
        guard !typography.fontName.isEmpty,
              let font = NSFont(name: typography.fontName, size: 13) else {
            return String(localized: "System Font")
        }
        return font.displayName ?? font.fontName
    }

    var body: some SwiftUI.View {
        Form {
            Section {
                LabeledContent(String(localized: "App font")) {
                    Text(fontLabel).lineLimit(2).textSelection(.enabled)
                }
                .accessibilityValue(fontLabel)
                Button(String(localized: "Choose Font…")) { AppFontPanel.shared.show() }
            } header: {
                Text(String(localized: "Font"))
            } footer: {
                Text(String(localized: "Choose a family and style in the macOS Fonts panel. Changes appear immediately."))
                    .appFont(.caption)
            }

            Section {
                Stepper(value: $contentSize, in: AppTypography.sizeRange, step: 1) {
                    LabeledContent(String(localized: "Content size")) {
                        Text(String(localized: "\(typography.contentSize.formatted(.number.precision(.fractionLength(0...1)))) pt"))
                            .monospacedDigit()
                    }
                }
                Stepper(value: $interfaceSize, in: AppTypography.sizeRange, step: 1) {
                    LabeledContent(String(localized: "Interface size")) {
                        Text(String(localized: "\(typography.interfaceSize.formatted(.number.precision(.fractionLength(0...1)))) pt"))
                            .monospacedDigit()
                    }
                }
            } header: {
                Text(String(localized: "Text Size"))
            } footer: {
                Text(String(localized: "Content size changes tasks, notes and task entry. Interface size changes navigation, settings and controls. The Fonts panel also sets content size."))
                    .appFont(.caption)
            }

            Section(String(localized: "Preview")) {
                VStack(alignment: .leading, spacing: 8) {
                    Text(String(localized: "Plan a little. Make progress."))
                        .appFont(.body, area: .content)
                    Text(String(localized: "Task details and notes"))
                        .appFont(.caption, area: .content)
                        .foregroundStyle(.secondary)
                    Divider()
                    Label(String(localized: "Projects and settings"), systemImage: "sidebar.left")
                        .appFont()
                }
                .padding(.vertical, 6)
            }
            Section {
                Button(String(localized: "Reset Fonts to Defaults")) {
                    AppTypography.reset()
                    AppFontPanel.shared.refresh()
                }
            }
        }
        .formStyle(.grouped)
        .onChange(of: contentSize) { _, _ in AppFontPanel.shared.refresh() }
        .onDisappear { AppFontPanel.shared.close() }
    }
}
