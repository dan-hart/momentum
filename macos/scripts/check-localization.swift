// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
// Run with: swift macos/scripts/check-localization.swift /path/to/Momentum.app
// Reads the built bundles without launching the application.
import Foundation

func require(_ condition: @autoclosure () -> Bool, _ message: String) {
    guard condition() else { fatalError(message) }
}
func germanBundle(_ bundle: Bundle) -> Bundle {
    guard let path = bundle.path(forResource: "de", ofType: "lproj"), let german = Bundle(path: path) else {
        fatalError("Missing compiled German localization in \(bundle.bundlePath)")
    }
    return german
}

require(CommandLine.arguments.count == 2, "Pass the path to the built Momentum.app")
let app = Bundle(path: CommandLine.arguments[1])!
let package = Bundle(url: app.resourceURL!.appendingPathComponent("MomentumKit_MomentumKit.bundle"))!
let appGerman = germanBundle(app)
let packageGerman = germanBundle(package)
let locale = Locale(identifier: "de")
let one: UInt32 = 1
let three: UInt32 = 3
let seven: UInt32 = 7

require(String(localized: "New Task…", bundle: appGerman, locale: locale) == "Neue Aufgabe…", "App menu translation missing")
require(String(localized: "Show More (\(seven) remaining)", bundle: appGerman, locale: locale) == "Mehr anzeigen (noch 7)", "App typed interpolation failed")
require(String(localized: "\(one) tasks today", bundle: appGerman, locale: locale) == "1 Aufgabe heute", "App singular failed")
require(String(localized: "\(three) tasks today", bundle: appGerman, locale: locale) == "3 Aufgaben heute", "App plural failed")
require(String(localized: "\(one) tasks deleted", bundle: packageGerman, locale: locale) == "1 Aufgabe gelöscht", "Package singular failed")
require(String(localized: "\(three) tasks deleted", bundle: packageGerman, locale: locale) == "3 Aufgaben gelöscht", "Package plural failed")
require(appGerman.localizedString(forKey: "Create task ${taskTitle}", value: nil, table: "Localizable") == "Aufgabe ${taskTitle} erstellen", "App Intent parameter was lost")
require(appGerman.localizedString(forKey: "NSLocalNetworkUsageDescription", value: nil, table: "InfoPlist") == "Momentum synchronisiert sich über das lokale Netzwerk mit deinen anderen Geräten.", "Local network usage description missing")
require(appGerman.localizedString(forKey: "NSUserNotificationsUsageDescription", value: nil, table: "InfoPlist") == "Erinnerungen und eine morgendliche Übersicht über die Aufgaben des Tages.", "Notification usage description missing")
require(String(localized: "Morning & Night", bundle: appGerman, locale: locale) == "Morgen & Abend", "Default grouping translation missing")
require(String(localized: "Evening", bundle: packageGerman, locale: locale) == "Abend", "Evening group translation missing")
require(String(localized: "Group By", bundle: appGerman, locale: locale) == "Gruppieren nach", "Grouping menu translation missing")
require(String(localized: "No estimate", bundle: packageGerman, locale: locale) == "Keine Schätzung", "Grouping fallback translation missing")
require(String(localized: "16–30 min", bundle: packageGerman, locale: locale) == "16–30 Min.", "Estimate group translation missing")
print("German app and package bundles: static strings, typed interpolation, singular/plural, App Intent parameter, and permission descriptions passed.")
