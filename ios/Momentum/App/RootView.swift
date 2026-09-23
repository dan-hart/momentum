// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore
import MomentumKit
import MomentumMobile
import SwiftUI
import UIKit

enum MobileTab: Int, CaseIterable {
    case today
    case upcoming
    case search
    case settings

    var titleKey: String {
        switch self {
        case .today: "Today"
        case .upcoming: "Upcoming"
        case .search: "Search"
        case .settings: "Settings"
        }
    }

    var title: LocalizedStringKey { LocalizedStringKey(titleKey) }

    var symbol: String {
        switch self {
        case .today: "star.fill"
        case .upcoming: "calendar"
        case .search: "magnifyingglass"
        case .settings: "gearshape.fill"
        }
    }

    var accessibilityIdentifier: String { "tab-\(titleKey.lowercased())" }
}

enum InteractionFeedbackPolicy {
    static func shouldPresent<Value: Equatable>(enabled: Bool, oldValue: Value, newValue: Value) -> Bool {
        enabled && oldValue != newValue
    }
}

enum RootStartupPhase: Equatable {
    case loadingToday
    case ready
    case superseded
}

struct RootView: SwiftUI.View {
    @Environment(MobileAppModel.self) private var model
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @AppStorage(MobileAppearance.hapticsKey) private var haptics = true
    @AppStorage(PrefKey.modifierKey) private var modifierRaw = ModifierKey.command.rawValue
    @State private var taskWorkspaceState = TaskWorkspaceState()
    @State private var startupPhase = RootStartupPhase.loadingToday
    var snapshotLoader: TaskSnapshotLoader

    init(snapshotLoader: @escaping TaskSnapshotLoader = {
        worker, view, query, archiveLimit in
        worker.snapshot(view: view, query: query, archiveLimit: archiveLimit)
    }) {
        self.snapshotLoader = snapshotLoader
    }

    private var isPresentingStartupSkeleton: Bool {
        startupPhase == .loadingToday
            && model.selectedTab == MobileTab.today.rawValue
            && model.notificationTask == nil
    }

    var body: some SwiftUI.View {
        Group {
            if let failure = model.startupFailure {
                StoreUnavailableView(details: failure, isRetrying: model.isOpeningStore) {
                    Task { await model.foreground() }
                }
            } else {
                startupTabs
            }
        }
        .mobileFont()
        .disabled(model.isRestoringBackup)
        .accessibilityHidden(model.isRestoringBackup)
        .overlay {
            if model.isRestoringBackup {
                ZStack {
                    Color(.systemBackground).opacity(0.95).ignoresSafeArea()
                    ProgressView("Restoring Backup…")
                        .padding(24)
                        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 16))
                }
                .accessibilityIdentifier("backup-restoring")
            }
        }
        .overlay(alignment: .bottom) {
            if let toast = model.feedbackToast {
                FeedbackToastView(toast: toast)
                    .padding(.horizontal, 16)
                    .padding(.bottom, 82)
                    .transition(reduceMotion ? .opacity : .move(edge: .bottom).combined(with: .opacity))
            }
        }
        .animation(reduceMotion ? nil : .smooth(duration: 0.24), value: model.feedbackToast?.id)
    }

    private var startupTabs: some SwiftUI.View {
        ZStack {
            tabs
                .redacted(reason: isPresentingStartupSkeleton ? .placeholder : [])
                .disabled(isPresentingStartupSkeleton)

            if isPresentingStartupSkeleton {
                Color.clear
                    .contentShape(Rectangle())
                    .accessibilityElement(children: .ignore)
                    .accessibilityLabel("Loading Momentum")
                    .accessibilityAddTraits(.updatesFrequently)
                    .accessibilityIdentifier("startup-loading-status")
            }
        }
        .animation(
            reduceMotion ? nil : .easeOut(duration: StartupSkeletonMotionPolicy.standardFadeDuration),
            value: startupPhase
        )
        .onAppear(perform: reconcileStartupRoute)
        .onChange(of: model.selectedTab) { _, _ in reconcileStartupRoute() }
        .onChange(of: model.notificationTask?.id) { _, _ in reconcileStartupRoute() }
    }

    private var tabs: some SwiftUI.View {
        @Bindable var model = model
        return TabView(selection: $model.selectedTab) {
            Tab(MobileTab.today.title, systemImage: MobileTab.today.symbol, value: MobileTab.today.rawValue) {
                TaskWorkspace(
                    state: $taskWorkspaceState,
                    loadingPresentation: isPresentingStartupSkeleton ? .todaySkeleton : .progress,
                    snapshotLoader: snapshotLoader,
                    onInitialTodayReady: initialTodaySnapshotReady
                )
                .accessibilityHidden(isPresentingStartupSkeleton)
            }
            .accessibilityIdentifier(MobileTab.today.accessibilityIdentifier)
            Tab(MobileTab.upcoming.title, systemImage: MobileTab.upcoming.symbol, value: MobileTab.upcoming.rawValue) {
                NavigationStack { TaskScreen(view: .upcoming) }
                    .accessibilityHidden(isPresentingStartupSkeleton)
            }
            .accessibilityIdentifier(MobileTab.upcoming.accessibilityIdentifier)
            Tab(MobileTab.search.title, systemImage: MobileTab.search.symbol, value: MobileTab.search.rawValue) {
                NavigationStack { TaskScreen(view: .search) }
                    .accessibilityHidden(isPresentingStartupSkeleton)
            }
            .accessibilityIdentifier(MobileTab.search.accessibilityIdentifier)
            Tab(MobileTab.settings.title, systemImage: MobileTab.settings.symbol, value: MobileTab.settings.rawValue) {
                NavigationStack { SettingsScreen() }
                    .accessibilityHidden(isPresentingStartupSkeleton)
            }
            .accessibilityIdentifier(MobileTab.settings.accessibilityIdentifier)
        }
        .background(TabBarAccessibilityBridge(hidden: isPresentingStartupSkeleton))
        // Keep the grouped canvas continuous behind native floating controls.
        .scrollEdgeEffectHidden(true, for: [.top, .bottom])
        .sensoryFeedback(.success, trigger: model.completionFeedback) { oldValue, newValue in
            InteractionFeedbackPolicy.shouldPresent(enabled: haptics, oldValue: oldValue, newValue: newValue)
        }
        .sensoryFeedback(.selection, trigger: model.selectionFeedback) { oldValue, newValue in
            InteractionFeedbackPolicy.shouldPresent(enabled: haptics, oldValue: oldValue, newValue: newValue)
        }
        .sensoryFeedback(.success, trigger: model.interactionSuccessFeedback) { oldValue, newValue in
            InteractionFeedbackPolicy.shouldPresent(enabled: haptics, oldValue: oldValue, newValue: newValue)
        }
        // Adding a task always opens the full editor; there is no quick-entry sheet.
        .sheet(isPresented: $model.showingAdd, onDismiss: model.presentPendingNotification) {
            if let worker = model.worker {
                TaskEditor(worker: worker, view: model.creationView, onChange: model.refreshAfterTaskCreation)
            }
        }
        .sheet(isPresented: $model.showingNewProject, onDismiss: model.presentPendingNotification) {
            ContextEditor(kind: .project)
        }
        .sheet(item: $model.notificationTask, onDismiss: model.presentPendingNotification) { task in
            if let worker = model.worker {
                TaskEditor(worker: worker, view: .today, taskID: task.id, onChange: model.refreshAfterEdit)
            }
        }
        .modifier(MobileKeyboardPresentation(model: model, modifierRaw: modifierRaw))
    }

    private func initialTodaySnapshotReady() {
        guard startupPhase == .loadingToday else { return }
        startupPhase = .ready
    }

    private func reconcileStartupRoute() {
        guard startupPhase == .loadingToday else { return }
        if model.selectedTab != MobileTab.today.rawValue || model.notificationTask != nil {
            startupPhase = .superseded
        }
    }
}

private struct FeedbackToastView: SwiftUI.View {
    @Environment(MobileAppModel.self) private var model
    @AccessibilityFocusState private var focused: Bool
    let toast: FeedbackToast

    var body: some SwiftUI.View {
        HStack(spacing: 12) {
            Image(systemName: toast.kind == .error ? "exclamationmark.triangle.fill" : "checkmark.circle.fill")
                .foregroundStyle(toast.kind == .error ? Color.red : Color.accentColor)
                .accessibilityHidden(true)
            Text(toast.message)
                .font(.callout.weight(.medium))
                .fixedSize(horizontal: false, vertical: true)
            Spacer(minLength: 4)
            if toast.undoBatchID != nil {
                Button("Undo") { Task { await model.undoFeedback() } }
                    .fontWeight(.semibold)
                    .accessibilityIdentifier("task-feedback-undo")
            }
            if toast.isPersistent {
                Button { model.dismissFeedback() } label: {
                    Image(systemName: "xmark").frame(minWidth: 44, minHeight: 44)
                }
                .accessibilityLabel("Dismiss message")
            }
        }
        .padding(.leading, 16)
        .padding(.trailing, toast.isPersistent ? 4 : 16)
        .padding(.vertical, toast.isPersistent ? 8 : 14)
        .frame(maxWidth: 560)
        .glassEffect(.regular.interactive(), in: .rect(cornerRadius: 22))
        .shadow(color: .black.opacity(0.16), radius: 12, y: 5)
        .accessibilityElement(children: .contain)
        .accessibilityFocused($focused)
        .accessibilityIdentifier("task-feedback")
        .onAppear { if toast.isPersistent { focused = true } }
    }
}

private struct TabBarAccessibilityBridge: UIViewRepresentable {
    let hidden: Bool

    @MainActor
    final class Coordinator {
        var requestedHidden = false
    }

    func makeCoordinator() -> Coordinator {
        Coordinator()
    }

    func makeUIView(context: Context) -> UIView {
        let view = UIView()
        view.backgroundColor = .clear
        view.isUserInteractionEnabled = false
        return view
    }

    func updateUIView(_ view: UIView, context: Context) {
        context.coordinator.requestedHidden = hidden
        apply(hidden: hidden, from: view)

        let requestedHidden = hidden
        Task { @MainActor in
            await Task.yield()
            guard context.coordinator.requestedHidden == requestedHidden else { return }
            apply(hidden: requestedHidden, from: view)
        }
    }

    private func apply(hidden: Bool, from view: UIView) {
        guard let root = view.window?.rootViewController,
              let tabBarController = findTabBarController(in: root) else { return }
        tabBarController.tabBar.accessibilityElementsHidden = hidden
        tabBarController.selectedViewController?.view.accessibilityElementsHidden = hidden
    }

    private func findTabBarController(in controller: UIViewController) -> UITabBarController? {
        if let tabBarController = controller as? UITabBarController { return tabBarController }
        for child in controller.children {
            if let found = findTabBarController(in: child) { return found }
        }
        if let presented = controller.presentedViewController {
            return findTabBarController(in: presented)
        }
        return nil
    }
}

private struct MobileKeyboardPresentation: ViewModifier {
    @Bindable var model: MobileAppModel
    let modifierRaw: String

    @ViewBuilder func body(content: Content) -> some SwiftUI.View {
        if model.platformCapabilities.showsKeyboardShortcuts {
            content
                .sheet(isPresented: $model.showingKeyboardHelp, onDismiss: model.presentPendingNotification) {
                    NavigationStack {
                        KeyboardSettings().toolbar {
                            ToolbarItem(placement: .confirmationAction) {
                                Button("Done") { model.showingKeyboardHelp = false }
                                    .keyboardShortcut(.cancelAction)
                            }
                        }
                    }
                }
                .onKeyPress(KeyEquivalent("f"), phases: .down) { press in
                    MobileKeyPressFallback.global(
                        key: press.key, modifiers: press.modifiers,
                        configuredModifier: ModifierKey(rawValue: modifierRaw) ?? .command,
                        model: model
                    )
                }
        } else {
            content
        }
    }
}

/// A failed recovery never exposes a fresh writable store or an empty task list.
struct StoreUnavailableView: SwiftUI.View {
    let details: String
    let isRetrying: Bool
    let retry: () -> Void

    var body: some SwiftUI.View {
        ScrollView {
            ContentUnavailableView {
                Label("Data Unavailable", systemImage: "externaldrive.badge.exclamationmark")
            } description: {
                Text("Momentum couldn’t safely open its data. Existing files have been preserved.")
            }
            DisclosureGroup("Details") {
                Text(details).font(.footnote).textSelection(.enabled)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .padding(.horizontal)
        }
        .safeAreaInset(edge: .bottom) {
            VStack {
                Button(action: retry) {
                    Label("Try Again", systemImage: "arrow.clockwise")
                        .frame(minHeight: 44)
                }
                .buttonStyle(.bordered)
                .foregroundStyle(.primary)
                .disabled(isRetrying)
                if isRetrying { ProgressView().accessibilityLabel("Opening data") }
            }
            .frame(maxWidth: .infinity)
            .padding()
            .background(.background)
        }
    }
}
