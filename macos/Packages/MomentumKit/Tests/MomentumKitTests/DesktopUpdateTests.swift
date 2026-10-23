// SPDX-License-Identifier: GPL-3.0-or-later
#if os(macOS)
import Foundation
import Testing
@testable import MomentumKit

@Suite struct DesktopUpdateTests {
    @Test func restartWaitsUntilSyncAndEveryDraftAreFinished() {
        #expect(DesktopUpdatePolicy.canRestart(syncing: false, editorOpen: false, confirmationOpen: false, hasDraft: false))
        #expect(!DesktopUpdatePolicy.canRestart(syncing: true, editorOpen: false, confirmationOpen: false, hasDraft: false))
        #expect(!DesktopUpdatePolicy.canRestart(syncing: false, editorOpen: true, confirmationOpen: false, hasDraft: false))
        #expect(!DesktopUpdatePolicy.canRestart(syncing: false, editorOpen: false, confirmationOpen: true, hasDraft: false))
        #expect(!DesktopUpdatePolicy.canRestart(syncing: false, editorOpen: false, confirmationOpen: false, hasDraft: true))
    }
    @Test func developmentAndPreviewNeverReplaceAnApp() {
        #expect(DesktopUpdatePolicy.channel(packagedRelease: false, preview: false, homebrewOwner: true, publicKey: "key") == .source)
        #expect(DesktopUpdatePolicy.channel(packagedRelease: true, preview: true, homebrewOwner: true, publicKey: "key") == .source)
    }
    @Test func exactCaskOwnershipWinsOverDirectChannel() {
        #expect(DesktopUpdatePolicy.channel(packagedRelease: true, preview: false, homebrewOwner: true, publicKey: "") == .homebrew)
        #expect(DesktopUpdatePolicy.channel(packagedRelease: true, preview: false, homebrewOwner: false, publicKey: "") == .unconfigured)
        #expect(DesktopUpdatePolicy.channel(packagedRelease: true, preview: false, homebrewOwner: false, publicKey: Data(repeating: 1, count: 32).base64EncodedString()) == .direct)
    }
    @Test func ambiguousOwnershipNeverFallsThroughToSparkle() {
        #expect(DesktopUpdatePolicy.channel(packagedRelease: true, preview: false, homebrewOwner: false, ownershipUncertain: true,
                                          publicKey: Data(repeating: 1, count: 32).base64EncodedString()) == .unconfigured)
    }
    @Test func invalidKeysFailClosed() {
        for key in ["", "$(MOMENTUM_UPDATE_PUBLIC_KEY)", "not base64", "a2V5"] {
            #expect(DesktopUpdatePolicy.channel(packagedRelease: true, preview: false, homebrewOwner: false, publicKey: key) == .unconfigured)
        }
    }
    @Test func caskMustMatchExactResolvedPathAndToken() throws {
        let json = Data(#"{"casks":[{"token":"momentum","full_token":"dan-hart/tap/momentum","installed":"0.4.13","version":"0.4.14","artifacts":[["Momentum.app",{"target":"/Applications/Momentum.app"}]]}]}"#.utf8)
        let cask = try HomebrewCaskInfo(data: json)
        #expect(cask.owns(installedPaths: ["/Applications/Momentum.app"], appURL: URL(fileURLWithPath: "/Applications/Momentum.app")))
        #expect(!cask.owns(installedPaths: ["/Applications/Momentum.app"], appURL: URL(fileURLWithPath: "/tmp/Momentum.app")))
        #expect(cask.latestVersion == "0.4.14")
        let foreign = Data(String(decoding: json, as: UTF8.self).replacingOccurrences(of: "dan-hart/tap/momentum", with: "other/tap/momentum").utf8)
        #expect(throws: (any Error).self) { try HomebrewCaskInfo(data: foreign) }
    }
    @Test func uninstalledOrAmbiguousCaskDoesNotOwnApp() throws {
        for installed in ["null", "false", "\"\""] {
            let json = Data("{\"casks\":[{\"token\":\"momentum\",\"full_token\":\"dan-hart/tap/momentum\",\"installed\":\(installed),\"version\":\"1\",\"artifacts\":[[\"Momentum.app\",{\"target\":\"/Applications/Momentum.app\"}]]}]}".utf8)
            #expect(throws: (any Error).self) { try HomebrewCaskInfo(data: json) }
        }
    }
    @Test func updateRevalidatesOwnershipAndUsesOnlyNamedCask() async throws {
        let app = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString).appendingPathComponent("Momentum.app")
        try FileManager.default.createDirectory(at: app.appendingPathComponent("Contents"), withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: app.deletingLastPathComponent()) }
        let plist = try PropertyListSerialization.data(fromPropertyList: ["CFBundleShortVersionString": "0.4.14", "CFBundleIdentifier": "com.codedbydan.Momentum"], format: .xml, options: 0)
        try plist.write(to: app.appendingPathComponent("Contents/Info.plist"))
        let runner = FixtureUpdateRunner(path: app.path)
        let driver = HomebrewUpdates(executable: URL(fileURLWithPath: "/fake/brew"), appURL: app, runner: runner)
        _ = try await driver.check(refresh: true)
        try await driver.install()
        let calls = await runner.calls
        #expect(calls.first == ["update", "--quiet"])
        #expect(calls.contains(["upgrade", "--cask", "dan-hart/tap/momentum"]))
        #expect(calls.filter { $0.first == "list" }.count == 3)
    }
    @Test func successfulExitWithoutNewBundleNeverReportsInstalled() async throws {
        let app = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString).appendingPathComponent("Momentum.app")
        try FileManager.default.createDirectory(at: app.appendingPathComponent("Contents"), withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: app.deletingLastPathComponent()) }
        let plist = try PropertyListSerialization.data(fromPropertyList: ["CFBundleShortVersionString": "0.4.13", "CFBundleIdentifier": "com.codedbydan.Momentum"], format: .xml, options: 0)
        try plist.write(to: app.appendingPathComponent("Contents/Info.plist"))
        let driver = HomebrewUpdates(executable: URL(fileURLWithPath: "/fake/brew"), appURL: app, runner: FixtureUpdateRunner(path: app.path))
        await #expect(throws: (any Error).self) { try await driver.install() }
    }
    @Test func groupCancellationStopsTermIgnoringDescendant() async throws {
        let marker = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: marker) }
        let task = Task {
            try await UpdateCommandRunner().run(executable: URL(fileURLWithPath: "/bin/sh"),
                arguments: ["-c", "(trap '' TERM; sleep 3; printf mutation > \"$1\") & wait", "fixture", marker.path], timeout: 10)
        }
        try await Task.sleep(for: .milliseconds(100)); task.cancel()
        await #expect(throws: CancellationError.self) { try await task.value }
        try await Task.sleep(for: .seconds(1.1))
        #expect(!FileManager.default.fileExists(atPath: marker.path))
    }
    @Test func copiedAppCannotUpgradeCask() async throws {
        let runner = FixtureUpdateRunner()
        let driver = HomebrewUpdates(executable: URL(fileURLWithPath: "/fake/brew"), appURL: URL(fileURLWithPath: "/tmp/Momentum.app"), runner: runner)
        await #expect(throws: (any Error).self) { try await driver.install() }
        #expect(await runner.calls.allSatisfy { $0.first != "upgrade" })
    }
    @Test func runnerCancellationPropagatesWithoutReportingTimeout() async throws {
        let task = Task { try await UpdateCommandRunner().run(executable: URL(fileURLWithPath: "/bin/sleep"), arguments: ["30"], timeout: 10) }
        try await Task.sleep(for: .milliseconds(100))
        task.cancel()
        await #expect(throws: CancellationError.self) { try await task.value }
    }
    @Test func runnerBoundsRuntimeAndOutput() async throws {
        await #expect(throws: DesktopUpdateError.timedOut) {
            try await UpdateCommandRunner().run(executable: URL(fileURLWithPath: "/bin/sleep"), arguments: ["30"], timeout: 0.05)
        }
        await #expect(throws: DesktopUpdateError.outputTooLarge) {
            try await UpdateCommandRunner().run(executable: URL(fileURLWithPath: "/usr/bin/yes"), arguments: [], timeout: 5)
        }
    }
    @Test func runnerPreservesLiteralArgumentsAndExitStatus() async throws {
        let result = try await UpdateCommandRunner().run(executable: URL(fileURLWithPath: "/usr/bin/printf"), arguments: ["%s", "$(no-shell); untouched"], timeout: 1)
        #expect(String(decoding: result.output, as: UTF8.self) == "$(no-shell); untouched")
        let failed = try await UpdateCommandRunner().run(executable: URL(fileURLWithPath: "/usr/bin/false"), arguments: [], timeout: 1)
        #expect(failed.status != 0)
    }
    @Test func commandFailureNeverBecomesSuccess() async throws {
        let runner = FixtureUpdateRunner(fail: true)
        let driver = HomebrewUpdates(executable: URL(fileURLWithPath: "/fake/brew"), appURL: URL(fileURLWithPath: "/Applications/Momentum.app"), runner: runner)
        await #expect(throws: (any Error).self) { try await driver.check(refresh: true) }
        #expect(await runner.calls.count == 1)
    }
}
private actor FixtureUpdateRunner: UpdateCommandRunning {
    var calls: [[String]] = []
    var upgraded = false
    let fail: Bool
    let path: String
    init(fail: Bool = false, path: String = "/Applications/Momentum.app") { self.fail = fail; self.path = path }
    func run(executable: URL, arguments: [String], timeout: TimeInterval) async throws -> UpdateCommandResult {
        calls.append(arguments)
        if arguments.first == "upgrade" { upgraded = true }
        if fail { return .init(output: Data(), status: 1) }
        let output = arguments.first == "info"
            ? #"{"casks":[{"token":"momentum","full_token":"dan-hart/tap/momentum","installed":"0.4.13","version":"0.4.14"}]}"#
            : path + "\n"
        return .init(output: Data((upgraded ? output.replacingOccurrences(of: "\"installed\":\"0.4.13\"", with: "\"installed\":\"0.4.14\"") : output).utf8), status: 0)
    }
}
#endif
