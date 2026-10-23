// SPDX-License-Identifier: GPL-3.0-or-later
import AppKit
import Combine
import MomentumKit
import Observation
import Sparkle

/// One updater per app. Sparkle belongs to the macOS target, never MomentumKit/iOS.
@MainActor @Observable final class DesktopUpdateController: NSObject, SPUUpdaterDelegate {
    static let shared = DesktopUpdateController()
    static let automaticKey = "desktopUpdatesAutomaticChecks"
    private(set) var channel: DesktopUpdateChannel = .source
    private(set) var ownershipUncertain = false
    private(set) var busy = false
    private(set) var installing = false
    private(set) var restarting = false
    private(set) var ready = false
    private(set) var latestVersion: String?
    private(set) var message = ""
    private(set) var restartAvailable = false
    private(set) var sparkleCanCheck = false
    private var controller: SPUStandardUpdaterController?
    private var observation: NSKeyValueObservation?
    private var brew: HomebrewUpdates?
    private var operation: Task<Void, Never>?
    private var timer: Timer?
    private var relaunchProcess: Process?
    private var relaunchPipe: Pipe?
    private var draftIDs: Set<String> = []
    private var postponedInstall: (() -> Void)?
    var automatic = UserDefaults.standard.bool(forKey: automaticKey) {
        didSet {
            UserDefaults.standard.set(automatic, forKey: Self.automaticKey)
            controller?.updater.automaticallyChecksForUpdates = automatic
            scheduleChecks(checkNow: automatic)
        }
    }
    func setDraft(_ id: String, hasContent: Bool) {
        if hasContent { draftIDs.insert(id) } else { draftIDs.remove(id) }
    }
    var canRestart: Bool {
        DesktopUpdatePolicy.canRestart(syncing: AppDelegate.shared?.syncInProgress == true,
            editorOpen: AppDelegate.shared?.sheet != nil, confirmationOpen: AppDelegate.shared?.confirmation != nil, hasDraft: !draftIDs.isEmpty)
    }
    var canCheck: Bool { ready && !busy && (channel != .direct || sparkleCanCheck) }
    let installedVersion = Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "—"
    var updateAvailable: Bool { latestVersion.map { $0.compare(installedVersion, options: .numeric) == .orderedDescending } ?? false }
    var supportsUpdates: Bool { channel == .direct || channel == .homebrew }

    func start() {
        guard !ready, operation == nil else { return }
        operation = Task { [weak self] in
            guard let self else { return }
            #if DEBUG
            let packaged = false
            #else
            let packaged = Bundle.main.object(forInfoDictionaryKey: "MomentumDistributionChannel") as? String == "direct"
            #endif
            let preview = ProcessInfo.processInfo.environment["MOMENTUM_DEMO"] != nil
            let key = Bundle.main.object(forInfoDictionaryKey: "SUPublicEDKey") as? String ?? ""
            var owner = false, ownershipUncertain = false
            if packaged && !preview {
                for path in ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"] {
                    let caskRoot = URL(fileURLWithPath: path).deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("Caskroom/momentum")
                    do { _ = try FileManager.default.attributesOfItem(atPath: caskRoot.path) }
                    catch let error as CocoaError where error.code == .fileReadNoSuchFile { continue }
                    catch { ownershipUncertain = true; continue }
                    guard FileManager.default.isExecutableFile(atPath: path) else { ownershipUncertain = true; continue }
                    let candidate = HomebrewUpdates(executable: URL(fileURLWithPath: path), appURL: Bundle.main.bundleURL)
                    do { _ = try await candidate.check(refresh: false); self.brew = candidate; owner = true; break }
                    catch { ownershipUncertain = true }
                }
            }
            self.ownershipUncertain = ownershipUncertain && !owner
            self.channel = DesktopUpdatePolicy.channel(packagedRelease: packaged, preview: preview, homebrewOwner: owner, ownershipUncertain: ownershipUncertain, publicKey: key)
            if self.channel == .direct {
                guard let url = URL(string: Bundle.main.object(forInfoDictionaryKey: "SUFeedURL") as? String ?? ""),
                      url.scheme == "https", url.host == "dan-hart.github.io" else {
                    self.channel = .unconfigured; self.ready = true; self.operation = nil; return
                }
                let controller = SPUStandardUpdaterController(startingUpdater: false, updaterDelegate: self, userDriverDelegate: nil)
                self.controller = controller
                self.observation = controller.updater.observe(\.canCheckForUpdates, options: [.initial, .new]) { [weak self] updater, _ in
                    let enabled = updater.canCheckForUpdates
                    Task { @MainActor in self?.sparkleCanCheck = enabled }
                }
                do { try controller.updater.start() }
                catch { self.channel = .unconfigured; self.controller = nil }
                controller.updater.automaticallyChecksForUpdates = self.automatic
                controller.updater.automaticallyDownloadsUpdates = false
            }
            self.ready = true; self.operation = nil; self.scheduleChecks(checkNow: true)
        }
    }
    private func scheduleChecks(checkNow: Bool) {
        timer?.invalidate(); timer = nil
        guard automatic, channel == .homebrew else { return }
        if checkNow { check() }
        // Only while Momentum is running. Opt-in schedules checks, never installs.
        timer = Timer.scheduledTimer(withTimeInterval: 86_400, repeats: true) { [weak self] _ in
            Task { @MainActor in self?.check() }
        }
    }
    func check() {
        guard canCheck else { return }
        if channel == .direct { controller?.checkForUpdates(nil); return }
        guard let brew else { return }
        busy = true; message = updateText("Checking for updates…")
        operation = Task { [weak self] in
            guard let self else { return }
            defer { self.busy = false; self.operation = nil }
            do {
                let info = try await brew.check(refresh: true)
                self.latestVersion = info.latestVersion
                self.message = self.updateAvailable ? updateText("An update is available.") : updateText("You’re up to date.")
            } catch {
                self.latestVersion = nil
                self.message = Task.isCancelled ? updateText("Update cancelled. Check again when you’re ready.") : updateText("Homebrew couldn’t complete this request. Try again, or update Momentum in Terminal.")
            }
        }
    }
    func install() {
        guard channel == .homebrew, !busy, updateAvailable, let brew else { return }
        busy = true; installing = true; message = updateText("Updating Momentum with Homebrew…")
        operation = Task { [weak self] in
            guard let self else { return }
            defer { self.busy = false; self.installing = false; self.operation = nil }
            do {
                try await brew.install()
                self.restartAvailable = true
                self.message = updateText("Update installed. Restart Momentum when you’re ready.")
            } catch {
                self.message = Task.isCancelled ? updateText("Update cancelled. Check again when you’re ready.") : updateText("Homebrew couldn’t complete this request. Try again, or update Momentum in Terminal.")
            }
        }
    }
    func cancel() { if !installing { operation?.cancel() } }
    func restart() {
        guard restartAvailable, !busy else { return }
        guard canRestart else {
            message = updateText("Finish editing tasks and wait for sync before restarting.")
            return
        }
        if let postponedInstall {
            self.postponedInstall = nil; restartAvailable = false; postponedInstall(); return
        }
        busy = true; restarting = true
        let process = Process(), pipe = Pipe()
        process.executableURL = Bundle.main.bundleURL.appendingPathComponent("Contents/MacOS/MomentumRelaunch")
        process.arguments = [String(ProcessInfo.processInfo.processIdentifier)]
        process.environment = [:]
        process.standardInput = FileHandle.nullDevice; process.standardOutput = pipe; process.standardError = FileHandle.nullDevice
        pipe.fileHandleForReading.readabilityHandler = { [weak self] handle in
            let data = handle.availableData
            handle.readabilityHandler = nil
            Task { @MainActor in
                guard let self else { return }
                guard data == Data("READY\n".utf8), self.canRestart else {
                    self.relaunchProcess?.terminate()
                    self.relaunchProcess = nil; self.relaunchPipe = nil
                    self.busy = false; self.restarting = false
                    self.message = updateText("Restart failed. Quit and reopen Momentum to use the update.")
                    return
                }
                NSApp.terminate(nil)
            }
        }
        do {
            try process.run(); relaunchProcess = process; relaunchPipe = pipe
            // If a damaged helper never produces the handshake, keep this app alive.
            Task { @MainActor [weak self] in
                try? await Task.sleep(for: .seconds(5))
                guard let self, self.relaunchProcess?.isRunning == true else { return }
                self.relaunchPipe?.fileHandleForReading.readabilityHandler = nil
                self.relaunchProcess?.terminate()
                self.relaunchProcess = nil; self.relaunchPipe = nil
                self.busy = false; self.restarting = false
                self.message = updateText("Restart failed. Quit and reopen Momentum to use the update.")
            }
        } catch {
            pipe.fileHandleForReading.readabilityHandler = nil
            busy = false; restarting = false; relaunchProcess = nil; relaunchPipe = nil
            message = updateText("Restart failed. Quit and reopen Momentum to use the update.")
        }
    }
    func updater(_ updater: SPUUpdater, shouldPostponeRelaunchForUpdate item: SUAppcastItem, untilInvokingBlock installHandler: @escaping () -> Void) -> Bool {
        guard !canRestart else { return false }
        postponedInstall = installHandler; restartAvailable = true
        message = updateText("Finish editing tasks and wait for sync before restarting.")
        return true
    }
    nonisolated func updater(_ updater: SPUUpdater, didFindValidUpdate item: SUAppcastItem) {
        let version = item.displayVersionString
        Task { @MainActor in self.latestVersion = version }
    }
}

func updateText(_ key: String) -> String { SettingsDiscovery.localized(key) }
