// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.platform

import android.app.Application
import android.app.appsearch.*
import androidx.test.core.app.ApplicationProvider
import androidx.test.filters.SdkSuppress
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.MomentumRepository
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import java.io.File
import java.util.UUID
import java.util.concurrent.Executor
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException
import kotlin.coroutines.suspendCoroutine

/** Real framework AppSearch, with private disposable databases never visible to system UI. */
@SdkSuppress(minSdkVersion = 31)
class TaskSearchIndexTest {
    private val executor = Executor { it.run() }

    @Test fun unfinishedTitlesRenameCompleteDeleteAndOptOutReconcileInTheNativeIndex() = runBlocking {
        fixture { repository, index, session ->
            val first = repository.create(draft("Native search alpha", "Never index private notes"), View.Today).id!!
            val second = repository.create(draft("Native search beta"), View.Today).id!!
            assertTrue(index.reconcile())
            assertTrue(documents(session).isEmpty())
            assertTrue(index.setEnabled(true))
            val initial = documents(session)
            assertEquals(setOf(first, second), initial.map { it.id }.toSet())
            assertEquals("Native search alpha", initial.single { it.id == first }.getPropertyString("name"))
            assertEquals(setOf("name", "url"), initial.single { it.id == first }.propertyNames)
            assertEquals(first, android.net.Uri.parse(initial.single { it.id == first }.getPropertyString("url")).getQueryParameter("id"))
            repository.mutate { saveTask(first, draft("Renamed native title")) }
            repository.mutate { setDone(second, true) }
            assertTrue(index.reconcile())
            assertEquals(listOf("Renamed native title"), documents(session).map { it.getPropertyString("name") })
            repository.mutate { deleteTask(first) }
            assertTrue(index.reconcile())
            assertTrue(documents(session).isEmpty())
            repository.create(draft("Final unfinished fixture"), View.Today)
            assertTrue(index.reconcile())
            assertEquals(1, documents(session).size)
            assertTrue(index.setEnabled(false))
            assertTrue(documents(session).isEmpty())
            assertTrue(index.state.value.cleared)
            assertNull(index.state.value.error)
            val schema = result<GetSchemaResponse> { session.getSchema(executor, it) }!!
            if(android.os.Build.VERSION.SDK_INT >= 33) assertTrue(schema.schemaTypesNotDisplayedBySystem.contains(TaskSearchIndex.SCHEMA))
        }
    }

    @Test fun restorePauseClearsAndDefersReindexingUntilExplicitResume() = runBlocking {
        fixture { repository, index, session ->
            repository.create(draft("Before restore"), View.Today)
            assertTrue(index.setEnabled(true))
            assertEquals(1, documents(session).size)
            assertTrue(index.pauseAndClear())
            repository.create(draft("During restore"), View.Today)
            assertTrue(index.reconcile())
            assertTrue(documents(session).isEmpty())
            index.resume()
            assertTrue(index.reconcile())
            assertEquals(2, documents(session).size)
        }
    }

    @Test fun failedStatusPersistenceRetainsLastSuccessUntilAnExplicitRetryCompletes() = runBlocking {
        fixture { repository, index, session ->
            repository.create(draft("Status failure fixture"), View.Today)
            assertTrue(index.setEnabled(true))
            val successful = index.state.value
            val blocker = File(repository.directory, "android-system-search-state.json.new")
            assertTrue(blocker.mkdir())
            try {
                assertFalse(index.setEnabled(false))
                assertFalse(repository.settings().systemSearch)
                assertTrue(documents(session).isEmpty())
                assertNotNull(index.state.value.error)
                assertEquals(successful.indexed, index.state.value.indexed)
                assertEquals(successful.updatedAt, index.state.value.updatedAt)
                assertFalse(index.state.value.cleared)
            } finally { assertTrue(blocker.delete()) }
            assertTrue(index.reconcile())
            assertTrue(index.state.value.cleared)
            assertEquals(0, index.state.value.indexed)
            assertNull(index.state.value.error)
        }
    }

    private suspend fun fixture(block: suspend (MomentumRepository, TaskSearchIndex, AppSearchSession) -> Unit) {
        val application = ApplicationProvider.getApplicationContext<Application>()
        val identity = UUID.randomUUID().toString()
        val directory = File(application.cacheDir, "search-test-$identity").also { it.mkdirs() }
        val repository = MomentumRepository(application, directory)
        val database = "momentum_test_$identity"
        val index = TaskSearchIndex(application, repository, database, displayedBySystem = false)
        val manager = application.getSystemService(AppSearchManager::class.java)
        val session = result<AppSearchSession> {
            manager.createSearchSession(AppSearchManager.SearchContext.Builder(database).build(), executor, it)
        }!!
        try { withTimeout(30_000) { block(repository, index, session) } }
        finally { index.setEnabled(false); index.close(); session.close(); repository.close(); directory.deleteRecursively() }
    }

    private suspend fun documents(session: AppSearchSession): List<GenericDocument> {
        val results = session.search("", SearchSpec.Builder().setTermMatch(SearchSpec.TERM_MATCH_PREFIX)
            .addFilterNamespaces(TaskSearchIndex.NAMESPACE).setResultCountPerPage(100).build())
        return results.use {
            val documents = mutableListOf<GenericDocument>()
            while(true) {
                val page = result<List<SearchResult>> { callback -> results.getNextPage(executor, callback) }!!
                if(page.isEmpty()) break
                documents += page.map { it.genericDocument }
            }
            documents
        }
    }

    private suspend fun <T> result(start: (java.util.function.Consumer<AppSearchResult<T>>) -> Unit): T? = suspendCoroutine { continuation ->
        start(java.util.function.Consumer { value ->
            if(value.isSuccess) continuation.resume(value.resultValue)
            else continuation.resumeWithException(IllegalStateException("Fixture AppSearch failed: ${value.resultCode}"))
        })
    }

    private fun draft(title: String, notes: String = "") = TaskDraft(title, "INBOX_PROJECT", null, null, null,
        0.0, notes, emptyList(), emptyList())
}
