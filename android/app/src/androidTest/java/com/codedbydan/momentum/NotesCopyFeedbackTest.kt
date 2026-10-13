// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import android.content.ClipboardManager
import android.os.Build
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.collectAsState
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.semantics.SemanticsActions
import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.viewModelScope
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import com.codedbydan.momentum.data.MomentumRepository
import com.codedbydan.momentum.platform.PrivateClipboard
import com.codedbydan.momentum.ui.TaskEditor
import com.codedbydan.momentum.ui.NotesCopyActions
import android.view.HapticFeedbackConstants
import kotlinx.coroutines.cancel
import kotlinx.coroutines.runBlocking
import org.junit.*
import org.junit.runner.RunWith
import java.io.File
import java.util.UUID

/** Copies only synthetic notes on an explicitly owned emulator. */
@RunWith(AndroidJUnit4::class)
class NotesCopyFeedbackTest {
    @get:Rule val compose = createComposeRule()
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()
    private lateinit var directory: File
    private lateinit var repository: MomentumRepository
    private lateinit var vm: MomentumViewModel
    private var admitted = false
    private val haptics = java.util.concurrent.CopyOnWriteArrayList<Int>()
    private var rejectCopy = false
    @Volatile private var copies = 0

    @Before fun prepare() {
        Assume.assumeTrue("Clipboard fixture requires an owned emulator",
            InstrumentationRegistry.getArguments().getString("momentum.disposableOsTests") == "true" &&
                Build.HARDWARE in listOf("ranchu", "goldfish"))
        admitted = true
        if(Build.VERSION.SDK_INT >= 28) app.getSystemService(ClipboardManager::class.java).clearPrimaryClip()
        directory = File(app.cacheDir, "notes-copy-${UUID.randomUUID()}")
        repository = MomentumRepository(app, directory)
        compose.runOnUiThread { vm = MomentumViewModel(app, SavedStateHandle(), repository, {}, {}) }
        val actions = NotesCopyActions(copy = { context, label, text ->
            copies += 1
            if(rejectCopy) throw IllegalStateException("Fixture clipboard rejection")
            PrivateClipboard.copy(context, label, text)
        }, haptic = { _, type -> haptics += type })
        compose.setContent { MaterialTheme { TaskEditor(vm.state.collectAsState().value, vm, actions) } }
        compose.waitUntil(15_000) { !vm.state.value.loading }
        compose.runOnUiThread { vm.newTask("Notes copy fixture") }
        compose.waitUntil(15_000) { vm.state.value.editor != null }
    }

    @After fun cleanup() = runBlocking {
        if (!admitted) return@runBlocking
        if (::vm.isInitialized) compose.runOnUiThread { vm.viewModelScope.cancel() }
        if (::repository.isInitialized) repository.close()
        val clipboard = app.getSystemService(ClipboardManager::class.java)
        if(Build.VERSION.SDK_INT >= 28) clipboard.clearPrimaryClip()
        else PrivateClipboard.copy(app, "Fixture cleared", "")
        if (::directory.isInitialized) directory.deleteRecursively()
    }

    @Test fun successfulCopyShowsFeedbackInTheModalEditorAndPreservesAllNotes() {
        val notes = "Complete notes 📝\nSecond line — Grüße"
        compose.runOnUiThread { vm.setEditor(vm.state.value.editor!!.copy(notes = notes)) }
        // Activate the accessible native control to isolate copy feedback from
        // keyboard/clipboard window animations. The repeat test also taps it.
        compose.onNodeWithText(app.getString(R.string.copy_notes)).performScrollTo().assertIsEnabled()
            .performSemanticsAction(SemanticsActions.OnClick) { it() }
        compose.waitUntil(5_000) { copies == 1 }
        compose.waitUntil(5_000) {
            compose.onAllNodesWithText(app.getString(R.string.notes_copied)).fetchSemanticsNodes().isNotEmpty()
        }
        compose.onNodeWithText(app.getString(R.string.notes_copied)).assertIsDisplayed()
        compose.runOnUiThread {
            val clipboard = app.getSystemService(ClipboardManager::class.java)
            Assert.assertEquals(notes, clipboard.primaryClip!!.getItemAt(0).text.toString())
            Assert.assertEquals(notes, vm.state.value.editor!!.notes)
            Assert.assertEquals(listOf(if(Build.VERSION.SDK_INT >= 30) HapticFeedbackConstants.CONFIRM else HapticFeedbackConstants.VIRTUAL_KEY), haptics)
        }
    }

    @Test fun emptyNotesDisableCopyWithoutClipboardFeedbackOrHaptics() {
        compose.onNodeWithText(app.getString(R.string.copy_notes)).performScrollTo().assertIsNotEnabled()
        compose.onAllNodesWithText(app.getString(R.string.notes_copied)).assertCountEquals(0)
        compose.runOnUiThread { Assert.assertEquals(0, copies); Assert.assertTrue(haptics.isEmpty()) }
    }

    @Test fun failedClipboardCopyShowsAnErrorWithoutSuccessFeedbackOrHaptics() {
        compose.runOnUiThread {
            rejectCopy = true
            vm.setEditor(vm.state.value.editor!!.copy(notes = "Rejected fixture notes"))
        }
        compose.onNodeWithText(app.getString(R.string.copy_notes)).performScrollTo().assertIsEnabled()
            .performSemanticsAction(SemanticsActions.OnClick) { it() }
        compose.waitUntil(5_000) { copies == 1 }
        compose.waitUntil(5_000) {
            compose.onAllNodesWithText(app.getString(R.string.clipboard_copy_failed)).fetchSemanticsNodes().isNotEmpty()
        }
        compose.onNodeWithText(app.getString(R.string.clipboard_copy_failed)).performScrollTo().assertIsDisplayed()
        compose.onAllNodesWithText(app.getString(R.string.notes_copied)).assertCountEquals(0)
        compose.runOnUiThread { Assert.assertEquals(1, copies); Assert.assertTrue(haptics.isEmpty()) }
    }

    @Test fun repeatedCopyRestartsFeedbackAndDoesNotQueueOldToasts() {
        compose.runOnUiThread { vm.setEditor(vm.state.value.editor!!.copy(notes = "First fixture notes")) }
        val button = compose.onNodeWithText(app.getString(R.string.copy_notes))
        compose.waitForIdle()
        button.performScrollTo().performClick()
        compose.waitUntil(5_000) { haptics.size == 1 }
        compose.mainClock.advanceTimeBy(250)
        compose.onNodeWithText(app.getString(R.string.notes_copied)).assertIsDisplayed()
        compose.mainClock.advanceTimeBy(2_500)
        compose.runOnUiThread { vm.setEditor(vm.state.value.editor!!.copy(notes = "Latest fixture notes 📝")) }
        compose.mainClock.advanceTimeByFrame()
        // System clipboard feedback and our Snackbar may cover the button.
        // Exercise its native action while controlling the feedback deadline.
        button.assertIsEnabled().performSemanticsAction(SemanticsActions.OnClick) { it() }
        compose.waitUntil(5_000) { haptics.size == 2 }
        compose.mainClock.advanceTimeBy(250)
        compose.mainClock.advanceTimeBy(2_000)
        compose.onNodeWithText(app.getString(R.string.notes_copied)).assertIsDisplayed()
        compose.runOnUiThread {
            Assert.assertEquals(2, haptics.size)
            Assert.assertEquals("Latest fixture notes 📝", app.getSystemService(ClipboardManager::class.java).primaryClip!!.getItemAt(0).text.toString())
        }
        // About 4.45 seconds after the second copy: a restarted Short
        // Snackbar has gone, while a second queued Snackbar would remain.
        compose.mainClock.advanceTimeBy(2_200)
        compose.onAllNodesWithText(app.getString(R.string.notes_copied)).assertCountEquals(0)
        compose.runOnUiThread { Assert.assertEquals(2, copies) }

    }
}
