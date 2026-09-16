// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// Reminders and the morning summary as User Notifications, with the same two buttons the
// GNOME app offers: Done and Snooze 1 hour. The Dock badge counts what is left today.
import AppKit
import MomentumCore
import MomentumKit
import UserNotifications

@MainActor
final class NotificationManager: NSObject, Notifier, UNUserNotificationCenterDelegate {
    private unowned let state: AppState
    private let center = UNUserNotificationCenter.current()
    private var authorized = false

    private enum Category {
        static let reminder = "reminder"
    }
    private enum Action {
        static let done = "done"
        static let snooze = "snooze"
    }

    init(state: AppState) {
        self.state = state
        super.init()
    }

    func register() {
        center.delegate = self
        let done = UNNotificationAction(identifier: Action.done, title: String(localized: "Done"))
        let snooze = UNNotificationAction(identifier: Action.snooze, title: String(localized: "Snooze 1 hour"))
        center.setNotificationCategories([
            UNNotificationCategory(identifier: Category.reminder, actions: [done, snooze],
                                   intentIdentifiers: [], options: [])
        ])
        center.requestAuthorization(options: [.alert, .sound, .badge]) { [weak self] granted, error in
            if let error { NSLog("notifications: \(error)") }
            Task { @MainActor in self?.authorized = granted }
        }
    }

    // MARK: Notifier

    func reminder(_ due: ReminderDue) {
        let content = UNMutableNotificationContent()
        content.title = due.title
        content.body = due.time.map { String(localized: "Due at \(Strings.time($0))") }
            ?? String(localized: "Reminder")
        content.categoryIdentifier = Category.reminder
        content.userInfo = ["taskId": due.taskId, "title": due.title]
        post(id: due.taskId, content)
    }

    func morningSummary(_ summary: MorningSummary) {
        let content = UNMutableNotificationContent()
        content.title = String(localized: "Good morning")
        var body = String(localized: "\(summary.total) tasks today")
        if summary.morning > 0 {
            body += String(localized: ", \(summary.morning) this morning")
        }
        if summary.tonight > 0 {
            body += String(localized: ", \(summary.tonight) tonight")
        }
        content.body = body
        post(id: "morning", content)
    }

    func updateBadge(_ count: UInt32) {
        NSApp.dockTile.badgeLabel = count == 0 ? nil : "\(count)"
    }

    private func post(id: String, _ content: UNMutableNotificationContent) {
        center.add(UNNotificationRequest(identifier: id, content: content, trigger: nil)) { error in
            if let error { NSLog("notification: \(error)") }
        }
    }

    // MARK: Responses

    nonisolated func userNotificationCenter(_ center: UNUserNotificationCenter,
                                            didReceive response: UNNotificationResponse) async {
        let info = response.notification.request.content.userInfo
        let taskId = info["taskId"] as? String
        let title = info["title"] as? String
        let action = response.actionIdentifier
        let delivered = response.notification.request.identifier
        await MainActor.run {
            switch action {
            case Action.done:
                if let taskId { state.completeFromNotification(taskId) }
            case Action.snooze:
                if let taskId { state.snooze(taskId, minutes: 60) }
            default:
                // Tapping the notification body opens the task in Search, as on GNOME.
                if let title { state.showSearch(for: title) }
                AppDelegate.showMainWindow()
            }
            // Reach for the shared centre here rather than carrying the parameter across
            // isolation domains; it is the same object.
            UNUserNotificationCenter.current().removeDeliveredNotifications(withIdentifiers: [delivered])
        }
    }

    /// Reminders are worth seeing even while Momentum is in front.
    nonisolated func userNotificationCenter(_ center: UNUserNotificationCenter,
                                            willPresent notification: UNNotification) async
        -> UNNotificationPresentationOptions {
        [.banner, .sound]
    }
}
