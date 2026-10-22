// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import os
import MomentumCore
import SwiftUI

/// Session selection survives redraws; callers reset it when scope or eligibility changes.
public struct CompletionMessageSelection {
    public private(set) var id: Int?
    private var context: String?
    private static let recent = OSAllocatedUnfairLock(initialState: [String: Int]())
    public init() {}
    public mutating func beginVisit(view: MomentumCore.View, completion: DayCompletion, date: Date = .now,
                                    calendar: Calendar = .current) {
        context = nil
        update(view: view, completion: completion, date: date, calendar: calendar)
    }
    public mutating func update(view: MomentumCore.View, completion: DayCompletion, date: Date = .now,
                                calendar: Calendar = .current) {
        let hour = calendar.component(.hour, from: date)
        let phase = hour < 12 ? 0 : hour < 18 ? 1 : 2
        let day = calendar.startOfDay(for: date).timeIntervalSince1970
        let key = "\(view)-\(completion)-\(day)-\(phase)"
        guard key != context else { return }
        context = key
        let pool = completionMessageIds(view: view, completion: completion, hour: UInt32(hour)).map(Int.init)
        id = Self.recent.withLock { recent in
            let scope = "\(view)"
            let picked = pool.filter { $0 != recent[scope] }.randomElement() ?? pool.first
            if let picked { recent[scope] = picked }
            return picked
        }
    }
    public var text: String? { id.map { Self.text(id: $0) } }
    public static func text(id: Int, locale: Locale = .current) -> String {
        let language = locale.language.languageCode?.identifier ?? "en"
        let bundle = Bundle.module.path(forResource: language, ofType: "lproj").flatMap(Bundle.init(path:)) ?? .module
        return bundle.localizedString(forKey: "completion.message.\(id)", value: nil, table: nil)
    }
}

public struct CompletionCelebration: SwiftUI.View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var celebrated = false
    @State private var burstStart: Date?
    @State private var cleanup: Task<Void, Never>?
    private var feedback: CelebrationFeedback { .presentation(dynamic: true, completion: .dayComplete, reduceMotion: reduceMotion) }
    public init() {}
    public var body: some SwiftUI.View {
        VStack {
            if celebrated && feedback == .staticSymbol {
                Image(systemName: "sparkles").foregroundStyle(.tint).accessibilityHidden(true)
            }
            Button(String(localized: "Celebrate", bundle: .module)) {
                cleanup?.cancel()
                celebrated = true
                guard feedback == .burst else { burstStart = nil; return }
                burstStart = .now
                cleanup = Task { @MainActor in
                    try? await Task.sleep(for: .milliseconds(900))
                    guard !Task.isCancelled else { return }
                    burstStart = nil
                }
            }.buttonStyle(.borderless)
        }
        .overlay {
            if let start = burstStart, feedback == .burst {
                TimelineView(.animation(minimumInterval: 1.0 / 30)) { timeline in
                    Canvas { context, size in
                        let progress = min(1, max(0, timeline.date.timeIntervalSince(start) / 0.9))
                        for index in 0..<24 {
                            let angle = Double(index) * .pi * 2 / 24
                            let radius = progress * (45 + Double(index % 5) * 9)
                            let x = size.width / 2 + cos(angle) * radius
                            let y = size.height / 2 + sin(angle) * radius + progress * progress * 35
                            let rect = CGRect(x: x, y: y, width: 4, height: 6)
                            context.opacity = 1 - progress
                            context.fill(Path(roundedRect: rect, cornerRadius: 1), with: .color(index.isMultiple(of: 2) ? .accentColor : .secondary))
                        }
                    }
                }.frame(width: 220, height: 160).allowsHitTesting(false).accessibilityHidden(true)
            }
        }
        .onDisappear { cleanup?.cancel(); burstStart = nil }
    }
}
