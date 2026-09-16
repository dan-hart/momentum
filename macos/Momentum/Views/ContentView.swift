// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
import MomentumCore
import MomentumKit
import SwiftUI
import UniformTypeIdentifiers

struct ContentView: SwiftUI.View {
    @Environment(AppState.self) private var state
    @Environment(\.openWindow) private var openWindow
    @Environment(\.appTypography) private var typography
    @AppStorage(PrefKey.sidebarVisible) private var sidebarVisible = true

    var body: some SwiftUI.View {
        @Bindable var state = state
        NavigationSplitView(columnVisibility: columnVisibility) {
            SidebarView()
                .navigationSplitViewColumnWidth(
                    min: min(320, 180 * sidebarScale),
                    ideal: min(360, 220 * sidebarScale),
                    max: min(420, 320 * sidebarScale))
        } detail: {
            DetailView()
        }
        .navigationTitle(Strings.viewTitle(state.listing.title))
        .navigationSubtitle(subtitle)
        .searchable(text: $state.searchText, isPresented: $state.isSearchPresented,
                    placement: .toolbar, prompt: String(localized: "Search Everything"))
        .onChange(of: state.isSearchPresented) { _, presented in state.searchPresentationChanged(presented) }
        .task(id: state.searchText) {
            // Coalesce keystrokes: rebuild the result list at most every 120 ms.
            try? await Task.sleep(for: .milliseconds(120))
            if !Task.isCancelled { state.searchTextChanged() }
        }
        .toolbar { ToolbarItems() }
        .sheet(item: $state.sheet, onDismiss: state.sheetDismissed) { sheet in
            sheetView(sheet)
        }
        .confirmationDialog(confirmationTitle, isPresented: confirmationPresented, titleVisibility: .visible,
                            presenting: state.confirmation) { c in
            Button(String(localized: "Delete"), role: .destructive) { state.deleteContext(c) }
            Button(String(localized: "Cancel"), role: .cancel) {}
        } message: { c in
            Text(confirmationMessage(c))
        }
        .onAppear {
            WindowOpener.shared.open = { id in openWindow(id: id) }
        }
        .onOpenURL { url in state.handle(url: url) }
        .focusedSceneValue(\.appState, state)
    }

    private var columnVisibility: Binding<NavigationSplitViewVisibility> {
        Binding(
            get: { sidebarVisible ? .all : .detailOnly },
            set: { sidebarVisible = $0 != .detailOnly })
    }

    private var sidebarScale: Double { max(1, typography.interfaceSize / 13) }

    private var subtitle: String {
        state.selection.count > 1 ? String(localized: "\(state.selection.count) selected") : ""
    }

    private var confirmationPresented: Binding<Bool> {
        Binding(get: { state.confirmation != nil }, set: { if !$0 { state.confirmation = nil } })
    }
    private var confirmationTitle: String {
        switch state.confirmation {
        case .deleteProject(_, let name, _), .deleteTag(_, let name): return String(localized: "Delete “\(name)”?")
        case nil: return ""
        }
    }
    private func confirmationMessage(_ c: Confirmation) -> String {
        switch c {
        case .deleteProject(_, _, let n):
            return String(localized: "\(n) tasks in this project will be deleted. This cannot be undone.")
        case .deleteTag:
            return String(localized: "Tasks keep their other tags.")
        }
    }

    @ViewBuilder
    private func sheetView(_ sheet: Sheet) -> some SwiftUI.View {
        switch sheet {
        case .newTask: TaskFormSheet(taskId: nil)
        case .editTask(let id): TaskFormSheet(taskId: id)
        case .repeatSchedule(let id): RepeatSheet(taskId: id)
        case .moveToProject(let ids): MoveToProjectSheet(ids: ids)
        case .addTag(let ids): AddTagSheet(ids: ids)
        case .editContext(let v): EditContextSheet(view: v)
        case .newProject: NewProjectSheet()
        case .devices: DevicesSheet()
        case .shortcuts: ShortcutsSheet()
        }
    }
}

/// The content column: banner, quick-add, the list, the sync caption, toasts on top.
struct DetailView: SwiftUI.View {
    @Environment(AppState.self) private var state

    var body: some SwiftUI.View {
        VStack(spacing: 0) {
            if let banner = state.banner {
                BannerView(text: banner)
            }
            if !state.isSearching {
                ListHeading()
                QuickAddField()
                    .padding(.horizontal, 20)
                    .padding(.bottom, 12)
            }
            TaskListView()
            if state.syncAvailable || state.isDemo {
                Divider()
                SyncStatusBar()
                    .padding(.horizontal, 20)
                    .padding(.vertical, 10)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .overlay(alignment: .bottom) {
            ToastStack()
        }
        .dropDestination(for: String.self) { items, _ in
            let text = items.joined(separator: "\n")
            guard !state.engine.isTaskIdList(text: text) else { return false }
            state.addFromText(text)
            return true
        }
    }
}

/// A clear content hierarchy; the window title remains available in native chrome.
private struct ListHeading: SwiftUI.View {
    @Environment(AppState.self) private var state

    var body: some SwiftUI.View {
        VStack(alignment: .leading, spacing: 3) {
            Text(Strings.viewTitle(state.listing.title))
                .appFont(.title)
                .lineLimit(2)
                .accessibilityAddTraits(.isHeader)
            if state.view == .today || state.view == .morning || state.view == .tonight {
                TimelineView(.periodic(from: .now, by: 60)) { context in
                    Text(context.date, format: .dateTime.weekday(.wide).month(.wide).day())
                        .appFont(.caption)
                        .foregroundStyle(.secondary)
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, 22)
        .padding(.top, 16)
        .padding(.bottom, 14)
    }
}

struct BannerView: SwiftUI.View {
    @Environment(AppState.self) private var state
    let text: String
    var body: some SwiftUI.View {
        HStack {
            Image(systemName: "exclamationmark.triangle")
            Text(text).lineLimit(2)
            Spacer()
            SettingsLink { Text(String(localized: "Settings…")) }
            Button { state.dismissBanner() } label: { Image(systemName: "xmark") }
                .buttonStyle(.plain)
                .help(String(localized: "Dismiss"))
                .accessibilityLabel(String(localized: "Dismiss"))
        }
        .padding(10)
        .background(.yellow.opacity(0.15))
    }
}

struct ToolbarItems: ToolbarContent {
    @Environment(AppState.self) private var state

    var body: some ToolbarContent {
        ToolbarItem(placement: .primaryAction) {
            Button { state.newTask() } label: { Label(String(localized: "New Task"), systemImage: "plus") }
                .help(String(localized: "New Task (\(state.modifier.symbol)N)"))
        }
        if state.syncAvailable {
            ToolbarItem {
                Button { state.sync() } label: {
                    if state.syncInProgress {
                        ProgressView().controlSize(.small)
                    } else {
                        Label(String(localized: "Sync Now"), systemImage: "arrow.triangle.2.circlepath")
                    }
                }
                .disabled(state.syncInProgress)
                .help(state.syncInProgress ? String(localized: "Syncing…") : String(localized: "Sync Now (\(state.modifier.symbol)R)"))
            }
        }
        ToolbarItem {
            Menu {
                SortMenu(state: state)
                Divider()
                Button(String(localized: "Archive Completed Tasks")) { state.archiveDone() }
                    .disabled(!state.listing.canArchive)
                Divider()
                Button(String(localized: "Edit Project or Tag…")) { state.sheet = .editContext(state.view) }
                    .disabled(!state.contextIsEditable)
                Button(String(localized: "Delete Project or Tag…")) { state.confirmDeleteContext(state.view) }
                    .disabled(!state.contextIsDeletable)
            } label: {
                Label(String(localized: "View Options"), systemImage: "ellipsis.circle")
            }
            .help(String(localized: "View Options"))
        }
    }
}

/// Undo toasts, bottom centre, newest last.
struct ToastStack: SwiftUI.View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency
    @Environment(AppState.self) private var state
    var body: some SwiftUI.View {
        VStack(spacing: 6) {
            ForEach(state.toasts) { t in
                HStack(spacing: 12) {
                    if t.celebratesCompletion {
                        Image(systemName: "checkmark.circle.fill")
                            .symbolRenderingMode(.hierarchical)
                            .foregroundStyle(.tint)
                            .accessibilityHidden(true)
                    }
                    Text(t.text).lineLimit(2)
                    if t.undo != nil {
                        Button(String(localized: "Undo")) { state.undoToast(t) }
                            .buttonStyle(.borderless)
                            .tint(.accentColor)
                    }
                    Button { state.dismissToast(t.id) } label: { Image(systemName: "xmark") }
                        .buttonStyle(.plain)
                        .foregroundStyle(.secondary)
                        .help(String(localized: "Dismiss"))
                        .accessibilityLabel(String(localized: "Dismiss"))
                }
                .padding(.horizontal, 14)
                .padding(.vertical, 8)
                .background {
                    if reduceTransparency {
                        RoundedRectangle(cornerRadius: 14)
                            .fill(Color(nsColor: .windowBackgroundColor))
                            .strokeBorder(.secondary, lineWidth: 1)
                    } else {
                        RoundedRectangle(cornerRadius: 14)
                            .fill(.clear)
                            .glassEffect(.regular, in: .rect(cornerRadius: 14))
                    }
                }
                .transition(reduceMotion ? .opacity : .move(edge: .bottom).combined(with: .opacity))
            }
        }
        .padding(.horizontal, 20)
        .padding(.bottom, 16)
        .animation(reduceMotion ? nil : .easeOut(duration: 0.2), value: state.toasts)
        .accessibilityElement(children: .contain)
        .accessibilityLabel(String(localized: "Notifications"))
    }
}

struct AppStateKey: FocusedValueKey {
    typealias Value = AppState
}
extension FocusedValues {
    var appState: AppState? {
        get { self[AppStateKey.self] }
        set { self[AppStateKey.self] = newValue }
    }
}

/// A stable, accessible home for progress and failures, including empty task lists.
struct SyncStatusBar: SwiftUI.View {
    @Environment(AppState.self) private var state
    @State private var showDetails = false

    var body: some SwiftUI.View {
        HStack(spacing: 10) {
            if state.syncInProgress {
                ProgressView().controlSize(.small)
                    .accessibilityLabel(String(localized: "Syncing…"))
            } else {
                Image(systemName: state.syncError == nil ? "arrow.triangle.2.circlepath" : "exclamationmark.triangle.fill")
                    .foregroundStyle(state.syncError == nil ? Color.secondary : Color.orange)
                    .accessibilityHidden(true)
            }
            VStack(alignment: .leading, spacing: 2) {
                if !state.isDemo {
                    Text(state.selectedSyncMethod.label).appFont(.caption).fontWeight(.semibold)
                }
                Text(state.syncCaption ?? "").appFont(.caption).foregroundStyle(.secondary)
            }
            Spacer(minLength: 8)
            if state.syncError != nil {
                Button(String(localized: "Details")) { showDetails.toggle() }
                    .popover(isPresented: $showDetails) {
                        VStack(alignment: .leading, spacing: 12) {
                            Text(String(localized: "Sync needs attention")).font(.headline)
                            Text(state.syncError ?? "").textSelection(.enabled)
                            Text(String(localized: "Your tasks are saved on this Mac."))
                                .foregroundStyle(.secondary)
                        }.padding(20).frame(width: 340, alignment: .leading)
                    }
            }
            if state.syncAvailable {
                Button(state.syncError == nil ? String(localized: "Sync Now") : String(localized: "Retry")) { state.sync() }
                    .disabled(state.syncInProgress)
            }
        }
        .controlSize(.small)
        .accessibilityElement(children: .contain)
    }
}
