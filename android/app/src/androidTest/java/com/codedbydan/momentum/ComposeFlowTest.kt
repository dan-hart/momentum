// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.lifecycle.SavedStateHandle
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.MomentumRepository
import com.codedbydan.momentum.ui.MomentumApp
import kotlinx.coroutines.runBlocking
import org.junit.*
import org.junit.runner.RunWith
import java.io.File
import java.util.UUID

@RunWith(AndroidJUnit4::class)
class ComposeFlowTest {
    @get:Rule val compose = createComposeRule()
    private lateinit var repo: MomentumRepository
    private lateinit var directory: File
    private lateinit var vm: MomentumViewModel
    private lateinit var saved: SavedStateHandle
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()

    @Before fun prepare() {
        val app = ApplicationProvider.getApplicationContext<MomentumApplication>()
        directory = File(app.cacheDir, "compose-test-${UUID.randomUUID()}")
        repo = MomentumRepository(app, directory)
        compose.runOnUiThread { saved = SavedStateHandle(); vm = MomentumViewModel(app, saved, repo, {}, {}) }
        compose.setContent { MomentumApp(vm) }
        compose.waitUntil(15_000) { !vm.state.value.loading && !vm.state.value.editorLoading }
    }
    @After fun cleanup() = runBlocking { repo.close(); directory.deleteRecursively(); Unit }

    @Test fun fullEditorCreatesTaskAndCheckboxUndoUsesRustBatch() {
        compose.onNodeWithContentDescription("Add task").performClick()
        compose.onNodeWithText("Title").performTextInput("Offline Compose task")
        compose.onNodeWithText("Notes").performScrollTo().performTextInput("Stored from native editor")
        compose.onNodeWithContentDescription(app.getString(R.string.save)).performClick()
        compose.waitUntil(15_000) { vm.state.value.editor == null && !vm.state.value.loading }
        compose.onNodeWithText("Offline Compose task").assertIsDisplayed()
        compose.onNodeWithContentDescription("Mark as done: Offline Compose task").performClick()
        compose.waitUntil(15_000) { vm.state.value.notice?.undo != null }
        compose.onNodeWithText("Undo").performClick()
        compose.waitUntil(15_000) {
            vm.state.value.snapshot?.listing?.sections.orEmpty().flatMap { it.rows }
                .filterIsInstance<Row.Task>().singleOrNull()?.row?.isDone == false
        }
        val tasks = runBlocking { repo.read { allTasks() } }
        Assert.assertFalse(tasks.single().isDone)
        Assert.assertEquals("Stored from native editor", tasks.single().notes)
    }

    @Test fun archivedTaskHasNoCompletionOrEditingAction() {
        runBlocking {
            val created = repo.create(TaskDraft("Archived fixture", "INBOX_PROJECT", null, null, null, 0.0, "", emptyList(), emptyList()), View.Today)
            repo.mutate { setDone(created.id!!, true) }
            repo.mutate { archiveDone() }
        }
        compose.runOnUiThread { vm.navigate(View.Archive) }
        compose.waitUntil(15_000) { !vm.state.value.loading }
        compose.onNodeWithText("Archived fixture").assertIsDisplayed()
        compose.onNodeWithContentDescription("Reopen task: Archived fixture").assertDoesNotExist()
        compose.onNodeWithContentDescription("Add task").assertDoesNotExist()
    }

    @Test fun sourceContextSurvivesNavigationBeforeSave() {
        val project = runBlocking {
            // Morning is a conditional destination. Seed a real member so native
            // snapshot reconciliation keeps that destination available during entry.
            repo.mutate { addTask("Source morning anchor", View.Morning) }
            repo.read { addProject("Source project")!! }
        }
        compose.runOnUiThread {
            vm.navigate(View.Morning)
            vm.newTask("Morning draft")
            Assert.assertEquals("The editor retains its initial source", "morning", saved.get<String>("editor-view"))
            vm.navigate(View.Project(project))
            vm.saveEditor()
        }
        compose.waitUntil(15_000) { vm.state.value.editor == null && !vm.state.value.loading }
        val task = runBlocking { repo.read { allTasks() }.single { it.title == "Morning draft" } }
        Assert.assertEquals("INBOX_PROJECT", runBlocking { repo.detail(task.id) }!!.projectId)
        val tags = runBlocking { repo.detail(task.id) }!!.tagIds
        val allTags = runBlocking { repo.read { tags() } }
        Assert.assertTrue("Morning draft tags: $tags; available tags: $allTags", allTags.any { it.id in tags && it.title == "Morning" })
        Assert.assertTrue(runBlocking { repo.snapshot(View.Morning).listing.sections.flatMap { it.rows }.filterIsInstance<Row.Task>().any { it.row.id == task.id } })
        Assert.assertTrue(runBlocking { repo.snapshot(View.Project(project)).listing.sections.flatMap { it.rows }.isEmpty() })
    }

    @Test fun failedEditorSaveRetainsDraftAndRetryCommitsOnce() {
        compose.runOnUiThread { vm.newTask("Retained draft") }
        val blocker = File(directory, ".momentum-transaction")
        blocker.writeText("isolated invalid journal")
        compose.onNodeWithContentDescription(app.getString(R.string.save)).performClick()
        compose.waitUntil(15_000) { vm.state.value.error != null && !vm.state.value.busy }
        compose.onNodeWithText("Retained draft").assertIsDisplayed()
        Assert.assertTrue(runBlocking { repo.read { allTasks() } }.isEmpty())
        blocker.delete()
        compose.onNodeWithContentDescription(app.getString(R.string.save)).performClick()
        compose.waitUntil(15_000) { vm.state.value.editor == null && !vm.state.value.loading }
        Assert.assertEquals("Retained draft", runBlocking { repo.read { allTasks() } }.single().title)
    }

    @Test fun largePrivateDraftUsesSmallSavedStateAndRestoresWithoutTruncation() {
        val notes = "Private Unicode 📝\n".repeat(25_000)
        compose.runOnUiThread { vm.newTask("Large draft"); vm.setEditor(vm.state.value.editor!!.copy(notes = notes)) }
        runBlocking { vm.persistDraft() }
        lateinit var restored: MomentumViewModel
        compose.runOnUiThread {
            val values = saved.keys().associateWith { saved.get<Any>(it) }
            Assert.assertFalse(values.containsKey("editor"))
            Assert.assertTrue(values.values.filterIsInstance<String>().sumOf { it.length } < 512)
            restored = MomentumViewModel(ApplicationProvider.getApplicationContext(), SavedStateHandle(values), repo, {}, {})
        }
        compose.waitUntil(15_000) { !restored.state.value.editorLoading }
        Assert.assertEquals(notes, restored.state.value.editor!!.notes)
        Assert.assertEquals("Large draft", restored.state.value.editor!!.title)
    }

    @Test fun removedTaskRetainsEditsAndCanBeSavedAsNew() {
        val id = runBlocking { repo.create(TaskDraft("Before", "INBOX_PROJECT", null, null, null, 0.0, "", emptyList(), emptyList()), View.Today).id!! }
        compose.runOnUiThread { vm.edit(id) }
        compose.waitUntil(15_000) { vm.state.value.editor != null }
        compose.onNodeWithText("Title").performTextReplacement("Retained edits")
        runBlocking { repo.mutate { deleteTask(id) } }
        compose.onNodeWithContentDescription(app.getString(R.string.save)).performClick()
        compose.waitUntil(15_000) { vm.state.value.editorUnavailable && !vm.state.value.busy }
        compose.onNodeWithText("Retained edits").assertIsDisplayed()
        compose.onNodeWithText("Save as a new task").performClick()
        compose.waitUntil(15_000) { vm.state.value.editor == null && !vm.state.value.loading }
        Assert.assertEquals("Retained edits", runBlocking { repo.read { allTasks() } }.single().title)
    }

    @Test fun distinctRapidPreferenceChangesPersistTogether() {
        compose.runOnUiThread {
            vm.settings { it.copy(dynamicColor = true) }
            vm.settings { it.copy(compact = true) }
            vm.preferences { it.copy(autoArchive = true) }
            vm.preferences { it.copy(upcomingDays = 30u) }
        }
        compose.waitUntil(15_000) {
            vm.state.value.snapshot?.let { it.settings.dynamicColor && it.settings.compact && it.preferences.autoArchive && it.preferences.upcomingDays == 30u } == true
        }
        Assert.assertTrue(runBlocking { repo.settings() }.dynamicColor)
        Assert.assertTrue(runBlocking { repo.settings() }.compact)
        Assert.assertEquals(30u, runBlocking { repo.snapshot(View.Today) }.preferences.upcomingDays)
    }

    @Test fun todayTabRetainsProjectAndSearchQuery() {
        val project = runBlocking { repo.read { addProject("Workspace")!! } }
        compose.runOnUiThread { vm.navigate(View.Project(project)); vm.tab(Tab.Search); vm.search("Retained query"); vm.tab(Tab.Settings); vm.tab(Tab.Today) }
        compose.waitUntil(15_000) { !vm.state.value.loading }
        Assert.assertEquals(View.Project(project), vm.state.value.view)
        compose.runOnUiThread { vm.tab(Tab.Search) }
        Assert.assertEquals("Retained query", vm.state.value.query)
    }

    @Test fun quickEntryCreatesUsingSharedTagsEstimateAndCurrentContext() {
        compose.onNodeWithText("Quick add").performTextInput("Native quick task #work 1h 30m")
        compose.onNodeWithContentDescription("Create quick task").performClick()
        compose.waitUntil(15_000) {
            vm.state.value.snapshot?.listing?.sections.orEmpty().flatMap { it.rows }.filterIsInstance<Row.Task>().any { it.row.title == "Native quick task" }
        }
        val task = runBlocking { repo.read { allTasks() } }.single()
        Assert.assertEquals("Native quick task", task.title)
        Assert.assertEquals(5_400_000.0, runBlocking { repo.detail(task.id) }!!.estimateMs, 0.01)
        Assert.assertTrue(runBlocking { repo.detail(task.id) }!!.tagIds.isNotEmpty())
        Assert.assertTrue(runBlocking { repo.read { tags() } }.any { it.title == "work" })
    }

    @Test fun longSharedSingleLineRetainsItsEntireSourceInDraft() {
        val source = "Long shared Unicode 📝 ".repeat(60)
        compose.runOnUiThread { vm.captureText(source) }
        compose.waitUntil(15_000) { vm.state.value.editor != null }
        Assert.assertEquals(source, vm.state.value.editor!!.notes)
        runBlocking { vm.persistDraft() }
        Assert.assertEquals(source, com.codedbydan.momentum.data.DraftStore(directory).read(saved.get<String>("editor-id")!!)!!.notes)
    }
}
