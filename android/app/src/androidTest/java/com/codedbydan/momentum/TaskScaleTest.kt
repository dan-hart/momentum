// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import android.os.SystemClock
import android.os.Build
import android.util.Log
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.ViewModelStore
import androidx.test.core.app.ApplicationProvider
import androidx.test.platform.app.InstrumentationRegistry
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.MomentumRepository
import com.codedbydan.momentum.ui.MomentumApp
import kotlinx.coroutines.runBlocking
import org.junit.*
import java.io.File
import java.util.UUID

/** Functional scale coverage and a separate opt-in observation, never shipping cold-launch timings. */
class TaskScaleTest {
    @get:Rule val compose = createComposeRule()

    @Test fun thousandTasksLoadScrollAndEditThroughTheNativeCore() = runScale(observeTimings = false)

    @Test fun observeThousandTaskCompleteProjectionAndNativeFixtureTimings() {
        val arguments = InstrumentationRegistry.getArguments()
        Assume.assumeTrue("Performance observation requires a separately opted-in disposable emulator",
            arguments.getString("momentum.performanceTests") == "true" &&
                arguments.getString("momentum.disposableOsTests") == "true" &&
                Build.HARDWARE in listOf("ranchu", "goldfish"))
        runScale(observeTimings = true)
    }

    private fun runScale(observeTimings: Boolean) {
        val app = ApplicationProvider.getApplicationContext<MomentumApplication>()
        val directory = File(app.cacheDir, "task-scale-${UUID.randomUUID()}")
        var repo = MomentumRepository(app, directory)
        val models = ViewModelStore()
        try {
            val titles = (0 until 1000).map { "Scale fixture %04d".format(it) }
            val source = titles.joinToString("\n")
            runBlocking {
                Assert.assertTrue(repo.mutate { addFromText(source, View.Today) }.changed)
                Assert.assertEquals(1000, repo.read { allTasks() }.size)
                repo.close()
            }
            repo = MomentumRepository(app, directory)
            val start = SystemClock.elapsedRealtimeNanos()
            val snapshot = runBlocking { repo.snapshot(View.Today) }
            val firstSnapshotMs = (SystemClock.elapsedRealtimeNanos() - start) / 1_000_000.0
            val rows = snapshot.listing.sections.flatMap { it.rows }.filterIsInstance<Row.Task>()
            Assert.assertEquals(1000, rows.size)
            Assert.assertEquals(titles.toSet(), rows.map { it.row.title }.toSet())
            Assert.assertEquals(1000, rows.map { it.row.id }.distinct().size)
            Assert.assertEquals(1000, snapshot.listing.sections.sumOf { it.count.toInt() })
            // Rust deliberately leaves built-in sidebar counts at zero; project entries carry
            // unfinished root-family counts. This isolated Today import belongs to Inbox.
            Assert.assertEquals(1000u, snapshot.sidebar.projects.single { it.view == View.Project("INBOX_PROJECT") }.taskCount)
            Assert.assertEquals(1000u, runBlocking { repo.read { taskCount(TaskCountMode.DUE_TODAY) } })
            lateinit var vm: MomentumViewModel
            val readyStart = SystemClock.elapsedRealtimeNanos()
            compose.runOnUiThread {
                vm = MomentumViewModel(app, SavedStateHandle(), repo, {}, {})
                models.put("isolated-scale", vm)
            }
            compose.setContent { MomentumApp(vm) }
            compose.waitUntil(30_000) { !vm.state.value.loading && !vm.state.value.editorLoading && vm.state.value.snapshot != null }
            Assert.assertEquals(1000, vm.state.value.snapshot!!.listing.sections.sumOf { section -> section.rows.count { it is Row.Task } })
            compose.onNodeWithText(rows.first().row.title).assertIsDisplayed()
            val readyMs = (SystemClock.elapsedRealtimeNanos() - readyStart) / 1_000_000.0
            // The drawer is another indexed lazy list, including while off screen.
            // Keep the task-list identity stable as its visible children change.
            val taskList = compose.onNodeWithTag("task-list")
            val scrollStart = SystemClock.elapsedRealtimeNanos()
            taskList.performScrollToNode(hasText(rows.last().row.title))
            compose.onNodeWithText(rows.last().row.title).assertIsDisplayed().performClick()
            compose.waitUntil(15_000) { !vm.state.value.editorLoading && vm.state.value.editor?.id == rows.last().row.id }
            val scrollAndOpenMs = (SystemClock.elapsedRealtimeNanos() - scrollStart) / 1_000_000.0
            compose.onNode(hasSetTextAction() and hasText(rows.last().row.title)).performTextReplacement("Edited scale fixture")
            compose.onNodeWithContentDescription(app.getString(R.string.save)).performClick()
            compose.waitUntil(15_000) { vm.state.value.editor == null && !vm.state.value.busy && !vm.state.value.loading }
            Assert.assertEquals("Edited scale fixture", runBlocking { repo.detail(rows.last().row.id) }!!.title)
            Assert.assertEquals(1000, runBlocking { repo.read { allTasks() } }.size)
            compose.runOnUiThread { models.clear() }
            runBlocking { repo.close() }
            repo = MomentumRepository(app, directory)
            Assert.assertEquals("Edited scale fixture", runBlocking { repo.detail(rows.last().row.id) }!!.title)
            val reopened = runBlocking { repo.snapshot(View.Today) }
            Assert.assertEquals(1000, reopened.listing.sections.sumOf { section -> section.rows.count { it is Row.Task } })
            Assert.assertEquals(1000u, reopened.sidebar.projects.single { it.view == View.Project("INBOX_PROJECT") }.taskCount)
            Assert.assertEquals(1000u, runBlocking { repo.read { taskCount(TaskCountMode.DUE_TODAY) } })
            if(observeTimings) Log.i("MomentumScale", "scope=isolated-complete-projection-and-production-compose-fixture tasks=1000 " +
                "firstSnapshotMs=$firstSnapshotMs firstListPaintMs=$readyMs scrollAndOpenMs=$scrollAndOpenMs persistedAfterReopen=true")
        } finally {
            compose.runOnUiThread { models.clear() }
            runBlocking { repo.close() }
            directory.deleteRecursively()
        }
    }
}
