// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumKit
import MomentumMobile
import SwiftUI
import UIKit

struct NotificationSettings: View {
    @Environment(MobileAppModel.self) private var model
    @AppStorage(PrefKey.morningSummaryEnabled) private var summaryEnabled = false
    @AppStorage(PrefKey.morningSummaryHour) private var hour = 8
    @AppStorage(PrefKey.morningSummaryMinute) private var minute = 0
    @AppStorage(PrefKey.dockBadgeMode) private var badgeMode = DockBadgeMode.dueToday.rawValue
    @State private var editingSummaryTime = false
    @State private var summaryTimeDraft = Date()

    private var notifications: MobileNotifications { model.notifications }

    var body: some View {
        Form {
            Section {
                LabeledContent { Text(permissionLabel).foregroundStyle(AccentTheme.secondaryText) } label: {
                    SettingsLabel("Permission", symbol: "bell.badge.fill")
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
            } header: { Text("Delivery").foregroundStyle(AccentTheme.secondaryText) }
            footer: {
                Text("Reminders work offline. Focus, notification settings, and iOS may delay delivery.")
                    .foregroundStyle(AccentTheme.secondaryText)
            }

            Section {
                Toggle(isOn: $summaryEnabled) {
                    SettingsLabel("Morning Summary", symbol: "sun.max.fill")
                }.accessibilityIdentifier("notifications-summary")
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
                }
            } header: { Text("Daily Overview").foregroundStyle(AccentTheme.secondaryText) }
            footer: {
                Text("An optional overview of today's tasks. Task reminders are configured separately on each task.")
                    .foregroundStyle(AccentTheme.secondaryText)
            }

            Section {
                Picker(selection: $badgeMode) {
                    ForEach(DockBadgeMode.allCases) { mode in Text(mode.label).tag(mode.rawValue) }
                } label: { SettingsLabel("App Badge", symbol: "app.badge.fill") }
                .accessibilityIdentifier("notifications-badge-mode")
                if notifications.badgeStatus == .blockedBySystem {
                    Text("Allow badges in iOS Settings to show the task count on the app icon.")
                        .foregroundStyle(AccentTheme.secondaryText)
                } else if notifications.badgeStatus == .failed {
                    Label("The app badge could not be updated. Refresh the schedule to retry.", systemImage: "exclamationmark.triangle")
                }
            } footer: {
                Text("Each unfinished parent task counts once. Subtasks are included with their parent; zero hides the badge.")
                    .foregroundStyle(AccentTheme.secondaryText)
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
            } header: { Text("Schedule").foregroundStyle(AccentTheme.secondaryText) }
            footer: {
                Text("Momentum plans a limited window ahead. Permission is required for delivery.")
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
        .onChange(of: "\(summaryEnabled)-\(hour)-\(minute)-\(badgeMode)") {
            Task { await model.preferencesChanged() }
        }
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
