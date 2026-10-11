// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.platform

import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.os.SystemClock
import android.util.AtomicFile
import androidx.test.platform.app.InstrumentationRegistry
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.MomentumRepository
import com.codedbydan.momentum.data.SyncProvider
import kotlinx.coroutines.*
import org.json.JSONObject
import org.junit.Assume
import org.junit.Test
import java.io.File
import java.security.KeyStore
import java.util.UUID

/** Opt-in two-process acceptance; the host transports private commands over adb stdin. */
class LibreSyncTwoEmulatorTest {
    @Test fun productionAdapterPairsAndConvergesOfflineChangesAcrossTwoEmulators() = runBlocking {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val args = InstrumentationRegistry.getArguments()
        Assume.assumeTrue(args.getString("momentum.disposableLibreSync") == "true" &&
            Build.HARDWARE in listOf("ranchu", "goldfish"))
        val run = requireNotNull(args.getString("momentum.fixture.run"))
        require(UUID.fromString(run).toString() == run)
        val role = requireNotNull(args.getString("momentum.fixture.role"))
        require(role in setOf("alpha", "beta"))
        val context = instrumentation.targetContext
        val root = File(context.cacheDir, "libresync-two-$run-$role")
        require(root.canonicalFile == File(context.cacheDir.canonicalFile, root.name))
        val owner = "$run:$role"
        val marker = File(root, "owner")
        val directory = File(root, "store")
        fun removeFixture() {
            check(!root.exists() || marker.readText() == owner)
            KeyStore.getInstance("AndroidKeyStore").apply { load(null); deleteEntry(SecretVault(directory).keyAlias) }
            check(!root.exists() || root.deleteRecursively())
        }
        if(args.getString("momentum.fixture.cleanup") == "true") { removeFixture(); return@runBlocking }
        check(!root.exists() && root.mkdirs())
        marker.writeText(owner)
        val status = AtomicFile(File(root, "status.json"))
        val pairing = AtomicFile(File(root, "pairing.json"))
        val command = File(root, "command.json")
        var repository: MomentumRepository? = null
        var platform: PlatformCoordinator? = null
        var sequence = 0
        var phase = "starting"
        var offlineStamp: ULong? = null
        fun publish(value: String) {
            phase = value
            write(status, JSONObject().put("phase", value).put("sequence", sequence))
        }
        suspend fun await(condition: suspend () -> Boolean) = withTimeout(45_000) {
            while(!condition()) delay(100)
        }
        val deadline = SystemClock.elapsedRealtime() + 240_000
        try {
            if(Build.VERSION.SDK_INT >= 37 && context.checkSelfPermission(LAN) != PackageManager.PERMISSION_GRANTED) {
                instrumentation.uiAutomation.grantRuntimePermission(context.packageName, LAN)
                instrumentation.sendStatus(0, Bundle().apply { putString("momentum.fixture.permission_restore", "$LAN=denied") })
                check(context.checkSelfPermission(LAN) == PackageManager.PERMISSION_GRANTED)
            }
            var repo = MomentumRepository(context, directory).also { repository = it }
            var adapter = PlatformCoordinator(context, repo).also { platform = it }
            if(role == "alpha") for(title in INITIAL) {
                check(repo.create(draft(title, "Initial fixture notes"), View.Project("INBOX_PROJECT")).outcome.changed)
            }
            adapter.selectProvider(SyncProvider.LibreSync)
            adapter.foreground(true)
            await { repo.read { p2pInfo() != null } }
            val originalIdentity = repo.read { p2pInfo()!!.deviceId }
            check(repo.read { syncStatus().lastNearbyMs } == 0uL)
            check(SecretVault(directory).get("device-cert-der") != null)
            check(SecretVault(directory).get("device-key-der") != null)
            if(role == "alpha") {
                adapter.pairing(true)
                await { adapter.state.value.pairingCode != null }
            }
            write(pairing, JSONObject().put("port", repo.read { p2pInfo()!!.port.toInt() })
                .put("code", adapter.state.value.pairingCode ?: JSONObject.NULL))
            publish("ready")
            while(SystemClock.elapsedRealtime() < deadline) {
                if(!command.exists()) { delay(100); continue }
                check(command.length() in 1..4096)
                val input = JSONObject(command.readText())
                check(command.delete())
                val next = input.getInt("sequence")
                check(next == sequence + 1 && next <= 32)
                sequence = next
                val operation = input.getString("operation")
                check(operation in setOf("link", "linked", "sync", "initial", "offline", "edit", "online", "verify", "reopen", "stop"))
                phase = operation
                when(operation) {
                    "link" -> {
                        check(role == "beta")
                        val address = input.getString("address")
                        check(address.matches(Regex("10\\.0\\.2\\.2:[0-9]{1,5}")) && address.substringAfter(':').toInt() in 1..65535)
                        val code = input.getString("code")
                        check(code.matches(Regex("[0-9]{6}")))
                        adapter.link(address, code)
                        await { adapter.state.value.devices.size == 1 }
                        publish("linked")
                    }
                    "linked" -> {
                        await { adapter.state.value.devices.size == 1 }
                        adapter.pairing(false)
                        await { adapter.state.value.pairingCode == null }
                        pairing.delete()
                        publish("linked")
                    }
                    "sync" -> {
                        check(role == "beta")
                        val before = repo.read { syncStatus().lastNearbyMs }
                        adapter.syncNow()
                        await { repo.read { syncStatus().lastNearbyMs } > before && !adapter.state.value.syncing }
                        check(adapter.state.value.error == null)
                        publish("synced")
                    }
                    "initial" -> {
                        await { repo.read { allTasks().map { it.title }.toSet() == INITIAL.toSet() } }
                        check(repo.read { syncStatus().lastNearbyMs } > 0uL)
                        publish("initial")
                    }
                    "offline" -> {
                        adapter.foreground(false)
                        check(!repo.read { p2pRunning() })
                        offlineStamp = repo.read { syncStatus().lastNearbyMs }
                        publish("offline")
                    }
                    "edit" -> {
                        check(offlineStamp != null && !repo.read { p2pRunning() })
                        repo.read {
                            val tasks = allTasks().associate { it.title to it.id }
                            if(role == "alpha") {
                                check(saveTask(tasks.getValue("Alpha editable"), draft("Alpha edited", "Alpha offline notes")).changed)
                                check(setDone(tasks.getValue("Completion fixture"), true).changed)
                                check(createTaskWithId(draft("Alpha offline addition", "Alpha added notes"), View.Project("INBOX_PROJECT")).outcome.changed)
                            } else {
                                check(saveTask(tasks.getValue("Beta editable"), draft("Beta edited", "Beta offline notes")).changed)
                                check(deleteTask(tasks.getValue("Deletion fixture")).changed)
                                check(createTaskWithId(draft("Beta offline addition", "Beta added notes"), View.Project("INBOX_PROJECT")).outcome.changed)
                            }
                        }
                        check(repo.read { syncStatus().lastNearbyMs } == offlineStamp)
                        publish("edited")
                    }
                    "online" -> {
                        adapter.foreground(true)
                        await { repo.read { p2pInfo() != null } }
                        check(repo.read { p2pInfo()!!.deviceId } == originalIdentity)
                        write(pairing, JSONObject().put("port", repo.read { p2pInfo()!!.port.toInt() }))
                        publish("online")
                    }
                    "verify" -> { await { converged(repo) }; publish("converged") }
                    "reopen" -> {
                        check(converged(repo))
                        adapter.foreground(false)
                        val vault = SecretVault(directory)
                        val credentials = listOf("device-cert-der", "device-key-der", "app-key").map(vault::get)
                        check(credentials.all { it != null })
                        repo.close()
                        repo = MomentumRepository(context, directory).also { repository = it }
                        adapter = PlatformCoordinator(context, repo).also { platform = it }
                        check(converged(repo) && repo.settings().provider == SyncProvider.LibreSync)
                        adapter.foreground(true)
                        await { repo.read { p2pInfo() != null } }
                        check(repo.read { p2pInfo()!!.deviceId } == originalIdentity)
                        check(repo.read { p2pDevices().size } == 1)
                        check(credentials == listOf("device-cert-der", "device-key-der", "app-key").map(vault::get))
                        publish("reopened")
                    }
                    "stop" -> { publish("stopped"); return@runBlocking }
                }
            }
            error("Fixture command deadline exceeded")
        } catch(failure: Throwable) {
            // Do not let framework/JUnit exception formatting disclose identities,
            // pairing credentials, native server addresses or task contents.
            val failedPhase = phase
            runCatching { publish("failed") }
            throw AssertionError("Disposable LibreSync $failedPhase failed: ${failure.javaClass.simpleName}")
        } finally {
            var cleaned = true
            withContext(NonCancellable) {
                try { platform?.foreground(false) } catch(_: Throwable) { cleaned = false }
                try { repository?.close() } catch(_: Throwable) { cleaned = false }
                try { removeFixture() } catch(_: Throwable) { cleaned = false }
            }
            check(cleaned) { "Disposable LibreSync cleanup requires the scoped cleanup command" }
        }
    }

    private suspend fun converged(repo: MomentumRepository): Boolean = repo.read {
        val rows = allTasks()
        if(rows.map { it.title }.toSet() != FINAL || rows.size != FINAL.size) return@read false
        val details = rows.associate { it.title to taskDetail(it.id)!! }
        details.getValue("Alpha edited").notes == "Alpha offline notes" &&
            details.getValue("Beta edited").notes == "Beta offline notes" &&
            details.getValue("Completion fixture").isDone &&
            details.getValue("Alpha offline addition").notes == "Alpha added notes" &&
            details.getValue("Beta offline addition").notes == "Beta added notes" &&
            syncStatus().lastNearbyMs > 0uL
    }
    private fun draft(title: String, notes: String) = TaskDraft(title, "INBOX_PROJECT", null, null, null, 0.0, notes, emptyList(), emptyList())
    private fun write(file: AtomicFile, json: JSONObject) {
        val output = file.startWrite()
        try { output.write(json.toString().toByteArray(Charsets.UTF_8)); file.finishWrite(output) }
        catch(failure: Throwable) { file.failWrite(output); throw failure }
    }
    companion object {
        private const val LAN = "android.permission.ACCESS_LOCAL_NETWORK"
        private val INITIAL = listOf("Alpha editable", "Beta editable", "Completion fixture", "Deletion fixture")
        private val FINAL = setOf("Alpha edited", "Beta edited", "Completion fixture", "Alpha offline addition", "Beta offline addition")
    }
}
