// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumKit
import MomentumMobile
import SwiftUI

struct SettingsScreen: View {
    @Environment(MobileAppModel.self) private var model
    var body: some View {
        Form {
            Section {
                NavigationLink { TaskListSettings() } label: {
                    SettingsLabel("Task Lists", symbol: "checklist", subtitle: "Grouping, sorting, and completed tasks")
                }.accessibilityIdentifier("settings-task-lists")
                NavigationLink { AppearanceOverview() } label: {
                    SettingsLabel("Appearance", symbol: "paintpalette.fill", subtitle: "Accent color, text, and feedback")
                }.accessibilityIdentifier("settings-appearance")
                NavigationLink { NotificationSettings() } label: {
                    SettingsLabel("Notifications", symbol: "bell.badge.fill", subtitle: "Reminders and morning summary")
                }.accessibilityIdentifier("settings-notifications")
                if model.platformCapabilities.showsKeyboardShortcuts {
                    NavigationLink { KeyboardSettings() } label: {
                        SettingsLabel("Keyboard Shortcuts", symbol: "keyboard")
                    }.accessibilityIdentifier("settings-keyboard")
                }
            }
            Section {
                NavigationLink { SyncSettings(state: model.sync) } label: {
                    SettingsLabel("Sync", symbol: "arrow.trianglehead.2.clockwise.rotate.90",
                                  subtitle: "Connect devices and keep tasks up to date")
                }.accessibilityIdentifier("settings-sync")
                NavigationLink { BackupSettings() } label: {
                    SettingsLabel("Backups", symbol: "externaldrive.fill", subtitle: "Save or restore your tasks")
                }.accessibilityIdentifier("settings-backups")
            }
            Section {
                NavigationLink { AboutSettings() } label: {
                    SettingsLabel("About Momentum", symbol: "info.circle.fill")
                }
            } footer: {
                Text("Free and open source. Your tasks work offline, without an account.")
                    .foregroundStyle(AccentTheme.secondaryText)
            }
        }
        .navigationTitle("Settings")
        .momentumNavigationCanvas()
    }
}

private struct TaskListSettings: View {
    @Environment(MobileAppModel.self) private var model
    @AppStorage(PrefKey.autoArchive) private var autoArchive = false
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
                    .accessibilityIdentifier("settings-group")
                AdaptiveSettingsPicker("Sort By", symbol: "line.3.horizontal.decrease", selection: $sort,
                                       options: [("manual", "Manual Order"), ("title", "Title"), ("due", "Due Day"),
                                                 ("estimate", "Estimate"), ("created", "Created")])
                    .accessibilityIdentifier("settings-sort")
                AdaptiveSettingsPicker("Order", symbol: "arrow.up.arrow.down", selection: $direction,
                                       options: [("ascending", "Ascending"), ("descending", "Descending")])
                    .accessibilityIdentifier("settings-order")
                AdaptiveSettingsPicker("Upcoming", symbol: "calendar", selection: $range,
                                       options: [("7", "Next 7 Days"), ("30", "Next 30 Days")])
                    .accessibilityIdentifier("settings-range")
            } header: { Text("Organization").foregroundStyle(AccentTheme.secondaryText) }
            Section {
                Toggle(isOn: $autoArchive) {
                    SettingsLabel("Archive completed tasks immediately", symbol: "archivebox.fill")
                }
            } header: { Text("Completed Tasks").foregroundStyle(AccentTheme.secondaryText) }
        }
        .navigationTitle("Task Lists")
        .momentumNavigationCanvas()
        .navigationBarTitleDisplayMode(.inline)
        .onChange(of: "\(autoArchive)-\(group)-\(sort)-\(direction)-\(range)") {
            Task { await model.preferencesChanged() }
        }
    }
}

private struct AppearanceOverview: View {
    @AppStorage(PrefKey.colorfulLabels) private var colorful = true
    var body: some View {
        Form {
            Section {
                NavigationLink { AccentSettings() } label: {
                    SettingsLabel("Accent Color", symbol: "swatchpalette.fill")
                }
                Toggle(isOn: $colorful) {
                    SettingsLabel("Color-code projects and tags", symbol: "tag.fill")
                }
            } header: { Text("Colors").foregroundStyle(AccentTheme.secondaryText) }
            Section {
                NavigationLink { AppearanceSettings() } label: {
                    SettingsLabel("Text & Feedback", symbol: "textformat.size")
                }
            }
        }
        .navigationTitle("Appearance")
        .momentumNavigationCanvas()
        .navigationBarTitleDisplayMode(.inline)
    }
}

private struct AboutSettings: View {
    var body: some View {
        Form {
            LabeledContent("Version", value: Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "—")
            Link(destination: URL(string: "https://github.com/dan-hart/momentum")!) {
                SettingsLabel("Source Code", symbol: "chevron.left.forwardslash.chevron.right")
            }
        }
        .navigationTitle("About Momentum")
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
