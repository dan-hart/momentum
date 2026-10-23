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
    private static let duration = 1.2
    private static let colors: [Color] = [.accentColor, .orange, .pink, .purple, .cyan, .yellow]
    private var feedback: CelebrationFeedback { .presentation(dynamic: true, completion: .dayComplete, reduceMotion: reduceMotion) }
    public init() {}
    public var body: some SwiftUI.View {
        Button(action: celebrate) {
            Label(String(localized: "Celebrate", bundle: .module), systemImage: "sparkles")
                .fontWeight(.semibold)
                .padding(.horizontal, 8)
                .frame(minHeight: 24)
        }
        .buttonStyle(CelebrationButtonStyle())
        .accessibilityAction(named: Text(String(localized: "Celebrate", bundle: .module)), celebrate)
        .overlay {
            if celebrated && feedback == .staticSymbol {
                // Overlay decorations never change the button's layout or hit region.
                ZStack {
                    Image(systemName: "sparkle").font(.title).foregroundStyle(.orange).offset(x: -94, y: -28)
                    Image(systemName: "star.fill").font(.title2).foregroundStyle(.pink).offset(x: 90, y: -34)
                    Image(systemName: "sparkles").font(.title).foregroundStyle(.tint).offset(x: 78, y: 32)
                }
                .allowsHitTesting(false)
                .accessibilityHidden(true)
            } else if let start = burstStart, feedback == .burst {
                TimelineView(.animation(minimumInterval: 1.0 / 30)) { timeline in
                    Canvas { context, size in
                        let progress = min(1, max(0, timeline.date.timeIntervalSince(start) / Self.duration))
                        let spread = 1 - pow(1 - progress, 3)
                        let center = CGPoint(x: size.width / 2, y: size.height / 2)
                        // A quick expanding halo anchors the larger, colorful shower.
                        let ringRadius = 12 + spread * 96
                        var ring = context
                        ring.opacity = max(0, 1 - progress * 2)
                        ring.stroke(Path(ellipseIn: CGRect(x: center.x - ringRadius, y: center.y - ringRadius * 0.65,
                                                          width: ringRadius * 2, height: ringRadius * 1.3)),
                                    with: .color(.accentColor), lineWidth: 3 * (1 - progress) + 1)
                        for index in 0..<48 {
                            let angle = Double(index) * .pi * 2 / 48
                            let radius = spread * (80 + Double(index % 7) * 7)
                            let x = center.x + cos(angle) * radius
                            let y = center.y + sin(angle) * radius * 0.65 + progress * progress * 18
                            var particle = context
                            particle.opacity = min(1, (1 - progress) * 2.5)
                            particle.translateBy(x: x, y: y)
                            particle.rotate(by: .degrees(Double(index * 37) + progress * 240))
                            let shape = index.isMultiple(of: 4)
                                ? Self.star(radius: 8)
                                : Path(roundedRect: CGRect(x: -3, y: -5, width: 6, height: 10), cornerRadius: 2)
                            particle.fill(shape, with: .color(Self.colors[index % Self.colors.count]))
                        }
                    }
                }
                .frame(width: 300, height: 220)
                .offset(y: -60)
                .allowsHitTesting(false)
                .accessibilityHidden(true)
            }
        }
        .onChange(of: reduceMotion) { _, _ in cleanup?.cancel(); cleanup = nil; burstStart = nil }
        .onDisappear { cleanup?.cancel(); cleanup = nil; burstStart = nil; celebrated = false }
    }

    private func celebrate() {
        cleanup?.cancel()
        celebrated = true
        guard feedback == .burst else { burstStart = nil; return }
        burstStart = .now
        cleanup = Task { @MainActor in
            try? await Task.sleep(for: .seconds(Self.duration))
            guard !Task.isCancelled else { return }
            burstStart = nil
            cleanup = nil
        }
    }

    private static func star(radius: Double) -> Path {
        Path { path in
            for point in 0..<10 {
                let angle = Double(point) * .pi / 5 - .pi / 2
                let length = point.isMultiple(of: 2) ? radius : radius * 0.45
                let position = CGPoint(x: cos(angle) * length, y: sin(angle) * length)
                if point == 0 { path.move(to: position) } else { path.addLine(to: position) }
            }
            path.closeSubpath()
        }
    }
}
