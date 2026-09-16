// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
import MomentumCore
import MomentumKit
import SwiftUI

struct SidebarView: SwiftUI.View {
    @Environment(AppState.self) private var state

    var body: some SwiftUI.View {
        List(selection: Binding(get: { Optional(state.view) }, set: { if let v = $0 { state.go(to: v) } })) {
            Section {
                ForEach(state.sidebar.fixed, id: \.view) { entry in
                    SidebarRow(entry: entry)
                }
            }
            Section(isExpanded: Binding(
                get: { state.projectsExpanded }, set: { state.setProjectsExpanded($0) })) {
                ForEach(state.sidebar.projects, id: \.view) { entry in
                    SidebarRow(entry: entry)
                }
            } header: {
                Text(String(localized: "Projects")).appFont(.caption)
            }
            Section(isExpanded: Binding(
                get: { state.tagsExpanded }, set: { state.setTagsExpanded($0) })) {
                ForEach(state.sidebar.tags, id: \.view) { entry in
                    SidebarRow(entry: entry)
                }
            } header: {
                Text(String(localized: "Tags")).appFont(.caption)
            }
        }
        .listStyle(.sidebar)
        .safeAreaInset(edge: .bottom) {
            HStack {
                Button { state.newTask() } label: {
                    Label(String(localized: "New Task"), systemImage: "plus.circle")
                }
                .buttonStyle(.borderedProminent)
                .tint(.accentColor)
                Spacer()
                Button { state.sheet = .newProject } label: {
                    Image(systemName: "folder.badge.plus")
                }
                .buttonStyle(.borderless)
                .help(String(localized: "New Project…"))
                .accessibilityLabel(String(localized: "New Project…"))
            }
            .padding(10)
            .background(.background)
        }
    }
}

struct SidebarRow: SwiftUI.View {
    @Environment(AppState.self) private var state
    @LabelColors private var colorful: Bool
    let entry: SidebarEntry
    @State private var targeted = false

    var body: some SwiftUI.View {
        Label {
            Text(title)
                .appFont()
                .lineLimit(2)
                .foregroundStyle(isSelected ? Color(nsColor: .alternateSelectedControlTextColor) : .primary)
        } icon: {
            Image(systemName: symbol)
                .foregroundStyle(iconColor)
        }
        .tag(entry.view)
        .listRowBackground(Group {
            if isSelected || targeted {
                RoundedRectangle(cornerRadius: 6)
                    .fill(isSelected ? Color.accentColor : Color.accentColor.opacity(0.2))
                    .padding(.horizontal, 10)
            }
        })
        .dropDestination(for: TaskTransfer.self) { items, _ in
            let ids = items.flatMap(\.ids)
            return state.drop(ids, on: entry.view)
        } isTargeted: { targeted = $0 }
        .contextMenu { contextMenu }
    }

    private var title: String {
        switch entry.view {
        case .project, .tag: return entry.title
        default: return Strings.sidebarTitle(entry.view)
        }
    }
    private var symbol: String {
        switch entry.view {
        case .today: return "star"
        case .morning: return "sun.max"
        case .tonight: return "moon.stars"
        case .upcoming: return "calendar"
        case .archive: return "archivebox"
        case .search: return "magnifyingglass"
        case .project: return "folder"
        case .tag: return "tag"
        }
    }
    private var isSelected: Bool { state.view == entry.view }

    private var iconColor: Color {
        if isSelected { return Color(nsColor: .alternateSelectedControlTextColor) }
        if entry.view == .today { return .accentColor }
        if colorful, let c = entry.color, let color = Color(hex: c) {
            return color
        }
        return entry.view == .today ? .accentColor : .secondary
    }

    @ViewBuilder
    private var contextMenu: some SwiftUI.View {
        switch entry.view {
        case .project(let id):
            Button(String(localized: "Open")) { state.go(to: entry.view) }
            Button(String(localized: "New Task Here…")) { state.go(to: entry.view); state.newTask() }
            Divider()
            Button(String(localized: "Edit…")) { state.sheet = .editContext(entry.view) }
            if id != "INBOX_PROJECT" {
                Button(String(localized: "Delete Project…"), role: .destructive) { state.confirmDeleteContext(entry.view) }
            }
        case .tag:
            Button(String(localized: "Open")) { state.go(to: entry.view) }
            Divider()
            Button(String(localized: "Edit…")) { state.sheet = .editContext(entry.view) }
            Button(String(localized: "Delete Tag…"), role: .destructive) { state.confirmDeleteContext(entry.view) }
        default:
            EmptyView()
        }
    }
}

extension Color {
    /// `#rrggbb` from the sync data.
    init?(hex: String) {
        var s = hex.trimmingCharacters(in: .whitespaces)
        guard s.hasPrefix("#") else { return nil }
        s.removeFirst()
        guard s.count == 6, let v = UInt32(s, radix: 16) else { return nil }
        self.init(red: Double((v >> 16) & 0xff) / 255, green: Double((v >> 8) & 0xff) / 255, blue: Double(v & 0xff) / 255)
    }
}
