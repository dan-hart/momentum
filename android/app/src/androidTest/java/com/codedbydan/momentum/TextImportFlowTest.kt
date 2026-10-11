// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import android.content.ClipData
import android.content.ClipboardManager
import android.os.Build
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.collectAsState
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.lifecycle.SavedStateHandle
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.DraftStore
import com.codedbydan.momentum.data.MomentumRepository
import com.codedbydan.momentum.ui.TextImportEditor
import com.codedbydan.momentum.platform.PrivateClipboard
import kotlinx.coroutines.runBlocking
import org.junit.*
import org.junit.runner.RunWith
import java.io.File
import java.util.UUID

@RunWith(AndroidJUnit4::class)
class TextImportFlowTest {
    @get:Rule val compose = createComposeRule()
    private lateinit var repo: MomentumRepository
    private lateinit var directory: File
    private lateinit var vm: MomentumViewModel
    private lateinit var saved: SavedStateHandle
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()

    @Before fun prepare() {
        directory = File(app.cacheDir, "text-import-test-${UUID.randomUUID()}")
        repo = MomentumRepository(app, directory)
        compose.runOnUiThread { saved = SavedStateHandle(); vm = MomentumViewModel(app, saved, repo, {}, {}) }
        compose.setContent { MaterialTheme { TextImportEditor(vm.state.collectAsState().value, vm) } }
        compose.waitUntil(15_000) { !vm.state.value.loading && !vm.state.value.editorLoading }
    }
    @After fun cleanup() = runBlocking { repo.close(); directory.deleteRecursively(); Unit }

    private fun requireDisposableClipboard() {
        Assume.assumeTrue("Clipboard mutation requires an explicitly owned emulator",
            InstrumentationRegistry.getArguments().getString("momentum.disposableOsTests") == "true" &&
                Build.HARDWARE in listOf("ranchu", "goldfish"))
    }

    @Test fun multilineImportUsesTheSourceContextDespiteLaterNavigation() {
        val elsewhere = runBlocking { repo.read { addProject("Elsewhere")!! } }
        compose.runOnUiThread { vm.navigate(View.Morning); vm.openTextImport() }
        compose.onNodeWithText("Source text").performTextInput("First task\n\nSecond task\n")
        compose.runOnUiThread { vm.navigate(View.Project(elsewhere)) }
        compose.onNodeWithContentDescription(app.getString(R.string.create_from_text)).performClick()
        compose.waitUntil(15_000) { vm.state.value.editor == null && !vm.state.value.loading }
        val tasks = runBlocking { repo.read { allTasks() } }
        Assert.assertEquals(setOf("First task", "Second task"), tasks.map { it.title }.toSet())
        Assert.assertTrue(tasks.all { runBlocking { repo.detail(it.id) }!!.projectId == "INBOX_PROJECT" })
        Assert.assertTrue(tasks.all { runBlocking { repo.read { taskMenu(it.id) } }!!.slot == Slot.MORNING })
    }

    @Test fun persistenceFailureKeepsTheDiskDraftAndRetryCreatesOneBatch() {
        val source = "Retained first\nRetained second"
        compose.runOnUiThread { vm.openTextImport() }
        compose.onNodeWithText("Source text").performTextInput(source)
        val id = saved.get<String>("editor-id")!!
        val blocker = File(directory, ".momentum-transaction").apply { writeText("isolated invalid journal") }
        compose.onNodeWithContentDescription(app.getString(R.string.create_from_text)).performClick()
        compose.waitUntil(15_000) { vm.state.value.error != null && !vm.state.value.busy }
        Assert.assertEquals(source, vm.state.value.editor!!.notes)
        Assert.assertTrue(vm.state.value.editor!!.isTextImport)
        Assert.assertEquals(source, DraftStore(directory).read(id)!!.notes)
        Assert.assertTrue(runBlocking { repo.read { allTasks() } }.isEmpty())
        Assert.assertTrue(blocker.delete())
        compose.onNodeWithContentDescription(app.getString(R.string.create_from_text)).performClick()
        compose.waitUntil(15_000) { vm.state.value.editor == null && !vm.state.value.loading }
        Assert.assertEquals(2, runBlocking { repo.read { allTasks() } }.size)
    }

    @Test fun largeImportRestoresFromDiskWithSmallSavedStateAndOriginalProject() {
        val source = "Private Unicode 📝\n".repeat(20_000)
        val project = runBlocking { repo.read { addProject("Restored source")!! } }
        // Restore a separate VM without putting a large text field through the rendering test.
        compose.runOnUiThread {
            vm.navigate(View.Project(project)); vm.openTextImport()
            vm.setEditor(vm.state.value.editor!!.copy(notes = source))
        }
        runBlocking { vm.persistDraft() }
        lateinit var restored: MomentumViewModel
        compose.runOnUiThread {
            val values = saved.keys().associateWith { saved.get<Any>(it) }
            Assert.assertFalse(values.containsKey("editor"))
            Assert.assertTrue(values.values.filterIsInstance<String>().sumOf { it.length } < 512)
            restored = MomentumViewModel(app, SavedStateHandle(values), repo, {}, {})
        }
        compose.waitUntil(15_000) { !restored.state.value.editorLoading }
        Assert.assertEquals(source, restored.state.value.editor!!.notes)
        Assert.assertTrue(restored.state.value.editor!!.isTextImport)
        // A smaller editable source lets the test verify restored destination without creating 20k tasks.
        compose.runOnUiThread {
            restored.navigate(View.Today)
            restored.setEditor(restored.state.value.editor!!.copy(notes = "Restored import"))
            restored.importEditorText(); restored.importEditorText()
        }
        compose.waitUntil(15_000) { restored.state.value.editor == null && !restored.state.value.busy }
        Assert.assertEquals(project, runBlocking { repo.detail(repo.read { allTasks() }.single().id) }!!.projectId)
    }

    @Test fun clipboardIsReadOnlyByExplicitPasteAndAppendsToTheSource() {
        requireDisposableClipboard()
        compose.runOnUiThread {
            app.getSystemService(ClipboardManager::class.java).setPrimaryClip(ClipData.newPlainText("Fixture", "Clipboard second"))
            vm.openTextImport()
        }
        Assert.assertEquals("", vm.state.value.editor!!.notes)
        compose.onNodeWithText("Source text").performTextInput("Typed first")
        compose.onNodeWithText("Paste text").performClick()
        Assert.assertEquals("Typed first\nClipboard second", vm.state.value.editor!!.notes)
    }

    @Test fun rejectedOversizedDraftKeepsThePreviousDiskFile() {
        val id = UUID.randomUUID().toString()
        val store = DraftStore(directory)
        val previous = com.codedbydan.momentum.data.EditorDraft(notes = "Previously saved", isTextImport = true)
        store.write(id, previous)
        val file = File(directory, "android-drafts/$id.draft")
        val bytes = file.readBytes()
        Assert.assertThrows(IllegalArgumentException::class.java) { store.write(id, previous.copy(notes = "x".repeat(4_000_001))) }
        Assert.assertArrayEquals(bytes, file.readBytes())
        Assert.assertEquals(previous, store.read(id))
    }

    @Test fun explicitPasteReadsOnlyOwnLargeTextClipAndRejectsOversizeWithoutChangingSource() {
        requireDisposableClipboard()
        Assume.assumeTrue("Android 8 rejects URI clipboard copies and provides explicit full-notes sharing", Build.VERSION.SDK_INT >= 28)
        compose.runOnUiThread { vm.openTextImport() }
        compose.onNode(hasSetTextAction() and hasText(app.getString(R.string.import_source))).assertIsFocused()
        val accepted = "Private Unicode 📝\n".repeat(4_000)
        runBlocking { PrivateClipboard.copy(app, "Disposable large fixture", accepted) }
        lateinit var uri: android.net.Uri
        compose.runOnUiThread { uri = requireNotNull(app.getSystemService(ClipboardManager::class.java).primaryClip!!.getItemAt(0).uri) }
        Assert.assertTrue(PrivateClipboard.owns(app, uri))
        Assert.assertTrue("Own-provider read must preserve the entire fixture",
            runBlocking { PrivateClipboard.readOwned(app, uri, 1_000_000) } == accepted)
        Assert.assertEquals("", vm.state.value.editor!!.notes)
        compose.onNodeWithText(app.getString(R.string.paste_text)).performClick()
        val failure = hasText(app.getString(R.string.import_clipboard_empty)) or
            hasText(app.getString(R.string.import_clipboard_unavailable)) or
            hasText(app.getString(R.string.import_text_too_large))
        compose.waitUntil(15_000) { vm.state.value.editor?.notes == accepted || compose.onAllNodes(failure).fetchSemanticsNodes().isNotEmpty() }
        Assert.assertTrue("Explicit paste must preserve all ${accepted.length} fixture characters; actual ${vm.state.value.editor?.notes?.length}",
            vm.state.value.editor?.notes == accepted)
        compose.runOnUiThread { vm.setEditor(vm.state.value.editor!!.copy(notes = "Retained source")) }
        runBlocking { PrivateClipboard.copy(app, "Disposable oversized fixture", "x".repeat(1_000_001)) }
        compose.onNodeWithText(app.getString(R.string.paste_text)).performClick()
        compose.waitUntil(15_000) { compose.onAllNodesWithText(app.getString(R.string.import_text_too_large)).fetchSemanticsNodes().isNotEmpty() }
        compose.onNodeWithText(app.getString(R.string.import_text_too_large)).assertIsDisplayed()
        Assert.assertEquals("Retained source", vm.state.value.editor!!.notes)
        runBlocking { PrivateClipboard.copy(app, "Fixture cleared", "") }
    }
}
