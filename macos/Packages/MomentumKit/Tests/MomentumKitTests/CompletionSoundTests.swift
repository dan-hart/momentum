// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
import AudioToolbox
import CryptoKit
import Foundation
import Testing
@testable import MomentumKit

@MainActor @Suite struct CompletionSoundTests {
    @Test func theCatalogPreservesLegacyIDsAndAddsTenCandidates() {
        #expect(CompletionSoundChoice.allCases.filter(\.isLegacy).map(\.rawValue) == [
            "wood-pop", "double-check", "rounded-lift", "pocket-piano", "string-flick",
            "bright-finish", "harp-snap", "click-and-glow", "tiny-ta-da", "warm-spark",
        ])
        #expect(CompletionSoundChoice.defaultChoice == .littleLift)
        #expect(CompletionSoundChoice.resolve(nil) == .littleLift)
        #expect(CompletionSoundChoice.resolve("unknown") == .littleLift)
        #expect(CompletionSoundChoice.candidates.count == 10)
        for (index, choice) in CompletionSoundChoice.allCases.filter(\.isLegacy).enumerated() {
            #expect(choice.label.hasPrefix("\(index + 1) "))
        }
    }

    @Test func everyClipCanRegisterAsAnAppleSystemSoundWithoutPlaying() throws {
        for choice in CompletionSoundChoice.allCases {
            let url = try #require(choice.url)
            var id: SystemSoundID = 0
            let status = AudioServicesCreateSystemSoundID(url as CFURL, &id)
            #expect(status == kAudioServicesNoError)
            if status == kAudioServicesNoError { AudioServicesDisposeSystemSoundID(id) }
        }
    }

    @Test func everyApprovedClipShipsUnchangedAndDistinct() throws {
        let clips: [(String, String)] = [
            ("wood-pop", "e38ab4a1d0216bac2e88aad767d5027ab6339caa83300a86947b956d2e594b28"),
            ("double-check", "6d6ad0d6f3a7cac492294b46b3e319af1f3250eeb25e41ed912a4348c1863368"),
            ("rounded-lift", "1c62315ecef6cca1bcd09e605e61124d36baf5c47ea1b00121dd8a8cb41566bc"),
            ("pocket-piano", "7ff557a44654bb3ef8d769c48eaa31ea16d7c7dbe628792882392426b0bf0721"),
            ("string-flick", "4bfd34325bbbc7f2baea000335dcfd89bb376f8b0acf695b298c976687c5e64d"),
            ("bright-finish", "87903557cd5d790e37019358b70ce213373e7988a6421b90f8097b1ba243527f"),
            ("harp-snap", "b8e065b15e28e1b486face48e2ca3dfb3db623d4845f1d19afb768838e60dd45"),
            ("click-and-glow", "a86dbb5689e7b31489849255e597bc3ae7aa4474f98d3b96e2a76ea7c898762b"),
            ("tiny-ta-da", "58ad4aed4ad6ebd5f737604f5595ec4efa26f196ee1132602fba7d1d5145c9a2"),
            ("warm-spark", "3e3ba756e1679e51465842450c58daf0ad39be82eeeff124a756c302c51fdd6e"),
        ]
        var seen: Set<String> = []
        for (slug, expectedHash) in clips {
            let choice = try #require(CompletionSoundChoice(rawValue: slug))
            let file = try #require(choice.url)
            #expect(FileManager.default.fileExists(atPath: file.path), "Missing approved clip: \(slug)")
            guard FileManager.default.fileExists(atPath: file.path) else { continue }
            let data = try Data(contentsOf: file)
            let hash = SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
            #expect(hash == expectedHash, "The approved master must remain byte-for-byte intact")
            #expect(seen.insert(hash).inserted, "Each option must be a distinct sound")
        }
        #expect(seen.count == 10)
    }
}

@MainActor @Suite struct QuietCompletionSoundTests {
    @Test func gainIsBoundedAndSilentAtZero() {
        #expect(CompletionSound.boundedVolume(-1) == 0)
        #expect(CompletionSound.boundedVolume(0) == 0)
        #expect(CompletionSound.boundedVolume(0.35) == 0.35)
        #expect(CompletionSound.boundedVolume(8) == 1)
        #expect(CompletionSound.boundedVolume(.nan) == 0.35)
    }
    @Test func previewDoesNotChangeSavedChoiceOrAutomaticPreference() {
        let name = UUID().uuidString
        let defaults = UserDefaults(suiteName: name)!
        defer { defaults.removePersistentDomain(forName: name) }
        defaults.set(false, forKey: PrefKey.completionSound)
        defaults.set("future-unknown", forKey: PrefKey.completionSoundChoice)
        let player = CompletionSound(defaults: defaults)
        player.preview(.softKnock, volume: 0)
        player.stop()
        #expect(defaults.bool(forKey: PrefKey.completionSound) == false)
        #expect(defaults.string(forKey: PrefKey.completionSoundChoice) == "future-unknown")
    }
}

@MainActor private final class RecordingCompletionClip: CompletionAudioClip {
    var volume: Float = 1
    var plays = 0
    var stops = 0
    func play() -> Bool { plays += 1; return true }
    func stop() { stops += 1 }
}

@MainActor @Suite struct CompletionAuditionTests {
    @Test func unsavedPreviewIgnoresAutomaticToggleAndReplacesPriorPlayback() {
        let name = UUID().uuidString
        let defaults = UserDefaults(suiteName: name)!
        defer { defaults.removePersistentDomain(forName: name) }
        defaults.set(false, forKey: PrefKey.completionSound)
        defaults.set("wood-pop", forKey: PrefKey.completionSoundChoice)
        var clips: [RecordingCompletionClip] = []
        var paths: [String] = []
        let player = CompletionSound(defaults: defaults) { url in
            let clip = RecordingCompletionClip(); clips.append(clip); paths.append(url.lastPathComponent)
            return clip
        }
        player.preview(.softKnock, volume: 0.35)
        #expect(paths == ["soft-knock.wav"])
        #expect(clips.first?.plays == 1)
        #expect(clips.first?.volume == 0.35)
        player.preview(.littleLift, volume: 0.5)
        #expect(clips.count == 2)
        #expect(clips.first?.stops == 1)
        #expect(clips.last?.volume == 0.5)
        player.stop()
        #expect(clips.last?.stops == 1)
        #expect(!defaults.bool(forKey: PrefKey.completionSound))
        #expect(defaults.string(forKey: PrefKey.completionSoundChoice) == "wood-pop")
    }
    @Test func completionAndPreviewUseTheSameGainWithoutOverlapping() {
        let name = UUID().uuidString
        let defaults = UserDefaults(suiteName: name)!
        defer { defaults.removePersistentDomain(forName: name) }
        defaults.set(0.2, forKey: PrefKey.completionSoundVolume)
        var clips: [RecordingCompletionClip] = []
        let player = CompletionSound(defaults: defaults) { _ in
            let clip = RecordingCompletionClip(); clips.append(clip); return clip
        }
        player.playCompletion(.woodPop)
        player.preview(.softKnock, volume: defaults.double(forKey: PrefKey.completionSoundVolume))
        #expect(clips.count == 2)
        #expect(clips.allSatisfy { $0.volume == 0.2 })
        #expect(clips.first?.stops == 1)
        player.stop()
    }
    @Test func zeroGainCancelsPreviousSoundWithoutStartingAnother() {
        let name = UUID().uuidString
        let defaults = UserDefaults(suiteName: name)!
        defer { defaults.removePersistentDomain(forName: name) }
        let clip = RecordingCompletionClip()
        var loads = 0
        let player = CompletionSound(defaults: defaults) { _ in loads += 1; return clip }
        player.preview(.softKnock, volume: 0.5)
        player.preview(.littleLift, volume: 0)
        #expect(loads == 1)
        #expect(clip.plays == 1)
        #expect(clip.stops == 1)
    }
}
