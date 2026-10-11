// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import androidx.lifecycle.SavedStateHandle
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import com.codedbydan.momentum.core.View
import com.codedbydan.momentum.data.CaptureQueue
import com.codedbydan.momentum.data.EditorDraft
import com.codedbydan.momentum.data.MomentumRepository
import kotlinx.coroutines.*
import org.junit.*
import org.junit.runner.RunWith
import java.io.File
import java.util.UUID

@RunWith(AndroidJUnit4::class)
class CaptureQueueTest {
    private lateinit var directory: File
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()
    private fun main(block: () -> Unit) = InstrumentationRegistry.getInstrumentation().runOnMainSync(block)
    private fun await(block: () -> Boolean) = runBlocking { withTimeout(15_000) { while(!block()) delay(20) } }

    @Before fun prepare() { directory = File(app.cacheDir, "capture-queue-test-${UUID.randomUUID()}") }
    @After fun cleanup() { directory.deleteRecursively() }

    @Test fun reconstructedQueueRetainsFullPayloadsAndContextInFifoOrder() {
        val source = "Private source 📝\n".repeat(20_000)
        val first = EditorDraft(title = "First", notes = source)
        val second = EditorDraft(title = "Second", newTags = "shared")
        val queue = CaptureQueue(directory)
        val firstId = queue.enqueue(first, View.Morning)
        val secondId = queue.enqueue(second, View.Project("source-project"))
        var restored = CaptureQueue(directory)
        Assert.assertEquals(firstId, restored.head()!!.id)
        Assert.assertEquals(first, restored.head()!!.draft)
        Assert.assertEquals(View.Morning, restored.head()!!.origin)
        restored.complete(firstId)
        restored = CaptureQueue(directory)
        Assert.assertEquals(secondId, restored.head()!!.id)
        Assert.assertEquals(second, restored.head()!!.draft)
        Assert.assertEquals(View.Project("source-project"), restored.head()!!.origin)
        restored.complete(secondId)
        Assert.assertEquals(0, CaptureQueue(directory).size())
    }

    @Test fun capacityFailureKeepsEveryOlderPayloadAndTheExistingIndex() {
        val queue = CaptureQueue(directory)
        val ids = (0 until CaptureQueue.CAPACITY).map { queue.enqueue(EditorDraft(title = "Capture $it"), View.Today) }
        val index = File(directory, "android-drafts/capture-queue")
        val previous = index.readBytes()
        Assert.assertThrows(CaptureQueue.Full::class.java) { queue.enqueue(EditorDraft(title = "Overflow"), View.Today) }
        Assert.assertArrayEquals(previous, index.readBytes())
        val restored = CaptureQueue(directory)
        ids.forEachIndexed { position, id ->
            Assert.assertEquals(id, restored.head()!!.id)
            Assert.assertEquals("Capture $position", restored.head()!!.draft.title)
            restored.complete(id)
        }
        Assert.assertEquals(0, restored.size())
    }

    @Test fun twoSharesSurviveViewModelReconstructionWithoutReplacingTheActiveEditor() = runBlocking {
        val repo = MomentumRepository(app, directory)
        try {
            lateinit var vm: MomentumViewModel
            lateinit var saved: SavedStateHandle
            main { saved = SavedStateHandle(); vm = MomentumViewModel(app, saved, repo, {}, {}) }
            await { !vm.state.value.loading }
            main { vm.newTask("Existing editor") }
            vm.persistDraft()
            main {
                vm.captureTask("First #shared", "First full source\nSecond line 📝", "", "")
                vm.captureTask("Second", "Second full source", "", "")
            }
            await { CaptureQueue(directory).size() == 2 }
            await { vm.state.value.queuedCaptures == 2 }
            Assert.assertEquals("Existing editor", vm.state.value.editor!!.title)
            lateinit var restored: MomentumViewModel
            lateinit var restoredState: SavedStateHandle
            main {
                val values = saved.keys().associateWith { saved.get<Any>(it) }
                Assert.assertFalse(values.containsKey("editor"))
                Assert.assertTrue(values.values.filterIsInstance<String>().sumOf { it.length } < 512)
                restoredState = SavedStateHandle(values)
                restored = MomentumViewModel(app, restoredState, repo, {}, {})
            }
            await { !restored.state.value.editorLoading && !restored.state.value.loading }
            Assert.assertEquals("Existing editor", restored.state.value.editor!!.title)
            main { restored.dismissEditor() }
            await { restored.state.value.editor?.title == "First" }
            await { restored.state.value.queuedCaptures == 1 }
            Assert.assertEquals("First full source\nSecond line 📝", restored.state.value.editor!!.notes)
            Assert.assertEquals(2, CaptureQueue(directory).size()) // Active head remains durable.
            main { restored.dismissEditor() }
            await { restored.state.value.editor?.title == "Second" }
            await { restored.state.value.queuedCaptures == 0 }
            Assert.assertEquals("Second full source", restored.state.value.editor!!.notes)
            main { restored.dismissEditor() }
            await { CaptureQueue(directory).size() == 0 }
            Assert.assertTrue(repo.read { allTasks() }.isEmpty()) // Captures are drafts, not task mutations.
        } finally { repo.close() }
    }

    @Test fun activeCaptureRestoresFromTheQueueWithoutAnAndroidSavedStateBundle() = runBlocking {
        val repo = MomentumRepository(app, directory)
        try {
            val first = EditorDraft(title = "Recovered capture", notes = "Full source")
            val id = CaptureQueue(directory).enqueue(first, View.Morning)
            lateinit var restored: MomentumViewModel
            main { restored = MomentumViewModel(app, SavedStateHandle(), repo, {}, {}) }
            await { restored.state.value.editor != null }
            Assert.assertEquals(first, restored.state.value.editor)
            Assert.assertEquals(id, CaptureQueue(directory).head()!!.id)
            main { restored.saveEditor() }
            await { restored.state.value.editor == null && !restored.state.value.busy }
            Assert.assertEquals(com.codedbydan.momentum.core.Slot.MORNING, repo.read { taskMenu(allTasks().single().id) }!!.slot)
            await { CaptureQueue(directory).size() == 0 }
        } finally { repo.close() }
    }

    @Test fun viewModelCapacityFailureIsVisibleAndKeepsTheActiveEditor() = runBlocking {
        val repo = MomentumRepository(app, directory)
        try {
            lateinit var vm: MomentumViewModel
            main { vm = MomentumViewModel(app, SavedStateHandle(), repo, {}, {}) }
            await { !vm.state.value.loading }
            main {
                vm.newTask("Existing editor")
                repeat(CaptureQueue.CAPACITY + 1) { vm.captureTask("Queued $it", "", "", "") }
            }
            await { vm.state.value.error == app.getString(R.string.capture_queue_full) }
            Assert.assertEquals("Existing editor", vm.state.value.editor!!.title)
            val queue = CaptureQueue(directory)
            Assert.assertEquals(CaptureQueue.CAPACITY, queue.size())
            repeat(CaptureQueue.CAPACITY) { position ->
                val capture = queue.head()!!
                Assert.assertEquals("Queued $position", capture.draft.title)
                queue.complete(capture.id)
            }
            Assert.assertTrue(repo.read { allTasks() }.isEmpty())
        } finally { repo.close() }
    }

    @Test fun failedQueueCompletionPreservesTheActivePayloadForRecovery() = runBlocking {
        val repo = MomentumRepository(app, directory)
        try {
            val first = EditorDraft(title = "Recoverable first", notes = "Private source 📝")
            val queue = CaptureQueue(directory)
            val id = queue.enqueue(first, View.Morning)
            queue.enqueue(EditorDraft(title = "Second"), View.Today)
            lateinit var vm: MomentumViewModel
            main { vm = MomentumViewModel(app, SavedStateHandle(), repo, {}, {}) }
            await { vm.state.value.editor?.title == first.title }
            val index = File(directory, "android-drafts/capture-queue")
            val validIndex = index.readBytes()
            // An actual metadata read failure prevents completion's atomic index commit.
            index.writeText("isolated queue fault")
            main { vm.dismissEditor() }
            await { vm.state.value.error != null }
            Assert.assertEquals(first, com.codedbydan.momentum.data.DraftStore(directory).read(id))
            index.writeBytes(validIndex)
            Assert.assertEquals(first, CaptureQueue(directory).head()!!.draft)
            lateinit var recovered: MomentumViewModel
            main { recovered = MomentumViewModel(app, SavedStateHandle(), repo, {}, {}) }
            await { recovered.state.value.editor?.title == first.title }
            Assert.assertEquals(first, recovered.state.value.editor)
            Assert.assertEquals(2, CaptureQueue(directory).size())
        } finally { repo.close() }
    }

    @Test fun simultaneousStartupAndDrainRequestsClaimOnlyOneHead() = runBlocking {
        val repo = MomentumRepository(app, directory)
        try {
            val queue = CaptureQueue(directory)
            val firstId = queue.enqueue(EditorDraft(title = "First"), View.Today)
            val secondId = queue.enqueue(EditorDraft(title = "Second"), View.Today)
            lateinit var vm: MomentumViewModel
            lateinit var saved: SavedStateHandle
            main {
                saved = SavedStateHandle()
                vm = MomentumViewModel(app, saved, repo, {}, {})
                repeat(20) { vm.resumeQueuedCaptures() }
            }
            try { await { vm.state.value.editor?.title == "First" && !vm.state.value.editorLoading } }
            catch(e: TimeoutCancellationException) { throw AssertionError("First head not claimed: title=${vm.state.value.editor?.title}, loading=${vm.state.value.editorLoading}, error=${vm.state.value.error}, queue=${queue.size()}", e) }
            main { repeat(20) { vm.resumeQueuedCaptures() } }
            Assert.assertEquals(firstId, saved.get<String>("editor-id"))
            Assert.assertEquals(2, queue.size())
            main { vm.dismissEditor(); repeat(20) { vm.resumeQueuedCaptures() } }
            try { await { vm.state.value.editor?.title == "Second" && !vm.state.value.editorLoading } }
            catch(e: TimeoutCancellationException) { throw AssertionError("Second head not claimed: title=${vm.state.value.editor?.title}, loading=${vm.state.value.editorLoading}, error=${vm.state.value.error}, queue=${queue.size()}", e) }
            Assert.assertEquals(secondId, saved.get<String>("editor-id"))
            Assert.assertEquals(secondId, queue.head()!!.id)
            Assert.assertEquals(1, queue.size())
        } finally { repo.close() }
    }

    @Test fun unreadableQueueIndexDoesNotLoseAValidRestoredEditor() = runBlocking {
        val repo = MomentumRepository(app, directory)
        try {
            val draft = EditorDraft(title = "Valid editor", notes = "Keep my source")
            val id = UUID.randomUUID().toString()
            com.codedbydan.momentum.data.DraftStore(directory).write(id, draft)
            File(directory, "android-drafts/capture-queue").writeText("isolated queue index fault")
            lateinit var restored: MomentumViewModel
            main { restored = MomentumViewModel(app, SavedStateHandle(mapOf("editor-id" to id)), repo, {}, {}) }
            await { !restored.state.value.editorLoading }
            Assert.assertEquals(draft, restored.state.value.editor)
            Assert.assertNotNull(restored.state.value.error)
            main { restored.dismissEditor() }
            // Unknown queue ownership preserves bytes instead of risking deletion of a referenced payload.
            Assert.assertEquals(draft, com.codedbydan.momentum.data.DraftStore(directory).read(id))
        } finally { repo.close() }
    }
}
