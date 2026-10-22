// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import androidx.datastore.preferences.core.PreferenceDataStoreFactory
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.ViewModelStore
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.MomentumRepository
import com.codedbydan.momentum.platform.CompletionSoundChoice
import com.codedbydan.momentum.platform.CompletionSoundPlayer
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.first
import org.junit.*
import org.junit.Assert.*
import org.junit.runner.RunWith
import java.io.File
import java.util.UUID

@RunWith(AndroidJUnit4::class)
class CompletionSoundNativeTest {
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()
    private lateinit var directory: File
    private lateinit var repo: MomentumRepository
    private val owners = ViewModelStore()
    private fun main(block: () -> Unit) = InstrumentationRegistry.getInstrumentation().runOnMainSync(block)
    private suspend fun await(condition: () -> Boolean) = withTimeout(15_000) { while(!condition()) delay(20) }
    private fun draft(title: String) = TaskDraft(title, "INBOX_PROJECT", null, null, null, 0.0, "", emptyList(), emptyList())
    private class RecordingSoundPlayer : CompletionSoundPlayer {
        val choices = mutableListOf<CompletionSoundChoice>()
        var cancellations = 0
        var closed = false
        override fun play(choice: CompletionSoundChoice) { choices.add(choice) }
        override fun cancelPending() { cancellations++ }
        override fun close() { closed = true }
    }
    private suspend fun model(sound: RecordingSoundPlayer): MomentumViewModel {
        lateinit var vm: MomentumViewModel
        main {
            vm = MomentumViewModel(app, SavedStateHandle(), repo, {}, {}, soundPlayer = sound)
            owners.put(UUID.randomUUID().toString(), vm)
        }
        await { !vm.state.value.loading }
        return vm
    }
    private suspend fun action(vm: MomentumViewModel, block: Engine.() -> Outcome): Outcome {
        var outcome: Outcome? = null
        lateinit var job: Job
        main { job = vm.action { block().also { outcome = it } } }
        job.join()
        return requireNotNull(outcome)
    }
    @Before fun prepare() { directory = File(app.cacheDir, "completion-sound-${UUID.randomUUID()}"); repo = MomentumRepository(app, directory) }
    @After fun cleanup() = runBlocking { main { owners.clear() }; repo.close(); directory.deleteRecursively(); Unit }

    @Test fun settingsPersistTheStableChoiceAcrossOffOnAndRepositoryRecreation() = runBlocking {
        assertFalse(repo.settings().completionSound)
        assertEquals(CompletionSoundChoice.LITTLE_LIFT, repo.settings().completionSoundChoice)
        for(choice in CompletionSoundChoice.entries) {
            repo.updateSettings { it.copy(completionSound = true, completionSoundChoice = choice) }
            repo.updateSettings { it.copy(completionSound = false) }
            repo.close(); repo = MomentumRepository(app, directory)
            assertFalse(repo.settings().completionSound)
            assertEquals(choice, repo.settings().completionSoundChoice)
            repo.updateSettings { it.copy(completionSound = true) }
            assertEquals(choice, repo.settings().completionSoundChoice)
        }
    }

    @Test fun unknownPersistedSlugFallsBackToBrightFinish() = runBlocking {
        repo.close()
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
        val store = PreferenceDataStoreFactory.create(scope = scope) { File(directory, "android.preferences_pb") }
        store.edit { it[stringPreferencesKey("completion-sound-choice")] = "future-sound" }
        scope.coroutineContext[Job]!!.cancelAndJoin()
        repo = MomentumRepository(app, directory)
        assertEquals(CompletionSoundChoice.LITTLE_LIFT, repo.settings().completionSoundChoice)
        assertFalse(repo.settings().completionSound)
        suspend fun assertRawChoiceIsRetained() {
            repo.close()
            val readScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
            try {
                val native = PreferenceDataStoreFactory.create(scope = readScope) { File(directory, "android.preferences_pb") }
                assertEquals("future-sound", native.data.first()[stringPreferencesKey("completion-sound-choice")])
            } finally { readScope.coroutineContext[Job]!!.cancelAndJoin() }
            repo = MomentumRepository(app, directory)
        }
        assertRawChoiceIsRetained()
        repo.updateSettings { it.copy(colorfulLabels = false) }
        val partial = File(directory, "unrelated-settings.json").apply {
            writeText("""{"version":1,"values":{"grouping":"project"}}""")
        }
        repo.importAppSettings(partial)
        assertEquals(CompletionSoundChoice.LITTLE_LIFT, repo.settings().completionSoundChoice)
        assertRawChoiceIsRetained()
    }

    @Test fun eachRealCompletionVariantUsesCurrentSavedChoiceOnce() = runBlocking {
        val sound = RecordingSoundPlayer()
        val vm = model(sound)
        // Leave the published UiState deliberately stale and disabled.
        repo.updateSettings { it.copy(completionSound = true, completionSoundChoice = CompletionSoundChoice.HARP_SNAP) }
        assertFalse(vm.state.value.snapshot!!.settings.completionSound)
        for(archive in listOf(false, true)) {
            repo.updatePreferences { it.copy(autoArchive = archive) }
            val single = repo.create(draft("Single $archive"), View.Today).id!!
            val result = action(vm) { toggleDone(single) }
            assertEquals(if(archive) Message.TaskCompletedArchived else Message.TaskCompleted, result.message)
            val batch = (1..2).map { repo.create(draft("Batch $archive $it"), View.Today).id!! }
            val bulk = action(vm) { bulkDone(batch) }
            assertEquals(if(archive) Message.TasksCompletedArchived(2u) else Message.TasksCompleted(2u), bulk.message)
        }
        assertEquals(List(4) { CompletionSoundChoice.HARP_SNAP }, sound.choices)
        val notice = vm.state.value.notice!!
        main { vm.consumeNotice(notice); vm.refresh(); vm.tab(Tab.Settings) }
        await { !vm.state.value.loading }
        model(sound)
        assertEquals(4, sound.choices.size)
    }

    @Test fun defaultOffUndoReopenFailureAndUnchangedActionsRemainSilent() = runBlocking {
        val sound = RecordingSoundPlayer()
        val vm = model(sound)
        val id = repo.create(draft("Silence fixture"), View.Today).id!!
        val completion = action(vm) { toggleDone(id) }
        assertTrue(sound.choices.isEmpty())
        repo.updateSettings { it.copy(completionSound = true) }
        // Toast batch undo intentionally has no message; its changed outcome and
        // reopened task are the evidence that a real undo succeeded while enabled.
        val batchUndo = action(vm) { undoBatch(completion.undo!!) }
        assertTrue(batchUndo.changed)
        assertNull(batchUndo.message)
        assertFalse(repo.detail(id)!!.isDone)
        action(vm) { setDone(id, false) }
        action(vm) { addTask("Added task", View.Today) }
        assertTrue(sound.choices.isEmpty())
        action(vm) { setDone(id, true) }
        assertEquals(listOf(CompletionSoundChoice.LITTLE_LIFT), sound.choices)
        // Shared-core setDone reports a changed completion for an existing task,
        // even when already done. Count that actual successful outcome.
        val repeated = action(vm) { setDone(id, true) }
        assertTrue(repeated.changed)
        assertEquals(Message.TaskCompleted, repeated.message)
        assertEquals(List(2) { CompletionSoundChoice.LITTLE_LIFT }, sound.choices)
        assertNull(action(vm) { toggleDone(id) }.message)
        assertFalse(repo.detail(id)!!.isDone)
        val keyboardUndo = action(vm) { undo() }
        assertTrue(keyboardUndo.changed)
        assertEquals(Message.Undone, keyboardUndo.message)
        assertFalse(repo.detail(id)!!.isDone)
        assertFalse(action(vm) { setDone("missing-fixture-task", true) }.changed)
        assertFalse(action(vm) { bulkDone(emptyList()) }.changed)
        assertEquals(2, sound.choices.size)
        val blocker = File(directory, ".momentum-transaction").also { it.writeText("fixture fault") }
        try { assertTrue(action(vm) { setDone(id, true) }.message is Message.SaveFailed) }
        finally { blocker.delete() }
        assertEquals(2, sound.choices.size)
        lateinit var disabled: Job
        main { disabled = vm.settings { it.copy(completionSound = false) } }
        disabled.join()
        action(vm) { setDone(id, true) }
        assertEquals(2, sound.choices.size)
    }

    @Test fun previewReadsPersistedSettingsAndOwnerEndClosesThePlayer() = runBlocking {
        val sound = RecordingSoundPlayer()
        val vm = model(sound)
        lateinit var preview: Job
        main { preview = vm.previewCompletionSound(CompletionSoundChoice.QUIET_CHEER) }; preview.join()
        assertEquals(listOf(CompletionSoundChoice.QUIET_CHEER), sound.choices)
        main {
            vm.settings { it.copy(completionSound = true, completionSoundChoice = CompletionSoundChoice.WARM_SPARK) }
            preview = vm.previewCompletionSound(CompletionSoundChoice.QUIET_CHEER)
        }
        preview.join()
        assertEquals(listOf(CompletionSoundChoice.QUIET_CHEER, CompletionSoundChoice.QUIET_CHEER), sound.choices)
        main { vm.completionSoundForeground(false); preview = vm.previewCompletionSound(CompletionSoundChoice.QUIET_CHEER) }; preview.join()
        assertEquals(2, sound.choices.size)
        assertTrue(sound.cancellations > 0)
        main { owners.clear() }
        assertTrue(sound.closed)
    }
}
