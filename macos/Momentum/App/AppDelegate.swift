// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// The AppKit side of the app: URLs, the Dock menu, the Services entry, the system-wide
// shortcuts, notifications, login item, and staying alive after the last window closes.
import AppKit
import AppIntents
import CoreSpotlight
import MomentumCore
import MomentumKit
import ServiceManagement
import SwiftUI
import UserNotifications

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    static var shared: AppState?
    private var hotKeys: GlobalHotKeys?
    private var notifications: NotificationManager?
    private var spotlight: SpotlightIndexer?
    private let services = ServiceProvider()

    func applicationWillFinishLaunching(_ notification: Notification) {
        NSAppleEventManager.shared().setEventHandler(
            self, andSelector: #selector(handleURLEvent(_:reply:)),
            forEventClass: AEEventClass(kInternetEventClass), andEventID: AEEventID(kAEGetURL))
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        guard let state = Self.shared else { return }
        NSApp.servicesProvider = services
        let manager = NotificationManager(state: state)
        notifications = manager
        state.notifier = manager
        manager.register()
        let spotlight = SpotlightIndexer()
        self.spotlight = spotlight
        state.indexer = spotlight
        // Everything the app reaches outside itself is attached: start the services now.
        state.startServices()
        MomentumShortcuts.updateAppShortcutParameters()
        hotKeys = GlobalHotKeys(
            quickAdd: { [weak self] in self?.openQuickAdd() },
            show: { NSApp.activate(); Self.showMainWindow() })
        let show = state.handle(arguments: CommandLine.arguments)
        if CommandLine.arguments.contains("--quick-add") {
            openQuickAdd()
        } else if !show {
            if !state.prefs.runInBackground {
                // Launched only to run a command: do not linger.
                DispatchQueue.main.asyncAfter(deadline: .now() + 1) { NSApp.terminate(nil) }
            } else {
                NSApp.windows.forEach { if $0.identifier?.rawValue.hasPrefix("main") == true { $0.orderOut(nil) } }
            }
        }
        updateLoginItem()
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        !(Self.shared?.prefs.runInBackground ?? false)
    }

    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        if !flag { Self.showMainWindow() }
        return true
    }

    func applicationWillTerminate(_ notification: Notification) {
        Self.shared?.stopServices()
    }

    // MARK: URLs

    @objc private func handleURLEvent(_ event: NSAppleEventDescriptor, reply: NSAppleEventDescriptor) {
        guard let s = event.paramDescriptor(forKeyword: AEKeyword(keyDirectObject))?.stringValue,
              let url = URL(string: s) else { return }
        Self.shared?.handle(url: url)
        Self.showMainWindow()
    }

    /// Spotlight resumes Search using the task's current title. Stale results are ignored.
    func application(_ application: NSApplication, continue userActivity: NSUserActivity,
                     restorationHandler: @escaping ([any NSUserActivityRestoring]) -> Void) -> Bool {
        guard userActivity.activityType == CSSearchableItemActionType,
              let id = userActivity.userInfo?[CSSearchableItemActivityIdentifier] as? String
        else { return false }
        guard Self.shared?.showSearchForTask(id) == true else { return false }
        Self.showMainWindow()
        return true
    }

    func application(_ application: NSApplication, open urls: [URL]) {
        for url in urls { Self.shared?.handle(url: url) }
        Self.showMainWindow()
    }

    // MARK: Dock menu

    func applicationDockMenu(_ sender: NSApplication) -> NSMenu? {
        let menu = NSMenu()
        menu.addItem(withTitle: String(localized: "New Task"), action: #selector(dockNewTask), keyEquivalent: "").target = self
        menu.addItem(withTitle: String(localized: "Today"), action: #selector(dockToday), keyEquivalent: "").target = self
        menu.addItem(withTitle: String(localized: "Search"), action: #selector(dockSearch), keyEquivalent: "").target = self
        return menu
    }
    @objc private func dockNewTask() {
        openQuickAdd()
    }
    @objc private func dockToday() {
        Self.shared?.go(to: .today)
        Self.showMainWindow()
    }
    @objc private func dockSearch() {
        Self.shared?.go(to: .search)
        Self.showMainWindow()
    }

    // MARK: Windows

    static func showMainWindow() {
        NSApp.activate()
        if let w = NSApp.windows.first(where: { $0.identifier?.rawValue.hasPrefix("main") == true }) {
            w.makeKeyAndOrderFront(nil)
        } else {
            // The window group has no window (closed while running in the background).
            WindowOpener.shared.open?("main")
        }
    }
    static func openWindow(id: String) {
        WindowOpener.shared.open?(id)
    }
    func openQuickAdd() {
        NSApp.activate()
        if let w = NSApp.windows.first(where: { $0.identifier?.rawValue.hasPrefix("quick-add") == true }) {
            w.makeKeyAndOrderFront(nil)
        } else {
            WindowOpener.shared.open?("quick-add")
        }
    }

    /// "Keep running in the background" also means "start at login", as it does on GNOME.
    func updateLoginItem() {
        let want = Self.shared?.prefs.runInBackground ?? false
        let service = SMAppService.mainApp
        do {
            if want, service.status != .enabled {
                try service.register()
            } else if !want, service.status == .enabled {
                try service.unregister()
            }
        } catch {
            NSLog("login item: \(error)")
        }
    }
}

/// SwiftUI's `openWindow` action, captured once from a view so AppKit code can use it.
@MainActor
final class WindowOpener {
    static let shared = WindowOpener()
    var open: ((String) -> Void)?
}

/// The Services menu entry: "New Momentum Task" from selected text in any app.
final class ServiceProvider: NSObject {
    @objc func newTaskFromSelection(_ pboard: NSPasteboard, userData: String, error: AutoreleasingUnsafeMutablePointer<NSString>) {
        guard let text = pboard.string(forType: .string), !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            error.pointee = String(localized: "No text selected") as NSString
            return
        }
        Task { @MainActor in
            AppDelegate.shared?.addFromText(text)
            AppDelegate.showMainWindow()
        }
    }
}
