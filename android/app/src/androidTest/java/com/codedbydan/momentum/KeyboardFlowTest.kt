// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import android.os.SystemClock
import android.view.KeyEvent
import android.view.InputDevice
import android.view.KeyCharacterMap
import android.view.View as AndroidView
import android.view.Window
import android.app.Activity
import android.content.ContextWrapper
import android.util.Log
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.lifecycle.SavedStateHandle
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.KeyboardModifier
import com.codedbydan.momentum.data.MomentumRepository
import com.codedbydan.momentum.ui.MomentumApp
import com.codedbydan.momentum.ui.KeyboardCommandBridge
import kotlinx.coroutines.runBlocking
import org.junit.*
import org.junit.runner.RunWith
import java.io.File
import java.util.UUID
import java.util.concurrent.CopyOnWriteArrayList

/** Real Android KeyEvents enter the focused Compose window; no direct resolver dispatch. */
@RunWith(AndroidJUnit4::class)
class KeyboardFlowTest {
    @get:Rule val compose = createComposeRule()
    private lateinit var repo: MomentumRepository
    private lateinit var directory: File
    private lateinit var vm: MomentumViewModel
    private lateinit var rootView: AndroidView
    private val delivered = CopyOnWriteArrayList<String>()
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()

    @Before fun prepare() {
        directory = File(app.cacheDir, "keyboard-test-${UUID.randomUUID()}")
        repo = MomentumRepository(app, directory)
        compose.runOnUiThread { vm = MomentumViewModel(app, SavedStateHandle(), repo, {}, {}) }
        compose.setContent { rootView = LocalView.current; MomentumApp(vm) }
        compose.waitUntil(15_000) { !vm.state.value.loading && !vm.state.value.editorLoading && rootView.hasWindowFocus() }
        compose.runOnUiThread {
            var context = rootView.context
            while(context !is Activity && context is ContextWrapper) context = context.baseContext
            val window = (context as Activity).window
            val original = window.callback
            window.callback = object : Window.Callback by original {
                override fun dispatchKeyEvent(event: KeyEvent): Boolean {
                    val handled = original.dispatchKeyEvent(event)
                    observe("key", event, handled)
                    return handled
                }
                override fun dispatchKeyShortcutEvent(event: KeyEvent): Boolean {
                    val handled = original.dispatchKeyShortcutEvent(event)
                    observe("shortcut", event, handled)
                    return handled
                }
            }
        }
    }
    @After fun cleanup() = runBlocking { repo.close(); directory.deleteRecursively(); Unit }
    private fun observe(path: String, event: KeyEvent, handled: Boolean) {
        val entry = "$path:${event.keyCode}:${event.action}:${event.metaState}:$handled"
        delivered.add(entry)
        Log.i("MomentumKeyboardFixture", entry)
    }
    private fun key(code: Int, modifiers: Int, repeat: Int = 0) {
        val editorWindow = vm.state.value.editor != null
        if(editorWindow) compose.waitForIdle() else compose.waitUntil(10_000) { rootView.hasWindowFocus() }
        delivered.clear()
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val held = buildList {
            if(modifiers and KeyEvent.META_CTRL_ON != 0) add(KeyEvent.KEYCODE_CTRL_LEFT to (KeyEvent.META_CTRL_ON or KeyEvent.META_CTRL_LEFT_ON))
            if(modifiers and KeyEvent.META_ALT_ON != 0) add(KeyEvent.KEYCODE_ALT_LEFT to (KeyEvent.META_ALT_ON or KeyEvent.META_ALT_LEFT_ON))
            if(modifiers and KeyEvent.META_META_ON != 0) add(KeyEvent.KEYCODE_META_LEFT to (KeyEvent.META_META_ON or KeyEvent.META_META_LEFT_ON))
            if(modifiers and KeyEvent.META_SHIFT_ON != 0) add(KeyEvent.KEYCODE_SHIFT_LEFT to (KeyEvent.META_SHIFT_ON or KeyEvent.META_SHIFT_LEFT_ON))
        }
        var meta = 0
        val down = SystemClock.uptimeMillis()
        fun send(action: Int, keyCode: Int, count: Int = 0) = instrumentation.sendKeySync(
            KeyEvent(down, SystemClock.uptimeMillis(), action, keyCode, count, meta,
                KeyCharacterMap.VIRTUAL_KEYBOARD, 0, 0, InputDevice.SOURCE_KEYBOARD))
        for((modifier, mask) in held) { meta = meta or mask; send(KeyEvent.ACTION_DOWN, modifier) }
        send(KeyEvent.ACTION_DOWN, code, repeat)
        send(KeyEvent.ACTION_UP, code)
        for((modifier, mask) in held.asReversed()) { meta = meta and mask.inv(); send(KeyEvent.ACTION_UP, modifier) }
        compose.waitForIdle()
        // The editor has its own native Dialog Window. Its acceptance assertions prove delivery;
        // the underlying Activity correctly loses window focus and never receives Dialog keys.
        if(!editorWindow) Assert.assertTrue("Native key reached the Activity window: $delivered", delivered.any { it.startsWith("key:$code:") || it.startsWith("shortcut:$code:") })
    }
    private fun fixture(title: String = "Keyboard fixture"): String = runBlocking {
        repo.create(TaskDraft(title, "INBOX_PROJECT", null, null, null, 0.0, "", emptyList(), emptyList()), View.Today).id!!
    }
    private fun focusViewOptions() {
        // Native buttons reject focus in touch mode. A real Tab enters keyboard mode first.
        key(KeyEvent.KEYCODE_TAB, 0)
        val controls = compose.onAllNodesWithContentDescription(app.getString(R.string.options))
        val visible = controls.fetchSemanticsNodes().indices.filter { controls[it].isDisplayed() }
        Assert.assertEquals("One visible view-options control", 1, visible.size)
        controls[visible.single()].performSemanticsAction(SemanticsActions.RequestFocus) { it() }
        controls[visible.single()].assertIsFocused()
    }

    @Test fun ctrlAInSearchSelectsTextAndTaskShortcutsLeaveTasksUntouched() {
        val id = fixture()
        compose.runOnUiThread { vm.tab(Tab.Search) }
        compose.waitUntil(15_000) { !vm.state.value.loading }
        val search = compose.onNode(hasSetTextAction() and hasText(app.getString(R.string.search)))
        search.performTextInput("Keyboard fixture")
        search.assertIsFocused()
        compose.waitUntil(15_000) { !vm.state.value.loading }
        key(KeyEvent.KEYCODE_A, KeyEvent.META_CTRL_ON)
        Assert.assertTrue(vm.state.value.selected.isEmpty())
        val selection = search.fetchSemanticsNode().config[SemanticsProperties.TextSelectionRange]
        Assert.assertEquals(0, selection.min)
        Assert.assertEquals("Keyboard fixture".length, selection.max)
        search.performTextInput("Replacement")
        Assert.assertEquals("Replacement", vm.state.value.query)
        search.performTextReplacement("Keyboard fixture")
        compose.waitUntil(15_000) { !vm.state.value.loading }
        compose.runOnUiThread { vm.selectAll() }
        Assert.assertEquals(setOf(id), vm.state.value.selected)
        key(KeyEvent.KEYCODE_D, KeyEvent.META_CTRL_ON)
        key(KeyEvent.KEYCODE_FORWARD_DEL, KeyEvent.META_CTRL_ON)
        key(KeyEvent.KEYCODE_Z, KeyEvent.META_CTRL_ON)
        val task = runBlocking { repo.read { allTasks() } }.single()
        Assert.assertEquals(id, task.id)
        Assert.assertFalse(task.isDone)
    }

    @Test fun archiveAndEditorContextsDoNotDispatchTaskMutations() {
        val id = fixture()
        runBlocking { repo.mutate { setDone(id, true) }; repo.mutate { archiveDone() } }
        compose.runOnUiThread { vm.navigate(View.Archive) }
        compose.waitUntil(15_000) { !vm.state.value.loading }
        // Focus a native control so the key travels through the app's bubbling handler.
        focusViewOptions()
        key(KeyEvent.KEYCODE_FORWARD_DEL, KeyEvent.META_CTRL_ON)
        Assert.assertEquals(1, runBlocking { repo.snapshot(View.Archive).listing.sections.flatMap { it.rows }.filterIsInstance<Row.Task>() }.size)
        compose.runOnUiThread { vm.navigate(View.Today); vm.newTask("Untouched draft") }
        compose.onNode(hasSetTextAction() and hasText(app.getString(R.string.title))).performClick()
        key(KeyEvent.KEYCODE_D, KeyEvent.META_CTRL_ON)
        key(KeyEvent.KEYCODE_N, KeyEvent.META_CTRL_ON)
        Assert.assertEquals("Untouched draft", vm.state.value.editor!!.title)
        Assert.assertTrue(runBlocking { repo.read { allTasks() } }.isEmpty())
    }

    @Test fun archiveKeepsUndoAvailableThroughNativeKeyEvent() {
        val id = fixture("Undo archived keyboard task")
        runBlocking { repo.mutate { setDone(id, true) }; repo.mutate { archiveDone() } }
        compose.runOnUiThread { vm.navigate(View.Archive) }
        compose.waitUntil(15_000) { !vm.state.value.loading && vm.state.value.snapshot?.canUndo == true }
        Assert.assertEquals(1, runBlocking { repo.snapshot(View.Archive).listing.sections.flatMap { it.rows }.filterIsInstance<Row.Task>() }.size)
        focusViewOptions()
        key(KeyEvent.KEYCODE_Z, KeyEvent.META_CTRL_ON)
        compose.waitUntil(15_000) { !vm.state.value.loading && vm.state.value.snapshot?.listing?.sections?.flatMap { it.rows }?.filterIsInstance<Row.Task>()?.isEmpty() == true }
        val restored = runBlocking { repo.read { allTasks() } }.single()
        Assert.assertEquals(id, restored.id)
        Assert.assertFalse("Rust archive Undo restores the original task as open", restored.isDone)
    }

    @Test fun configuredSaveRejectsExtraModifiersAndRepeatThenCommitsOnce() {
        compose.runOnUiThread { vm.settings { it.copy(keyboardModifier = KeyboardModifier.Alt) } }
        compose.waitUntil(15_000) { vm.state.value.snapshot?.settings?.keyboardModifier == KeyboardModifier.Alt }
        compose.runOnUiThread { vm.newTask("One keyboard save") }
        compose.onNode(hasSetTextAction() and hasText(app.getString(R.string.title))).performClick()
        key(KeyEvent.KEYCODE_ENTER, KeyEvent.META_ALT_ON or KeyEvent.META_CTRL_ON)
        key(KeyEvent.KEYCODE_ENTER, KeyEvent.META_ALT_ON, repeat = 1)
        Assert.assertNotNull(vm.state.value.editor)
        Assert.assertTrue(runBlocking { repo.read { allTasks() } }.isEmpty())
        key(KeyEvent.KEYCODE_ENTER, KeyEvent.META_ALT_ON)
        key(KeyEvent.KEYCODE_ENTER, KeyEvent.META_ALT_ON)
        compose.waitUntil(15_000) { vm.state.value.editor == null && !vm.state.value.loading }
        Assert.assertEquals("One keyboard save", runBlocking { repo.read { allTasks() } }.single().title)
    }

    @Test fun oneNativeEventIsConsumedOnceAcrossComposeAndActivityFallback() {
        val bridge = KeyboardCommandBridge()
        var saves = 0
        bridge.editorHandler = { _, _ -> saves++; true }
        val now = SystemClock.uptimeMillis()
        val event = KeyEvent(now, now, KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_ENTER, 0, KeyEvent.META_CTRL_ON)
        Assert.assertTrue(bridge.dispatch(event, true))
        Assert.assertTrue(bridge.dispatch(event, true))
        Assert.assertEquals(1, saves)
    }

    @Test fun escapeKeepsNativeDiscardConfirmation() {
        compose.runOnUiThread { vm.newTask("Unsaved keyboard draft") }
        compose.onNode(hasSetTextAction() and hasText(app.getString(R.string.title))).performClick()
        key(KeyEvent.KEYCODE_ESCAPE, 0)
        compose.onNodeWithText(app.getString(R.string.discard_title)).assertIsDisplayed()
        Assert.assertNotNull(vm.state.value.editor)
        Assert.assertTrue(runBlocking { repo.read { allTasks() } }.isEmpty())
    }
}
