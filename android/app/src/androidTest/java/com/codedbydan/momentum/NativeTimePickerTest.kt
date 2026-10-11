// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import android.view.View
import android.widget.TimePicker
import android.os.SystemClock
import android.view.accessibility.AccessibilityNodeInfo
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.lifecycle.SavedStateHandle
import androidx.test.core.app.ApplicationProvider
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.espresso.Espresso.onView
import androidx.test.espresso.UiController
import androidx.test.espresso.ViewAction
import androidx.test.espresso.action.ViewActions.click
import androidx.test.espresso.assertion.ViewAssertions.matches
import androidx.test.espresso.matcher.ViewMatchers.*
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.EditorDraft
import com.codedbydan.momentum.data.MomentumRepository
import com.codedbydan.momentum.ui.MomentumApp
import kotlinx.coroutines.runBlocking
import org.hamcrest.Matcher
import org.junit.*
import java.io.File
import java.util.UUID

/** Android's actual clock dialog, followed by the generated Rust task boundary. */
class NativeTimePickerTest {
    @get:Rule val compose = createComposeRule()
    private lateinit var repo: MomentumRepository
    private lateinit var directory: File
    private lateinit var vm: MomentumViewModel
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()
    @Before fun prepare() {
        directory = File(app.cacheDir, "native-clock-${UUID.randomUUID()}")
        repo = MomentumRepository(app, directory)
        compose.runOnUiThread { vm = MomentumViewModel(app, SavedStateHandle(), repo, {}, {}) }
        compose.setContent { MomentumApp(vm) }
        compose.waitUntil(15_000) { !vm.state.value.loading }
    }
    @After fun cleanup() = runBlocking { repo.close(); directory.deleteRecursively(); Unit }

    @Test fun frameworkClockConfirmsCanonicalTimeAndPreservesReminderDateAndNotes() {
        val day = "2026-12-31"
        compose.runOnUiThread { vm.newTask(); vm.setEditor(EditorDraft(title = "Native clock fixture",
            time = "2pm", dueDay = day, reminder = 5, notes = "Unrelated note stays intact")) }
        compose.onNodeWithContentDescription(app.getString(R.string.choose_time)).performScrollTo().performClick()
        awaitNativeClock()
        onView(isAssignableFrom(TimePicker::class.java)).check(matches(isDisplayed())).perform(object : ViewAction {
            override fun getConstraints(): Matcher<View> = isAssignableFrom(TimePicker::class.java)
            override fun getDescription() = "Verify the parsed initial native clock value"
            override fun perform(controller: UiController, view: View) {
                val clock = view as TimePicker
                Assert.assertEquals("Rust parses the existing 2pm field for the native picker", 14, clock.hour)
                Assert.assertEquals(0, clock.minute)
                controller.loopMainThreadUntilIdle()
            }
        })
        // Exercise the framework clock's actual accessible hour/minute actions.
        // Direct property setters bypass its header/radial input coordination on API26.
        clickClockValue(if(android.text.format.DateFormat.is24HourFormat(app)) "17" else "5")
        // API26's radial accessibility action changes the hour without advancing
        // the dial. Select the native minute header before its own dial action.
        onView(withResourceName("minutes")).perform(click())
        clickClockValue("45")
        onView(isAssignableFrom(TimePicker::class.java)).perform(object : ViewAction {
            override fun getConstraints(): Matcher<View> = isAssignableFrom(TimePicker::class.java)
            override fun getDescription() = "Verify the selected native clock value"
            override fun perform(controller: UiController, view: View) {
                val clock = view as TimePicker
                Assert.assertEquals("Native clock has committed the selected hour", 17, clock.hour)
                Assert.assertEquals(45, clock.minute)
            }
        })
        onView(withId(android.R.id.button1)).perform(click())
        compose.runOnIdle {
            val draft = vm.state.value.editor!!
            Assert.assertEquals("17:45", draft.time)
            Assert.assertEquals(day, draft.dueDay)
            Assert.assertEquals(5L, draft.reminder)
            Assert.assertEquals("Unrelated note stays intact", draft.notes)
        }
        compose.onNodeWithContentDescription(app.getString(R.string.save)).performClick()
        compose.waitUntil(15_000) { vm.state.value.editor == null && !vm.state.value.loading }
        val task = runBlocking { val id = repo.read { allTasks() }.single().id; repo.detail(id) }!!
        Assert.assertEquals(day, task.dueDay)
        Assert.assertEquals(ClockTime(17u, 45u), task.time)
        Assert.assertEquals(5u, task.reminderMinutesBefore)
        Assert.assertEquals("Unrelated note stays intact", task.notes)
    }

    @Test fun nativeClockCancelLeavesTheUncommittedDraftUntouched() {
        compose.runOnUiThread { vm.newTask(); vm.setEditor(EditorDraft(title = "Cancelled clock", time = "14:30", reminder = 10)) }
        val before = vm.state.value.editor
        compose.onNodeWithContentDescription(app.getString(R.string.choose_time)).performScrollTo().performClick()
        awaitNativeClock()
        onView(isAssignableFrom(TimePicker::class.java)).check(matches(isDisplayed()))
        onView(withId(android.R.id.button2)).perform(click())
        compose.runOnIdle { Assert.assertEquals(before, vm.state.value.editor) }
        Assert.assertTrue(runBlocking { repo.read { allTasks() } }.isEmpty())
    }

    private fun awaitNativeClock() {
        fun visibleClock(node: AccessibilityNodeInfo?): Boolean {
            if(node == null) return false
            if(node.className == TimePicker::class.java.name && node.isVisibleToUser) return true
            return (0 until node.childCount).any { visibleClock(node.getChild(it)) }
        }
        // Rust parses the current value on IO before the framework dialog opens.
        // Espresso cannot idle that repository coroutine on its own.
        val automation = InstrumentationRegistry.getInstrumentation().uiAutomation
        compose.waitUntil(15_000) { visibleClock(automation.rootInActiveWindow) }
    }

    private fun clickClockValue(value: String) {
        fun find(node: AccessibilityNodeInfo?): AccessibilityNodeInfo? {
            if(node == null) return null
            if(node.contentDescription?.toString() == value &&
                node.actionList.any { it.id == AccessibilityNodeInfo.ACTION_CLICK }) return node
            for(index in 0 until node.childCount) find(node.getChild(index))?.let { return it }
            return null
        }
        val automation = InstrumentationRegistry.getInstrumentation().uiAutomation
        val end = SystemClock.elapsedRealtime() + 5_000
        while(SystemClock.elapsedRealtime() < end) {
            val node = find(automation.rootInActiveWindow)
            if(node != null && node.performAction(AccessibilityNodeInfo.ACTION_CLICK)) return
            SystemClock.sleep(20)
        }
        Assert.fail("Native clock exposes an accessible selectable value: $value")
    }
}
