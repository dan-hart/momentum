// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import android.os.SystemClock
import android.view.InputDevice
import android.view.MotionEvent
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.lifecycle.SavedStateHandle
import androidx.test.core.app.ApplicationProvider
import androidx.test.platform.app.InstrumentationRegistry
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.MomentumRepository
import com.codedbydan.momentum.ui.MomentumApp
import kotlinx.coroutines.runBlocking
import org.junit.*
import java.io.File
import java.util.UUID

/** Real framework touch input crosses Compose's native drag boundary into Rust. */
class NativeTaskDragTest {
    @get:Rule val compose = createComposeRule()
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()
    private lateinit var directory: File
    private lateinit var repo: MomentumRepository
    private lateinit var vm: MomentumViewModel

    @Before fun prepare() {
        directory = File(app.cacheDir, "native-drag-${UUID.randomUUID()}")
        repo = MomentumRepository(app, directory)
    }
    @After fun cleanup() = runBlocking { repo.close(); directory.deleteRecursively(); Unit }
    private fun present() {
        compose.runOnUiThread { vm = MomentumViewModel(app, SavedStateHandle(), repo, {}, {}) }
        compose.setContent { MomentumApp(vm) }
        compose.waitUntil(15_000) { !vm.state.value.loading }
        compose.runOnUiThread { vm.navigate(View.Project("INBOX_PROJECT")) }
        compose.waitUntil(15_000) { !vm.state.value.loading && vm.state.value.snapshot != null }
    }
    private fun rows() = vm.state.value.snapshot!!.listing.sections.flatMap { it.rows }.filterIsInstance<Row.Task>().map { it.row }
    private fun select(first: TaskRow, second: TaskRow) {
        compose.onNodeWithText(first.title).performTouchInput { longClick() }
        compose.waitUntil(10_000) { first.id in vm.state.value.selected }
        compose.onNodeWithText(second.title).performClick()
        compose.waitUntil(10_000) { vm.state.value.selected == setOf(first.id, second.id) }
    }
    private fun drag(source: SemanticsNodeInteraction, destination: SemanticsNodeInteraction, checkHover: Boolean = false) {
        source.assertIsDisplayed(); destination.assertIsDisplayed()
        val start = source.fetchSemanticsNode().boundsInWindow.center
        val end = destination.fetchSemanticsNode().boundsInWindow.center
        val automation = InstrumentationRegistry.getInstrumentation().uiAutomation
        val down = SystemClock.uptimeMillis()
        fun send(action: Int, x: Float, y: Float) {
            val event = MotionEvent.obtain(down, SystemClock.uptimeMillis(), action, x, y, 0)
            event.source = InputDevice.SOURCE_TOUCHSCREEN
            try { Assert.assertTrue("Native touch input delivered", automation.injectInputEvent(event, true)) }
            finally { event.recycle() }
        }
        send(MotionEvent.ACTION_DOWN, start.x, start.y)
        try {
            // Input is delivered by Android, while the Compose test environment
            // owns coroutine time. Advance its long-press timer as well as real
            // elapsed time so this tests the shipping gesture rather than a
            // suspended test-clock timeout.
            compose.mainClock.advanceTimeBy(700)
            compose.waitForIdle()
            SystemClock.sleep(700)
            for(step in 1..12) {
                val fraction = step / 12f
                send(MotionEvent.ACTION_MOVE, start.x + (end.x - start.x) * fraction, start.y + (end.y - start.y) * fraction)
                SystemClock.sleep(25)
            }
            compose.mainClock.advanceTimeByFrame()
            compose.waitForIdle()
            if(checkHover) destination.assertIsSelected()
        } finally { send(MotionEvent.ACTION_UP, end.x, end.y) }
    }
    private fun undo() {
        compose.waitUntil(10_000) { vm.state.value.notice?.undo != null && !vm.state.value.busy }
        compose.onNodeWithText(app.getString(R.string.undo)).performClick()
        compose.waitUntil(15_000) { !vm.state.value.busy && !vm.state.value.loading }
    }

    @Test fun selectedTaskFamilyAndCompanionMoveByTouchToProjectThenUndo() {
        val destination = runBlocking { repo.read { addProject("Native drag destination")!! } }
        val parent = runBlocking { repo.read { addTask("Drag parent", View.Project("INBOX_PROJECT")); allTasks().single().id } }
        runBlocking { repo.mutate { addSubtask(parent, "Drag child"); addTask("Drag companion", View.Project("INBOX_PROJECT")) } }
        present()
        val initial = rows()
        select(initial.single { it.title == "Drag parent" }, initial.single { it.title == "Drag companion" })
        val chip = compose.onNode(hasText("Native drag destination") and hasAnyAncestor(hasTestTag("task-destinations"))).performScrollTo()
        drag(compose.onNodeWithText("Drag parent"), chip, checkHover = true)
        compose.waitUntil(15_000) { !vm.state.value.busy && !vm.state.value.selecting }
        val moved = runBlocking { repo.read { allTasks().map { taskDetail(it.id)!! } } }
        Assert.assertEquals(3, moved.size)
        Assert.assertTrue("Native project drop moves the complete parent family and companion", moved.all { it.projectId == destination })
        Assert.assertEquals(parent, moved.single { it.title == "Drag child" }.parentId)
        undo()
        Assert.assertTrue(runBlocking { repo.read { allTasks().all { taskDetail(it.id)!!.projectId == "INBOX_PROJECT" } } })
    }

    @Test fun selectedRowsReorderInVisibleOrderByTouchThenOneUndoRestoresThem() {
        runBlocking { repo.mutate { addFromText("Reorder alpha\nReorder beta\nReorder gamma\nReorder delta", View.Project("INBOX_PROJECT")) } }
        present()
        val original = rows()
        Assert.assertEquals(4, original.size)
        val selected = original.takeLast(2)
        select(selected.first(), selected.last())
        drag(compose.onNodeWithText(selected.first().title), compose.onNodeWithText(original.first().title))
        val expected = selected.map { it.id } + original.dropLast(2).map { it.id }
        compose.waitUntil(15_000) { !vm.state.value.busy && !vm.state.value.loading && rows().map { it.id } == expected }
        Assert.assertEquals(expected, rows().map { it.id })
        undo()
        // The native Snackbar invokes a coroutine; an initially idle snapshot
        // can precede the actual Undo projection. Wait for its exact row result.
        compose.waitUntil(15_000) { !vm.state.value.loading && rows().map { it.id } == original.map { it.id } }
        Assert.assertEquals(original.map { it.id }, rows().map { it.id })
    }
}
