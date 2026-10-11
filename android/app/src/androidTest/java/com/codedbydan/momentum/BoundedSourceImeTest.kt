// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import android.view.inputmethod.EditorInfo
import android.view.inputmethod.ExtractedTextRequest
import android.view.inputmethod.InputConnection
import android.os.SystemClock
import android.view.InputDevice
import android.view.KeyCharacterMap
import android.view.KeyEvent
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.platform.InterceptPlatformTextInput
import androidx.compose.ui.platform.PlatformTextInputInterceptor
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.text.TextRange
import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.ViewModelStore
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import com.codedbydan.momentum.data.DraftStore
import com.codedbydan.momentum.data.MomentumRepository
import com.codedbydan.momentum.ui.MomentumApp
import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.runBlocking
import org.junit.*
import org.junit.runner.RunWith
import java.io.File
import java.util.UUID
import java.util.concurrent.atomic.AtomicReference

/**
 * Supported Compose session interception, using the editor's real Android InputConnection.
 * This covers composing commands and admission; it does not claim installed-keyboard acceptance.
 */
@RunWith(AndroidJUnit4::class)
class BoundedSourceImeTest {
    @get:Rule val compose = createComposeRule()
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()
    private lateinit var directory: File
    private lateinit var repository: MomentumRepository
    private lateinit var vm: MomentumViewModel
    private lateinit var saved: SavedStateHandle
    private val models = ViewModelStore()
    private val shown = mutableStateOf(false)
    private val currentConnection = AtomicReference<InputConnection?>(null)
    private val interceptor = PlatformTextInputInterceptor { request, _ ->
        val connection = request.createInputConnection(EditorInfo())
        currentConnection.set(connection)
        try { awaitCancellation() }
        finally {
            currentConnection.compareAndSet(connection, null)
            connection.closeConnection()
        }
    }

    @Before fun prepare() {
        directory = File(app.cacheDir, "bounded-ime-${UUID.randomUUID()}")
        repository = MomentumRepository(app, directory)
        compose.runOnUiThread {
            saved = SavedStateHandle()
            vm = MomentumViewModel(app, saved, repository, {}, {})
            models.put("vm", vm)
            shown.value = true
        }
        compose.setContent {
            if(shown.value) InterceptPlatformTextInput(interceptor) { MomentumApp(vm) }
        }
        compose.waitUntil(15_000) { !vm.state.value.loading && !vm.state.value.editorLoading }
    }

    @After fun cleanup() {
        compose.runOnUiThread {
            shown.value = false
            models.clear()
        }
        compose.waitForIdle()
        compose.runOnUiThread { currentConnection.getAndSet(null)?.closeConnection() }
        runBlocking { repository.close() }
        directory.deleteRecursively()
    }

    private fun field() = compose.onNodeWithTag("bounded-source-field")
    private fun visibleText() = field().fetchSemanticsNode().config[SemanticsProperties.EditableText].text
    private fun selection() = field().fetchSemanticsNode().config[SemanticsProperties.TextSelectionRange]

    private fun notes(source: String) {
        compose.runOnUiThread {
            if(vm.state.value.editor == null) vm.newTask("Native composing fixture")
            vm.setEditor(vm.state.value.editor!!.copy(notes = source))
        }
        field().performScrollTo().performClick().assertIsFocused()
        compose.waitUntil(5_000) { currentConnection.get() != null }
        Assert.assertEquals(4096, visibleText().length)
    }

    private fun command(action: InputConnection.() -> Boolean) {
        val currentVisible = visibleText()
        compose.runOnIdle {
            Assert.assertTrue("Commands target the current Notes session and visible section",
                currentVisible == currentConnection.get()!!.getExtractedText(ExtractedTextRequest(), 0).text.toString())
            Assert.assertTrue("The current Android input connection receives the command",
                currentConnection.get()!!.action())
        }
        compose.waitForIdle()
    }

    private fun assertComplete(expected: String) {
        Assert.assertTrue("A native composing edit preserves the entire source",
            vm.state.value.editor!!.notes == expected)
        runBlocking { vm.persistDraft() }
        Assert.assertTrue("The complete admitted source remains in the private draft",
            DraftStore(directory).read(saved.get<String>("editor-id")!!)!!.notes == expected)
    }

    private fun nativeUndo() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val down = SystemClock.uptimeMillis()
        val modifier = KeyEvent.META_CTRL_ON or KeyEvent.META_CTRL_LEFT_ON
        fun send(action: Int, code: Int, meta: Int) = instrumentation.sendKeySync(
            KeyEvent(down, SystemClock.uptimeMillis(), action, code, 0, meta,
                KeyCharacterMap.VIRTUAL_KEYBOARD, 0, 0, InputDevice.SOURCE_KEYBOARD))
        send(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_CTRL_LEFT, modifier)
        send(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_Z, modifier)
        send(KeyEvent.ACTION_UP, KeyEvent.KEYCODE_Z, modifier)
        send(KeyEvent.ACTION_UP, KeyEvent.KEYCODE_CTRL_LEFT, 0)
        compose.waitForIdle()
    }

    @Test fun smallNativeComposingInsertAtBothFullWindowEdgesRetainsCompositionAndCaret() {
        for(at in listOf(0, 4096)) {
            val source = "a".repeat(50_000)
            notes(source)
            command { setSelection(at, at) }
            Assert.assertEquals(TextRange(at), selection())
            command { setComposingText("日", 1) }
            assertComplete(source.take(at) + "日" + source.drop(at))
            Assert.assertTrue("Active composition uses a finite temporary section", visibleText().length <= 8192)
            Assert.assertEquals(TextRange(visibleText().indexOf("日") + 1), selection())

            // Replacing the current composition must replace 日, never append 日本 after it.
            command { setComposingText("日本", 1) }
            assertComplete(source.take(at) + "日本" + source.drop(at))
            Assert.assertTrue("Active composition uses a finite temporary section", visibleText().length <= 8192)
            Assert.assertEquals(TextRange(visibleText().indexOf("日本") + 2), selection())
            command { finishComposingText() }
            Assert.assertTrue("Committed text returns to the normal section bound", visibleText().length <= 4096)
            command { commitText("語", 1) }
            assertComplete(source.take(at) + "日本語" + source.drop(at))
        }
    }

    @Test fun nativeUndoAfterSectionNavigationDoesNotReplaceTheCurrentSectionWithPreviousText() {
        val source = "a".repeat(4096) + "b".repeat(4096) + "c".repeat(42_000)
        notes(source)
        command { setSelection(0, 1) }
        command { commitText("X", 1) }
        val edited = "X" + source.drop(1)
        assertComplete(edited)

        compose.onNodeWithText(app.getString(R.string.source_section_next, app.getString(R.string.notes)))
            .performScrollTo().performClick()
        field().performScrollTo().performClick().assertIsFocused()
        Assert.assertTrue("Navigation displays the distinct second source section", visibleText() == "b".repeat(4096))
        nativeUndo()
        assertComplete(edited)
        Assert.assertTrue("Native undo cannot paste previous-section text into this section",
            visibleText() == "b".repeat(4096))
    }

    @Test fun nativeUndoWithinCurrentSectionPreservesOtherSectionsAndPrivateDraft() {
        val source = "a".repeat(4096) + "b".repeat(4096) + "c".repeat(42_000)
        notes(source)
        command { setSelection(0, 1) }
        command { commitText("X", 1) }
        assertComplete("X" + source.drop(1))
        nativeUndo()
        assertComplete(source)
        Assert.assertTrue("Ordinary native undo restores only the edited section",
            visibleText() == "a".repeat(4096))
    }

    @Test fun nativeUndoWithinShortFinalSectionPreservesOtherSectionsAndPrivateDraft() {
        val source = "a".repeat(32_768) + "b".repeat(512)
        notes(source)
        repeat(8) {
            compose.onNodeWithText(app.getString(R.string.source_section_next, app.getString(R.string.notes)))
                .performScrollTo().performClick()
        }
        field().performScrollTo().performClick().assertIsFocused()
        Assert.assertTrue("Navigation reaches the short final source section", visibleText() == "b".repeat(512))
        command { setSelection(0, 1) }
        command { commitText("X", 1) }
        assertComplete(source.take(32_768) + "X" + source.drop(32_769))
        nativeUndo()
        assertComplete(source)
        Assert.assertTrue("Ordinary native undo restores only the short final section",
            visibleText() == "b".repeat(512))
    }

    @Test fun nativeUndoAfterSameStartCompositionCommitCannotDuplicateTheRewindowedTail() {
        val source = "a".repeat(50_000)
        notes(source)
        command { setSelection(0, 0) }
        command { setComposingText("日", 1) }
        command { setComposingText("日本", 1) }
        val edited = "日本" + source
        assertComplete(edited)
        command { finishComposingText() }
        field().assertIsFocused()
        Assert.assertEquals(4096, visibleText().length)
        Assert.assertTrue("Committed section retains the same beginning", visibleText().startsWith("日本"))
        nativeUndo()
        assertComplete(edited)
        Assert.assertEquals(4096, visibleText().length)
    }

    @Test fun oversizedNativeCompositionRetainsVisibleTextSelectionAndWholePrivateSource() {
        val source = "a".repeat(50_000)
        notes(source)
        command { setSelection(0, 0) }
        val originalVisible = visibleText()
        val originalSelection = selection()
        command { setComposingText("日".repeat(4097), 1) }
        compose.onNodeWithText(app.getString(R.string.source_section_composition)).assertExists()
        Assert.assertTrue("A rejected composing span leaves the visible source intact",
            visibleText() == originalVisible)
        Assert.assertEquals(originalSelection, selection())
        assertComplete(source)
    }
}
