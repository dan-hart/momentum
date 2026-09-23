// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumMobile
import SwiftUI

struct InteractionMotionPresentation: Equatable {
    let duration: TimeInterval
    let pressedScale: CGFloat
    let usesScaleTransition: Bool
    let usesSymbolTransition: Bool

    var animation: Animation { .easeOut(duration: duration) }

    var transition: AnyTransition {
        usesScaleTransition ? .scale(scale: 0.96).combined(with: .opacity) : .opacity
    }
}

enum InteractionMotionPolicy {
    static func presentation(reduceMotion: Bool) -> InteractionMotionPresentation {
        reduceMotion
            ? InteractionMotionPresentation(duration: 0.1, pressedScale: 1,
                                            usesScaleTransition: false, usesSymbolTransition: false)
            : InteractionMotionPresentation(duration: 0.18, pressedScale: 0.96,
                                            usesScaleTransition: true, usesSymbolTransition: true)
    }
}

struct FloatingAddTaskButton: View {
    let action: () -> Void
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @AppStorage(AccentChoice.preferenceKey) private var selected = AccentChoice.momentum.id
    @State private var pressed = false
    @State private var releaseTask: Task<Void, Never>?

    var body: some View {
        let motion = InteractionMotionPolicy.presentation(reduceMotion: reduceMotion)
        Button {
            releaseTask?.cancel()
            pressed = true
            action()
            releaseTask = Task { @MainActor in
                try? await Task.sleep(for: .milliseconds(90))
                guard !Task.isCancelled else { return }
                pressed = false
            }
        } label: {
            ViewThatFits(in: .horizontal) {
                Label("Add task", systemImage: "plus")
                    .fixedSize(horizontal: true, vertical: false)
                Image(systemName: "plus")
            }
            .font(.headline)
            .foregroundStyle(AccentTheme.onAccent(AccentChoice.resolve(selected)))
            .padding(.horizontal, 8)
            .frame(minHeight: 32)
        }
        .buttonStyle(.borderedProminent)
        .buttonBorderShape(.capsule)
        .controlSize(.large)
        .shadow(color: .black.opacity(0.16), radius: 4, x: 0, y: 2)
        .scaleEffect(pressed ? motion.pressedScale : 1)
        .opacity(pressed && reduceMotion ? 0.78 : 1)
        .animation(motion.animation, value: pressed)
        .accessibilityLabel("Add task")
        .accessibilityIdentifier("floating-add-task")
        .onDisappear {
            releaseTask?.cancel()
            pressed = false
        }
    }
}
