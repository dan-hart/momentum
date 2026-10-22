// SPDX-License-Identifier: GPL-3.0-or-later
import SwiftUI

/// Shared native controls; audition and deliberate selection are separate actions.
public struct SoundSettingsSection: View {
    @Environment(\.locale) private var locale
    @Environment(\.scenePhase) private var scenePhase
    @AppStorage(PrefKey.completionSound) private var enabled = false
    @AppStorage(PrefKey.completionSoundChoice) private var selected = CompletionSoundChoice.defaultChoice.rawValue
    @AppStorage(PrefKey.completionSoundVolume) private var volume = 0.35
    private var normalizedVolume: Double { Double(CompletionSound.boundedVolume(volume)) }
    private func localized(_ key: String) -> String {
        let language = locale.language.languageCode?.identifier ?? "en"
        let bundle = Bundle.module.path(forResource: language, ofType: "lproj").flatMap(Bundle.init(path:)) ?? .module
        return bundle.localizedString(forKey: key, value: nil, table: nil)
    }
    public init() {}
    public var body: some View {
        Section {
            Toggle(localized("Completion sound"), isOn: $enabled).settingsTarget("sounds.enabled")
            VStack(alignment: .leading) {
                Text(localized("Volume"))
                Slider(value: Binding(get: { normalizedVolume }, set: { volume = $0 }), in: 0...1).settingsTarget("sounds.volume")
                    .accessibilityLabel(localized("Volume"))
                    .accessibilityValue(normalizedVolume.formatted(.percent.precision(.fractionLength(0))))
                Text(normalizedVolume.formatted(.percent.precision(.fractionLength(0)))).font(.caption).foregroundStyle(.secondary)
            }
            if let old = CompletionSoundChoice(rawValue: selected), old.isLegacy {
                candidate(old, legacy: true)
            } else if CompletionSoundChoice(rawValue: selected) == nil {
                Text(localized("Saved sound from another version"))
            }
            ForEach(CompletionSoundChoice.candidates) { candidate($0) }.settingsTarget("sounds.selection")
        } header: {
            Text(localized("Sounds & Feedback"))
        } footer: {
            Text(localized("Preview any sound before choosing it. Volume follows your device's sound settings."))
        }
        .onDisappear { CompletionSound.shared.stop() }
        .onChange(of: scenePhase) { _, phase in if phase != .active { CompletionSound.shared.stop() } }
    }
    private func candidate(_ choice: CompletionSoundChoice, legacy: Bool = false) -> some View {
        HStack {
            Button { selected = choice.rawValue } label: {
                HStack {
                    Image(systemName: selected == choice.rawValue ? "checkmark.circle.fill" : "circle").accessibilityHidden(true)
                    Text(choice.label(locale: locale) + (legacy ? " (" + localized("Legacy") + ")" : ""))
                }
            }
            .buttonStyle(.plain)
            .accessibilityLabel(Text(choice.label(locale: locale) + (legacy ? " (" + localized("Legacy") + ")" : "")))
            .accessibilityIdentifier("completion-sound-select-" + choice.rawValue)
            .accessibilityAddTraits(selected == choice.rawValue ? .isSelected : [])
            Spacer()
            Button { CompletionSound.shared.preview(choice, volume: normalizedVolume) } label: {
                Image(systemName: "play.circle")
            }
            .buttonStyle(.borderless)
            .accessibilityLabel(localized("Preview") + " " + choice.label(locale: locale))
            .accessibilityIdentifier("completion-sound-preview-" + choice.rawValue)
        }
    }
}
