// SPDX-License-Identifier: GPL-3.0-or-later
// Verify compiled app resources without launching an app or reading catalog JSON.
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
    let singular = language == "en" ? ["day", "week", "month", "year"] : ["Tag", "Woche", "Monat", "Jahr"]
    let plural = language == "en" ? ["days", "weeks", "months", "years"] : ["Tage", "Wochen", "Monate", "Jahre"]
    for count: UInt32 in [0, 1, 2, 99] {
        // Keep the same typed interpolation used by both native repeat editors.
        let actual = [
            String(localized: "\(count) days", bundle: bundle, locale: locale),
            String(localized: "\(count) weeks", bundle: bundle, locale: locale),
            String(localized: "\(count) months", bundle: bundle, locale: locale),
            String(localized: "\(count) years", bundle: bundle, locale: locale),
        ]
        for index in actual.indices {
            checks += 1
            let expected = "\(count) \((count == 1 ? singular : plural)[index])"
            if actual[index] != expected {
                failures.append("\(language): expected '\(expected)', got '\(actual[index])'")
            }
        }
    }
}
for failure in failures { print(failure) }
print("\(checks) compiled repeat-interval checks; \(failures.count) failures")
exit(failures.isEmpty ? 0 : 1)
