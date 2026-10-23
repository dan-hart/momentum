// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// Native searchable Settings categories. The same preferences as the GNOME app, under the
// same keys, with secrets going to the Keychain rather than into the defaults.
import MomentumCore
import MomentumKit
import SwiftUI
import UserNotifications

@MainActor @Observable final class SettingsNavigation {
    static let shared = SettingsNavigation()
    private var route = SettingsRouteState()
    var category: String? { get { route.category } set { route.category = newValue; route.destination = nil } }
    var destination: String? { route.destination }
    var query: String { get { route.query } set { route.query = newValue } }
    func open(_ id: String?, query: String? = nil) {
        route.open(id, query: query, platform: .macos)
    }
}

struct SettingsView: SwiftUI.View {
    @State private var navigation = SettingsNavigation.shared
    var body: some SwiftUI.View {
        NavigationSplitView {
            List(selection: $navigation.category) {
                if navigation.query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                    ForEach(SettingsDiscovery.categories) { category in
                        Label(category.title, systemImage: category.symbol).tag(category.id)
                    }
                } else {
                    let results = SettingsDiscovery.results(navigation.query, platform: .macos)
                    ForEach(results, id: \.id) { entry in
                        Button { navigation.open(entry.id) } label: {
                            VStack(alignment: .leading) { Text(entry.title); Text(entry.category).font(.caption).foregroundStyle(.secondary) }
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .contentShape(Rectangle())
                        }.buttonStyle(.borderless)
                    }
                    if results.isEmpty { Button(String(localized: "Clear search")) { navigation.query = "" } }
                }
            }
            .searchable(text: $navigation.query, prompt: String(localized: "Search settings"))
            .navigationSplitViewColumnWidth(min: 210, ideal: 240)
        } detail: {
            ScrollViewReader { proxy in
                category
                    .environment(\.settingsDestination, navigation.destination)
                    .onChange(of: navigation.destination, initial: true) { _, id in
                        guard let id else { return }
                        Task { await Task.yield(); proxy.scrollTo(id, anchor: .center) }
                    }
            }
            .navigationTitle(SettingsDiscovery.categories.first { $0.id == navigation.category }?.title ?? "Settings")
        }
        .frame(minWidth: 780, idealWidth: 920, minHeight: 520, idealHeight: 700)
    }
    @ViewBuilder private var category: some SwiftUI.View {
        switch navigation.category {
        case "appearance": AppearanceSettingsPage()
        case "sounds": Form { SoundSettingsSection() }.formStyle(.grouped)
        case "notifications": GeneralSettings(category: "notifications")
        case "sync": SyncSettings()
        case "data": BackupSettings()
        case "integrations": GeneralSettings(category: "integrations")
        case "about": MomentumAboutView()
        default: GeneralSettings(category: "tasks")
        }
    }
}

private struct AppearanceSettingsPage: SwiftUI.View {
    @AppStorage(PrefKey.colorfulLabels) private var colorful = true
    @AppStorage(PrefKey.rowDensity) private var density = "regular"
    var body: some SwiftUI.View {
        Form {
            Section {
                Toggle(String(localized: "Color-code projects and tags"), isOn: $colorful).settingsTarget("appearance.colors")
                Toggle(String(localized: "Compact rows"), isOn: Binding(get: { density == "compact" }, set: { density = $0 ? "compact" : "regular" })).settingsTarget("appearance.compact")
                Text(String(localized: "Uses the system appearance and accent color.")).settingsTarget("appearance.accent")
            }
            FontSettingsControls()
        }.formStyle(.grouped)
    }
}

struct SettingsSearchResults: SwiftUI.View {
    let query: String
    @Environment(\.openSettings) private var openSettings
    private var results: [SettingsEntry] { SettingsDiscovery.results(query, platform: .macos) }
    var body: some SwiftUI.View {
        if !results.isEmpty {
            Section(String(localized: "Settings")) {
                ForEach(Array(results.prefix(5)), id: \.id) { entry in
                    Button { SettingsNavigation.shared.open(entry.id); openSettings() } label: {
                        VStack(alignment: .leading) { Text(entry.title); Text(entry.category).font(.caption).foregroundStyle(.secondary) }
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .contentShape(Rectangle())
                    }.buttonStyle(.borderless)
                }
                if results.count > 5 {
                    Button(String(localized: "See all settings results")) { SettingsNavigation.shared.open(nil, query: query); openSettings() }
                }
            }
        }
    }
}

private struct GeneralSettings: SwiftUI.View {
    let category: String
    @State private var notificationStatus = ""
    @AppStorage(PrefKey.groupBy) private var group = "morning-night"
    @AppStorage(PrefKey.taskSort) private var sort = "manual"
    @AppStorage(PrefKey.sortDirection) private var direction = "ascending"
    @AppStorage(PrefKey.upcomingRange) private var range = "7"
    @Environment(AppState.self) private var state
    @AppStorage(PrefKey.showDayPeriodTasksInToday) private var showDayPeriods = true
    @AppStorage(PrefKey.dynamicEmptyMessages) private var dynamicMessages = true
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
            if category == "tasks" {
            Section(String(localized: "Tasks")) {
                Picker(String(localized: "Group By"), selection: $group) {
                    Text(String(localized: "Morning & Night")).tag("morning-night")
                    Text(String(localized: "None")).tag("none")
                    Text(String(localized: "Project")).tag("project")
                    Text(String(localized: "Tag")).tag("tag")
                    Text(String(localized: "Time Estimate")).tag("estimate")
                }.settingsTarget("tasks.grouping")
                Picker(String(localized: "Sort By"), selection: $sort) {
                    Text(String(localized: "Manual Order")).tag("manual")
                    Text(String(localized: "Title")).tag("title")
                    Text(String(localized: "Due Day")).tag("due")
                    Text(String(localized: "Estimate")).tag("estimate")
                    Text(String(localized: "Created")).tag("created")
                }.settingsTarget("tasks.sort")
                Picker(String(localized: "Order"), selection: $direction) {
                    Text(String(localized: "Ascending")).tag("ascending")
                    Text(String(localized: "Descending")).tag("descending")
                }.settingsTarget("tasks.sort-direction")
                Picker(String(localized: "Upcoming"), selection: $range) {
                    Text(String(localized: "Next 7 Days")).tag("7")
                    Text(String(localized: "Next 30 Days")).tag("30")
                }.settingsTarget("tasks.upcoming")
                Toggle(String(localized: "Show Morning and Evening tasks in Today"), isOn: $showDayPeriods).settingsTarget("tasks.day-period-visibility")
                Toggle(String(localized: "Archive completed tasks immediately"), isOn: $autoArchive).settingsTarget("tasks.archive")
                Text(String(localized: "A task you complete goes straight to the archive. Undo brings it back."))
                    .appFont(.caption).foregroundStyle(.secondary)
                Toggle("Dynamic empty-state messages", isOn: $dynamicMessages).settingsTarget("tasks.dynamic-messages")
            }
            }
            if category == "notifications" {
            Section(String(localized: "Notifications")) {
                LabeledContent(String(localized: "Notification permission"), value: notificationStatus).settingsTarget("notifications.permission")
                Button(String(localized: "Open System Settings")) {
                    if let url = URL(string: "x-apple.systempreferences:com.apple.Notifications-Settings.extension") { NSWorkspace.shared.open(url) }
                }
                Text(String(localized: "Task reminders are configured on each task.")).settingsTarget("notifications.reminders")
                Toggle(String(localized: "Morning summary"), isOn: $morningSummary).settingsTarget("notifications.morning-summary")
                DatePicker(String(localized: "Summary time"), selection: summaryTime,
                           displayedComponents: .hourAndMinute)
                    .disabled(!morningSummary)
                Text(String(localized: "Summarize today's tasks once a day at this local time, or when Momentum next opens. Task reminders are separate."))
                    .appFont(.caption).foregroundStyle(.secondary)
                Text(String(localized: "Off by default. Momentum must be running to send notifications."))
                    .appFont(.caption).foregroundStyle(.secondary)
            }
            }
            if category == "integrations" {
            Section {
                Picker(String(localized: "Shortcut modifier"), selection: $modifier) {
                    ForEach(ModifierKey.allCases) { m in
                        Text("\(m.label) (\(m.symbol))").tag(m.rawValue)
                    }
                }.settingsTarget("integrations.shortcuts")
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
                }.settingsTarget("integrations.dock")
                Text(String(localized: "Counts unfinished tasks. Subtasks are included with their parent. The badge is hidden when the count is zero."))
                    .appFont(.caption).foregroundStyle(.secondary)
                Toggle(String(localized: "Keep running in the background"), isOn: $background).settingsTarget("integrations.startup")
                Text(String(localized: "Reminders and sync keep going when the window is closed, and Momentum starts at login."))
                    .appFont(.caption).foregroundStyle(.secondary)
                Toggle(String(localized: "Show in the menu bar"), isOn: $menuBar).settingsTarget("integrations.menu-bar")
            }
            RemindersSettingsControls().settingsTarget("integrations.reminders-import")
            }
        }
        .formStyle(.grouped)
        .task {
            guard category == "notifications" else { return }
            let settings = await UNUserNotificationCenter.current().notificationSettings()
            notificationStatus = settings.authorizationStatus == .authorized ? String(localized: "Notifications allowed") : String(localized: "Notifications unavailable")
        }
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
    @AppStorage(PrefKey.nextcloudOptimizedConnections) private var optimizedConnections = "[]"
    @State private var confirmingOptimizedUpgrade = false
    @State private var optimizedUpgradeIdentity = ""
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
                }.settingsTarget("sync.provider")
                .disabled(state.syncInProgress)
            } footer: {
                Text(String(localized: "Choose one service to keep your tasks in sync. Switching services keeps your tasks and saved connections."))
            }
            Section(String(localized: "App settings")) {
                Toggle(String(localized: "Sync app settings"), isOn: Binding(
                    get: { state.appSettingsSync.enabled }, set: { state.setAppSettingsSync($0) })).settingsTarget("sync.settings")
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
            if let target = SettingsNavigation.shared.destination, target.hasPrefix("sync."),
               (["sync.nextcloud", "sync.automatic", "sync.compression", "sync.folder-copy"].contains(target) && method != "nextcloud") || (["sync.libresync", "sync.devices"].contains(target) && method != "libresync") || (["sync.status", "sync.background"].contains(target) && method == "off") {
                Text(String(localized: "This setting belongs to an inactive sync provider. Choose that provider to edit its connection; opening this result does not change your provider.")).settingsTarget(target)
            }
            if state.isDemo {
                Text(String(localized: "Sync is unavailable in preview mode. Open Momentum normally to sync your tasks."))
            } else if method == SyncMethod.off.rawValue {
                Text(String(localized: "Your tasks are stored on this Mac. Choose a service above to sync them with your other devices."))
                    .foregroundStyle(.secondary)
            }
            if method == SyncMethod.nextcloud.rawValue {
                Section(String(localized: "Connection")) {
                    TextField(String(localized: "Server URL"), text: $server, prompt: Text(verbatim: "https://cloud.example.com")).settingsTarget("sync.nextcloud")
                    TextField(String(localized: "Username"), text: $user)
                    SecureField(String(localized: "App password"), text: $password)
                    TextField(String(localized: "Folder"), text: $folder)
                    SecureField(String(localized: "Encryption password"), text: $encryption)
                }
                .disabled(state.syncInProgress)

                if folder != "momentum" || state.folderCopyMessage != nil {
                    Section {
                        Text(String(localized: "Copy to Momentum folder…")).settingsTarget("sync.folder-copy")
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
                    Toggle(String(localized: "Sync automatically"), isOn: $autoSync).settingsTarget("sync.automatic")
                    backgroundSyncToggle
                    Toggle(String(localized: "Compress the sync file"), isOn: $compress).settingsTarget("sync.compression")
                    if state.prefs.nextcloudOptimized {
                        Label(String(localized: "Optimized sync enabled"), systemImage: "checkmark.circle")
                    } else {
                        Button(String(localized: "Upgrade to optimized sync…")) {
                            optimizedUpgradeIdentity = state.prefs.nextcloudConnectionIdentity
                            confirmingOptimizedUpgrade = true
                        }
                            .disabled(state.isSyncing || !state.prefs.syncConfigured)
                    }
                    Text(String(localized: "Upgrade every participating device and enable optimized sync on each one. This connection cannot return to legacy sync. Existing legacy files stay intact. Turning sync off remains available."))
                        .font(.caption).foregroundStyle(.secondary)

                }
            } else if method == SyncMethod.libresync.rawValue {
                Section("LibreSync") {
                    Text("LibreSync").settingsTarget("sync.libresync")
                    Text(String(localized: "Your devices sync directly over the local network, end-to-end encrypted, with no server. Link them once with a six-digit code."))
                    LabeledContent(String(localized: "Linked devices"), value: "\(state.syncStatus.linkedDevices)")
                    Button(String(localized: "Manage Devices…")) { state.showDevices() }.settingsTarget("sync.devices")
                }
                Section(String(localized: "When")) {
                    backgroundSyncToggle
                }
            }
            if method == "nextcloud", folder == "momentum", SettingsNavigation.shared.destination == "sync.folder-copy" {
                Text(String(localized: "The Momentum folder is already selected.")).settingsTarget("sync.folder-copy")
            }
            if state.syncAvailable {
                Section(String(localized: "Status")) {
                    SyncStatusBar().settingsTarget("sync.status")
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
        .alert(String(localized: "Upgrade to optimized sync?"), isPresented: $confirmingOptimizedUpgrade) {
            Button(String(localized: "Cancel"), role: .cancel) {}
            Button(String(localized: "Upgrade")) { state.upgradeOptimizedSync(expectedIdentity: optimizedUpgradeIdentity) }
        } message: {
            Text(String(localized: "Upgrade every participating device and enable optimized sync on each one. This connection cannot return to legacy sync. Existing legacy files stay intact. Turning sync off remains available."))
        }
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
        Toggle(String(localized: "Sync in the background"), isOn: $backgroundSync).settingsTarget("sync.background")
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
                }.settingsTarget("data.tasks-import")
                Toggle(String(localized: "Include app settings"), isOn: $includeSettings)
                Button(String(localized: "Export Backup…")) {
                    let panel = NSSavePanel()
                    panel.allowedContentTypes = [.json]
                    panel.nameFieldStringValue = "\(today()).json"
                    if panel.runModal() == .OK, let url = panel.url { state.exportBackup(url, includeSettings: includeSettings) }
                }.settingsTarget("data.tasks-export")
            } footer: {
                Text(String(localized: "Importing replaces all tasks on this Mac and discards unsynced task changes. App settings are optional; credentials and device controls stay local."))
            }
            Section(String(localized: "App settings only")) {
                Button(String(localized: "Import app settings…")) {
                    let panel = NSOpenPanel()
                    panel.allowedContentTypes = [.json]
                    if panel.runModal() == .OK, let url = panel.url { state.importAppSettings(url) }
                }.settingsTarget("data.settings-import")
                Button(String(localized: "Export app settings…")) {
                    let panel = NSSavePanel()
                    panel.allowedContentTypes = [.json]
                    panel.nameFieldStringValue = "momentum-settings.json"
                    if panel.runModal() == .OK, let url = panel.url { state.exportAppSettings(url) }
                }.settingsTarget("data.settings-export")
                if let error = state.appSettingsError {
                    Text(error).foregroundStyle(.red)
                    if state.canRetryAppSettings {
                        Button(String(localized: "Retry")) { state.retryAppSettings() }
                    }
                }
            }
            Section(String(localized: "Data")) {
                LabeledContent(String(localized: "Folder"), value: state.dataDir.path).settingsTarget("data.summary")
                    .textSelection(.enabled)
                Button(String(localized: "Show in Finder")) {
                    NSWorkspace.shared.activateFileViewerSelecting([state.dataDir])
                }
            }
        }
        .formStyle(.grouped)
    }
}
