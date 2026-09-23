// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumCore
import MomentumKit
import MomentumMobile
import SwiftUI

struct ListPicker: SwiftUI.View {
    @Environment(MobileAppModel.self) private var model
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.colorSchemeContrast) private var contrast
    @Environment(\.accessibilityDifferentiateWithoutColor) private var differentiate
    @AppStorage(PrefKey.colorfulLabels) private var colorful = true
    let snapshot: TaskSnapshot?
    @Binding var selection: MomentumCore.View
    @State private var context: EditableContext?

    private struct EditableContext: Identifiable {
        let id = UUID()
        let kind: ContextEditor.Kind
        var contextID: String?
        var name = ""
        var color: String?
    }

    var body: some SwiftUI.View {
        List(selection: listSelection) {
            Section {
                NavigationLink(value: MomentumCore.View.today) {
                    sidebarLabel("Today", systemImage: "star", selectedImage: "star.fill", view: .today)
                }
                    .accessibilityIdentifier("list-today")
                if snapshot?.sidebar.fixed.contains(where: { $0.view == .morning }) == true {
                    NavigationLink(value: MomentumCore.View.morning) {
                        sidebarLabel("Morning", systemImage: "sun.max", selectedImage: "sun.max.fill", view: .morning)
                    }
                        .accessibilityIdentifier("list-morning")
                }
                if snapshot?.sidebar.fixed.contains(where: { $0.view == .tonight }) == true {
                    NavigationLink(value: MomentumCore.View.tonight) {
                        sidebarLabel("Evening", systemImage: "moon", selectedImage: "moon.fill", view: .tonight)
                    }
                        .accessibilityIdentifier("list-evening")
                }
            } header: { Text("Your day").foregroundStyle(AccentTheme.secondaryText) }
            if let snapshot {
                Section {
                    ForEach(snapshot.sidebar.projects, id: \.view) { project in
                        NavigationLink(value: project.view) {
                            contextLabel(project.title, systemImage: "folder.fill",
                                         color: project.color, count: project.taskCount, view: project.view)
                        }
                        .contextMenu {
                            Button("Edit Project") { context = .init(kind: .project, contextID: contextID(project.view), name: project.title, color: project.color) }
                        }
                        .swipeActions(edge: .trailing) {
                            Button("Edit Project", systemImage: "pencil") {
                                context = .init(kind: .project, contextID: contextID(project.view),
                                                name: project.title, color: project.color)
                            }
                            .tint(.accentColor)
                        }
                        .dropDestination(for: TaskTransfer.self) { transfers, _ in
                            Task { await model.organize(.drop(transfers.flatMap(\.ids), project.view)) }
                            return true
                        }
                    }
                } header: { Text("Projects").foregroundStyle(AccentTheme.secondaryText) }
                Section {
                    ForEach(snapshot.sidebar.tags, id: \.view) { tag in
                        NavigationLink(value: tag.view) {
                            contextLabel(tag.title, systemImage: "tag.fill",
                                         color: tag.color, count: tag.taskCount, view: tag.view)
                        }
                        .contextMenu {
                            Button("Edit Tag") { context = .init(kind: .tag, contextID: contextID(tag.view), name: tag.title, color: tag.color) }
                        }
                        .swipeActions(edge: .trailing) {
                            Button("Edit Tag", systemImage: "pencil") {
                                context = .init(kind: .tag, contextID: contextID(tag.view),
                                                name: tag.title, color: tag.color)
                            }
                            .tint(.accentColor)
                        }
                        .dropDestination(for: TaskTransfer.self) { transfers, _ in
                            Task { await model.organize(.drop(transfers.flatMap(\.ids), tag.view)) }
                            return true
                        }
                    }
                } header: { Text("Tags").foregroundStyle(AccentTheme.secondaryText) }
            }
            Section {
                NavigationLink(value: MomentumCore.View.archive) {
                    sidebarLabel("Archive", systemImage: "archivebox", selectedImage: "archivebox.fill", view: .archive)
                }
            }
        }
        .navigationTitle("Lists")
        .momentumNavigationCanvas()
        .toolbar {
            Menu {
                Button("New Project") { context = .init(kind: .project) }
                Button("New Tag") { context = .init(kind: .tag) }
            } label: { Label("Add list", systemImage: "plus") }
        }
        .sheet(item: $context, onDismiss: model.presentPendingNotification) { item in
            ContextEditor(kind: item.kind, id: item.contextID, name: item.name, savedColor: item.color)
        }
    }

    private var listSelection: Binding<MomentumCore.View?> {
        Binding(
            get: { selection },
            set: { if let view = $0 { selection = view } }
        )
    }

    private func sidebarLabel(_ title: LocalizedStringKey, systemImage: String,
                              selectedImage: String, view: MomentumCore.View) -> some SwiftUI.View {
        Label {
            Text(title)
        } icon: {
            sidebarSymbol(systemImage, selectedImage: selectedImage, view: view)
        }
    }

    private func contextLabel(_ title: String, systemImage: String,
                              color: String?, count: UInt32, view: MomentumCore.View) -> some SwiftUI.View {
        HStack(spacing: 10) {
            sidebarSymbol(systemImage, selectedImage: systemImage, view: view)
                .foregroundStyle(contextColor(color))
            Text(title)
            Spacer(minLength: 8)
            Text(count.formatted())
                .font(.callout)
                .foregroundStyle(AccentTheme.secondaryText)
                .monospacedDigit()
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel(count == 1
            ? String(localized: "\(title), 1 task")
            : String(localized: "\(title), \(count) tasks"))
    }

    private func sidebarSymbol(_ systemImage: String, selectedImage: String,
                               view: MomentumCore.View) -> some SwiftUI.View {
        let motion = InteractionMotionPolicy.presentation(reduceMotion: reduceMotion)
        return Image(systemName: selection == view ? selectedImage : systemImage)
            .contentTransition(.symbolEffect(.replace))
            .transaction { if !motion.usesSymbolTransition { $0.animation = nil; $0.disablesAnimations = true } }
            .animation(motion.animation, value: selection == view)
    }

    private func contextColor(_ color: String?) -> Color {
        guard colorful, contrast != .increased, !differentiate, let color else {
            return AccentTheme.secondaryText
        }
        return AccentTheme.ink(hex: color)
    }

    private func contextID(_ view: MomentumCore.View) -> String? {
        switch view {
        case .project(let id), .tag(let id): id
        default: nil
        }
    }
}
