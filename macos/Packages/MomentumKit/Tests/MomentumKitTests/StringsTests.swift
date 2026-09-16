// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// Every structured value the core hands over has to come out as text a person can read.
// These tests walk each enum exhaustively: if the core gains a case and this app forgets
// it, `Strings` stops compiling, and if a case maps to nothing these fail. That pairing
// is what keeps the two platforms from drifting apart quietly.
import Foundation
import MomentumCore
import Testing

@testable import MomentumKit

@Suite struct MessageWording {
    /// One of every `Message`, so a new case forces this list to be updated too.
    static let all: [Message] = [
        .taskCompleted, .taskCompletedArchived, .tasksCompleted(n: 2), .tasksCompletedArchived(n: 2),
        .taskDeleted, .tasksDeleted(n: 3), .taskAdded, .tasksAdded(n: 4), .taskDuplicated,
        .movedToTonight, .movedToMorning, .movedToToday,
        .tasksMovedToTonight(n: 2), .tasksMovedToMorning(n: 2), .tasksMovedToToday(n: 2),
        .movedToTomorrow, .movedToNextWeek, .tasksMovedToTomorrow(n: 2), .tasksMovedToNextWeek(n: 2),
        .plannedForToday, .plannedForMorning, .plannedForTonight, .removedFromToday,
        .tasksPlannedForToday(n: 5), .movedToProject(name: "Home"), .tagged(name: "urgent"),
        .tasksTagged(n: 2), .archived(n: 7), .snoozed(minutes: 60), .undone, .nothingToUndo,
        .manualOrderOnly, .pickAWeekday, .noLongerRepeats(title: "Standup"),
        .repeatSaved(description: .daily), .backupImported, .backupExported,
        .synced(opsUploaded: 3), .linkedWith(name: "iMac"), .linking, .linkFailed, .linkDeclined,
        .identityChanged(name: "iMac"), .nearbyStartFailed(error: "port in use"),
    ]

    @Test func everyMessageHasWording() {
        for m in Self.all {
            let text = Strings.message(m)
            #expect(!text.isEmpty, "\(m) has no wording")
            #expect(!text.contains("%"), "\(m) left a format placeholder in: \(text)")
        }
    }

    @Test func countsAndNamesReachTheReader() {
        #expect(Strings.message(.tasksDeleted(n: 3)).contains("3"))
        #expect(Strings.message(.movedToProject(name: "Home")).contains("Home"))
        #expect(Strings.message(.tagged(name: "urgent")).contains("urgent"))
        #expect(Strings.message(.snoozed(minutes: 60)).contains("60"))
        #expect(Strings.message(.synced(opsUploaded: 3)).contains("3"))
        #expect(Strings.message(.noLongerRepeats(title: "Standup")).contains("Standup"))
    }

    @Test func singularAndPluralReadDifferently() {
        #expect(Strings.message(.taskDeleted) != Strings.message(.tasksDeleted(n: 2)))
        #expect(Strings.message(.movedToTonight) != Strings.message(.tasksMovedToTonight(n: 2)))
    }

    @Test func everyMessageNamesTheUndoItWouldReverse() {
        for m in Self.all {
            #expect(!Strings.undoName(m).isEmpty)
        }
        #expect(Strings.undoName(.taskDeleted) != Strings.undoName(.archived(n: 1)))
    }
}

@Suite struct DayAndTimeWording {
    @Test func relativeDaysReadNaturally() {
        #expect(Strings.day(dayLabel(day: today())) == String(localized: "Today"))
        #expect(Strings.day(dayLabel(day: dayOffset(day: today(), days: 1))) == String(localized: "Tomorrow"))
        #expect(Strings.day(dayLabel(day: dayOffset(day: today(), days: -1))) == String(localized: "Yesterday"))
        let soon = Strings.day(dayLabel(day: dayOffset(day: today(), days: 3)))
        #expect(!soon.isEmpty && soon.rangeOfCharacter(from: .decimalDigits) == nil, "a weekday name: \(soon)")
        let far = Strings.day(dayLabel(day: dayOffset(day: today(), days: 400)))
        #expect(far.rangeOfCharacter(from: .decimalDigits) != nil, "a real date: \(far)")
    }

    @Test func anUnparsableDayIsShownAsItIs() {
        #expect(Strings.day(dayLabel(day: "not a day")) == "not a day")
    }

    @Test func clockTimesUseTheReadersOwnFormat() {
        let text = Strings.time(ClockTime(hour: 15, minute: 30))
        #expect(text.contains("15") || text.lowercased().contains("3"), "15:30 or 3:30 PM, not \(text)")
        #expect(!Strings.time(ClockTime(hour: 0, minute: 5)).isEmpty)
    }

    @Test func agoCountsUpInTheRightUnits() {
        let now = nowMs()
        #expect(Strings.ago(now, now: now) == String(localized: "just now"))
        #expect(Strings.ago(now - 30_000, now: now).contains("30"))
        #expect(Strings.ago(now - 5 * 60_000, now: now).contains("5"))
        #expect(Strings.ago(now - 2 * 3_600_000, now: now).contains("2"))
        #expect(Strings.ago(now - 3 * 86_400_000, now: now).contains("3"))
    }

    @Test func estimatesFormatAndParseBackToTheSameValue() {
        #expect(Strings.estimate(5_400_000) == "1h 30m")
        #expect(Strings.estimate(600_000) == "10m")
        #expect(parseEstimate(text: Strings.estimate(8_100_000)) == 8_100_000)
    }
}

@Suite struct RepeatWording {
    static let all: [RepeatDescription] = [
        .daily, .everyNDays(n: 3), .everyWeekday(weekday: 1),
        .weekly(n: 1, weekdays: [1, 3]), .weekly(n: 2, weekdays: [1]),
        .monthly(n: 1, rule: .dayOfMonth(day: 5)), .monthly(n: 1, rule: .sameDay),
        .monthly(n: 1, rule: .lastDay), .monthly(n: 2, rule: .nthWeekday(week: 2, weekday: 2)),
        .monthly(n: 1, rule: .nthWeekday(week: -1, weekday: 5)),
        .yearly(n: 1, month: 3, day: 5), .yearly(n: 2, month: 12, day: 31), .repeats,
    ]

    @Test func everyScheduleIsDescribedInPlainWords() {
        for r in Self.all {
            let text = Strings.repeatText(r)
            #expect(!text.isEmpty, "\(r) has no description")
            #expect(!text.contains("%"), "\(r): \(text)")
        }
    }

    @Test func schedulesThatDifferReadDifferently() {
        #expect(Strings.repeatText(.daily) != Strings.repeatText(.everyNDays(n: 3)))
        #expect(Strings.repeatText(.weekly(n: 1, weekdays: [1, 3])) != Strings.repeatText(.weekly(n: 2, weekdays: [1, 3])))
        #expect(Strings.repeatText(.monthly(n: 1, rule: .lastDay))
            != Strings.repeatText(.monthly(n: 1, rule: .dayOfMonth(day: 5))))
    }

    @Test func theNthWeekdayRuleNamesBothTheWeekAndTheDay() {
        let text = Strings.monthlyRuleText(.nthWeekday(week: 2, weekday: 2))
        #expect(text.contains(Strings.weekdayName(2)))
        #expect(text != Strings.monthlyRuleText(.nthWeekday(week: -1, weekday: 2)), "the last is not the second")
    }
}

@Suite struct ViewWording {
    @Test func everyViewTitleHasAName() {
        let titles: [ViewTitle] = [.today, .morning, .tonight, .comingUp, .archive, .search, .named(name: "Home")]
        for t in titles {
            #expect(!Strings.viewTitle(t).isEmpty)
        }
        #expect(Strings.viewTitle(.named(name: "Home")) == "Home")
    }

    @Test func everySidebarEntryHasAName() {
        for v: View in [.today, .morning, .tonight, .upcoming, .archive, .search] {
            #expect(!Strings.sidebarTitle(v).isEmpty)
        }
        #expect(Strings.sidebarTitle(.project(id: "x")).isEmpty, "projects carry their own name")
    }

    @Test func sectionHeadingsCarryTheirCount() {
        let overdue = Section(group: nil, kind: .overdue, count: 3, rows: [], note: nil)
        #expect(Strings.sectionTitle(overdue)?.contains("3") == true)
        let plain = Section(group: nil, kind: .plain, count: 2, rows: [], note: nil)
        #expect(Strings.sectionTitle(plain) == nil, "an ungrouped list has no heading")
        let day = Section(group: nil, kind: .day(label: dayLabel(day: today())), count: 1, rows: [], note: nil)
        #expect(Strings.sectionTitle(day) == String(localized: "Today"))
    }

    @Test func aCappedSectionSaysHowMuchIsHidden() {
        let narrow = Strings.sectionNote(SectionNote(shown: 60, total: 140, suggestNarrowing: true))
        #expect(narrow.contains("60") && narrow.contains("140"))
        let plain = Strings.sectionNote(SectionNote(shown: 30, total: 90, suggestNarrowing: false))
        #expect(plain.contains("30") && plain != narrow)
    }

    @Test func everyEmptyStateHasAnIconTitleAndNextStep() {
        let states: [EmptyState] = [
            .today, .morning, .tonight, .upcoming(days: 7), .archive, .search, .noResults,
            .project(name: "Home"), .tag(name: "urgent"),
        ]
        for s in states {
            let copy = Strings.empty(s, modifier: "⌘", syncConfigured: true)
            #expect(!copy.symbol.isEmpty && !copy.title.isEmpty && !copy.description.isEmpty, "\(s)")
        }
        #expect(Strings.empty(.project(name: "Home"), modifier: "⌘", syncConfigured: true).title.contains("Home"))
        #expect(Strings.empty(.tag(name: "urgent"), modifier: "⌘", syncConfigured: true).title.contains("urgent"))
        #expect(Strings.empty(.upcoming(days: 30), modifier: "⌘", syncConfigured: true).description.contains("30"))
    }

    @Test func allDoneCopyChangesWithTheView() {
        let today = Strings.allDone(.today(completed: 4))
        #expect(today.description.contains("4"))
        #expect(today.title != Strings.allDone(.tonight).title)
        #expect(!Strings.allDone(.context).title.isEmpty)
        #expect(!Strings.allDone(.morning).title.isEmpty)
    }
}

@Suite struct ErrorWording {
    @Test func everyCoreErrorIsExplained() {
        let errors: [CoreError] = [
            .NotConfigured, .Actionable(message: "wrong password"), .Transient(message: "network down"),
            .Busy, .Io(message: "no such file"), .Invalid(message: "bad json"), .Nearby(message: "port in use"),
        ]
        for e in errors {
            #expect(!Strings.error(e).isEmpty, "\(e)")
        }
        #expect(Strings.error(CoreError.Actionable(message: "wrong password")).contains("wrong password"))
    }
}
