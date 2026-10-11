// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.collectAsState
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.text.TextLayoutResult
import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.viewModelScope
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.codedbydan.momentum.core.TaskDraft
import com.codedbydan.momentum.core.View
import com.codedbydan.momentum.data.MomentumRepository
import com.codedbydan.momentum.ui.TaskEditor
import kotlinx.coroutines.cancel
import kotlinx.coroutines.runBlocking
import org.junit.*
import org.junit.runner.RunWith
import java.io.File
import java.util.UUID

/** Preserved imported strings must not bury native editor controls. */
@RunWith(AndroidJUnit4::class)
class TaskEditorImportedNamesTest {
    @get:Rule val compose = createComposeRule()
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()
    private lateinit var directory: File
    private lateinit var repository: MomentumRepository
    private lateinit var vm: MomentumViewModel
    private lateinit var project: String
    private lateinit var task: String
    private val projectTitle = "Imported project " + "📝".repeat(4096)
    private val childTitle = "Imported subtask " + "📝".repeat(4096)
    private val notes = "Full retained notes"

    @Before fun prepare() {
        directory = File(app.cacheDir, "editor-imported-${UUID.randomUUID()}").also { it.mkdirs() }
        repository = MomentumRepository(app, File(directory, "target"))
        runBlocking {
            val source = MomentumRepository(app, File(directory, "source"))
            val backup = File(directory, "import.json")
            try {
                project = source.read { addProject(projectTitle)!! }
                task = source.create(TaskDraft("Parent fixture", project, null, null, null,
                    0.0, notes, emptyList(), emptyList()), View.Today).id!!
                Assert.assertTrue(source.read { addSubtask(task, childTitle) }.changed)
                source.read { exportBackup(backup.absolutePath) }
            } finally { source.close() }
            repository.mutate { importBackup(backup.absolutePath) }
        }
        compose.runOnUiThread { vm = MomentumViewModel(app, SavedStateHandle(), repository, {}, {}) }
        compose.setContent { MaterialTheme { TaskEditor(vm.state.collectAsState().value, vm) } }
        compose.waitUntil(15_000) { !vm.state.value.loading && !vm.state.value.editorLoading }
        compose.runOnUiThread { vm.edit(task) }
        compose.waitUntil(15_000) { vm.state.value.editor?.id == task && !vm.state.value.editorLoading }
    }

    @After fun cleanup() = runBlocking {
        if(::vm.isInitialized) compose.runOnUiThread { vm.viewModelScope.cancel() }
        repository.close()
        directory.deleteRecursively()
        Unit
    }

    private fun assertBounded(node: SemanticsNodeInteraction, fullText: String, lines: Int) {
        node.assertIsDisplayed()
        val layouts = mutableListOf<TextLayoutResult>()
        node.performSemanticsAction(SemanticsActions.GetTextLayoutResult) { it(layouts) }
        Assert.assertEquals(fullText, layouts.single().layoutInput.text.text)
        Assert.assertTrue("Full imported name remains visually bounded", layouts.single().lineCount <= lines)
    }

    private fun assertPreserved() = runBlocking {
        Assert.assertEquals(projectTitle, repository.read { projects().single { it.id == project }.title })
        val detail = repository.detail(task)!!
        Assert.assertEquals(childTitle, detail.subTasks.single().title)
        Assert.assertEquals(notes, detail.notes)
        Assert.assertEquals(project, detail.projectId)
    }

    @Test fun selectedImportedProjectAndMenuKeepFullNamesWithoutBuryingNotesOrShare() {
        val field = compose.onNode(SemanticsMatcher.keyIsDefined(SemanticsProperties.EditableText) and
            hasText(app.getString(R.string.project)))
        Assert.assertEquals(projectTitle, field.fetchSemanticsNode().config[SemanticsProperties.EditableText].text)
        assertBounded(field, projectTitle, 1)
        field.performClick()
        val inMenu = hasAnyAncestor(isPopup())
        val option = compose.onNode(hasText(projectTitle) and inMenu, useUnmergedTree = true)
        assertBounded(option, projectTitle, 2)
        compose.onNode(hasText(projectTitle) and hasClickAction() and inMenu).performClick()
        compose.onNode(hasSetTextAction() and hasText(app.getString(R.string.notes))).performScrollTo().assertIsDisplayed()
        compose.onNodeWithText(app.getString(R.string.share_notes)).performScrollTo().assertIsDisplayed().assertIsEnabled()
        Assert.assertEquals(project, vm.state.value.editor!!.projectId)
        assertPreserved()
    }

    @Test fun importedSubtaskLeavesRepeatAndDuplicateReachableAndKeepsFullSemantics() {
        compose.waitUntil(15_000) { compose.onAllNodesWithText(childTitle).fetchSemanticsNodes().isNotEmpty() }
        assertBounded(compose.onNodeWithText(childTitle, useUnmergedTree = true).performScrollTo(), childTitle, 2)
        compose.onNodeWithText(app.getString(R.string.repeat)).performScrollTo().assertIsDisplayed().assertIsEnabled()
        compose.onNodeWithText(app.getString(R.string.duplicate)).performScrollTo().assertIsDisplayed().assertIsEnabled()
        Assert.assertEquals(notes, vm.state.value.editor!!.notes)
        assertPreserved()
    }
}
