// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumMobile
import SwiftUI
import UniformTypeIdentifiers

struct BackupDocument: FileDocument {
    static var readableContentTypes: [UTType] { [.json] }
    var data: Data
    init(data: Data) { self.data = data }
    init(configuration: ReadConfiguration) throws {
        guard let contents = configuration.file.regularFileContents else { throw CocoaError(.fileReadCorruptFile) }
        data = contents
    }
    func fileWrapper(configuration: WriteConfiguration) throws -> FileWrapper {
        FileWrapper(regularFileWithContents: data)
    }
}

struct BackupSettings: View {
    @Environment(MobileAppModel.self) private var model
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var state = BackupState()
    @State private var importing = false
    @State private var exporting = false
    @State private var confirmingRestore = false
    @AccessibilityFocusState private var statusFocused: Bool

    private var document: BackupDocument? { state.preparedExport.map(BackupDocument.init(data:)) }
    private var available: Bool { state.canStart && model.worker != nil && !importing && !exporting }

    var body: some View {
        Form {
            Section {
                Button {
                    Task {
                        if await state.prepareExport(operation: { try await model.exportBackup() }) { exporting = true }
                    }
                } label: {
                    SettingsLabel("Export Backup", symbol: "square.and.arrow.up",
                                  subtitle: "Save a copy in Files or another location")
                        .frame(minHeight: 44)
                }
                .accessibilityIdentifier("backup-export")
                .disabled(!available)
            } footer: {
                Text("Backups include your tasks, projects, tags, and repeat schedules. Keep a copy somewhere safe.")
                    .foregroundStyle(AccentTheme.secondaryText)
            }

            Section {
                Button { importing = true } label: {
                    SettingsLabel("Import Backup", symbol: "square.and.arrow.down",
                                  subtitle: "Choose a Momentum or Super Productivity backup")
                        .frame(minHeight: 44)
                }
                .accessibilityIdentifier("backup-import")
                .disabled(!available)
                if let selected = state.selection {
                    VStack(alignment: .leading, spacing: 4) {
                        Text("Selected Backup").font(.caption).foregroundStyle(AccentTheme.secondaryText)
                        Text(verbatim: selected.name).fixedSize(horizontal: false, vertical: true)
                    }
                    .accessibilityElement(children: .combine)
                    Button(role: .destructive) { confirmingRestore = true } label: {
                        Label("Restore Backup", systemImage: "arrow.counterclockwise")
                            .frame(minHeight: 44)
                    }
                    .accessibilityIdentifier("backup-restore")
                    .disabled(!available)
                    Button("Choose Another File", systemImage: "folder") { importing = true }
                        .foregroundStyle(Color.primary).disabled(!available)
                }
            } footer: {
                Text("Restoring replaces all tasks on this device and discards unsynced changes. This cannot be undone.")
                    .foregroundStyle(AccentTheme.secondaryText)
            }

            if let activity = state.activity {
                Section {
                    switch activity {
                    case .reading: ProgressView("Reading Backup…")
                    case .preparingExport: ProgressView("Preparing Backup…")
                    case .restoring: ProgressView("Restoring Backup…")
                    }
                }
            }
            if let error = state.error {
                Section {
                    Label("Backup Couldn’t Be Completed", systemImage: "exclamationmark.circle")
                        .font(.headline)
                    Text(error).textSelection(.enabled)
                        .accessibilityIdentifier("backup-error")
                        .accessibilityFocused($statusFocused)
                }
            }
            if let completion = state.completion {
                Section {
                    Group {
                        if completion == .restored { Label("Backup Restored", systemImage: "checkmark.circle.fill") }
                        else { Label("Backup Saved", systemImage: "checkmark.circle.fill") }
                    }
                        .foregroundStyle(.primary)
                        .accessibilityIdentifier("backup-status")
                        .accessibilityFocused($statusFocused)
                }
            }
        }
        .navigationTitle("Backups")
        .momentumNavigationCanvas()
        .navigationBarTitleDisplayMode(.inline)
        .animation(reduceMotion ? nil : .easeInOut(duration: 0.18), value: state.completion)
        .onChange(of: state.error) { _, error in if error != nil { statusFocused = true } }
        .onChange(of: state.completion) { _, completion in if completion != nil { statusFocused = true } }
        .fileImporter(isPresented: $importing, allowedContentTypes: [.json], allowsMultipleSelection: false) { result in
            switch result {
            case .success(let urls):
                if let url = urls.first {
                    Task { if await state.load(url) { confirmingRestore = true } }
                }
            case .failure(let error): state.report(error)
            }
        }
        .fileExporter(isPresented: $exporting, document: document, contentTypes: [.json],
                      defaultFilename: String(localized: "Momentum Backup"), onCompletion: { state.finishExport($0) },
                      onCancellation: { state.finishExport(nil) })
        .alert("Replace All Tasks?", isPresented: $confirmingRestore) {
            Button("Restore Backup", role: .destructive) {
                Task { await state.restore { try await model.restoreBackup($0) } }
            }
            Button("Cancel", role: .cancel) { state.discardSelection() }
        } message: {
            Text("“\(state.selection?.name ?? "")” will replace all tasks on this device. Unsynced changes will be lost. This cannot be undone.")
        }
    }
}
