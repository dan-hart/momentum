// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore
import MomentumKit
import MomentumMobile
import SwiftUI
import XCTest

@MainActor final class SyncSettingsIntegrationTests: XCTestCase {
    func testSyncScreenPresentationDoesNotOwnStateLoading() async throws {
        let name = "momentum-sync-presentation-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        let probe = PresentationSyncProbe()
        let state = NextcloudSyncState(defaults: defaults, persistence: probe)
        state.connect(makeOperation: { _ in RenderOperation() }, status: { await probe.readStatus() })
        let appeared = expectation(description: "Sync settings appeared")
        let mounted = try HostingFixture.mount(
            NavigationStack { SyncSettings(state: state).onAppear { appeared.fulfill() } },
            defaults: defaults,
            defaultsName: name
        )
        defer { mounted.unmount() }

        await fulfillment(of: [appeared], timeout: 5)
        for _ in 0..<10 { await Task.yield() }

        let counts = await probe.counts()
        XCTAssertEqual(counts.loads, 0, "Foreground lifecycle, not navigation, owns credential loading")
        XCTAssertEqual(counts.statusReads, 0, "Foreground lifecycle, not navigation, owns status refresh")
    }

    func testIsolatedModelsCannotStartSyncOrReadProductionCredentials() async throws {
        let name = "momentum-sync-isolation-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { defaults.removePersistentDomain(forName: name); try? FileManager.default.removeItem(at: directory) }
        defaults.set("nextcloud", forKey: PrefKey.syncMethod)
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
        await model.foreground()
        XCTAssertFalse(model.sync.allowed)
        XCTAssertEqual(model.sync.provider, .off)
        XCTAssertFalse(model.sync.loaded)
        XCTAssertFalse(model.sync.isSyncing)
        XCTAssertNil(model.sync.failure)
        _ = await model.perform(.add("Pending fixture", .today))
        await model.sync.refreshStatus()
        XCTAssertEqual(model.sync.status?.pendingOps, 1)
    }

    func testModelPropagatesLowPowerModeWithoutDisablingManualWork() throws {
        let name = "momentum-power-mode-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { defaults.removePersistentDomain(forName: name); try? FileManager.default.removeItem(at: directory) }

        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults,
                                   initialLowPowerMode: true)
        XCTAssertTrue(model.lowPowerMode)
        XCTAssertTrue(model.sync.lowPowerMode)

        model.setLowPowerMode(false)
        XCTAssertFalse(model.lowPowerMode)
        XCTAssertFalse(model.sync.lowPowerMode)
    }

    func testSettingsFitCompactWidthsAndGermanAccessibilityText() async throws {
        let name = "momentum-sync-render-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name) }
        let store = RenderConnection()
        let state = NextcloudSyncState(defaults: defaults, persistence: store)
        state.connect(makeOperation: { _ in RenderOperation() }, status: {
            SyncStatus(syncing: false, pendingOps: 3, lastNextcloudMs: 0,
                       lastNearbyMs: 0, nearbyRunning: false, linkedDevices: 0)
        })
        await state.load()
        await state.select(.nextcloud)
        for scheme in [ColorScheme.light, .dark] {
            for large in [false, true] {
                let render = try HostingFixture.render(
                    NavigationStack { SyncSettings(state: state) }.frame(height: 700), width: 320,
                    size: large ? .accessibility5 : .large, scheme: scheme, locale: large ? "de" : "en")
                XCTAssertLessThanOrEqual(render.size.width, 320)
                let attachment = XCTAttachment(image: render.image)
                attachment.name = "Sync-\(scheme)-\(large ? "German-AX5" : "English")"
                attachment.lifetime = .keepAlways
                add(attachment)
            }
        }
        let german = try XCTUnwrap(Bundle.main.path(forResource: "de", ofType: "lproj"))
        let strings = try XCTUnwrap(Bundle(path: german))
        XCTAssertEqual(strings.localizedString(forKey: "Save Connection", value: nil, table: nil), "Verbindung speichern")
    }

    func testTaskListStatusExpandsForGermanLargestTextInBothAppearances() async throws {
        let name = "momentum-sync-status-render-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name) }
        let state = NextcloudSyncState(defaults: defaults, persistence: RenderConnection())
        state.connect(makeOperation: { _ in RenderOperation() }, status: {
            SyncStatus(syncing: false, pendingOps: 123, lastNextcloudMs: 1_789_660_000_000,
                       lastNearbyMs: 0, nearbyRunning: false, linkedDevices: 0)
        })
        await state.load()
        await state.select(.nextcloud)
        for scheme in [ColorScheme.light, .dark] {
            let normal = try HostingFixture.render(SyncStatusLink(state: state), scheme: scheme)
            let large = try HostingFixture.render(SyncStatusLink(state: state), size: .accessibility5, scheme: scheme, locale: "de")
            XCTAssertGreaterThanOrEqual(normal.size.height, 44)
            XCTAssertGreaterThan(large.size.height, normal.size.height)
            XCTAssertLessThan(large.size.width, 321, "Allow only pixel rounding at the viewport edge")
            XCTAssertLessThan(large.size.height, 600, "Metadata must stack, not squeeze into narrow columns")
            for (name, render) in [("English", normal), ("German AX5", large)] {
                let attachment = XCTAttachment(image: render.image)
                attachment.name = "Nextcloud status \(scheme) \(name)"
                attachment.lifetime = .keepAlways
                add(attachment)
            }
        }
    }

    func testConfigurationErrorWrapsAtLargestTextOnPhoneAndPadWidths() throws {
        for width: CGFloat in [320] {
            let render = try HostingFixture.render(
                NextcloudConfigurationError(issue: .secureServerRequired),
                width: width,
                size: .accessibility5
            )
            XCTAssertLessThanOrEqual(render.size.width, width)
            XCTAssertGreaterThan(render.size.height, 44)
            XCTAssertLessThan(render.size.height, 500)
        }
    }

    func testLibreSyncDeviceManagementFitsPhoneAndPadAtLargestText() async throws {
        let state = NearbyLifecycle(runtime: RenderNearbyRuntime(), allowed: true)

        for width: CGFloat in [320, 744] {
            let render = try HostingFixture.render(
                LibreSyncDeviceList(state: state,
                                    linking: .constant(nil),
                                    unlinking: .constant(nil))
                    .frame(height: 900),
                width: width,
                size: .accessibility5
            )
            XCTAssertLessThanOrEqual(render.size.width, width)
            XCTAssertGreaterThan(render.size.height, 44)
            XCTAssertLessThan(render.size.height, 2_000)
        }
    }

    func testLateExpirationCannotCancelANewerExchange() {
        var expires: (@MainActor @Sendable () -> Void)?
        var cancellations = 0
        let allowance = SyncExecutionAllowance(expiration: { cancellations += 1 }, begin: { handler in
            expires = handler
            return UIBackgroundTaskIdentifier(rawValue: 44)
        }, end: { _ in })
        allowance.finish()
        expires?()
        XCTAssertEqual(cancellations, 0)
    }

    func testImmediateExpirationStillBalancesReturnedExecutionToken() {
        var ended: [UIBackgroundTaskIdentifier] = []
        let token = UIBackgroundTaskIdentifier(rawValue: 43)
        let allowance = SyncExecutionAllowance(expiration: {}, begin: { handler in
            handler()
            return token
        }, end: { ended.append($0) })
        XCTAssertEqual(ended, [token])
        allowance.finish()
        XCTAssertEqual(ended, [token])
    }

    func testExecutionAllowanceEndsExactlyOnceOnExpiryOrCompletion() {
        var expires: (@MainActor @Sendable () -> Void)?
        var ended: [UIBackgroundTaskIdentifier] = []
        var cancellations = 0
        let token = UIBackgroundTaskIdentifier(rawValue: 42)
        let allowance = SyncExecutionAllowance(expiration: { cancellations += 1 }, begin: { handler in
            expires = handler
            return token
        }, end: { ended.append($0) })
        expires?()
        allowance.finish()
        XCTAssertEqual(cancellations, 1)
        XCTAssertEqual(ended, [token])
        let denied = SyncExecutionAllowance(expiration: {}, begin: { _ in .invalid }, end: { ended.append($0) })
        denied.finish()
        XCTAssertEqual(ended, [token])
    }
}

private actor PresentationSyncProbe: NextcloudConnectionPersistence {
    private var loadCount = 0
    private var statusCount = 0

    func load() -> NextcloudConnection? {
        loadCount += 1
        return nil
    }

    func save(_ connection: NextcloudConnection) {}

    func readStatus() -> SyncStatus {
        statusCount += 1
        return SyncStatus(syncing: false, pendingOps: 0, lastNextcloudMs: 0,
                          lastNearbyMs: 0, nearbyRunning: false, linkedDevices: 0)
    }

    func counts() -> (loads: Int, statusReads: Int) {
        (loadCount, statusCount)
    }
}

private actor RenderConnection: NextcloudConnectionPersistence {
    func load() -> NextcloudConnection? {
        var value = NextcloudConnection()
        value.serverURL = "https://fixture.invalid"
        value.userName = "fixture"
        value.password = "fixture-only"
        value.automatic = false
        return value
    }
    func save(_ connection: NextcloudConnection) {}
}

private struct RenderOperation: NextcloudRunningOperation {
    func run() async throws -> SyncReport { throw CancellationError() }
    func cancel() -> Bool { true }
}

private actor RenderNearbyRuntime: NearbyRuntime {
    func start(events: @escaping @Sendable (P2pEvent) async -> Void) -> P2pInfo {
        P2pInfo(deviceName: "Fixture iPhone", deviceId: "fixture-phone", port: 42424)
    }
    func stop() {}
    func snapshot() -> NearbySnapshot {
        NearbySnapshot(
            linked: [NearbyDevice(deviceId: "linked", name: "Kitchen Mac", address: nil,
                                  lastSeenMs: 1_789_660_000_000, linked: true)],
            discovered: [NearbyDevice(deviceId: "new", name: "Office iPad",
                                      address: "127.0.0.1:42425", lastSeenMs: nil, linked: false)],
            status: SyncStatus(syncing: false, pendingOps: 2, lastNextcloudMs: 0,
                               lastNearbyMs: 1_789_660_000_000, nearbyRunning: true, linkedDevices: 1)
        )
    }
    func beginPairing() -> String? { "123456" }
    func endPairing() {}
    func link(address: String, code: String) throws {}
    func unlink(id: String) {}
    func syncNow() -> UInt64 { 1 }
}
