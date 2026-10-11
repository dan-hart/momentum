// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import android.app.Application
import android.app.KeyguardManager
import android.net.Uri
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.viewModelScope
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.*
import com.codedbydan.momentum.platform.PlatformCoordinator
import com.codedbydan.momentum.platform.CompletionSound
import com.codedbydan.momentum.platform.CompletionSoundPlayer
import com.codedbydan.momentum.platform.isInteractiveCompletion
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import java.io.File
import java.io.InputStream
import java.util.UUID

enum class Tab { Today, Upcoming, Search, Settings }
data class UiState(
    val tab: Tab = Tab.Today,
    val view: View = View.Today,
    val query: String = "",
    val snapshot: Snapshot? = null,
    val loading: Boolean = true,
    val busy: Boolean = false,
    val selected: Set<String> = emptySet(),
    val selecting: Boolean = false,
    val error: String? = null,
    val notice: Outcome? = null,
    val editor: EditorDraft? = null,
    val editorError: Boolean = false,
    val importReady: Boolean = false,
    val importPreparing: Boolean = false,
    val settingsPage: String = "main",
    val editorLoading: Boolean = false,
    val editorUnavailable: Boolean = false,
    val editorCapacityError: Boolean = false,
    val draftRecoveryPending: Boolean = false,
    val restoring: Boolean = false,
    val queuedCaptures: Int = 0,
)

class MomentumViewModel(
    application: Application,
    private val saved: SavedStateHandle,
    val repository: MomentumRepository = (application as MomentumApplication).repository,
    private val notifyChanges: () -> Unit = { (application as MomentumApplication).platform.changed() },
    private val pauseForRestore: suspend () -> Unit = { (application as MomentumApplication).platform.pauseForRestore() },
    val platform: PlatformCoordinator = if(repository.directory.canonicalFile == File(application.noBackupFilesDir, "tasks").canonicalFile)
        (application as MomentumApplication).platform else PlatformCoordinator(application, repository),
    private val resumeAfterRestore: suspend () -> Unit = { platform.resumeAfterRestore() },
    private val openImportSource: (Uri) -> InputStream? = { application.contentResolver.openInputStream(it) },
    private val soundPlayer: CompletionSoundPlayer = CompletionSound(application),
) : AndroidViewModel(application) {
    private val draftBudget = DraftStorageBudget()
    private var legacyInlineDraft = saved.get<String>("editor")?.let(EditorDraft::decode)
    private val blockedInlineDraft = legacyInlineDraft?.let { !draftBudget.admit(it) } == true
    private val mutable = MutableStateFlow(UiState(
        editor = legacyInlineDraft.takeUnless { blockedInlineDraft },
        error = if(blockedInlineDraft) application.getString(R.string.draft_open_capacity) else null,
        tab = saved.get<String>("tab")?.let { value -> Tab.entries.firstOrNull { it.name == value } } ?: Tab.Today,
        view = restoreView(saved.get<String>("view"), saved.get<String>("context")),
        query = saved["query"] ?: "",
        settingsPage = saved["settings-page"] ?: "main",
        editorLoading = saved.get<String>("editor-id") != null,
        draftRecoveryPending = blockedInlineDraft || saved.get<String>("failed-editor-id") != null,
    ))
    val state: StateFlow<UiState> = mutable.asStateFlow()
    private var request = 0L
    private var archiveLimit = 100u
    private var importFile: File? = null
    private var importPreparation: Job? = null
    private var importGeneration = 0L
    private var editorView = restoreView(saved.get<String>("editor-view"), saved.get<String>("editor-context"))
    private data class FailedDraft(val id: String, val origin: View)
    private val failedDrafts = saved.get<ArrayList<String>>("failed-editor-references").orEmpty().chunked(3)
        .filter { it.size == 3 && runCatching { UUID.fromString(it[0]).toString() == it[0] }.getOrDefault(false) }
        .take(32).map { FailedDraft(it[0], restoreView(it[1], it[2])) }.toMutableList()
    private var blockedDraftRecovery = false
    private val captures = CaptureQueue(repository.directory)
    private val captureLock = Mutex()
    private var capturesLoaded = false
    private var activeCaptureId: String? = null
    private var pendingCaptureCompletion: String? = null
    private var pendingOpen: String? = saved["pending-open"]
    private var pendingOpenSystemSearch: Boolean = saved["pending-open-system-search"] ?: false
    private var searchJob: Job? = null
    private var quickAddJob: Job? = null
    private val drafts = DraftStore(repository.directory)
    private val draftLock = Mutex()
    private var draftId: String? = saved["editor-id"]
    private var persistJob: Job? = null
    private var soundSettingsJob: Job? = null
    private var soundGeneration = 0L
    private var soundForeground = true

    init {
        // The admitted editor or preserved SavedState value owns this source after initialization.
        legacyInlineDraft = null
        refresh()
        val restoreId = draftId
        viewModelScope.launch {
            try {
                captureLock.withLock {
                    try {
                        activeCaptureId = withContext(Dispatchers.IO) { restoreId?.takeIf(captures::contains) }
                        captureCount(withContext(Dispatchers.IO) { captures.size() })
                        capturesLoaded = true
                    } catch(e: Exception) {
                        if(e is CancellationException) throw e
                        // An unreadable queue index must not prevent restoring a valid active editor.
                        capturesLoaded = false; fail(e)
                    }
                    if(restoreId != null) {
                        val draft = withContext(Dispatchers.IO) { drafts.read(restoreId) }
                        // Preserve a readable legacy record whose extra v2 marker would exceed
                        // the existing budget through the same explicit recovery path as I/O failures.
                        if(draft != null && !draftBudget.admit(draft))
                            throw IllegalStateException(getApplication<Application>().getString(R.string.draft_open_capacity))
                        // Queue ownership must be known before a restored editor can be dismissed.
                        mutable.update { it.copy(editor = draft, editorLoading = false) }
                        if(draft == null) { saved.remove<String>("editor-id"); draftId = null }
                    }
                }
                resumeCapture()
            } catch(e: Exception) {
                if(e is CancellationException) throw e
                // Preserve both unreadable bytes and the bounded reference needed for Retry.
                activeCaptureId = null
                if(restoreId != null) retainFailedDraft(restoreId, editorView)
                mutable.update { it.copy(editorLoading = false) }; fail(e)
            }
        }
    }
    fun refresh() {
        val version = ++request
        val route = mutable.value.view
        val query = mutable.value.query
        mutable.update { it.copy(loading = true) }
        viewModelScope.launch {
            try {
                val snapshot = repository.snapshot(route, query, archiveLimit)
                if (version == request) mutable.update {
                    it.copy(snapshot = snapshot, view = snapshot.listing.view, loading = false,
                        selected = it.selected.intersect(snapshot.listing.sections.flatMap { section ->
                            section.rows.filterIsInstance<Row.Task>().map { row -> row.row.id }
                        }.toSet()))
                }
            } catch (e: Exception) {
                if (e is CancellationException) throw e
                if (version == request) mutable.update { it.copy(loading = false, error = e.message ?: "") }
            }
        }
    }
    fun navigate(view: View, tab: Tab = when(view) { View.Upcoming -> Tab.Upcoming; View.Search -> Tab.Search; else -> Tab.Today }) {
        archiveLimit = 100u
        saved["view"] = viewKey(view); saved["context"] = when(view) { is View.Project -> view.id; is View.Tag -> view.id; else -> "" }
        saved["tab"] = tab.name
        if(tab == Tab.Today) { saved["today-view"] = viewKey(view); saved["today-context"] = saved.get<String>("context") }
        mutable.update { it.copy(tab = tab, view = view, selected = emptySet(), selecting = false, snapshot = null) }
        refresh()
    }
    fun tab(tab: Tab) {
        if (tab == Tab.Settings) {
            saved["tab"] = tab.name
            mutable.update { it.copy(tab = tab, selecting = false, selected = emptySet()) }
        } else navigate(when(tab) { Tab.Today -> restoreView(saved["today-view"], saved["today-context"]); Tab.Upcoming -> View.Upcoming; else -> View.Search }, tab)
    }
    fun settingsPage(page: String) { saved["settings-page"] = page; mutable.update { it.copy(settingsPage = page) } }
    fun openSync() { settingsPage("sync"); tab(Tab.Settings) }
    fun search(query: String) {
        if(query.length > 4096) {
            mutable.update { it.copy(error = getApplication<Application>().getString(R.string.search_too_large)) }
            return
        }
        saved["query"] = query
        mutable.update { it.copy(query = query) }
        // Immediately invalidate the prior result, including during the debounce window.
        ++request
        searchJob?.cancel()
        searchJob = viewModelScope.launch { delay(180); refresh() }
    }
    fun moreArchive() { archiveLimit += 100u; refresh() }
    fun clearError() { mutable.update { it.copy(error = null, editorError = false) } }
    fun consumeNotice(outcome: Outcome) { mutable.update { if (it.notice == outcome) it.copy(notice = null) else it } }
    fun select(id: String) = mutable.update {
        it.copy(selecting = true, selected = if (id in it.selected) it.selected - id else it.selected + id)
    }
    fun selectAll() = mutable.update { state ->
        state.copy(selecting = true, selected = state.snapshot?.listing?.sections.orEmpty().flatMap {
            it.rows.filterIsInstance<Row.Task>().filter { row -> !row.row.archived }.map { row -> row.row.id }
        }.toSet())
    }
    fun selectGroup(ids: List<String>) = mutable.update { state ->
        val shown = state.snapshot?.listing?.sections.orEmpty().flatMap { it.rows }.filterIsInstance<Row.Task>()
            .filter { !it.row.archived }.map { it.row.id }.toSet()
        state.copy(selecting = true, selected = ids.toSet().intersect(shown))
    }
    fun cancelSelection() = mutable.update { it.copy(selecting = false, selected = emptySet()) }
    fun drop(ids: List<String>, destination: View) = action {
        if(isTaskIdList(ids.joinToString("\n"))) dropTasks(ids, destination) else Outcome(false, null, null, false)
    }
    fun reorder(ids: List<String>, before: String, destination: View) = action {
        if(before !in ids && isTaskIdList(ids.joinToString("\n"))) reorderTasks(ids, before, destination) else Outcome(false, null, null, false)
    }
    fun importText(text: String, destination: View = mutable.value.view, onSuccess: () -> Unit = {}) = action(onSuccess) { addFromText(text, destination) }
    fun openTextImport() {
        if(recoveryFull()) { fail(IllegalStateException(recoveryCapacityMessage())); return }
        val state = mutable.value
        if(state.busy || state.editor != null || state.editorLoading) return
        setEditor(EditorDraft(isTextImport = true), state.view)
    }
    fun importEditorText() {
        val state = mutable.value
        val editor = state.editor?.takeIf { it.isTextImport } ?: return
        if(state.busy || state.editorLoading) return
        if(editor.notes.length > EditorDraft.MAX_TEXT_IMPORT_CHARS) {
            mutable.update { it.copy(error = getApplication<Application>().getString(R.string.import_text_too_large)) }
            return
        }
        if(editor.notes.isBlank()) {
            mutable.update { it.copy(editorError = true) }
            return
        }
        val destination = editorView
        // Guard before launching so repeated taps/shortcuts cannot create duplicate batches.
        mutable.update { it.copy(busy = true, editorError = false) }
        persistJob?.cancel()
        viewModelScope.launch {
            try {
                persistDraft()
                val outcome = repository.mutate { addFromText(editor.notes, destination) }
                if(outcome.message is Message.SaveFailed) {
                    mutable.update { it.copy(error = (outcome.message as Message.SaveFailed).error) }
                } else if(outcome.changed) {
                    mutable.update { it.copy(busy = false) }
                    dismissEditor()
                    mutable.update { it.copy(notice = outcome, error = null) }
                    notifyChanges(); refresh()
                } else {
                    mutable.update { it.copy(error = getApplication<Application>().getString(R.string.import_no_changes)) }
                }
            } catch(e: Exception) { fail(e) }
            finally { mutable.update { it.copy(busy = false) } }
        }
    }
    fun quickAdd(text: String, onSuccess: () -> Unit): Job? {
        if(quickAddJob?.isActive == true) return null
        val destination = mutable.value.view
        return action(onSuccess) { addTask(text, destination) }.also { quickAddJob = it }
    }
    fun action(onSuccess: () -> Unit = {}, block: Engine.() -> Outcome) = viewModelScope.launch {
        if(mutable.value.restoring) return@launch
        try {
            val outcome = repository.mutate(block)
            if (outcome.message is Message.SaveFailed) mutable.update { it.copy(error = (outcome.message as Message.SaveFailed).error) }
            else {
                mutable.update { it.copy(notice = outcome, error = null, selecting = false, selected = emptySet()) }
                notifyChanges()
                refresh()
                onSuccess()
                if(outcome.isInteractiveCompletion()) playCurrentCompletionSound()
            }
        } catch (e: Exception) { fail(e) }
    }
    fun newTask(text: String = "") {
        if(recoveryFull()) { fail(IllegalStateException(recoveryCapacityMessage())); return }
        val view = mutable.value.view
        val draft = EditorDraft(title = text,
            projectId = (view as? View.Project)?.id ?: "INBOX_PROJECT",
            dueDay = if (view == View.Today || view == View.Morning || view == View.Tonight) java.time.LocalDate.now().toString() else "",
            tagIds = (view as? View.Tag)?.let { listOf(it.id) }.orEmpty())
        if (mutable.value.editor != null || mutable.value.editorLoading || mutable.value.restoring || mutable.value.busy || !captureUnlocked()) {
            enqueueCapture(draft, view); return
        }
        setEditor(draft, view)
    }
    fun edit(id: String) { openTask(id, unfinishedOnly = false) }
    fun openSystemSearchTask(id: String): Job? = openTask(id, unfinishedOnly = true)
    private fun openTask(id: String, unfinishedOnly: Boolean): Job? {
        if(id.length !in 1..128 || !id.matches(Regex("[A-Za-z0-9_-]+"))) return null
        if (mutable.value.editor != null || mutable.value.editorLoading || mutable.value.restoring || mutable.value.busy || !captureUnlocked() || recoveryFull()) {
            pendingOpen = id; pendingOpenSystemSearch = unfinishedOnly
            saved["pending-open"] = id; saved["pending-open-system-search"] = unfinishedOnly
            return null
        }
        // Claim the presentation slot before the first suspend, preserving later capture.
        mutable.update { it.copy(editorLoading = true) }
        return viewModelScope.launch {
            try {
                val detail = repository.read {
                    val row = taskRow(id) ?: return@read null
                    if(row.archived || (unfinishedOnly && row.isDone)) null else taskDetail(id)
                }
                if(detail == null || (unfinishedOnly && detail.isDone)) {
                    if(unfinishedOnly) fail(IllegalStateException(getApplication<Application>().getString(R.string.system_search_task_unavailable)))
                    return@launch
                }
                detail.let {
                    setEditor(EditorDraft(it.id, it.title, it.notes, it.projectId, it.dueDay.orEmpty(),
                        it.time?.let { time -> "%02d:%02d".format(time.hour.toInt(), time.minute.toInt()) }.orEmpty(),
                        if (it.estimateMs > 0) repository.read { formatEstimate(it.estimateMs) } else "",
                        it.reminderMinutesBefore?.toLong(), it.tagIds))
                }
            } catch(e: Exception) { fail(e) }
            finally { mutable.update { it.copy(editorLoading = false) }; if(mutable.value.editor == null) resumeCapture() }
        }
    }
    private fun admitEditor(draft: EditorDraft): Boolean {
        if(draftBudget.admit(draft)) return true
        mutable.update { current ->
            if(current.editor != null) current.copy(editorCapacityError = true)
            else current.copy(error = getApplication<Application>().getString(R.string.draft_open_capacity))
        }
        return false
    }
    fun setEditor(draft: EditorDraft, origin: View? = null): Boolean {
        if(draftId == null && recoveryFull()) { fail(IllegalStateException(recoveryCapacityMessage())); return false }
        // Refuse deterministic codec failures before publishing text or changing its owner.
        // The last accepted private draft and pending persistence job remain intact.
        if(!admitEditor(draft)) return false
        if (mutable.value.editor == null || origin != null) {
            editorView = origin ?: mutable.value.view
            saved["editor-view"] = viewKey(editorView)
            saved["editor-context"] = when(val route = editorView) { is View.Project -> route.id; is View.Tag -> route.id; else -> "" }
        }
        if(draftId == null) draftId = UUID.randomUUID().toString()
        saved["editor-id"] = draftId
        saved.remove<String>("editor")
        mutable.update { it.copy(editor = draft, editorError = false, editorUnavailable = false, editorCapacityError = false) }
        persistJob?.cancel()
        persistJob = viewModelScope.launch {
            delay(100)
            try { writeCurrentDraft() }
            catch(e: CancellationException) { throw e }
            catch(e: Exception) { fail(e) }
        }
        return true
    }
    suspend fun persistDraft() {
        // A manual/lifecycle flush owns publication after the debounced writer drains.
        // Join before taking draftLock: the cancelled writer may still hold it during I/O.
        val pending = withContext(Dispatchers.Main.immediate) {
            persistJob.also { persistJob = null }
        }
        pending?.cancelAndJoin()
        writeCurrentDraft()
    }
    private suspend fun writeCurrentDraft() = withContext(Dispatchers.IO) {
        draftLock.withLock {
            val id = draftId ?: return@withLock
            mutable.value.editor?.let { drafts.write(id, it) }
        }
    }
    private fun recoveryFull() = blockedInlineDraft || failedDrafts.size >= 32 || blockedDraftRecovery
    private fun recoveryCapacityMessage() = getApplication<Application>().getString(
        if(blockedInlineDraft) R.string.draft_open_capacity else R.string.draft_recovery_full)
    private fun retainFailedDraft(id: String, origin: View) {
        if(failedDrafts.none { it.id == id }) {
            if(failedDrafts.size < 32) failedDrafts += FailedDraft(id, origin)
            else blockedDraftRecovery = true // Keep the additional active SavedState reference intact.
        }
        draftId = null
        if(!blockedDraftRecovery) saved.remove<String>("editor-id")
        persistFailedDraftReferences()
    }
    private fun persistFailedDraftReferences() {
        saved["failed-editor-references"] = ArrayList(failedDrafts.flatMap { reference ->
            listOf(reference.id, viewKey(reference.origin), when(val view = reference.origin) {
                is View.Project -> view.id; is View.Tag -> view.id; else -> ""
            })
        })
        failedDrafts.firstOrNull()?.let { saved["failed-editor-id"] = it.id }
            ?: saved.remove<String>("failed-editor-id")
        mutable.update { it.copy(draftRecoveryPending = blockedInlineDraft || failedDrafts.isNotEmpty() || blockedDraftRecovery) }
    }
    fun retryDraftRecovery(): Job? {
        if(blockedInlineDraft) { fail(IllegalStateException(recoveryCapacityMessage())); return null }
        val state = mutable.value
        if(state.editor != null || state.editorLoading || state.busy || state.restoring || !captureUnlocked()) return null
        val reference = failedDrafts.firstOrNull() ?: return null
        mutable.update { it.copy(editorLoading = true) }
        return viewModelScope.launch {
            try {
                captureLock.withLock {
                    val (draft, queued, count) = withContext(Dispatchers.IO) {
                        val draft = requireNotNull(drafts.read(reference.id)) { getApplication<Application>().getString(R.string.draft_recovery_unavailable) }
                        val queued = captures.contains(reference.id)
                        check(!queued || captures.head()?.id == reference.id) { getApplication<Application>().getString(R.string.draft_recovery_wait) }
                        Triple(draft, queued, captures.size())
                    }
                    val current = mutable.value
                    if(current.editor != null || current.busy || current.restoring || !captureUnlocked()) return@withLock
                    // A legacy version-one record may fit its old header but not version two.
                    // Keep its recovery identity and bytes until a recoverable editor can own it.
                    if(!admitEditor(draft)) return@withLock
                    failedDrafts.removeAll { it.id == reference.id }
                    if(blockedDraftRecovery) {
                        saved.get<String>("editor-id")?.let { failedDrafts += FailedDraft(it, editorView) }
                        blockedDraftRecovery = false
                    }
                    draftId = reference.id
                    activeCaptureId = reference.id.takeIf { queued }
                    capturesLoaded = true
                    setEditor(draft, reference.origin)
                    persistFailedDraftReferences()
                    captureCount(count)
                    mutable.update { it.copy(error = null) }
                }
            } catch(e: Exception) { fail(e) }
            finally {
                mutable.update { it.copy(editorLoading = false) }
                if(mutable.value.editor == null) resumeCapture()
            }
        }
    }
    fun dismissEditor() {
        if(mutable.value.busy) return
        persistJob?.cancel()
        val removed = draftId
        draftId = null; saved.remove<String>("editor-id")
        // CaptureQueue.complete first commits its index, then deletes the owned payload.
        // A crash or completion failure must never leave a queue reference to a deleted draft.
        if(removed != null && capturesLoaded && removed != activeCaptureId) viewModelScope.launch {
            withContext(Dispatchers.IO) { draftLock.withLock { drafts.delete(removed) } }
        }
        if(removed != null && removed == activeCaptureId) {
            // Every queued drain sees the dismissal before it can read the old head.
            pendingCaptureCompletion = removed
            activeCaptureId = null
        }
        saved.remove<String>("editor")
        saved.remove<String>("editor-view"); saved.remove<String>("editor-context")
        mutable.update { it.copy(editor = null, editorError = false, editorCapacityError = false) }
        resumeCapture()
    }
    private fun resumeCapture() {
        viewModelScope.launch {
            try {
                captureLock.withLock {
                    pendingCaptureCompletion?.let { id ->
                        withContext(Dispatchers.IO) { captures.complete(id) }
                        if(pendingCaptureCompletion == id) pendingCaptureCompletion = null
                        captureCount(withContext(Dispatchers.IO) { captures.size() })
                    }
                    fun idle() = capturesLoaded && !recoveryFull() && mutable.value.editor == null && !mutable.value.editorLoading &&
                        !mutable.value.restoring && !mutable.value.busy && captureUnlocked()
                    if(!idle()) return@withLock
                    val hasCaptures = withContext(Dispatchers.IO) { captures.size() > 0 }
                    if(!idle()) return@withLock
                    if(!hasCaptures) {
                        pendingOpen?.let {
                            val unfinishedOnly = pendingOpenSystemSearch
                            pendingOpen = null; pendingOpenSystemSearch = false
                            saved.remove<String>("pending-open"); saved.remove<Boolean>("pending-open-system-search")
                            openTask(it, unfinishedOnly)
                        }
                        return@withLock
                    }
                    // Reserve presentation while reading a potentially large payload off the main thread.
                    mutable.update { it.copy(editorLoading = true) }
                    try {
                        val capture = withContext(Dispatchers.IO) { captures.head() }
                        if(capture != null && mutable.value.editor == null && !mutable.value.restoring &&
                            !mutable.value.busy && captureUnlocked()) {
                            if(!admitEditor(capture.draft)) return@withLock
                            draftId = capture.id; activeCaptureId = capture.id
                            setEditor(capture.draft, capture.origin)
                            captureCount(withContext(Dispatchers.IO) { captures.size() })
                        }
                    } finally { mutable.update { it.copy(editorLoading = false) } }
                }
            } catch(e: Exception) { fail(e) }
        }
    }
    fun resumeQueuedCaptures() = resumeCapture()
    private fun captureCount(total: Int) = mutable.update { it.copy(queuedCaptures = (total - if(activeCaptureId != null) 1 else 0).coerceAtLeast(0)) }
    private fun captureUnlocked(): Boolean = getApplication<Application>().getSystemService(KeyguardManager::class.java)?.isDeviceLocked != true
    private fun enqueueCapture(draft: EditorDraft, origin: View) = viewModelScope.launch {
        try {
            captureLock.withLock { withContext(Dispatchers.IO) { captures.enqueue(draft, origin) }; captureCount(withContext(Dispatchers.IO) { captures.size() }) }
            resumeCapture()
        } catch(_: CaptureQueue.Full) {
            mutable.update { it.copy(error = getApplication<Application>().getString(R.string.capture_queue_full)) }
        } catch(e: Exception) { fail(e) }
    }
    fun saveEditor() = commitEditor(true) {}
    fun saveAsNew() {
        val draft = mutable.value.editor ?: return
        if(setEditor(draft.copy(id = null, projectId = draft.projectId.takeIf { id -> mutable.value.snapshot?.projects.orEmpty().any { it.id == id } } ?: "INBOX_PROJECT"), View.Project("INBOX_PROJECT"))) saveEditor()
    }
    fun saveEditorThen(onSaved: (String) -> Unit) = commitEditor(false, onSaved)
    private fun commitEditor(close: Boolean, onSaved: (String) -> Unit) = viewModelScope.launch {
        val editor = mutable.value.editor ?: return@launch
        if (mutable.value.busy) return@launch
        mutable.update { it.copy(busy = true) }
        try {
            val draft = repository.draft(editor)
            if (draft == null) mutable.update { it.copy(editorError = true) }
            else {
                val creation = if(editor.id == null) repository.create(draft, editorView) else null
                val outcome = creation?.outcome ?: repository.mutate { saveTask(editor.id!!, draft) }
                val id = editor.id ?: creation?.id
                if(editor.id != null && !outcome.changed && repository.read { taskRow(editor.id)?.archived != false }) {
                    mutable.update { it.copy(editorUnavailable = true, error = getApplication<Application>().getString(R.string.task_unavailable)) }
                }
                else if (outcome.message is Message.SaveFailed) mutable.update { it.copy(error = (outcome.message as Message.SaveFailed).error) }
                else if(id != null) {
                    mutable.update { it.copy(busy = false) }
                    if(close) dismissEditor() else setEditor(editor.copy(id = id))
                    mutable.update { it.copy(notice = outcome, error = null) }; notifyChanges(); refresh(); onSaved(id)
                }
            }
        } catch(e: Exception) { fail(e) }
        finally { mutable.update { it.copy(busy = false) } }
    }
    fun preferences(change: (com.codedbydan.momentum.core.Preferences) -> com.codedbydan.momentum.core.Preferences) = viewModelScope.launch {
        try { repository.updatePreferences(change); notifyChanges(); refresh() } catch(e: Exception) { fail(e) }
    }
    fun settings(change: (AppSettings) -> AppSettings): Job {
        // Invalidate pending audio synchronously, before the DataStore write suspends.
        stopCompletionSound()
        return viewModelScope.launch {
            try { repository.updateSettings(change); notifyChanges(); refresh() } catch(e: Exception) { fail(e) }
        }.also { soundSettingsJob = it }
    }
    fun previewCompletionSound() = viewModelScope.launch { playCurrentCompletionSound() }
    fun stopCompletionSound() { ++soundGeneration; soundPlayer.cancelPending() }
    fun completionSoundForeground(visible: Boolean) {
        soundForeground = visible
        if(!visible) stopCompletionSound()
    }
    private suspend fun playCurrentCompletionSound() {
        try {
            if(!soundForeground) return
            val generation = soundGeneration
            soundSettingsJob?.join()
            val settings = repository.settings()
            if(soundForeground && generation == soundGeneration && settings.completionSound) soundPlayer.play(settings.completionSoundChoice)
        } catch(e: Exception) {
            if(e is CancellationException) throw e
            // Optional feedback must not turn a successfully persisted task into a failed action.
        }
    }
    fun organization(project: Boolean, id: String?, title: String, color: String?, onEntity: (String) -> Unit = {}, onSuccess: () -> Unit = {}) = viewModelScope.launch {
        try {
            val (entity, result) = repository.read {
                val existing = if(project) emptySet() else tags().map { it.id }.toSet()
                val entity = id ?: if (project) addProject(title) else addTag(title)
                requireNotNull(entity) { getApplication<Application>().getString(R.string.organization_failed) }
                entity to if(id == null && entity in existing) Outcome(false, null, null, false)
                    else if(project) updateProject(entity, title, color) else updateTag(entity, title, color)
            }
            onEntity(entity)
            if (result.message is Message.SaveFailed) mutable.update { it.copy(error = result.message.error) }
            else { mutable.update { it.copy(error = null) }; onSuccess() }
            notifyChanges(); refresh()
        } catch(e: Exception) { fail(e) }
    }
    fun export(uri: Uri) = viewModelScope.launch {
        try {
            withContext(Dispatchers.IO) {
                val file = File(getApplication<Application>().cacheDir, "export-${UUID.randomUUID()}.json")
                try {
                    repository.read { exportBackup(file.absolutePath) }
                    getApplication<Application>().contentResolver.openOutputStream(uri, "wt").use { output ->
                        requireNotNull(output)
                        file.inputStream().use { it.copyTo(output) }
                    }
                } finally { file.delete() }
            }
            mutable.update { it.copy(notice = Outcome(false, Message.BackupExported, null, false)) }
        } catch(e: Exception) { fail(e) }
    }
    fun prepareImport(uri: Uri): Job {
        if(mutable.value.restoring) return viewModelScope.launch { }
        val generation = ++importGeneration
        importPreparation?.cancel()
        importFile?.delete(); importFile = null
        mutable.update { it.copy(importReady = false, importPreparing = true) }
        return viewModelScope.launch {
            val file = File(getApplication<Application>().cacheDir, "import-${UUID.randomUUID()}.json")
            var published = false
            try {
                withContext(Dispatchers.IO) {
                    openImportSource(uri).use { input ->
                        requireNotNull(input)
                        file.outputStream().use { output ->
                            val buffer = ByteArray(8192); var total = 0L
                            while(true) { ensureActive(); val count = input.read(buffer); if(count < 0) break
                                total += count; require(total <= 64L * 1024 * 1024); output.write(buffer, 0, count) }
                        }
                    }
                }
                if(generation != importGeneration || mutable.value.restoring) return@launch
                importFile = file; published = true
                mutable.update { it.copy(importReady = true) }
            } catch(e: Exception) {
                if(e is CancellationException) throw e
                if(generation == importGeneration) fail(e)
            } finally {
                if(!published) withContext(NonCancellable + Dispatchers.IO) { file.delete() }
                if(generation == importGeneration) mutable.update { it.copy(importPreparing = false) }
            }
        }.also { importPreparation = it }
    }
    fun cancelImport() {
        if(mutable.value.restoring) return
        ++importGeneration; importPreparation?.cancel(); importPreparation = null
        importFile?.delete(); importFile = null
        mutable.update { it.copy(importReady = false, importPreparing = false) }
    }
    fun restore() {
        val file = importFile ?: return
        if(mutable.value.editor != null || mutable.value.editorLoading || mutable.value.busy) return
        mutable.update { it.copy(busy = true, restoring = true) }
        viewModelScope.launch {
            try {
                pauseForRestore()
                val outcome = repository.read { importBackup(file.absolutePath) }
                if(importFile === file) { file.delete(); importFile = null }
                mutable.update { it.copy(notice = outcome, error = null, importReady = false) }
                refresh()
            } catch(e: Exception) { fail(e) }
            finally {
                // The restore owns provider admission through native completion, including
                // failure/cancellation. An older edit's changed() cannot release it early.
                withContext(NonCancellable) {
                    try { resumeAfterRestore() } catch(e: Exception) { fail(e) }
                    finally {
                        mutable.update { it.copy(busy = false, restoring = false) }
                        notifyChanges(); if(mutable.value.editor == null) resumeCapture()
                    }
                }
            }
        }
    }
    fun reportError(e: Exception) = fail(e)
    private fun fail(e: Exception) {
        if (e is CancellationException) throw e
        mutable.update { it.copy(error = e.message ?: "") }
    }
    fun captureTask(title: String, notes: String, due: String, tags: String) {
        viewModelScope.launch {
            try {
                // Parsing and enqueue share one serial section so rapid shares keep their arrival order.
                captureLock.withLock {
                    withContext(Dispatchers.IO) {
                        require(title.length.toLong() + notes.length + due.length + tags.length <= 1_000_200) {
                            getApplication<Application>().getString(R.string.capture_too_large)
                        }
                        captures.enqueue(capturedDraft(title, notes, due, tags), View.Project("INBOX_PROJECT"))
                    }
                    captureCount(withContext(Dispatchers.IO) { captures.size() })
                }
                resumeCapture()
            } catch(_: CaptureQueue.Full) {
                mutable.update { it.copy(error = getApplication<Application>().getString(R.string.capture_queue_full)) }
            } catch(e: Exception) { fail(e) }
        }
    }
    fun captureText(text: String) {
        val firstLine = text.lineSequence().firstOrNull().orEmpty()
        // A manageable preview may be shorter; the full shared source remains editable.
        captureTask(firstLine.take(200), text.takeIf { it.contains('\n') || it.startsWith("http") || firstLine.length > 200 }.orEmpty(), "", "")
    }
    override fun onCleared() { ++importGeneration; importPreparation?.cancel(); importFile?.delete(); soundPlayer.close() }
}

private fun viewKey(view: View): String = when(view) {
    View.Today -> "today"; View.Morning -> "morning"; View.Tonight -> "evening"
    View.Upcoming -> "upcoming"; View.Archive -> "archive"; View.Search -> "search"
    is View.Project -> "project"; is View.Tag -> "tag"
}
private fun restoreView(key: String?, id: String?): View = when(key) {
    "morning" -> View.Morning; "evening" -> View.Tonight; "upcoming" -> View.Upcoming
    "archive" -> View.Archive; "search" -> View.Search; "project" -> id?.let { View.Project(it) } ?: View.Today
    "tag" -> id?.let { View.Tag(it) } ?: View.Today; else -> View.Today
}
