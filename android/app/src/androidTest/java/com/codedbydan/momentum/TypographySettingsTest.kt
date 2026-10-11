// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import androidx.compose.material3.MaterialTheme
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.codedbydan.momentum.data.*
import com.codedbydan.momentum.ui.*
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.util.UUID

@RunWith(AndroidJUnit4::class)
class TypographySettingsTest {
    @get:Rule val compose = createComposeRule()

    @Test fun independentTypographyPersistsAndResetPreservesUnrelatedSettings() = runBlocking {
        val app = ApplicationProvider.getApplicationContext<MomentumApplication>()
        val directory = File(app.cacheDir, "typography-${UUID.randomUUID()}")
        var repository = MomentumRepository(app, directory)
        try {
            repository.updateSettings { it.copy(font = NativeFont.Monospace,
                contentTextScale = 1.5f, interfaceTextScale = 0.85f, compact = true) }
            repository.close()
            repository = MomentumRepository(app, directory)
            assertEquals(NativeFont.Monospace, repository.settings().font)
            assertEquals(1.5f, repository.settings().contentTextScale, 0f)
            assertEquals(0.85f, repository.settings().interfaceTextScale, 0f)
            repository.updateSettings { resetNativeTypography(it) }
            repository.close()
            repository = MomentumRepository(app, directory)
            assertEquals(NativeFont.System, repository.settings().font)
            assertEquals(1f, repository.settings().contentTextScale, 0f)
            assertEquals(1f, repository.settings().interfaceTextScale, 0f)
            assertTrue(repository.settings().compact)
        } finally { repository.close(); directory.deleteRecursively() }
    }

    @Test fun persistedOutOfRangeAndNonfiniteScalesRecoverIndependently() = runBlocking {
        val app = ApplicationProvider.getApplicationContext<MomentumApplication>()
        val directory = File(app.cacheDir, "typography-limits-${UUID.randomUUID()}")
        var repository = MomentumRepository(app, directory)
        try {
            repository.updateSettings { it.copy(contentTextScale = Float.NaN, interfaceTextScale = 9f) }
            repository.close()
            repository = MomentumRepository(app, directory)
            assertEquals(1f, repository.settings().contentTextScale, 0f)
            assertEquals(2f, repository.settings().interfaceTextScale, 0f)
            repository.updateSettings { it.copy(contentTextScale = -1f, interfaceTextScale = 1.25f) }
            assertEquals(0.75f, repository.settings().contentTextScale, 0f)
            assertEquals(1.25f, repository.settings().interfaceTextScale, 0f)
        } finally { repository.close(); directory.deleteRecursively() }
    }

    @Test fun themeKeepsSystemDensityAndContentDoesNotCompoundInterfaceScale() {
        // On API 34+ this Density retains Android's non-linear font-size conversion.
        val system = Density(density = 2f, fontScale = 2f)
        var settings by mutableStateOf(AppSettings(contentTextScale = 1.5f, interfaceTextScale = 0.8f))
        var observed: Density? = null
        var interfaceSize = 0f
        var contentSize = 0f
        compose.setContent {
            CompositionLocalProvider(LocalDensity provides system) {
                MomentumTheme(settings) {
                    observed = LocalDensity.current
                    interfaceSize = MaterialTheme.typography.bodyLarge.fontSize.value
                    contentSize = taskTypography().bodyLarge.fontSize.value
                }
            }
        }
        compose.runOnIdle {
            assertSame(system, observed)
            assertEquals(16f * 0.8f, interfaceSize, 0.001f)
            assertEquals(16f * 1.5f, contentSize, 0.001f)
            settings = settings.copy(interfaceTextScale = 2f)
        }
        compose.runOnIdle {
            assertSame(system, observed)
            assertEquals(32f, interfaceSize, 0.001f)
            assertEquals(24f, contentSize, 0.001f)
        }
    }

    @Test fun nativeControlsChangeOnlyTheirAreaAndResetBoth() {
        val app = ApplicationProvider.getApplicationContext<MomentumApplication>()
        var settings by mutableStateOf(AppSettings(compact = true))
        compose.setContent {
            MomentumTheme(settings) {
                Column(Modifier.verticalScroll(rememberScrollState())) {
                    TypographySettings(settings) { update -> settings = update(settings) }
                }
            }
        }
        val content = compose.onNodeWithContentDescription(app.getString(R.string.content_text_size))
        content.performScrollTo().assertHeightIsAtLeast(48.dp)
        content.performSemanticsAction(SemanticsActions.SetProgress) { it(1.5f) }
        compose.runOnIdle {
            assertEquals(1.5f, settings.contentTextScale, 0.001f)
            assertEquals(1f, settings.interfaceTextScale, 0f)
        }
        val controls = compose.onNodeWithContentDescription(app.getString(R.string.interface_text_size))
        controls.performScrollTo().assertHeightIsAtLeast(48.dp)
        controls.performSemanticsAction(SemanticsActions.SetProgress) { it(0.8f) }
        compose.runOnIdle {
            assertEquals(1.5f, settings.contentTextScale, 0.001f)
            assertEquals(0.8f, settings.interfaceTextScale, 0.001f)
        }
        compose.onNodeWithText(app.getString(R.string.font_monospace)).performScrollTo().performClick()
        compose.runOnIdle { assertEquals(NativeFont.Monospace, settings.font) }
        compose.onNodeWithText(app.getString(R.string.typography_reset)).performScrollTo().performClick()
        compose.runOnIdle {
            assertEquals(NativeFont.System, settings.font)
            assertEquals(1f, settings.contentTextScale, 0f)
            assertEquals(1f, settings.interfaceTextScale, 0f)
            assertTrue(settings.compact)
        }
    }
}
