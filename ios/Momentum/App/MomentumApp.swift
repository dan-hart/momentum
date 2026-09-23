// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumMobile
import CoreSpotlight
import Foundation
import SwiftUI
import UIKit

@main struct MomentumApp: App {
    @UIApplicationDelegateAdaptor(MobileAppDelegate.self) private var appDelegate
    @State private var model = MobileAppModel.shared
    @Environment(\.scenePhase) private var scenePhase
    @State private var clockRevision = 0

    var body: some Scene {
        WindowGroup {
            RootView()
                .onOpenURL { url in Task { await model.handle(url: url) } }
                .onContinueUserActivity(CSSearchableItemActionType) { activity in
                    guard let id = MobileSpotlightActivity.taskID(from: activity) else { return }
                    Task { await model.openSpotlightTask(id) }
                }
                .momentumAccent()
                .environment(model)
                .environment(model.sync)
                .defaultAppStorage(model.defaults)
                .onChange(of: scenePhase, initial: true) { _, phase in
                    model.setLowPowerMode(ProcessInfo.processInfo.isLowPowerModeEnabled)
                    model.sync.setForeground(phase == .active)
                    model.setForeground(phase == .active)
                }
                .task(id: "\(scenePhase)-\(clockRevision)") {
                    guard scenePhase == .active else { return }
                    await model.foreground()
                    while !Task.isCancelled {
                        guard let next = MobileLifecycle.nextDay(after: Date()) else { return }
                        do { try await Task.sleep(for: .seconds(max(1, next.timeIntervalSinceNow))) }
                        catch { return }
                        await model.foreground()
                    }
                }
                .onReceive(NotificationCenter.default.publisher(for: UIApplication.significantTimeChangeNotification)) { _ in
                    clockRevision += 1
                }
                .onReceive(NotificationCenter.default.publisher(for: .NSProcessInfoPowerStateDidChange)
                    .receive(on: RunLoop.main)) { _ in
                    model.setLowPowerMode(ProcessInfo.processInfo.isLowPowerModeEnabled)
                    Task {
                        await NotificationBackgroundRefresh.updateSchedule(
                            enabled: model.shouldScheduleNotificationBackgroundRefresh
                        )
                    }
                }
        }
        .commands {
            if model.platformCapabilities.showsKeyboardShortcuts {
                MobileKeyboardCommands(model: model)
            }
        }
    }
}
