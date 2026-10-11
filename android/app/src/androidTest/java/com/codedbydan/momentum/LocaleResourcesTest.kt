// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import android.content.res.Configuration
import android.content.res.Resources
import androidx.activity.ComponentActivity
import androidx.compose.foundation.layout.Column
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.filters.SdkSuppress
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.ui.dayText
import com.codedbydan.momentum.ui.messageText
import org.junit.Assert.assertEquals
import org.junit.After
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.util.Locale

@RunWith(AndroidJUnit4::class)
@SdkSuppress(minSdkVersion = 33)
class LocaleResourcesTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()
    private lateinit var locales: NativeAppLocaleFixture

    @Before fun prepare() {
        locales = NativeAppLocaleFixture(compose.activity)
        locales.apply(Locale.US)
        compose.waitUntil(15_000) { locales.isApplied }
    }

    @After fun cleanup() {
        if(::locales.isInitialized) {
            locales.restore()
            compose.waitUntil(15_000) { locales.isApplied }
        }
    }

    private fun resources(locale: Locale): Resources = app.createConfigurationContext(
        Configuration(app.resources.configuration).apply { setLocale(locale) }).resources

    @Test fun composedDateAndFeedbackObserveLocaleChangesWithoutChangingContext() {
        compose.setContent {
            MaterialTheme {
                Column {
                    Text(dayText(DayLabel("2026-10-05", DayRelation.OTHER, 1u)))
                    Text(messageText(Message.MovedToDay("2026-10-05")))
                }
            }
        }
        compose.onNodeWithText("Oct 5, 2026").assertIsDisplayed()
        compose.onNodeWithText("Moved to Oct 5, 2026").assertIsDisplayed()
        locales.apply(Locale.GERMAN)
        compose.waitUntil(15_000) { locales.isApplied }
        compose.onNodeWithText("05.10.2026").assertIsDisplayed()
        compose.onNodeWithText("Nach 05.10.2026 verschoben").assertIsDisplayed()
        compose.onNodeWithText("Oct 5, 2026").assertDoesNotExist()
    }

    @Test fun quantityResourcesUseSingularAndPluralInEnglishAndGerman() {
        val english = resources(Locale.US)
        val german = resources(Locale.GERMAN)
        val examples = listOf(
            R.plurals.task_selection_count to listOf("1 task selected", "2 tasks selected", "1 Aufgabe ausgewählt", "2 Aufgaben ausgewählt"),
            R.plurals.reminder_minutes to listOf("1 minute before", "2 minutes before", "1 Minute vorher", "2 Minuten vorher"),
            R.plurals.pending_changes to listOf("1 pending change", "2 pending changes", "1 ausstehende Änderung", "2 ausstehende Änderungen"),
            R.plurals.summary_count to listOf("1 task planned for today", "2 tasks planned for today", "1 Aufgabe für heute geplant", "2 Aufgaben für heute geplant"),
            R.plurals.delete_project_scope to listOf(
                "This deletes the project and its 1 task, including subtasks.", "This deletes the project and its 2 tasks, including subtasks.",
                "Das Projekt und seine 1 Aufgabe einschließlich Unteraufgaben werden gelöscht.", "Das Projekt und seine 2 Aufgaben einschließlich Unteraufgaben werden gelöscht."),
        )
        for ((id, expected) in examples) {
            assertEquals(expected[0], english.getQuantityString(id, 1, 1))
            assertEquals(expected[1], english.getQuantityString(id, 2, 2))
            assertEquals(expected[2], german.getQuantityString(id, 1, 1))
            assertEquals(expected[3], german.getQuantityString(id, 2, 2))
        }
        assertEquals("0 tasks selected", english.getQuantityString(R.plurals.task_selection_count, 0, 0))
        assertEquals("0 Aufgaben ausgewählt", german.getQuantityString(R.plurals.task_selection_count, 0, 0))
        assertEquals("Last index update: 12:00 (1 unfinished title)", english.getQuantityString(R.plurals.system_search_last_update, 1, "12:00", 1))
        assertEquals("Last index update: 12:00 (2 unfinished titles)", english.getQuantityString(R.plurals.system_search_last_update, 2, "12:00", 2))
        assertEquals("Letzte Indexaktualisierung: 12:00 (1 unfertiger Titel)", german.getQuantityString(R.plurals.system_search_last_update, 1, "12:00", 1))
        assertEquals("Letzte Indexaktualisierung: 12:00 (2 unfertige Titel)", german.getQuantityString(R.plurals.system_search_last_update, 2, "12:00", 2))
    }
}
