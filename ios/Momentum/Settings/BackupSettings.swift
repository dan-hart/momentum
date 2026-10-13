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
    @State private var settingsState = BackupState()
    @State private var importing = false
    @State private var exporting = false
    @State private var confirmingRestore = false
    @State private var includeSettings = false
    @State private var restoreSettings = false
    @State private var settingsOnly = false
    @State private var exportingSettings = false
    @AccessibilityFocusState private var statusFocused: Bool

    private var currentState: BackupState { settingsOnly ? settingsState : state }
    private var document: BackupDocument? { currentState.preparedExport.map(BackupDocument.init(data:)) }
    private var available: Bool {
        state.canStart && settingsState.canStart && model.worker != nil
            && !importing && !exporting && !exportingSettings && !model.sync.folderCopyInProgress
    }

    var body: some View {
        Form {
            Section {
                Toggle("Include app settings", isOn: $includeSettings)
                    .disabled(!available)
                Button {
                    settingsOnly = false
                    Task {
                        if await state.prepareExport(operation: { try await model.exportBackup(includeSettings: includeSettings) }) { exporting = true }
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
                Toggle("Restore included app settings", isOn: $restoreSettings)
                    .disabled(!available)
                Button { settingsOnly = false; importing = true } label: {
                    SettingsLabel("Import Backup", symbol: "square.and.arrow.down",
                                  subtitle: "Choose a Momentum backup")
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
                    Button(role: .destructive) { settingsOnly = false; confirmingRestore = true } label: {
                        Label("Restore Backup", systemImage: "arrow.counterclockwise")
                            .frame(minHeight: 44)
                    }
                    .accessibilityIdentifier("backup-restore")
                    .disabled(!available)
                    Button("Choose Another File", systemImage: "folder") { settingsOnly = false; importing = true }
                        .foregroundStyle(Color.primary).disabled(!available)
                }
            } footer: {
                Text("Restoring replaces all tasks on this device and discards unsynced changes. This cannot be undone.")
                    .foregroundStyle(AccentTheme.secondaryText)
            }

            Section {
                Button("Import app settings", systemImage: "square.and.arrow.down") {
                    settingsOnly = true; importing = true
                }.disabled(!available)
                Button("Export app settings", systemImage: "square.and.arrow.up") {
                    settingsOnly = true
                    Task {
                        if await settingsState.prepareExport(operation: { try await model.exportAppSettings() }) { exportingSettings = true }
                    }
                }.disabled(!available)
                if let selected = settingsState.selection {
                    Text(verbatim: selected.name).fixedSize(horizontal: false, vertical: true)
                    Button("Retry importing app settings") {
                        settingsOnly = true
                        Task { await settingsState.restore { try await model.importAppSettings($0) } }
                    }.disabled(!available)
                    Button("Choose Another File", systemImage: "folder") { settingsOnly = true; importing = true }
                        .disabled(!available)
                }
            } header: { Text("App settings only") } footer: {
                Text("Task display, grouping, sorting, archive and sound preferences. Credentials, connections, permissions and device controls stay local.")
                    .foregroundStyle(AccentTheme.secondaryText)
            }

            if let activity = currentState.activity {
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
            if let error = settingsState.error {
                Section {
                    Label("App settings couldn’t be transferred", systemImage: "exclamationmark.circle")
                    Text(error).textSelection(.enabled).accessibilityFocused($statusFocused)
                }
            }
            if let completion = settingsState.completion {
                Section {
                    Label(completion == .restored ? "App settings imported" : "App settings saved", systemImage: "checkmark.circle.fill")
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
        .onChange(of: settingsState.error) { _, error in if error != nil { statusFocused = true } }
        .onChange(of: settingsState.completion) { _, completion in if completion != nil { statusFocused = true } }
        .fileImporter(isPresented: $importing, allowedContentTypes: [.json], allowsMultipleSelection: false) { result in
            switch result {
            case .success(let urls):
                if let url = urls.first {
                    Task {
                        if await currentState.load(url) {
                            if settingsOnly { await settingsState.restore { try await model.importAppSettings($0) } }
                            else { confirmingRestore = true }
                        }
                    }
                }
            case .failure(let error): currentState.report(error)
            }
        }
        .fileExporter(isPresented: $exporting, document: document, contentTypes: [.json],
                      defaultFilename: String(localized: "Momentum Backup"), onCompletion: { state.finishExport($0) },
                      onCancellation: { state.finishExport(nil) })
        .fileExporter(isPresented: $exportingSettings, document: document, contentTypes: [.json],
                      defaultFilename: "momentum-settings", onCompletion: { settingsState.finishExport($0) },
                      onCancellation: { settingsState.finishExport(nil) })
        .alert("Replace All Tasks?", isPresented: $confirmingRestore) {
            Button("Restore Backup", role: .destructive) {
                Task { await state.restore { try await model.restoreBackup($0, restoreSettings: restoreSettings) } }
            }
            Button("Cancel", role: .cancel) { state.discardSelection() }
        } message: {
            Text("“\(state.selection?.name ?? "")” will replace all tasks on this device. Unsynced changes will be lost. This cannot be undone.")
        }
    }
}
