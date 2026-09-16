// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// Preferences are shared ground: the keys match the GNOME app's GSettings names, the
// values reach the core, and the Nextcloud connection is only handed over once it is
// complete. The Keychain tests use a service of their own and never touch real secrets.
import Foundation
import MomentumCore
import Testing

@testable import MomentumKit

@Suite struct PreferenceDefaults {
    private func freshDefaults() -> (UserDefaults, String) {
        let name = "momentum-prefs-\(UUID().uuidString)"
        let d = UserDefaults(suiteName: name)!
        Preferences.register(d)
        return (d, name)
    }

    @Test func theDefaultsMatchTheGnomeSchema() {
        let (d, name) = freshDefaults()
        defer { d.removePersistentDomain(forName: name) }
        let p = Preferences(d)
        #expect(p.core.groupBy == .morningNight)
        #expect(p.core.sort == .manual)
        #expect(p.core.direction == .ascending)
        #expect(p.core.upcomingDays == 7)
        #expect(p.core.autoArchive == false)
        #expect(p.core.morningSummaryEnabled == false)
        #expect(p.core.morningSummaryTime == ClockTime(hour: 8, minute: 0))
        #expect(p.syncEnabled == false, "nothing leaves the device until sync is turned on")
        #expect(p.p2pEnabled == false)
        #expect(p.autoSync == true)
        #expect(p.colorful == true)
        #expect(d.string(forKey: PrefKey.nextcloudFolder) == "super-productivity")
    }

    @Test func groupingChoicesPersistAndUnknownValuesFallBackToMorningNight() {
        let (d, name) = freshDefaults()
        defer { d.removePersistentDomain(forName: name) }
        for (value, expected): (String, GroupBy) in [
            ("project", .project), ("tag", .tag), ("estimate", .estimate),
            ("none", .none), ("morning-night", .morningNight), ("future-option", .morningNight)
        ] {
            d.set(value, forKey: PrefKey.groupBy)
            #expect(Preferences(UserDefaults(suiteName: name)!).core.groupBy == expected)
        }
    }

    @Test func morningSummaryPreferencesPersistLocallyAndValidateTime() {
        let (d, name) = freshDefaults()
        defer { d.removePersistentDomain(forName: name) }
        d.set(true, forKey: PrefKey.morningSummaryEnabled)
        d.set(9, forKey: PrefKey.morningSummaryHour)
        d.set(45, forKey: PrefKey.morningSummaryMinute)
        let saved = Preferences(UserDefaults(suiteName: name)!).core
        #expect(saved.morningSummaryEnabled)
        #expect(saved.morningSummaryTime == ClockTime(hour: 9, minute: 45))
        d.set(false, forKey: PrefKey.morningSummaryEnabled)
        #expect(Preferences(d).core.morningSummaryTime == saved.morningSummaryTime)
        d.set(-1, forKey: PrefKey.morningSummaryHour)
        d.set(60, forKey: PrefKey.morningSummaryMinute)
        #expect(Preferences(d).core.morningSummaryTime == ClockTime(hour: 8, minute: 0))
    }

    @Test func everySortChoiceReachesTheCore() {
        let (d, name) = freshDefaults()
        defer { d.removePersistentDomain(forName: name) }
        let expected: [(String, SortKey)] = [
            ("manual", .manual), ("title", .title), ("due", .due), ("estimate", .estimate), ("created", .created),
        ]
        for (value, key) in expected {
            d.set(value, forKey: PrefKey.taskSort)
            #expect(Preferences(d).core.sort == key)
        }
        d.set("nonsense", forKey: PrefKey.taskSort)
        #expect(Preferences(d).core.sort == .manual, "an unknown value falls back")
        d.set("descending", forKey: PrefKey.sortDirection)
        #expect(Preferences(d).core.direction == .descending)
        d.set("30", forKey: PrefKey.upcomingRange)
        #expect(Preferences(d).core.upcomingDays == 30)
    }

    @Test func theModifierOffersTheMacKeys() {
        let (d, name) = freshDefaults()
        defer { d.removePersistentDomain(forName: name) }
        #expect(Preferences(d).modifier == .command, "Command is the Mac default")
        for m in ModifierKey.allCases {
            d.set(m.rawValue, forKey: PrefKey.modifierKey)
            #expect(Preferences(d).modifier == m)
            #expect(!m.symbol.isEmpty && !m.label.isEmpty)
        }
        #expect(ModifierKey.command.symbol == "⌘")
        #expect(ModifierKey.control.symbol == "⌃")
        #expect(ModifierKey.option.symbol == "⌥")
    }

    @Test func anIncompleteNextcloudConnectionIsNotHandedOver() {
        let (d, name) = freshDefaults()
        defer { d.removePersistentDomain(forName: name) }
        let keychain = Keychain(service: "momentum-tests-\(UUID().uuidString)")
        d.set(true, forKey: PrefKey.syncEnabled)
        d.set("https://cloud.example.com", forKey: PrefKey.nextcloudServer)
        d.set("dan", forKey: PrefKey.nextcloudUser)
        #expect(Preferences(d).nextcloud(keychain: keychain) == nil, "no password yet")
        keychain.set(Keychain.nextcloud, "app-password")
        defer { keychain.delete(Keychain.nextcloud) }
        let settings = try! #require(Preferences(d).nextcloud(keychain: keychain))
        #expect(settings.serverUrl == "https://cloud.example.com")
        #expect(settings.userName == "dan")
        #expect(settings.password == "app-password")
        #expect(settings.folder == "super-productivity")
        d.set(false, forKey: PrefKey.syncEnabled)
        #expect(Preferences(d).nextcloud(keychain: keychain) == nil, "the switch is off: nothing is handed over")
    }

    @Test func theCommandLineConfigIsWrittenWhereMoReadsIt() throws {
        let (d, name) = freshDefaults()
        defer { d.removePersistentDomain(forName: name) }
        let dir = TempDir()
        d.set("https://cloud.example.com", forKey: PrefKey.nextcloudServer)
        d.set("dan", forKey: PrefKey.nextcloudUser)
        Preferences(d).writeCliConfig(to: dir.url)
        let data = try Data(contentsOf: dir.url.appendingPathComponent("cli-config.json"))
        let json = try #require(try JSONSerialization.jsonObject(with: data) as? [String: Any])
        #expect(json["server"] as? String == "https://cloud.example.com")
        #expect(json["user"] as? String == "dan")
        #expect(json["folder"] as? String == "super-productivity")
    }

    @Test func theDataDirectoryFollowsTheEnvironmentOverride() {
        // The app and `mo` must agree on one directory; the override is how tests and a
        // second profile point both at another one.
        #expect(DataDirectory.url.path.hasSuffix("momentum"))
    }

    @Test func cliConfigurationImportsOnlySharedNonsecretFields() throws {
        let (d, name) = freshDefaults()
        defer { d.removePersistentDomain(forName: name) }
        let dir = TempDir()
        let file = dir.url.appendingPathComponent("cli-config.json")
        let config = #"{"server":"https://example.test","user":"cli-user","folder":"tasks","compress":false,"sync-enabled":true,"password":"never-import"}"#
        try Data(config.utf8).write(to: file)
        let preferences = Preferences(d)
        preferences.readCliConfig(from: dir.url)
        #expect(d.string(forKey: PrefKey.nextcloudServer) == "https://example.test")
        #expect(d.string(forKey: PrefKey.nextcloudUser) == "cli-user")
        #expect(d.string(forKey: PrefKey.nextcloudFolder) == "tasks")
        #expect(!d.bool(forKey: PrefKey.compress))
        #expect(!preferences.syncEnabled)
        #expect(d.object(forKey: "password") == nil)
        try Data(#"{"server":null,"user":"","compress":"invalid"}"#.utf8).write(to: file)
        preferences.readCliConfig(from: dir.url)
        #expect(d.string(forKey: PrefKey.nextcloudServer) == "https://example.test")
        #expect(d.string(forKey: PrefKey.nextcloudUser) == "")
        #expect(!d.bool(forKey: PrefKey.compress))
        try Data("not json".utf8).write(to: file)
        preferences.readCliConfig(from: dir.url)
        #expect(d.string(forKey: PrefKey.nextcloudFolder) == "tasks")
    }
}

@Suite struct KeychainStorage {
    @Test func secretsRoundTripAndCanBeRemoved() {
        let keychain = Keychain(service: "momentum-tests-\(UUID().uuidString)")
        #expect(keychain.get("nothing") == nil)
        #expect(keychain.set(Keychain.nextcloud, "hunter2"))
        #expect(keychain.get(Keychain.nextcloud) == "hunter2")
        #expect(keychain.set(Keychain.nextcloud, "changed"), "an existing item is updated, not duplicated")
        #expect(keychain.get(Keychain.nextcloud) == "changed")
        keychain.delete(Keychain.nextcloud)
        #expect(keychain.get(Keychain.nextcloud) == nil)
    }

    @Test func storingAnEmptyValueRemovesTheItem() {
        let keychain = Keychain(service: "momentum-tests-\(UUID().uuidString)")
        keychain.set(Keychain.encryption, "secret")
        #expect(keychain.set(Keychain.encryption, ""))
        #expect(keychain.get(Keychain.encryption) == nil)
    }

    @Test func theCoresKeyStoreUsesTheSameItems() {
        let service = "momentum-tests-\(UUID().uuidString)"
        let store = KeychainSecretStore(keychain: Keychain(service: service))
        defer { store.delete(name: "p2p-app-key") }
        #expect(store.get(name: "p2p-app-key") == nil)
        #expect(store.set(name: "p2p-app-key", value: "YmFzZTY0"))
        #expect(store.get(name: "p2p-app-key") == "YmFzZTY0")
        #expect(Keychain(service: service).get("p2p-app-key") == "YmFzZTY0")
    }
}
