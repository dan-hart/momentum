// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
import Foundation

guard CommandLine.arguments.count == 3 else {
    fatalError("Pass the compiled catalog directory and app, ios, or shared")
}
let root = URL(fileURLWithPath: CommandLine.arguments[1])
let scope = CommandLine.arguments[2]
let common = [
    "Notes copied": "Notizen kopiert",
    "Overdue": "Überfällig",
    "Recurring preview": "Wiederholungsvorschau",
    "Edit repeat schedule": "Wiederholungsplan bearbeiten",
]
let settings = [
    "App settings": "App-Einstellungen",
    "Include app settings": "App-Einstellungen einschließen",
    "Restore included app settings": "Enthaltene App-Einstellungen wiederherstellen",
    "Sync app settings": "App-Einstellungen synchronisieren",
    "Use this device’s settings": "Einstellungen dieses Geräts verwenden",
    "Use synced settings": "Synchronisierte Einstellungen verwenden",
    "Copy to Momentum folder?": "In Momentum-Ordner kopieren?",
]
var expected = common
if scope == "app" {
    expected["Dynamic empty-state messages"] = "Dynamische Nachrichten bei leeren Listen"
    expected["Settings"] = "Einstellungen"
}
if scope != "shared" {
    expected.merge(settings) { _, value in value }
}
if scope == "ios" {
    expected["Momentum plans the task for today when it next opens."] = "Momentum plant die Aufgabe beim nächsten Öffnen für heute."
    expected["Added a shared task"] = "Eine geteilte Aufgabe hinzugefügt"
}
var failures = [String]()
var checks = 0
for language in ["en", "de"] {
    guard let bundle = Bundle(url: root.appendingPathComponent(language + ".lproj")) else {
        fatalError("Missing compiled \(language) resources")
    }
    let locale = Locale(identifier: language)
    for (key, translation) in expected {
        let value = bundle.localizedString(forKey: key, value: nil, table: nil)
        let wanted = language == "en" ? key : translation
        checks += 1
        if value != wanted { failures.append("\(language): expected '\(wanted)', got '\(value)'") }
    }
    if scope == "shared" {
        for days in [7, 30] {
            let value = String(localized: "\(days) days", bundle: bundle, locale: locale)
            let wanted = "\(days) \(language == "en" ? "days" : "Tage")"
            checks += 1
            if value != wanted { failures.append("\(language): expected '\(wanted)', got '\(value)'") }
        }
    }
    if scope == "ios" {
        let count = 2
        let value = String(localized: "Added \(count) shared tasks", bundle: bundle, locale: locale)
        let wanted = language == "en" ? "Added 2 shared tasks" : "2 geteilte Aufgaben hinzugefügt"
        checks += 1
        if value != wanted { failures.append("\(language): expected '\(wanted)', got '\(value)'") }
    }
}
for failure in failures { print(failure) }
print("\(checks) compiled native feature checks; \(failures.count) failures")
exit(failures.isEmpty ? 0 : 1)
