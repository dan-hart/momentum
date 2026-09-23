// SPDX-License-Identifier: GPL-3.0-or-later
import SwiftUI
import UIKit

struct StartupSkeletonMotionPolicy: Equatable, Sendable {
    static let maximumFramesPerSecond = 30.0
    static let standardFadeDuration = 0.2

    let isComplete: Bool
    let reduceMotion: Bool
    let lowPowerMode: Bool
    let sceneIsActive: Bool

    var shimmers: Bool {
        !isComplete && !reduceMotion && !lowPowerMode && sceneIsActive
    }

    var minimumInterval: TimeInterval {
        1.0 / Self.maximumFramesPerSecond
    }

    var fadeDuration: TimeInterval {
        reduceMotion ? 0 : Self.standardFadeDuration
    }
}

/// A deliberately content-free preview of Today. RootView owns the single
/// accessible loading status; these shapes never pretend to be real tasks.
struct TodaySkeletonView: View {
    let motion: StartupSkeletonMotionPolicy

    @ScaledMetric(relativeTo: .body) private var rowHeight = 72.0
    @ScaledMetric(relativeTo: .headline) private var primaryHeight = 18.0
    @ScaledMetric(relativeTo: .subheadline) private var secondaryHeight = 12.0
    @ScaledMetric(relativeTo: .body) private var completionSize = 28.0
    @ScaledMetric(relativeTo: .caption) private var headingHeight = 12.0

    var body: some View {
        TimelineView(.animation(minimumInterval: motion.minimumInterval, paused: !motion.shimmers)) { context in
            let phase = shimmerPhase(at: context.date)
            ScrollView {
                VStack(alignment: .leading, spacing: 14) {
                    placeholder(
                        cornerRadius: headingHeight / 2,
                        phase: phase
                    )
                    .frame(width: 92, height: headingHeight)
                    .accessibilityIdentifier("skeleton-section-heading")
                    .padding(.leading, 18)

                    VStack(spacing: 0) {
                        ForEach(0..<5, id: \.self) { index in
                            skeletonRow(index: index, phase: phase)
                            if index < 4 {
                                Divider().padding(.leading, completionSize + 52)
                            }
                        }
                    }
                    .background(Color(uiColor: .secondarySystemGroupedBackground))
                    .clipShape(RoundedRectangle(cornerRadius: 26, style: .continuous))
                }
                .padding(.horizontal, 20)
                .padding(.top, 12)
                .padding(.bottom, 24)
            }
            .scrollDisabled(true)
            .background(Color(uiColor: .systemGroupedBackground))
        }
        .background(StartupSkeletonMountMarker())
        .accessibilityIdentifier("today-loading-skeleton")
        .accessibilityHidden(true)
    }

    private func skeletonRow(index: Int, phase: Double) -> some View {
        HStack(spacing: 18) {
            Circle()
                .strokeBorder(fillStyle(phase: phase), lineWidth: 4)
                .frame(width: completionSize, height: completionSize)
                .accessibilityIdentifier("skeleton-row-\(index)-completion")

            VStack(alignment: .leading, spacing: 10) {
                placeholder(cornerRadius: primaryHeight / 2, phase: phase)
                    .frame(maxWidth: index.isMultiple(of: 2) ? 230 : 180)
                    .frame(height: primaryHeight)
                    .accessibilityIdentifier("skeleton-row-\(index)-primary")
                placeholder(cornerRadius: secondaryHeight / 2, phase: phase)
                    .frame(maxWidth: index.isMultiple(of: 3) ? 150 : 112)
                    .frame(height: secondaryHeight)
                    .accessibilityIdentifier("skeleton-row-\(index)-secondary")
            }
            Spacer(minLength: 0)
        }
        .padding(.horizontal, 20)
        .frame(minHeight: max(64, rowHeight))
        .accessibilityIdentifier("skeleton-row-\(index)")
    }

    private func placeholder(cornerRadius: CGFloat, phase: Double) -> some View {
        RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
            .fill(fillStyle(phase: phase))
    }

    private func fillStyle(phase: Double) -> AnyShapeStyle {
        let base = Color(uiColor: .tertiarySystemFill)
        guard motion.shimmers else { return AnyShapeStyle(base) }
        let center = phase * 2.4 - 0.7
        let leading = min(1, max(0, center - 0.35))
        let highlight = min(1, max(0, center))
        let trailing = min(1, max(0, center + 0.35))
        return AnyShapeStyle(
            LinearGradient(
                stops: [
                    .init(color: base, location: leading),
                    .init(color: Color(uiColor: .secondarySystemFill), location: highlight),
                    .init(color: base, location: trailing),
                ],
                startPoint: .leading,
                endPoint: .trailing
            )
        )
    }

    private func shimmerPhase(at date: Date) -> Double {
        guard motion.shimmers else { return 0 }
        let duration = 1.35
        return date.timeIntervalSinceReferenceDate
            .truncatingRemainder(dividingBy: duration) / duration
    }
}

private struct StartupSkeletonMountMarker: UIViewRepresentable {
    func makeUIView(context: Context) -> UIView {
        let view = UIView()
        view.backgroundColor = .clear
        view.isUserInteractionEnabled = false
        view.isAccessibilityElement = false
        view.accessibilityIdentifier = "today-loading-skeleton"
        return view
    }

    func updateUIView(_ view: UIView, context: Context) {
        view.accessibilityIdentifier = "today-loading-skeleton"
    }
}
