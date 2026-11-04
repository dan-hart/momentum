// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import Testing
@testable import MomentumKit

@MainActor @Suite struct NextcloudConnectionTestTests {
    @Test func cancellationDiscardsLateSuccess() async throws {
        let probe = NextcloudConnectionTest { _, _, _ in
            Thread.sleep(forTimeInterval: 0.05)
        }
        let h = Harness()
        let settings = NextcloudSettings(serverUrl: "https://example.invalid", userName: "fixture", password: "fixture", folder: "momentum", compress: false, encryptionPassword: nil, optimized: false)
        probe.start(engine: h.engine, settings: settings)
        #expect(probe.isRunning)
        probe.cancel()
        try await Task.sleep(for: .milliseconds(100))
        #expect(!probe.isRunning)
        #expect(probe.succeeded == nil)
        #expect(probe.error == nil)
    }
    @Test func failedProbeKeepsDiagnosisSeparateFromTaskSync() async throws {
        let probe = NextcloudConnectionTest { _, _, _ in throw CocoaError(.fileReadNoPermission) }
        let h = Harness()
        let before = h.engine.syncStatus()
        let settings = NextcloudSettings(serverUrl: "https://example.invalid", userName: "fixture", password: "fixture", folder: "momentum", compress: false, encryptionPassword: nil)
        probe.start(engine: h.engine, settings: settings)
        for _ in 0..<100 where probe.isRunning { try await Task.sleep(for: .milliseconds(5)) }
        #expect(probe.succeeded == false)
        #expect(probe.error != nil)
        #expect(h.engine.syncStatus() == before)
        #expect(h.state.syncError == nil)
    }
    @Test func successfulProbeDoesNotMarkTasksSynced() async throws {
        let probe = NextcloudConnectionTest { _, _, _ in }
        let h = Harness()
        let before = h.engine.syncStatus()
        let settings = NextcloudSettings(serverUrl: "https://example.invalid", userName: "fixture", password: "fixture", folder: "momentum", compress: false, encryptionPassword: nil, optimized: false)
        probe.start(engine: h.engine, settings: settings)
        for _ in 0..<100 where probe.isRunning { try await Task.sleep(for: .milliseconds(5)) }
        #expect(probe.succeeded == true)
        #expect(h.engine.syncStatus() == before)
    }
}
