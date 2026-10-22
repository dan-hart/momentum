// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import android.content.res.Configuration
import androidx.activity.ComponentActivity
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalResources
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.unit.Density
import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.viewModelScope
import androidx.test.core.app.ApplicationProvider
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.MomentumRepository
import com.codedbydan.momentum.ui.MomentumApp
import kotlinx.coroutines.cancel
import kotlinx.coroutines.runBlocking
import org.junit.*
import java.io.File
import java.time.LocalDate
import java.util.Locale
import java.util.UUID

class DayPeriodSectionsTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()
    private lateinit var repository: MomentumRepository
    private lateinit var directory: File
    private lateinit var vm: MomentumViewModel
    private lateinit var localized: android.content.Context

    @Before fun prepare() {
        directory = File(app.cacheDir, "day-period-${UUID.randomUUID()}")
        repository = MomentumRepository(app, directory)
    }
    @After fun cleanup() = runBlocking {
        if(::vm.isInitialized) compose.runOnUiThread { vm.viewModelScope.cancel() }
        repository.close(); directory.deleteRecursively(); Unit
    }
    private fun create(title: String, view: View) = runBlocking {
        repository.create(TaskDraft(title, "INBOX_PROJECT", LocalDate.now().toString(), null, null, 0.0, "", emptyList(), emptyList()), view).id!!
    }
    private fun show(locale: Locale = Locale.ENGLISH, scale: Float = 1f) {
        localized = compose.activity.createConfigurationContext(Configuration(app.resources.configuration).apply {
            setLocale(locale); fontScale = scale
        })
        compose.runOnUiThread { vm = MomentumViewModel(app, SavedStateHandle(), repository, {}, {}) }
        compose.setContent {
            CompositionLocalProvider(LocalResources provides localized.resources,
                LocalConfiguration provides localized.resources.configuration, LocalDensity provides Density(localized)) {
                MomentumApp(vm)
            }
        }
        compose.waitUntil(15_000) { !vm.state.value.loading && vm.state.value.snapshot != null }
    }
    @Test fun populatedSectionsHaveOnlyInteriorDecorativeDividers() {
        create("Morning row", View.Morning); create("Today row", View.Today); create("Evening row", View.Tonight)
        show()
        compose.onNodeWithTag("day-period-separator-1", useUnmergedTree = true).assertExists()
            .assert(SemanticsMatcher.expectValue(SemanticsProperties.HideFromAccessibility, Unit))
        compose.onNodeWithTag("day-period-separator-2", useUnmergedTree = true).assertExists()
        compose.onNodeWithTag("day-period-separator-0", useUnmergedTree = true).assertDoesNotExist()
        compose.runOnUiThread { vm.preferences { it.copy(groupBy = GroupBy.NONE) } }
        compose.waitUntil(15_000) { vm.state.value.snapshot?.preferences?.groupBy == GroupBy.NONE }
        compose.onNodeWithTag("day-period-separator-1", useUnmergedTree = true).assertDoesNotExist()
    }
    @Test fun missingMiddleGroupGetsOneDividerAndToggleClearsHiddenSelection() {
        val morning = create("Morning selected", View.Morning)
        create("Evening selected", View.Tonight)
        show()
        compose.onNodeWithTag("day-period-separator-1", useUnmergedTree = true).assertExists()
            .assert(SemanticsMatcher.expectValue(SemanticsProperties.HideFromAccessibility, Unit))
        compose.onNodeWithTag("day-period-separator-2", useUnmergedTree = true).assertDoesNotExist()
        compose.runOnUiThread { vm.select(morning) }
        compose.onNodeWithContentDescription(localized.getString(R.string.options)).performClick()
        compose.onNodeWithContentDescription("Show Morning and Evening tasks in Today").performScrollTo().performClick()
        compose.waitUntil(15_000) { vm.state.value.snapshot?.preferences?.showDayPeriodTasksInToday == false }
        compose.runOnIdle { Assert.assertTrue(vm.state.value.selected.isEmpty()) }
        compose.onNodeWithText(localized.getString(R.string.close)).performClick()
        compose.onNodeWithText("Morning selected").assertDoesNotExist()
        compose.onNodeWithText("Evening selected").assertDoesNotExist()
    }
    @Test fun englishSettingsToggleWrapsAtLargeText() = settingsToggleWraps(Locale.ENGLISH, "Show Morning and Evening tasks in Today")
    @Test fun germanSettingsToggleWrapsAtLargeText() = settingsToggleWraps(Locale.GERMAN, "Morgen- und Abendaufgaben in Heute anzeigen")
    private fun settingsToggleWraps(locale: Locale, label: String) {
        show(locale, 2f)
        compose.runOnUiThread { vm.tab(Tab.Settings) }
        val text = compose.onNodeWithText(label).performScrollTo().assertIsDisplayed()
        val layouts = mutableListOf<TextLayoutResult>()
        text.performSemanticsAction(SemanticsActions.GetTextLayoutResult) { it(layouts) }
        Assert.assertFalse("Full toggle label must wrap without ellipsis", layouts.single().hasVisualOverflow)
        compose.onNodeWithContentDescription(label).performScrollTo().assertIsOn().performClick()
        compose.waitUntil(15_000) { vm.state.value.snapshot?.preferences?.showDayPeriodTasksInToday == false }
        compose.onNodeWithContentDescription(label).assertIsOff()
    }
}
