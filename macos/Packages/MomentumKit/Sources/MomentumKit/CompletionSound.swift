// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// Short, bundled CC0 feedback shared by both Apple apps (see NOTICE.md). System
// sounds mix with other audio, use alert volume on Mac, and follow Ring/Silent on
// iPhone. Missing resources and registration failures simply remain silent.
import AudioToolbox
import Foundation

@MainActor
public final class CompletionSound: SoundPlayer {
    public static let shared = CompletionSound()
    public static let resourceName = CompletionSoundChoice.defaultChoice.rawValue
    public static let resourceExtension = "wav"

    /// The default resource, retained for callers that do not choose a sound.
    public static var url: URL? { CompletionSoundChoice.defaultChoice.url }

    private var soundIDs: [CompletionSoundChoice: SystemSoundID] = [:]
    private var registrationFailures: Set<CompletionSoundChoice> = []

    public init() {}

    deinit {
        for id in soundIDs.values { AudioServicesDisposeSystemSoundID(id) }
    }

    public func playCompletion(_ choice: CompletionSoundChoice = .defaultChoice) {
        guard let id = register(choice) else { return }
        AudioServicesPlaySystemSound(id)
    }

    /// Register each selected clip on first use and keep its ID for this player's life.
    private func register(_ choice: CompletionSoundChoice) -> SystemSoundID? {
        if let id = soundIDs[choice] { return id }
        guard !registrationFailures.contains(choice), let url = choice.url else {
            registrationFailures.insert(choice)
            return nil
        }
        var id: SystemSoundID = 0
        let status = AudioServicesCreateSystemSoundID(url as CFURL, &id)
        guard status == kAudioServicesNoError else {
            registrationFailures.insert(choice)
            return nil
        }
        soundIDs[choice] = id
        return id
    }
}
