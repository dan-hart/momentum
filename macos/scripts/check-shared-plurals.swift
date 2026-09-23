// SPDX-License-Identifier: GPL-3.0-or-later
// Exercise compiled shared resources through the same typed Foundation lookups as Strings.
import Foundation

guard CommandLine.arguments.count == 2 else {
    fatalError("Pass the directory containing compiled en.lproj and de.lproj resources")
}
let root = URL(fileURLWithPath: CommandLine.arguments[1])
var failures = [String]()
var checks = 0
for language in ["en", "de"] {
    guard let bundle = Bundle(url: root.appendingPathComponent(language + ".lproj")) else {
        fatalError("Missing compiled \(language) resources")
    }
    let locale = Locale(identifier: language)
    for count: UInt32 in [0, 1, 2, 99] {
        let wide = UInt64(count)
        let actual = [
            String(localized: "\(wide) days ago", bundle: bundle, locale: locale),
            String(localized: "\(wide) hours ago", bundle: bundle, locale: locale),
            String(localized: "\(wide) minutes ago", bundle: bundle, locale: locale),
            String(localized: "\(wide) seconds ago", bundle: bundle, locale: locale),
            String(localized: "\(count) completed tasks archived", bundle: bundle, locale: locale),
            String(localized: "\(count) linked devices, not synced yet", bundle: bundle, locale: locale),
            String(localized: "\(count) tasks added", bundle: bundle, locale: locale),
            String(localized: "\(count) tasks completed", bundle: bundle, locale: locale),
            String(localized: "\(count) tasks completed and archived", bundle: bundle, locale: locale),
            String(localized: "\(count) tasks deleted", bundle: bundle, locale: locale),
            String(localized: "\(count) tasks moved to next week", bundle: bundle, locale: locale),
            String(localized: "\(count) tasks moved to the morning", bundle: bundle, locale: locale),
            String(localized: "\(count) tasks moved to today", bundle: bundle, locale: locale),
            String(localized: "\(count) tasks moved to tomorrow", bundle: bundle, locale: locale),
            String(localized: "\(count) tasks moved to tonight", bundle: bundle, locale: locale),
            String(localized: "\(count) tasks planned for today", bundle: bundle, locale: locale),
            String(localized: "\(count) tasks tagged", bundle: bundle, locale: locale),
            String(localized: "Repeats every \(count) days", bundle: bundle, locale: locale),
            String(localized: "Snoozed for \(count) minutes", bundle: bundle, locale: locale),
            String(localized: "Tasks due in the next \(count) days appear here. Set a due day in a task's details.", bundle: bundle, locale: locale),
            String(localized: "You completed \(count) tasks. Time to switch off.", bundle: bundle, locale: locale),
        ]
        let one = count == 1
        let english = [
            "\(count) \(one ? "day" : "days") ago",
            "\(count) \(one ? "hour" : "hours") ago",
            "\(count) \(one ? "minute" : "minutes") ago",
            "\(count) \(one ? "second" : "seconds") ago",
            "\(count) completed \(one ? "task" : "tasks") archived",
            "\(count) linked \(one ? "device" : "devices"), not synced yet",
        ] + ["added", "completed", "completed and archived", "deleted", "moved to next week",
             "moved to the morning", "moved to today", "moved to tomorrow", "moved to tonight",
             "planned for today", "tagged"].map { "\(count) \(one ? "task" : "tasks") \($0)" } + [
            "Repeats every \(count) \(one ? "day" : "days")",
            "Snoozed for \(count) \(one ? "minute" : "minutes")",
            "Tasks due in the next \(count) \(one ? "day" : "days") appear here. Set a due day in a task's details.",
            "You completed \(count) \(one ? "task" : "tasks"). Time to switch off.",
        ]
        let german = [
            "vor \(count) \(one ? "Tag" : "Tagen")",
            "vor \(count) \(one ? "Stunde" : "Stunden")",
            "vor \(count) \(one ? "Minute" : "Minuten")",
            "vor \(count) \(one ? "Sekunde" : "Sekunden")",
            "\(count) erledigte \(one ? "Aufgabe" : "Aufgaben") archiviert",
            "\(count) \(one ? "gekoppeltes Gerät" : "gekoppelte Geräte"), noch nicht synchronisiert",
        ] + ["hinzugefügt", "erledigt", "erledigt und archiviert", "gelöscht", "auf nächste Woche verschoben",
             "auf den Morgen verschoben", "auf heute verschoben", "auf morgen verschoben", "auf heute Abend verschoben",
             "für heute geplant", "mit Schlagwort versehen"].map { "\(count) \(one ? "Aufgabe" : "Aufgaben") \($0)" } + [
            "Wiederholt sich \(one ? "jeden" : "alle") \(count) \(one ? "Tag" : "Tage")",
            "Erneute Erinnerung in \(count) \(one ? "Minute" : "Minuten")",
            "Hier erscheinen Aufgaben, die in \(one ? "dem nächsten" : "den nächsten") \(count) \(one ? "Tag" : "Tagen") fällig sind. Lege das Fälligkeitsdatum in den Aufgabendetails fest.",
            "Du hast \(count) \(one ? "Aufgabe" : "Aufgaben") erledigt. Zeit zum Abschalten.",
        ]
        let expected = language == "en" ? english : german
        precondition(actual.count == expected.count)
        for index in actual.indices {
            checks += 1
            if actual[index] != expected[index] {
                failures.append("\(language): expected '\(expected[index])', got '\(actual[index])'")
            }
        }
    }
}
for failure in failures { print(failure) }
print("\(checks) compiled shared plural checks; \(failures.count) failures")
exit(failures.isEmpty ? 0 : 1)
