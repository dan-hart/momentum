// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import android.content.res.Configuration
import androidx.compose.foundation.layout.Column
import androidx.compose.runtime.*
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.unit.dp
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.codedbydan.momentum.data.AppSettings
import com.codedbydan.momentum.platform.CompletionSoundChoice
import com.codedbydan.momentum.ui.CompletionSoundSettings
import com.codedbydan.momentum.ui.MomentumTheme
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.util.Locale

@RunWith(AndroidJUnit4::class)
class CompletionSoundSettingsTest {
    @get:Rule val compose = createComposeRule()
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()

    @Test fun candidatesCanBeAuditionedBeforeSelectionWithAutomaticFeedbackOff() {
        var settings by mutableStateOf(AppSettings())
        val previews = mutableListOf<CompletionSoundChoice>()
        compose.setContent { MomentumTheme(settings) { Column {
            CompletionSoundSettings(settings, { change -> settings = change(settings) }, previews::add, {})
        } } }
        val selected = { compose.onNodeWithText(app.getString(R.string.completion_sound_choice) + ": " + app.getString(settings.completionSoundChoice.label)) }
        selected().assertIsEnabled().performClick()
        for(choice in CompletionSoundChoice.candidates) {
            compose.onNodeWithContentDescription(app.getString(R.string.completion_sound_preview) + ": " + app.getString(choice.label))
                .performScrollTo().performClick()
            compose.runOnIdle { assertFalse(settings.completionSound); assertEquals(CompletionSoundChoice.LITTLE_LIFT, settings.completionSoundChoice) }
        }
        compose.onNodeWithText(app.getString(CompletionSoundChoice.SOFT_KNOCK.label)).performScrollTo().performClick()
        compose.runOnIdle { assertEquals(CompletionSoundChoice.SOFT_KNOCK, settings.completionSoundChoice); assertEquals(10, previews.size) }
    }

    @Test fun catalogLabelsAreNumberedAndLocalizedInEnglishAndGerman() {
        val labels = listOf(
            listOf("1 Wood Pop", "2 Double Check", "3 Rounded Lift", "4 Pocket Piano", "5 String Flick", "6 Bright Finish",
                "7 Harp Snap", "8 Click and Glow", "9 Tiny Ta-Da", "10 Warm Spark"),
            listOf("1 Holzklang", "2 Doppeltes Häkchen", "3 Sanfter Aufstieg", "4 Taschenklavier", "5 Saitenzupfer", "6 Heller Abschluss",
                "7 Harfenzupfer", "8 Klick und Glanz", "9 Kleines Ta-Da", "10 Warmer Funke"),
        )
        for((index, locale) in listOf(Locale.US, Locale.GERMAN).withIndex()) {
            val localized = app.createConfigurationContext(Configuration(app.resources.configuration).apply { setLocale(locale) })
            assertEquals(labels[index], CompletionSoundChoice.entries.take(10).map { localized.getString(it.label) })
        }
    }
}
