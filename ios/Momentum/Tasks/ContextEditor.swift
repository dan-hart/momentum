// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumMobile
import SwiftUI
import UIKit

struct ContextEditor: View {
    enum Kind { case project, tag }
    @Environment(MobileAppModel.self) private var model
    @Environment(\.dismiss) private var dismiss
    let kind: Kind
    var id: String?
    @State var name = ""
    @State private var color = Color.accentColor
    @State private var useColor = false
    @State private var saving = false
    @State private var confirmingDelete = false
    @State private var taskCount: UInt32 = 0
    @FocusState private var nameFocused: Bool
    var savedColor: String?

    var body: some View {
        NavigationStack {
            Form {
                TextField("Name", text: $name).focused($nameFocused)
                Toggle("Custom color", isOn: $useColor)
                if useColor { ColorPicker("Color", selection: $color, supportsOpacity: false) }
                if id != nil && id != "INBOX_PROJECT" {
                    Button(kind == .project ? "Delete Project" : "Delete Tag", role: .destructive) {
                        confirmingDelete = true
                    }
                }
            }
            .navigationTitle(kind == .project ? "Project" : "Tag")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) { Button("Cancel", systemImage: "xmark") { dismiss() } }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Save", systemImage: "checkmark") {
                        saving = true
                        Task {
                            guard let worker = model.worker else { saving = false; return }
                            let savedID: String?
                            let shouldUpdateMetadata: Bool
                            if let id {
                                savedID = id
                                shouldUpdateMetadata = true
                            } else if kind == .project {
                                savedID = await worker.createProject(name)
                                shouldUpdateMetadata = true
                            } else {
                                let result = await worker.createTagIfAbsent(name)
                                savedID = result?.id
                                shouldUpdateMetadata = result?.created == true
                            }
                            if let savedID {
                                if shouldUpdateMetadata {
                                    let hex = useColor ? color.momentumHex : ""
                                    await model.organize(kind == .project
                                        ? .updateProject(savedID, name, hex) : .updateTag(savedID, name, hex))
                                }
                                model.refreshAfterEdit()
                                dismiss()
                            }
                            saving = false
                        }
                    }
                    .disabled(name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || saving)
                    .keyboardShortcut(.return, modifiers: .command)
                }
            }
            .confirmationDialog(kind == .project ? "Delete project and all its tasks?" : "Delete tag?",
                                isPresented: $confirmingDelete, titleVisibility: .visible) {
                Button("Delete", role: .destructive) {
                    if let id {
                        Task {
                            await model.organize(kind == .project ? .deleteProject(id) : .deleteTag(id))
                            dismiss()
                        }
                    }
                }
            } message: {
                if kind == .project {
                    Text("This deletes \(taskCount) tasks from the project. This cannot be undone.")
                } else {
                    Text("Tasks keep their other tags. This cannot be undone.")
                }
            }
            .task {
                if let id, kind == .project { taskCount = await model.worker?.projectTaskCount(id) ?? 0 }
            }
            .onAppear {
                model.activeTaskEditors += 1
                if id == nil { nameFocused = true }
                if let savedColor, let parsed = Color(momentumHex: savedColor) {
                    useColor = true
                    color = parsed
                }
            }
            .onDisappear { model.activeTaskEditors = max(0, model.activeTaskEditors - 1) }
        }
    }
}

extension Color {
    init?(momentumHex: String) {
        let value = momentumHex.trimmingCharacters(in: CharacterSet(charactersIn: "#"))
        guard value.count == 6, let number = UInt32(value, radix: 16) else { return nil }
        self.init(.sRGB, red: Double((number >> 16) & 255) / 255,
                  green: Double((number >> 8) & 255) / 255, blue: Double(number & 255) / 255)
    }

    var momentumHex: String? {
        var r: CGFloat = 0, g: CGFloat = 0, b: CGFloat = 0, a: CGFloat = 0
        guard UIColor(self).getRed(&r, green: &g, blue: &b, alpha: &a) else { return nil }
        return String(format: "#%02X%02X%02X", Int(r * 255), Int(g * 255), Int(b * 255))
    }
}
