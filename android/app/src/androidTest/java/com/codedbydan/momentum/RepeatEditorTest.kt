// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.semantics.SemanticsActions
import androidx.lifecycle.SavedStateHandle
import androidx.test.core.app.ApplicationProvider
import androidx.test.platform.app.InstrumentationRegistry
import android.view.KeyEvent
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.MomentumRepository
import com.codedbydan.momentum.ui.*
import kotlinx.coroutines.runBlocking
import org.junit.*
import java.io.File
import java.time.DayOfWeek
import java.time.format.TextStyle
import java.time.temporal.WeekFields
import java.util.UUID

class RepeatEditorTest {
    @get:Rule val compose = createComposeRule()
    private lateinit var repo: MomentumRepository
    private lateinit var directory: File
    private lateinit var id: String
    private lateinit var vm: MomentumViewModel
    private var dismissed = false
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()

    @Before fun prepare() {
        directory = File(app.cacheDir, "repeat-editor-${UUID.randomUUID()}")
        repo = MomentumRepository(app, directory)
        id = runBlocking { repo.create(TaskDraft("Repeating fixture", "INBOX_PROJECT", null, null, null,
            0.0, "", emptyList(), emptyList()), View.Today).id!! }
        compose.runOnUiThread { vm = MomentumViewModel(app, SavedStateHandle(), repo, {}, {}) }
        compose.setContent { MomentumTheme(repoSettings()) { RepeatEditor(id, vm) { dismissed = true } } }
        compose.waitUntil(15_000) { compose.onAllNodesWithText(app.getString(R.string.every)).fetchSemanticsNodes().isNotEmpty() }
    }
    private fun repoSettings() = com.codedbydan.momentum.data.AppSettings()
    @After fun cleanup() { runBlocking { repo.close() }; directory.deleteRecursively() }

    @Test fun invalidIntervalAndEmptyWeeklySelectionKeepTheEditorAndTaskUnchanged() {
        compose.onNodeWithText(app.getString(R.string.every)).performTextReplacement("0")
        compose.onNodeWithText(app.getString(R.string.save)).assertIsNotEnabled()
        compose.onNodeWithText(app.getString(R.string.repeat_interval_invalid)).assertExists()
        compose.onNodeWithText(app.getString(R.string.every)).performTextReplacement("1")
        val before = runBlocking { repo.read { repeatDraft(id)!! } }
        before.weekdays.forEachIndexed { index, selected -> if(selected) {
            val label = DayOfWeek.of(if(index == 0) 7 else index).getDisplayName(TextStyle.SHORT, app.resources.configuration.locales[0])
            compose.onNodeWithText(label).performScrollTo().performClick()
        } }
        compose.onNodeWithText(app.getString(R.string.save)).performClick()
        compose.onNodeWithText(app.getString(R.string.pick_weekday)).assertExists()
        // A chip accepts keyboard focus in Android's keyboard input mode.
        // Exercise the native key path rather than require focus after a touch tap.
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        instrumentation.sendKeyDownUpSync(KeyEvent.KEYCODE_TAB)
        compose.onNodeWithText(app.getString(R.string.save)).performSemanticsAction(SemanticsActions.RequestFocus) { it() }
        compose.onNodeWithText(app.getString(R.string.save)).assertIsFocused()
        instrumentation.sendKeyDownUpSync(KeyEvent.KEYCODE_ENTER)
        compose.waitForIdle()
        val firstWeekday = WeekFields.of(app.resources.configuration.locales[0]).firstDayOfWeek
            .getDisplayName(TextStyle.FULL, app.resources.configuration.locales[0])
        compose.onNodeWithContentDescription(firstWeekday).assertIsFocused()
        Assert.assertFalse(dismissed)
        Assert.assertNull(runBlocking { repo.detail(id)!!.repeat })
    }

    @Test fun monthlyLastWeekdayRadioAndSaveFailureRetainTheChosenRuleForRetry() {
        compose.onNodeWithText(app.getString(R.string.monthly)).performClick()
        compose.onNodeWithText(app.getString(R.string.nth_weekday)).performScrollTo().performClick()
        compose.onNodeWithText(app.getString(R.string.repeat_second_week)).performScrollTo().assertExists()
        compose.onNodeWithText(app.getString(R.string.repeat_last_week)).performScrollTo().performClick()
        val friday = DayOfWeek.FRIDAY.getDisplayName(TextStyle.SHORT, app.resources.configuration.locales[0])
        compose.onNodeWithText(friday).performScrollTo().performClick()
        compose.onNodeWithContentDescription(app.getString(R.string.date)).performScrollTo().performClick()
        compose.onNodeWithText("Select date").assertExists()
        compose.onAllNodesWithText(app.getString(R.string.cancel)).onLast().performClick()
        val blocker = File(directory, ".momentum-transaction").also { it.writeText("invalid isolated journal") }
        compose.onNodeWithText(app.getString(R.string.save)).performClick()
        compose.waitUntil(15_000) { vm.state.value.error != null }
        compose.onNodeWithText(app.getString(R.string.save_failed)).assertExists()
        Assert.assertFalse(dismissed)
        Assert.assertNull(runBlocking { repo.detail(id)!!.repeat })
        Assert.assertTrue(blocker.delete())
        compose.onNodeWithText(app.getString(R.string.save)).performClick()
        compose.waitUntil(15_000) { dismissed }
        val saved = runBlocking { repo.read { repeatDraft(id)!! } }
        Assert.assertEquals(RepeatCycle.MONTHLY, saved.cycle)
        Assert.assertEquals(MonthlyRule.NthWeekday(-1, 5u), saved.monthly)
        Assert.assertTrue(saved.existing)
    }
}
