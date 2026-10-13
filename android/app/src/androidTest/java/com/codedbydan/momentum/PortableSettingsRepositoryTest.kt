// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import androidx.test.core.app.ApplicationProvider
import androidx.datastore.preferences.core.PreferenceDataStoreFactory
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.booleanPreferencesKey
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.*
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.cancelAndJoin
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.util.UUID

@RunWith(AndroidJUnit4::class)
class PortableSettingsRepositoryTest {
    private suspend fun fixture(block: suspend (MomentumRepository, File) -> Unit) {
        val context = ApplicationProvider.getApplicationContext<android.content.Context>()
        val directory = File(context.cacheDir, "portable-${UUID.randomUUID()}")
        val repo = MomentumRepository(context, directory)
        try { block(repo, directory) } finally { repo.close(); directory.deleteRecursively() }
    }
    @Test fun nativePortableEditsPersistInCoreWithoutDeviceControlsOrConsent() = runBlocking { fixture { repo, _ ->
        repo.updateSettings { it.copy(compact = true, colorfulLabels = false, notifications = true,
            provider = SyncProvider.Nextcloud, font = NativeFont.Serif, systemSearch = true) }
        repo.updatePreferences { it.copy(groupBy = GroupBy.PROJECT, upcomingDays = 30u, morningSummaryEnabled = true) }
        val json = JSONObject(repo.read { portableSettingsJson() })
        val values = json.getJSONObject("values")
        assertEquals("compact", values.getString("rowDensity"))
        assertEquals("project", values.getString("grouping"))
        assertEquals(30, values.getInt("upcomingDays"))
        assertEquals(9, values.length())
        assertFalse(repo.read { settingsSyncStatus().enabled })
        assertFalse(json.toString().contains("Nextcloud"))
        assertFalse(values.has("notifications")); assertFalse(values.has("systemSearch")); assertFalse(values.has("font"))
    } }
    @Test fun importedPartialSettingsReachNativeSnapshotAndKeepPrivateSettingsLocal() = runBlocking { fixture { repo, directory ->
        repo.updateSettings { it.copy(notifications = true, font = NativeFont.Serif, automaticSync = false) }
        repo.updatePreferences { it.copy(morningSummaryEnabled = true) }
        val settings = File(directory, "partial.json").apply { writeText("""{"version":1,"values":{"rowDensity":"compact","sort":"title","unknown":"ignored"}}""") }
        repo.read { importSettings(settings.absolutePath) }
        val snapshot = repo.snapshot(View.Today)
        assertTrue(snapshot.settings.compact)
        assertEquals(SortKey.TITLE, snapshot.preferences.sort)
        assertTrue(snapshot.settings.notifications); assertFalse(snapshot.settings.automaticSync)
        assertTrue(snapshot.preferences.morningSummaryEnabled); assertEquals(NativeFont.Serif, snapshot.settings.font)
        assertFalse(repo.read { settingsSyncStatus().enabled })
        repo.close()
        val reopened = MomentumRepository(ApplicationProvider.getApplicationContext(), directory)
        try { assertTrue(reopened.snapshot(View.Today).settings.compact); assertFalse(reopened.read { settingsSyncStatus().enabled }) }
        finally { reopened.close() }
    } }
    @Test fun invalidIncludedSettingsCannotPartiallyRestoreTasksOrNativeValues() = runBlocking { fixture { repo, directory ->
        val task = repo.create(TaskDraft("Retained", "INBOX_PROJECT", null, null, null, 0.0, "", emptyList(), emptyList()), View.Today).id!!
        val backup = File(directory, "invalid.json")
        repo.read { exportBackupWithSettings(backup.absolutePath, true) }
        val json = JSONObject(backup.readText())
        json.getJSONObject("momentumSettings").getJSONObject("values").put("rowDensity", "compact").put("upcomingDays", 999)
        backup.writeText(json.toString())
        try { repo.read { importBackupWithSettings(backup.absolutePath, true) }; fail("Invalid settings must fail") } catch(_: CoreException.Invalid) { }
        assertEquals("Retained", repo.detail(task)!!.title); assertFalse(repo.snapshot(View.Today).settings.compact)
    } }
    @Test fun transferFlagsAreIndependentAndSettingsOnlyNeverReplaceTasks() = runBlocking { fixture { repo, directory ->
        val task = repo.create(TaskDraft("Keep this task", "INBOX_PROJECT", null, null, null, 0.0, "", emptyList(), emptyList()), View.Today).id!!
        repo.updateSettings { it.copy(compact = true, notifications = true, backgroundSync = true) }
        val plain = File(directory, "plain.json"); val included = File(directory, "included.json"); val only = File(directory, "only.json")
        repo.exportBackup(plain); repo.exportBackup(included, true); repo.exportAppSettings(only)
        assertFalse(JSONObject(plain.readText()).has("momentumSettings"))
        assertTrue(JSONObject(included.readText()).has("momentumSettings"))
        assertFalse(JSONObject(only.readText()).has("data"))
        repo.updateSettings { it.copy(compact = false) }
        repo.restoreBackup(included)
        assertFalse(repo.snapshot(View.Today).settings.compact)
        repo.restoreBackup(included, true)
        assertTrue(repo.snapshot(View.Today).settings.compact)
        repo.updateSettings { it.copy(compact = false) }
        repo.importAppSettings(only)
        assertTrue(repo.snapshot(View.Today).settings.compact)
        assertEquals(listOf(task), repo.read { allTasks() }.map { it.id })
        assertTrue(repo.settings().notifications); assertTrue(repo.settings().backgroundSync)
        assertFalse(repo.read { settingsSyncStatus().enabled })
    } }
    @Test fun failedCoreDurabilityCannotPublishNativeSettingsAndRetryWorks() = runBlocking { fixture { repo, directory ->
        repo.snapshot(View.Today)
        val blocker = File(directory, ".momentum-transaction").apply { writeText("isolated invalid journal") }
        try { repo.updateSettings { it.copy(compact = true) }; fail("Save must fail") } catch(_: CoreException.Io) { }
        assertFalse(repo.settings().compact)
        assertEquals("regular", JSONObject(repo.read { portableSettingsJson() }).getJSONObject("values").getString("rowDensity"))
        blocker.delete()
        repo.updateSettings { it.copy(compact = true) }
        assertTrue(repo.settings().compact)
    } }
    @Test fun syncConsentSurvivesReopenButIsNeverTransferredAndRejectsStaleChoices() = runBlocking { fixture { repo, directory ->
        repo.snapshot(View.Today)
        repo.setSettingsSync(true)
        try { repo.chooseSettings(SettingsBootstrapChoice.USE_REMOTE, "stale-review"); fail("Stale review must fail") } catch(_: CoreException.Invalid) { }
        val file = File(directory, "settings-only.json"); repo.exportAppSettings(file)
        assertFalse(file.readText().contains("enabled"))
        repo.setSettingsSync(false)
        repo.importAppSettings(file)
        assertFalse(repo.read { settingsSyncStatus().enabled })
        repo.setSettingsSync(true); repo.close()
        val reopened = MomentumRepository(ApplicationProvider.getApplicationContext(), directory)
        try { assertTrue(reopened.snapshot(View.Today).settingsSync!!.enabled) } finally { reopened.close() }
    } }

    @Test fun unreadableNativePreferencesDoNotHideOfflineTasksOrActivateServices() = runBlocking { fixture { repo, directory ->
        val id = repo.create(TaskDraft("Offline task survives preference recovery", "INBOX_PROJECT", null, null, null, 0.0, "", emptyList(), emptyList()), View.Today).id!!
        repo.updateSettings { it.copy(compact = true) }
        repo.close()
        File(directory, "android.preferences_pb").writeBytes(byteArrayOf(0xff.toByte(), 0xff.toByte(), 0xff.toByte()))
        val reopened = MomentumRepository(ApplicationProvider.getApplicationContext(), directory)
        try {
            val snapshot = reopened.snapshot(View.Project("INBOX_PROJECT"))
            assertTrue(snapshot.listing.sections.flatMap { it.rows }.filterIsInstance<Row.Task>().any { it.row.id == id })
            assertNotNull(snapshot.appSettingsError)
            assertEquals(SyncProvider.Off, snapshot.settings.provider); assertFalse(snapshot.settings.notifications)
            assertFalse(snapshot.settingsSync!!.enabled)
        } finally { reopened.close() }
    } }

    @Test fun unseenRemoteEditToAnotherKeySurvivesLocalNativeDelta() = runBlocking {
        val context = ApplicationProvider.getApplicationContext<android.content.Context>()
        val directory = File(context.cacheDir, "portable-race-${UUID.randomUUID()}")
        val repo = MomentumRepository(context, directory, beforePortableStage = {
            stagePortableSettings("""{"version":1,"values":{"sort":"title"}}""")
        }, observeDayChange = { dayChanged() })
        try {
            repo.snapshot(View.Today)
            repo.updateSettings { it.copy(colorfulLabels = false) }
            val snapshot = repo.snapshot(View.Today)
            assertEquals(SortKey.TITLE, snapshot.preferences.sort)
            assertFalse(snapshot.settings.colorfulLabels)
            assertEquals("title", JSONObject(repo.read { portableSettingsJson() }).getJSONObject("values").getString("sort"))
        } finally { repo.close(); directory.deleteRecursively() }
    }

    private suspend fun seedEnabledLocalControls(directory: File) {
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
        val store = PreferenceDataStoreFactory.create(scope = scope) { File(directory, "android.preferences_pb") }
        try { store.edit {
            it[booleanPreferencesKey("notifications")] = true
            it[booleanPreferencesKey("automatic-sync")] = true
            it[booleanPreferencesKey("background-sync")] = true
            it[stringPreferencesKey("provider")] = SyncProvider.Nextcloud.name
            it[booleanPreferencesKey("morning-summary")] = true
            it[booleanPreferencesKey("compact")] = true
        } } finally { scope.coroutineContext[Job]!!.cancelAndJoin() }
    }

    @Test fun failedInitialPortableSeedStillAllowsDurableLocalOptOuts() = runBlocking {
        val context = ApplicationProvider.getApplicationContext<android.content.Context>()
        val directory = File(context.cacheDir, "portable-local-opt-outs-${UUID.randomUUID()}").also { assertTrue(it.mkdirs()) }
        seedEnabledLocalControls(directory)
        val blocker = File(directory, "portable-settings.json.tmp").also { assertTrue(it.mkdir()); File(it, "isolated-blocker").writeText("retained") }
        var repo = MomentumRepository(context, directory)
        try {
            val before = repo.snapshot(View.Today)
            assertNotNull(before.appSettingsError)
            assertTrue(before.settings.notifications); assertTrue(before.settings.automaticSync); assertTrue(before.settings.backgroundSync)
            assertEquals(SyncProvider.Nextcloud, before.settings.provider); assertTrue(before.preferences.morningSummaryEnabled)
            val portableBefore = repo.read { portableSettingsJson() }
            repo.updateSettings { it.copy(notifications = false) }
            repo.updateSettings { it.copy(provider = SyncProvider.Off) }
            repo.updateSettings { it.copy(automaticSync = false) }
            repo.updateSettings { it.copy(backgroundSync = false) }
            repo.updatePreferences { it.copy(morningSummaryEnabled = false) }
            val changed = repo.snapshot(View.Today)
            assertNotNull(changed.appSettingsError)
            assertFalse(changed.settings.notifications); assertFalse(changed.settings.automaticSync); assertFalse(changed.settings.backgroundSync)
            assertEquals(SyncProvider.Off, changed.settings.provider); assertFalse(changed.preferences.morningSummaryEnabled)
            assertEquals(portableBefore, repo.read { portableSettingsJson() })
            assertFalse(File(directory, "portable-settings.json").exists())
            try { repo.updateSettings { it.copy(compact = false) }; fail("Portable edits must remain blocked") } catch(_: IllegalStateException) { }
            try { repo.updatePreferences { it.copy(sort = SortKey.TITLE) }; fail("Portable rules must remain blocked") } catch(_: IllegalStateException) { }
            try { repo.exportAppSettings(File(directory, "export.json")); fail("Portable export must remain blocked") } catch(_: IllegalStateException) { }
            assertTrue(repo.settings().compact)
            repo.close(); repo = MomentumRepository(context, directory)
            val reopened = repo.snapshot(View.Today)
            assertNotNull(reopened.appSettingsError)
            assertFalse(reopened.settings.notifications); assertFalse(reopened.settings.automaticSync); assertFalse(reopened.settings.backgroundSync)
            assertEquals(SyncProvider.Off, reopened.settings.provider); assertFalse(reopened.preferences.morningSummaryEnabled)
            assertEquals(portableBefore, repo.read { portableSettingsJson() })
            assertTrue(blocker.deleteRecursively())
            repo.refreshAppSettings()
            val recovered = repo.snapshot(View.Today)
            assertNull(recovered.appSettingsError)
            assertEquals(9, JSONObject(repo.read { portableSettingsJson() }).getJSONObject("values").length())
            assertFalse(recovered.settings.notifications); assertFalse(recovered.preferences.morningSummaryEnabled)
        } finally { repo.close(); directory.deleteRecursively() }
    }

    @Test fun importedCoreChoicesAreAppliedBeforeRetryingAnInitialNativeSeed() = runBlocking {
        val context = ApplicationProvider.getApplicationContext<android.content.Context>()
        val directory = File(context.cacheDir, "portable-seed-import-${UUID.randomUUID()}").also { assertTrue(it.mkdirs()) }
        seedEnabledLocalControls(directory)
        val blocker = File(directory, "portable-settings.json.tmp").also { assertTrue(it.mkdir()); File(it, "isolated-blocker").writeText("retained") }
        val repo = MomentumRepository(context, directory)
        try {
            assertNotNull(repo.snapshot(View.Today).appSettingsError)
            assertTrue(blocker.deleteRecursively())
            val imported = File(directory, "received-settings.json").apply {
                writeText("""{"version":1,"values":{"rowDensity":"regular","sort":"title"}}""")
            }
            repo.read { importSettings(imported.absolutePath) }
            val snapshot = repo.snapshot(View.Today)
            assertNull(snapshot.appSettingsError)
            assertFalse(snapshot.settings.compact); assertEquals(SortKey.TITLE, snapshot.preferences.sort)
            assertTrue(snapshot.settings.notifications); assertTrue(snapshot.settings.backgroundSync)
            assertEquals(SyncProvider.Nextcloud, snapshot.settings.provider)
            val values = JSONObject(repo.read { portableSettingsJson() }).getJSONObject("values")
            assertEquals("regular", values.getString("rowDensity")); assertEquals("title", values.getString("sort"))
            assertEquals(9, values.length()); assertFalse(snapshot.settingsSync!!.enabled)
        } finally { repo.close(); directory.deleteRecursively() }
    }

}
