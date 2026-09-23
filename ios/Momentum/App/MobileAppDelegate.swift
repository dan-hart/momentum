// SPDX-License-Identifier: GPL-3.0-or-later
import AppIntents
import BackgroundTasks
import MomentumMobile
import UIKit
import UserNotifications

final class MobileAppDelegate: NSObject, UIApplicationDelegate, UNUserNotificationCenterDelegate {
    /// UserNotifications owns this one-shot Objective-C callback. We transfer it to
    /// the main actor so UIKit can finish background state restoration on its thread.
    private struct MainActorCompletion: @unchecked Sendable {
        let call: () -> Void
    }

    private static let doneAction = "momentum.notification.done.v1"
    private static let snoozeAction = "momentum.notification.snooze.v1"

    func application(_ application: UIApplication,
                     didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]? = nil) -> Bool {
        NotificationBackgroundRefresh.register()
        let arguments = ProcessInfo.processInfo.arguments
        if !arguments.contains("--demo") { MomentumShortcuts.updateAppShortcutParameters() }
        guard !arguments.contains("--demo") else { return true }
        let center = UNUserNotificationCenter.current()
        center.delegate = self
        center.setNotificationCategories([
            UNNotificationCategory(identifier: SystemNotificationCenter.reminderCategory, actions: [
                UNNotificationAction(identifier: Self.doneAction, title: String(localized: "Done"),
                    options: [], icon: UNNotificationActionIcon(systemImageName: "checkmark.circle.fill")),
                UNNotificationAction(identifier: Self.snoozeAction, title: String(localized: "Snooze 1 hour"),
                    options: [], icon: UNNotificationActionIcon(systemImageName: "clock.arrow.circlepath"))
            ], intentIdentifiers: []),
            UNNotificationCategory(identifier: SystemNotificationCenter.summaryCategory,
                actions: [], intentIdentifiers: [])
        ])
        return true
    }

    func applicationDidEnterBackground(_ application: UIApplication) {
        Task { @MainActor in
            await NotificationBackgroundRefresh.updateSchedule(
                enabled: MobileAppModel.shared.shouldScheduleNotificationBackgroundRefresh
            )
        }
    }

    nonisolated func userNotificationCenter(_ center: UNUserNotificationCenter,
                                            willPresent notification: UNNotification) async -> UNNotificationPresentationOptions {
        guard Self.route(for: notification.request) != nil else { return [] }
        return [.banner, .list, .sound]
    }

    nonisolated func userNotificationCenter(_: UNUserNotificationCenter,
                                            didReceive response: UNNotificationResponse,
                                            withCompletionHandler completionHandler: @escaping () -> Void) {
        let completion = MainActorCompletion(call: completionHandler)
        let request = response.notification.request
        guard let route = Self.route(for: request) else {
            Task { @MainActor in completion.call() }
            return
        }
        let identifier = request.identifier
        let action: TaskNotificationAction?
        switch response.actionIdentifier {
        case UNNotificationDefaultActionIdentifier: action = nil
        case Self.doneAction: action = .complete
        case Self.snoozeAction: action = .snooze
        default:
            Task { @MainActor in completion.call() }
            return
        }
        Task { @MainActor in
            defer { completion.call() }
            let handled = await MobileAppModel.shared.handleNotification(route, action: action, deliveryID: identifier)
            if handled { UNUserNotificationCenter.current().removeDeliveredNotifications(withIdentifiers: [identifier]) }
        }
    }

    private nonisolated static func route(for request: UNNotificationRequest) -> NotificationRoute? {
        let metadata = request.content.userInfo.reduce(into: [String: String]()) { result, item in
            if let key = item.key as? String, let value = item.value as? String { result[key] = value }
        }
        guard NotificationIdentity.decode(identifier: request.identifier, metadata: metadata) != nil,
              let route = SystemNotificationCenter.decodeRoute(metadata: metadata) else { return nil }
        switch route {
        case .reminder where request.content.categoryIdentifier == SystemNotificationCenter.reminderCategory: return route
        case .summary where request.content.categoryIdentifier == SystemNotificationCenter.summaryCategory: return route
        default: return nil
        }
    }
}
