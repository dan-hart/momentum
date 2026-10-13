// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.*
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.viewModelScope
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.MomentumRepository
import com.codedbydan.momentum.ui.TaskList
import com.codedbydan.momentum.ui.RepeatEditor
import kotlinx.coroutines.cancel
import kotlinx.coroutines.runBlocking
import org.junit.*
import org.junit.runner.RunWith
import java.io.File
import java.time.LocalDate
import java.util.UUID

@RunWith(AndroidJUnit4::class)
class UpcomingPreviewTest {
    @get:Rule val compose = createComposeRule()
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()
    private lateinit var directory: File
    private lateinit var repository: MomentumRepository
    private lateinit var vm: MomentumViewModel

    @Before fun prepare() = runBlocking {
        directory = File(app.cacheDir, "upcoming-preview-${UUID.randomUUID()}")
        repository = MomentumRepository(app, directory)
        repository.read {
            val seed = createTaskWithId(TaskDraft("Recurring fixture", "", null, null, null,
                0.0, "", emptyList(), emptyList()), View.Today).id!!
            val repeat = repeatDraft(seed)!!
            saveRepeat(seed, repeat.copy(cycle = RepeatCycle.DAILY, every = 1u,
                startDate = LocalDate.now().plusDays(1).toString()))
            createTask(TaskDraft("Overdue fixture", "", LocalDate.now().minusDays(2).toString(), null, null,
                0.0, "", emptyList(), emptyList()), View.Today)
        }
        compose.runOnUiThread { vm = MomentumViewModel(app, SavedStateHandle(), repository, {}, {}) }
        compose.setContent { MaterialTheme {
            var configuration by remember { mutableStateOf<String?>(null) }
            TaskList(vm.state.collectAsState().value, vm, editRepeatSchedule = { configuration = it }) {}
            configuration?.let { id -> RepeatEditor(id, vm, configuration = true) { configuration = null; vm.refresh() } }
        } }
        compose.waitUntil(15_000) { vm.state.value.snapshot != null && !vm.state.value.loading }
    }

    @After fun cleanup() = runBlocking {
        if (::vm.isInitialized) compose.runOnUiThread { vm.viewModelScope.cancel() }
        if (::repository.isInitialized) repository.close()
        if (::directory.isInitialized) directory.deleteRecursively()
    }

    @Test fun previewsAreVisibleAndCannotBeCompletedOrBulkSelected() {
        compose.runOnUiThread { vm.navigate(View.Upcoming) }
        compose.waitUntil(15_000) { !vm.state.value.loading && vm.state.value.snapshot?.listing?.sections
            ?.flatMap { it.rows }?.filterIsInstance<Row.Task>()?.any { it.row.occurrence != null } == true }
        compose.onAllNodesWithText(app.getString(R.string.edit_repeat_schedule)).onFirst().assertIsDisplayed()
        compose.onAllNodes(isToggleable()).assertCountEquals(0)
        compose.runOnUiThread { vm.selectAll() }
        Assert.assertTrue("read-only occurrences cannot enter a bulk task command", vm.state.value.selected.isEmpty())
    }

    @Test fun overdueRowsHaveAnExplicitCalloutAlongsideTheirDate() {
        compose.onNodeWithText("Overdue fixture").assertIsDisplayed()
        compose.onNodeWithText(app.getString(R.string.overdue)).assertIsDisplayed()
    }

    @Test fun scheduleActionEditsTheConfigurationAndStoppingLeavesStoredTasksIntact() = runBlocking {
        val before = repository.read { allTasks().map { it.id }.toSet() }
        compose.runOnUiThread { vm.navigate(View.Upcoming) }
        compose.waitUntil(15_000) { !vm.state.value.loading && vm.state.value.view == View.Upcoming }
        compose.onAllNodesWithText(app.getString(R.string.edit_repeat_schedule)).onFirst().performClick()
        compose.waitUntil(5_000) { compose.onAllNodesWithText(app.getString(R.string.stop_repeat)).fetchSemanticsNodes().isNotEmpty() }
        compose.onNodeWithText(app.getString(R.string.stop_repeat)).performScrollTo().performClick()
        compose.waitUntil(15_000) { !vm.state.value.loading && vm.state.value.snapshot?.listing?.sections
            ?.flatMap { it.rows }?.filterIsInstance<Row.Task>()?.none { it.row.occurrence != null } == true }
        Assert.assertEquals(before, repository.read { allTasks().map { it.id }.toSet() })
    }
}
