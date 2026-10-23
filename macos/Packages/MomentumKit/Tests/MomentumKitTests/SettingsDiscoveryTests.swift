// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import Testing
@testable import MomentumKit

@MainActor @Suite struct SettingsDiscoveryTests {
    @Test func nativeDirectoryHasUniqueKnownDestinationsAndEveryCategory() {
        for platform in [SettingsPlatform.macos, .ios] {
            let entries = SettingsDiscovery.entries(platform: platform)
            #expect(Set(entries.map(\.id)).count == entries.count)
            #expect(Set(entries.map { SettingsDiscovery.category(for: $0.id) }) == Set(SettingsDiscovery.categories(platform: platform).map(\.id)))
            #expect(entries.allSatisfy { entry in settingsDirectory().contains { $0.id == entry.id && $0.platforms.contains(platform) } })
            #expect(!entries.contains { $0.id == "notifications.exact-alarm" })
        }
        #expect(SettingsDiscovery.entries(platform: .ios).contains { $0.id == "appearance.compact" })
    }
    @Test func everyNativeSearchTitleAndCategoryHasEnglishAndGermanResources() throws {
        for language in ["en", "de"] {
            let path = try #require(Bundle.module.path(forResource: language, ofType: "lproj"))
            let resources = try #require(Bundle(path: path))
            for title in Set(SettingsDiscovery.entries(platform: .macos).map(\.title) + SettingsDiscovery.entries(platform: .ios).map(\.title) + SettingsDiscovery.categories.map(\.title)) {
                #expect(resources.localizedString(forKey: title, value: "__missing__", table: "Settings") != "__missing__")
            }
        }
    }
    @Test func aliasesAndPureSearchPreserveExactDestinationIDs() {
        #expect(SettingsDiscovery.results("night", platform: .macos).contains { $0.id == "tasks.day-period-visibility" })
        #expect(SettingsDiscovery.results("restore", platform: .macos).contains { $0.id == "data.tasks-import" })
        #expect(SettingsDiscovery.results("   ", platform: .macos).isEmpty)
        #expect(SettingsDiscovery.results("xyz-no-setting", platform: .ios).isEmpty)
        #expect(SettingsDiscovery.category(for: "sync.nextcloud") == "sync")
    }
    @Test func exactRouteClearsOnlySettingsSearchAndRevalidatesCapability() {
        var route = SettingsRouteState()
        route.open(nil, query: "old settings query", platform: .macos)
        route.open("sounds.volume", platform: .macos)
        #expect(route.category == "sounds")
        #expect(route.destination == "sounds.volume")
        #expect(route.query.isEmpty)
        route.open("notifications.exact-alarm", platform: .macos)
        #expect(route.category == "notifications")
        #expect(route.destination == nil)
        route.open("unknown.control", platform: .macos)
        #expect(route.category == "tasks")
        #expect(route.destination == nil)
    }
    @Test func allSettingsResultsRetainQueryAndClearPreviousRowTarget() {
        var route = SettingsRouteState()
        route.open("sync.nextcloud", platform: .macos)
        #expect(route.destination == "sync.nextcloud")
        route.open(nil, query: "backup restore", platform: .macos)
        #expect(route.query == "backup restore")
        #expect(route.destination == nil)
        #expect(route.category == "sync")
        route.query = ""
        #expect(route.category == "sync")
        #expect(route.destination == nil)
    }
    @Test func copiedDetailsContainOnlyDisplayedPublicMetadata() {
        let details = MomentumAppDetails.text(platform: "macOS")
        #expect(details.hasPrefix("Momentum\nmacOS"))
        #expect(!details.contains(NSHomeDirectory()))
        #expect(!details.contains("server"))
        #expect(!details.contains("account"))
    }
}
