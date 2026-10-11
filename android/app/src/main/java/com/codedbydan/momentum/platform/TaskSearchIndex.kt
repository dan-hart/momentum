// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.platform

import android.app.appsearch.*
import android.content.Context
import android.net.Uri
import android.os.Build
import android.util.AtomicFile
import androidx.annotation.RequiresApi
import com.codedbydan.momentum.data.MomentumRepository
import kotlinx.coroutines.*
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import org.json.JSONObject
import java.io.File
import java.io.ByteArrayOutputStream
import java.util.concurrent.Executor
import java.util.function.Consumer
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException
import kotlin.coroutines.suspendCoroutine

data class TaskSearchState(
    val updating: Boolean = false,
    val error: String? = null,
    /** Count at the last completely successful reconciliation, not a delivery assertion. */
    val indexed: Int = 0,
    val updatedAt: Long = 0,
    val cleared: Boolean = false,
)

/** A derived title-only projection. It never edits tasks or grants access to another package. */
internal class TaskSearchIndex(
    context: Context,
    private val repository: MomentumRepository,
    private val databaseName: String = DATABASE,
    private val displayedBySystem: Boolean = true,
) {
    private val context = context.applicationContext
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val requests = Channel<Unit>(Channel.CONFLATED)
    private val lock = Mutex()
    private val mutable = MutableStateFlow(TaskSearchState())
    val state: StateFlow<TaskSearchState> = mutable.asStateFlow()
    private val statusFile = AtomicFile(File(repository.directory, "android-system-search-state.json"))
    @Volatile private var paused = false
    @Volatile private var suppressed = false
    private var loaded = false

    init {
        val isolated = repository.directory.canonicalFile != File(this.context.noBackupFilesDir, "tasks").canonicalFile
        require(!isolated || (!displayedBySystem && databaseName.startsWith("momentum_test_"))) {
            "Disposable stores require a separate search database hidden from system UI"
        }
        scope.launch {
            for(ignored in requests) {
                // Coalesce edits without cancelling a submitted native mutation.
                delay(350)
                while(requests.tryReceive().isSuccess) Unit
                reconcile()
            }
        }
    }

    fun requestReconcile() { requests.trySend(Unit) }
    fun resume() { paused = false; requestReconcile() }

    suspend fun setEnabled(enabled: Boolean): Boolean {
        // Opt-out closes admission immediately, even if preferences cannot be saved.
        suppressed = !enabled
        return serialized {
            try {
                repository.updateSettings { it.copy(systemSearch = enabled) }
                reconcileLocked()
            } catch(error: Exception) {
                if(!enabled) runCatching { clearLocked() }
                failure(error); false
            }
        }
    }

    suspend fun reconcile(): Boolean = serialized { reconcileLocked() }

    suspend fun pauseAndClear(): Boolean {
        paused = true // Stops admission before waiting for an in-flight native callback.
        return serialized {
            try { clearLocked(); true } catch(error: Exception) { failure(error); false }
        }
    }

    suspend fun close() {
        scope.coroutineContext[Job]?.cancelAndJoin()
        requests.close()
        serialized { Unit }
    }

    private suspend fun <T> serialized(block: suspend () -> T): T = withContext(NonCancellable + Dispatchers.IO) {
        lock.withLock { loadStatus(); block() }
    }

    private suspend fun reconcileLocked(): Boolean {
        try {
            if(paused || suppressed || !repository.settings().systemSearch) { clearLocked(); return true }
            if(Build.VERSION.SDK_INT >= 31) {
                reconcileNativeLocked()
            } else {
                error("System task search requires Android 12 or later")
            }
            return true
        } catch(error: Exception) { failure(error); return false }
    }

    @RequiresApi(31)
    private suspend fun reconcileNativeLocked() {
        mutable.update { it.copy(updating = true) }
        // Rust supplies live tasks. Only unfinished titles leave the private store.
        val desired = repository.read { allTasks().filterNot { it.isDone }.associate { it.id to it.title } }
        withSession { session ->
            // A partial or failed update must not publish stale completed/deleted titles.
            setSchema(session, displayed = false)
            val existing = documents(session).associateBy { it.id }
            val stale = existing.keys - desired.keys
            for(ids in stale.chunked(BATCH_SIZE)) remove(session, ids)
            for(entries in desired.entries.filter { (id, title) ->
                val old = existing[id]
                old?.getPropertyString("name") != title || old?.getPropertyString("url") != taskUrl(id)
            }.chunked(BATCH_SIZE)) {
                if(paused || suppressed || !repository.settings().systemSearch) break
                put(session, entries.map { (id, title) -> TitleDocumentBuilder(id)
                    .setPropertyString("name", title).setPropertyString("url", taskUrl(id)).build() })
            }
            if(!paused && !suppressed && repository.settings().systemSearch) setSchema(session, displayedBySystem)
        }
        // A setting change or restore during a native put must clear its late result.
        if(paused || suppressed || !repository.settings().systemSearch) clearLocked()
        else success(desired.size, cleared = false)
    }

    private suspend fun clearLocked() {
        if(Build.VERSION.SDK_INT < 31) { success(0, cleared = true); return }
        mutable.update { it.copy(updating = true) }
        withSession { session ->
            // Hide first: deletion failure must never leave titles deliberately visible.
            setSchema(session, displayed = false)
            result<Void> { callback -> session.remove("", SearchSpec.Builder()
                .setTermMatch(SearchSpec.TERM_MATCH_PREFIX).addFilterNamespaces(NAMESPACE).build(), EXECUTOR, callback) }
        }
        success(0, cleared = true)
    }

    @RequiresApi(31)
    private suspend fun <T> withSession(block: suspend (AppSearchSession) -> T): T {
        val manager = context.getSystemService(AppSearchManager::class.java)
            ?: error("The Android system search service is unavailable")
        val session = result<AppSearchSession> { callback ->
            manager.createSearchSession(AppSearchManager.SearchContext.Builder(databaseName).build(), EXECUTOR, callback)
        } ?: error("The Android system search service returned no session")
        return session.use { block(it) }
    }

    @RequiresApi(31)
    private suspend fun setSchema(session: AppSearchSession, displayed: Boolean) {
        val schema = AppSearchSchema.Builder(SCHEMA)
            .addProperty(AppSearchSchema.StringPropertyConfig.Builder("name")
                .setCardinality(AppSearchSchema.PropertyConfig.CARDINALITY_REQUIRED)
                .setIndexingType(AppSearchSchema.StringPropertyConfig.INDEXING_TYPE_PREFIXES)
                .setTokenizerType(AppSearchSchema.StringPropertyConfig.TOKENIZER_TYPE_PLAIN).build())
            .addProperty(AppSearchSchema.StringPropertyConfig.Builder("url")
                .setCardinality(AppSearchSchema.PropertyConfig.CARDINALITY_REQUIRED)
                .setIndexingType(AppSearchSchema.StringPropertyConfig.INDEXING_TYPE_NONE)
                .setTokenizerType(AppSearchSchema.StringPropertyConfig.TOKENIZER_TYPE_NONE).build()).build()
        result<SetSchemaResponse> { callback -> session.setSchema(SetSchemaRequest.Builder().addSchemas(schema)
            .setVersion(1).setSchemaTypeDisplayedBySystem(SCHEMA, displayed).build(), EXECUTOR, EXECUTOR, callback) }
    }

    @RequiresApi(31)
    private suspend fun documents(session: AppSearchSession): List<GenericDocument> {
        return session.search("", SearchSpec.Builder().setTermMatch(SearchSpec.TERM_MATCH_PREFIX)
            .addFilterNamespaces(NAMESPACE).addFilterSchemas(SCHEMA).setResultCountPerPage(100).build()).use { results ->
            val documents = mutableListOf<GenericDocument>()
            while(true) {
                val page = result<List<SearchResult>> { results.getNextPage(EXECUTOR, it) }.orEmpty()
                if(page.isEmpty()) break
                documents += page.map { it.genericDocument }
            }
            documents
        }
    }

    @RequiresApi(31)
    private suspend fun put(session: AppSearchSession, documents: List<GenericDocument>) {
        batch<Void> { callback -> session.put(PutDocumentsRequest.Builder().addGenericDocuments(documents).build(), EXECUTOR, callback) }
    }

    @RequiresApi(31)
    private suspend fun remove(session: AppSearchSession, ids: List<String>) {
        batch<Void> { callback -> session.remove(RemoveByDocumentIdRequest.Builder(NAMESPACE).addIds(ids).build(), EXECUTOR, callback) }
    }

    /** Submitted binder operations are drained; cancellation cannot reorder a late put after opt-out. */
    @RequiresApi(31)
    private suspend fun <T> result(start: (Consumer<AppSearchResult<T>>) -> Unit): T? = suspendCoroutine { continuation ->
        start(Consumer { value ->
            if(value.isSuccess) continuation.resume(value.resultValue)
            else continuation.resumeWithException(IllegalStateException("Android search operation failed (code ${value.resultCode})"))
        })
    }

    @RequiresApi(31)
    private suspend fun <T> batch(start: (BatchResultCallback<String, T>) -> Unit): Unit = suspendCoroutine { continuation ->
        start(object : BatchResultCallback<String, T> {
            override fun onResult(value: AppSearchBatchResult<String, T>) {
                val failure = value.failures.values.firstOrNull()
                if(failure == null) continuation.resume(Unit)
                else continuation.resumeWithException(IllegalStateException("Android search operation failed (code ${failure.resultCode})"))
            }
            override fun onSystemError(error: Throwable?) {
                continuation.resumeWithException(IllegalStateException("The Android system search service failed", error))
            }
        })
    }

    private fun loadStatus() {
        if(loaded) return
        loaded = true
        if(!statusFile.baseFile.exists() && !File(statusFile.baseFile.path + ".bak").exists()) return
        try {
            val bytes = statusFile.openRead().use { input ->
                val output = ByteArrayOutputStream(); val buffer = ByteArray(8192)
                while(true) {
                    val count = input.read(buffer); if(count < 0) break
                    require(output.size() + count <= 65_536) { "The saved search status is too large" }
                    output.write(buffer, 0, count)
                }
                output.toByteArray()
            }
            val json = JSONObject(bytes.toString(Charsets.UTF_8)); require(json.getInt("version") == 1)
            mutable.value = TaskSearchState(error = if(json.isNull("error")) null else json.getString("error"),
                indexed = json.getInt("indexed").also { require(it >= 0) },
                updatedAt = json.getLong("updated-at").also { require(it >= 0) }, cleared = json.getBoolean("cleared"))
        } catch(error: Exception) { failure(error) }
    }

    private fun success(count: Int, cleared: Boolean) {
        val next = TaskSearchState(indexed = count, updatedAt = System.currentTimeMillis(), cleared = cleared)
        saveStatus(next)
        mutable.value = next
    }

    private fun failure(error: Exception) {
        val next = mutable.value.copy(updating = false, error = error.message ?: "System search could not be updated")
        mutable.value = next
        runCatching { saveStatus(next) }
    }

    private fun saveStatus(state: TaskSearchState) {
        val bytes = JSONObject().put("version", 1).put("error", state.error ?: JSONObject.NULL)
            .put("indexed", state.indexed).put("updated-at", state.updatedAt).put("cleared", state.cleared)
            .toString().toByteArray(Charsets.UTF_8)
        check(statusFile.baseFile.parentFile!!.exists() || statusFile.baseFile.parentFile!!.mkdirs())
        val output = statusFile.startWrite()
        try { output.write(bytes); statusFile.finishWrite(output) }
        catch(error: Exception) { statusFile.failWrite(output); throw error }
    }

    @RequiresApi(31)
    private class TitleDocumentBuilder(id: String) : GenericDocument.Builder<TitleDocumentBuilder>(NAMESPACE, id, SCHEMA)

    companion object {
        const val DATABASE = "momentum_system_task_search"
        const val NAMESPACE = "unfinished_task_titles"
        const val SCHEMA = "com.codedbydan.momentum.TaskTitle"
        private const val BATCH_SIZE = 64
        private val EXECUTOR = Executor { it.run() }
        fun taskUrl(id: String): String = Uri.Builder().scheme("momentum").authority("system-search")
            .appendQueryParameter("id", id).build().toString()
    }
}
