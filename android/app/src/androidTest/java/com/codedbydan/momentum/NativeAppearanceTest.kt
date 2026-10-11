// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.unit.dp
import androidx.lifecycle.SavedStateHandle
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.codedbydan.momentum.data.*
import com.codedbydan.momentum.ui.*
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.util.UUID

@RunWith(AndroidJUnit4::class)
class NativeAppearanceTest {
    @get:Rule val compose = createComposeRule()
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()

    @Test fun accentPersistsSeparatelyFromDynamicColorAndTypography() = runBlocking {
        val directory = File(app.cacheDir, "appearance-${UUID.randomUUID()}")
        var repository = MomentumRepository(app, directory)
        try {
            repository.updateSettings { it.copy(accent = NativeAccent.Amethyst, dynamicColor = true,
                contentTextScale = 1.5f, interfaceTextScale = 0.85f) }
            repository.close()
            repository = MomentumRepository(app, directory)
            assertEquals(NativeAccent.Amethyst, repository.settings().accent)
            assertTrue(repository.settings().dynamicColor)
            assertEquals(1.5f, repository.settings().contentTextScale, 0f)
            assertEquals(0.85f, repository.settings().interfaceTextScale, 0f)
            repository.updateSettings { it.copy(dynamicColor = false) }
            assertEquals(NativeAccent.Amethyst, repository.settings().accent)
        } finally { repository.close(); directory.deleteRecursively() }
    }

    @Test fun namedNativePaletteSelectionIsAccessibleAndPreservesOtherPreferences() {
        var settings by mutableStateOf(AppSettings(dynamicColor = true, contentTextScale = 1.5f))
        compose.setContent {
            MomentumTheme(settings) {
                Column(Modifier.verticalScroll(rememberScrollState())) {
                    AccentSettings(settings) { change -> settings = change(settings) }
                }
            }
        }
        val name = app.getString(R.string.accent_amethyst)
        compose.onNodeWithText(name).performScrollTo().assertHeightIsAtLeast(48.dp).performClick()
        compose.onNodeWithText(name).assertIsSelected()
        compose.runOnIdle {
            assertEquals(NativeAccent.Amethyst, settings.accent)
            assertTrue(settings.dynamicColor)
            assertEquals(1.5f, settings.contentTextScale, 0f)
        }
    }

    @Test fun initialLoadingHasOneStatusAndKeepsNavigationAndAddStable() = runBlocking {
        val directory = File(app.cacheDir, "loading-${UUID.randomUUID()}")
        val repository = MomentumRepository(app, directory)
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
        val entered = CompletableDeferred<Unit>()
        val release = CompletableDeferred<Unit>()
        val hold = scope.launch { repository.read { entered.complete(Unit); release.await() } }
        try {
            withTimeout(15_000) { entered.await() }
            lateinit var vm: MomentumViewModel
            compose.runOnUiThread { vm = MomentumViewModel(app, SavedStateHandle(), repository, {}, {}) }
            compose.setContent { MomentumApp(vm) }
            compose.onAllNodesWithText(app.getString(R.string.loading)).assertCountEquals(1)
            compose.onNodeWithText(app.getString(R.string.loading)).assertIsDisplayed().assertHasNoClickAction()
            compose.onNodeWithText(app.getString(R.string.settings)).assertIsDisplayed()
            val add = compose.onNodeWithContentDescription(app.getString(R.string.add_task))
            add.assertIsDisplayed().assertHasClickAction()
            val before = add.fetchSemanticsNode().boundsInRoot
            release.complete(Unit)
            withTimeout(15_000) { hold.join() }
            compose.waitUntil(15_000) { vm.state.value.snapshot != null && !vm.state.value.loading }
            compose.onNodeWithText(app.getString(R.string.loading)).assertDoesNotExist()
            add.assertIsDisplayed()
            val after = add.fetchSemanticsNode().boundsInRoot
            assertEquals(before.left, after.left, 1f)
            assertEquals(before.top, after.top, 1f)
        } finally {
            release.complete(Unit); hold.join(); scope.cancel(); repository.close(); directory.deleteRecursively()
        }
    }
}
