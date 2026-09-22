// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// The one object behind every window, menu and sheet. It holds the engine, the current
// view and its listing, the selection, toasts and sheets, and it is where every user
// action lands. Nothing here decides what a task list contains — the engine does, and
// this only asks it again after each change.
import AppKit
import Foundation
import MomentumCore
import Observation

public struct Toast: Identifiable, Equatable, Sendable {
    public let id = UUID()
    public let text: String
    public let undo: UInt64?
    public var celebratesCompletion = false
}

public enum Sheet: Identifiable, Equatable, Sendable {
    case newTask
    case editTask(String)
    case repeatSchedule(String)
    case moveToProject([String])
    case addTag([String])
    case editContext(View)
    case newProject
    case devices
    case shortcuts

    public var id: String {
        switch self {
        case .newTask: return "new-task"
        case .editTask(let id): return "edit-\(id)"
        case .repeatSchedule(let id): return "repeat-\(id)"
        case .moveToProject(let ids): return "move-\(ids.joined(separator: ","))"
        case .addTag(let ids): return "tag-\(ids.joined(separator: ","))"
        case .editContext(let v): return "edit-context-\(v)"
        case .newProject: return "new-project"
        case .devices: return "devices"
        case .shortcuts: return "shortcuts"
        }
    }
}

public enum Confirmation: Identifiable, Equatable, Sendable {
    case deleteProject(id: String, name: String, tasks: UInt32)
    case deleteTag(id: String, name: String)

    public var id: String {
        switch self {
        case .deleteProject(let id, _, _): return "project-\(id)"
        case .deleteTag(let id, _): return "tag-\(id)"
        }
    }
}

@MainActor
@Observable
public final class AppState {
    public let engine: Engine
    public let defaults: UserDefaults
    public let keychain: Keychain
    public let dataDir: URL
    public let isDemo: Bool

    public private(set) var view: View = .today
    public private(set) var listing: Listing
    public private(set) var sidebar: Sidebar
    public var selection: Set<String> = []
    public var searchText = ""
    public var isSearchPresented = false
    public private(set) var archiveLimit: UInt32 = 100
    public private(set) var toasts: [Toast] = []
    public private(set) var banner: String?
    public private(set) var syncStatus: SyncStatus
    public private(set) var isSyncing = false
    public private(set) var nearbySyncing = false
    public private(set) var syncError: String?
    public private(set) var selectedSyncMethod: SyncMethod = .off
    public var syncInProgress: Bool { isSyncing || nearbySyncing }
    /// A sync passed one second: show the spinner rather than flashing it.
    public private(set) var syncIsSlow = false
    public var sheet: Sheet?
    private var pendingSearchTaskId: String?
    public var confirmation: Confirmation?
    /// Bumped to move keyboard focus into the quick-add field.
    public private(set) var quickAddFocusRequest = 0
    public private(set) var todayOpenCount: UInt32 = 0
    public private(set) var pairingCode: String?
    public private(set) var p2pInfo: P2pInfo?
    public private(set) var nearbyLinked: [NearbyDevice] = []
    public private(set) var nearbyDiscovered: [NearbyDevice] = []
    public private(set) var modifier: ModifierKey
    public private(set) var colorful: Bool
    public private(set) var projectsExpanded: Bool
    public private(set) var tagsExpanded: Bool

    /// Names for the Edit menu's "Undo …" item, by undo batch id.
    private var undoNames: [UInt64: String] = [:]
    private var syncDebounce: Task<Void, Never>?
    private var tickTimer: Timer?
    private var periodicSync: Timer?
    private var fileWatcher: FileWatcher?
    private var defaultsObserver: NSObjectProtocol?

    public weak var notifier: Notifier?
    public weak var indexer: SearchIndexer?

    public var prefs: Preferences { Preferences(defaults) }
    public var isSearching: Bool { view == .search }
    public var canUndo: Bool { engine.canUndo() }

    // MARK: Lifecycle

    /// `services` starts the timers, the file watcher and the `mo` socket. Tests leave it
    /// off, which is what makes them fast and side-effect free.
    public init(engine: Engine,
                defaults: UserDefaults = .standard,
                keychain: Keychain = Keychain(),
                services: Bool = true,
                isDemo: Bool = false) {
        Preferences.register(defaults)
        self.engine = engine
        self.defaults = defaults
        self.keychain = keychain
        self.isDemo = isDemo
        self.dataDir = URL(fileURLWithPath: engine.dataDir())
        let p = Preferences(defaults)
        if !isDemo { p.readCliConfig(from: self.dataDir) }
        engine.setPreferences(prefs: p.core)
        self.selectedSyncMethod = p.syncMethod
        self.modifier = p.modifier
        self.colorful = p.colorful
        self.projectsExpanded = !defaults.bool(forKey: PrefKey.projectsCollapsed)
        self.tagsExpanded = !defaults.bool(forKey: PrefKey.tagsCollapsed)
        self.sidebar = engine.sidebar()
        self.listing = engine.listing(view: .today, archiveLimit: 100)
        self.syncStatus = engine.syncStatus()
        self.todayOpenCount = engine.todayOpenCount()
        _ = engine.spawnRepeats()
        refresh()
        if services {
            startServices()
        }
    }

    /// Timers, the store-file watch, the `mo` socket, nearby sync and the first sync.
    public func startServices() {
        updateDockBadge()
        prefs.writeCliConfig(to: dataDir)
        fileWatcher = FileWatcher(file: dataDir.appendingPathComponent("pending.json")) { [weak self] in
            Task { @MainActor in
                guard let self, !self.isSyncing, self.engine.reloadFromDisk() else { return }
                self.refresh()
            }
        }
        indexer?.reindex(engine.allTasks())
        try? engine.serveCli(delegate: CliBridge(state: self))
        tickTimer = Timer.scheduledTimer(withTimeInterval: 30, repeats: true) { [weak self] _ in
            Task { @MainActor in self?.tick() }
        }
        periodicSync = Timer.scheduledTimer(withTimeInterval: 300, repeats: true) { [weak self] _ in
            Task { @MainActor in
                guard let self, self.prefs.autoSync, self.prefs.syncConfigured else { return }
                self.sync()
            }
        }
        defaultsObserver = NotificationCenter.default.addObserver(
            forName: UserDefaults.didChangeNotification, object: defaults, queue: .main
        ) { [weak self] _ in
            Task { @MainActor in self?.preferencesChanged() }
        }
        applyP2pSetting()
        checkReminders()
        if prefs.autoSync, prefs.syncConfigured {
            sync()
        }
    }

    public func stopServices() {
        tickTimer?.invalidate()
        periodicSync?.invalidate()
        syncDebounce?.cancel()
        fileWatcher = nil
        if let o = defaultsObserver { NotificationCenter.default.removeObserver(o) }
        defaultsObserver = nil
        engine.stopCliServer()
        engine.p2pStop()
    }

    /// A preference changed somewhere (Settings, `defaults write`): re-read the ones the
    /// core and the views depend on.
    public func preferencesChanged() {
        let p = prefs
        if selectedSyncMethod != p.syncMethod {
            selectedSyncMethod = p.syncMethod
            syncDebounce?.cancel()
            nearbySyncing = false
            syncError = nil
            banner = nil
        }
        if p.core != engine.preferences() {
            engine.setPreferences(prefs: p.core)
            refresh()
        }
        if p.modifier != modifier { modifier = p.modifier }
        if p.colorful != colorful { colorful = p.colorful }
        updateDockBadge()
        p.writeCliConfig(to: dataDir)
        applyP2pSetting()
        syncStatus = engine.syncStatus()
    }

    public func setProjectsExpanded(_ on: Bool) {
        projectsExpanded = on
        defaults.set(!on, forKey: PrefKey.projectsCollapsed)
    }
    public func setTagsExpanded(_ on: Bool) {
        tagsExpanded = on
        defaults.set(!on, forKey: PrefKey.tagsCollapsed)
    }

    // MARK: Views

    public func go(to view: View) {
        selection.removeAll()
        archiveLimit = 100
        self.view = view
        if view == .search {
            isSearchPresented = true
        } else if isSearchPresented {
            isSearchPresented = false
        }
        refresh()
    }

    /// The sidebar entries in order: what ⌘1…⌘9 and next/previous view step through.
    public var orderedViews: [View] {
        sidebar.fixed.map(\.view)
            + (projectsExpanded ? sidebar.projects.map(\.view) : [])
            + (tagsExpanded ? sidebar.tags.map(\.view) : [])
    }
    public func stepView(_ delta: Int) {
        let views = orderedViews
        guard !views.isEmpty else { return }
        let pos = views.firstIndex(of: view) ?? 0
        let next = ((pos + delta) % views.count + views.count) % views.count
        go(to: views[next])
    }
    public func jump(to index: Int) {
        let views = orderedViews
        guard index >= 0, index < views.count else { return }
        go(to: views[index])
    }

    public func searchPresentationChanged(_ presented: Bool) {
        if presented, view != .search {
            go(to: .search)
        } else if !presented, view == .search {
            searchText = ""
            go(to: .today)
        }
    }
    public func searchTextChanged() {
        if view == .search { refresh() }
    }
    public func showMoreArchive() {
        archiveLimit += 200
        refresh()
    }

    /// Recompute what the window shows from the engine.
    public func refresh() {
        sidebar = engine.sidebar()
        if !engine.viewExists(view: view) {
            // The slot emptied, or the project or tag is gone.
            view = .today
        }
        listing = view == .search
            ? engine.search(query: searchText)
            : engine.listing(view: view, archiveLimit: archiveLimit)
        selection = selection.intersection(Set(allRowIds))
        todayOpenCount = engine.todayOpenCount()
        syncStatus = engine.syncStatus()
        updateDockBadge()
    }

    private func updateDockBadge() {
        notifier?.updateBadge(engine.taskCount(mode: prefs.dockBadgeMode.taskCountMode))
    }

    /// After a local change: refresh, keep the system search index fresh, upload later.
    private func changed() {
        refresh()
        indexer?.reindex(engine.allTasks())
        scheduleSync()
    }

    /// The engine already applied the socket command. Refresh system integrations too.
    public func cliStoreChanged() {
        prefs.readCliConfig(from: dataDir)
        changed()
    }

    public var allRowIds: [String] {
        listing.sections.flatMap { s in
            s.rows.compactMap { r -> String? in
                if case .task(let row) = r { return row.id }
                return nil
            }
        }
    }
    public func row(_ id: String) -> TaskRow? {
        for s in listing.sections {
            for r in s.rows {
                if case .task(let row) = r, row.id == id { return row }
            }
        }
        return nil
    }

    /// The selected tasks in list order: the targets of every task command.
    public var targets: [String] {
        allRowIds.filter { selection.contains($0) }
    }
    /// The one selected task, for the commands that act on a single row.
    public var target: String? {
        selection.count == 1 ? selection.first : nil
    }
    public var selectedTopLevel: [String] {
        targets.filter { row($0)?.isSubtask == false }
    }

    public func dragTransfer(for id: String) -> TaskTransfer? {
        guard row(id)?.archived == false else { return nil }
        let ids = selection.contains(id) ? targets : [id]
        return TaskTransfer(ids: ids.filter { row($0)?.archived == false })
    }

    // MARK: Applying outcomes

    public func apply(_ o: Outcome, undoName: String? = nil) {
        if o.changed {
            changed()
        }
        if let id = o.undo {
            undoNames[id] = undoName ?? o.message.map(Strings.undoName) ?? String(localized: "Change", bundle: .module)
            // The core keeps 50 batches; names for older ones can never be asked for again.
            if undoNames.count > 60 {
                for old in undoNames.keys.sorted().prefix(undoNames.count - 50) {
                    undoNames[old] = nil
                }
            }
        }
        if let m = o.message {
            let completed: Bool
            switch m {
            case .taskCompleted, .taskCompletedArchived, .tasksCompleted, .tasksCompletedArchived:
                completed = true
            default:
                completed = false
            }
            toast(Strings.message(m), undo: o.undo, celebratesCompletion: completed)
        }
        if o.syncNow, prefs.syncConfigured {
            sync()
        }
    }

    public func toast(_ text: String, undo: UInt64? = nil, celebratesCompletion: Bool = false) {
        let t = Toast(text: text, undo: undo, celebratesCompletion: celebratesCompletion)
        toasts.append(t)
        if toasts.count > 3 { toasts.removeFirst() }
        Task { [weak self] in
            try? await Task.sleep(for: .seconds(6))
            self?.dismissToast(t.id)
        }
    }
    public func dismissToast(_ id: UUID) {
        toasts.removeAll { $0.id == id }
    }
    public func undoToast(_ t: Toast) {
        dismissToast(t.id)
        if let id = t.undo {
            undoNames[id] = nil
            apply(engine.undoBatch(id: id))
        }
    }
    public func undo() {
        if let id = engine.undoTop() { undoNames[id] = nil }
        apply(engine.undo())
    }
    /// The Edit menu's item: "Undo Delete" rather than a bare "Undo".
    public var undoMenuTitle: String {
        if let id = engine.undoTop(), let name = undoNames[id] {
            return String(localized: "Undo \(name)", bundle: .module)
        }
        return String(localized: "Undo", bundle: .module)
    }

    // MARK: Creating

    public func addTask(_ text: String) {
        apply(engine.addTask(text: text, view: view))
    }
    public func addFromText(_ text: String) {
        apply(engine.addFromText(text: text, view: view))
    }
    @discardableResult
    public func addTaskForToday(_ text: String) -> Bool {
        let outcome = engine.addTaskForToday(text: text)
        apply(outcome)
        return outcome.changed
    }
    public func addTask(withNotes text: String, notes: String?, due: String?) {
        apply(engine.addTaskWithNotes(text: text, notes: notes, due: due))
    }
    public func createTask(_ draft: TaskDraft) {
        apply(engine.createTask(draft: draft, view: view))
    }
    public func saveTask(_ id: String, _ draft: TaskDraft) {
        apply(engine.saveTask(id: id, draft: draft))
    }
    public func addSubtask(_ parent: String, _ title: String) {
        apply(engine.addSubtask(parentId: parent, title: title))
    }
    public func duplicate(_ id: String) {
        apply(engine.duplicateTask(id: id))
        if let new = engine.lastAddedId() { selection = [new] }
    }
    public func copyTitle(_ id: String) {
        guard let title = engine.taskTitle(id: id) else { return }
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(title, forType: .string)
        toast(String(localized: "Title copied", bundle: .module))
    }
    public func requestQuickAddFocus() {
        quickAddFocusRequest += 1
    }
    public func newTask() {
        sheet = .newTask
    }
    public func open(_ id: String) {
        sheet = .editTask(id)
    }

    // MARK: Completing, deleting

    public func setDone(_ id: String, _ done: Bool) {
        apply(engine.setDone(id: id, done: done))
    }
    public func toggleDone(_ ids: [String]) {
        guard !ids.isEmpty else { return }
        apply(ids.count == 1 ? engine.toggleDone(id: ids[0]) : engine.bulkDone(ids: ids))
    }
    public func delete(_ ids: [String]) {
        guard !ids.isEmpty else { return }
        apply(ids.count == 1 ? engine.deleteTask(id: ids[0]) : engine.bulkDelete(ids: ids))
    }

    // MARK: Day moves

    public func toggleToday(_ ids: [String]) {
        guard !ids.isEmpty else { return }
        apply(ids.count == 1 ? engine.toggleToday(id: ids[0]) : engine.planForToday(ids: ids))
    }
    public func toggleSlot(_ ids: [String], _ slot: Slot) {
        apply(engine.toggleSlot(ids: ids, slot: slot))
    }
    public func moveToTomorrow(_ ids: [String]) {
        apply(engine.moveToTomorrow(ids: ids))
    }
    public func moveToNextWeek(_ ids: [String]) {
        apply(engine.moveToNextWeek(ids: ids))
    }
    public func moveToProject(_ ids: [String], _ projectId: String) {
        apply(engine.moveToProject(ids: ids, projectId: projectId))
    }
    public func addTag(_ ids: [String], tagId: String) {
        apply(engine.addTagTo(ids: ids, tagId: tagId))
    }
    public func addTag(_ ids: [String], name: String) {
        apply(engine.addTagByName(ids: ids, name: name))
    }
    @discardableResult
    public func drop(_ ids: [String], on dest: View) -> Bool {
        let out = engine.dropTasks(ids: ids, dest: dest)
        apply(out)
        return out.changed
    }
    @discardableResult
    public func reorder(_ moved: [String], before: String) -> Bool {
        let out = engine.reorderTasks(moved: moved, before: before, view: view)
        apply(out, undoName: String(localized: "Reorder", bundle: .module))
        return out.changed
    }
    public func nudge(_ delta: Int32) {
        guard let id = target else { return }
        apply(engine.nudge(id: id, delta: delta, view: view), undoName: String(localized: "Move", bundle: .module))
        selection = [id]
    }
    public func archiveDone() {
        apply(engine.archiveDone())
    }

    // MARK: Projects and tags

    public func addProject(_ title: String) {
        if let id = engine.addProject(title: title) {
            changed()
            go(to: .project(id: id))
        }
    }
    public func updateContext(_ v: View, title: String, color: String?) {
        switch v {
        case .project(let id): apply(engine.updateProject(id: id, title: title, color: color))
        case .tag(let id): apply(engine.updateTag(id: id, title: title, color: color))
        default: break
        }
    }
    public func confirmDeleteContext(_ v: View) {
        switch v {
        case .project(let id):
            guard id != inboxProjectId, let p = engine.project(id: id) else { return }
            confirmation = .deleteProject(id: id, name: p.title, tasks: engine.projectTaskCount(id: id))
        case .tag(let id):
            guard let t = engine.tag(id: id) else { return }
            confirmation = .deleteTag(id: id, name: t.title)
        default: break
        }
    }
    public func deleteContext(_ c: Confirmation) {
        switch c {
        case .deleteProject(let id, _, _):
            if view == .project(id: id) { view = .today }
            apply(engine.deleteProject(id: id))
        case .deleteTag(let id, _):
            if view == .tag(id: id) { view = .today }
            apply(engine.deleteTag(id: id))
        }
        confirmation = nil
    }
    public var contextIsEditable: Bool {
        if case .project = view { return true }
        if case .tag = view { return true }
        return false
    }
    public var contextIsDeletable: Bool {
        if case .project(let id) = view { return id != inboxProjectId }
        if case .tag = view { return true }
        return false
    }
    /// Upstream's fixed id for the project that cannot be deleted.
    public let inboxProjectId = "INBOX_PROJECT"

    // MARK: Repeats, reminders, the day turning over

    @discardableResult
    public func saveRepeat(_ id: String, _ draft: RepeatDraft) -> Bool {
        let out = engine.saveRepeat(taskId: id, draft: draft)
        apply(out)
        return out.changed
    }
    public func stopRepeat(_ id: String) {
        apply(engine.stopRepeat(taskId: id))
    }

    public func tick() {
        if engine.dayChanged() {
            _ = engine.spawnRepeats()
            changed()
        }
        checkReminders()
        syncStatus = engine.syncStatus()
    }
    public func checkReminders() {
        guard let notifier else { return }
        for r in engine.dueReminders() {
            notifier.reminder(r)
        }
        if let s = engine.morningSummary() {
            notifier.morningSummary(s)
        }
    }
    public func completeFromNotification(_ id: String) {
        apply(engine.setDone(id: id, done: true))
    }
    public func snooze(_ id: String, minutes: UInt32) {
        apply(engine.snooze(id: id, minutes: minutes))
    }
    public func completeByTitle(_ title: String) {
        apply(engine.completeByTitle(title: title))
    }
    public func showSearch(for title: String) {
        go(to: .search)
        searchText = title
        refresh()
    }

    @discardableResult
    public func showSearchForTask(_ id: String) -> Bool {
        guard let title = engine.taskTitle(id: id) else { return false }
        if let sheet {
            pendingSearchTaskId = id
            // Editors save on disappearance; resolve the title after that save.
            // Creation and other drafts stay open until their owner finishes them.
            if case .editTask = sheet { self.sheet = nil }
            return true
        }
        showSearch(for: title)
        return true
    }

    public func sheetDismissed() {
        guard let id = pendingSearchTaskId else { return }
        pendingSearchTaskId = nil
        if let title = engine.taskTitle(id: id) { showSearch(for: title) }
    }

    // MARK: Sync

    /// A burst of edits becomes one upload: 20 s after the last change.
    private func scheduleSync() {
        guard !isDemo, prefs.autoSync, prefs.syncConfigured else { return }
        syncDebounce?.cancel()
        syncDebounce = Task { [weak self] in
            try? await Task.sleep(for: .seconds(20))
            guard let self, !Task.isCancelled else { return }
            if self.isSyncing {
                self.scheduleSync()
            } else if self.engine.pendingCount() > 0 {
                self.sync()
            }
        }
    }

    public func sync() {
        guard !isDemo else {
            toast(String(localized: "Sync is unavailable in preview mode. Open Momentum normally to sync your tasks.", bundle: .module))
            return
        }
        if prefs.p2pEnabled {
            guard !syncInProgress else { return }
            applyP2pSetting()
            guard engine.p2pRunning() else { return }
            nearbySyncing = true
            syncError = nil
            engine.p2pSyncNow()
            return
        }
        guard prefs.syncEnabled else {
            if !prefs.p2pEnabled {
                toast(String(localized: "Sync is turned off. Turn it on in Settings.", bundle: .module))
            }
            return
        }
        guard let settings = prefs.nextcloud(keychain: keychain) else {
            banner = String(localized: "Sync needs attention: Nextcloud sync is not configured", bundle: .module)
            syncError = banner
            return
        }
        guard !isSyncing else { return }
        isSyncing = true
        syncError = nil
        syncIsSlow = false
        // Say what is happening right away, but only animate if it takes a while.
        Task { [weak self] in
            try? await Task.sleep(for: .seconds(1))
            if let self, self.isSyncing { self.syncIsSlow = true }
        }
        let engine = self.engine
        Task { [weak self] in
            let result = await Task.detached(priority: .utility) { () -> Result<SyncReport, Error> in
                do { return .success(try engine.syncNextcloud(settings: settings)) } catch { return .failure(error) }
            }.value
            guard let self else { return }
            self.isSyncing = false
            self.syncIsSlow = false
            switch result {
            case .success(let r):
                self.banner = nil
                self.refresh()
                self.indexer?.reindex(engine.allTasks())
                if r.downloaded || r.uploaded {
                    self.toast(Strings.message(.synced(opsUploaded: r.opsUploaded)))
                }
            case .failure(let e):
                self.syncError = Strings.error(e)
                switch e {
                case CoreError.Actionable, CoreError.NotConfigured:
                    // A problem the user can fix: a banner that stays, not a toast.
                    self.banner = String(localized: "Sync needs attention: \(Strings.error(e))", bundle: .module)
                case CoreError.Busy:
                    break // another cycle is running; it will report
                default:
                    self.toast(String(localized: "Sync failed: \(Strings.error(e))", bundle: .module))
                }
            }
            self.applyP2pSetting()
        }
    }
    public func dismissBanner() {
        banner = nil
    }

    public var syncCaption: String? {
        // Reading an observable value keeps views current after defaults changes.
        _ = selectedSyncMethod
        if isDemo { return String(localized: "Preview mode · Sync is off", bundle: .module) }
        if syncInProgress { return String(localized: "Syncing…", bundle: .module) }
        if syncError != nil { return String(localized: "Sync needs attention", bundle: .module) }
        switch prefs.syncMethod {
        case .off: return nil
        case .nextcloud:
            return syncStatus.lastNextcloudMs > 0
                ? String(localized: "Last synced \(Strings.ago(syncStatus.lastNextcloudMs))", bundle: .module)
                : Strings.notSyncedYet
        case .libresync:
            if syncStatus.linkedDevices == 0 { return String(localized: "No linked devices", bundle: .module) }
            return syncStatus.lastNearbyMs > 0
                ? String(localized: "Last synced \(Strings.ago(syncStatus.lastNearbyMs))", bundle: .module)
                : Strings.notSyncedYet
        }
    }
    public var syncAvailable: Bool {
        _ = selectedSyncMethod
        return !isDemo && prefs.syncMethod != .off
    }

    // MARK: Backup

    public func importBackup(_ url: URL) {
        do { apply(try engine.importBackup(path: url.path)) } catch { toast(Strings.error(error)) }
    }
    public func exportBackup(_ url: URL) {
        do { apply(try engine.exportBackup(path: url.path)) } catch { toast(Strings.error(error)) }
    }

    // MARK: Nearby devices

    public func applyP2pSetting() {
        let want = !isDemo && prefs.p2pEnabled && !isSyncing
        if want, !engine.p2pRunning() {
            do {
                p2pInfo = try engine.p2pStart(
                    delegate: P2pBridge(state: self),
                    secrets: KeychainSecretStore(keychain: keychain),
                    deviceName: Host.current().localizedName ?? "Mac",
                    demoKeyDir: nil)
                if let code = ProcessInfo.processInfo.environment["MOMENTUM_P2P_CODE"] {
                    engine.p2pBeginPairingWith(code: code)
                }
            } catch {
                syncError = Strings.message(.nearbyStartFailed(error: Strings.error(error)))
                toast(syncError!)
            }
        } else if !want, engine.p2pRunning() {
            engine.p2pStop()
            p2pInfo = nil
        }
        syncStatus = engine.syncStatus()
    }
    public func handle(_ event: P2pEvent) {
        switch event {
        case .syncStarted:
            guard prefs.p2pEnabled, !isDemo else { return }
            nearbySyncing = true
            syncError = nil
        case .syncCompleted(let error):
            guard prefs.p2pEnabled, !isDemo else { return }
            nearbySyncing = false
            syncError = error
            syncStatus = engine.syncStatus()
        case .started(let port):
            if var info = p2pInfo {
                info.port = port
                p2pInfo = info
            }
        case .storeChanged:
            let previous = syncStatus.lastNearbyMs
            refresh()
            if syncStatus.lastNearbyMs > previous { syncError = nil }
            indexer?.reindex(engine.allTasks())
        case .statusChanged:
            let previous = syncStatus.lastNearbyMs
            syncStatus = engine.syncStatus()
            if syncStatus.lastNearbyMs > previous { syncError = nil }
        case .devicesChanged, .discoveryUpdated:
            refreshDevices()
            syncStatus = engine.syncStatus()
        case .notice(let message):
            toast(Strings.message(message))
        }
    }
    public func refreshDevices() {
        nearbyLinked = engine.p2pDevices()
        nearbyDiscovered = engine.p2pDiscovered()
    }
    public func showDevices() {
        guard !isDemo else { sync(); return }
        guard prefs.p2pEnabled else {
            toast(String(localized: "Turn on nearby sync in Settings first.", bundle: .module))
            return
        }
        pairingCode = engine.p2pBeginPairing()
        p2pInfo = engine.p2pInfo()
        refreshDevices()
        sheet = .devices
    }
    /// The Nearby Devices sheet closed: the pairing window and discovery stop with it.
    public func devicesClosed() {
        engine.p2pEndPairing()
        pairingCode = nil
        syncStatus = engine.syncStatus()
    }
    public func link(_ device: NearbyDevice, code: String) {
        guard let address = device.address else { return }
        do {
            try engine.p2pLink(address: address, code: code)
            toast(Strings.message(.linking))
        } catch {
            toast(Strings.error(error))
        }
    }
    public func unlink(_ device: NearbyDevice) {
        engine.p2pUnlink(deviceId: device.deviceId)
        refreshDevices()
    }

    // MARK: URLs and the command line

    /// `momentum://add?title=&notes=&due=&tags=`, `superproductivity://create-task`,
    /// `superproductivity://complete-task?title=`.
    public func handle(url: URL) {
        let items = URLComponents(url: url, resolvingAgainstBaseURL: false)?.queryItems ?? []
        func param(_ k: String) -> String? {
            items.first { $0.name == k }?.value
        }
        switch url.host() {
        case "add", "create-task":
            guard let title = param("title"), !title.trimmingCharacters(in: .whitespaces).isEmpty else { return }
            var text = title
            if let tags = param("tags") {
                for t in tags.split(separator: ",").map({ $0.trimmingCharacters(in: .whitespaces) }) where !t.isEmpty {
                    text += " #\(t)"
                }
            }
            addTask(withNotes: text, notes: param("notes"), due: param("due"))
        case "complete-task":
            if let title = param("title") { completeByTitle(title) }
        default:
            break
        }
    }

    /// `Momentum --add "title"`, `--today`, `--search q`, `--quick-add`, `--background`.
    /// Returns whether the main window should be shown.
    @discardableResult
    public func handle(arguments: [String]) -> Bool {
        var show = true
        var it = arguments.dropFirst().makeIterator()
        while let a = it.next() {
            switch a {
            case "--add":
                if let t = it.next() { addTaskForToday(t) }
                show = false
            case "--today":
                go(to: .today)
            case "--search":
                if let q = it.next() { showSearch(for: q) }
            case "--background", "--quick-add":
                show = false
            default:
                break
            }
        }
        return show
    }
}
