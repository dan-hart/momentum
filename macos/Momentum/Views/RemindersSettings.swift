// SPDX-License-Identifier: GPL-3.0-or-later
import Accessibility
import AppKit
import MomentumKit
import SwiftUI

struct RemindersSettings: SwiftUI.View {
    @Environment(AppState.self) private var model
    var announce: (String) -> Void = { AccessibilityNotification.Announcement($0).post() }
    private var state: RemindersImport { model.reminders }

    var body: some SwiftUI.View {
        Form {
            Section {
                if !state.connected {
                    Button("Connect Apple Reminders") { Task { await state.connect() } }
                        .disabled(state.busy)
                        .accessibilityIdentifier("reminders-connect")
                } else {
                    Picker("Reminders List", selection: Binding(get: { state.selectedListID }, set: { state.selectList($0) })) {
                        Text("Choose a List").tag("")
                        if !state.selectedListID.isEmpty && !state.lists.contains(where: { $0.id == state.selectedListID }) {
                            Text("List Unavailable").tag(state.selectedListID)
                        }
                        ForEach(state.lists) { list in Text(verbatim: list.title).tag(list.id) }
                    }
                    .pickerStyle(.menu)
                    .disabled(state.busy)
                    .accessibilityIdentifier("reminders-list")
                    Button("Import Now") { Task { await state.importNow() } }
                        .disabled(state.busy || state.selectedListID.isEmpty || state.issue == .missingList)
                        .accessibilityIdentifier("reminders-import")
                    Toggle("Automatically Import New Reminders", isOn: Binding(get: { state.automatic }, set: { state.setAutomatic($0) }))
                        .disabled((state.selectedListID.isEmpty || state.issue == .missingList) && !state.automatic)
                        .accessibilityIdentifier("reminders-automatic")
                    if state.lists.isEmpty { Text("No reminders lists are available. Create a list in Apple Reminders, then return here.") }
                }
            } footer: {
                Text("Automatic import checks your chosen list while Momentum is active and when you return to it. Imports pause when another app is active or Momentum is closed. Later edits or completions in either app are not synchronized.")
            }
            Section {
                DisclosureGroup("What Gets Imported") {
                    Text("Import incomplete reminders into Inbox. Titles, notes, links, and due days are copied. Times, alerts, subtasks, and repeat rules are not copied. Apple Reminders stays unchanged.")
                }
            }
            if state.automatic && !state.connected {
                Section {
                    Button("Turn Off Automatic Import") { state.setAutomatic(false) }
                }
            }
            Section {
                if state.busy {
                    ProgressView("Checking Reminders…")
                } else if let count = state.lastImportCount {
                    Text("Imported \(count) new reminders.")
                        .accessibilityIdentifier("reminders-result")
                }
                if let issue = state.issue {
                    Text(message(for: issue)).foregroundStyle(.secondary)
                        .accessibilityIdentifier("reminders-error")
                    if issue == .permission {
                        Button("Open System Settings") {
                            if let url = NSWorkspace.shared.urlForApplication(withBundleIdentifier: "com.apple.systempreferences") {
                                NSWorkspace.shared.openApplication(at: url, configuration: .init(), completionHandler: nil)
                            }
                        }
                    } else if issue != .unavailable {
                        Button("Try Again") { Task { await state.retry() } }
                    }
                }
            } footer: {
                Text("Previously imported reminders are skipped, even if you delete their Momentum tasks. Turning this off keeps imported tasks and your selected list.")
            }
        }
        .formStyle(.grouped)
        .task { await state.loadLists() }
        .onChange(of: state.feedbackRevision) { _, _ in
            // An announcement is an event, so repeated identical attempts are spoken too.
            // The coordinator publishes the error rather than a partial-success count.
            switch state.feedback {
            case .issue(let issue): announce(String(localized: message(for: issue)))
            case .result(let count): announce(String(localized: "Imported \(count) new reminders."))
            case nil: break
            }
        }
    }

    private func message(for issue: RemindersImportIssue) -> LocalizedStringResource {
        switch issue {
        case .permission: "Allow Reminders access in System Settings → Privacy & Security → Reminders to import your list."
        case .missingList: "The selected list is unavailable. Choose another list."
        case .fetch: "Reminders could not be loaded. Try again."
        case .save: "Momentum could not save a reminder. Retry Import Now."
        case .unavailable: "Apple Reminders is unavailable in this preview or test session."
        }
    }
}
