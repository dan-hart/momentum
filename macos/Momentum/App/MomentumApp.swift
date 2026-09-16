// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
import AppKit
import MomentumCore
import MomentumKit
import SwiftUI

@main
struct MomentumApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var delegate
    @State private var state: AppState

    init() {
        Preferences.migrateLegacyDomain()
        Preferences.register()
        Preferences.migrateSyncMethod()
        let engine: Engine
        let isDemo = ProcessInfo.processInfo.environment["MOMENTUM_DEMO"] != nil
        if isDemo {
            #if DEBUG
            // Deterministic design previews without changing the Mac's appearance.
            switch ProcessInfo.processInfo.environment["MOMENTUM_PREVIEW_APPEARANCE"] {
            case "light": NSApplication.shared.appearance = NSAppearance(named: .aqua)
            case "dark": NSApplication.shared.appearance = NSAppearance(named: .darkAqua)
            default: break
            }
            #endif
            // Screenshots and a first look: sample data in a temporary directory.
            let dir = FileManager.default.temporaryDirectory.appendingPathComponent("momentum-demo")
            engine = Engine.demo(dir: dir.path)
        } else {
            engine = Engine.open(dir: DataDirectory.url.path)
        }
        // The services wait for the delegate to attach notifications and Spotlight.
        let s = AppState(engine: engine, services: false, isDemo: isDemo)
        _state = State(initialValue: s)
        AppDelegate.shared = s
    }

    var body: some Scene {
        WindowGroup(id: "main") {
            ContentView()
                .environment(state)
                .momentumTypography()
                .frame(minWidth: 680, minHeight: 440)
        }
        .defaultSize(width: 1000, height: 700)
        .commands {
            MomentumCommands(state: state)
        }

        Window(String(localized: "Add Task"), id: "quick-add") {
            QuickAddWindow()
                .environment(state)
                .momentumTypography()
        }
        .windowResizability(.contentSize)
        .defaultPosition(.center)

        Settings {
            SettingsView()
                .environment(state)
                .momentumTypography()
        }

        MenuBarExtra(isInserted: Binding(
            get: { state.prefs.showInMenuBar },
            set: { UserDefaults.standard.set($0, forKey: PrefKey.showInMenuBar) }
        )) {
            MenuBarView()
                .environment(state)
                .momentumTypography()
        } label: {
            // The count is the point of the item; the icon alone when there is nothing left.
            Label(state.todayOpenCount == 0 ? "" : "\(state.todayOpenCount)",
                  systemImage: "checkmark.circle")
        }
    }
}
