// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
import AVFoundation
import Foundation

@MainActor
protocol CompletionAudioClip: AnyObject {
    var volume: Float { get set }
    @discardableResult func play() -> Bool
    func stop()
}
extension AVAudioPlayer: CompletionAudioClip {}

/// One gain-controlled clip at a time. Audio failure never affects task mutations.
@MainActor
public final class CompletionSound: SoundPlayer {
    public static let shared = CompletionSound()
    public static let resourceName = CompletionSoundChoice.defaultChoice.rawValue
    public static let resourceExtension = "wav"
    public static var url: URL? { CompletionSoundChoice.defaultChoice.url }
    private var player: (any CompletionAudioClip)?
    private let makePlayer: @MainActor (URL) throws -> any CompletionAudioClip
    private let defaults: UserDefaults

    public init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
        self.makePlayer = { try AVAudioPlayer(contentsOf: $0) }
    }
    init(defaults: UserDefaults, makePlayer: @escaping @MainActor (URL) throws -> any CompletionAudioClip) {
        self.defaults = defaults; self.makePlayer = makePlayer
    }

    public static func boundedVolume(_ value: Double) -> Float {
        Float(value.isFinite ? min(1, max(0, value)) : 0.35)
    }

    public func playCompletion(_ choice: CompletionSoundChoice = .defaultChoice) {
        let volume = defaults.object(forKey: PrefKey.completionSoundVolume) as? Double ?? 0.35
        play(choice, volume: volume)
    }

    /// Preview does not inspect or mutate automatic feedback or selected sound.
    public func preview(_ choice: CompletionSoundChoice, volume: Double) { play(choice, volume: volume) }

    public func stop() { player?.stop(); player = nil }

    private func play(_ choice: CompletionSoundChoice, volume: Double) {
        stop()
        let gain = Self.boundedVolume(volume)
        guard gain > 0, let url = choice.url else { return }
        do {
            #if os(iOS)
            // Ambient mixes with other audio and respects the Ring/Silent switch.
            try AVAudioSession.sharedInstance().setCategory(.ambient, mode: .default)
            #endif
            let clip = try makePlayer(url)
            clip.volume = gain
            player = clip
            clip.play()
        } catch { player = nil }
    }
}
