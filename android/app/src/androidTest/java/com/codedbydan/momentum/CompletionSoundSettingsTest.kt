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

    @Test fun nativePickerAndExplicitPreviewAreDisabledOffAndSelectionDoesNotPlay() {
        var settings by mutableStateOf(AppSettings())
        var previews = 0
        compose.setContent {
            MomentumTheme(settings) { Column {
                CompletionSoundSettings(settings, { change -> settings = change(settings) }, { previews++ })
            } }
        }
        val preview = compose.onNodeWithText(app.getString(R.string.completion_sound_preview))
        val selected = { compose.onNodeWithText(app.getString(R.string.completion_sound_choice) + ": " + app.getString(settings.completionSoundChoice.label)) }
        preview.assertIsNotEnabled()
        selected().assertIsNotEnabled()
        compose.onNodeWithContentDescription(app.getString(R.string.completion_sound_enabled)).performClick()
        preview.assertIsEnabled().assertHeightIsAtLeast(48.dp)
        selected().assertIsEnabled().assertHeightIsAtLeast(48.dp)
        for(choice in CompletionSoundChoice.entries) {
            selected().performClick()
            compose.onNodeWithText(app.getString(choice.label)).performScrollTo().assertHeightIsAtLeast(48.dp).performClick()
            compose.runOnIdle { assertEquals(choice, settings.completionSoundChoice); assertEquals(0, previews) }
        }
        preview.performClick()
        compose.runOnIdle { assertEquals(1, previews) }
        compose.onNodeWithContentDescription(app.getString(R.string.completion_sound_enabled)).performClick()
        preview.assertIsNotEnabled(); selected().assertIsNotEnabled()
        compose.runOnIdle { assertEquals(CompletionSoundChoice.WARM_SPARK, settings.completionSoundChoice) }
        compose.onNodeWithContentDescription(app.getString(R.string.completion_sound_enabled)).performClick()
        selected().assertIsEnabled()
        compose.runOnIdle { assertEquals(CompletionSoundChoice.WARM_SPARK, settings.completionSoundChoice); assertEquals(1, previews) }
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
            assertEquals(labels[index], CompletionSoundChoice.entries.map { localized.getString(it.label) })
        }
    }
}
