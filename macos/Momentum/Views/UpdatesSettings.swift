// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumKit
import SwiftUI

struct UpdatesSettings: View {
    @State private var updater = DesktopUpdateController.shared
    @State private var confirmingUpdate = false
    @State private var confirmingRestart = false
    var body: some View {
        Form {
            Section {
                LabeledContent(updateText("Installed version"), value: updater.installedVersion)
                if let latest = updater.latestVersion { LabeledContent(updateText("Latest version"), value: latest) }
                Text(channelDescription).foregroundStyle(.secondary)
                if !updater.ready || updater.busy { ProgressView().controlSize(.small) }
                Button(updateText("Check for Updates…")) { updater.check() }
                    .disabled(!updater.canCheck || !updater.supportsUpdates).settingsTarget("updates.check")
                Toggle(updateText("Check automatically"), isOn: Binding(get: { updater.automatic }, set: { updater.automatic = $0 }))
                    .disabled(!updater.supportsUpdates || updater.busy).settingsTarget("updates.automatic")
                Text(updateText("Off by default. Checks only while Momentum is running. Updates require your approval."))
                    .font(.caption).foregroundStyle(.secondary)
                if updater.channel == .homebrew, updater.updateAvailable, !updater.restartAvailable {
                    Button(updateText("Update with Homebrew…")) { confirmingUpdate = true }.disabled(updater.busy)
                }
                if updater.restartAvailable {
                    Button(updateText("Restart Momentum…")) { confirmingRestart = true }.disabled(updater.busy || !updater.canRestart).settingsTarget("updates.restart")
                }
                if updater.busy && !updater.installing && !updater.restarting { Button(updateText("Cancel")) { updater.cancel() } }
                if !updater.message.isEmpty { Text(updater.message).textSelection(.enabled).accessibilityAddTraits(.updatesFrequently) }
                Link(updateText("Release notes"), destination: URL(string: "https://github.com/dan-hart/momentum/releases")!)
            }
        }
        .formStyle(.grouped)
        .task { updater.start() }
        .confirmationDialog(updateText("Update Momentum with Homebrew?"), isPresented: $confirmingUpdate) {
            Button(updateText("Update")) { updater.install() }
        } message: { Text(updateText("Only the Momentum cask will be upgraded. You can restart after the update finishes.")) }
        .confirmationDialog(updateText("Restart Momentum now?"), isPresented: $confirmingRestart) {
            Button(updateText("Restart")) { updater.restart() }
        } message: { Text(updateText("Momentum will close and reopen using the installed update.")) }
    }
    private var channelDescription: String {
        switch updater.channel {
        case .source: updateText("Development and source builds are updated by rebuilding. This copy won’t replace itself.")
        case .homebrew: updateText("Installed with Homebrew. Updates keep your Homebrew installation in sync.")
        case .direct: updateText("Direct download. Momentum verifies signed updates before installing them.")
        case .unconfigured:
            updater.ownershipUncertain
                ? updateText("Momentum couldn’t verify its Homebrew installation. Repair Homebrew or update the Momentum cask in Terminal.")
                : updateText("Signed updates aren’t configured for this copy. Download the latest release to update.")
        }
    }
}
