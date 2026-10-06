// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// The completion chime both Apple apps share. The clip is Kenney's "Interface Sounds"
// `confirmation_001` (CC0-1.0, see NOTICE.md), bundled with this package. It plays as a
// system sound: short, mixed with whatever else is playing, at the alert volume on the
// Mac and subject to the Ring/Silent switch on iPhone. Nothing here blocks or throws; a
// missing resource or a failed registration simply plays nothing.
import AudioToolbox
import Foundation

@MainActor
public final class CompletionSound: SoundPlayer {
    public static let shared = CompletionSound()
    public static let resourceName = "task-complete"
    public static let resourceExtension = "wav"

    /// Where the bundled clip lives, if the package resources are present.
    public static var url: URL? {
        Bundle.module.url(forResource: resourceName, withExtension: resourceExtension)
    }

    private var soundID: SystemSoundID?
    private var registrationFailed = false

    public init() {}

    deinit {
        if let id = soundID { AudioServicesDisposeSystemSoundID(id) }
    }

    public func playCompletion() {
        guard let id = register() else { return }
        AudioServicesPlaySystemSound(id)
    }

    /// Registers the clip on first use and keeps the id for the life of the player.
    private func register() -> SystemSoundID? {
        if let id = soundID { return id }
        guard !registrationFailed, let url = Self.url else {
            registrationFailed = true
            return nil
        }
        var id: SystemSoundID = 0
        let status = AudioServicesCreateSystemSoundID(url as CFURL, &id)
        guard status == kAudioServicesNoError else {
            registrationFailed = true
            return nil
        }
        soundID = id
        return id
    }
}
