// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import android.content.Intent
import android.net.Uri
import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.ViewModelStore
import androidx.test.core.app.ApplicationProvider
import androidx.test.platform.app.InstrumentationRegistry
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.*
import com.codedbydan.momentum.platform.TaskEntryPoints
import kotlinx.coroutines.*
import org.junit.*
import java.io.File
import java.util.UUID

/** Shipping Activity adapter + real Intents, isolated core/drafts; not launcher/OS-lock acceptance. */
class TaskEntryPointsTest {
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()
    private lateinit var repository: MomentumRepository
    private lateinit var directory: File
    private lateinit var vm: MomentumViewModel
    private lateinit var saved: SavedStateHandle
    private var owners = ViewModelStore()
    private fun main(block: () -> Unit) = InstrumentationRegistry.getInstrumentation().runOnMainSync(block)
    private fun await(block: () -> Boolean) = runBlocking { withTimeout(15_000) { while(!block()) delay(20) } }
    private fun start(values: Map<String, Any?> = emptyMap()) {
        main {
            saved = SavedStateHandle(values)
            vm = MomentumViewModel(app, saved, repository, {}, {})
            owners.put("entries", vm)
        }
        await { !vm.state.value.loading && !vm.state.value.editorLoading }
    }
    @Before fun prepare() {
        directory = File(app.cacheDir, "task-entries-${UUID.randomUUID()}")
        repository = MomentumRepository(app, directory)
        start()
    }
    @After fun cleanup() = runBlocking {
        main { owners.clear() }
        repository.close(); directory.deleteRecursively(); Unit
    }
    private fun uri(host: String, vararg parameters: Pair<String, String>) = Intent(Intent.ACTION_VIEW,
        Uri.Builder().scheme("momentum").authority(host).apply {
            parameters.forEach { appendQueryParameter(it.first, it.second) }
        }.build())
    private fun capture(intent: Intent, locked: Boolean = false) = main { TaskEntryPoints(app, vm) { locked }.capture(intent) }
    private suspend fun create(title: String) = repository.create(TaskDraft(title, "INBOX_PROJECT", null, null, null,
        0.0, "", emptyList(), emptyList()), View.Today).id!!

    @Test fun warmDraftCapturePreservesEditorAndColdReconstructionKeepsFullParameters() = runBlocking {
        main { vm.newTask("Existing editor") }
        val notes = "Private source 📝\nSecond line\n".repeat(300)
        capture(uri("create-task", "title" to "Review #shared", "notes" to notes,
            "due" to "2030-12-14", "tags" to "native,shared"))
        await { vm.state.value.queuedCaptures == 1 }
        Assert.assertEquals("Existing editor", vm.state.value.editor!!.title)
        Assert.assertTrue(repository.read { allTasks() }.isEmpty())
        main { vm.dismissEditor() }
        await { vm.state.value.editor?.title == "Review" }
        vm.persistDraft()
        val values = saved.keys().associateWith { saved.get<Any>(it) }
        main { owners.clear(); owners = ViewModelStore() }
        start(values)
        Assert.assertEquals(notes, vm.state.value.editor!!.notes)
        Assert.assertEquals("2030-12-14", vm.state.value.editor!!.dueDay)
        Assert.assertEquals(notes, DraftStore(directory).read(saved.get<String>("editor-id")!!)!!.notes)
        main { vm.saveEditor() }
        await { vm.state.value.editor == null && !vm.state.value.busy }
        val task = repository.read { allTasks().single() }
        Assert.assertEquals(notes, repository.detail(task.id)!!.notes)
        Assert.assertEquals("2030-12-14", repository.detail(task.id)!!.dueDay)
        Assert.assertEquals(setOf("native", "shared"), repository.read { tags().map { it.title }.toSet() })
    }

    @Test fun reopenUrlIsIdempotentAndPreservesRealUndoForTheFirstReopen() = runBlocking {
        val id = create("Completed entry fixture")
        repository.mutate { setDone(id, true) }
        capture(uri("reopen-task", "id" to id))
        await { vm.state.value.notice?.changed == true && !vm.state.value.loading }
        Assert.assertFalse(repository.detail(id)!!.isDone)
        Assert.assertTrue(repository.read { canUndo() })
        capture(uri("reopen-task", "id" to id))
        await { vm.state.value.notice?.changed == false }
        Assert.assertFalse(repository.detail(id)!!.isDone)
        Assert.assertTrue(repository.mutate { undo() }.changed)
        Assert.assertTrue(repository.detail(id)!!.isDone)
        capture(uri("reopen-task", "id" to "missing-entry-id"))
        await { vm.state.value.notice?.changed == false && !vm.state.value.loading }
        Assert.assertTrue(repository.detail(id)!!.isDone)
    }

    @Test fun findAndTodayNavigateWithoutReplacingTheActiveDraftOrItsProject() = runBlocking {
        val project = repository.read { addProject("Entry project")!! }
        create("Find native entry")
        main { vm.navigate(View.Project(project)) }
        await { vm.state.value.snapshot != null }
        main { vm.newTask("Keep project draft") }
        capture(uri("find-tasks", "query" to "Find native"))
        await { vm.state.value.snapshot?.listing?.sections?.flatMap { it.rows }?.filterIsInstance<Row.Task>()?.any { it.row.title == "Find native entry" } == true }
        Assert.assertEquals("Keep project draft", vm.state.value.editor!!.title)
        capture(Intent("com.codedbydan.momentum.OPEN_TODAY"))
        Assert.assertEquals(View.Today, vm.state.value.view)
        Assert.assertEquals("Keep project draft", vm.state.value.editor!!.title)
        main { vm.saveEditor() }
        await { vm.state.value.editor == null && !vm.state.value.busy }
        val task = repository.read { allTasks().single { it.title == "Keep project draft" } }
        Assert.assertEquals(project, repository.detail(task.id)!!.projectId)
    }

    @Test fun lockedTaskEntriesDoNotReadIntoAnEditorOrMutateTheCurrentStore() = runBlocking {
        val id = create("Private locked fixture")
        val previous = repository.read { allTasks() }
        val undo = repository.read { canUndo() }
        for(intent in listOf(uri("complete-task", "title" to "Private locked fixture"), uri("reopen-task", "id" to id),
            uri("plan-task", "id" to id), uri("open-task", "id" to id), uri("find-tasks", "query" to "Private"),
            Intent("com.codedbydan.momentum.OPEN_TODAY"), Intent(Intent.ACTION_MAIN).putExtra("task-id", id))) {
            capture(intent, locked = true)
            Assert.assertEquals(app.getString(R.string.capture_unlock_required), vm.state.value.error)
            Assert.assertNull(vm.state.value.editor)
            Assert.assertEquals("", vm.state.value.query)
            Assert.assertEquals(previous, repository.read { allTasks() })
            Assert.assertEquals(undo, repository.read { canUndo() })
        }
    }

    @Test fun oversizedIntentAndSearchParametersPreserveTheCurrentDraftAndTaskStore() = runBlocking {
        main { vm.newTask("Bounded editor") }
        capture(uri("add", "title" to "Rejected", "notes" to "x".repeat(1_000_001)))
        Assert.assertEquals(app.getString(R.string.capture_too_large), vm.state.value.error)
        capture(uri("find-tasks", "query" to "q".repeat(4097)))
        Assert.assertEquals(app.getString(R.string.search_too_large), vm.state.value.error)
        Assert.assertEquals("Bounded editor", vm.state.value.editor!!.title)
        Assert.assertEquals("", vm.state.value.query)
        Assert.assertTrue(repository.read { allTasks() }.isEmpty())
    }
}
