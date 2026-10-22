import struct MomentumCore.SettingsEntry
// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumKit
import MomentumMobile
import SwiftUI

struct SettingsScreen: View {
    @Environment(MobileAppModel.self) private var model
    @Environment(\.horizontalSizeClass) private var horizontalSizeClass
    @State private var query: String
    @State private var path: [String]
    @State private var regularDestination: String?
    init(initialDestination: String? = nil, initialQuery: String = "") {
        _query = State(initialValue: initialQuery)
        _path = State(initialValue: initialDestination.map { [$0] } ?? [])
        _regularDestination = State(initialValue: initialDestination ?? "tasks.")
    }
    var body: some View {
        Group {
            if horizontalSizeClass == .regular {
                NavigationSplitView {
                    List(selection: $regularDestination) { directoryRows }
                        .navigationTitle("Settings")
                        .momentumNavigationCanvas()
                } detail: {
                    NavigationStack {
                        if let destination = regularDestination { destinationContent(destination) }
                    }.id(regularDestination)
                }
            } else {
                NavigationStack(path: $path) {
                    Form { directoryRows }
                        .navigationTitle("Settings")
                        .navigationDestination(for: String.self) { destinationContent($0) }
                        .momentumNavigationCanvas()
                }
            }
        }
        .searchable(text: $query, prompt: Text("Search settings"))
        .onChange(of: regularDestination) { _, destination in
            if horizontalSizeClass == .regular { path = destination.map { [$0] } ?? [] }
        }
        .onChange(of: path) { _, value in
            if let destination = value.last { regularDestination = destination }
        }
        .onChange(of: query) { _, value in
            if !value.isEmpty { path.removeAll() }
        }
    }
    @ViewBuilder private var directoryRows: some View {
        if query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            ForEach(SettingsDiscovery.categories) { category in
                NavigationLink(value: category.id + ".") {
                    Label {
                        VStack(alignment: .leading, spacing: 4) {
                            Text(category.title)
                            Text(category.subtitle).font(.caption).foregroundStyle(AccentTheme.secondaryText)
                                .fixedSize(horizontal: false, vertical: true)
                        }
                    } icon: { Image(systemName: category.symbol) }
                }.tag(category.id + ".")
            }
        } else {
            let results = entries
            ForEach(results, id: \.id) { entry in
                NavigationLink(value: entry.id) {
                    VStack(alignment: .leading) { Text(entry.title); Text(entry.category).font(.caption).foregroundStyle(.secondary) }
                }.tag(entry.id)
            }
            if results.isEmpty {
                Text("No matching settings")
                Button("Clear search") { query = "" }
            }
        }
    }
    private func destinationContent(_ destination: String) -> some View {
        ScrollViewReader { proxy in
            destinationView(destination)
                .environment(\.settingsDestination, destination)
                .task(id: destination) { await Task.yield(); proxy.scrollTo(destination, anchor: .center) }
        }
        .navigationTitle(SettingsDiscovery.categories.first { $0.id == SettingsDiscovery.category(for: destination) }?.title ?? "Settings")
        .navigationBarTitleDisplayMode(.inline)
    }
    private var entries: [SettingsEntry] {
        SettingsDiscovery.results(query, platform: .ios).filter { $0.id != "integrations.shortcuts" || model.platformCapabilities.showsKeyboardShortcuts }
    }
    @ViewBuilder private func destinationView(_ destination: String) -> some View {
        if destination == "integrations.reminders-import" {
            RemindersSettings().settingsTarget(destination)
        } else if destination == "integrations.shortcuts", model.platformCapabilities.showsKeyboardShortcuts {
            KeyboardSettings().settingsTarget(destination)
        } else if destination == "appearance.accent" {
            AccentSettings().settingsTarget(destination)
        } else {
            category(SettingsDiscovery.category(for: destination))
        }
    }
    @ViewBuilder private func category(_ id: String) -> some View {
        switch id {
        case "appearance": AppearanceOverview()
        case "sounds": MobileSoundSettings()
        case "notifications": NotificationSettings()
        case "sync": SyncSettings(state: model.sync)
        case "data": BackupSettings()
        case "integrations": Form {
            NavigationLink { RemindersSettings() } label: { SettingsLabel("Apple Reminders", symbol: "checklist", subtitle: "Import from a reminders list") }.settingsTarget("integrations.reminders-import")
            if model.platformCapabilities.showsKeyboardShortcuts {
                NavigationLink { KeyboardSettings() } label: { SettingsLabel("Keyboard Shortcuts", symbol: "keyboard") }.settingsTarget("integrations.shortcuts")
            }
        }
        case "about": MomentumAboutView()
        default: TaskListSettings()
        }
    }
}

struct MobileSoundSettings: View {
    @AppStorage(MobileAppearance.hapticsKey) private var haptics = true
    var body: some View {
        Form {
            SoundSettingsSection()
            Section { Toggle("Haptic feedback", isOn: $haptics).settingsTarget("sounds.haptics") }
        }
        .momentumNavigationCanvas()
    }
}

struct SettingsSearchResults: View {
    let query: String
    @Environment(MobileAppModel.self) private var model
    @State private var destination: String?
    @State private var showingSettings = false
    private var results: [SettingsEntry] {
        SettingsDiscovery.results(query, platform: .ios).filter { $0.id != "integrations.shortcuts" || model.platformCapabilities.showsKeyboardShortcuts }
    }
    var body: some View {
        if !results.isEmpty {
            Section("Settings") {
                ForEach(Array(results.prefix(5)), id: \.id) { entry in
                    Button { destination = entry.id; showingSettings = true } label: {
                        VStack(alignment: .leading) { Text(entry.title); Text(entry.category).font(.caption).foregroundStyle(.secondary) }
                    }
                }
                if results.count > 5 {
                    Button("See all settings results") { destination = nil; showingSettings = true }
                }
            }
            .sheet(isPresented: $showingSettings) {
                SettingsScreen(initialDestination: destination, initialQuery: destination == nil ? query : "")
                    .environment(model)
            }
        }
    }
}

private struct TaskListSettings: View {
    @Environment(MobileAppModel.self) private var model
    @AppStorage(PrefKey.dynamicEmptyMessages) private var dynamicMessages = true
    @AppStorage(PrefKey.autoArchive) private var autoArchive = false
    @AppStorage(PrefKey.showDayPeriodTasksInToday) private var showDayPeriods = true
    @AppStorage(PrefKey.groupBy) private var group = "morning-night"
    @AppStorage(PrefKey.taskSort) private var sort = "manual"
    @AppStorage(PrefKey.sortDirection) private var direction = "ascending"
    @AppStorage(PrefKey.upcomingRange) private var range = "7"

    var body: some View {
        Form {
            Section {
                AdaptiveSettingsPicker("Group By", symbol: "square.stack.3d.up.fill", selection: $group,
                                       options: [("morning-night", "Morning & Night"), ("none", "None"),
                                                 ("project", "Project"), ("tag", "Tag"), ("estimate", "Time Estimate")])
                    .accessibilityIdentifier("settings-group").settingsTarget("tasks.grouping")
                Toggle("Show Morning and Evening tasks in Today", isOn: $showDayPeriods)
                    .accessibilityIdentifier("settings-day-periods").settingsTarget("tasks.day-period-visibility")
                AdaptiveSettingsPicker("Sort By", symbol: "line.3.horizontal.decrease", selection: $sort,
                                       options: [("manual", "Manual Order"), ("title", "Title"), ("due", "Due Day"),
                                                 ("estimate", "Estimate"), ("created", "Created")])
                    .accessibilityIdentifier("settings-sort").settingsTarget("tasks.sort")
                AdaptiveSettingsPicker("Order", symbol: "arrow.up.arrow.down", selection: $direction,
                                       options: [("ascending", "Ascending"), ("descending", "Descending")])
                    .accessibilityIdentifier("settings-order").settingsTarget("tasks.sort-direction")
                AdaptiveSettingsPicker("Upcoming", symbol: "calendar", selection: $range,
                                       options: [("7", "Next 7 Days"), ("30", "Next 30 Days")])
                    .accessibilityIdentifier("settings-range").settingsTarget("tasks.upcoming")
            } header: { Text("Organization").foregroundStyle(AccentTheme.secondaryText) }
            footer: {
                Text("Upcoming is always grouped by day. Group By applies to Today, projects and tags.")
                    .foregroundStyle(AccentTheme.secondaryText)
            }
            Section {
                Toggle("Dynamic empty-state messages", isOn: $dynamicMessages).settingsTarget("tasks.dynamic-messages")
                Toggle(isOn: $autoArchive) {
                    SettingsLabel("Archive completed tasks immediately", symbol: "archivebox.fill").settingsTarget("tasks.archive")
                }
            } header: { Text("Completed Tasks").foregroundStyle(AccentTheme.secondaryText) }
        }
        .navigationTitle("Task Lists")
        .momentumNavigationCanvas()
        .navigationBarTitleDisplayMode(.inline)
        .onChange(of: "\(autoArchive)-\(group)-\(showDayPeriods)-\(sort)-\(direction)-\(range)") {
            Task { await model.preferencesChanged() }
        }
    }
}

private struct AppearanceOverview: View {
    @AppStorage(PrefKey.rowDensity) private var density = "regular"
    @AppStorage(PrefKey.colorfulLabels) private var colorful = true
    var body: some View {
        Form {
            Section {
                NavigationLink { AccentSettings() } label: {
                    SettingsLabel("Accent Color", symbol: "swatchpalette.fill").settingsTarget("appearance.accent")
                }
                Toggle(isOn: $colorful) {
                    SettingsLabel("Color-code projects and tags", symbol: "tag.fill").settingsTarget("appearance.colors")
                }
            } header: { Text("Colors").foregroundStyle(AccentTheme.secondaryText) }
            Section {
                Toggle("Compact rows", isOn: Binding(get: { density == "compact" }, set: { density = $0 ? "compact" : "regular" })).settingsTarget("appearance.compact")
            }
            AppearanceSettingsControls()
        }
        .navigationTitle("Appearance")
        .momentumNavigationCanvas()
        .navigationBarTitleDisplayMode(.inline)
    }
}

/// Menu values are single-line system controls. At accessibility sizes, give the
/// current value its own wrapping row and use a native list for choosing an option.
private struct AdaptiveSettingsPicker: View {
    private struct Option: Identifiable {
        let id: String
        let title: LocalizedStringKey
    }
    let title: LocalizedStringKey
    let symbol: String
    @Binding var selection: String
    private let options: [Option]
    @Environment(\.dynamicTypeSize) private var textSize

    init(_ title: LocalizedStringKey, symbol: String, selection: Binding<String>,
         options: [(String, LocalizedStringKey)]) {
        self.title = title
        self.symbol = symbol
        _selection = selection
        self.options = options.map { Option(id: $0.0, title: $0.1) }
    }

    var body: some View {
        if textSize.isAccessibilitySize {
            NavigationLink {
                Form {
                    Picker(title, selection: $selection) { choices }
                        .pickerStyle(.inline)
                        .labelsHidden()
                }
                .navigationTitle(title)
                .navigationBarTitleDisplayMode(.inline)
                .momentumNavigationCanvas()
            } label: {
                VStack(alignment: .leading, spacing: 8) {
                    SettingsLabel(title, symbol: symbol)
                    Text(options.first { $0.id == selection }?.title ?? "")
                        .foregroundStyle(AccentTheme.secondaryText)
                }
                .fixedSize(horizontal: false, vertical: true)
                .padding(.vertical, 4)
            }
        } else {
            Picker(selection: $selection) { choices } label: {
                SettingsLabel(title, symbol: symbol)
            }
        }
    }

    private var choices: some View {
        ForEach(options) { option in
            Text(option.title)
                .fixedSize(horizontal: false, vertical: true)
                .tag(option.id)
        }
    }
}
