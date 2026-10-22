// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import Testing
@testable import MomentumKit

@Suite(.serialized) struct CompletionMessageTests {
    @Test func catalogHasOneHundredTranslatedMessages() {
        for id in 1...100 {
            let en = CompletionMessageSelection.text(id: id, locale: Locale(identifier: "en"))
            let de = CompletionMessageSelection.text(id: id, locale: Locale(identifier: "de"))
            #expect(!en.hasPrefix("completion.message."))
            #expect(!de.hasPrefix("completion.message."))
            #expect(en != de)
        }
    }
    @Test func copyStaysStableAndDisappearsWhenTaskReopens() {
        var selection = CompletionMessageSelection()
        let date = Date(timeIntervalSince1970: 1_800_000_000)
        selection.update(view: .morning, completion: .periodComplete, date: date)
        let selected = selection.id
        #expect((1...20).contains(selected ?? 0))
        selection.update(view: .morning, completion: .periodComplete, date: date)
        #expect(selection.id == selected)
        selection.update(view: .morning, completion: .pending, date: date)
        #expect(selection.id == nil)
    }
    @Test func returningToCompletedDestinationDoesNotImmediatelyRepeat() {
        var first = CompletionMessageSelection()
        first.update(view: .morning, completion: .periodComplete)
        var returned = CompletionMessageSelection()
        returned.update(view: .morning, completion: .periodComplete)
        #expect(first.id != returned.id)
    }
    @Test func retainedDestinationStartsANewVisitWithoutImmediateRepeat() {
        var selection = CompletionMessageSelection()
        selection.beginVisit(view: .morning, completion: .periodComplete)
        let first = selection.id
        selection.beginVisit(view: .morning, completion: .periodComplete)
        #expect(selection.id != first)
    }
    @Test func hiddenTodayPendingCannotClaimCompletion() {
        var selection = CompletionMessageSelection()
        selection.update(view: .today, completion: .pending)
        #expect(selection.text == nil)
        selection.update(view: .today, completion: .unplanned)
        #expect(selection.text == nil)
    }
}

@MainActor @Suite struct LocalFeedbackPreferenceTests {
    @Test func localFeedbackControlsStayOutOfPortableSettings() throws {
        let name = UUID().uuidString
        let defaults = UserDefaults(suiteName: name)!
        defer { defaults.removePersistentDomain(forName: name) }
        Preferences.register(defaults, domain: name)
        #expect(defaults.bool(forKey: PrefKey.dynamicEmptyMessages))
        #expect(defaults.double(forKey: PrefKey.completionSoundVolume) == 0.35)
        defaults.set(false, forKey: PrefKey.dynamicEmptyMessages)
        defaults.set(0.8, forKey: PrefKey.completionSoundVolume)
        defaults.set("future-sound", forKey: PrefKey.completionSoundChoice)
        let json = try PortableAppSettings.encode(defaults)
        #expect(!json.contains(PrefKey.dynamicEmptyMessages))
        #expect(!json.contains(PrefKey.completionSoundVolume))
        #expect(json.contains("future-sound"))
    }
}

@Suite struct CelebrationFeedbackTests {
    @Test func incompleteScopesAndDisabledMessagesHaveNoCelebration() {
        for completion in [DayCompletion.unplanned, .pending] {
            #expect(CelebrationFeedback.presentation(dynamic: true, completion: completion, reduceMotion: false) == .hidden)
        }
        #expect(CelebrationFeedback.presentation(dynamic: false, completion: .dayComplete, reduceMotion: false) == .hidden)
    }
    @Test func reducedMotionUsesAStaticManualAlternative() {
        #expect(CelebrationFeedback.presentation(dynamic: true, completion: .periodComplete, reduceMotion: true) == .staticSymbol)
        #expect(CelebrationFeedback.presentation(dynamic: true, completion: .dayComplete, reduceMotion: false) == .burst)
    }
}

@Suite struct CompletedListPresentationTests {
    @Test func visibleRowsFromPriorDaysNeverImplyTodaysCompletion() {
        for view in [MomentumCore.View.today, .morning, .tonight] {
            #expect(CompletedListPresentation.resolve(view: view, completion: .unplanned) == .neutralDay)
            #expect(CompletedListPresentation.resolve(view: view, completion: .pending) == .neutralDay)
            #expect(CompletedListPresentation.resolve(view: view, completion: .periodComplete) == .dayCompletion)
        }
        #expect(CompletedListPresentation.resolve(view: .project(id: "fixture"), completion: .unplanned) == .legacyList)
    }
}
