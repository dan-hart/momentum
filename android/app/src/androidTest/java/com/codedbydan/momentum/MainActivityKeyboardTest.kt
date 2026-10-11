// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import android.os.SystemClock
import android.os.Build
import android.view.InputDevice
import android.view.KeyCharacterMap
import android.view.KeyEvent
import android.view.Window
import android.util.Log
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import com.codedbydan.momentum.data.KeyboardModifier
import com.codedbydan.momentum.data.SyncProvider
import org.junit.Assert
import org.junit.Assume
import org.junit.Test
import org.junit.rules.RuleChain
import org.junit.rules.TestRule
import org.junit.runner.Description
import org.junit.runner.RunWith
import org.junit.runners.model.Statement

/** The real Activity installs the production bridge; this test changes only transient search text. */
@RunWith(AndroidJUnit4::class)
class MainActivityKeyboardTest {
    private val compose = createAndroidComposeRule<MainActivity>()
    private val disposableDevice = object : TestRule {
        override fun apply(base: Statement, description: Description): Statement = object : Statement() {
            override fun evaluate() {
                // The outer rule checks before ActivityScenario can start stored OS integrations.
                Assume.assumeTrue("Explicit owned-emulator consent is required before launching the real app",
                    InstrumentationRegistry.getArguments().getString("momentum.disposableOsTests") == "true" &&
                        Build.HARDWARE in listOf("ranchu", "goldfish"))
                base.evaluate()
            }
        }
    }
    @get:org.junit.Rule val rules: RuleChain = RuleChain.outerRule(disposableDevice).around(compose)

    private fun key(code: Int, modifiers: Int) {
        compose.waitUntil(15_000) { compose.activity.window.decorView.hasWindowFocus() }
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val held = buildList {
            if(modifiers and KeyEvent.META_CTRL_ON != 0) add(KeyEvent.KEYCODE_CTRL_LEFT to (KeyEvent.META_CTRL_ON or KeyEvent.META_CTRL_LEFT_ON))
            if(modifiers and KeyEvent.META_ALT_ON != 0) add(KeyEvent.KEYCODE_ALT_LEFT to (KeyEvent.META_ALT_ON or KeyEvent.META_ALT_LEFT_ON))
            if(modifiers and KeyEvent.META_META_ON != 0) add(KeyEvent.KEYCODE_META_LEFT to (KeyEvent.META_META_ON or KeyEvent.META_META_LEFT_ON))
        }
        val down = SystemClock.uptimeMillis()
        var meta = 0
        fun send(action: Int, keyCode: Int) = instrumentation.sendKeySync(
            KeyEvent(down, SystemClock.uptimeMillis(), action, keyCode, 0, meta,
                KeyCharacterMap.VIRTUAL_KEYBOARD, 0, 0, InputDevice.SOURCE_KEYBOARD))
        for((modifier, mask) in held) { meta = meta or mask; send(KeyEvent.ACTION_DOWN, modifier) }
        send(KeyEvent.ACTION_DOWN, code)
        send(KeyEvent.ACTION_UP, code)
        for((modifier, mask) in held.asReversed()) { meta = meta and mask.inv(); send(KeyEvent.ACTION_UP, modifier) }
        compose.waitForIdle()
    }

    @Test fun actualActivityRoutesSearchAndPreservesNativeSelectAll() {
        lateinit var vm: MomentumViewModel
        compose.runOnUiThread {
            vm = ViewModelProvider(compose.activity)[MomentumViewModel::class.java]
            val window = compose.activity.window
            val original = window.callback
            window.callback = object : Window.Callback by original {
                override fun dispatchKeyEvent(event: KeyEvent): Boolean {
                    val editor = compose.activity.currentFocus?.onCheckIsTextEditor() == true
                    val handled = original.dispatchKeyEvent(event)
                    Log.i("MomentumKeyboardMain", "key:${event.keyCode}:${event.action}:${event.metaState}:editor=$editor:handled=$handled")
                    return handled
                }
                override fun dispatchKeyShortcutEvent(event: KeyEvent): Boolean {
                    val editor = compose.activity.currentFocus?.onCheckIsTextEditor() == true
                    val handled = original.dispatchKeyShortcutEvent(event)
                    Log.i("MomentumKeyboardMain", "shortcut:${event.keyCode}:${event.action}:${event.metaState}:editor=$editor:handled=$handled")
                    return handled
                }
            }
        }
        compose.waitUntil(15_000) { !vm.state.value.loading && !vm.state.value.editorLoading && vm.state.value.snapshot != null }
        val before = vm.state.value
        val snapshot = before.snapshot!!
        // The acceptance device holds only owned synthetic data and has no connected account.
        Assert.assertEquals(SyncProvider.Off, snapshot.settings.provider)
        Assert.assertNull("An existing draft must never be replaced by a keyboard fixture", before.editor)
        val modifier = when(snapshot.settings.keyboardModifier) {
            KeyboardModifier.Ctrl -> KeyEvent.META_CTRL_ON
            KeyboardModifier.Alt -> KeyEvent.META_ALT_ON
            KeyboardModifier.Meta -> KeyEvent.META_META_ON
        }
        try {
            key(KeyEvent.KEYCODE_F, modifier)
            compose.waitUntil(15_000) { vm.state.value.tab == Tab.Search && !vm.state.value.loading }
            val search = compose.onNode(hasSetTextAction() and hasText(compose.activity.getString(R.string.search)))
            search.assertIsFocused()
            compose.runOnUiThread { vm.search("") }
            compose.waitUntil(15_000) { !vm.state.value.loading && vm.state.value.query.isEmpty() }
            search.performTouchInput { click() }
            InstrumentationRegistry.getInstrumentation().sendStringSync("Keyboard fixture")
            compose.waitUntil(15_000) { !vm.state.value.loading && vm.state.value.query == "Keyboard fixture" }
            search.assertIsFocused()
            key(KeyEvent.KEYCODE_A, KeyEvent.META_CTRL_ON)
            val selection = search.fetchSemanticsNode().config[SemanticsProperties.TextSelectionRange]
            Assert.assertEquals(0, selection.min)
            Assert.assertEquals("Keyboard fixture".length, selection.max)
            InstrumentationRegistry.getInstrumentation().sendStringSync("Replacement")
            compose.waitUntil(15_000) { vm.state.value.query == "Replacement" && !vm.state.value.loading }
            Assert.assertEquals("Replacement", vm.state.value.query)
            Assert.assertTrue("Native text selection must not select tasks", vm.state.value.selected.isEmpty())
        } finally {
            compose.runOnUiThread { vm.search(before.query); vm.navigate(before.view, before.tab) }
            compose.waitUntil(15_000) { !vm.state.value.loading }
        }
    }
}
