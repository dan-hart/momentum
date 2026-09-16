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
            BackupSettings()
                .tabItem { Label(String(localized: "Backup"), systemImage: "externaldrive") }
        }
        .frame(minWidth: 520, idealWidth: 600, minHeight: 460, idealHeight: 600)
        .environment(state)
    }
}

private struct GeneralSettings: SwiftUI.View {
    @Environment(AppState.self) private var state
    @AppStorage(PrefKey.autoArchive) private var autoArchive = false
    @AppStorage(PrefKey.colorfulLabels) private var colorful = true
    @AppStorage(PrefKey.runInBackground) private var background = false
    @AppStorage(PrefKey.showInMenuBar) private var menuBar = false
    @AppStorage(PrefKey.dockBadgeMode) private var dockBadgeMode = DockBadgeMode.dueToday.rawValue
    @AppStorage(PrefKey.modifierKey) private var modifier = "command"
    @AppStorage(PrefKey.morningSummaryEnabled) private var morningSummary = false
    @AppStorage(PrefKey.morningSummaryHour) private var morningHour = 8
    @AppStorage(PrefKey.morningSummaryMinute) private var morningMinute = 0

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
                Toggle(String(localized: "Archive completed tasks immediately"), isOn: $autoArchive)
                Text(String(localized: "A task you complete goes straight to the archive. Undo brings it back."))
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
    @AppStorage(PrefKey.nextcloudFolder) private var folder = "super-productivity"
    @AppStorage(PrefKey.autoSync) private var autoSync = true
    @AppStorage(PrefKey.compress) private var compress = false
    @State private var password = ""
    @State private var encryption = ""

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

                Section(String(localized: "When")) {
                    Toggle(String(localized: "Sync automatically"), isOn: $autoSync)
                    Toggle(String(localized: "Compress the sync file"), isOn: $compress)
                }
            } else if method == SyncMethod.libresync.rawValue {
                Section("LibreSync") {
                    Text(String(localized: "Your devices sync directly over the local network, end-to-end encrypted, with no server. Link them once with a six-digit code."))
                    LabeledContent(String(localized: "Linked devices"), value: "\(state.syncStatus.linkedDevices)")
                    Button(String(localized: "Manage Devices…")) { state.showDevices() }
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
        .onChange(of: password) { _, new in state.keychain.set(Keychain.nextcloud, new) }
        .onChange(of: encryption) { _, new in state.keychain.set(Keychain.encryption, new) }
    }
}

private struct BackupSettings: SwiftUI.View {
    @Environment(AppState.self) private var state

    var body: some SwiftUI.View {
        Form {
            Section {
                Button(String(localized: "Import Backup…")) {
                    let panel = NSOpenPanel()
                    panel.allowedContentTypes = [.json]
                    if panel.runModal() == .OK, let url = panel.url { state.importBackup(url) }
                }
                Button(String(localized: "Export Backup…")) {
                    let panel = NSSavePanel()
                    panel.allowedContentTypes = [.json]
                    panel.nameFieldStringValue = "\(today()).json"
                    if panel.runModal() == .OK, let url = panel.url { state.exportBackup(url) }
                }
            } footer: {
                Text(String(localized: "Super Productivity's backup format. Importing replaces everything on this Mac and discards changes that have not synced."))
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
