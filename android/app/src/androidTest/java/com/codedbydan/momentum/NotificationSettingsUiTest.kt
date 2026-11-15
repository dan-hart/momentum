// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.lifecycle.SavedStateHandle
import androidx.test.core.app.ApplicationProvider
import com.codedbydan.momentum.core.TaskDraft
import com.codedbydan.momentum.core.View
import com.codedbydan.momentum.data.AppSettings
import com.codedbydan.momentum.data.MomentumRepository
import com.codedbydan.momentum.ui.MomentumTheme
import com.codedbydan.momentum.ui.SettingsScreen
import kotlinx.coroutines.runBlocking
import org.junit.Rule
import org.junit.Test
import java.io.File
import java.time.DayOfWeek
import java.time.LocalDate
import java.time.format.TextStyle
import java.util.UUID

class NotificationSettingsUiTest {
    @get:Rule val compose = createComposeRule()
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()

    @Test fun customDaysAndSectionsUpdatePreviewIndependentlyFromBadges(): Unit = runBlocking {
        val directory = File(app.cacheDir, "notification-options-ui-${UUID.randomUUID()}")
        val repo = MomentumRepository(app, directory)
        lateinit var vm: MomentumViewModel
        try {
            repo.create(TaskDraft("An overdue task", "INBOX_PROJECT", LocalDate.now().minusDays(1).toString(),
                null, null, 0.0, "", emptyList(), emptyList()), View.Today)
            compose.runOnUiThread {
                vm = MomentumViewModel(app, SavedStateHandle(), repo, {}, {})
                vm.settingsPage("notifications")
            }
            compose.setContent {
                val state by vm.state.collectAsState()
                MomentumTheme(state.snapshot?.settings ?: AppSettings()) { SettingsScreen(state, vm) }
            }
            compose.waitUntil(10_000) { vm.state.value.snapshot != null }
            compose.onNodeWithContentDescription(app.getString(R.string.summary)).performScrollTo().assertIsOff().performClick()
            compose.waitUntil(10_000) { vm.state.value.snapshot?.preferences?.morningSummaryEnabled == true }
            compose.onNodeWithText(app.getString(R.string.summary_days_option, app.getString(R.string.summary_every_day)))
                .performScrollTo().performClick()
            compose.onNodeWithText(app.getString(R.string.summary_weekdays)).performClick()
            val locale = app.resources.configuration.locales[0]
            val friday = DayOfWeek.FRIDAY.getDisplayName(TextStyle.FULL_STANDALONE, locale)
            compose.onNodeWithText(friday).performScrollTo().assertIsOn().performClick()
            compose.onNodeWithText(app.getString(R.string.save)).performClick()
            compose.waitUntil(10_000) { vm.state.value.snapshot?.preferences?.morningSummaryWeekdays == 15u.toUByte() }

            compose.onNodeWithText(app.getString(R.string.overdue)).performScrollTo().assertIsOff().performClick()
            compose.waitUntil(10_000) { vm.state.value.snapshot?.preferences?.morningSummarySections == 15u.toUByte() }
            compose.waitUntil(10_000) {
                compose.onAllNodesWithText(app.getString(R.string.summary_overdue_count, 1L), substring = true)
                    .fetchSemanticsNodes().isNotEmpty()
            }
            compose.onNodeWithText(app.getString(R.string.summary_overdue_count, 1L), substring = true)
                .performScrollTo().assertIsDisplayed()

            compose.onNodeWithContentDescription(app.getString(R.string.badge_task_reminders))
                .performScrollTo().assertIsOn().performClick()
            compose.waitUntil(10_000) { vm.state.value.snapshot?.settings?.badgeTaskReminders == false }
            compose.onNodeWithContentDescription(app.getString(R.string.badge_morning_summary)).performScrollTo().assertIsOn()
            compose.onNodeWithContentDescription(app.getString(R.string.summary)).performScrollTo().assertIsOn()
        } finally {
            vm.platform.foreground(false)
            repo.close()
            directory.deleteRecursively()
        }
    }
}
