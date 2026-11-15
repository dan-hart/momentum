// SPDX-License-Identifier: GPL-3.0-or-later
import struct MomentumCore.MorningSummary
import MomentumKit
import MomentumMobile
import SwiftUI
import UIKit

struct NotificationSettings: View {
    @Environment(MobileAppModel.self) private var model
    @AppStorage(PrefKey.morningSummaryEnabled) private var summaryEnabled = false
    @AppStorage(PrefKey.morningSummaryHour) private var hour = 8
    @AppStorage(PrefKey.morningSummaryMinute) private var minute = 0
    @AppStorage(PrefKey.morningSummaryWeekdays) private var weekdays = 127
    @AppStorage(PrefKey.morningSummarySections) private var sections = 7
    @AppStorage(PrefKey.remindersUpdateBadge) private var remindersUpdateBadge = false
    @AppStorage(PrefKey.summaryUpdatesBadge) private var summaryUpdatesBadge = false
    @AppStorage(PrefKey.dockBadgeMode) private var badgeMode = DockBadgeMode.dueToday.rawValue
    @State private var editingSummaryTime = false
    @State private var summaryTimeDraft = Date()
    @State private var summaryPreview: MorningSummary?

    private var notifications: MobileNotifications { model.notifications }

    var body: some View {
        Form {
            Section {
                LabeledContent { Text(permissionLabel).foregroundStyle(AccentTheme.secondaryText) } label: {
                    SettingsLabel("Permission", symbol: "bell.badge.fill").settingsTarget("notifications.permission")
                }
                if notifications.isolated {
                    Text("System notifications are disabled in preview and test mode.")
                        .foregroundStyle(AccentTheme.secondaryText)
                } else if notifications.status?.authorization == .notDetermined {
                    Button("Enable Notifications", systemImage: "bell.fill") {
                        Task { await notifications.requestPermission() }
                    }.disabled(notifications.requestingPermission)
                } else {
                    Button("Open iOS Settings", systemImage: "arrow.up.forward.app") {
                        guard let url = URL(string: UIApplication.openSettingsURLString) else { return }
                        UIApplication.shared.open(url)
                    }
                }
                if notifications.permissionFailed {
                    Text("Permission could not be requested. Try again or open iOS Settings.")
                        .foregroundStyle(AccentTheme.secondaryText)
                }
            } header: { Text("Delivery").mobileFont(.footnote).foregroundStyle(AccentTheme.secondaryText) }
            footer: {
                Text("Reminders work offline. Focus, notification settings, and iOS may delay delivery.").settingsTarget("notifications.reminders").mobileFont(.footnote)
                    .foregroundStyle(AccentTheme.secondaryText)
            }

            Section {
                Toggle(isOn: $summaryEnabled) {
                    SettingsLabel("Morning Summary", symbol: "sun.max.fill")
                }.accessibilityIdentifier("notifications-summary").settingsTarget("notifications.morning-summary")
                if summaryEnabled {
                    Button {
                        summaryTimeDraft = summaryTime.wrappedValue
                        editingSummaryTime = true
                    } label: {
                        LabeledContent {
                            Text(summaryTime.wrappedValue.formatted(date: .omitted, time: .shortened))
                                .foregroundStyle(AccentTheme.secondaryText)
                        } label: {
                            SettingsLabel("Time", symbol: "clock.fill")
                        }
                    }
                    .accessibilityIdentifier("notifications-summary-time")

                    NavigationLink {
                        summaryDays
                    } label: {
                        LabeledContent {
                            Text(daySelectionLabel).foregroundStyle(AccentTheme.secondaryText)
                        } label: {
                            SettingsLabel("Days", symbol: "calendar")
                        }
                    }
                    .accessibilityIdentifier("notifications-summary-days")

                    NavigationLink {
                        Form {
                            ForEach(MorningSummarySection.allCases) { section in
                                Toggle(section.label, isOn: selected($sections, bit: Int(section.rawValue)))
                                    .accessibilityIdentifier("notifications-summary-section-\(section.rawValue)")
                            }
                        }
                        .navigationTitle("Include in Summary")
                        .momentumNavigationTypography()
                    } label: {
                        SettingsLabel("Include in Summary", symbol: "list.bullet")
                    }
                    .accessibilityIdentifier("notifications-summary-sections")

                    if weekdays == 0 || sections == 0 {
                        Text("Select at least one day and one section to receive the summary.")
                            .foregroundStyle(AccentTheme.secondaryText)
                    } else if let preview = summaryPreview, Int(preview.sections) == sections {
                        LabeledContent("Preview") { Text(preview.displayText) }
                            .accessibilityIdentifier("notifications-summary-preview")
                    }
                }
            } header: { Text("Daily Overview").mobileFont(.footnote).foregroundStyle(AccentTheme.secondaryText) }
            footer: {
                Text("The summary skips days when the selected sections are empty. Configure individual reminders on each task.").mobileFont(.footnote)
                    .foregroundStyle(AccentTheme.secondaryText)
            }

            Section {
                Picker(selection: $badgeMode) {
                    ForEach(DockBadgeMode.allCases) { mode in Text(mode.label).tag(mode.rawValue) }
                } label: { SettingsLabel("App Badge", symbol: "app.badge.fill") }
                .accessibilityIdentifier("notifications-badge-mode").settingsTarget("notifications.badge")
                Group {
                    Toggle("Task Reminders Update Badge", isOn: $remindersUpdateBadge)
                        .accessibilityIdentifier("notifications-reminders-update-badge")
                    Toggle("Morning Summary Updates Badge", isOn: $summaryUpdatesBadge)
                        .accessibilityIdentifier("notifications-summary-updates-badge")
                }
                .disabled(badgeMode == DockBadgeMode.none.rawValue)
                if badgeMode != DockBadgeMode.none.rawValue && notifications.badgeStatus == .blockedBySystem {
                    Text("Allow badges in iOS Settings to show the task count on the app icon.")
                        .foregroundStyle(AccentTheme.secondaryText)
                } else if notifications.badgeStatus == .failed {
                    Label("The app badge could not be updated. Refresh the schedule to retry.", systemImage: "exclamationmark.triangle")
                }
            } footer: {
                Text("Each unfinished parent task counts once. Subtasks are included with their parent; zero hides the badge.").mobileFont(.footnote)
                    .foregroundStyle(AccentTheme.secondaryText)
                if badgeMode != DockBadgeMode.none.rawValue && (remindersUpdateBadge || summaryUpdatesBadge) {
                    Text("When Momentum is closed, notifications use the badge count calculated when they were scheduled.").mobileFont(.footnote)
                        .foregroundStyle(AccentTheme.secondaryText)
                }
            }

            Section {
                if let status = notifications.status {
                    if let horizon = status.horizonEndMs {
                        LabeledContent("Planning Through") {
                            // The core horizon is exclusive; show the final planned local date.
                            Text(Date(timeIntervalSince1970: Double(horizon > 0 ? horizon - 1 : 0) / 1_000)
                                .formatted(date: .abbreviated, time: .omitted))
                                .foregroundStyle(AccentTheme.secondaryText)
                        }
                    }
                    if status.overflowReminders > 0 || status.overflowSummaries > 0 {
                        Text("Some notifications are outside the available schedule. Open Momentum regularly to keep reminders up to date.")
                            .foregroundStyle(AccentTheme.secondaryText)
                    }
                    if !status.issues.isEmpty {
                        Label("Notifications could not be fully updated. Retry to reconcile the schedule.", systemImage: "exclamationmark.triangle")
                    }
                }
                Button {
                    Task { await notifications.refresh() }
                } label: {
                    Label {
                        Text("Refresh Schedule").foregroundStyle(Color.primary)
                    } icon: {
                        Image(systemName: "arrow.clockwise").foregroundStyle(.tint)
                    }
                }
                .disabled(notifications.isRefreshing)
            } header: { Text("Schedule").mobileFont(.footnote).foregroundStyle(AccentTheme.secondaryText) }
            footer: {
                Text("Momentum plans a limited window ahead. Permission is required for delivery.").mobileFont(.footnote)
                    .foregroundStyle(AccentTheme.secondaryText)
            }
        }
        .navigationTitle("Notifications")
        .momentumNavigationCanvas()
        .navigationBarTitleDisplayMode(.inline)
        .sheet(isPresented: $editingSummaryTime) {
            NavigationStack {
                Form {
                    DatePicker("Time", selection: $summaryTimeDraft, displayedComponents: .hourAndMinute)
                        .datePickerStyle(.wheel)
                        .accessibilityIdentifier("notifications-summary-time-picker")
                }
                .navigationTitle("Morning Summary")
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) {
                        Button("Cancel", systemImage: "xmark") { editingSummaryTime = false }
                    }
                    ToolbarItem(placement: .confirmationAction) {
                        Button("Save", systemImage: "checkmark") {
                            summaryTime.wrappedValue = summaryTimeDraft
                            editingSummaryTime = false
                        }
                    }
                }
            }
            .presentationDetents([.medium])
            .presentationDragIndicator(.visible)
        }
        .task { await model.start(); await notifications.refresh() }
        .task(id: PreviewInput(sections: sections, revision: model.revision, ready: model.worker != nil)) {
            summaryPreview = await model.worker?.morningSummaryPreview(sections: UInt8(clamping: sections))
        }
        .onChange(of: preferences) {
            Task { await model.preferencesChanged() }
        }
    }

    private struct PreviewInput: Equatable {
        let sections: Int
        let revision: Int
        let ready: Bool
    }

    private struct NotificationPreferences: Equatable {
        let summaryEnabled: Bool
        let hour: Int
        let minute: Int
        let weekdays: Int
        let sections: Int
        let badgeMode: String
        let remindersUpdateBadge: Bool
        let summaryUpdatesBadge: Bool
    }

    private var preferences: NotificationPreferences {
        NotificationPreferences(summaryEnabled: summaryEnabled, hour: hour, minute: minute, weekdays: weekdays,
            sections: sections, badgeMode: badgeMode, remindersUpdateBadge: remindersUpdateBadge,
            summaryUpdatesBadge: summaryUpdatesBadge)
    }

    private var daySelectionLabel: LocalizedStringKey {
        switch weekdays {
        case 0: "None"
        case 31: "Weekdays"
        case 127: "Every Day"
        default: "Custom"
        }
    }

    private var summaryDays: some View {
        let calendar = Calendar.current
        let symbols = calendar.weekdaySymbols
        return Form {
            Section {
                Button("Every Day") { weekdays = 127 }
                Button("Weekdays") { weekdays = 31 }
            }
            Section {
                ForEach(0..<7) { offset in
                    let index = (calendar.firstWeekday - 1 + offset) % 7
                    let bit = 1 << ((index + 6) % 7)
                    Toggle(symbols[index], isOn: selected($weekdays, bit: bit))
                        .accessibilityIdentifier("notifications-summary-day-\(bit)")
                }
            }
        }
        .navigationTitle("Days")
    }

    private func selected(_ mask: Binding<Int>, bit: Int) -> Binding<Bool> {
        Binding(get: { mask.wrappedValue & bit != 0 }, set: { enabled in
            let current = mask.wrappedValue
            mask.wrappedValue = enabled ? current | bit : current & ~bit
        })
    }

    private var permissionLabel: LocalizedStringKey {
        switch notifications.status?.authorization {
        case .authorized: "Allowed"
        case .provisional, .ephemeral: "Quiet Delivery"
        case .denied: "Off in iOS Settings"
        case .notDetermined: "Not Enabled"
        case .unavailable: "Unavailable"
        case nil: "Checking…"
        }
    }

    private var summaryTime: Binding<Date> {
        Binding(get: {
            let time = Preferences(model.defaults).morningSummaryTime
            return Calendar.current.date(from: DateComponents(year: 2001, month: 1, day: 1,
                hour: Int(time.hour), minute: Int(time.minute))) ?? Date()
        }, set: {
            let parts = Calendar.current.dateComponents([.hour, .minute], from: $0)
            hour = parts.hour ?? 8
            minute = parts.minute ?? 0
        })
    }
}
