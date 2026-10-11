// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import android.net.Uri
import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.ViewModelStore
import androidx.test.core.app.ApplicationProvider
import androidx.test.platform.app.InstrumentationRegistry
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.DraftStore
import com.codedbydan.momentum.data.EditorDraft
import com.codedbydan.momentum.data.MomentumRepository
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import java.io.File
import java.io.FilterInputStream
import java.io.InputStream
import java.util.UUID
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

/** Disposable files and blocking stream seam exercise real VM/Rust recovery boundaries. */
class NativeRecoveryTest {
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()
    private fun main(block: () -> Unit) = InstrumentationRegistry.getInstrumentation().runOnMainSync(block)
    private suspend fun await(condition: () -> Boolean) = withTimeout(10_000) { while(!condition()) delay(10) }

    @Test fun olderBlockedBackupCannotReplaceTheLaterChosenBackup() = runBlocking {
        val fixture = Fixture()
        val blocked = BlockingSource(fixture.backup("First backup"))
        try {
            val latest = fixture.backup("Latest backup")
            fixture.start { uri -> if(uri.lastPathSegment == "first") blocked.input() else latest.inputStream() }
            lateinit var first: Job
            main { first = fixture.vm.prepareImport(Uri.parse("content://isolated-recovery/first")) }
            assertTrue(blocked.entered.await(5, TimeUnit.SECONDS))
            lateinit var second: Job
            main { second = fixture.vm.prepareImport(Uri.parse("content://isolated-recovery/latest")) }
            second.join()
            assertTrue(fixture.vm.state.value.importReady)
            blocked.release.countDown()
            first.join()
            assertTrue(fixture.vm.state.value.importReady)
            main { fixture.vm.restore() }
            await { !fixture.vm.state.value.restoring && !fixture.vm.state.value.importReady }
            assertEquals(listOf("Latest backup"), fixture.repository.read { allTasks() }.map { it.title })
        } finally { blocked.release.countDown(); fixture.close() }
    }

    @Test fun cancelInvalidatesABlockedBackupWithoutRepublishingOrChangingTasks() = runBlocking {
        val fixture = Fixture()
        val blocked = BlockingSource(fixture.backup("Retained local task"))
        try {
            fixture.start { blocked.input() }
            lateinit var read: Job
            main { read = fixture.vm.prepareImport(Uri.parse("content://isolated-recovery/cancel")) }
            assertTrue(blocked.entered.await(5, TimeUnit.SECONDS))
            main { fixture.vm.cancelImport() }
            blocked.release.countDown(); read.join()
            assertFalse(fixture.vm.state.value.importReady)
            assertFalse(fixture.vm.state.value.importPreparing)
            assertEquals(listOf("Retained local task"), fixture.repository.read { allTasks() }.map { it.title })
            assertNull(fixture.vm.state.value.error)
        } finally { blocked.release.countDown(); fixture.close() }
    }

    @Test fun failedNormalDraftRetainsRecoveryIdentityAndCannotOverwriteAnActiveDraft() = runBlocking {
        val fixture = Fixture()
        val id = UUID.randomUUID().toString()
        val draft = EditorDraft(title = "Recover original", notes = "Private original notes\n📝")
        val store = DraftStore(fixture.directory)
        store.write(id, draft)
        val payload = File(fixture.directory, "android-drafts/$id.draft")
        val original = payload.readBytes()
        payload.writeText("isolated malformed draft")
        try {
            fixture.start(saved = SavedStateHandle(mapOf("editor-id" to id, "editor-view" to "morning")))
            await { !fixture.vm.state.value.editorLoading }
            assertTrue(fixture.vm.state.value.draftRecoveryPending)
            assertEquals(id, fixture.saved.get<String>("failed-editor-id"))
            assertNull(fixture.saved.get<String>("editor-id"))
            main { fixture.vm.newTask("Current draft") }
            await { fixture.vm.state.value.editor?.title == "Current draft" }
            assertNotEquals(id, fixture.saved.get<String>("editor-id"))
            payload.writeBytes(original)
            main { fixture.vm.retryDraftRecovery() }
            assertEquals("Current draft", fixture.vm.state.value.editor!!.title)
            assertTrue(fixture.vm.state.value.draftRecoveryPending)
            assertArrayEquals(original, payload.readBytes())
            main { fixture.vm.dismissEditor(); fixture.vm.retryDraftRecovery() }
            await { fixture.vm.state.value.editor == draft && !fixture.vm.state.value.editorLoading }
            assertFalse(fixture.vm.state.value.draftRecoveryPending)
            assertEquals(id, fixture.saved.get<String>("editor-id"))
            assertEquals("morning", fixture.saved.get<String>("editor-view"))
            assertNull(fixture.saved.get<String>("failed-editor-id"))
            assertTrue(fixture.repository.read { allTasks() }.isEmpty())
        } finally { fixture.close() }
    }

    @Test fun failedDraftCanRetryAfterVmReconstructionAndFailedRetryKeepsBytes() = runBlocking {
        val fixture = Fixture()
        val id = UUID.randomUUID().toString()
        val draft = EditorDraft(title = "Restart recovery", notes = "Unchanged source")
        DraftStore(fixture.directory).write(id, draft)
        val payload = File(fixture.directory, "android-drafts/$id.draft")
        val original = payload.readBytes()
        payload.writeText("isolated malformed restart draft")
        val malformed = payload.readBytes()
        try {
            fixture.start(saved = SavedStateHandle(mapOf("editor-id" to id)))
            await { !fixture.vm.state.value.editorLoading }
            main { fixture.vm.retryDraftRecovery() }
            await { !fixture.vm.state.value.editorLoading }
            assertArrayEquals(malformed, payload.readBytes())
            assertEquals(id, fixture.saved.get<String>("failed-editor-id"))
            fixture.reconstruct()
            assertTrue(fixture.vm.state.value.draftRecoveryPending)
            payload.writeBytes(original)
            main { fixture.vm.retryDraftRecovery() }
            await { fixture.vm.state.value.editor == draft && !fixture.vm.state.value.editorLoading }
            assertEquals(id, fixture.saved.get<String>("editor-id"))
            assertArrayEquals(original, payload.readBytes())
        } finally { fixture.close() }
    }

    @Test fun aSecondFailedDraftKeepsBothReferencesAcrossReconstruction() = runBlocking {
        val fixture = Fixture()
        val ids = List(2) { UUID.randomUUID().toString() }
        val drafts = List(2) { EditorDraft(title = "Retained failure $it", notes = "Original private source $it") }
        val payloads = ids.map { File(fixture.directory, "android-drafts/$it.draft") }
        val original = ids.mapIndexed { index, id -> DraftStore(fixture.directory).write(id, drafts[index]); payloads[index].readBytes() }
        payloads.forEach { it.writeText("isolated malformed source") }
        try {
            fixture.start(saved = SavedStateHandle(mapOf("editor-id" to ids[0])))
            main { fixture.saved["editor-id"] = ids[1] }
            fixture.reconstruct()
            assertEquals(ids[0], fixture.saved.get<String>("failed-editor-id"))
            assertEquals(6, fixture.saved.get<ArrayList<String>>("failed-editor-references")!!.size)
            payloads.forEachIndexed { index, file -> file.writeBytes(original[index]) }
            for(index in ids.indices) {
                main { fixture.vm.retryDraftRecovery() }
                await { fixture.vm.state.value.editor == drafts[index] && !fixture.vm.state.value.editorLoading }
                assertEquals(index == 0, fixture.vm.state.value.draftRecoveryPending)
                assertEquals(ids[index], fixture.saved.get<String>("editor-id"))
                main { fixture.vm.dismissEditor() }
            }
        } finally { fixture.close() }
    }

    @Test fun fullRecoveryListPreservesTheExtraActiveReferenceAndRejectsNewEditors() = runBlocking {
        val fixture = Fixture()
        val ids = List(33) { UUID.randomUUID().toString() }
        val store = DraftStore(fixture.directory)
        ids.forEachIndexed { index, id -> store.write(id, EditorDraft(title = "Quota source $index")) }
        File(fixture.directory, "android-drafts/${ids.last()}.draft").writeText("isolated active read fault")
        val references = ArrayList(ids.take(32).flatMap { listOf(it, "today", "") })
        try {
            fixture.start(saved = SavedStateHandle(mapOf("failed-editor-references" to references,
                "failed-editor-id" to ids.first(), "editor-id" to ids.last(), "editor-view" to "morning")))
            assertEquals(ids.last(), fixture.saved.get<String>("editor-id"))
            main { fixture.vm.newTask("Must not replace a recovery reference") }
            assertNull(fixture.vm.state.value.editor)
            assertEquals(ids.last(), fixture.saved.get<String>("editor-id"))
            assertEquals(96, fixture.saved.get<ArrayList<String>>("failed-editor-references")!!.size)
            main { fixture.vm.retryDraftRecovery() }
            await { fixture.vm.state.value.editor?.title == "Quota source 0" && !fixture.vm.state.value.editorLoading }
            val retained = fixture.saved.get<ArrayList<String>>("failed-editor-references")!!
            assertEquals(96, retained.size)
            assertTrue(retained.contains(ids.last()))
            assertFalse(retained.contains(ids.first()))
            assertEquals("isolated active read fault", File(fixture.directory, "android-drafts/${ids.last()}.draft").readText())
        } finally { fixture.close() }
    }

    private inner class Fixture {
        val directory = File(app.cacheDir, "recovery-${UUID.randomUUID()}")
        val repository = MomentumRepository(app, directory)
        val owners = ViewModelStore()
        lateinit var vm: MomentumViewModel
        lateinit var saved: SavedStateHandle
        private var source: (Uri) -> InputStream? = { app.contentResolver.openInputStream(it) }
        suspend fun start(saved: SavedStateHandle = SavedStateHandle(), open: (Uri) -> InputStream? = source) {
            this.saved = saved; source = open
            main { vm = MomentumViewModel(app, saved, repository, {}, {}, resumeAfterRestore = {}, openImportSource = open); owners.put("recovery", vm) }
            await { !vm.state.value.loading && !vm.state.value.editorLoading }
        }
        suspend fun reconstruct() { main { owners.clear() }; start(saved, source) }
        suspend fun backup(title: String): File {
            repository.read { allTasks() }.forEach { repository.mutate { deleteTask(it.id) } }
            repository.create(TaskDraft(title, "INBOX_PROJECT", null, null, null, 0.0, "", emptyList(), emptyList()), View.Today)
            return File(directory, "fixture-${UUID.randomUUID()}.json").also { repository.read { exportBackup(it.absolutePath) } }
        }
        suspend fun close() { if(::vm.isInitialized) main { vm.cancelImport(); owners.clear() }; repository.close(); directory.deleteRecursively() }
    }

    private class BlockingSource(private val file: File) {
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        fun input(): InputStream = object : FilterInputStream(file.inputStream()) {
            override fun read(buffer: ByteArray, offset: Int, length: Int): Int {
                entered.countDown()
                check(release.await(10, TimeUnit.SECONDS))
                return super.read(buffer, offset, length)
            }
        }
    }
}
