// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import android.os.SystemClock
import android.view.View
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.text.TextRange
import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.ViewModelStore
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.codedbydan.momentum.data.DraftStore
import com.codedbydan.momentum.data.EditorDraft
import com.codedbydan.momentum.data.MomentumRepository
import com.codedbydan.momentum.ui.MomentumApp
import com.codedbydan.momentum.ui.SourceSectionBuffer
import com.codedbydan.momentum.ui.nativeSourceBoundaries
import kotlinx.coroutines.runBlocking
import org.junit.*
import org.junit.runner.RunWith
import java.io.File
import java.util.UUID
import java.security.MessageDigest

/** Real native input plus Rust/private-draft preservation, with entirely isolated synthetic data. */
@RunWith(AndroidJUnit4::class)
class BoundedSourceFieldTest {
    @get:Rule val compose = createComposeRule()
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()
    private lateinit var directory: File
    private lateinit var repo: MomentumRepository
    private lateinit var vm: MomentumViewModel
    private lateinit var saved: SavedStateHandle
    private var models = ViewModelStore()
    private val shown = mutableStateOf<MomentumViewModel?>(null)
    private lateinit var hostView: View

    @Before fun prepare() {
        directory = File(app.cacheDir, "bounded-source-${UUID.randomUUID()}")
        repo = MomentumRepository(app, directory)
        compose.runOnUiThread {
            saved = SavedStateHandle(); vm = MomentumViewModel(app, saved, repo, {}, {})
            models.put("vm", vm); shown.value = vm
        }
        compose.setContent {
            hostView = LocalView.current
            shown.value?.let { current -> key(current) { MomentumApp(current) } }
        }
        compose.waitUntil(15_000) { !vm.state.value.loading && !vm.state.value.editorLoading }
    }

    @After fun cleanup() {
        compose.runOnUiThread { shown.value = null; models.clear() }
        runBlocking { repo.close() }
        directory.deleteRecursively()
    }

    private fun field() = compose.onNodeWithTag("bounded-source-field")
    private fun visibleText() = field().fetchSemanticsNode().config[SemanticsProperties.EditableText].text
    private fun assertComplete(expected: String) {
        Assert.assertTrue("Complete source remains in the VM", expected == vm.state.value.editor!!.notes)
        runBlocking { vm.persistDraft() }
        Assert.assertTrue("Complete source remains in the private disk draft",
            expected == DraftStore(directory).read(saved.get<String>("editor-id")!!)!!.notes)
    }
    private fun privateSignature(file: File): Pair<Long, List<Byte>> {
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
    private fun notes(source: String, started: Long = SystemClock.elapsedRealtime()) {
        compose.runOnUiThread { vm.newTask("Bounded Unicode fixture"); vm.setEditor(vm.state.value.editor!!.copy(notes = source)) }
        awaitFieldVisible(started)
        Assert.assertTrue("Native field shapes at most one section", visibleText().length <= 4096)
    }

    private fun awaitFieldVisible(started: Long = SystemClock.elapsedRealtime()) {
        val remaining = 10_000 - (SystemClock.elapsedRealtime() - started)
        if(remaining <= 0) throw AssertionError(visibilityDiagnostic(started))
        var scrolled = false
        try {
            // Dialog attachment and the new task's asynchronous Title focus/IME insets use Android
            // work outside the Compose clock. A late inset/bring-into-view can invalidate a scroll;
            // re-scroll only while not displayed, within the original wall-clock deadline.
            compose.waitUntil("native editor field is displayed", remaining) {
                val node = compose.onAllNodesWithTag("bounded-source-field").fetchSemanticsNodes().singleOrNull()
                if(node == null || !node.layoutInfo.isPlaced) false
                else {
                    if(!scrolled || !field().isDisplayed()) {
                        field().performScrollTo()
                        scrolled = true
                    }
                    field().isDisplayed()
                }
            }
        } catch(_: ComposeTimeoutException) {
            throw AssertionError(visibilityDiagnostic(started))
        } catch(_: AssertionError) {
            throw AssertionError(visibilityDiagnostic(started))
        }
        if(SystemClock.elapsedRealtime() - started >= 10_000)
            throw AssertionError(visibilityDiagnostic(started))
    }

    /** Geometry/focus only: never dump EditableText, source content, or the complete semantics tree. */
    private fun visibilityDiagnostic(started: Long): String {
        val geometry = try {
            val nodes = compose.onAllNodesWithTag("bounded-source-field").fetchSemanticsNodes()
            val node = nodes.singleOrNull()
            val root = generateSequence(node) { it.parent }.lastOrNull()
            val focused = node?.config?.let { if(it.contains(SemanticsProperties.Focused)) it[SemanticsProperties.Focused] else null }
            "fields=${nodes.size},placed=${node?.layoutInfo?.isPlaced},fieldRoot=${node?.boundsInRoot}," +
                "fieldWindow=${node?.boundsInWindow},semanticRoot=${root?.boundsInWindow},fieldFocused=$focused"
        } catch(_: Exception) { "field geometry unavailable" }
        var host = "host unavailable"
        compose.runOnUiThread {
            if(::hostView.isInitialized) {
                val bounds = android.graphics.Rect()
                val visible = hostView.getGlobalVisibleRect(bounds)
                // The host activity normally loses native window focus to its separate Dialog.
                host = "hostAttached=${hostView.isAttachedToWindow},hostShown=${hostView.isShown}," +
                    "hostVisible=$visible,hostBounds=${bounds.toShortString()},hostWindowFocused=${hostView.hasWindowFocus()}"
            }
        }
        return "Native editor field not displayed within 10000ms; elapsed=${SystemClock.elapsedRealtime() - started}ms;$geometry;$host"
    }
    private fun next(label: Int) = compose.onNodeWithText(app.getString(R.string.source_section_next, app.getString(label)))
    private fun previous(label: Int) = compose.onNodeWithText(app.getString(R.string.source_section_previous, app.getString(label)))

    @Test fun hugeEmojiEditorShowsPromptlyAndSectionEditSavesWholeSourceOnceThroughRust() {
        val source = "😀".repeat(276_480) // 552,960 UTF-16 units, same measured shipping regression.
        val started = SystemClock.elapsedRealtime()
        notes(source, started)
        Assert.assertTrue("Native Notes field must become usable before the prior 27-second stall",
            SystemClock.elapsedRealtime() - started < 10_000)
        val firstLength = visibleText().length
        next(R.string.notes).performScrollTo().performClick()
        field().performScrollTo().performTextInputSelection(TextRange(0))
        val inserted = "Edited 📝\r\n"
        field().performTextInput(inserted)
        val expected = source.take(firstLength) + inserted + source.drop(firstLength)
        assertComplete(expected)
        Assert.assertTrue(visibleText().length <= 4096)
        previous(R.string.notes).performScrollTo().performClick()
        next(R.string.notes).performScrollTo().assertIsDisplayed()
        compose.onNodeWithContentDescription(app.getString(R.string.save)).performClick()
        compose.waitUntil(15_000) { vm.state.value.editor == null && !vm.state.value.busy && !vm.state.value.loading }
        val tasks = runBlocking { repo.read { allTasks() } }
        Assert.assertEquals(1, tasks.size)
        Assert.assertTrue("One Rust save retains both edited and undisplayed text", tasks.single().notes == expected)
    }

    @Test fun restoredPrivateHugeDraftOpensBoundedNotesWithoutFullTextInSavedState() {
        val source = "😀".repeat(276_480)
        notes(source); assertComplete(source)
        val values = saved.keys().associateWith { saved.get<Any>(it) }
        Assert.assertFalse(values.containsKey("editor"))
        Assert.assertTrue(values.values.filterIsInstance<String>().sumOf { it.length } < 512)
        compose.runOnUiThread {
            models.clear(); models = ViewModelStore()
            saved = SavedStateHandle(values); vm = MomentumViewModel(app, saved, repo, {}, {})
            models.put("restored", vm); shown.value = vm
        }
        compose.waitUntil(15_000) { !vm.state.value.editorLoading && !vm.state.value.loading }
        awaitFieldVisible()
        Assert.assertTrue(visibleText().length <= 4096); assertComplete(source)
        next(R.string.notes).performScrollTo().performClick()
        previous(R.string.notes).performScrollTo().assertIsEnabled()
    }

    @Test fun importRejectsWholeOverLimitEditWithoutChangingVisibleSectionSelectionOrPrivateSource() {
        val source = "😀".repeat(EditorDraft.MAX_TEXT_IMPORT_CHARS / 2)
        compose.runOnUiThread { vm.openTextImport(); vm.setEditor(vm.state.value.editor!!.copy(notes = source)) }
        field().performScrollTo().performTextInputSelection(TextRange(0))
        val before = visibleText()
        val selection = field().fetchSemanticsNode().config[SemanticsProperties.TextSelectionRange]
        field().performTextInput("x")
        compose.onNodeWithText(app.getString(R.string.import_text_too_large)).assertExists()
        Assert.assertTrue("Whole over-cap admission leaves the visible text intact", before == visibleText())
        Assert.assertEquals(selection, field().fetchSemanticsNode().config[SemanticsProperties.TextSelectionRange])
        assertComplete(source)
        next(R.string.import_source).performScrollTo().performClick()
        Assert.assertTrue(visibleText().length <= 4096); assertComplete(source)
    }

    @Test fun existingDraftByteLimitRejectsWholeNativeEditThenDeletionRestoresDurably() {
        val source = "a".repeat(4_000_000) // Existing private UTF-8 field budget, not a Rust task limit.
        notes(source)
        assertComplete(source)
        val identity = saved.get<String>("editor-id")!!
        val privateFile = File(directory, "android-drafts/$identity.draft")
        val privateBefore = privateSignature(privateFile)
        field().performScrollTo().performTextInputSelection(TextRange(2048))
        val visibleBefore = visibleText()
        val selectionBefore = field().fetchSemanticsNode().config[SemanticsProperties.TextSelectionRange]

        field().performTextInput("x")
        compose.onNodeWithText(app.getString(R.string.draft_edit_capacity)).assertExists()
        Assert.assertTrue("Whole over-budget input keeps the complete VM source", vm.state.value.editor!!.notes == source)
        Assert.assertEquals(identity, saved.get<String>("editor-id"))
        Assert.assertTrue("Refusal keeps the complete visible section", visibleBefore == visibleText())
        Assert.assertEquals(selectionBefore, field().fetchSemanticsNode().config[SemanticsProperties.TextSelectionRange])
        runBlocking { vm.persistDraft() }
        Assert.assertEquals("Refusal keeps the prior private draft length/SHA-256", privateBefore, privateSignature(privateFile))

        field().performTextInputSelection(TextRange(2048, 2049))
        field().performTextInput("")
        val expected = vm.state.value.editor!!.notes
        Assert.assertTrue("Exactly one ASCII character was deleted", expected.length == source.length - 1 && expected.all { it == 'a' })
        compose.onNodeWithText(app.getString(R.string.draft_edit_capacity)).assertDoesNotExist()
        Assert.assertEquals(identity, saved.get<String>("editor-id"))
        assertComplete(expected)
        val values = saved.keys().associateWith { saved.get<Any>(it) }
        compose.runOnUiThread {
            models.clear(); models = ViewModelStore()
            saved = SavedStateHandle(values); vm = MomentumViewModel(app, saved, repo, {}, {})
            models.put("restored", vm); shown.value = vm
        }
        compose.waitUntil(15_000) { !vm.state.value.editorLoading && !vm.state.value.loading }
        awaitFieldVisible()
        Assert.assertEquals(identity, saved.get<String>("editor-id"))
        Assert.assertTrue("Private reconstruction retains the whole accepted deletion", vm.state.value.editor!!.notes == expected)
        Assert.assertTrue(visibleText().length <= 4096)
        assertComplete(expected)
    }

    @Test fun actualAndroidIcuKeepsFamilyEmojiAndCrLfTogetherAtSectionBoundary() {
        val family = "👩‍👩‍👧‍👦"
        val source = "a".repeat(4094) + family + "\r\n" + "tail".repeat(10_000)
        notes(source)
        Assert.assertEquals(4094, visibleText().length)
        next(R.string.notes).performScrollTo().performClick()
        field().performScrollTo()
        Assert.assertTrue("Native ICU keeps the extended grapheme and CRLF intact", visibleText().startsWith(family + "\r\n"))
        assertComplete(source)
    }

    @Test fun singleOversizedCombiningSequenceStaysBoundedAndAdvancesWithoutSourceLoss() {
        val source = "a" + "\u0301".repeat(50_000) + "\r\n📝"
        notes(source)
        compose.onNodeWithText(app.getString(R.string.source_section_long_character)).assertExists()
        val first = compose.onNodeWithTag("source-section-position").fetchSemanticsNode().config[SemanticsProperties.Text].single().text
        next(R.string.notes).performScrollTo().performClick()
        field().performScrollTo()
        val second = compose.onNodeWithTag("source-section-position").fetchSemanticsNode().config[SemanticsProperties.Text].single().text
        Assert.assertNotEquals(first, second); Assert.assertTrue(visibleText().length <= 4096)
        assertComplete(source)
    }

    @Test fun nativeIcuOversizedZwjClusterTraversesSurrogateSafeSectionsWithoutLooping() {
        val source = "😀" + "\u200D😀".repeat(20_000)
        notes(source)
        val first = compose.onNodeWithTag("source-section-position").fetchSemanticsNode().config[SemanticsProperties.Text].single().text
        next(R.string.notes).performScrollTo().performClick()
        field().performScrollTo()
        val second = compose.onNodeWithTag("source-section-position").fetchSemanticsNode().config[SemanticsProperties.Text].single().text
        Assert.assertNotEquals(first, second)
        val buffer = SourceSectionBuffer(source, ::nativeSourceBoundaries)
        val complete = StringBuilder(); var sections = 0
        while(true) {
            Assert.assertEquals(complete.length, buffer.section.start)
            Assert.assertTrue(buffer.section.end > buffer.section.start && buffer.input.text.length <= 4096)
            Assert.assertFalse(buffer.input.text.first().isLowSurrogate())
            Assert.assertFalse(buffer.input.text.last().isHighSurrogate())
            complete.append(buffer.input.text); sections++
            if(buffer.section.end == source.length) break
            Assert.assertTrue("Native ICU traversal advances finitely", sections < 100)
            buffer.next()
        }
        Assert.assertTrue("Every Unicode source unit is retained", complete.toString() == source)
        assertComplete(source)
    }
}
