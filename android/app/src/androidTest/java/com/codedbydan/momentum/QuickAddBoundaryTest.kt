// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.collectAsState
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.StateRestorationTester
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.unit.dp
import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.viewModelScope
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import com.codedbydan.momentum.core.completeHashWord
import com.codedbydan.momentum.data.MomentumRepository
import com.codedbydan.momentum.ui.QuickAdd
import com.codedbydan.momentum.ui.TaskEditor
import kotlinx.coroutines.cancel
import kotlinx.coroutines.runBlocking
import org.junit.*
import org.junit.runner.RunWith
import java.io.File
import java.util.UUID

@RunWith(AndroidJUnit4::class)
class QuickAddBoundaryTest {
    @get:Rule val compose = createComposeRule()
    private lateinit var repository: MomentumRepository
    private lateinit var directory: File
    private lateinit var vm: MomentumViewModel
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()

    @Before fun prepare() {
        directory = File(app.cacheDir, "quick-boundary-${UUID.randomUUID()}")
        repository = MomentumRepository(app, directory)
    }
    @After fun cleanup() = runBlocking {
        if(::vm.isInitialized) compose.runOnUiThread { vm.viewModelScope.cancel() }
        repository.close()
        directory.deleteRecursively()
        Unit
    }

    private fun show(tag: String): StateRestorationTester {
        runBlocking { repository.read { addTag(tag)!! } }
        compose.runOnUiThread { vm = MomentumViewModel(app, SavedStateHandle(), repository, {}, {}) }
        val restoration = StateRestorationTester(compose)
        restoration.setContent { MaterialTheme { QuickAdd(vm.state.collectAsState().value, vm) } }
        compose.waitUntil(15_000) { !vm.state.value.loading && !vm.state.value.editorLoading && vm.state.value.snapshot != null }
        return restoration
    }
    private fun input() = compose.onNode(hasSetTextAction() and hasText(app.getString(R.string.quick_add)))
    private fun inputText() = input().fetchSemanticsNode().config[SemanticsProperties.EditableText].text

    @Test fun oversizedUnicodeTagCompletionKeepsEnteredTextAndTagAcrossSavedStateRestore() {
        val tag = "Oversized" + "📝".repeat(4096)
        val original = "Keep this #Over"
        Assert.assertTrue(completeHashWord(original, original.length.toUInt(), tag)!!.text.length > 4096)
        val restoration = show(tag)
        input().performTextInput(original)
        // The displayed suggestion stays compact even when a preserved imported tag is huge.
        val suggestion = compose.onNodeWithText("#$tag").assertIsDisplayed()
        val bounds = suggestion.getUnclippedBoundsInRoot()
        Assert.assertTrue("Long tag suggestion stays within a compact chip", bounds.bottom - bounds.top <= 64.dp)
        suggestion.performClick()
        val feedback = compose.onNodeWithText(app.getString(R.string.quick_add_limit))
        compose.waitUntil(5_000) { feedback.isDisplayed() }
        feedback.assertIsDisplayed()
        Assert.assertEquals(original, inputText())
        // Native caret-only callbacks must keep the reason for the rejected chip visible.
        input().performClick().performSemanticsAction(SemanticsActions.SetSelection) {
            it(original.length, original.length, false)
        }
        feedback.assertIsDisplayed()
        InstrumentationRegistry.getInstrumentation().sendKeyDownUpSync(android.view.KeyEvent.KEYCODE_DPAD_LEFT)
        compose.waitUntil(5_000) {
            input().fetchSemanticsNode().config[SemanticsProperties.TextSelectionRange].end == original.length - 1
        }
        feedback.assertIsDisplayed()
        Assert.assertEquals(original, inputText())
        restoration.emulateSavedInstanceStateRestore()
        Assert.assertEquals(original, inputText())
        input().performTextInput("x")
        compose.onNodeWithText(app.getString(R.string.quick_add_hint)).assertIsDisplayed()
        feedback.assertDoesNotExist()
        Assert.assertEquals(tag, runBlocking { repository.read { tags() } }.single().title)
        Assert.assertTrue(runBlocking { repository.read { allTasks() } }.isEmpty())
    }

    @Test fun completionExactlyAtLimitPreservesCoreTextCursorAndSavedState() {
        val tag = "Edge" + "x".repeat(4085)
        val original = "Task #Ed"
        val completed = completeHashWord(original, original.length.toUInt(), tag)!!
        Assert.assertEquals(4096, completed.text.length)
        val restoration = show(tag)
        input().performTextInput(original)
        compose.onNodeWithText("#$tag").performClick()
        Assert.assertEquals(completed.text, inputText())
        Assert.assertEquals(completed.cursor.toInt(), input().fetchSemanticsNode().config[SemanticsProperties.TextSelectionRange].end)
        restoration.emulateSavedInstanceStateRestore()
        Assert.assertEquals(completed.text, inputText())
        Assert.assertEquals(tag, runBlocking { repository.read { tags() } }.single().title)
    }

    @Test fun importedLongTagKeepsFullSemanticsAndNotesShareControlsReachableInTheActualEditor() {
        val tag = "Oversized" + "📝".repeat(4096)
        runBlocking { repository.read { addTag(tag)!! } }
        compose.runOnUiThread { vm = MomentumViewModel(app, SavedStateHandle(), repository, {}, {}) }
        compose.setContent { MaterialTheme { TaskEditor(vm.state.collectAsState().value, vm) } }
        compose.waitUntil(15_000) { !vm.state.value.loading && !vm.state.value.editorLoading }
        compose.runOnUiThread { vm.newTask() }
        compose.waitUntil(15_000) { vm.state.value.editor != null && !vm.state.value.editorLoading }
        compose.runOnUiThread { vm.setEditor(vm.state.value.editor!!.copy(title = "Disposable editor", notes = "Retained full notes")) }
        val chip = compose.onNodeWithText(tag).performScrollTo().assertIsDisplayed()
        val bounds = chip.getUnclippedBoundsInRoot()
        Assert.assertTrue("Imported editor tag stays within a compact chip", bounds.bottom - bounds.top <= 64.dp)
        compose.onNode(hasSetTextAction() and hasText(app.getString(R.string.notes))).performScrollTo().assertIsDisplayed()
        compose.onNodeWithText(app.getString(R.string.share_notes)).performScrollTo().assertIsDisplayed().assertIsEnabled()
        Assert.assertEquals("Retained full notes", vm.state.value.editor!!.notes)
        Assert.assertEquals(tag, runBlocking { repository.read { tags() } }.single().title)
    }
}
