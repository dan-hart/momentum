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
    private var windowObserver: NSObjectProtocol?

    /// The one main window's identifier begins with its scene id ("main-AppWindow-1").
    static func isMainWindow(_ w: NSWindow) -> Bool {
        w.identifier?.rawValue.hasPrefix("main") == true
    }

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
        // Automatic sync may pause while the main window is closed (Settings › Sync).
        windowObserver = NotificationCenter.default.addObserver(
            forName: NSWindow.willCloseNotification, object: nil, queue: .main
        ) { note in
            guard let w = note.object as? NSWindow, Self.isMainWindow(w) else { return }
            Task { @MainActor in Self.shared?.setMainWindowVisible(false) }
        }
        let show = state.handle(arguments: CommandLine.arguments)
        if CommandLine.arguments.contains("--quick-add") {
            openQuickAdd()
        } else if !show {
            if !state.prefs.runInBackground {
                // Launched only to run a command: do not linger.
                DispatchQueue.main.asyncAfter(deadline: .now() + 1) { NSApp.terminate(nil) }
            } else {
                NSApp.windows.forEach { if Self.isMainWindow($0) { $0.orderOut(nil) } }
                state.setMainWindowVisible(false)
            }
        }
        updateLoginItem()
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        !(Self.shared?.prefs.runInBackground ?? false)
    }

    /// A Dock click with no visible window: show the main window ourselves and tell AppKit
    /// the reopen is handled. The main scene is a single `Window`, so even a reopen that
    /// SwiftUI also answers brings the same window forward rather than a second one.
    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        if flag { return true }
        return !Self.showMainWindow()
    }

    func applicationDidBecomeActive(_ notification: Notification) {
        Self.shared?.setApplicationActive(true)
    }

    func applicationWillResignActive(_ notification: Notification) {
        Self.shared?.setApplicationActive(false)
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

    /// Returns false only when no main window exists and SwiftUI's `openWindow` has not been
    /// captured yet, so the caller can leave the window to the system.
    @discardableResult
    static func showMainWindow() -> Bool {
        NSApp.activate()
        defer { Self.shared?.setMainWindowVisible(true) }
        if let w = NSApp.windows.first(where: isMainWindow) {
            w.makeKeyAndOrderFront(nil)
            return true
        }
        // The window was closed while running in the background: reopen the one scene.
        guard let open = WindowOpener.shared.open else { return false }
        open("main")
        return true
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
