// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.data

import android.content.Context
import androidx.datastore.preferences.core.*
import androidx.datastore.preferences.preferencesDataStoreFile
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.core.Preferences
import com.codedbydan.momentum.platform.CompletionSoundChoice
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import java.io.File

enum class SyncProvider { Off, Nextcloud, LibreSync }
enum class Appearance { System, Light, Dark }
enum class NativeFont { System, Sans, Serif, Monospace }
enum class NativeAccent { Momentum, Alizarin, Carrot, Orange, Emerald, Turquoise, PeterRiver, Amethyst, Pomegranate, GreenSea }
enum class KeyboardModifier { Ctrl, Alt, Meta }

fun normalizedNativeTextScale(value: Float): Float = if(value.isFinite()) value.coerceIn(0.75f, 2f) else 1f

data class AppSettings(
    val appearance: Appearance = Appearance.System,
    val dynamicColor: Boolean = false,
    val compact: Boolean = false,
    val colorfulLabels: Boolean = true,
    val notifications: Boolean = false,
    val completionSound: Boolean = false,
    val completionSoundChoice: CompletionSoundChoice = CompletionSoundChoice.defaultChoice,
    val provider: SyncProvider = SyncProvider.Off,
    val automaticSync: Boolean = true,
    val backgroundSync: Boolean = false,
    val systemSearch: Boolean = false,
    val font: NativeFont = NativeFont.System,
    val contentTextScale: Float = 1f,
    val interfaceTextScale: Float = 1f,
    val accent: NativeAccent = NativeAccent.Momentum,
    val keyboardModifier: KeyboardModifier = KeyboardModifier.Ctrl,
    val taskCountMode: TaskCountMode = TaskCountMode.NONE,
)

data class Snapshot(
    val listing: Listing,
    val sidebar: Sidebar,
    val projects: List<ProjectRef>,
    val tags: List<TagRef>,
    val preferences: Preferences,
    val settings: AppSettings,
    val sync: SyncStatus,
    val canUndo: Boolean,
    val parentTitles: Map<String, String> = emptyMap(),
    val settingsSync: SettingsSyncStatus? = null,
    val appSettingsError: String? = null,
)

/** One engine per application. Disk/FFI work never runs on the Android main thread. */
class MomentumRepository internal constructor(
    private val context: Context,
    val directory: File,
    private val beforePortableStage: Engine.() -> Unit = {},
    private val observeDayChange: Engine.() -> Boolean,
) {
    constructor(context: Context, directory: File = File(context.noBackupFilesDir, "tasks")) :
        this(context, directory, observeDayChange = { dayChanged() })
    private val lock = Mutex()
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val store = PreferenceDataStoreFactory.create(scope = scope) {
        File(directory, "android.preferences_pb")
    }
    private var instance: Engine? = null
    private var closed = false
    private var portableRevision: ULong? = null
    private var portableError: String? = null
    private var needsNativeBootstrap = true
    private var lastNativePreferences = emptyPreferences()
    private suspend fun engine(): Engine {
        check(!closed)
        return instance ?: Engine.openChecked(directory.absolutePath).also {
            try {
                // Restore durable core choices before staging this device's full allowlist.
                store.edit { native -> PortableAppSettings.apply(it.portableSettingsJson(), native) }
                val prefs = store.data.first().also { lastNativePreferences = it }
                val local = corePreferences(prefs)
                it.applyDevicePreferences(local)
                it.stagePortableSettings(PortableAppSettings.encode(appSettings(prefs), corePreferences(prefs)))
                portableRevision = it.settingsSyncStatus().revision
                needsNativeBootstrap = false
            } catch(error: Exception) {
                if(error is CancellationException) { it.destroy(); throw error }
                // Preference recovery must not prevent reading offline tasks.
                portableError = error.message ?: error.javaClass.simpleName
            }
            it.spawnRepeats()
            instance = it
        }
    }

    suspend fun <T> read(block: suspend Engine.() -> T): T = withContext(Dispatchers.IO) {
        lock.withLock { engine().block() }
    }
    /** Admission is checked after queueing; day consumption and repeat creation stay together. */
    suspend fun refreshLocalDayIfNeeded(admitted: () -> Boolean = { true }): UInt? = withContext(Dispatchers.IO) {
        lock.withLock { if (admitted()) engine().refreshLocalDayLocked() else null }
    }
    suspend fun <T> readProjection(admitted: () -> Boolean = { true }, block: suspend Engine.() -> T): T? = withContext(Dispatchers.IO) {
        lock.withLock {
            if (!admitted()) return@withLock null
            val engine = engine()
            engine.refreshLocalDayLocked()
            if (admitted()) engine.block() else null
        }
    }
    private fun Engine.refreshLocalDayLocked(): UInt = if (observeDayChange(this)) spawnRepeats() else 0u
    suspend fun mutate(block: Engine.() -> Outcome): Outcome = read { block() }
    suspend fun create(draft: TaskDraft, view: View): TaskCreation = read { createTaskWithId(draft, view) }
    suspend fun detail(id: String): TaskDetail? = read { taskDetail(id) }
    suspend fun snapshot(view: View, query: String = "", archiveLimit: UInt = 100u): Snapshot = readProjection {
        refreshPortableLocked(this)
        val resolved = if (viewExists(view)) view else View.Today
        val list = if (resolved == View.Search) search(query) else listing(resolved, archiveLimit)
        val parents = list.sections.flatMap { it.rows }.filterIsInstance<Row.Task>().mapNotNull { it.row.parentId }.distinct()
            .mapNotNull { id -> taskRow(id)?.title?.let { id to it } }.toMap()
        Snapshot(list, sidebar(), projects(), tags(), preferences(), appSettings(nativePreferencesForDisplay()), syncStatus(), canUndo(), parents, settingsSyncStatus(), portableError)
    }!! // Snapshots have unconditional admission and always produce a value.
    suspend fun settings(): AppSettings = read { refreshPortableLocked(this); appSettings(nativePreferencesForDisplay()) }
    private suspend fun nativePreferencesForDisplay(): androidx.datastore.preferences.core.Preferences = try {
        store.data.first().also { lastNativePreferences = it }
    } catch(error: Exception) {
        if(error is CancellationException) throw error
        portableError = error.message ?: error.javaClass.simpleName
        lastNativePreferences
    }
    private suspend fun refreshPortableLocked(engine: Engine) {
        val revision = engine.settingsSyncStatus().revision
        if(portableRevision != revision || needsNativeBootstrap) {
            try {
                store.edit { PortableAppSettings.apply(engine.portableSettingsJson(), it) }
                if(needsNativeBootstrap) {
                    val native = store.data.first()
                    engine.applyDevicePreferences(corePreferences(native))
                    engine.stagePortableSettings(PortableAppSettings.encode(appSettings(native), corePreferences(native)))
                    portableRevision = engine.settingsSyncStatus().revision
                    needsNativeBootstrap = false
                } else portableRevision = revision
                portableError = null
            } catch(error: Exception) {
                if(error is CancellationException) throw error
                portableError = error.message ?: error.javaClass.simpleName
            }
        }
    }
    suspend fun refreshAppSettings() = read { refreshPortableLocked(this) }
    suspend fun setSettingsSync(enabled: Boolean) = read {
        if(enabled) { refreshPortableLocked(this); check(portableError == null) { "Apply the saved app settings before enabling sync." } }
        setSettingsSyncEnabled(enabled)
    }
    suspend fun chooseSettings(choice: SettingsBootstrapChoice, reviewId: String) = read {
        setSettingsBootstrapChoice(choice, reviewId)
    }
    suspend fun importAppSettings(file: File): Outcome = read {
        val outcome = importSettings(file.absolutePath)
        refreshPortableLocked(this)
        check(portableError == null) { "Imported app settings were saved but could not be applied. Retry from app settings." }
        outcome
    }
    suspend fun exportAppSettings(file: File) = read {
        refreshPortableLocked(this); check(portableError == null) { "Apply the saved app settings before exporting them." }
        exportSettings(file.absolutePath)
    }
    suspend fun exportBackup(file: File, includeSettings: Boolean = false) = read {
        if(includeSettings) { refreshPortableLocked(this); check(portableError == null) { "Apply the saved app settings before exporting them." } }
        exportBackupWithSettings(file.absolutePath, includeSettings)
    }
    suspend fun restoreBackup(file: File, restoreSettings: Boolean = false): Outcome = read {
        val outcome = importBackupWithSettings(file.absolutePath, restoreSettings)
        refreshPortableLocked(this)
        outcome
    }
    suspend fun updateSettings(settings: AppSettings) = updateSettings { settings }
    suspend fun updateSettings(change: (AppSettings) -> AppSettings) = read {
        refreshPortableLocked(this)
        val current = store.data.first()
        val previous = PortableAppSettings.encode(appSettings(current), corePreferences(current))
        val changed = change(appSettings(current))
        val next = PortableAppSettings.encode(changed, corePreferences(current))
        val delta = PortableAppSettings.changedDocument(previous, next)
        val portableChanged = PortableAppSettings.values(delta).length() > 0
        if(portableChanged) {
            check(portableError == null) { "App settings could not be applied. Retry before changing them." }
            beforePortableStage()
            stagePortableSettings(delta)
        }
        try { store.edit { writeSettings(it, changed) } }
        catch(error: Exception) {
            if(portableChanged) stagePortableSettings(PortableAppSettings.changedDocument(next, previous))
            throw error
        }
        // Device-local opt-outs remain usable while optional portable recovery fails.
        // An exchange may have committed another key while native I/O was pending.
        if(portableChanged) portableRevision = null
        refreshPortableLocked(this)
    }
    suspend fun updatePreferences(prefs: Preferences) = updatePreferences { prefs }
    suspend fun updatePreferences(change: (Preferences) -> Preferences) = read {
        refreshPortableLocked(this)
        val current = store.data.first()
        val originalPreferences = corePreferences(current)
        val prefs = change(originalPreferences)
        val previous = PortableAppSettings.encode(appSettings(current), originalPreferences)
        val next = PortableAppSettings.encode(appSettings(current), prefs)
        val delta = PortableAppSettings.changedDocument(previous, next)
        val portableChanged = PortableAppSettings.values(delta).length() > 0
        if(portableChanged) {
            check(portableError == null) { "App settings could not be applied. Retry before changing them." }
            beforePortableStage()
            stagePortableSettings(delta)
        }
        try { store.edit { writeCorePreferences(it, prefs) } }
        catch(error: Exception) {
            if(portableChanged) stagePortableSettings(PortableAppSettings.changedDocument(next, previous))
            throw error
        }
        // Rust already applied the portable rules. Narrow setters retain any
        // concurrent portable commit while applying durable device-only choices.
        applyDevicePreferences(prefs)
        if(portableChanged) portableRevision = null
        refreshPortableLocked(this)
    }
    private fun Engine.applyDevicePreferences(prefs: Preferences) {
        setDeviceSummaryPreferences(prefs.morningSummaryEnabled, prefs.morningSummaryTime)
        setDayPeriodTasksInToday(prefs.showDayPeriodTasksInToday)
    }
    suspend fun draft(input: EditorDraft): TaskDraft? = read {
        if (input.problem() != null) return@read null
        val time = input.time.takeIf { it.isNotBlank() }?.let { parseTime(it) ?: return@read null }
        val estimate = if (input.estimate.isBlank()) 0.0 else parseEstimate(input.estimate) ?: return@read null
        TaskDraft(input.title.trim(), input.projectId, input.dueDay.takeIf { it.isNotBlank() }, time,
            input.reminder?.takeIf { time != null }?.toUInt(), estimate, input.notes, input.tagIds,
            input.newTags.split(',').map { it.trim() }.filter { it.isNotEmpty() })
    }

    // Network I/O deliberately runs outside the serial edit queue so offline edits stay usable.
    suspend fun sync(settings: NextcloudSettings, cancellation: SyncCancellation): SyncReport {
        val engine = read { this }
        return withContext(Dispatchers.IO) { engine.syncNextcloudCancellable(settings, cancellation) }
    }
    suspend fun testConnection(settings: NextcloudSettings, cancellation: SyncCancellation) {
        val engine = read { this }
        withContext(Dispatchers.IO) { engine.testNextcloudConnectionCancellable(settings, cancellation, 30_000uL) }
    }
    suspend fun close() = withContext(Dispatchers.IO) {
        lock.withLock {
            if (!closed) {
                instance?.p2pStop()
                instance?.destroy()
                instance = null
                closed = true
                scope.coroutineContext[Job]?.cancelAndJoin()
            }
        }
    }
}

private fun appSettings(prefs: androidx.datastore.preferences.core.Preferences): AppSettings = AppSettings(
    appearance = enumValueOr(prefs[stringPreferencesKey("appearance")], Appearance.System),
    dynamicColor = prefs[booleanPreferencesKey("dynamic-color")] ?: false,
    compact = prefs[booleanPreferencesKey("compact")] ?: false,
    colorfulLabels = prefs[booleanPreferencesKey("colorful-labels")] ?: true,
    notifications = prefs[booleanPreferencesKey("notifications")] ?: false,
    completionSound = prefs[booleanPreferencesKey("completion-sound")] ?: false,
    completionSoundChoice = CompletionSoundChoice.resolve(prefs[stringPreferencesKey("completion-sound-choice")]),
    provider = enumValueOr(prefs[stringPreferencesKey("provider")], SyncProvider.Off),
    automaticSync = prefs[booleanPreferencesKey("automatic-sync")] ?: true,
    backgroundSync = prefs[booleanPreferencesKey("background-sync")] ?: false,
    systemSearch = prefs[booleanPreferencesKey("system-search")] ?: false,
    font = enumValueOr(prefs[stringPreferencesKey("font")], NativeFont.System),
    contentTextScale = normalizedNativeTextScale(prefs[floatPreferencesKey("content-text-scale")] ?: 1f),
    interfaceTextScale = normalizedNativeTextScale(prefs[floatPreferencesKey("interface-text-scale")] ?: 1f),
    accent = enumValueOr(prefs[stringPreferencesKey("accent")], NativeAccent.Momentum),
    keyboardModifier = enumValueOr(prefs[stringPreferencesKey("keyboard-modifier")], KeyboardModifier.Ctrl),
    taskCountMode = enumValueOr(prefs[stringPreferencesKey("task-count-mode")], TaskCountMode.NONE),
)
private inline fun <reified T : Enum<T>> enumValueOr(value: String?, fallback: T): T =
    enumValues<T>().firstOrNull { it.name == value } ?: fallback

private fun corePreferences(prefs: androidx.datastore.preferences.core.Preferences): com.codedbydan.momentum.core.Preferences =
    com.codedbydan.momentum.core.Preferences(
        enumValueOr(prefs[stringPreferencesKey("group")], GroupBy.MORNING_NIGHT),
        prefs[booleanPreferencesKey("show-day-period-tasks-in-today")] ?: true,
        enumValueOr(prefs[stringPreferencesKey("sort")], SortKey.MANUAL),
        enumValueOr(prefs[stringPreferencesKey("direction")], SortDirection.ASCENDING),
        (prefs[intPreferencesKey("upcoming-days")] ?: 7).let { if (it == 30) 30u else 7u },
        prefs[booleanPreferencesKey("auto-archive")] ?: false,
        prefs[booleanPreferencesKey("morning-summary")] ?: false,
        ClockTime((prefs[intPreferencesKey("summary-hour")] ?: 8).coerceIn(0, 23).toUInt(),
            (prefs[intPreferencesKey("summary-minute")] ?: 0).coerceIn(0, 59).toUInt()),
    )

private fun writeSettings(it: MutablePreferences, settings: AppSettings) {
    it[stringPreferencesKey("appearance")] = settings.appearance.name
    it[booleanPreferencesKey("dynamic-color")] = settings.dynamicColor
    it[booleanPreferencesKey("compact")] = settings.compact
    it[booleanPreferencesKey("colorful-labels")] = settings.colorfulLabels
    it[booleanPreferencesKey("notifications")] = settings.notifications
    it[booleanPreferencesKey("completion-sound")] = settings.completionSound
    val soundKey = stringPreferencesKey("completion-sound-choice")
    if(CompletionSoundChoice.resolve(it[soundKey]) != settings.completionSoundChoice) it[soundKey] = settings.completionSoundChoice.key
    it[stringPreferencesKey("provider")] = settings.provider.name
    it[booleanPreferencesKey("automatic-sync")] = settings.automaticSync
    it[booleanPreferencesKey("background-sync")] = settings.backgroundSync
    it[booleanPreferencesKey("system-search")] = settings.systemSearch
    it[stringPreferencesKey("font")] = settings.font.name
    it[floatPreferencesKey("content-text-scale")] = normalizedNativeTextScale(settings.contentTextScale)
    it[floatPreferencesKey("interface-text-scale")] = normalizedNativeTextScale(settings.interfaceTextScale)
    it[stringPreferencesKey("accent")] = settings.accent.name
    it[stringPreferencesKey("keyboard-modifier")] = settings.keyboardModifier.name
    it[stringPreferencesKey("task-count-mode")] = settings.taskCountMode.name
}
private fun writeCorePreferences(it: MutablePreferences, prefs: Preferences) {
    it[stringPreferencesKey("group")] = prefs.groupBy.name
    it[booleanPreferencesKey("show-day-period-tasks-in-today")] = prefs.showDayPeriodTasksInToday
    it[stringPreferencesKey("sort")] = prefs.sort.name
    it[stringPreferencesKey("direction")] = prefs.direction.name
    it[intPreferencesKey("upcoming-days")] = prefs.upcomingDays.toInt()
    it[booleanPreferencesKey("auto-archive")] = prefs.autoArchive
    it[booleanPreferencesKey("morning-summary")] = prefs.morningSummaryEnabled
    it[intPreferencesKey("summary-hour")] = prefs.morningSummaryTime.hour.toInt()
    it[intPreferencesKey("summary-minute")] = prefs.morningSummaryTime.minute.toInt()
}
