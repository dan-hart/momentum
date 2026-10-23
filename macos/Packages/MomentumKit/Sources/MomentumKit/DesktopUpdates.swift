// SPDX-License-Identifier: GPL-3.0-or-later
#if os(macOS)
import Foundation
import Darwin

public enum DesktopUpdateChannel: Equatable, Sendable { case source, homebrew, direct, unconfigured }
public enum DesktopUpdatePolicy {
    public static func canRestart(syncing: Bool, editorOpen: Bool, confirmationOpen: Bool, hasDraft: Bool) -> Bool {
        !syncing && !editorOpen && !confirmationOpen && !hasDraft
    }
    public static func channel(packagedRelease: Bool, preview: Bool, homebrewOwner: Bool, ownershipUncertain: Bool = false, publicKey: String) -> DesktopUpdateChannel {
        guard packagedRelease, !preview else { return .source }
        if homebrewOwner { return .homebrew }
        if ownershipUncertain { return .unconfigured }
        guard let key = Data(base64Encoded: publicKey), key.count == 32 else { return .unconfigured }
        return .direct
    }
}

public enum DesktopUpdateError: Error, Equatable { case invalidReceipt, commandFailed, timedOut, outputTooLarge }
/// Ownership requires both an installed cask from our tap and an exact path reported by
/// `brew list --cask`. Finding a brew executable alone is never sufficient.
public struct HomebrewCaskInfo: Sendable {
    public let latestVersion: String
    public let installedVersion: String
    public init(data: Data) throws {
        guard let root = try JSONSerialization.jsonObject(with: data) as? [String: Any],
              let casks = root["casks"] as? [[String: Any]], casks.count == 1,
              let cask = casks.first, cask["token"] as? String == "momentum",
              cask["full_token"] as? String == "dan-hart/tap/momentum",
              let installed = cask["installed"] as? String, !installed.isEmpty,
              let version = cask["version"] as? String, !version.isEmpty else { throw DesktopUpdateError.invalidReceipt }
        latestVersion = version
        installedVersion = installed
    }
    public func owns(installedPaths: [String], appURL: URL) -> Bool {
        let target = appURL.resolvingSymlinksInPath().standardizedFileURL
        return installedPaths.contains { path in
            guard path.hasPrefix("/"), path.hasSuffix(".app") else { return false }
            return URL(fileURLWithPath: path).resolvingSymlinksInPath().standardizedFileURL == target
        }
    }
}

public struct UpdateCommandResult: Sendable {
    public let output: Data
    public let status: Int32
    public init(output: Data, status: Int32) { self.output = output; self.status = status }
}
public protocol UpdateCommandRunning: Sendable {
    func run(executable: URL, arguments: [String], timeout: TimeInterval) async throws -> UpdateCommandResult
}

/// No shell, no inherited task/account environment, bounded output and runtime. Child
/// stderr is deliberately discarded: package-manager logs can contain private URLs.
public struct UpdateCommandRunner: UpdateCommandRunning {
    public init() {}
    public func run(executable: URL, arguments: [String], timeout: TimeInterval) async throws -> UpdateCommandResult {
        let execution = UpdateProcessExecution()
        return try await withTaskCancellationHandler {
            try Task.checkCancellation()
            let result: UpdateCommandResult = try await withCheckedThrowingContinuation { continuation in
                DispatchQueue.global(qos: .utility).async {
                    do { continuation.resume(returning: try execution.run(executable, arguments, timeout)) }
                    catch { continuation.resume(throwing: execution.wasCancelled ? CancellationError() : error) }
                }
            }
            try Task.checkCancellation()
            return result
        } onCancel: { execution.stop() }
    }
}
private final class UpdateProcessExecution: @unchecked Sendable {
    private let lock = NSLock()
    private var cancelled = false
    var wasCancelled: Bool { lock.withLock { cancelled } }
    func stop() { lock.withLock { cancelled = true } }
    func run(_ executable: URL, _ arguments: [String], _ timeout: TimeInterval) throws -> UpdateCommandResult {
        if wasCancelled { throw CancellationError() }
        let pipe = Pipe()
        let readFD = pipe.fileHandleForReading.fileDescriptor
        let writeFD = pipe.fileHandleForWriting.fileDescriptor
        let nullFD = open("/dev/null", O_RDWR)
        guard nullFD >= 0 else { throw DesktopUpdateError.commandFailed }
        defer { close(nullFD) }
        var actions: posix_spawn_file_actions_t?
        var attributes: posix_spawnattr_t?
        posix_spawn_file_actions_init(&actions); posix_spawnattr_init(&attributes)
        defer { posix_spawn_file_actions_destroy(&actions); posix_spawnattr_destroy(&attributes) }
        posix_spawn_file_actions_adddup2(&actions, nullFD, STDIN_FILENO)
        posix_spawn_file_actions_adddup2(&actions, writeFD, STDOUT_FILENO)
        posix_spawn_file_actions_adddup2(&actions, nullFD, STDERR_FILENO)
        // A private process group lets cancellation and timeout stop Ruby/curl and
        // other descendants, rather than leaving an upgrade running invisibly.
        posix_spawnattr_setflags(&attributes, Int16(POSIX_SPAWN_SETPGROUP | POSIX_SPAWN_CLOEXEC_DEFAULT | POSIX_SPAWN_SETSIGMASK | POSIX_SPAWN_SETSIGDEF))
        posix_spawnattr_setpgroup(&attributes, 0)
        var signals = sigset_t(0), mask = sigset_t(0)
        sigemptyset(&signals); sigemptyset(&mask)
        sigaddset(&signals, SIGTERM); sigaddset(&signals, SIGINT); sigaddset(&signals, SIGPIPE)
        posix_spawnattr_setsigdefault(&attributes, &signals); posix_spawnattr_setsigmask(&attributes, &mask)
        let home = FileManager.default.homeDirectoryForCurrentUser.path
        let environment = ["HOME=\(home)", "PATH=/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin",
                           "HOMEBREW_NO_ANALYTICS=1", "HOMEBREW_NO_ENV_HINTS=1", "HOMEBREW_NO_INSTALL_CLEANUP=1",
                           "HOMEBREW_NO_AUTO_UPDATE=1", "HOMEBREW_NO_UPGRADE_QUIT_CASKS=1", "HOMEBREW_NO_ASK=1", "NONINTERACTIVE=1", "CI=1"]
        let argv = ([executable.path] + arguments).map { strdup($0) } + [nil]
        let envp = environment.map { strdup($0) } + [nil]
        defer { argv.forEach { free($0) }; envp.forEach { free($0) } }
        var pid: pid_t = 0
        let launch = argv.withUnsafeBufferPointer { args in
            envp.withUnsafeBufferPointer { env in
                posix_spawn(&pid, executable.path, &actions, &attributes,
                            UnsafeMutablePointer(mutating: args.baseAddress!), UnsafeMutablePointer(mutating: env.baseAddress!))
            }
        }
        guard launch == 0 else { throw DesktopUpdateError.commandFailed }
        try? pipe.fileHandleForWriting.close()
        _ = fcntl(readFD, F_SETFL, O_NONBLOCK)
        let started = ProcessInfo.processInfo.systemUptime
        var output = Data(), overflow = false, timedOut = false
        var bytes = [UInt8](repeating: 0, count: 65_536)
        var status: Int32 = 0, exited = false
        var terminationAt: TimeInterval?
        while true {
            let now = ProcessInfo.processInfo.systemUptime
            if now - started >= timeout { timedOut = true }
            if wasCancelled || timedOut || overflow {
                if terminationAt == nil { kill(-pid, SIGTERM); terminationAt = now }
                else if now - terminationAt! >= 2 { kill(-pid, SIGKILL) }
            }
            let count = Darwin.read(readFD, &bytes, bytes.count)
            if count > 0 {
                if output.count + count <= 4 * 1024 * 1024 { output.append(contentsOf: bytes.prefix(count)) }
                else { overflow = true }
            }
            if !exited { exited = waitpid(pid, &status, WNOHANG) == pid }
            // Group lifetime includes descendants even when the launcher exits first.
            let groupExists = kill(-pid, 0) == 0
            if exited && !groupExists && count <= 0 { break }
            if let terminationAt, exited && now - terminationAt >= 3 { break }
            Thread.sleep(forTimeInterval: 0.01)
        }
        if wasCancelled { throw CancellationError() }
        if overflow { throw DesktopUpdateError.outputTooLarge }
        if timedOut { throw DesktopUpdateError.timedOut }
        let exitCode: Int32 = (status & 0x7f) == 0 ? (status >> 8) & 0xff : 128 + (status & 0x7f)
        return UpdateCommandResult(output: output, status: exitCode)
    }
}

public struct HomebrewUpdates: Sendable {
    public let executable: URL
    public let appURL: URL
    private let runner: any UpdateCommandRunning
    public init(executable: URL, appURL: URL, runner: any UpdateCommandRunning = UpdateCommandRunner()) {
        self.executable = executable; self.appURL = appURL; self.runner = runner
    }
    private func command(_ arguments: [String], timeout: TimeInterval = 30) async throws -> Data {
        try Task.checkCancellation()
        let result = try await runner.run(executable: executable, arguments: arguments, timeout: timeout)
        try Task.checkCancellation()
        guard result.status == 0 else { throw DesktopUpdateError.commandFailed }
        return result.output
    }
    public func check(refresh: Bool) async throws -> HomebrewCaskInfo {
        if refresh { _ = try await command(["update", "--quiet"], timeout: 180) }
        let info = try HomebrewCaskInfo(data: await command(["info", "--json=v2", "--cask", "dan-hart/tap/momentum"]))
        let paths = String(decoding: try await command(["list", "--cask", "dan-hart/tap/momentum"]), as: UTF8.self).split(separator: "\n").map(String.init)
        guard info.owns(installedPaths: paths, appURL: appURL) else { throw DesktopUpdateError.invalidReceipt }
        return info
    }
    public func install() async throws {
        let expected = try await check(refresh: false).latestVersion
        _ = try await command(["upgrade", "--cask", "dan-hart/tap/momentum"], timeout: 600)
        let receipt = try await check(refresh: false)
        let data = try Data(contentsOf: appURL.appendingPathComponent("Contents/Info.plist"))
        guard let info = try PropertyListSerialization.propertyList(from: data, options: [], format: nil) as? [String: Any],
              info["CFBundleIdentifier"] as? String == "com.codedbydan.Momentum",
              info["CFBundleShortVersionString"] as? String == expected,
              receipt.installedVersion == expected else { throw DesktopUpdateError.invalidReceipt }
    }
}
#endif
