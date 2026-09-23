// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// Every piece of text the core hands over as data becomes a string here, in the user's
// language and date format. Nothing else in the app formats a day, a time or a toast.
import Foundation
import MomentumCore

public enum Strings {
    // MARK: Days and times

    private static let dayFormatterNoYear: DateFormatter = {
        let f = DateFormatter()
        f.setLocalizedDateFormatFromTemplate("d MMMM")
        return f
    }()
    private static let dayFormatterWithYear: DateFormatter = {
        let f = DateFormatter()
        f.setLocalizedDateFormatFromTemplate("d MMMM yyyy")
        return f
    }()
    private static let isoDay: DateFormatter = {
        let f = DateFormatter()
        f.calendar = Calendar(identifier: .gregorian)
        f.locale = Locale(identifier: "en_US_POSIX")
        f.dateFormat = "yyyy-MM-dd"
        return f
    }()
    private static let timeFormatter: DateFormatter = {
        let f = DateFormatter()
        f.timeStyle = .short
        f.dateStyle = .none
        return f
    }()

    public static func date(fromDay day: String) -> Date? {
        isoDay.date(from: day)
    }
    public static func day(fromDate date: Date) -> String {
        isoDay.string(from: date)
    }

    /// "Today", "Tomorrow", "Friday", "14 October", "3 January 2027".
    public static func day(_ label: DayLabel) -> String {
        switch label.relation {
        case .today: return String(localized: "Today", bundle: .module)
        case .tomorrow: return String(localized: "Tomorrow", bundle: .module)
        case .yesterday: return String(localized: "Yesterday", bundle: .module)
        case .thisWeek:
            let symbols = Calendar.current.standaloneWeekdaySymbols
            return symbols[Int(label.weekday) % 7]
        case .thisYear:
            guard let d = date(fromDay: label.day) else { return label.day }
            return dayFormatterNoYear.string(from: d)
        case .other:
            guard let d = date(fromDay: label.day) else { return label.day }
            return dayFormatterWithYear.string(from: d)
        }
    }
    public static func day(_ day: String) -> String {
        Self.day(dayLabel(day: day))
    }

    /// The user's own clock style: "15:30" or "3:30 PM".
    public static func time(_ t: ClockTime) -> String {
        var comps = DateComponents()
        comps.year = 2000
        comps.month = 1
        comps.day = 1
        comps.hour = Int(t.hour)
        comps.minute = Int(t.minute)
        guard let d = Calendar.current.date(from: comps) else {
            return String(format: "%02d:%02d", t.hour, t.minute)
        }
        return timeFormatter.string(from: d)
    }

    /// "just now", "5 minutes ago", … for a past epoch-millisecond timestamp.
    public static func ago(_ ms: UInt64, now: UInt64 = nowMs()) -> String {
        let secs = now > ms ? (now - ms) / 1000 : 0
        switch secs {
        case 0...9: return String(localized: "just now", bundle: .module)
        case 10...59: return String(localized: "\(secs) seconds ago", bundle: .module)
        case 60...3599: return String(localized: "\(secs / 60) minutes ago", bundle: .module)
        case 3600...86399: return String(localized: "\(secs / 3600) hours ago", bundle: .module)
        default: return String(localized: "\(secs / 86400) days ago", bundle: .module)
        }
    }

    public static func estimate(_ ms: Double) -> String {
        formatEstimate(ms: ms)
    }

    // MARK: Repeats

    public static func weekdayName(_ weekday: UInt32) -> String {
        Calendar.current.standaloneWeekdaySymbols[Int(weekday) % 7]
    }
    public static func shortWeekdayName(_ weekday: UInt32) -> String {
        Calendar.current.shortStandaloneWeekdaySymbols[Int(weekday) % 7]
    }

    public static func repeatText(_ r: RepeatDescription) -> String {
        switch r {
        case .daily: return String(localized: "Repeats daily", bundle: .module)
        case .everyNDays(let n): return String(localized: "Repeats every \(n) days", bundle: .module)
        case .everyWeekday(let wd): return String(localized: "Repeats every \(weekdayName(wd))", bundle: .module)
        case .weekly(let n, let days):
            let list = days.map(shortWeekdayName).joined(separator: ", ")
            return n == 1
                ? String(localized: "Repeats weekly on \(list)", bundle: .module)
                : String(localized: "Repeats every \(n) weeks on \(list)", bundle: .module)
        case .monthly(let n, let rule):
            let day = monthlyRuleText(rule)
            return n == 1
                ? String(localized: "Repeats monthly on \(day)", bundle: .module)
                : String(localized: "Repeats every \(n) months on \(day)", bundle: .module)
        case .yearly(let n, let month, let day):
            var comps = DateComponents()
            comps.year = 2001
            comps.month = Int(month)
            comps.day = Int(day)
            let date = Calendar.current.date(from: comps).map { dayFormatterNoYear.string(from: $0) } ?? "\(day)/\(month)"
            return n == 1
                ? String(localized: "Repeats yearly on \(date)", bundle: .module)
                : String(localized: "Repeats every \(n) years on \(date)", bundle: .module)
        case .repeats: return String(localized: "Repeats", bundle: .module)
        }
    }

    public static func monthlyRuleText(_ rule: MonthlyRule) -> String {
        switch rule {
        case .dayOfMonth(let d):
            let ordinal = NumberFormatter.localizedString(from: NSNumber(value: Int(d)), number: .ordinal)
            return String(localized: "the \(ordinal)", bundle: .module)
        case .sameDay: return String(localized: "the same day", bundle: .module)
        case .lastDay: return String(localized: "the last day", bundle: .module)
        case .nthWeekday(let week, let wd):
            let which: String
            switch week {
            case 1: which = String(localized: "the first", bundle: .module)
            case 2: which = String(localized: "the second", bundle: .module)
            case 3: which = String(localized: "the third", bundle: .module)
            case 4: which = String(localized: "the fourth", bundle: .module)
            default: which = String(localized: "the last", bundle: .module)
            }
            return "\(which) \(weekdayName(wd))"
        }
    }

    /// The caption before the first sync: one place, so the view and its test agree.
    public static var notSyncedYet: String { String(localized: "Not synced yet", bundle: .module) }

    // MARK: Views, sections, empty states

    public static func viewTitle(_ t: ViewTitle) -> String {
        switch t {
        case .today: return String(localized: "Today", bundle: .module)
        case .morning: return String(localized: "Morning", bundle: .module)
        case .tonight: return String(localized: "Tonight", bundle: .module)
        case .comingUp: return String(localized: "Coming Up", bundle: .module)
        case .archive: return String(localized: "Archive", bundle: .module)
        case .search: return String(localized: "Search", bundle: .module)
        case .named(let name): return name
        }
    }

    public static func sidebarTitle(_ view: View) -> String {
        switch view {
        case .today: return String(localized: "Today", bundle: .module)
        case .morning: return String(localized: "Morning", bundle: .module)
        case .tonight: return String(localized: "Tonight", bundle: .module)
        case .upcoming: return String(localized: "Coming Up", bundle: .module)
        case .archive: return String(localized: "Archive", bundle: .module)
        case .search: return String(localized: "Search", bundle: .module)
        case .project, .tag: return ""
        }
    }

    public static func sectionTitle(_ s: Section) -> String? {
        let base = sectionContextTitle(s)
        guard let group = s.group else { return base }
        let title = groupTitle(group)
        return base.map { "\($0) · \(title)" } ?? title
    }

    public static func groupTitle(_ group: TaskGroup) -> String {
        switch group {
        case .today: return String(localized: "Today", bundle: .module)
        case .morning: return String(localized: "Morning", bundle: .module)
        case .evening: return String(localized: "Evening", bundle: .module)
        case .project(_, let title, _): return title
        case .tag(_, let title, _): return "#" + title
        case .noProject: return String(localized: "No Project", bundle: .module)
        case .untagged: return String(localized: "Untagged", bundle: .module)
        case .estimate(let range):
            switch range {
            case .upTo15Minutes: return String(localized: "Up to 15 min", bundle: .module)
            case .upTo30Minutes: return String(localized: "16–30 min", bundle: .module)
            case .upTo60Minutes: return String(localized: "31–60 min", bundle: .module)
            case .upTo2Hours: return String(localized: "1–2 hours", bundle: .module)
            case .over2Hours: return String(localized: "Over 2 hours", bundle: .module)
            case .noEstimate: return String(localized: "No estimate", bundle: .module)
            }
        }
    }

    public static func sectionContextTitle(_ s: Section) -> String? {
        switch s.kind {
        case .plain: return nil
        case .overdue: return String(localized: "Overdue (\(s.count))", bundle: .module)
        case .morning: return String(localized: "Morning", bundle: .module)
        case .today: return String(localized: "Today", bundle: .module)
        case .tonight: return String(localized: "Tonight", bundle: .module)
        case .day(let label): return day(label)
        case .completed: return String(localized: "Completed (\(s.count))", bundle: .module)
        case .searchTasks: return String(localized: "Tasks (\(s.count))", bundle: .module)
        case .searchProjects: return String(localized: "Projects", bundle: .module)
        case .searchTags: return String(localized: "Tags", bundle: .module)
        case .searchArchived: return String(localized: "Archived (\(s.count))", bundle: .module)
        }
    }

    public static func sectionNote(_ n: SectionNote) -> String {
        n.suggestNarrowing
            ? String(localized: "Showing \(n.shown) of \(n.total). Add another word to narrow it down.", bundle: .module)
            : String(localized: "Showing \(n.shown) of \(n.total).", bundle: .module)
    }

    public struct EmptyCopy {
        public let symbol: String
        public let title: String
        public let description: String
    }

    /// What an empty view says; `modifier` is the configured shortcut modifier's symbol
    /// and `syncConfigured` decides the hint's ending.
    public static func empty(_ e: EmptyState, modifier: String, syncConfigured: Bool) -> EmptyCopy {
        let add = syncConfigured
            ? String(localized: "Add a task above, or press \(modifier)N.", bundle: .module)
            : String(localized: "Add a task above, press \(modifier)N, or turn on sync in Settings to bring in your tasks.", bundle: .module)
        switch e {
        case .today:
            return EmptyCopy(symbol: "star", title: String(localized: "Nothing planned for today", bundle: .module),
                             description: add + " " + String(localized: "Drag tasks here from Coming Up, or press \(modifier)T on any task.", bundle: .module))
        case .morning:
            return EmptyCopy(symbol: "sun.max", title: String(localized: "Nothing planned for the morning", bundle: .module),
                             description: String(localized: "Tag a task “Morning”, or press \(modifier)⇧M on a task to move it here.", bundle: .module))
        case .tonight:
            return EmptyCopy(symbol: "moon.stars", title: String(localized: "Nothing planned for tonight", bundle: .module),
                             description: String(localized: "Tag a task “Evening”, or press \(modifier)⇧T on a task to move it here.", bundle: .module))
        case .upcoming(let days):
            return EmptyCopy(symbol: "calendar", title: String(localized: "Nothing coming up", bundle: .module),
                             description: String(localized: "Tasks due in the next \(days) days appear here. Set a due day in a task's details.", bundle: .module))
        case .archive:
            return EmptyCopy(symbol: "archivebox", title: String(localized: "No archived tasks", bundle: .module),
                             description: String(localized: "Completed tasks land here when you archive them with \(modifier)E.", bundle: .module))
        case .search:
            return EmptyCopy(symbol: "magnifyingglass", title: String(localized: "Search Everything", bundle: .module),
                             description: String(localized: "Tasks, notes, subtasks, projects, tags and the archive", bundle: .module))
        case .noResults:
            return EmptyCopy(symbol: "magnifyingglass", title: String(localized: "No Results Found", bundle: .module),
                             description: String(localized: "Try a different search", bundle: .module))
        case .project(let name):
            return EmptyCopy(symbol: "folder", title: String(localized: "No tasks in \(name)", bundle: .module),
                             description: add + " " + String(localized: "Drag tasks here from any other view.", bundle: .module))
        case .tag(let name):
            return EmptyCopy(symbol: "tag", title: String(localized: "No tasks tagged #\(name)", bundle: .module),
                             description: add + " " + String(localized: "Drag tasks here to tag them.", bundle: .module))
        }
    }

    public static func allDone(_ a: AllDone) -> (title: String, description: String) {
        switch a {
        case .today(let completed):
            return (String(localized: "All done for today", bundle: .module),
                    String(localized: "You completed \(completed) tasks. Time to switch off.", bundle: .module))
        case .morning:
            return (String(localized: "Morning done", bundle: .module), String(localized: "The rest of the day is yours.", bundle: .module))
        case .tonight:
            return (String(localized: "All done for tonight", bundle: .module), String(localized: "Enjoy the rest of your evening.", bundle: .module))
        case .context:
            return (String(localized: "All caught up", bundle: .module), String(localized: "Every task here is complete.", bundle: .module))
        }
    }

    // MARK: Toasts

    public static func message(_ m: Message) -> String {
        switch m {
        case .taskCompleted: return String(localized: "Task completed", bundle: .module)
        case .taskCompletedArchived: return String(localized: "Task completed and archived", bundle: .module)
        case .tasksCompleted(let n): return String(localized: "\(n) tasks completed", bundle: .module)
        case .tasksCompletedArchived(let n): return String(localized: "\(n) tasks completed and archived", bundle: .module)
        case .taskDeleted: return String(localized: "Task deleted", bundle: .module)
        case .tasksDeleted(let n): return String(localized: "\(n) tasks deleted", bundle: .module)
        case .taskAdded: return String(localized: "Task added", bundle: .module)
        case .tasksAdded(let n): return String(localized: "\(n) tasks added", bundle: .module)
        case .taskDuplicated: return String(localized: "Task duplicated", bundle: .module)
        case .movedToTonight: return String(localized: "Moved to tonight", bundle: .module)
        case .movedToMorning: return String(localized: "Moved to the morning", bundle: .module)
        case .movedToToday: return String(localized: "Moved to today", bundle: .module)
        case .tasksMovedToTonight(let n): return String(localized: "\(n) tasks moved to tonight", bundle: .module)
        case .tasksMovedToMorning(let n): return String(localized: "\(n) tasks moved to the morning", bundle: .module)
        case .tasksMovedToToday(let n): return String(localized: "\(n) tasks moved to today", bundle: .module)
        case .movedToTomorrow: return String(localized: "Moved to tomorrow", bundle: .module)
        case .movedToNextWeek: return String(localized: "Moved to next week", bundle: .module)
        case .tasksMovedToTomorrow(let n): return String(localized: "\(n) tasks moved to tomorrow", bundle: .module)
        case .tasksMovedToNextWeek(let n): return String(localized: "\(n) tasks moved to next week", bundle: .module)
        case .plannedForToday: return String(localized: "Planned for today", bundle: .module)
        case .plannedForMorning: return String(localized: "Planned for the morning", bundle: .module)
        case .plannedForTonight: return String(localized: "Planned for tonight", bundle: .module)
        case .removedFromToday: return String(localized: "Removed from today", bundle: .module)
        case .tasksPlannedForToday(let n): return String(localized: "\(n) tasks planned for today", bundle: .module)
        case .movedToProject(let name): return String(localized: "Moved to \(name)", bundle: .module)
        case .tagged(let name): return String(localized: "Tagged #\(name)", bundle: .module)
        case .tasksTagged(let n): return String(localized: "\(n) tasks tagged", bundle: .module)
        case .archived(let n): return String(localized: "\(n) completed tasks archived", bundle: .module)
        case .snoozed(let minutes): return String(localized: "Snoozed for \(minutes) minutes", bundle: .module)
        case .undone: return String(localized: "Undone", bundle: .module)
        case .nothingToUndo: return String(localized: "Nothing to undo", bundle: .module)
        case .manualOrderOnly: return String(localized: "Switch to Manual Order to rearrange tasks", bundle: .module)
        case .pickAWeekday: return String(localized: "Pick at least one weekday", bundle: .module)
        case .noLongerRepeats(let title): return String(localized: "“\(title)” no longer repeats", bundle: .module)
        case .repeatSaved(let description): return repeatText(description)
        case .saveFailed(let error): return String(localized: "Could not save changes: \(error)", bundle: .module)
        case .backupImported: return String(localized: "Backup imported", bundle: .module)
        case .backupExported: return String(localized: "Backup exported", bundle: .module)
        case .synced(let n): return String(localized: "Synced (\(n)↑)", bundle: .module)
        case .linkedWith(let name): return String(localized: "Linked with \(name)", bundle: .module)
        case .linking: return String(localized: "Linking…", bundle: .module)
        case .linkFailed: return String(localized: "Could not link. Check the code and try again.", bundle: .module)
        case .linkDeclined: return String(localized: "The other device declined. Check the code and try again.", bundle: .module)
        case .identityChanged(let name): return String(localized: "\(name) changed its identity. Unlink it and link again.", bundle: .module)
        case .nearbyStartFailed(let error): return String(localized: "Nearby sync could not start: \(error)", bundle: .module)
        }
    }

    /// The Edit menu's "Undo <this>" name for a change.
    public static func undoName(_ m: Message) -> String {
        switch m {
        case .taskCompleted, .taskCompletedArchived, .tasksCompleted, .tasksCompletedArchived:
            return String(localized: "Complete", bundle: .module)
        case .taskDeleted, .tasksDeleted: return String(localized: "Delete", bundle: .module)
        case .taskDuplicated: return String(localized: "Duplicate", bundle: .module)
        case .movedToTonight, .movedToMorning, .movedToToday, .tasksMovedToTonight, .tasksMovedToMorning,
             .tasksMovedToToday, .movedToTomorrow, .movedToNextWeek, .tasksMovedToTomorrow, .tasksMovedToNextWeek,
             .plannedForToday, .plannedForMorning, .plannedForTonight, .removedFromToday, .tasksPlannedForToday:
            return String(localized: "Move", bundle: .module)
        case .movedToProject: return String(localized: "Move to Project", bundle: .module)
        case .tagged, .tasksTagged: return String(localized: "Tag", bundle: .module)
        case .archived: return String(localized: "Archive", bundle: .module)
        default: return String(localized: "Change", bundle: .module)
        }
    }

    public static func error(_ e: Error) -> String {
        if let c = e as? CoreError {
            switch c {
            case .NotConfigured: return String(localized: "Nextcloud sync is not configured", bundle: .module)
            case .Actionable(let message), .Transient(let message), .Io(let message),
                 .Invalid(let message), .Nearby(let message):
                return message
            case .Busy: return String(localized: "A sync is already running", bundle: .module)
            }
        }
        return e.localizedDescription
    }
}
