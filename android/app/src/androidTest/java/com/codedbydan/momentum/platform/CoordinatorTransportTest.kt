// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.platform

import android.content.pm.PackageManager
import android.app.NotificationManager
import android.content.Intent
import android.os.Build
import android.os.Bundle
import android.util.AtomicFile
import androidx.test.platform.app.InstrumentationRegistry
import com.codedbydan.momentum.R
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.*
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.first
import org.junit.Assert.*
import org.junit.Test
import java.io.File
import java.net.InetAddress
import java.net.ServerSocket
import java.security.KeyStore
import java.util.UUID
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import java.io.BufferedInputStream
import java.time.LocalDate
import org.json.JSONObject
import kotlin.concurrent.thread

/** Real generated FFI + loopback HTTP, with no WorkManager/notification collection. */
class CoordinatorTransportTest {
    @Test fun editsDuringRestoreCannotReleaseAdmissionBeforeExplicitResume() = runBlocking {
        val fixture = Fixture()
        val server = ServerSocket(0, 1, InetAddress.getByName("127.0.0.1"))
        val entered = CountDownLatch(1)
        val serving = thread(name = "momentum-isolated-restore-admission") {
            runCatching { server.accept().use { socket ->
                socket.soTimeout = 5_000
                val reader = socket.getInputStream().bufferedReader()
                while (!reader.readLine().isNullOrEmpty()) { }
                entered.countDown()
                socket.getOutputStream().write("HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".toByteArray())
            } }
        }
        try {
            fixture.connect(server.localPort)
            fixture.platform.pauseForRestore()
            val revision = fixture.platform.state.value.revision
            fixture.repo.create(TaskDraft("An earlier edit finished", "INBOX_PROJECT", null, null, null,
                0.0, "", emptyList(), emptyList()), View.Today)
            fixture.platform.changed()
            withTimeout(5_000) { fixture.platform.state.first { it.revision > revision } }
            fixture.platform.syncNow(test = true)
            withTimeout(5_000) { fixture.platform.state.first { it.error?.contains("paused") == true } }
            assertEquals("No exchange may start before restore releases ownership", 1L, entered.count)
            fixture.platform.resumeAfterRestore()
            fixture.platform.syncNow(test = true)
            withTimeout(5_000) { fixture.platform.state.first { !it.syncing && it.failureKind == SyncFailureKind.UNAUTHORIZED } }
            assertEquals(0L, entered.count)
        } finally { server.close(); serving.join(5_000); fixture.close() }
    }

    @Test fun syncSetupFailureCannotKeepAPostedPrivateReminderAfterOptOut() = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        org.junit.Assume.assumeTrue("OS tests require an explicitly disposable emulator",
            InstrumentationRegistry.getArguments().getString("momentum.disposableOsTests") == "true" &&
                Build.HARDWARE in listOf("ranchu", "goldfish"))
        val notifications = context.getSystemService(NotificationManager::class.java)
        org.junit.Assume.assumeTrue("Grant notifications in the disposable environment", notifications.areNotificationsEnabled())
        val fixture = Fixture(allowDisposableNotifications = true)
        val scheduler = ReminderScheduler(context, fixture.repo, isolated = false)
        try {
            fixture.repo.updateSettings { it.copy(notifications = true, automaticSync = false) }
            val id = fixture.repo.create(TaskDraft("Private opt-out fixture", "INBOX_PROJECT", LocalDate.now().plusDays(1).toString(),
                ClockTime(12u, 0u), 0u, 0.0, "", emptyList(), emptyList()), View.Today).id!!
            val request = fixture.repo.read { notificationPlan(System.currentTimeMillis().toULong()).requests.single() }
            val key = ReminderIdentity.key(ReminderIdentity.observation(request))
            scheduler.reconcile()
            assertTrue(scheduler.receive(Intent().putExtra("momentum.notification.key", key)))
            withTimeout(5_000) { while(notifications.activeNotifications.none { it.tag == key }) delay(20) }
            // Invalid native credentials make foreground setup fail before it opens transport.
            assertTrue(SecretVault(fixture.directory).set("device-cert-der", "AA=="))
            fixture.platform.selectProvider(SyncProvider.LibreSync)
            fixture.platform.foreground(true)
            assertTrue(fixture.platform.state.value.error!!.contains("credentials are incomplete"))
            assertTrue(notifications.activeNotifications.any { it.tag == key })
            fixture.repo.updateSettings { it.copy(notifications = false) }
            fixture.platform.changed()
            withTimeout(5_000) { while(notifications.activeNotifications.any { it.tag == key }) delay(20) }
            assertFalse(fixture.repo.settings().notifications)
            assertFalse(fixture.repo.detail(id)!!.isDone)
            assertTrue(fixture.platform.state.value.error!!.contains("credentials are incomplete"))
        } finally { scheduler.cancelAndDrain(); fixture.close() }
    }

    @Test fun confirmedDamagedCopyRecoveryParksTheOldCopyAndPublishesTheLocalTasks() = runBlocking {
        val fixture = Fixture()
        val server = RecoveryServer()
        try {
            fixture.connect(server.socket.localPort)
            val id = fixture.repo.create(TaskDraft("Recovery fixture", "INBOX_PROJECT", null, null, null,
                0.0, "", emptyList(), emptyList()), View.Today).id!!
            // The core intentionally requires a complete task dataset for server replacement.
            val backup = File(fixture.directory, "fixture-backup.json")
            fixture.repo.read { exportBackup(backup.absolutePath) }
            val json = JSONObject(backup.readText())
            json.getJSONObject("data").put("globalConfig", JSONObject())
            backup.writeText(json.toString())
            fixture.repo.read { importBackup(backup.absolutePath) }
            fixture.platform.syncNow()
            withTimeout(5_000) { fixture.platform.state.first { !it.syncing && it.failureKind == SyncFailureKind.REMOTE_FILE_DAMAGED } }
            assertEquals(0, server.copies)
            val damaged = fixture.platform.state.value
            fixture.platform.syncNow(test = true)
            val probed = withTimeout(5_000) { fixture.platform.state.first {
                !it.syncing && it.information == R.string.connection_ok && it.revision > damaged.revision
            } }
            assertEquals(SyncFailureKind.REMOTE_FILE_DAMAGED, probed.failureKind)
            assertEquals(damaged.errorDetails, probed.errorDetails)
            assertEquals(0uL, fixture.repo.read { syncStatus().lastNextcloudMs })
            fixture.platform.saveConnection(fixture.platform.connection()!!)
            assertEquals(SyncFailureKind.REMOTE_FILE_DAMAGED, fixture.platform.state.value.failureKind)
            assertEquals(damaged.errorDetails, fixture.platform.state.value.errorDetails)
            assertEquals(0, server.copies)
            val statusFile = fixture.statusFile.baseFile
            val previousStatus = statusFile.readBytes()
            // API26 AtomicFile writes base/.bak, newer releases use .new.
            // Fault only this receipt directory, never the core store.
            val writeFault = statusFile.parentFile!!
            assertTrue(statusFile.setWritable(false, false))
            assertTrue(writeFault.setWritable(false, false))
            fixture.platform.replaceServerCopy()
            withTimeout(5_000) { fixture.platform.state.first { !it.syncing && it.error == null && it.revision > 0 } }
            assertEquals(1, server.copies)
            assertEquals("damaged-fixture", server.parked)
            val uploaded = JSONObject(server.body!!.substringAfter("__"))
            assertEquals("Recovery fixture", uploaded.getJSONObject("state").getJSONObject("task").getJSONObject("entities").getJSONObject(id).getString("title"))
            assertTrue(fixture.repo.read { syncStatus().lastNextcloudMs } > 0uL)
            assertEquals("Recovery fixture", fixture.repo.detail(id)!!.title)
            assertNotNull("A status receipt failure cannot conceal the real exchange result",
                fixture.platform.state.value.statusPersistenceError)
            assertArrayEquals(previousStatus, statusFile.readBytes())
            val lastSuccess = fixture.repo.read { syncStatus().lastNextcloudMs }
            // Retain the newest desired status, never an older success receipt.
            fixture.platform.reportFailure(CoreException.Actionable(SyncFailureKind.UNREACHABLE, "Latest isolated failure"))
            fixture.platform.retryStatusPersistence()
            delay(100)
            assertNotNull(fixture.platform.state.value.statusPersistenceError)
            assertEquals(SyncFailureKind.UNREACHABLE, fixture.platform.state.value.failureKind)
            assertEquals("Latest isolated failure", fixture.platform.state.value.errorDetails)
            assertArrayEquals(previousStatus, statusFile.readBytes())
            assertTrue(writeFault.setWritable(true, true))
            assertTrue(statusFile.setWritable(true, true))
            fixture.platform.retryStatusPersistence()
            withTimeout(5_000) { fixture.platform.state.first { it.statusPersistenceError == null } }
            assertEquals("Latest isolated failure", JSONObject(statusFile.readText()).getString("error"))
            assertEquals(SyncFailureKind.UNREACHABLE.name, JSONObject(statusFile.readText()).getString("failure-kind"))
            assertEquals(1, server.copies)
            assertEquals(lastSuccess, fixture.repo.read { syncStatus().lastNextcloudMs })
            val restarted = PlatformCoordinator(fixture.context, fixture.repo, false, fixture.statusFile)
            val restored = withTimeout(5_000) { restarted.state.first { it.failureKind == SyncFailureKind.UNREACHABLE } }
            assertEquals("Latest isolated failure", restored.errorDetails)
            assertNull(restored.statusPersistenceError)
            restarted.foreground(false)
        } finally { server.close(); fixture.close() }
    }

    @Test fun stoppedBackgroundCallerCancelsAndDrainsItsNativeExchange() = runBlocking {
        val fixture = Fixture()
        val server = ServerSocket(0, 1, InetAddress.getByName("127.0.0.1"))
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        val serving = thread(name = "momentum-isolated-worker") {
            runCatching { server.accept().use { socket ->
                socket.soTimeout = 5_000
                val reader = socket.getInputStream().bufferedReader()
                while (!reader.readLine().isNullOrEmpty()) { }
                entered.countDown(); release.await(5, TimeUnit.SECONDS)
                socket.getOutputStream().write("HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".toByteArray())
            } }
        }
        try {
            fixture.connect(server.localPort)
            fixture.repo.updateSettings { it.copy(automaticSync = true, backgroundSync = true) }
            val worker = async(Dispatchers.IO) { fixture.platform.backgroundSync(ignoreIsolationForTests = true) }
            assertTrue(withContext(Dispatchers.IO) { entered.await(5, TimeUnit.SECONDS) })
            worker.cancel()
            delay(100)
            assertFalse("Stopped worker must drain the native call before releasing ownership", worker.isCompleted)
            release.countDown()
            withTimeout(5_000) { worker.join() }
            assertFalse(fixture.platform.state.value.syncing)
            assertEquals(0uL, fixture.repo.read { syncStatus().lastNextcloudMs })
        } finally { release.countDown(); server.close(); serving.join(5_000); fixture.close() }
    }

    @Test fun restoreCancelsAndDrainsNetworkingWhileOfflineEditsRemainUsable() = runBlocking {
        val fixture = Fixture()
        val server = ServerSocket(0, 1, InetAddress.getByName("127.0.0.1"))
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        val serving = thread(name = "momentum-isolated-dav") {
            runCatching { server.accept().use { socket ->
                socket.soTimeout = 5_000
                val reader = socket.getInputStream().bufferedReader()
                while (!reader.readLine().isNullOrEmpty()) { }
                entered.countDown()
                release.await(5, TimeUnit.SECONDS)
                socket.getOutputStream().write("HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".toByteArray())
            } }
        }
        try {
            fixture.connect(server.localPort)
            fixture.platform.syncNow()
            assertTrue(withContext(Dispatchers.IO) { entered.await(5, TimeUnit.SECONDS) })
            val id = withTimeout(2_000) {
                fixture.repo.create(TaskDraft("Offline during exchange", "INBOX_PROJECT", null, null, null,
                    0.0, "", emptyList(), emptyList()), View.Today).id!!
            }
            val barrierEntered = CompletableDeferred<Unit>()
            val barrier = async(Dispatchers.IO) { barrierEntered.complete(Unit); fixture.platform.pauseForRestore() }
            barrierEntered.await()
            delay(100)
            assertFalse("Cancellation must retain and drain the pending native HTTP call", barrier.isCompleted)
            release.countDown()
            withTimeout(5_000) { barrier.await() }
            assertFalse(fixture.platform.state.value.syncing)
            assertNull(fixture.platform.state.value.information)
            assertEquals(0uL, fixture.repo.read { syncStatus().lastNextcloudMs })
            fixture.platform.selectProvider(SyncProvider.Off)
            assertEquals(SyncProvider.Off, fixture.repo.settings().provider)
            assertEquals("Offline during exchange", fixture.repo.detail(id)!!.title)
            assertNotNull(fixture.platform.connection())
        } finally {
            release.countDown(); server.close(); serving.join(5_000); fixture.close()
        }
    }

    @Test fun failedConnectionTestNeverReportsSuccessAndErrorSurvivesCoordinatorRestart() = runBlocking {
        val fixture = Fixture()
        val server = ServerSocket(0, 1, InetAddress.getByName("127.0.0.1"))
        val serving = thread(name = "momentum-isolated-auth") {
            runCatching { server.accept().use { socket ->
                socket.soTimeout = 5_000
                val reader = socket.getInputStream().bufferedReader()
                while (!reader.readLine().isNullOrEmpty()) { }
                socket.getOutputStream().write("HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".toByteArray())
            } }
        }
        try {
            fixture.connect(server.localPort)
            fixture.platform.syncNow(test = true)
            val failure = withTimeout(5_000) { fixture.platform.state.first { it.error != null && !it.syncing } }
            assertTrue(failure.error!!.contains("401"))
            assertEquals(SyncFailureKind.UNAUTHORIZED, failure.failureKind)
            assertEquals(failure.error, failure.errorDetails)
            assertNull(failure.information)
            assertEquals(0uL, fixture.repo.read { syncStatus().lastNextcloudMs })
            fixture.platform.foreground(false)
            val restarted = PlatformCoordinator(fixture.context, fixture.repo, false, fixture.statusFile)
            val restored = withTimeout(5_000) { restarted.state.first { it.error != null } }
            assertEquals(failure.error, restored.error)
            assertEquals(SyncFailureKind.UNAUTHORIZED, restored.failureKind)
            restarted.foreground(false)
        } finally { server.close(); serving.join(5_000); fixture.close() }
    }

    private class Fixture(allowDisposableNotifications: Boolean = false) {
        private val instrumentation = InstrumentationRegistry.getInstrumentation()
        val context = instrumentation.targetContext
        init {
            if (Build.VERSION.SDK_INT >= 37 && context.checkSelfPermission(LOCAL_NETWORK_PERMISSION) != PackageManager.PERMISSION_GRANTED) {
                // Even loopback sockets require this permission on Android 17. Revocation
                // kills the instrumented app, so the host restores the prior denied grant
                // after the run; the status record makes that cleanup requirement explicit.
                instrumentation.uiAutomation.grantRuntimePermission(context.packageName, LOCAL_NETWORK_PERMISSION)
                instrumentation.sendStatus(0, Bundle().apply {
                    putString("momentum.fixture.permission_restore", "$LOCAL_NETWORK_PERMISSION=denied")
                })
                check(context.checkSelfPermission(LOCAL_NETWORK_PERMISSION) == PackageManager.PERMISSION_GRANTED)
            }
        }
        val directory = File(context.cacheDir, "transport-test-${UUID.randomUUID()}").also { it.mkdirs() }
        val repo = MomentumRepository(context, directory)
        val statusDirectory = File(directory, "status-receipts").also { check(it.mkdirs()) }
        val statusFile = AtomicFile(File(statusDirectory, "android-platform-state.json"))
        val platform = PlatformCoordinator(context, repo, allowDisposableNotifications, statusFile)
        suspend fun connect(port: Int) {
            repo.updateSettings(AppSettings(automaticSync = false))
            platform.saveConnection(NextcloudSettings("http://127.0.0.1:$port", "fixture-user", "fixture-password", "Momentum", false, null))
            assertNull(platform.state.value.error)
            platform.selectProvider(SyncProvider.Nextcloud)
        }
        suspend fun close() {
            statusDirectory.setWritable(true, true)
            statusFile.baseFile.setWritable(true, true)
            platform.foreground(false)
            repo.close()
            KeyStore.getInstance("AndroidKeyStore").apply { load(null); deleteEntry(SecretVault(directory).keyAlias) }
            directory.deleteRecursively()
        }
        companion object { private const val LOCAL_NETWORK_PERMISSION = "android.permission.ACCESS_LOCAL_NETWORK" }
    }

    private class RecoveryServer {
        val socket = ServerSocket(0, 4, InetAddress.getByName("127.0.0.1"))
        private val running = AtomicBoolean(true)
        @Volatile var copies = 0
        @Volatile var parked: String? = null
        @Volatile var body: String? = "damaged-fixture"
        private val serving = thread(name = "momentum-isolated-recovery") {
            while (running.get()) runCatching { socket.accept().use { connection ->
                connection.soTimeout = 5_000
                val input = BufferedInputStream(connection.getInputStream())
                val request = line(input).split(' ')
                val headers = linkedMapOf<String, String>()
                while (true) {
                    val header = line(input)
                    if (header.isEmpty()) break
                    val parts = header.split(':', limit = 2)
                    if (parts.size == 2) headers[parts[0].lowercase()] = parts[1].trim()
                }
                val length = headers["content-length"]?.toInt() ?: 0
                check(length in 0..1_048_576)
                val bytes = ByteArray(length)
                var offset = 0
                while (offset < length) { val count = input.read(bytes, offset, length - offset); check(count > 0); offset += count }
                val (status, payload) = when (request.first()) {
                    "GET" -> 200 to body.orEmpty()
                    "PROPFIND" -> 207 to "<d:multistatus xmlns:d=\"DAV:\"><d:getcontentlength>${body.orEmpty().toByteArray().size}</d:getcontentlength><d:getetag>&quot;fixture&quot;</d:getetag></d:multistatus>"
                    "COPY" -> {
                        check(headers["destination"]?.endsWith("sync-data.json.damaged") == true)
                        parked = body; copies++; 201 to ""
                    }
                    "PUT" -> { body = bytes.toString(Charsets.UTF_8); 201 to "" }
                    else -> 400 to ""
                }
                val response = "HTTP/1.1 $status Fixture\r\nContent-Length: ${payload.toByteArray().size}\r\nOC-ETag: \"fixture\"\r\nConnection: close\r\n\r\n$payload"
                connection.getOutputStream().write(response.toByteArray())
            } }
        }
        fun close() { running.set(false); socket.close(); serving.join(5_000) }
        private fun line(input: BufferedInputStream): String {
            val result = StringBuilder()
            while (true) { val byte = input.read(); check(byte >= 0); if (byte == 10) break; if (byte != 13) result.append(byte.toChar()); check(result.length <= 8_192) }
            return result.toString()
        }
    }
}
