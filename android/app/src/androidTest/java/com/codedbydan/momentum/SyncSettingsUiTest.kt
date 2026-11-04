// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import androidx.compose.runtime.*
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.lifecycle.SavedStateHandle
import androidx.test.core.app.ApplicationProvider
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.*
import com.codedbydan.momentum.ui.*
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import java.io.File
import java.util.UUID

/** Private stores and disposable saved credentials; these cases never start an exchange. */
class SyncSettingsUiTest {
    @get:Rule val compose = createComposeRule()
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()

    @Test fun editedConnectionDisablesSavedActionsAndKeepsRecoveryVisible() = runBlocking {
        fixture { repo, vm ->
            vm.platform.saveConnection(NextcloudSettings("https://cloud.example.com", "alex", "disposable", "momentum", true, null, false))
            vm.platform.reportFailure(CoreException.Actionable(SyncFailureKind.UNAUTHORIZED, "Disposable connection was refused"))
            val initial = repo.snapshot(View.Today)
            compose.setContent { MomentumTheme(initial.settings) { SyncScreen(UiState(snapshot = initial, loading = false), vm) } }
            val test = compose.onNodeWithText(app.getString(R.string.test_connection))
            compose.waitUntil(10_000) { !test.fetchSemanticsNode().config.contains(androidx.compose.ui.semantics.SemanticsProperties.Disabled) }
            test.performScrollTo().assertIsEnabled()
            compose.onNodeWithText("https://cloud.example.com").performScrollTo().performTextReplacement("https://other.example.com")
            test.performScrollTo().assertIsNotEnabled()
            compose.onNodeWithText(app.getString(R.string.sync_now)).performScrollTo().assertIsNotEnabled()
            compose.onNodeWithText(app.getString(R.string.sync_unauthorized)).performScrollTo().assertIsDisplayed()
            compose.onAllNodesWithText(app.getString(R.string.sync_unsaved_connection))[0].performScrollTo().assertIsDisplayed()
            assertEquals("https://cloud.example.com", vm.platform.connection()?.serverUrl)
        }
    }

    @Test fun compressionSearchRevealsAdvancedControlsWithoutChangingProvider() = runBlocking {
        fixture { repo, vm ->
            val initial = repo.snapshot(View.Today)
            var destination by mutableStateOf<String?>(null)
            compose.setContent { MomentumTheme(initial.settings) {
                SyncScreen(UiState(snapshot = initial, loading = false, settingsDestination = destination), vm)
            } }
            compose.onNodeWithText(app.getString(R.string.folder)).assertDoesNotExist()
            compose.runOnUiThread { destination = "sync.compression" }
            compose.onNodeWithText(app.getString(R.string.folder)).performScrollTo().assertIsDisplayed()
            compose.onNodeWithText(app.getString(R.string.compress)).performScrollTo().assertIsDisplayed()
            assertEquals(SyncProvider.Nextcloud, repo.settings().provider)
        }
    }

    @Test fun offKeepsSavedProviderFailuresOutOfTheLocalOnlyOverview() = runBlocking {
        fixture { repo, vm ->
            vm.platform.reportFailure(CoreException.Actionable(SyncFailureKind.UNAUTHORIZED, "Saved provider failure"))
            repo.updateSettings { it.copy(provider = SyncProvider.Off) }
            val initial = repo.snapshot(View.Today)
            compose.setContent { MomentumTheme(initial.settings) { SyncScreen(UiState(snapshot = initial, loading = false), vm) } }
            compose.onNodeWithText(app.getString(R.string.sync_off_explanation)).assertIsDisplayed()
            compose.onNodeWithText(app.getString(R.string.sync_unauthorized)).assertDoesNotExist()
            compose.onNodeWithText(app.getString(R.string.sync_now)).assertDoesNotExist()
            assertEquals(SyncFailureKind.UNAUTHORIZED, vm.platform.state.value.failureKind)
        }
    }

    private suspend fun fixture(body: suspend (MomentumRepository, MomentumViewModel) -> Unit) {
        val directory = File(app.cacheDir, "sync-settings-${UUID.randomUUID()}")
        val repo = MomentumRepository(app, directory)
        lateinit var vm: MomentumViewModel
        try {
            repo.updateSettings { it.copy(provider = SyncProvider.Nextcloud, automaticSync = false, backgroundSync = false) }
            compose.runOnUiThread { vm = MomentumViewModel(app, SavedStateHandle(), repo, {}, {}) }
            body(repo, vm)
        } finally {
            vm.platform.foreground(false)
            repo.close(); directory.deleteRecursively()
        }
    }
}
