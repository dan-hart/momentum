// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.ViewModelStore
import androidx.test.core.app.ApplicationProvider
import androidx.test.platform.app.InstrumentationRegistry
import com.codedbydan.momentum.data.DraftStore
import com.codedbydan.momentum.data.EditorDraft
import com.codedbydan.momentum.data.MomentumRepository
import com.codedbydan.momentum.core.TaskDraft
import com.codedbydan.momentum.core.View
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import java.io.File
import java.io.DataOutputStream
import java.util.Base64
import java.security.MessageDigest
import java.util.UUID

/** Real VM/private-file boundary; only disposable synthetic source is used. */
class NativeDraftAdmissionTest {
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()
    private fun main(block: () -> Unit) = InstrumentationRegistry.getInstrumentation().runOnMainSync(block)
    private suspend fun await(condition: () -> Boolean) = withTimeout(10_000) { while(!condition()) delay(10) }

    @Test fun overLimitEditKeepsVmSavedIdentityAndPrivateDraftExactlyUnchanged() = runBlocking {
        val directory = File(app.cacheDir, "draft-admission-${UUID.randomUUID()}")
        val repo = MomentumRepository(app, directory)
        val saved = SavedStateHandle()
        var models = ViewModelStore()
        lateinit var vm: MomentumViewModel
        val source = "a".repeat(4_000_000)
        val initial = EditorDraft(title = "Draft admission fixture", notes = source)
        try {
            main { vm = MomentumViewModel(app, saved, repo, {}, {}); models.put("vm", vm) }
            await { !vm.state.value.loading && !vm.state.value.editorLoading }
            main { vm.setEditor(initial) }
            vm.persistDraft()
            assertNull("A manual flush must not report cancelled autosave as an error", vm.state.value.error)
            val id = requireNotNull(saved.get<String>("editor-id"))
            val payload = File(directory, "android-drafts/$id.draft")
            val before = signature(payload)

            main { vm.setEditor(initial.copy(notes = source + "x")) }
            assertTrue("Rejected edit must preserve complete VM source", vm.state.value.editor == initial)
            assertTrue("Rejected edit explains the existing draft capacity", vm.state.value.editorCapacityError)
            assertEquals(id, saved.get<String>("editor-id"))
            vm.persistDraft()
            assertNull("Refusal and manual flush must preserve useful error state", vm.state.value.error)
            assertTrue("Rejected edit must preserve private draft bytes", before == signature(payload))
            assertTrue("Rejected edit must preserve decoded private source", DraftStore(directory).read(id) == initial)

            main { models.clear(); models = ViewModelStore(); vm = MomentumViewModel(app, saved, repo, {}, {}); models.put("vm", vm) }
            await { !vm.state.value.loading && !vm.state.value.editorLoading }
            assertTrue("Reconstructed VM must retain the complete original draft", vm.state.value.editor == initial)
            assertTrue("No core tasks were created by rejected draft editing", repo.read { allTasks() }.isEmpty())
        } finally {
            main { models.clear() }
            repo.close()
            directory.deleteRecursively()
        }
    }

    @Test fun oversizedExistingTaskRefusesEditorWithoutChangingTaskOrClaimingDraftIdentity() = runBlocking {
        val directory = File(app.cacheDir, "draft-opening-${UUID.randomUUID()}")
        val repo = MomentumRepository(app, directory)
        val saved = SavedStateHandle()
        val models = ViewModelStore()
        lateinit var vm: MomentumViewModel
        val source = "a".repeat(4_000_001)
        try {
            val id = requireNotNull(repo.create(TaskDraft("Oversized original fixture", "INBOX_PROJECT",
                null, null, null, 0.0, source, emptyList(), emptyList()), View.Today).id)
            main { vm = MomentumViewModel(app, saved, repo, {}, {}); models.put("vm", vm) }
            await { !vm.state.value.loading && !vm.state.value.editorLoading }
            main { vm.edit(id) }
            await { !vm.state.value.editorLoading }
            assertNull(vm.state.value.editor)
            assertEquals(app.getString(R.string.draft_open_capacity), vm.state.value.error)
            assertNull(saved.get<String>("editor-id"))
            assertNull(saved.get<String>("editor-view"))
            assertNull(saved.get<String>("editor-context"))
            assertTrue("The original core task remains complete", repo.detail(id)?.notes == source)
            assertEquals(1, repo.read { allTasks() }.size)
            assertFalse("Refused opening never creates a private draft", File(directory, "android-drafts").exists())
        } finally {
            main { models.clear() }
            repo.close()
            directory.deleteRecursively()
        }
    }

    @Test fun legacyDiskAndInlineRecordsAtOldLimitRetainTheirOriginalReferencesAcrossRetry() = runBlocking {
        val directory = File(app.cacheDir, "draft-legacy-budget-${UUID.randomUUID()}")
        val repo = MomentumRepository(app, directory)
        val id = UUID.randomUUID().toString()
        val saved = SavedStateHandle(mapOf("editor-id" to id, "editor-view" to "morning"))
        val models = ViewModelStore()
        lateinit var vm: MomentumViewModel
        val payload = File(directory, "android-drafts/$id.draft")
        writeLegacyBoundary(payload)
        val original = signature(payload)
        try {
            main { vm = MomentumViewModel(app, saved, repo, {}, {}); models.put("vm", vm) }
            await { !vm.state.value.loading && !vm.state.value.editorLoading }
            assertNull(vm.state.value.editor)
            assertEquals(app.getString(R.string.draft_open_capacity), vm.state.value.error)
            assertTrue(vm.state.value.draftRecoveryPending)
            assertEquals(id, saved.get<String>("failed-editor-id"))
            assertNull(saved.get<String>("editor-id"))
            assertTrue("Rejected upgrade retains the original record", original == signature(payload))
            main { vm.retryDraftRecovery() }
            await { !vm.state.value.editorLoading }
            assertNull(vm.state.value.editor)
            assertTrue(vm.state.value.draftRecoveryPending)
            assertEquals(id, saved.get<String>("failed-editor-id"))
            assertNull(saved.get<String>("editor-id"))
            assertTrue("Retry cannot replace or delete the old record", original == signature(payload))
            assertTrue(repo.read { allTasks() }.isEmpty())

            val legacy = payload.readText(Charsets.UTF_8)
            val inline = SavedStateHandle(mapOf("editor" to legacy))
            main { models.clear(); vm = MomentumViewModel(app, inline, repo, {}, {}); models.put("vm", vm) }
            await { !vm.state.value.loading && !vm.state.value.editorLoading }
            assertNull("Oversized inline recovery must not publish an editable draft", vm.state.value.editor)
            assertTrue(vm.state.value.draftRecoveryPending)
            assertEquals(app.getString(R.string.draft_open_capacity), vm.state.value.error)
            main { vm.retryDraftRecovery(); vm.newTask("Must preserve the legacy draft") }
            assertNull(vm.state.value.editor)
            assertNull(inline.get<String>("editor-id"))
            assertTrue("Retry/new capture cannot replace the original inline reference", inline.get<String>("editor") == legacy)
            assertTrue("Inline refusal also retains the existing disk record", original == signature(payload))
            assertTrue(repo.read { allTasks() }.isEmpty())
        } finally {
            main { models.clear() }
            repo.close()
            directory.deleteRecursively()
        }
    }
    /** Construct a valid old record without retaining a second binary/Base64 payload in the fixture. */
    private fun writeLegacyBoundary(payload: File) {
        val notes = "a".repeat(4_000_000)
        val project = "INBOX_PROJECT"
        val title = "t".repeat(4_500_000 - 48 - notes.length - project.length)
        payload.parentFile!!.mkdirs()
        payload.outputStream().use { raw ->
            DataOutputStream(Base64.getEncoder().wrap(raw)).use { output ->
                output.writeInt(1)
                listOf("", title, notes, project, "", "", "", "").forEach { text ->
                    val bytes = text.toByteArray(Charsets.UTF_8)
                    output.writeInt(bytes.size); output.write(bytes)
                }
                output.writeLong(-1); output.writeInt(0)
            }
        }
        assertEquals(6_000_000L, payload.length())
    }

    /** Equality is exact length plus a streamed SHA-256, without a text-sized snapshot buffer. */
    private fun signature(file: File): Pair<Long, List<Byte>> {
        val digest = MessageDigest.getInstance("SHA-256")
        file.inputStream().use { input ->
            val buffer = ByteArray(8192)
            while(true) {
                val count = input.read(buffer)
                if(count < 0) break
                digest.update(buffer, 0, count)
            }
        }
        return file.length() to digest.digest().toList()
    }

}
