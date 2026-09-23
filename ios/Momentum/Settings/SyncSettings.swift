// SPDX-License-Identifier: GPL-3.0-or-later
import struct MomentumCore.NearbyDevice
import MomentumKit
import MomentumMobile
import SwiftUI

struct SyncSettings: View {
    @Bindable var state: NextcloudSyncState
    @AccessibilityFocusState private var failureFocused: Bool
    @AccessibilityFocusState private var savedFocused: Bool

    var body: some View {
        Form {
            if !state.allowed {
                Section {
                    Label("Sync is disabled in preview and test mode.", systemImage: "network.slash")
                        .fixedSize(horizontal: false, vertical: true)
                }
            }
            Section {
                if state.allowed {
                    NavigationLink { SyncProviderSettings(state: state) } label: {
                        SettingsLabel("Sync Provider", symbol: "network", subtitle: LocalizedStringKey(state.provider.label))
                    }
                    .accessibilityIdentifier("sync-provider")
                } else {
                    LabeledContent("Sync Provider") { Text("Off").foregroundStyle(Color.primary) }
                        .accessibilityIdentifier("sync-provider-readonly")
                }
            } footer: {
                Text("Sync is optional. Turning it off keeps your tasks and saved connection on this device.")
                    .foregroundStyle(AccentTheme.secondaryText)
            }
            if state.provider == .nextcloud {
                status
                if state.loading { ProgressView("Loading Connection…") }
                if state.loaded { connection }
            } else if state.provider == .libresync {
                libreSync
            }
            if state.provider != .off, state.loaded {
                connectionTest
            }
            if let failure = state.failure {
                Section {
                    Label("Sync Needs Attention", systemImage: "exclamationmark.arrow.trianglehead.2.clockwise.rotate.90")
                        .font(.headline)
                        .accessibilityFocused($failureFocused)
                        .accessibilityIdentifier("sync-error")
                    Text(failure.explanation).fixedSize(horizontal: false, vertical: true)
                    Button {
                        Task {
                            switch failure {
                            case .credentialsRead: await state.load()
                            case .credentialsWrite: await state.save()
                            default: await state.syncNow()
                            }
                        }
                    } label: { SettingsLabel("Try Again", symbol: "arrow.clockwise") }
                    .disabled(state.loading || state.saving || state.isSyncing ||
                              (failure != .credentialsRead && failure != .credentialsWrite && !state.canSync))
                    .accessibilityIdentifier("sync-retry")
                }
            }
        }
        .navigationTitle("Sync")
        .navigationBarTitleDisplayMode(.inline)
        .momentumNavigationCanvas()
        .onChange(of: state.failure) { _, failure in if failure != nil { failureFocused = true } }
    }

    private var connectionTest: some View {
        Section {
            Button {
                Task { await state.testConnection() }
            } label: {
                SettingsLabel("Test Connection", symbol: "network.badge.shield.half.filled")
                    .frame(minHeight: 44)
            }
            .disabled(!state.canTestConnection)
            .accessibilityIdentifier("sync-test-connection")

            switch state.connectionTest {
            case .idle:
                EmptyView()
            case .testing:
                ProgressView("Testing Connection…")
                    .accessibilityIdentifier("sync-test-progress")
            case .success:
                Label("Connection Successful", systemImage: "checkmark.circle.fill")
                    .foregroundStyle(.green)
                    .accessibilityIdentifier("sync-test-success")
            case .failure(let message):
                AccessibleErrorMessage(String(localized: "Connection test failed: \(message)"))
                    .accessibilityIdentifier("sync-test-failure")
            }
        } footer: {
            Text(state.provider == .libresync
                 ? "Checks the link to a paired device without changing your tasks."
                 : "Checks the saved server and credentials without uploading or downloading tasks.")
                .foregroundStyle(AccentTheme.secondaryText)
        }
    }

    @ViewBuilder private var libreSync: some View {
        if let nearby = state.nearby {
            Section {
                if nearby.phase == .starting || nearby.phase == .stopping {
                    ProgressView(nearby.phase == .starting ? "Starting LibreSync…" : "Stopping LibreSync…")
                }
                if let status = nearby.status {
                    if status.lastNearbyMs > 0 {
                        LabeledContent("Last Successful Sync") {
                            Text(Date(timeIntervalSince1970: Double(status.lastNearbyMs) / 1000),
                                 format: .dateTime.month().day().hour().minute())
                        }
                    } else { LabeledContent("Last Successful Sync") { Text("Never") } }
                    LabeledContent("Linked Devices", value: status.linkedDevices.formatted())
                    LabeledContent("Pending Changes", value: status.pendingOps.formatted())
                }
                Button { Task { await state.syncNow() } } label: {
                    SettingsLabel("Sync Now", symbol: "arrow.trianglehead.2.clockwise.rotate.90")
                        .frame(minHeight: 44)
                }
                .disabled(!state.canSync)
                .accessibilityIdentifier("sync-now")
                NavigationLink { LibreSyncSettings(state: nearby) } label: {
                    SettingsLabel("Nearby Devices", symbol: "antenna.radiowaves.left.and.right")
                        .frame(minHeight: 44)
                }
                .disabled(nearby.phase != .running)
                .accessibilityIdentifier("libresync-devices")
            } footer: {
                Text("LibreSync exchanges encrypted task data directly with linked devices while Momentum is open. No Momentum account is required.")
                    .foregroundStyle(AccentTheme.secondaryText)
            }
            if nearby.failure != nil || nearby.transportError != nil {
                Section {
                    AccessibleErrorMessage("LibreSync needs attention. Check the linked device and try again.")
                    Button("Try Again", systemImage: "arrow.clockwise") { nearby.retry() }
                        .frame(minHeight: 44)
                }
            }
        } else {
            Section { ProgressView("Starting LibreSync…") }
        }
    }

    private var status: some View {
        Section {
            if state.lowPowerMode {
                Label("Automatic sync is paused in Low Power Mode.", systemImage: "battery.25percent")
                    .fixedSize(horizontal: false, vertical: true)
                    .accessibilityIdentifier("sync-low-power")
            }
            if state.isSyncing {
                ProgressView(state.isStopping ? "Stopping Sync…" : "Syncing…")
                    .accessibilityIdentifier("sync-progress")
            }
            if let status = state.status {
                if status.lastNextcloudMs > 0 {
                    LabeledContent("Last Successful Sync") {
                        Text(Date(timeIntervalSince1970: Double(status.lastNextcloudMs) / 1000), format: .dateTime.month().day().hour().minute())
                    }
                } else { LabeledContent("Last Successful Sync") { Text("Never") } }
                LabeledContent("Pending Changes", value: status.pendingOps.formatted())
            }
            Button { Task { await state.syncNow() } } label: {
                SettingsLabel("Sync Now", symbol: "arrow.trianglehead.2.clockwise.rotate.90")
                    .frame(minHeight: 44)
            }
            .disabled(!state.canSync)
            .accessibilityIdentifier("sync-now")
        } footer: {
            Text(state.lowPowerMode
                 ? "Sync Now remains available. Automatic sync resumes when Low Power Mode turns off."
                 : "Open Momentum to sync. Exchanges stop when you leave the app; unsynced changes stay on this device.")
                .foregroundStyle(AccentTheme.secondaryText)
        }
    }

    private func field<Content: View>(_ title: LocalizedStringKey, @ViewBuilder content: () -> Content) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(title).font(.caption).foregroundStyle(AccentTheme.secondaryText)
                .fixedSize(horizontal: false, vertical: true)
            content()
        }.padding(.vertical, 4)
    }

    private var connection: some View {
        Group {
            Section {
                field("Server URL") {
                    TextField("Server URL", text: $state.draft.serverURL)
                        .keyboardType(.URL).textContentType(.URL)
                        .accessibilityIdentifier("sync-server")
                }
                field("Username") {
                    TextField("Username", text: $state.draft.userName)
                        .textContentType(.username).accessibilityIdentifier("sync-username")
                }
                field("App Password") {
                    SecureField("App Password", text: $state.draft.password)
                        .textContentType(.password).accessibilityIdentifier("sync-password")
                }
                field("Sync Folder") {
                    TextField("Sync Folder", text: $state.draft.folder)
                        .accessibilityIdentifier("sync-folder")
                }
            } header: { Text("Connection").foregroundStyle(Color.primary) }
            footer: {
                Text("Use an app password from your Nextcloud security settings. Connection details are saved securely on this device.")
                    .foregroundStyle(AccentTheme.secondaryText)
            }
            .textInputAutocapitalization(.never)
            .autocorrectionDisabled()

            Section {
                Toggle("Sync Automatically", isOn: $state.draft.automatic)
                Toggle("Compress Sync File", isOn: $state.draft.compress)
                field("Encryption Password (Optional)") {
                    SecureField("Encryption Password (Optional)", text: $state.draft.encryptionPassword)
                        .textInputAutocapitalization(.never).autocorrectionDisabled()
                }
            } header: { Text("Options").foregroundStyle(Color.primary) }
            footer: {
                VStack(alignment: .leading, spacing: 12) {
                    Text("Use the same encryption password on every device. Automatic sync combines nearby edits and checks for changes while the app is open.")
                    if !state.hasUnsavedChanges {
                        Text("Connection Saved")
                            .fixedSize(horizontal: false, vertical: true)
                            .accessibilityIdentifier("sync-saved")
                            .accessibilityFocused($savedFocused)
                    }
                }
                .foregroundStyle(AccentTheme.secondaryText)
            }
            if state.hasUnsavedChanges || state.saving {
                Section {
                    if state.failure == .configuration, let issue = state.configurationIssue {
                        NextcloudConfigurationError(issue: issue)
                    }
                    Button {
                        Task { if await state.save() { savedFocused = true } }
                    } label: {
                        SettingsLabel("Save Connection", symbol: "checkmark.circle.fill")
                            .frame(minHeight: 44)
                    }
                    .accessibilityIdentifier("sync-save")
                    .disabled(state.saving)
                    if state.saving { ProgressView("Saving Connection…") }
                    Text("Save your changes before syncing.").foregroundStyle(AccentTheme.secondaryText)
                }
            }
        }
        .disabled(state.saving)
    }
}

extension NextcloudSyncState.Failure {
    var explanation: LocalizedStringKey {
        switch self {
        case .credentialsRead: "The saved connection couldn’t be read. Unlock your device and try again. Your saved details have not been replaced."
        case .credentialsWrite: "The connection couldn’t be saved. Your changes are still here. Unlock your device and try again."
        case .configuration: "Correct the connection field described above, then save again."
        case .connection: "Check your server address, app password, encryption password, and the sync file’s compatibility, then try again."
        case .network: "Sync couldn’t finish. Check your connection and server, then try again. Your local changes are safe."
        case .busy: "Another exchange is finishing. Wait a moment, then try again."
        }
    }
}

struct NextcloudConfigurationError: View {
    let issue: NextcloudConnectionIssue

    var body: some View {
        AccessibleErrorMessage(issue.explanation)
        .accessibilityIdentifier("sync-configuration-error")
    }
}

private extension NextcloudConnectionIssue {
    var explanation: LocalizedStringKey {
        switch self {
        case .serverURLRequired: "Enter your Nextcloud server URL."
        case .serverURLInvalid: "Enter a complete server URL without login details, a query, or a fragment."
        case .secureServerRequired: "Use HTTPS. HTTP is allowed only for local test servers."
        case .userNameRequired: "Enter your Nextcloud username."
        case .passwordRequired: "Enter a Nextcloud app password."
        case .folderRequired: "Enter the folder where Momentum should store its sync file."
        }
    }
}

private struct SyncProviderSettings: View {
    @Bindable var state: NextcloudSyncState
    @Environment(\.dismiss) private var dismiss
    var body: some View {
        List {
            ForEach([SyncMethod.off, .nextcloud, .libresync]) { provider in
                Button {
                    Task { await state.select(provider); dismiss() }
                } label: {
                    HStack {
                        Text(provider.label).fixedSize(horizontal: false, vertical: true)
                        Spacer()
                        if state.provider == provider {
                            Image(systemName: "checkmark").accessibilityHidden(true)
                        }
                    }
                    .foregroundStyle(Color.primary)
                    .frame(minHeight: 44)
                }
                .accessibilityAddTraits(state.provider == provider ? .isSelected : [])
                .accessibilityIdentifier("sync-provider-\(provider.rawValue)")
                .disabled(!state.allowed || state.isStopping || state.nearby?.phase == .stopping)
            }
            if state.isStopping { ProgressView("Stopping Sync…") }
        }
        .navigationTitle("Sync Provider")
        .navigationBarTitleDisplayMode(.inline)
    }
}

/// Compact list footer; its minute cadence runs only while the list is visible.
struct SyncStatusLink: View {
    var state: NextcloudSyncState

    var body: some View {
        if state.provider != .off {
            NavigationLink { SyncSettings(state: state) } label: {
                TimelineView(.periodic(from: .now, by: 60)) { context in
                    HStack(spacing: 7) {
                        Image(systemName: symbol).accessibilityHidden(true)
                        Text(statusText(now: context.date))
                            .lineLimit(nil)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                    .font(.footnote)
                    .foregroundStyle(needsAttention ? Color.primary : AccentTheme.secondaryText)
                    .frame(maxWidth: .infinity, alignment: .center)
                }
                .frame(minHeight: 44)
                .accessibilityElement(children: .combine)
            }
            .accessibilityIdentifier("sync-status")
        }
    }

    private var syncing: Bool {
        state.provider == .libresync ? state.nearby?.isSyncing == true : state.isSyncing
    }

    private var needsAttention: Bool {
        state.provider == .libresync
            ? state.nearby?.failure != nil || state.nearby?.transportError != nil
            : state.failure != nil
    }

    private var symbol: String {
        if needsAttention { return "exclamationmark.circle" }
        if syncing { return "arrow.trianglehead.2.clockwise.rotate.90" }
        return state.provider == .libresync ? "antenna.radiowaves.left.and.right" : "checkmark.circle"
    }

    private func statusText(now: Date) -> String {
        if syncing { return state.isStopping ? String(localized: "Stopping sync…") : String(localized: "Syncing…") }
        if needsAttention { return String(localized: "Sync needs attention") }
        guard let status = state.status else { return String(localized: "Sync status unavailable") }
        let milliseconds = state.provider == .libresync ? status.lastNearbyMs : status.lastNextcloudMs
        guard milliseconds > 0 else { return String(localized: "Not synced yet") }
        let date = Date(timeIntervalSince1970: Double(milliseconds) / 1000)
        let seconds = max(0, now.timeIntervalSince(date))
        if seconds < 60 { return String(localized: "Last synced just now") }
        let minutes = max(1, Int(seconds / 60))
        if minutes == 1 { return String(localized: "Last synced 1 minute ago") }
        if minutes < 60 { return String(localized: "Last synced \(minutes) minutes ago") }
        return String(localized: "Last synced \(date.formatted(date: .abbreviated, time: .shortened))")
    }
}

struct LibreSyncSettings: View {
    @Bindable var state: NearbyLifecycle
    let automaticallyPairs: Bool
    @State private var linking: NearbyDevice?
    @State private var unlinking: NearbyDevice?

    init(state: NearbyLifecycle, automaticallyPairs: Bool = true) {
        self.state = state
        self.automaticallyPairs = automaticallyPairs
    }

    var body: some View {
        LibreSyncDeviceList(state: state, linking: $linking, unlinking: $unlinking)
            .navigationTitle("Nearby Devices")
            .navigationBarTitleDisplayMode(.inline)
            .momentumNavigationCanvas()
            .task { if automaticallyPairs { _ = await state.beginPairing() } }
            .onDisappear { if automaticallyPairs { Task { await state.endPairing() } } }
            .sheet(isPresented: Binding(get: { linking != nil }, set: { if !$0 { linking = nil } })) {
                if let device = linking {
                    LibreSyncLinkSheet(state: state, device: device) { linking = nil }
                }
            }
            .confirmationDialog("Unlink this device?", isPresented: Binding(
                get: { unlinking != nil }, set: { if !$0 { unlinking = nil } }
            ), presenting: unlinking) { device in
                Button("Unlink \(device.name)", role: .destructive) {
                    Task { await state.unlink(id: device.deviceId); unlinking = nil }
                }
                Button("Cancel", role: .cancel) { unlinking = nil }
            } message: { device in
                Text("\(device.name) will need a new pairing code before it can sync again.")
            }
    }
}

struct LibreSyncDeviceList: View {
    @Bindable var state: NearbyLifecycle
    @Binding var linking: NearbyDevice?
    @Binding var unlinking: NearbyDevice?

    var body: some View {
        Form {
            Section("This Device") {
                LabeledContent(state.info?.deviceName ?? String(localized: "This Device")) {
                    if let port = state.info?.port {
                        Text("Listening on port \(port)")
                            .foregroundStyle(AccentTheme.secondaryText)
                    } else {
                        Text("Starting…").foregroundStyle(AccentTheme.secondaryText)
                    }
                }
                if let code = state.pairingCode {
                    LabeledContent("Pairing Code") {
                        Text(formatted(code))
                            .font(.title2.monospacedDigit())
                            .textSelection(.enabled)
                            .accessibilityLabel("Pairing code \(code.map(String.init).joined(separator: " "))")
                    }
                }
            }

            Section {
                if state.discovered.isEmpty {
                    Label("Searching for nearby devices…", systemImage: "antenna.radiowaves.left.and.right")
                        .foregroundStyle(AccentTheme.secondaryText)
                        .fixedSize(horizontal: false, vertical: true)
                }
                ForEach(state.discovered, id: \.deviceId) { device in
                    Button {
                        linking = device
                    } label: {
                        HStack(alignment: .firstTextBaseline, spacing: 12) {
                            Image(systemName: "iphone.gen3")
                                .foregroundStyle(.tint)
                                .frame(width: 28)
                                .accessibilityHidden(true)
                            VStack(alignment: .leading, spacing: 3) {
                                Text(verbatim: device.name).foregroundStyle(Color.primary)
                                if let address = device.address {
                                    Text(verbatim: address)
                                        .font(.footnote)
                                        .foregroundStyle(AccentTheme.secondaryText)
                                }
                            }
                            .fixedSize(horizontal: false, vertical: true)
                        }
                            .frame(minHeight: 44)
                    }
                    .disabled(device.address == nil)
                    .accessibilityLabel("Link with \(device.name)")
                }
            } header: { Text("Nearby Devices") }
            footer: {
                Text("Open this screen on another Momentum device, then enter its six-digit code here.")
                    .foregroundStyle(AccentTheme.secondaryText)
            }

            Section("Linked Devices") {
                if state.linked.isEmpty {
                    Text("No linked devices yet.").foregroundStyle(AccentTheme.secondaryText)
                }
                ForEach(state.linked, id: \.deviceId) { device in
                    HStack(alignment: .firstTextBaseline) {
                        VStack(alignment: .leading, spacing: 4) {
                            Text(device.name)
                            if let lastSeen = device.lastSeenMs {
                                Text(Date(timeIntervalSince1970: Double(lastSeen) / 1000),
                                     format: .dateTime.month().day().hour().minute())
                                    .font(.caption)
                                    .foregroundStyle(AccentTheme.secondaryText)
                            } else {
                                Text("Not synced yet")
                                    .font(.caption)
                                    .foregroundStyle(AccentTheme.secondaryText)
                            }
                        }
                        .fixedSize(horizontal: false, vertical: true)
                        Spacer()
                        Button("Unlink", systemImage: "link.badge.minus", role: .destructive) {
                            unlinking = device
                        }
                        .labelStyle(.iconOnly)
                        .frame(minWidth: 44, minHeight: 44)
                        .accessibilityLabel("Unlink \(device.name)")
                    }
                }
            }
        }
    }

    private func formatted(_ code: String) -> String {
        guard code.count > 3 else { return code }
        return "\(code.prefix(3)) \(code.dropFirst(3))"
    }
}

private struct LibreSyncLinkSheet: View {
    @Bindable var state: NearbyLifecycle
    let device: NearbyDevice
    let dismiss: () -> Void
    @State private var code = ""
    @State private var saving = false
    @State private var failed = false

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    Text("Enter the six-digit pairing code shown on \(device.name).")
                        .fixedSize(horizontal: false, vertical: true)
                    TextField("Pairing Code", text: $code)
                        .keyboardType(.numberPad)
                        .textContentType(.oneTimeCode)
                        .multilineTextAlignment(.center)
                        .font(.title2.monospacedDigit())
                        .accessibilityIdentifier("libresync-pairing-code")
                    if failed {
                        AccessibleErrorMessage("Pairing failed. Check the code and try again.")
                            .accessibilityIdentifier("libresync-pairing-error")
                    }
                }
            }
            .navigationTitle("Link Device")
            .navigationBarTitleDisplayMode(.inline)
            .momentumNavigationCanvas()
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel", systemImage: "xmark", action: dismiss)
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Link", systemImage: "link") {
                        guard let address = device.address else { return }
                        saving = true
                        failed = false
                        Task {
                            do {
                                try await state.link(address: address,
                                                     code: code.filter(\.isNumber))
                                dismiss()
                            } catch {
                                failed = true
                                saving = false
                            }
                        }
                    }
                    .disabled(code.filter(\.isNumber).count != 6 || saving || device.address == nil)
                }
            }
            .disabled(saving)
        }
        .presentationDetents([.medium, .large])
    }
}
