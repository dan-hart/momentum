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
                            VStack(alignment: .leading) { Text(entry.title); Text(entry.category).appFont(.caption).foregroundStyle(.secondary) }
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
        case "updates": UpdatesSettings()
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
                        VStack(alignment: .leading) { Text(entry.title); Text(entry.category).appFont(.caption).foregroundStyle(.secondary) }
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
    @AppStorage(PrefKey.morningSummaryWeekdays) private var summaryWeekdays = 127
    @AppStorage(PrefKey.morningSummarySections) private var summarySections = 7
    @AppStorage(PrefKey.remindersUpdateBadge) private var remindersUpdateBadge = false
    @AppStorage(PrefKey.summaryUpdatesBadge) private var summaryUpdatesBadge = false


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

    private var dockBadgePicker: some SwiftUI.View {
        Picker(String(localized: "Dock badge"), selection: $dockBadgeMode) {
            ForEach(DockBadgeMode.allCases) { mode in
                Text(mode.label).tag(mode.rawValue)
            }
        }
    }

    private func includes(_ bit: Int, in selection: Binding<Int>, defaultValue: Int, maximum: Int) -> Binding<Bool> {
        func normalized() -> Int {
            let saved = selection.wrappedValue
            return (0...maximum).contains(saved) ? saved : defaultValue
        }
        return Binding {
            normalized() & bit != 0
        } set: { isOn in
            let current = normalized()
            selection.wrappedValue = isOn ? current | bit : current & ~bit
        }
    }

    private var summaryDayChoices: some SwiftUI.View {
        let calendar = Calendar.current
        let symbols = calendar.weekdaySymbols
        return DisclosureGroup(String(localized: "Summary days")) {
            HStack {
                Button(String(localized: "Every day")) { summaryWeekdays = 127 }
                Button(String(localized: "Weekdays")) { summaryWeekdays = 31 }
            }
            .buttonStyle(.borderless)
            ForEach(0..<7) { index in
                // Foundation weekday symbols start at Sunday. The stored mask starts at Monday.
                let day = (calendar.firstWeekday - 1 + index) % 7
                let bit = 1 << ((day + 6) % 7)
                Toggle(symbols[day], isOn: includes(bit, in: $summaryWeekdays, defaultValue: 127, maximum: 127))
            }
        }
    }

    private var summarySectionChoices: some SwiftUI.View {
        DisclosureGroup(String(localized: "Summary sections")) {
            ForEach(MorningSummarySection.allCases) { section in
                Toggle(section.label, isOn: includes(Int(section.rawValue), in: $summarySections, defaultValue: 7, maximum: 15))
            }
        }
    }

    private var summaryPreview: String {
        let sections = (0...15).contains(summarySections) ? UInt8(summarySections) : 7
        return NotificationManager.summaryBody(state.engine.morningSummaryPreview(sections: sections))
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
            }
            Section(String(localized: "Badges")) {
                dockBadgePicker.settingsTarget("notifications.badge")
                Toggle(String(localized: "Refresh badge with task reminders"), isOn: $remindersUpdateBadge)
                    .disabled(dockBadgeMode == DockBadgeMode.none.rawValue)
                Toggle(String(localized: "Refresh badge with morning summary"), isOn: $summaryUpdatesBadge)
                    .disabled(dockBadgeMode == DockBadgeMode.none.rawValue)
            }
            Section(String(localized: "Morning summary")) {
                Toggle(String(localized: "Send morning summary"), isOn: $morningSummary)
                    .settingsTarget("notifications.morning-summary")
                Group {
                    DatePicker(String(localized: "Summary time"), selection: summaryTime,
                               displayedComponents: .hourAndMinute)
                    summaryDayChoices
                    summarySectionChoices
                    LabeledContent(String(localized: "Preview"), value: summaryPreview)
                }
                .disabled(!morningSummary)
                Text(String(localized: "Momentum sends the summary on selected days when the app is running and the selected sections contain tasks."))
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
                dockBadgePicker.settingsTarget("integrations.dock")
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
    @State private var password = SyncCredentialDraft(account: Keychain.nextcloud)
    @State private var encryption = SyncCredentialDraft(account: Keychain.encryption)
    @State private var confirmingFolderCopy = false
    @State private var showAdvanced = false
    @State private var connectionTest = NextcloudConnectionTest()

    var body: some SwiftUI.View {
        Form {
            Section {
                Picker(String(localized: "Sync method"), selection: $method) {
                    ForEach(SyncMethod.allCases) { service in
                        Text(service.label).tag(service.rawValue)
                    }
                }.settingsTarget("sync.provider")
                .disabled(state.syncInProgress)
            }
            if method == SyncMethod.nextcloud.rawValue {
                nextcloudConnection
            }
            if state.syncAvailable {
                Section(String(localized: "Status")) {
                    SyncStatusBar().settingsTarget("sync.status")
                        .disabled(connectionTest.isRunning || (method == "nextcloud" && !credentialsSaved))
                    if method == SyncMethod.nextcloud.rawValue {
                        if state.syncStatus.pendingOps == 0 {
                            Text(String(localized: "No changes waiting to sync")).foregroundStyle(.secondary)
                        } else {
                            Text(String(localized: "Waiting changes are saved on this Mac.")).foregroundStyle(.secondary)
                        }
                    } else {
                        Text(String(localized: "Changes sync when linked devices are reachable.")).foregroundStyle(.secondary)
                    }
                    if lastTaskSync > 0 {
                        LabeledContent(String(localized: "Last successful sync")) {
                            Text(Date(timeIntervalSince1970: Double(lastTaskSync) / 1_000), style: .relative)
                        }
                    }
                    if let kind = state.syncFailureKind, state.syncError != nil {
                        Text(Strings.syncFailureHeadline(kind)).fontWeight(.semibold)
                        Text(Strings.syncFailureRemedy(kind))
                            .fixedSize(horizontal: false, vertical: true)
                    }
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
                Section(String(localized: "When")) {
                    Toggle(String(localized: "Sync automatically"), isOn: $autoSync).settingsTarget("sync.automatic")
                    backgroundSyncToggle
                }
                Section {
                    DisclosureGroup(String(localized: "Advanced Nextcloud options"), isExpanded: Binding(
                        get: { showAdvanced || SyncSettingsDisclosure.requiresExpansion(
                            destination: SettingsNavigation.shared.destination,
                            operationActive: state.folderCopyInProgress, operationMessage: state.folderCopyMessage) },
                        set: { showAdvanced = $0 }
                    )) {
                        TextField(String(localized: "Folder"), text: $folder)
                            .disabled(state.syncInProgress)
                        Text(String(localized: "Use the same folder on every device. Changing this field does not move existing sync files."))
                            .appFont(.caption).foregroundStyle(.secondary)
                        SecureField(String(localized: "Encryption password"), text: Binding(
                            get: { encryption.value }, set: { connectionTest.cancel(); encryption.save($0, to: state.keychain) }
                        )).disabled(!encryption.isLoaded)
                            .disabled(state.syncInProgress)
                        Toggle(String(localized: "Compress the sync file"), isOn: $compress).settingsTarget("sync.compression")
                        if state.prefs.nextcloudOptimized {
                            Label(String(localized: "Optimized sync enabled"), systemImage: "checkmark.circle")
                        } else {
                            Button(String(localized: "Upgrade to optimized sync…")) {
                                optimizedUpgradeIdentity = state.prefs.nextcloudConnectionIdentity
                                confirmingOptimizedUpgrade = true
                            }
                                .disabled(state.isSyncing || !state.prefs.syncConfigured || !credentialsSaved)
                        }
                        Text(String(localized: "Upgrade every participating device and enable optimized sync on each one. This connection cannot return to legacy sync. Existing legacy files stay intact. Turning sync off remains available."))
                            .appFont(.caption).foregroundStyle(.secondary)
                        if folder != "momentum" || state.folderCopyMessage != nil {
                            Text(String(localized: "Copy to Momentum folder…")).settingsTarget("sync.folder-copy")
                            if state.folderCopyInProgress {
                                ProgressView(String(localized: "Copying and verifying files…"))
                                Button(String(localized: "Cancel")) { state.cancelFolderCopy() }
                            } else if folder != "momentum" {
                                Button(String(localized: "Copy to Momentum folder…")) { confirmingFolderCopy = true }
                                    .disabled(!state.canCopyFolder || !credentialsSaved)
                            }
                            if let message = state.folderCopyMessage { Text(message).textSelection(.enabled) }
                            Text(String(localized: "The original folder stays intact. The saved folder changes only after every copied file is verified."))
                                .appFont(.caption).foregroundStyle(.secondary)
                        }

                        if folder == "momentum", SettingsNavigation.shared.destination == "sync.folder-copy" {
                            Text(String(localized: "The Momentum folder is already selected.")).settingsTarget("sync.folder-copy")
                        }
                    }
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
            Section(String(localized: "App settings")) {
                Toggle(String(localized: "Sync app settings"), isOn: Binding(
                    get: { state.appSettingsSync.enabled }, set: { state.setAppSettingsSync($0) })).settingsTarget("sync.settings")
                    .disabled(state.isDemo)
                Text(String(localized: "Off by default. Uses your selected sync provider for task display, grouping, sorting, archive and sound preferences. Connections, credentials, permissions and device controls stay local."))
                    .appFont(.caption).foregroundStyle(.secondary)
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
                        .disabled(connectionTest.isRunning || (method == "nextcloud" && !credentialsSaved))
                }
            }

        }
        .formStyle(.grouped)
        .disabled(state.isDemo)
        .onAppear {
            guard !state.isDemo else { return }
            password.load(from: state.keychain)
            encryption.load(from: state.keychain)
        }
        .onChange(of: method) { _, _ in connectionTest.cancel(); state.preferencesChanged() }
        .onChange(of: server) { _, _ in connectionTest.cancel() }
        .onChange(of: user) { _, _ in connectionTest.cancel() }
        .onChange(of: folder) { _, _ in connectionTest.cancel() }
        .onChange(of: compress) { _, _ in connectionTest.cancel() }
        .onChange(of: optimizedConnections) { _, _ in connectionTest.cancel() }
        .onChange(of: state.syncInProgress) { _, active in if active { connectionTest.cancel() } }
        .onChange(of: backgroundSync) { _, _ in state.preferencesChanged() }
        .onDisappear { connectionTest.cancel(); state.cancelFolderCopy() }
        .alert(String(localized: "Upgrade to optimized sync?"), isPresented: $confirmingOptimizedUpgrade) {
            Button(String(localized: "Cancel"), role: .cancel) {}
            Button(String(localized: "Upgrade")) {
                guard credentialsSaved else { return }
                state.upgradeOptimizedSync(expectedIdentity: optimizedUpgradeIdentity)
            }
        } message: {
            Text(String(localized: "Upgrade every participating device and enable optimized sync on each one. This connection cannot return to legacy sync. Existing legacy files stay intact. Turning sync off remains available."))
        }
        .alert(String(localized: "Copy to Momentum folder?"), isPresented: $confirmingFolderCopy) {
            Button(String(localized: "Cancel"), role: .cancel) {}
            Button(String(localized: "Copy and switch")) {
                guard credentialsSaved else { return }
                Task { await state.copyToMomentumFolder() }
            }
        } message: {
            Text(String(localized: "Copy and verify your sync files in the momentum folder, then switch this device to it. The original folder stays intact. Choose the new folder on your other devices after the copy succeeds."))
        }
    }
}

private extension SyncSettings {
    @ViewBuilder var nextcloudConnection: some SwiftUI.View {
        Section(String(localized: "Nextcloud connection")) {
            TextField(String(localized: "Server URL"), text: $server, prompt: Text(verbatim: "https://cloud.example.com")).settingsTarget("sync.nextcloud")
            TextField(String(localized: "Username"), text: $user)
            SecureField(String(localized: "App password"), text: Binding(
                get: { password.value }, set: { connectionTest.cancel(); password.save($0, to: state.keychain) }
            )).disabled(!password.isLoaded)
            DisclosureGroup(String(localized: "Connection Help")) {
                Text(String(localized: "Include any installation subfolder in the server URL. In Nextcloud, create an app password under Personal settings → Security."))
                    .appFont(.caption)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
        .disabled(state.syncInProgress)
        if password.hasError || encryption.hasError {
            Section {
                Label(String(localized: "Credentials need attention"), systemImage: "exclamationmark.triangle")
                Text(String(localized: "The credentials shown could not be read or saved in Keychain. Unlock your Keychain and retry before testing the connection or using Sync Now."))
                    .fixedSize(horizontal: false, vertical: true)
                Button(String(localized: "Retry Keychain")) {
                    connectionTest.cancel()
                    if password.hasError { password.retry(using: state.keychain) }
                    if encryption.hasError { encryption.retry(using: state.keychain) }
                }
            }
        }
        Section {
            if connectionTest.isRunning {
                ProgressView(String(localized: "Testing saved connection…"))
                Button(String(localized: "Cancel")) { connectionTest.cancel() }
            } else {
                Button(String(localized: "Test saved connection")) {
                    guard !state.isDemo, credentialsSaved, let saved = state.prefs.nextcloud(keychain: state.keychain) else { return }
                    connectionTest.start(engine: state.engine, settings: saved)
                }
                .disabled(state.syncInProgress || !state.prefs.syncConfigured || !credentialsSaved)
            }
            if connectionTest.succeeded == true {
                Label(String(localized: "Connection test passed"), systemImage: "checkmark.circle")
            }
            if let error = connectionTest.error {
                Label(String(localized: "Connection test failed"), systemImage: "exclamationmark.triangle")
                Text(error).textSelection(.enabled)
            }
        }
    }

    var credentialsSaved: Bool { password.canUseSavedValue && encryption.canUseSavedValue }

    var lastTaskSync: UInt64 {
        method == SyncMethod.nextcloud.rawValue ? state.syncStatus.lastNextcloudMs : state.syncStatus.lastNearbyMs
    }

    /// Automatic sync keeps running with the window closed only while this is on; Sync
    /// Now always works. Pair it with General › Keep running in the background.
    @ViewBuilder var backgroundSyncToggle: some SwiftUI.View {
        Toggle(String(localized: "Sync in the background"), isOn: $backgroundSync).settingsTarget("sync.background")
        Text(String(localized: "Keep syncing while Momentum runs without a window. Turn this off to sync only while the window is open; Sync Now always works. Needs “Keep running in the background” in General."))
            .appFont(.caption).foregroundStyle(.secondary)
    }
}

@MainActor enum BackupRestoreConfirmation {
    static func confirm(recovery: Bool = false) -> Bool {
        let alert = NSAlert()
        alert.alertStyle = .warning
        alert.messageText = recovery ? String(localized: "Restore Recovery Copy?") : String(localized: "Replace All Tasks?")
        alert.informativeText = recovery
            ? String(localized: "Replace your tasks with the previous tasks and unsynced changes. The recovery copy will then hold your current tasks. App settings stay as they are.")
            : String(localized: "Your current tasks and unsynced changes will be kept in a recovery copy.")
        let cancel = alert.addButton(withTitle: String(localized: "Cancel"))
        cancel.keyEquivalent = "\r"
        let restore = alert.addButton(withTitle: recovery ? String(localized: "Restore Recovery Copy") : String(localized: "Restore Backup"))
        restore.hasDestructiveAction = true
        return alert.runModal() == .alertSecondButtonReturn
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
                    if panel.runModal() == .OK, let url = panel.url, BackupRestoreConfirmation.confirm() {
                        state.importBackup(url, restoreSettings: restoreSettings)
                    }
                }.settingsTarget("data.tasks-import").disabled(!state.canRestoreBackup)
                if state.hasBackupRecovery {
                    Button(String(localized: "Restore Recovery Copy…")) {
                        if BackupRestoreConfirmation.confirm(recovery: true) { state.restoreBackupRecovery() }
                    }.disabled(!state.canRestoreBackup)
                }
                Toggle(String(localized: "Include app settings"), isOn: $includeSettings)
                Button(String(localized: "Export Backup…")) {
                    let panel = NSSavePanel()
                    panel.allowedContentTypes = [.json]
                    panel.nameFieldStringValue = "\(today()).json"
                    if panel.runModal() == .OK, let url = panel.url { state.exportBackup(url, includeSettings: includeSettings) }
                }.settingsTarget("data.tasks-export")
            } footer: {
                Text(String(localized: "Importing replaces your tasks. A recovery copy keeps the previous tasks and unsynced changes on this Mac."))
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
        .onAppear { state.refreshBackupRecovery() }
    }
}
