// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore
import MomentumKit
import MomentumMobile
import SwiftUI

struct RepeatEditor: SwiftUI.View {
    let worker: EngineWorker
    let taskID: String
    let onChange: @MainActor () -> Void
    @Environment(\.dismiss) private var dismiss
    @State private var draft: RepeatDraft?
    @State private var description: RepeatDescription?
    @State private var loaded = false
    @State private var busy = false
    @State private var error: String?
    @AccessibilityFocusState private var errorFocused: Bool

    private var weekdayOrder: [UInt32] {
        let first = Calendar.current.firstWeekday - 1
        return (0..<7).map { UInt32((first + $0) % 7) }
    }

    var body: some SwiftUI.View {
        NavigationStack {
            Form {
                if let error {
                    Section {
                        AccessibleErrorMessage(error)
                            .accessibilityFocused($errorFocused)
                            .accessibilityIdentifier("repeat-editor-error")
                    }
                }
                if let draft {
                    if let description {
                        Section { Text(Strings.repeatText(description)).foregroundStyle(AccentTheme.secondaryText) }
                    }
                    schedule(draft)
                        .disabled(busy)
                    if draft.existing {
                        Section {
                            Button(String(localized: "Stop Repeating"), systemImage: "stop.circle", role: .destructive) { stop() }
                                .disabled(busy)
                        }
                    }
                } else if loaded {
                    Text(String(localized: "This task cannot repeat."))
                } else {
                    ProgressView(String(localized: "Loading schedule…"))
                }
            }
            .momentumNavigationCanvas()
            .accessibilityIdentifier("repeat-editor-form")
            .navigationTitle(String(localized: "Repeat"))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(String(localized: "Cancel"), systemImage: "xmark") { dismiss() }.disabled(busy)
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button(draft?.existing == true ? String(localized: "Save") : String(localized: "Repeat"), systemImage: "checkmark") { save() }
                        .disabled(draft == nil || busy)
                        .keyboardShortcut(.return, modifiers: .command)
                }
            }
            .task {
                guard !loaded else { return }
                draft = await worker.repeatDraft(taskID)
                loaded = true
            }
            .task(id: draft) {
                guard let draft else { return }
                let value = await worker.repeatDescription(draft)
                guard !Task.isCancelled else { return }
                description = value
            }
            .onChange(of: error) { _, value in if value != nil { errorFocused = true } }
            .interactiveDismissDisabled(busy)
        }
    }

    @ViewBuilder private func schedule(_ value: RepeatDraft) -> some SwiftUI.View {
        Section {
            Picker(String(localized: "Repeats"), selection: binding(\.cycle, fallback: .weekly)) {
                Text(String(localized: "Daily")).tag(RepeatCycle.daily)
                Text(String(localized: "Weekly")).tag(RepeatCycle.weekly)
                Text(String(localized: "Monthly")).tag(RepeatCycle.monthly)
                Text(String(localized: "Yearly")).tag(RepeatCycle.yearly)
            }
            Stepper(value: binding(\.every, fallback: 1), in: 1...99) {
                LabeledContent(String(localized: "Every"), value: unitLabel(value))
            }
            if value.cycle == .weekly {
                ForEach(weekdayOrder, id: \.self) { weekday in
                    Toggle(Strings.weekdayName(weekday), isOn: Binding(
                        get: { draft?.weekdays[Int(weekday)] ?? false },
                        set: { draft?.weekdays[Int(weekday)] = $0 }))
                }
            }
            if value.cycle == .monthly {
                Picker(String(localized: "Monthly on"), selection: monthlyKind) {
                    Text(String(localized: "The same date")).tag(0)
                    Text(String(localized: "The last day of the month")).tag(1)
                    Text(String(localized: "A weekday of the month")).tag(2)
                }
                if case .nthWeekday = value.monthly {
                    Picker(String(localized: "Which"), selection: nthWeek) {
                        Text(String(localized: "First")).tag(Int32(1))
                        Text(String(localized: "Second")).tag(Int32(2))
                        Text(String(localized: "Third")).tag(Int32(3))
                        Text(String(localized: "Fourth")).tag(Int32(4))
                        Text(String(localized: "Last")).tag(Int32(-1))
                    }
                    Picker(String(localized: "Weekday"), selection: nthWeekday) {
                        ForEach(weekdayOrder, id: \.self) { Text(Strings.weekdayName($0)).tag($0) }
                    }
                }
            }
            DatePicker(String(localized: "Starts"), selection: Binding(
                get: { draft?.startDate.flatMap(Strings.date(fromDay:)) ?? Date() },
                set: { draft?.startDate = Strings.day(fromDate: $0) }), displayedComponents: .date)
            Toggle(String(localized: "Paused"), isOn: binding(\.paused, fallback: false))
        } header: {
            Label(String(localized: "Schedule"), systemImage: "repeat")
                .foregroundStyle(AccentTheme.secondaryText)
        } footer: {
            Text(String(localized: "A paused schedule keeps its settings but creates no tasks."))
                .foregroundStyle(AccentTheme.secondaryText)
        }
    }

    private func binding<T>(_ key: WritableKeyPath<RepeatDraft, T>, fallback: T) -> Binding<T> {
        Binding(get: { draft?[keyPath: key] ?? fallback }, set: { draft?[keyPath: key] = $0 })
    }

    private var monthlyKind: Binding<Int> {
        Binding(get: {
            switch draft?.monthly {
            case .lastDay: return 1
            case .nthWeekday: return 2
            default: return 0
            }
        }, set: {
            switch $0 {
            case 1: draft?.monthly = .lastDay
            case 2: draft?.monthly = .nthWeekday(week: 1, weekday: 1)
            default: draft?.monthly = .sameDay
            }
        })
    }

    private var nthWeek: Binding<Int32> {
        Binding(get: {
            if case .nthWeekday(let week, _) = draft?.monthly { return week }
            return 1
        }, set: { week in
            if case .nthWeekday(_, let weekday) = draft?.monthly {
                draft?.monthly = .nthWeekday(week: week, weekday: weekday)
            }
        })
    }

    private var nthWeekday: Binding<UInt32> {
        Binding(get: {
            if case .nthWeekday(_, let weekday) = draft?.monthly { return weekday }
            return 1
        }, set: { weekday in
            if case .nthWeekday(let week, _) = draft?.monthly {
                draft?.monthly = .nthWeekday(week: week, weekday: weekday)
            }
        })
    }

    private func unitLabel(_ draft: RepeatDraft) -> String {
        switch draft.cycle {
        case .daily: String(localized: "\(draft.every) days")
        case .weekly: String(localized: "\(draft.every) weeks")
        case .monthly: String(localized: "\(draft.every) months")
        case .yearly: String(localized: "\(draft.every) years")
        }
    }

    private func save() {
        guard let draft, !busy else { return }
        busy = true
        error = nil
        Task { await finish(worker.saveRepeat(taskID, draft: draft)) }
    }

    private func stop() {
        guard !busy else { return }
        busy = true
        error = nil
        Task { await finish(worker.stopRepeat(taskID)) }
    }

    private func finish(_ outcome: Outcome) {
        busy = false
        if outcome.changed {
            onChange()
            dismiss()
        } else {
            error = outcome.message.map(Strings.message) ?? String(localized: "The schedule could not be changed.")
        }
    }
}
