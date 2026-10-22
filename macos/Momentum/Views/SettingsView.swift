// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// Settings, in the tabs a Mac app uses. The same preferences as the GNOME app, under the
// same keys, with secrets going to the Keychain rather than into the defaults.
import MomentumCore
import MomentumKit
import SwiftUI

struct SettingsView: SwiftUI.View {
    @Environment(AppState.self) private var state

    var body: some SwiftUI.View {
        TabView {
            GeneralSettings()
                .tabItem { Label(String(localized: "General"), systemImage: "gearshape") }
            FontSettings()
                .tabItem { Label(String(localized: "Fonts"), systemImage: "textformat") }
            SyncSettings()
                .tabItem { Label(String(localized: "Sync"), systemImage: "arrow.triangle.2.circlepath") }
            RemindersSettings()
                .tabItem { Label(String(localized: "Apple Reminders"), systemImage: "list.bullet") }
            BackupSettings()
                .tabItem { Label(String(localized: "Backup"), systemImage: "externaldrive") }
        }
        .frame(minWidth: 520, idealWidth: 600, minHeight: 460, idealHeight: 600)
        .environment(state)
    }
}

private struct GeneralSettings: SwiftUI.View {
    @Environment(AppState.self) private var state
    @Environment(\.locale) private var locale
    @AppStorage(PrefKey.showDayPeriodTasksInToday) private var showDayPeriods = true
    @AppStorage(PrefKey.autoArchive) private var autoArchive = false
    @AppStorage(PrefKey.completionSound) private var completionSound = false
    @AppStorage(PrefKey.completionSoundChoice) private var completionSoundChoiceRaw = CompletionSoundChoice.defaultChoice.rawValue
    @AppStorage(PrefKey.colorfulLabels) private var colorful = true
    @AppStorage(PrefKey.runInBackground) private var background = false
    @AppStorage(PrefKey.showInMenuBar) private var menuBar = false
    @AppStorage(PrefKey.dockBadgeMode) private var dockBadgeMode = DockBadgeMode.dueToday.rawValue
    @AppStorage(PrefKey.modifierKey) private var modifier = "command"
    @AppStorage(PrefKey.morningSummaryEnabled) private var morningSummary = false
    @AppStorage(PrefKey.morningSummaryHour) private var morningHour = 8
    @AppStorage(PrefKey.morningSummaryMinute) private var morningMinute = 0

    private var completionSoundChoice: Binding<CompletionSoundChoice> {
        Binding {
            CompletionSoundChoice.resolve(completionSoundChoiceRaw)
        } set: { choice in
            completionSoundChoiceRaw = choice.rawValue
        }
    }

    private var summaryTime: Binding<Date> {
        Binding {
            // A fixed ordinary day keeps the picker independent of today's DST gap.
            Calendar.current.date(from: DateComponents(year: 2001, month: 1, day: 15,
                hour: (0...23).contains(morningHour) ? morningHour : 8,
                minute: (0...59).contains(morningMinute) ? morningMinute : 0)) ?? .now
        } set: { date in
            morningHour = Calendar.current.component(.hour, from: date)
            morningMinute = Calendar.current.component(.minute, from: date)
        }
    }

    var body: some SwiftUI.View {
        Form {
            Section(String(localized: "Tasks")) {
                Toggle(String(localized: "Show Morning and Evening tasks in Today"), isOn: $showDayPeriods)
                Toggle(String(localized: "Archive completed tasks immediately"), isOn: $autoArchive)
                Text(String(localized: "A task you complete goes straight to the archive. Undo brings it back."))
                    .appFont(.caption).foregroundStyle(.secondary)
                Toggle(String(localized: "Play a sound when a task is completed"), isOn: $completionSound)
                Picker(String(localized: "Sound"), selection: completionSoundChoice) {
                    ForEach(CompletionSoundChoice.allCases) { choice in
                        Text(choice.label(locale: locale)).tag(choice)
                    }
                }
                .disabled(!completionSound)
                Button(String(localized: "Preview sound")) {
                    guard completionSound else { return }
                    CompletionSound.shared.playCompletion(completionSoundChoice.wrappedValue)
                }
                .disabled(!completionSound)
                Text(String(localized: "Choose a short sound for completed tasks. Preview plays the selected sound."))
                    .appFont(.caption).foregroundStyle(.secondary)
            }
            Section(String(localized: "Notifications")) {
                Toggle(String(localized: "Morning summary"), isOn: $morningSummary)
                DatePicker(String(localized: "Summary time"), selection: summaryTime,
                           displayedComponents: .hourAndMinute)
                    .disabled(!morningSummary)
                Text(String(localized: "Summarize today's tasks once a day at this local time, or when Momentum next opens. Task reminders are separate."))
                    .appFont(.caption).foregroundStyle(.secondary)
                Text(String(localized: "Off by default. Momentum must be running to send notifications."))
                    .appFont(.caption).foregroundStyle(.secondary)
            }
            Section(String(localized: "Appearance")) {
                Toggle(String(localized: "Color-code projects and tags"), isOn: $colorful)
            }
            Section {
                Picker(String(localized: "Shortcut modifier"), selection: $modifier) {
                    ForEach(ModifierKey.allCases) { m in
                        Text("\(m.label) (\(m.symbol))").tag(m.rawValue)
                    }
                }
                Text(String(localized: "Momentum's own shortcuts use this key. System shortcuts keep theirs."))
                    .appFont(.caption).foregroundStyle(.secondary)
                Text((ModifierKey(rawValue: modifier) ?? .command).shortcutWarning)
                    .appFont(.caption).foregroundStyle(.secondary)
            } header: {
                Text(String(localized: "Keyboard"))
            }
            Section(String(localized: "Desktop")) {
                Picker(String(localized: "Dock badge"), selection: $dockBadgeMode) {
                    ForEach(DockBadgeMode.allCases) { mode in
                        Text(mode.label).tag(mode.rawValue)
                    }
                }
                Text(String(localized: "Counts unfinished tasks. Subtasks are included with their parent. The badge is hidden when the count is zero."))
                    .appFont(.caption).foregroundStyle(.secondary)
                Toggle(String(localized: "Keep running in the background"), isOn: $background)
                Text(String(localized: "Reminders and sync keep going when the window is closed, and Momentum starts at login."))
                    .appFont(.caption).foregroundStyle(.secondary)
                Toggle(String(localized: "Show in the menu bar"), isOn: $menuBar)
            }
        }
        .formStyle(.grouped)
        .onChange(of: background) { _, _ in
            (NSApp.delegate as? AppDelegate)?.updateLoginItem()
        }
    }
}

private struct SyncSettings: SwiftUI.View {
    @Environment(AppState.self) private var state
    @AppStorage(PrefKey.syncMethod) private var method = SyncMethod.off.rawValue
    @AppStorage(PrefKey.nextcloudServer) private var server = ""
    @AppStorage(PrefKey.nextcloudUser) private var user = ""
    @AppStorage(PrefKey.nextcloudFolder) private var folder = "momentum"
    @AppStorage(PrefKey.autoSync) private var autoSync = true
    @AppStorage(PrefKey.backgroundSync) private var backgroundSync = true
    @AppStorage(PrefKey.compress) private var compress = false
    @State private var password = ""
    @State private var encryption = ""
    @State private var confirmingFolderCopy = false

    var body: some SwiftUI.View {
        Form {
            Section {
                Picker(String(localized: "Sync method"), selection: $method) {
                    ForEach(SyncMethod.allCases) { service in
                        Text(service.label).tag(service.rawValue)
                    }
                }
                .disabled(state.syncInProgress)
            } footer: {
                Text(String(localized: "Choose one service to keep your tasks in sync. Switching services keeps your tasks and saved connections."))
            }
            Section(String(localized: "App settings")) {
                Toggle(String(localized: "Sync app settings"), isOn: Binding(
                    get: { state.appSettingsSync.enabled }, set: { state.setAppSettingsSync($0) }))
                    .disabled(state.isDemo)
                Text(String(localized: "Off by default. Uses your selected sync provider for task display, grouping, sorting, archive and sound preferences. Connections, credentials, permissions and device controls stay local."))
                    .font(.caption).foregroundStyle(.secondary)
                if state.appSettingsSync.enabled, !state.appSettingsSync.bootstrapRequired {
                    if state.appSettingsSync.pendingKeys > 0 {
                        Label(String(localized: "App settings waiting to sync"), systemImage: "clock")
                    } else if state.appSettingsSync.lastSuccessMs > 0, state.appSettingsSync.error == nil {
                        Label(String(localized: "App settings are up to date"), systemImage: "checkmark.circle")
                    }
                    if state.appSettingsSync.lastSuccessMs > 0 {
                        LabeledContent(String(localized: "Last settings sync")) {
                            Text(Date(timeIntervalSince1970: Double(state.appSettingsSync.lastSuccessMs) / 1_000), style: .relative)
                        }
                    }
                }
                if state.appSettingsSync.bootstrapRequired, let reviewID = state.appSettingsSync.reviewId,
                   let remote = state.appSettingsSync.remoteSettingsJson {
                    Text(String(localized: "This device and your other devices have saved settings. Choose which settings to start with."))
                    DisclosureGroup(String(localized: "This device")) {
                        ForEach(PortableAppSettings.summary((try? PortableAppSettings.encode(state.defaults)) ?? "")) { item in
                            LabeledContent(item.label, value: item.value)
                        }
                    }
                    DisclosureGroup(String(localized: "Synced settings")) {
                        ForEach(PortableAppSettings.summary(remote)) { item in LabeledContent(item.label, value: item.value) }
                    }
                    Button(String(localized: "Use this device’s settings")) { state.chooseAppSettingsBootstrap(.useLocal, reviewID: reviewID) }
                        .disabled(state.canRetryAppSettings)
                    Button(String(localized: "Use synced settings")) { state.chooseAppSettingsBootstrap(.useRemote, reviewID: reviewID) }
                        .disabled(state.canRetryAppSettings)
                }
                if let error = state.appSettingsError {
                    Text(error).foregroundStyle(.red).textSelection(.enabled)
                    if state.canRetryAppSettings {
                        Button(String(localized: "Retry")) { state.retryAppSettings() }
                    }
                }
                if let error = state.appSettingsSync.error {
                    Text(error).foregroundStyle(.red).textSelection(.enabled)
                    Button(String(localized: "Retry")) { state.sync() }
                }
            }
            if state.isDemo {
                Text(String(localized: "Sync is unavailable in preview mode. Open Momentum normally to sync your tasks."))
            } else if method == SyncMethod.off.rawValue {
                Text(String(localized: "Your tasks are stored on this Mac. Choose a service above to sync them with your other devices."))
                    .foregroundStyle(.secondary)
            }
            if method == SyncMethod.nextcloud.rawValue {
                Section(String(localized: "Connection")) {
                    TextField(String(localized: "Server URL"), text: $server, prompt: Text(verbatim: "https://cloud.example.com"))
                    TextField(String(localized: "Username"), text: $user)
                    SecureField(String(localized: "App password"), text: $password)
                    TextField(String(localized: "Folder"), text: $folder)
                    SecureField(String(localized: "Encryption password"), text: $encryption)
                }
                .disabled(state.syncInProgress)

                if folder != "momentum" || state.folderCopyMessage != nil {
                    Section {
                        if state.folderCopyInProgress {
                            ProgressView(String(localized: "Copying and verifying files…"))
                            Button(String(localized: "Cancel")) { state.cancelFolderCopy() }
                        } else if folder != "momentum" {
                            Button(String(localized: "Copy to Momentum folder…")) { confirmingFolderCopy = true }
                                .disabled(!state.canCopyFolder)
                        }
                        if let message = state.folderCopyMessage { Text(message).textSelection(.enabled) }
                        Text(String(localized: "The original folder stays intact. The saved folder changes only after every copied file is verified."))
                            .font(.caption).foregroundStyle(.secondary)
                    }
                }

                Section(String(localized: "When")) {
                    Toggle(String(localized: "Sync automatically"), isOn: $autoSync)
                    backgroundSyncToggle
                    Toggle(String(localized: "Compress the sync file"), isOn: $compress)
                }
            } else if method == SyncMethod.libresync.rawValue {
                Section("LibreSync") {
                    Text(String(localized: "Your devices sync directly over the local network, end-to-end encrypted, with no server. Link them once with a six-digit code."))
                    LabeledContent(String(localized: "Linked devices"), value: "\(state.syncStatus.linkedDevices)")
                    Button(String(localized: "Manage Devices…")) { state.showDevices() }
                }
                Section(String(localized: "When")) {
                    backgroundSyncToggle
                }
            }
            if state.syncAvailable {
                Section(String(localized: "Status")) {
                    SyncStatusBar()
                }
            }
        }
        .formStyle(.grouped)
        .disabled(state.isDemo)
        .onAppear {
            guard !state.isDemo else { return }
            password = state.keychain.get(Keychain.nextcloud) ?? ""
            encryption = state.keychain.get(Keychain.encryption) ?? ""
        }
        .onChange(of: method) { _, _ in state.preferencesChanged() }
        .onChange(of: backgroundSync) { _, _ in state.preferencesChanged() }
        .onChange(of: password) { _, new in state.keychain.set(Keychain.nextcloud, new) }
        .onChange(of: encryption) { _, new in state.keychain.set(Keychain.encryption, new) }
        .onDisappear { state.cancelFolderCopy() }
        .alert(String(localized: "Copy to Momentum folder?"), isPresented: $confirmingFolderCopy) {
            Button(String(localized: "Cancel"), role: .cancel) {}
            Button(String(localized: "Copy and switch")) { Task { await state.copyToMomentumFolder() } }
        } message: {
            Text(String(localized: "Copy and verify your sync files in the momentum folder, then switch this device to it. The original folder stays intact. Choose the new folder on your other devices after the copy succeeds."))
        }
    }
}

private extension SyncSettings {
    /// Automatic sync keeps running with the window closed only while this is on; Sync
    /// Now always works. Pair it with General › Keep running in the background.
    @ViewBuilder var backgroundSyncToggle: some SwiftUI.View {
        Toggle(String(localized: "Sync in the background"), isOn: $backgroundSync)
        Text(String(localized: "Keep syncing while Momentum runs without a window. Turn this off to sync only while the window is open; Sync Now always works. Needs “Keep running in the background” in General."))
            .appFont(.caption).foregroundStyle(.secondary)
    }
}

private struct BackupSettings: SwiftUI.View {
    @Environment(AppState.self) private var state
    @State private var includeSettings = false
    @State private var restoreSettings = false

    var body: some SwiftUI.View {
        Form {
            Section {
                Toggle(String(localized: "Restore included app settings"), isOn: $restoreSettings)
                Button(String(localized: "Import Backup…")) {
                    let panel = NSOpenPanel()
                    panel.allowedContentTypes = [.json]
                    if panel.runModal() == .OK, let url = panel.url { state.importBackup(url, restoreSettings: restoreSettings) }
                }
                Toggle(String(localized: "Include app settings"), isOn: $includeSettings)
                Button(String(localized: "Export Backup…")) {
                    let panel = NSSavePanel()
                    panel.allowedContentTypes = [.json]
                    panel.nameFieldStringValue = "\(today()).json"
                    if panel.runModal() == .OK, let url = panel.url { state.exportBackup(url, includeSettings: includeSettings) }
                }
            } footer: {
                Text(String(localized: "Importing replaces all tasks on this Mac and discards unsynced task changes. App settings are optional; credentials and device controls stay local."))
            }
            Section(String(localized: "App settings only")) {
                Button(String(localized: "Import app settings…")) {
                    let panel = NSOpenPanel()
                    panel.allowedContentTypes = [.json]
                    if panel.runModal() == .OK, let url = panel.url { state.importAppSettings(url) }
                }
                Button(String(localized: "Export app settings…")) {
                    let panel = NSSavePanel()
                    panel.allowedContentTypes = [.json]
                    panel.nameFieldStringValue = "momentum-settings.json"
                    if panel.runModal() == .OK, let url = panel.url { state.exportAppSettings(url) }
                }
                if let error = state.appSettingsError {
                    Text(error).foregroundStyle(.red)
                    if state.canRetryAppSettings {
                        Button(String(localized: "Retry")) { state.retryAppSettings() }
                    }
                }
            }
            Section(String(localized: "Data")) {
                LabeledContent(String(localized: "Folder"), value: state.dataDir.path)
                    .textSelection(.enabled)
                Button(String(localized: "Show in Finder")) {
                    NSWorkspace.shared.activateFileViewerSelecting([state.dataDir])
                }
            }
        }
        .formStyle(.grouped)
    }
}
