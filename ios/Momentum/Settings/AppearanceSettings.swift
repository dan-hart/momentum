// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumKit
import MomentumMobile
import SwiftUI
import UIKit

struct AppearanceSettings: View {
    @AppStorage(PrefKey.appFontName) private var fontName = ""
    @AppStorage(MobileAppearance.contentScaleKey) private var contentScale = 1.0
    @AppStorage(MobileAppearance.interfaceScaleKey) private var interfaceScale = 1.0
    @AppStorage(MobileAppearance.hapticsKey) private var haptics = true
    @State private var pickingFont = false

    var body: some View {
        Form {
            Section {
                Button { pickingFont = true } label: {
                    LabeledContent("Font", value: fontName.isEmpty ? String(localized: "System") : fontName)
                }
                Stepper(value: $contentScale, in: MobileAppearance.scaleRange, step: 0.1) {
                    LabeledContent("Task text", value: contentScale.formatted(.percent.precision(.fractionLength(0))))
                }
                Stepper(value: $interfaceScale, in: MobileAppearance.scaleRange, step: 0.1) {
                    LabeledContent("Interface text", value: interfaceScale.formatted(.percent.precision(.fractionLength(0))))
                }
                Button("Reset Typography") { fontName = ""; contentScale = 1; interfaceScale = 1 }
            } header: { Text("Typography").foregroundStyle(AccentTheme.secondaryText) }
            Section {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Make room for what matters").mobileFont(content: true)
                    Text("Your tasks, at your pace.").mobileFont(content: true, caption: true).foregroundStyle(AccentTheme.secondaryText)
                }.padding(.vertical, 8)
            } header: { Text("Preview").foregroundStyle(AccentTheme.secondaryText) } footer: {
                Text("Text also follows your device's Dynamic Type size. Navigation and system controls keep their native styles.")
                    .foregroundStyle(AccentTheme.secondaryText)
            }
            Section { Toggle("Haptic feedback", isOn: $haptics) }
                header: { Text("Feedback").foregroundStyle(AccentTheme.secondaryText) }
        }
        .navigationTitle("Text & Feedback")
        .momentumNavigationCanvas()
        .sheet(isPresented: $pickingFont) { NativeFontPicker(fontName: $fontName) }
    }
}
