// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.platform

import androidx.test.platform.app.InstrumentationRegistry
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.*
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import java.io.File
import java.security.KeyStore
import java.util.UUID
import java.util.concurrent.atomic.AtomicInteger

class FolderMigrationBoundaryTest {
    @Test fun cancellingTheCopyCoroutineResumesForegroundAutomaticSync() = runBlocking { fixture { repo, _, connection ->
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val entered = CompletableDeferred<Unit>()
        val exchanges = AtomicInteger()
        repo.updateSettings { it.copy(automaticSync = true) }
        val platform = PlatformCoordinator(context, repo, false, migrateFolder = { _, _, _ ->
            entered.complete(Unit)
            awaitCancellation()
        }, exchangeNextcloud = { _, _, _, _ -> exchanges.incrementAndGet(); Unit })
        try {
            platform.foreground(true)
            assertTrue("Initial automatic exchange starts", withTimeoutOrNull(5_000) { while(exchanges.get() == 0) delay(10); true } ?: false)
            val beforeCopy = exchanges.get()
            val copy = async(Dispatchers.IO) { platform.migrateConnection(connection) }
            entered.await()
            copy.cancelAndJoin()
            assertEquals(connection, platform.connection())
            assertFalse(platform.state.value.migratingFolder)
            assertTrue("Automatic exchange resumes after caller cancellation", withTimeoutOrNull(5_000) { while(exchanges.get() <= beforeCopy) delay(10); true } ?: false)
        } finally { platform.foreground(false) }
    } }
    private suspend fun fixture(block: suspend (MomentumRepository, File, NextcloudSettings) -> Unit) {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val dir = File(context.cacheDir, "migration-${UUID.randomUUID()}")
        val repo = MomentumRepository(context, dir)
        val connection = NextcloudSettings("https://fixture.invalid", "fixture-user", "private-password", "Existing", true, "private-encryption")
        repo.updateSettings { it.copy(provider = SyncProvider.Nextcloud, automaticSync = false) }
        SecretVault(dir).saveConnection(connection)
        try { block(repo, dir, connection) } finally {
            repo.close()
            KeyStore.getInstance("AndroidKeyStore").apply { load(null); deleteEntry(SecretVault(dir).keyAlias) }
            dir.deleteRecursively()
        }
    }
    @Test fun verifiedCopySwitchesOnlySavedFolderAndRetainsOfflineEdits() = runBlocking { fixture { repo, dir, connection ->
        val entered = CompletableDeferred<Unit>(); val release = CompletableDeferred<Unit>()
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val platform = PlatformCoordinator(context, repo, false, migrateFolder = { engine, saved, _ ->
            assertEquals(connection, saved); assertFalse(engine.p2pRunning())
            entered.complete(Unit); release.await()
            NextcloudFolderMigrationReport(saved.folder, "momentum", 1u, listOf("sync-data.json"), true)
        })
        val copy = async(Dispatchers.IO) { platform.migrateConnection(connection) }
        entered.await()
        assertTrue(platform.state.value.migratingFolder)
        assertEquals(connection, platform.connection())
        val id = repo.create(TaskDraft("Offline during copy", "INBOX_PROJECT", null, null, null, 0.0, "", emptyList(), emptyList()), View.Today).id!!
        release.complete(Unit); assertTrue(copy.await().verified)
        assertEquals(connection.copy(folder = "momentum"), platform.connection())
        assertEquals("Offline during copy", repo.detail(id)!!.title)
        assertFalse(platform.state.value.migratingFolder)
        platform.foreground(false)
    } }
    @Test fun failedOrUnverifiedCopyAndCredentialSaveFailureKeepOriginalConnection() = runBlocking { fixture { repo, dir, connection ->
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        for(mode in 0..2) {
            val platform = PlatformCoordinator(context, repo, false, migrateFolder = { _, saved, _ ->
                if(mode == 0) throw IllegalStateException("Isolated readback failure")
                NextcloudFolderMigrationReport(saved.folder, "momentum", 1u, emptyList(), mode != 1)
            }, saveMigratedConnection = if(mode == 2) { _ -> throw java.io.IOException("Isolated credential save failure") } else null)
            try { platform.migrateConnection(connection); fail("Copy must fail") } catch(_: Exception) { }
            assertEquals(connection, platform.connection())
            assertFalse(platform.state.value.migratingFolder)
            platform.foreground(false)
        }
    } }
    @Test fun cancellationAndStaleConnectionCannotSwitchTheSavedFolder() = runBlocking { fixture { repo, dir, connection ->
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        var called = false
        val entered = CompletableDeferred<Unit>(); val release = CompletableDeferred<Unit>()
        val platform = PlatformCoordinator(context, repo, false, migrateFolder = { _, saved, token ->
            called = true; entered.complete(Unit); release.await(); assertTrue(token.isCancelled())
            NextcloudFolderMigrationReport(saved.folder, "momentum", 1u, emptyList(), true)
        })
        try { platform.migrateConnection(connection.copy(folder = "Stale")); fail("Stale connection must fail") } catch(_: IllegalStateException) { }
        assertFalse(called)
        val copy = async(Dispatchers.IO) { runCatching { platform.migrateConnection(connection) } }
        entered.await(); platform.cancelFolderMigration(); release.complete(Unit)
        assertTrue(copy.await().isFailure)
        assertEquals(connection, platform.connection()); assertFalse(platform.state.value.migratingFolder)
        platform.foreground(false)
    } }
}
