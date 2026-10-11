// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import android.net.Uri
import androidx.lifecycle.SavedStateHandle
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.MomentumRepository
import kotlinx.coroutines.*
import org.junit.*
import org.junit.runner.RunWith
import java.io.File
import java.util.UUID

@RunWith(AndroidJUnit4::class)
class ViewModelConcurrencyTest {
    private lateinit var repo: MomentumRepository
    private lateinit var directory: File
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()
    private fun main(block: () -> Unit) = InstrumentationRegistry.getInstrumentation().runOnMainSync(block)
    private suspend fun await(condition: () -> Boolean) = withTimeout(15_000) { while(!condition()) delay(20) }
    @Before fun prepare() { directory = File(app.cacheDir, "concurrency-${UUID.randomUUID()}"); repo = MomentumRepository(app, directory) }
    @After fun cleanup() = runBlocking { repo.close(); directory.deleteRecursively(); Unit }

    @Test fun aCaptureDuringSlowTaskOpenCannotOverwriteEitherDraft() = runBlocking {
        val id = repo.create(TaskDraft("Original task", "INBOX_PROJECT", null, null, null, 0.0, "", emptyList(), emptyList()), View.Today).id!!
        lateinit var vm: MomentumViewModel
        main { vm = MomentumViewModel(app, SavedStateHandle(), repo, {}, {}) }
        await { !vm.state.value.loading }
        val entered = CompletableDeferred<Unit>(); val release = CompletableDeferred<Unit>()
        val holding = launch(Dispatchers.IO) { repo.read { entered.complete(Unit); release.await() } }
        entered.await()
        main { vm.edit(id); vm.newTask("Queued new task") }
        Assert.assertTrue(vm.state.value.editorLoading)
        Assert.assertNull(vm.state.value.editor)
        release.complete(Unit); holding.join()
        await { vm.state.value.editor?.id == id && !vm.state.value.editorLoading }
        Assert.assertEquals("Original task", vm.state.value.editor!!.title)
        main { vm.dismissEditor() }
        await { vm.state.value.editor?.title == "Queued new task" }
        Assert.assertNull(vm.state.value.editor!!.id)
    }

    @Test fun restoreClaimsItsSourceBeforeSuspendAndRejectsCancelOrReentry() = runBlocking {
        repo.create(TaskDraft("Backup task", "INBOX_PROJECT", null, null, null, 0.0, "", emptyList(), emptyList()), View.Today)
        val backup = File(directory, "fixture-backup.json")
        repo.read { exportBackup(backup.absolutePath) }
        repo.create(TaskDraft("After backup", "INBOX_PROJECT", null, null, null, 0.0, "", emptyList(), emptyList()), View.Today)
        val entered = CompletableDeferred<Unit>(); val release = CompletableDeferred<Unit>(); var pauses = 0
        lateinit var vm: MomentumViewModel
        main { vm = MomentumViewModel(app, SavedStateHandle(), repo, {}, { pauses++; entered.complete(Unit); release.await() }) }
        await { !vm.state.value.loading }
        main { vm.prepareImport(Uri.fromFile(backup)) }
        await { vm.state.value.importReady }
        main { vm.restore(); vm.restore(); vm.cancelImport(); vm.action { addTask("Must wait", View.Today) } }
        entered.await()
        Assert.assertTrue(vm.state.value.restoring)
        Assert.assertTrue(vm.state.value.importReady)
        Assert.assertEquals(1, pauses)
        release.complete(Unit)
        await { !vm.state.value.restoring && !vm.state.value.loading }
        Assert.assertEquals(listOf("Backup task"), repo.read { allTasks() }.map { it.title })
        Assert.assertFalse(vm.state.value.importReady)
        Assert.assertNull(vm.state.value.error)
    }

    @Test fun externalQueriesAndIdentifiersCannotInflateSavedState() = runBlocking {
        val saved = SavedStateHandle()
        lateinit var vm: MomentumViewModel
        main { vm = MomentumViewModel(app, saved, repo, {}, {}) }
        await { !vm.state.value.loading }
        main { vm.search("Retained search"); vm.newTask("Draft"); vm.search("x".repeat(450_000)); vm.edit("y".repeat(450_000)) }
        Assert.assertEquals("Retained search", saved.get<String>("query"))
        Assert.assertNull(saved.get<String>("pending-open"))
        Assert.assertEquals(app.getString(R.string.search_too_large), vm.state.value.error)
        Assert.assertTrue(saved.keys().mapNotNull { saved.get<Any>(it) as? String }.sumOf { it.length } < 512)
        Assert.assertEquals("Draft", vm.state.value.editor?.title)
    }

    @Test fun restoreKeepsAdmissionUntilResumeAndResumesAfterAPauseFailure() = runBlocking {
        val backup = File(directory, "resume-fixture.json")
        repo.create(TaskDraft("Retained task", "INBOX_PROJECT", null, null, null, 0.0, "", emptyList(), emptyList()), View.Today)
        repo.read { exportBackup(backup.absolutePath) }
        for(failPause in listOf(false, true)) {
            val entered = CompletableDeferred<Unit>(); val release = CompletableDeferred<Unit>()
            var resumes = 0
            lateinit var vm: MomentumViewModel
            main { vm = MomentumViewModel(app, SavedStateHandle(), repo, {},
                { if(failPause) throw IllegalStateException("Fixture pause failed") },
                resumeAfterRestore = { resumes++; entered.complete(Unit); release.await() }) }
            val owners = androidx.lifecycle.ViewModelStore().also { main { it.put("fixture", vm) } }
            try {
                await { !vm.state.value.loading }
                main { vm.prepareImport(Uri.fromFile(backup)) }
                await { vm.state.value.importReady }
                main { vm.restore() }
                withTimeout(5_000) { entered.await() }
                Assert.assertTrue("UI admission stays claimed until policy resume finishes", vm.state.value.busy)
                Assert.assertTrue(vm.state.value.restoring)
                Assert.assertEquals(1, resumes)
                Assert.assertEquals(listOf("Retained task"), repo.read { allTasks() }.map { it.title })
                if(failPause) Assert.assertEquals("Fixture pause failed", vm.state.value.error)
                release.complete(Unit)
                await { !vm.state.value.restoring && !vm.state.value.busy }
            } finally { release.complete(Unit); main { owners.clear() } }
        }
    }
}
