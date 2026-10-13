// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.platform

import androidx.test.platform.app.InstrumentationRegistry
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.*
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.first
import org.junit.Assert.*
import org.junit.Test
import java.io.File
import java.security.KeyStore
import java.util.UUID

class SettingsCompletionBoundaryTest {
    @Test fun committedSettingsReachNativeEvenWhenTaskExchangeFails() = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val dir = File(context.cacheDir, "settings-completion-${UUID.randomUUID()}")
        val repo = MomentumRepository(context, dir)
        val settingsFile = File(dir, "received-settings.json")
        val platform = PlatformCoordinator(context, repo, false, exchangeNextcloud = { engine, _, _, _ ->
            engine.importSettings(settingsFile.absolutePath)
            throw CoreException.Actionable(SyncFailureKind.UNREACHABLE, "Isolated task exchange failure after settings commit")
        })
        try {
            repo.updateSettings { it.copy(provider = SyncProvider.Nextcloud, automaticSync = false, notifications = true) }
            settingsFile.writeText("""{"version":1,"values":{"rowDensity":"compact","sort":"title"}}""")
            SecretVault(dir).saveConnection(NextcloudSettings("https://fixture.invalid", "fixture-user", "fixture-password", "custom", false, null))
            val before = platform.state.value.revision
            platform.syncNow()
            withTimeout(5_000) { platform.state.first { !it.syncing && it.revision > before && it.failureKind == SyncFailureKind.UNREACHABLE } }
            val snapshot = repo.snapshot(View.Today)
            assertTrue(snapshot.settings.compact); assertEquals(SortKey.TITLE, snapshot.preferences.sort)
            assertTrue(snapshot.settings.notifications); assertEquals(SyncProvider.Nextcloud, snapshot.settings.provider)
            assertEquals(0uL, snapshot.sync.lastNextcloudMs)
            assertNotNull(platform.state.value.error)
        } finally {
            platform.foreground(false); repo.close()
            KeyStore.getInstance("AndroidKeyStore").apply { load(null); deleteEntry(SecretVault(dir).keyAlias) }
            dir.deleteRecursively()
        }
    }
}
