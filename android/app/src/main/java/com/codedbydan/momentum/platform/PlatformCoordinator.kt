// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.platform

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.PackageManager
import android.net.wifi.WifiManager
import android.os.Build
import android.os.PowerManager
import android.util.AtomicFile
import androidx.core.content.ContextCompat
import androidx.work.*
import com.codedbydan.momentum.R
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.*
import kotlinx.coroutines.*
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import org.json.JSONObject
import java.io.File
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicLong

data class PlatformState(
    val syncing: Boolean = false,
    val migratingFolder: Boolean = false,
    val error: String? = null,
    val errorDetails: String? = null,
    val failureKind: SyncFailureKind? = null,
    val information: Int? = null,
    val statusPersistenceError: String? = null,
    val revision: Long = 0,
    val pairingCode: String? = null,
    val devices: List<NearbyDevice> = emptyList(),
    val discovered: List<NearbyDevice> = emptyList(),
    val taskCountError: String? = null,
    val taskCountBlocked: Boolean = false,
    val searchUpdating: Boolean = false,
    val searchError: String? = null,
    val searchIndexed: Int = 0,
    val searchUpdatedAt: Long = 0,
    val searchCleared: Boolean = false,
)

internal data class PlatformFailure(val kind: SyncFailureKind?, val details: String) {
    companion object {
        fun from(error: Throwable): PlatformFailure = when (error) {
            is CoreException.Actionable -> PlatformFailure(error.kind, error.detail)
            is CoreException.Transient -> PlatformFailure(error.kind, error.detail)
            is CoreException.Io -> PlatformFailure(null, error.detail)
            is CoreException.Invalid -> PlatformFailure(null, error.detail)
            is CoreException.Nearby -> PlatformFailure(null, error.detail)
            else -> PlatformFailure(null, error.message ?: error.javaClass.simpleName)
        }
    }
}

internal fun AppSettings.backgroundSyncEligible(): Boolean =
    provider == SyncProvider.Nextcloud && automaticSync && backgroundSync

/** Process-owned native services. The control queue is separate from local edits. */
class PlatformCoordinator internal constructor(context: Context, private val repository: MomentumRepository,
    allowDisposableNotifications: Boolean,
    private val stateFile: AtomicFile = AtomicFile(File(repository.directory, "android-platform-state.json")),
    private val migrateFolder: suspend (Engine, NextcloudSettings, SyncCancellation) -> NextcloudFolderMigrationReport = { engine, settings, token ->
        engine.migrateNextcloudFolder(settings, "momentum", token, 120_000uL)
    },
    private val saveMigratedConnection: ((NextcloudSettings) -> Unit)? = null,
    private val exchangeNextcloud: (Engine, NextcloudSettings, SyncCancellation, ULong) -> Unit = { engine, connection, token, budget ->
        engine.syncNextcloudWithBudget(connection, token, budget)
    },
) {
    constructor(context: Context, repository: MomentumRepository) : this(context, repository, false)
    private val context = context.applicationContext
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val control = Mutex()
    private val mutable = MutableStateFlow(PlatformState())
    val state: StateFlow<PlatformState> = mutable.asStateFlow()
    private val isolated = (repository.directory.canonicalFile != File(this.context.noBackupFilesDir, "tasks").canonicalFile).also {
        require(!allowDisposableNotifications || (it && Build.HARDWARE in listOf("ranchu", "goldfish"))) {
            "OS notification tests require a disposable store on an emulator"
        }
    }
    private val vault = SecretVault(repository.directory)
    init {
        require(stateFile.baseFile.canonicalFile == File(repository.directory, "android-platform-state.json").canonicalFile ||
            (isolated && stateFile.baseFile.canonicalFile.toPath().startsWith(repository.directory.canonicalFile.toPath())))
    }
    private val reminders = ReminderScheduler(this.context, repository, isolated && !allowDisposableNotifications)
    private val taskCount = TaskCountNotifier(this.context, repository, isolated && !allowDisposableNotifications)
    private val taskSearch = if(!isolated && Build.VERSION.SDK_INT >= 31) TaskSearchIndex(this.context, repository) else null
    private var exchange: Exchange? = null
    @Volatile private var migrationCancellation: SyncCancellation? = null
    @Volatile private var foregroundRequested = false
    @Volatile private var restorePaused = false
    @Volatile private var pairingRequested = false
    private val pairingSequence = AtomicLong()
    private var generation = 0L
    private var automaticJob: Job? = null
    private var debounceJob: Job? = null
    private var multicast: WifiManager.MulticastLock? = null
    private var nearbyEvents: Channel<P2pEvent>? = null
    private var nearbyEventJob: Job? = null
    private var powerReceiverRegistered = false
    @Volatile private var errorUpdated = false
    private data class PersistedStatus(val error: String?, val kind: SyncFailureKind?)
    private var desiredStatus: PersistedStatus? = null
    private data class Exchange(val cancellation: SyncCancellation, val job: Deferred<Boolean>)
    private val powerReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            scope.launch { guarded { control.withLock { configureForeground() } } }
        }
    }

    init {
        scope.launch { taskCount.state.collect { status -> mutable.update { it.copy(taskCountError = status.error, taskCountBlocked = status.blocked) } } }
        taskSearch?.let { index -> scope.launch {
            index.state.collect { status -> mutable.update { it.copy(searchUpdating = status.updating,
                searchError = status.error, searchIndexed = status.indexed, searchUpdatedAt = status.updatedAt,
                searchCleared = status.cleared) } }
        } }
        scope.launch {
            control.withLock {
                runCatching {
                    val json = synchronized(this@PlatformCoordinator) {
                        if (errorUpdated || !stateFile.baseFile.exists()) null
                        else JSONObject(stateFile.openRead().use { it.readBytes().toString(Charsets.UTF_8) })
                    }
                    if (json != null) {
                        val detail = if (json.isNull("error")) null else json.getString("error")
                        val kind = json.optString("failure-kind").let { value -> SyncFailureKind.entries.firstOrNull { it.name == value } }
                        mutable.update {
                            if (errorUpdated) it else it.copy(error = detail, errorDetails = detail, failureKind = if (detail != null) kind else null)
                        }
                    }
                }.onFailure { recordError(it) }
            }
        }
    }

    suspend fun foreground(active: Boolean) {
        foregroundRequested = active
        if(active) taskSearch?.requestReconcile()
        withContext(Dispatchers.IO) {
            guarded { control.withLock {
                guarded { configureForeground() }
                guarded { configureBackground() }
            } }
            // Privacy and local reminders remain independent of sync setup failures.
            reconcileReminders(waitForCount = false)
        }
    }

    /** Only the restore owner releases this barrier, after import has returned or failed. */
    suspend fun resumeAfterRestore(): Unit = withContext(NonCancellable + Dispatchers.IO) {
        guarded { control.withLock {
            restorePaused = false
            reminders.resume()
            taskCount.resume()
            taskSearch?.resume()
            guarded { configureForeground() }
            guarded { configureBackground() }
            mutable.update { it.copy(revision = it.revision + 1) }
        } }
        reconcileReminders()
        Unit
    }

    fun leaveForeground() {
        foregroundRequested = false // Admission stops immediately, before the coroutine drains.
        scope.launch { guarded { control.withLock { configureForeground() } } }
    }

    fun changed() {
        scope.launch {
            guarded {
                control.withLock {
                    mutable.update { it.copy(revision = it.revision + 1) }
                    if (restorePaused) return@withLock
                    reminders.resume()
                    taskCount.resume()
                    taskSearch?.resume()
                    guarded { configureForeground() }
                    guarded { configureBackground() }
                    debounceJob?.cancel()
                    if (foregroundRequested && !savingPower()) debounceJob = scope.launch {
                        delay(10_000)
                        if (repository.settings().automaticSync) automaticSync()
                    }
                }
            }
            reconcileReminders()
        }
    }

    /** Cancellation is not completion: retain the native token and await actual I/O. */
    suspend fun pauseForRestore() {
        restorePaused = true
        reminders.pause()
        taskCount.pause()
        taskCount.cancelAndDrain()
        withContext(Dispatchers.IO) { control.withLock {
            debounceJob?.cancel(); automaticJob?.cancel(); automaticJob = null
            drainExchange()
            stopNearby()
        } }
        reminders.cancelAndDrain()
        check(taskSearch?.pauseAndClear() != false) { "System search could not be cleared before restore. Retry before replacing tasks." }
    }

    suspend fun setSystemSearch(enabled: Boolean) {
        val index = taskSearch
        if(index == null) {
            if(enabled && !isolated) mutable.update { it.copy(searchError = context.getString(R.string.system_search_unavailable)) }
            return
        }
        if(index.setEnabled(enabled)) changed()
        else mutable.update { it.copy(revision = it.revision + 1) }
    }

    fun retrySystemSearch() { taskSearch?.requestReconcile() }

    suspend fun selectProvider(provider: SyncProvider) = withContext(Dispatchers.IO) {
        guarded { control.withLock {
            val old = repository.settings()
            if (old.provider != provider) {
                debounceJob?.cancel()
                drainExchange()
                stopNearby()
                repository.updateSettings { it.copy(provider = provider) }
                pairingRequested = false
                pairingSequence.incrementAndGet()
            }
            guarded { configureForeground() }
            guarded { configureBackground() }
            mutable.update { it.copy(revision = it.revision + 1) }
        } }
    }

    suspend fun connection(): NextcloudSettings? = withContext(Dispatchers.IO) {
        try { vault.connection() } catch (e: Exception) { if (e is CancellationException) throw e; recordError(e); null }
    }

    suspend fun saveConnection(settings: NextcloudSettings) = withContext(Dispatchers.IO) {
        guarded {
        require(ConnectionPolicy.accepts(settings.serverUrl, isolated)) { "Use an HTTPS server URL without credentials, query or fragment" }
        require(settings.userName.isNotBlank() && settings.password.isNotBlank() && settings.folder.isNotBlank()) { "Complete the server, user, app password and sync folder" }
        control.withLock {
            drainExchange()
            val saved = settings.copy(serverUrl = settings.serverUrl.trim())
            val previous = vault.connection()
            vault.saveConnection(saved)
            // A new target/configuration invalidates the old recovery dialog. Saving
            // the same connection does not prove its previously failed sync is fixed.
            if (previous != saved) clearError(R.string.connection_saved)
            else mutable.update { it.copy(information = R.string.connection_saved) }
            configureBackground()
        }
        }
    }

    /** Explicit checked copy. Other provider controls queue until the switch is committed. */
    suspend fun migrateConnection(expected: NextcloudSettings): NextcloudFolderMigrationReport = withContext(Dispatchers.IO) {
        control.withLock {
            check(!restorePaused && repository.settings().provider == SyncProvider.Nextcloud)
            check(vault.connection() == expected) { "The saved connection changed. Review it before copying the folder." }
            check(expected.folder != "momentum") { "This connection already uses the Momentum folder." }
            val token = SyncCancellation()
            migrationCancellation = token
            restorePaused = true
            mutable.update { it.copy(migratingFolder = true, information = null) }
            try {
                debounceJob?.cancel(); automaticJob?.cancel(); automaticJob = null
                drainExchange(); stopNearby()
                val engine = repository.read { this }
                val report = migrateFolder(engine, expected, token)
                currentCoroutineContext().ensureActive()
                check(!token.isCancelled()) { "Folder copying was cancelled. The original connection is unchanged." }
                check(report.verified && report.sourceFolder == expected.folder && report.targetFolder == "momentum") {
                    "The copied folder was not verified. The original connection is unchanged."
                }
                check(vault.connection() == expected) { "The saved connection changed during copying. Review it before switching." }
                val saved = expected.copy(folder = report.targetFolder)
                if(saveMigratedConnection != null) saveMigratedConnection.invoke(saved) else vault.saveConnection(saved)
                clearError(R.string.folder_migration_complete)
                report
            } catch(error: Exception) {
                if(error !is CancellationException && !token.isCancelled()) recordError(error)
                throw error
            } finally {
                migrationCancellation = null; token.destroy()
                restorePaused = false
                mutable.update { it.copy(migratingFolder = false, revision = it.revision + 1) }
                withContext(NonCancellable + Dispatchers.IO) {
                    guarded { configureForeground() }; guarded { configureBackground() }
                }
            }
        }
    }
    fun cancelFolderMigration() { migrationCancellation?.cancel() }

    fun syncNow(test: Boolean = false) {
        scope.launch { guarded {
            control.withLock {
                val settings = repository.settings()
                if (test) {
                    check(settings.provider == SyncProvider.Nextcloud) { "Select Nextcloud to test the connection" }
                    startNextcloud(test = true, budgetMs = 30_000uL)
                } else when (settings.provider) {
                    SyncProvider.Off -> error("Select a sync provider first")
                    SyncProvider.Nextcloud -> startNextcloud(test = false, budgetMs = 120_000uL)
                    SyncProvider.LibreSync -> {
                        check(foregroundRequested && !restorePaused) { "Keep Momentum open to sync nearby devices" }
                        configureForeground()
                        check(repository.read { p2pRunning() }) { "Nearby sync is paused; check local network access and battery saver" }
                        val engine = repository.read { this }
                        engine.p2pSyncNow()
                    }
                }
            }
        } }
    }

    /** The UI confirms replacing the server copy; ordinary/background retries never do. */
    fun replaceServerCopy() {
        scope.launch { guarded { control.withLock {
            check(!restorePaused && repository.settings().provider == SyncProvider.Nextcloud) { "Select Nextcloud before recovering its server copy" }
            drainExchange()
            // A newer successful exchange or connection change invalidates an old dialog.
            check(mutable.value.failureKind == SyncFailureKind.REMOTE_FILE_DAMAGED) { "Server-copy recovery is no longer available; try syncing again" }
            startNextcloud(test = false, budgetMs = 300_000uL, replace = true)
        } } }
    }

    fun pairing(open: Boolean) {
        pairingRequested = open
        val request = pairingSequence.incrementAndGet()
        scope.launch { guarded { control.withLock {
            if (request != pairingSequence.get()) return@withLock
            configureForeground()
            if (open) {
                check(repository.settings().provider == SyncProvider.LibreSync && foregroundRequested && !restorePaused) { "Select LibreSync and keep Momentum open to pair" }
                check(repository.read { p2pRunning() }) { "Local network permission is required for nearby pairing" }
                val code = repository.read { p2pBeginPairing() }
                mutable.update { it.copy(pairingCode = code) }
            } else {
                repository.read { p2pEndPairing() }
                mutable.update { it.copy(pairingCode = null, discovered = emptyList()) }
            }
            refreshNearby()
        } } }
    }

    suspend fun link(address: String, code: String) = withContext(Dispatchers.IO) {
        guarded { control.withLock {
            check(foregroundRequested && !restorePaused && repository.settings().provider == SyncProvider.LibreSync)
            require(code.trim().matches(Regex("[0-9]{6}"))) { "Enter the six-digit pairing code" }
            require(address.length <= 256 && address.isNotBlank()) { "Enter the device's network address and port" }
            // Pairing performs network I/O; release the repository's edit queue first.
            repository.read { this }.p2pLink(address.trim(), code.trim())
            refreshNearby()
            mutable.update { it.copy(information = null) } // Admission is not a completed link.
        } }
    }

    suspend fun unlink(id: String) = withContext(Dispatchers.IO) {
        guarded { control.withLock {
            check(repository.settings().provider == SyncProvider.LibreSync && foregroundRequested && !restorePaused)
            check(repository.read { p2pRunning() }) { "Nearby sync is paused; check local network access and battery saver" }
            repository.read { p2pUnlink(id) }
            refreshNearby()
        } }
    }

    /** WorkManager admission rechecks opt-in and provider, including after process death. */
    internal suspend fun backgroundSync(ignoreIsolationForTests: Boolean = false): Boolean {
        check(!ignoreIsolationForTests || isolated) { "Test admission is restricted to a disposable store" }
        val started = control.withLock {
            val settings = repository.settings()
            if ((isolated && !ignoreIsolationForTests) || restorePaused || !settings.backgroundSyncEligible() || savingPower()) return true
            startNextcloud(test = false, budgetMs = 20_000uL)
        } ?: return true
        return try { started.job.await() }
        finally {
            if (!currentCoroutineContext().isActive) withContext(NonCancellable + Dispatchers.IO) {
                control.withLock { if (exchange === started) drainExchange() }
            }
        }
    }

    internal suspend fun reconcileReminders(reset: Boolean = false, waitForCount: Boolean = true): Boolean {
        val dayReady = try { prepareLocalDay() } catch (e: Exception) {
            if (e is CancellationException) throw e
            recordError(e)
            false
        }
        if (restorePaused) return true
        val remindersReady = try { reminders.reconcile(reset); true } catch (e: Exception) {
            if (e is CancellationException) throw e
            recordError(e)
            false
        }
        // A reminder receipt failure must not keep a revoked count visible.
        val countReady = if(waitForCount) taskCount.reconcile() else { taskCount.requestReconcile(); true }
        return dayReady && remindersReady && countReady
    }
    private suspend fun prepareLocalDay(): Boolean {
        val created = repository.refreshLocalDayIfNeeded { !restorePaused } ?: return false
        if (created > 0u) mutable.update { it.copy(revision = it.revision + 1) }
        return !restorePaused
    }
    fun refreshTaskCount() { scope.launch { guarded { reconcileReminders(waitForCount = false) } } }
    internal suspend fun reminder(intent: Intent) {
        try { if (reminders.receive(intent)) changed() }
        catch(e: ReminderLockedException) {
            // Authentication denial cannot replace an unresolved exchange/recovery failure.
            mutable.update { it.copy(information = R.string.reminder_unlock_required) }
        } catch(e: Exception) { if(e is CancellationException) throw e; recordError(e) }
    }
    internal fun reportFailure(error: Throwable) { recordError(error) }

    /** Retries the latest status receipt only; it never starts a network exchange. */
    fun retryStatusPersistence() { scope.launch { writeDesiredStatus() } }

    private suspend fun configureForeground() {
        val active = foregroundRequested && !restorePaused
        if (!active) {
            automaticJob?.cancel(); automaticJob = null
            debounceJob?.cancel()
            drainExchange()
            stopNearby()
            if (powerReceiverRegistered) { context.unregisterReceiver(powerReceiver); powerReceiverRegistered = false }
            return
        }
        if (!powerReceiverRegistered) {
            ContextCompat.registerReceiver(context, powerReceiver, IntentFilter(PowerManager.ACTION_POWER_SAVE_MODE_CHANGED), ContextCompat.RECEIVER_NOT_EXPORTED)
            powerReceiverRegistered = true
        }
        val settings = repository.settings()
        if (settings.provider == SyncProvider.LibreSync && !savingPower()) startNearby() else stopNearby()
        if (automaticJob?.isActive != true) automaticJob = scope.launch {
            automaticSync()
            while (isActive && foregroundRequested) { delay(150_000); automaticSync(); reconcileReminders() }
        }
    }

    private suspend fun automaticSync() = guarded {
        control.withLock {
            val settings = repository.settings()
            if (!foregroundRequested || restorePaused || savingPower() || !settings.automaticSync) return@withLock
            if (settings.provider == SyncProvider.Nextcloud && vault.connection() != null) startNextcloud(false, 120_000uL)
            // The Rust LibreSync scheduler owns periodic/exchange debounce while running.
        }
    }

    private suspend fun startNextcloud(test: Boolean, budgetMs: ULong, replace: Boolean = false): Exchange? {
        check(!restorePaused) { "Sync is paused while tasks are being restored" }
        if (exchange?.job?.isCompleted == false) return null
        exchange?.cancellation?.destroy(); exchange = null
        val connection = vault.connection() ?: error("Save a Nextcloud connection first")
        require(ConnectionPolicy.accepts(connection.serverUrl, isolated)) { "The saved connection must use HTTPS" }
        check(!ConnectionPolicy.isLocalServer(connection.serverUrl) || hasLocalNetworkAccess()) {
            "Allow local network access in Android settings to reach this Nextcloud server"
        }
        val engine = repository.read { this }
        val token = SyncCancellation()
        val job = scope.async {
            mutable.update { it.copy(syncing = true, information = null) }
            try {
                when {
                    test -> engine.testNextcloudConnectionCancellable(connection, token, budgetMs)
                    // This core entry point has the shared bounded foreground budget (300s).
                    replace -> engine.replaceRemoteSyncFile(connection, token)
                    else -> exchangeNextcloud(engine, connection, token, budgetMs)
                }
                // A read-only reachability probe cannot repair a damaged sync file
                // or prove the last exchange succeeded. Keep its recovery/error state.
                if (test) mutable.update { it.copy(information = R.string.connection_ok) }
                else clearError(null)
                mutable.update { it.copy(revision = it.revision + 1) }
                if (!test) { taskSearch?.requestReconcile(); reconcileReminders() }
                true
            } catch (e: Exception) {
                if (e is CancellationException) throw e
                // A requested cancellation is not a failed exchange or a success.
                if (!token.isCancelled()) recordError(e)
                false
            } finally {
                // Settings can commit even when the task exchange returns an error.
                runCatching { repository.refreshAppSettings() }.onFailure { recordError(it) }
                mutable.update { it.copy(syncing = false, revision = it.revision + 1) }
            }
        }
        return Exchange(token, job).also { exchange = it }
    }

    private suspend fun drainExchange() {
        val running = exchange ?: return
        if (!running.job.isCompleted) running.cancellation.cancel()
        withContext(NonCancellable) { running.job.join() }
        running.cancellation.destroy()
        if (exchange === running) exchange = null
    }

    private suspend fun startNearby() {
        if (repository.read { p2pRunning() }) return
        check(hasLocalNetworkAccess()) { "Allow local network access in Android settings to use LibreSync" }
        // Refuse a malformed vault before Rust considers creating replacement keys.
        vault.checkReadable()
        val epoch = ++generation
        val events = Channel<P2pEvent>(Channel.UNLIMITED)
        nearbyEvents = events
        nearbyEventJob = scope.launch {
            for (event in events) guarded { control.withLock {
                if (epoch != generation || !foregroundRequested || restorePaused) return@withLock
                when (event) {
                    is P2pEvent.SyncStarted -> mutable.update { it.copy(syncing = true, information = null) }
                    is P2pEvent.SyncCompleted -> {
                        repository.refreshAppSettings()
                        mutable.update { it.copy(syncing = false, revision = it.revision + 1) }
                        if (event.error != null) recordError(IllegalStateException(event.error)) else clearError(null)
                        reconcileReminders()
                    }
                    is P2pEvent.Notice -> when (val message = event.message) {
                        is Message.SaveFailed -> recordError(IllegalStateException(message.error))
                        is Message.LinkedWith -> clearError(R.string.linked)
                        Message.LinkFailed, Message.LinkDeclined -> recordError(IllegalStateException(context.getString(R.string.link_failed)))
                        else -> Unit
                    }
                    P2pEvent.StoreChanged -> { repository.refreshAppSettings(); mutable.update { it.copy(revision = it.revision + 1) }; taskSearch?.requestReconcile(); reconcileReminders() }
                    P2pEvent.StatusChanged -> { repository.refreshAppSettings(); mutable.update { it.copy(revision = it.revision + 1) } }
                    P2pEvent.DevicesChanged, P2pEvent.DiscoveryUpdated -> refreshNearby()
                    is P2pEvent.Started -> Unit
                }
            } }
        }
        val delegate = object : P2pDelegate {
            override fun onEvent(event: P2pEvent) { events.trySend(event) }
        }
        val secrets = VaultSecretStore(vault, ::recordError)
        val wifi = context.getSystemService(WifiManager::class.java)
        try {
            multicast = wifi?.createMulticastLock("Momentum nearby sync")?.apply { setReferenceCounted(false); acquire() }
            repository.read { this }.p2pStart(delegate, secrets, context.getString(R.string.app_name) + " · " + Build.MODEL, null)
            refreshNearby()
            if (pairingRequested) {
                val code = repository.read { p2pBeginPairing() }
                mutable.update { it.copy(pairingCode = code) }
            }
        } catch (e: Exception) {
            events.close(); nearbyEventJob?.cancel(); nearbyEvents = null; nearbyEventJob = null
            multicast?.let { if (it.isHeld) it.release() }; multicast = null; throw (secrets.failure ?: e)
        }
    }

    private suspend fun stopNearby() {
        generation++
        nearbyEvents?.close(); nearbyEventJob?.cancel(); nearbyEvents = null; nearbyEventJob = null
        try {
            val engine = repository.read { this }
            if (engine.p2pRunning()) { engine.p2pEndPairing(); engine.p2pStop() }
        }
        finally {
            multicast?.let { if (it.isHeld) it.release() }; multicast = null
            mutable.update { it.copy(syncing = exchange?.job?.isCompleted == false, pairingCode = null, discovered = emptyList()) }
        }
    }

    private suspend fun refreshNearby() {
        val result = repository.read { p2pDevices() to if (pairingRequested) p2pDiscovered() else emptyList() }
        mutable.update { it.copy(devices = result.first, discovered = result.second) }
    }

    private fun hasLocalNetworkAccess() = Build.VERSION.SDK_INT < 37 ||
        ContextCompat.checkSelfPermission(context, "android.permission.ACCESS_LOCAL_NETWORK") == PackageManager.PERMISSION_GRANTED
    private fun savingPower() = context.getSystemService(PowerManager::class.java)?.isPowerSaveMode == true

    private suspend fun configureBackground() {
        if (isolated) return
        val settings = repository.settings()
        val work = WorkManager.getInstance(context)
        // Local replenishment must not depend on reading optional sync credentials.
        if (!restorePaused && (settings.notifications || settings.taskCountMode != TaskCountMode.NONE)) {
            work.enqueueUniquePeriodicWork(REMINDER_WORK, ExistingPeriodicWorkPolicy.UPDATE,
                PeriodicWorkRequestBuilder<ReminderWorker>(if(settings.taskCountMode == TaskCountMode.NONE) 12L else 1L, TimeUnit.HOURS).build())
        } else work.cancelUniqueWork(REMINDER_WORK)
        if (settings.backgroundSyncEligible() && vault.connection() != null && !restorePaused) {
            val constraints = Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).setRequiresBatteryNotLow(true).build()
            work.enqueueUniquePeriodicWork(SYNC_WORK, ExistingPeriodicWorkPolicy.UPDATE,
                PeriodicWorkRequestBuilder<NextcloudWorker>(15, TimeUnit.MINUTES).setConstraints(constraints)
                    .setBackoffCriteria(BackoffPolicy.EXPONENTIAL, 30, TimeUnit.SECONDS).build())
        } else work.cancelUniqueWork(SYNC_WORK)
    }

    private suspend fun guarded(block: suspend () -> Unit) {
        try { block() } catch (e: Exception) { if (e is CancellationException) throw e; recordError(e) }
    }
    @Synchronized private fun recordError(error: Throwable) {
        errorUpdated = true
        val failure = PlatformFailure.from(error)
        val detail = failure.details.take(16_384)
        mutable.update { it.copy(error = detail, errorDetails = detail, failureKind = failure.kind, information = null) }
        persistError(detail, failure.kind)
    }
    @Synchronized private fun clearError(information: Int?) {
        errorUpdated = true
        mutable.update { it.copy(error = null, errorDetails = null, failureKind = null, information = information) }
        persistError(null, null)
    }
    @Synchronized private fun persistError(error: String?, kind: SyncFailureKind?) {
        desiredStatus = PersistedStatus(error, kind)
        writeDesiredStatus()
    }
    @Synchronized private fun writeDesiredStatus() {
        val desired = desiredStatus ?: return
        runCatching {
            check(repository.directory.exists() || repository.directory.mkdirs())
            val stream = stateFile.startWrite()
            try {
                stream.write(JSONObject().put("error", desired.error ?: JSONObject.NULL).put("failure-kind", desired.kind?.name ?: JSONObject.NULL).toString().toByteArray())
                stateFile.finishWrite(stream)
            }
            catch (e: Exception) { stateFile.failWrite(stream); throw e }
        }.onSuccess {
            mutable.update { it.copy(statusPersistenceError = null) }
        }.onFailure { failure ->
            // Keep exchange/recovery state intact. Do not recursively persist a
            // write failure into the same broken file or lose the desired receipt.
            mutable.update { it.copy(statusPersistenceError = (failure.message ?: failure.javaClass.simpleName).take(16_384)) }
        }
    }

    private companion object {
        const val SYNC_WORK = "momentum.nextcloud"
        const val REMINDER_WORK = "momentum.reminder-plan"
    }
}
