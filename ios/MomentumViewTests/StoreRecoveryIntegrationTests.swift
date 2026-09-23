// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumMobile
import SwiftUI
import XCTest

@MainActor final class StoreRecoveryIntegrationTests: XCTestCase {
    func testStartupErrorPreservesDataAndRetryRestoresSelectedTab() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let name = "momentum-recovery-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name); try? FileManager.default.removeItem(at: directory) }
        let seed = try await EngineWorker.openChecked(directory: directory)
        _ = await seed.perform(.add("Keep me", .today))
        let before = try Data(contentsOf: directory.appendingPathComponent("state.json"))
        let journal = directory.appendingPathComponent(".momentum-transaction")
        try FileManager.default.createDirectory(at: journal, withIntermediateDirectories: false)
        try Data("{}".utf8).write(to: journal.appendingPathComponent("manifest.json"))
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
        model.selectedTab = 3
        await model.foreground()
        XCTAssertNil(model.worker)
        XCTAssertNotNil(model.startupFailure)
        XCTAssertFalse(model.isOpeningStore)
        XCTAssertEqual(try Data(contentsOf: directory.appendingPathComponent("state.json")), before)
        let rendered = try HostingFixture.render(RootView().environment(model).environment(model.sync).frame(height: 640), width: 320)
        XCTAssertEqual(rendered.size.width, 320)
        XCTAssertEqual(rendered.size.height, 640)
        // Remove only this test's injected fault, then exercise the same retry entry point as the button.
        try FileManager.default.removeItem(at: journal)
        await model.foreground()
        XCTAssertNil(model.startupFailure)
        XCTAssertEqual(model.selectedTab, 3)
        let worker = try XCTUnwrap(model.worker)
        let tasks = await worker.snapshot(view: .today).tasks
        XCTAssertEqual(tasks.map(\.title), ["Keep me"])
    }

    func testRecoveryScreenRendersInGermanAtLargestTextInBothAppearances() throws {
        // Guard the host resource setup; locale alone does not supply translations.
        let german = try XCTUnwrap(Bundle.main.path(forResource: "de", ofType: "lproj"))
        let strings = try XCTUnwrap(Bundle(path: german))
        XCTAssertEqual(strings.localizedString(forKey: "Data Unavailable", value: nil, table: nil), "Daten nicht verfügbar")
        for scheme in [ColorScheme.light, .dark] {
            let rendered = try HostingFixture.render(
                StoreUnavailableView(details: "incomplete store recovery image", isRetrying: false, retry: {})
                    .frame(height: 640), width: 280, size: .accessibility5, scheme: scheme, locale: "de")
            XCTAssertEqual(rendered.size.width, 280, accuracy: 0.5) // allow native pixel rounding
            XCTAssertEqual(rendered.size.height, 640)
            let attachment = XCTAttachment(image: rendered.image)
            attachment.name = "Store recovery German AX5 \(scheme)"
            attachment.lifetime = .keepAlways
            add(attachment)
        }
    }

    func testStartupFailureReplacesSkeletonAndRetryReturnsThroughReadyState() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let name = "momentum-skeleton-recovery-tests-\(UUID())"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer {
            defaults.removePersistentDomain(forName: name)
            try? FileManager.default.removeItem(at: directory)
        }
        _ = try await EngineWorker.openChecked(directory: directory)
        let journal = directory.appendingPathComponent(".momentum-transaction")
        try FileManager.default.createDirectory(at: journal, withIntermediateDirectories: false)
        try Data("{}".utf8).write(to: journal.appendingPathComponent("manifest.json"))
        let model = MobileAppModel(isolatedDirectory: directory, defaults: defaults)
        await model.foreground()
        XCTAssertNotNil(model.startupFailure)
        let gate = StoreRecoverySnapshotGate()
        let mounted = try HostingFixture.mount(
            RootView(snapshotLoader: { worker, view, query, archiveLimit in
                await gate.wait()
                return await worker.snapshot(view: view, query: query, archiveLimit: archiveLimit)
            })
            .environment(model)
            .environment(model.sync),
            defaults: defaults,
            defaultsName: name
        )
        defer { mounted.unmount() }

        await mounted.settle()
        XCTAssertTrue(
            mounted.hasAccessibilityElement(label: "Try Again"),
            "Recovery accessibility: \(mounted.accessibilitySnapshot())"
        )
        XCTAssertFalse(mounted.hasVisibleView(identifier: "today-loading-skeleton"))

        try FileManager.default.removeItem(at: journal)
        await model.foreground()
        await gate.waitForArrival()
        XCTAssertTrue(mounted.hasVisibleView(identifier: "today-loading-skeleton"))
        XCTAssertFalse(mounted.hasAccessibilityElement(label: "Try Again"))

        await gate.releaseAll()
        try await mounted.wait(until: "Today ready after recovery") {
            !mounted.hasVisibleView(identifier: "today-loading-skeleton")
                && mounted.hasAccessibilityElement(label: "Add task")
        }
    }
}

private actor StoreRecoverySnapshotGate {
    private var arrived = false
    private var arrivalWaiter: CheckedContinuation<Void, Never>?
    private var pending: [CheckedContinuation<Void, Never>] = []

    func wait() async {
        arrived = true
        arrivalWaiter?.resume()
        arrivalWaiter = nil
        await withCheckedContinuation { pending.append($0) }
    }

    func waitForArrival() async {
        guard !arrived else { return }
        await withCheckedContinuation { arrivalWaiter = $0 }
    }

    func releaseAll() {
        let waiters = pending
        pending.removeAll()
        waiters.forEach { $0.resume() }
    }
}
