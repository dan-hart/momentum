// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import android.content.res.Configuration
import android.net.Uri
import android.graphics.Bitmap
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import androidx.lifecycle.SavedStateHandle
import androidx.test.core.app.ApplicationProvider
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.*
import com.codedbydan.momentum.ui.*
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.collect
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import java.io.File
import java.util.Locale
import java.util.UUID

class PortableSettingsUiTest {
    @get:Rule val compose = createComposeRule()
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()
    @Test fun transferAndSyncControlsStartOffAndAreAccessibleAtLargeGermanText() = runBlocking {
        val directory = File(app.cacheDir, "settings-ui-${UUID.randomUUID()}")
        val repo = MomentumRepository(app, directory)
        lateinit var vm: MomentumViewModel
        val initial = repo.snapshot(View.Today)
        val localized = app.createConfigurationContext(Configuration(app.resources.configuration).apply { setLocale(Locale.GERMAN) })
        var included by mutableStateOf(false); var restored by mutableStateOf(false)
        try {
            compose.runOnUiThread { vm = MomentumViewModel(app, SavedStateHandle(), repo, {}, {}) }
            compose.setContent {
                val state by vm.state.collectAsState(initial = UiState(snapshot = initial, loading = false))
                CompositionLocalProvider(LocalContext provides localized, LocalDensity provides Density(LocalDensity.current.density, 2f)) {
                    MomentumTheme(initial.settings) {
                        Column(Modifier.verticalScroll(rememberScrollState())) {
                            TransferCheck(R.string.include_app_settings, included) { included = it }
                            TransferCheck(R.string.restore_app_settings, restored) { restored = it }
                            AppSettingsSync(state, vm)
                        }
                    }
                }
            }
            val include = compose.onNodeWithContentDescription(localized.getString(R.string.include_app_settings))
            val restore = compose.onNodeWithContentDescription(localized.getString(R.string.restore_app_settings))
            include.assertIsOff().assertHeightIsAtLeast(48.dp)
            restore.assertIsOff().assertHeightIsAtLeast(48.dp)
            include.performClick(); include.assertIsOn(); restore.assertIsOff()
            restore.performClick(); restore.assertIsOn(); include.assertIsOn()
            compose.onRoot().captureToImage().asAndroidBitmap().let { bitmap ->
                File(app.cacheDir, "momentum-portable-settings-de-large-transfer.png").outputStream().use { bitmap.compress(Bitmap.CompressFormat.PNG, 100, it) }
            }
            val sync = compose.onNodeWithContentDescription(localized.getString(R.string.sync_app_settings))
            sync.performScrollTo().assertIsOff().assertHeightIsAtLeast(48.dp).performClick()
            compose.waitUntil(10_000) { vm.state.value.snapshot?.settingsSync?.enabled == true }
            sync.performScrollTo().assertIsOn()
            compose.onNodeWithText(localized.getString(R.string.settings_sync_provider_off)).performScrollTo().assertIsDisplayed()
            compose.onRoot().captureToImage().asAndroidBitmap().let { bitmap ->
                File(app.cacheDir, "momentum-portable-settings-de-large-sync.png").outputStream().use { bitmap.compress(Bitmap.CompressFormat.PNG, 100, it) }
            }
            assertEquals(SyncProvider.Off, repo.settings().provider)
            assertFalse(repo.settings().notifications)
        } finally { vm.platform.foreground(false); repo.close(); directory.deleteRecursively() }
    }
    @Test fun bootstrapSummaryShowsNativeValuesWithoutExposingJsonOrPrivateFields() {
        val json = """{"version":1,"values":{"grouping":"project","rowDensity":"compact","completionSoundChoice":"warm-spark","upcomingDays":30}}"""
        compose.setContent { MomentumTheme(AppSettings()) { Column(Modifier.verticalScroll(rememberScrollState())) {
            SettingsSummary(R.string.settings_synced_device, json)
        } } }
        compose.onNodeWithText(app.getString(R.string.settings_synced_device)).assertIsDisplayed()
        compose.onNodeWithText(app.getString(R.string.settings_summary_item, app.getString(R.string.group), app.getString(R.string.project))).assertIsDisplayed()
        compose.onNodeWithText(app.getString(R.string.settings_summary_item, app.getString(R.string.completion_sound_choice), app.getString(R.string.sound_warm_spark))).performScrollTo().assertIsDisplayed()
        compose.onAllNodesWithText("rowDensity", substring = true).assertCountEquals(0)
        compose.onAllNodesWithText("version", substring = true).assertCountEquals(0)
    }
    @Test fun actualBackupScreenKeepsExportAndRestoreConsentIndependentAndUnchecked() = runBlocking {
        val directory = File(app.cacheDir, "actual-transfer-ui-${UUID.randomUUID()}")
        val repo = MomentumRepository(app, directory)
        lateinit var vm: MomentumViewModel
        try {
            repo.create(TaskDraft("Retained task", "INBOX_PROJECT", null, null, null, 0.0, "", emptyList(), emptyList()), View.Today)
            val backup = File(directory, "fixture.json"); repo.exportBackup(backup, true)
            compose.runOnUiThread { vm = MomentumViewModel(app, SavedStateHandle(), repo, {}, {}) }
            compose.setContent {
                val state by vm.state.collectAsState()
                MomentumTheme(state.snapshot?.settings ?: AppSettings()) { SettingsScreen(state, vm) }
            }
            compose.waitUntil(10_000) { vm.state.value.snapshot != null && !vm.state.value.loading }
            val include = compose.onNodeWithContentDescription(app.getString(R.string.include_app_settings))
            include.performScrollTo().assertIsOff().performClick(); include.assertIsOn()
            compose.runOnUiThread { vm.prepareImport(Uri.fromFile(backup)) }
            compose.waitUntil(10_000) { vm.state.value.importReady }
            compose.onNodeWithContentDescription(app.getString(R.string.restore_app_settings)).assertIsOff()
            compose.onNodeWithText(app.getString(R.string.cancel)).performClick()
            compose.waitUntil(10_000) { !vm.state.value.importReady }
            assertFalse(repo.read { settingsSyncStatus().enabled })
            assertEquals(1, repo.read { allTasks() }.size)
            val only = File(directory, "settings-only.json").apply { writeText("""{"version":1,"values":{"rowDensity":"compact"}}""") }
            compose.runOnUiThread { vm.prepareImport(Uri.fromFile(only), settingsOnly = true) }
            compose.waitUntil(10_000) { !vm.state.value.importPreparing && vm.state.value.settingsFeedback == R.string.app_settings_imported }
            assertFalse(vm.state.value.importReady)
            assertTrue(repo.settings().compact)
            assertEquals(1, repo.read { allTasks() }.size)
        } finally { vm.platform.foreground(false); repo.close(); directory.deleteRecursively() }
    }

}
