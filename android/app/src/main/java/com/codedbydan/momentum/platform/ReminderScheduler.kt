// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.platform

import android.Manifest
import android.app.AlarmManager
import android.app.KeyguardManager
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import com.codedbydan.momentum.MainActivity
import com.codedbydan.momentum.R
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.MomentumRepository
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import java.io.File
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter

/** Stable OS identity includes the precise source, without putting task text in a URI. */
internal object ReminderIdentity {
    fun key(request: NotificationObservation): String = digest("${request.id.length}:${request.id}${request.sourceRevision}")
    fun observation(request: NotificationRequest) = NotificationObservation(request.id, request.sourceRevision)
}

internal object ReminderActions {
    fun performIfCurrent(engine: Engine, request: NotificationRequest, nowMs: ULong, action: String): Outcome? {
        val command = when (action) {
            "done" -> NotificationAction.Done
            "snooze" -> NotificationAction.Snooze(60u)
            else -> return null
        }
        // The shared owner lock covers source validation, the task rule and its
        // durable commit, including concurrent native sync/restore operations.
        return engine.performNotificationAction(request, command, nowMs).takeIf {
            it.changed || it.message is Message.SaveFailed
        }
    }
}

/** Expected admission rejection, distinct from a failed sync or storage operation. */
internal class ReminderLockedException(message: String) : IllegalStateException(message)

/** Android cannot enumerate alarms. Atomic observations repair/reschedule future alarms. */
internal class ReminderScheduler(
    private val context: Context,
    private val repository: MomentumRepository,
    private val isolated: Boolean,
    private val deviceLocked: () -> Boolean = { context.getSystemService(KeyguardManager::class.java).isDeviceLocked },
) {
    private val lock = Mutex()
    private val observations = ReminderObservations(File(repository.directory, "android-reminder-schedule.json"))
    private val alarm = context.getSystemService(AlarmManager::class.java)
    private val notifications = context.getSystemService(NotificationManager::class.java)
    @Volatile private var paused = false

    suspend fun reconcile(reset: Boolean = false) = withContext(Dispatchers.IO) {
        lock.withLock {
            // Test/preview stores never alter the user's OS notification collection.
            if (isolated || paused) return@withLock
            if (!allowed()) {
                // Opting out must hide displayed reminders even when saved alarm
                // observations are corrupt. Leave unreadable bytes intact; unknown
                // alarm callbacks cannot post because receive() also checks opt-in.
                notifications.activeNotifications.filter { it.notification.channelId == CHANNEL }
                    .forEach { notifications.cancel(it.tag, it.id) }
                val disabled = observations.load()
                disabled.values.forEach { cancel(it.request) }
                disabled.clear(); observations.save(disabled)
                repository.read { reconcileNotifications(now(), emptyList(), emptyList()) }
                return@withLock
            }
            val records = observations.load()
            if (reset) {
                records.values.filter { !it.delivered }.toList().forEach { cancelAlarm(it.request) }
                records.entries.removeAll { !it.value.delivered }
                observations.save(records)
            }
            ensureChannel()
            val delivered = notifications.activeNotifications.filter { it.notification.channelId == CHANNEL }.mapNotNull {
                val extras = it.notification.extras
                val id = extras.getString(ID) ?: return@mapNotNull null
                val revision = extras.getString(REVISION) ?: return@mapNotNull null
                NotificationObservation(id, revision)
            }
            // Dismissed receipts can be dropped: the core durably suppresses their
            // elapsed identities, and there is no longer an active action to handle.
            val active = delivered.toSet()
            records.entries.removeAll { it.value.delivered && ReminderIdentity.observation(it.value.request) !in active }
            val plan = repository.readProjection({ !paused }) {
                reconcileNotifications(now(), records.values.filter { !it.delivered }.map { ReminderIdentity.observation(it.request) }, delivered)
            } ?: return@withLock
            for (identity in plan.cancellations) {
                val key = ReminderIdentity.key(identity)
                records.remove(key)?.let { cancel(it.request) }
                notifications.cancel(key, 0)
            }
            // Bound private at-most-once receipts; the core's durable consumption survives.
            val cutoff = System.currentTimeMillis() - TimeUnitDays7
            records.entries.filter { it.value.request.fireAtMs.toLong() < cutoff }.forEach { (_, record) -> cancel(record.request) }
            records.entries.removeAll { it.value.request.fireAtMs.toLong() < cutoff }
            observations.save(records)
            for (request in plan.requests) {
                if (!allowed()) break
                val key = ReminderIdentity.key(ReminderIdentity.observation(request))
                if (records[key]?.delivered == true) continue
                // Always reinstall future desired alarms. Android gives no pending-alarm snapshot,
                // including after special-access revocation or a process/OS restart.
                schedule(request)
                try {
                    records[key] = ReminderRecord(request)
                    observations.save(records)
                    val accepted = repository.read { acceptNotification(request, now()) }
                    if (!accepted.accepted) {
                        cancel(request); records.remove(key); observations.save(records)
                    }
                } catch (e: Exception) {
                    cancel(request); records.remove(key)
                    runCatching { observations.save(records) }
                    throw e
                }
            }
        }
    }

    suspend fun cancelAndDrain() = withContext(Dispatchers.IO) {
        lock.withLock {
            if (isolated) return@withLock
            val records = observations.load()
            records.values.forEach { cancel(it.request) }
            records.clear(); observations.save(records)
        }
    }

    fun pause() { paused = true }
    fun resume() { paused = false }

    /** False includes stale, dismissed, duplicated or disabled callbacks. */
    suspend fun receive(intent: Intent): Boolean = withContext(Dispatchers.IO) {
        lock.withLock {
            if (isolated || paused || !allowed()) return@withLock false
            if (intent.hasExtra(ACTION) && deviceLocked()) {
                throw ReminderLockedException(context.getString(R.string.reminder_unlock_required))
            }
            val key = intent.getStringExtra(KEY) ?: return@withLock false
            val records = observations.load()
            val record = records[key] ?: return@withLock false
            if (key != ReminderIdentity.key(ReminderIdentity.observation(record.request))) return@withLock false
            val action = intent.getStringExtra(ACTION)
            if (action == null && record.delivered) return@withLock false
            if (action != null && (!record.delivered || action in record.actions || action !in setOf(DONE, SNOOZE))) return@withLock false
            if (action != null) {
                // One shared-core transaction validates and mutates the exact source.
                val outcome = repository.read {
                    if(deviceLocked()) throw ReminderLockedException(context.getString(R.string.reminder_unlock_required))
                    ReminderActions.performIfCurrent(this, record.request, now(), action)
                }
                if (outcome == null) {
                    cancel(record.request); records.remove(key); observations.save(records)
                    return@withLock false
                }
                (outcome.message as? Message.SaveFailed)?.let { error(it.error) }
                if (outcome.changed) {
                    records[key] = record.copy(actions = record.actions + action)
                    observations.save(records)
                    cancel(record.request)
                }
                return@withLock outcome.changed
            }
            // A scheduled alarm may outlive an edit or restore. Check its exact
            // source again immediately before creating the delivery receipt.
            if (!repository.read { acceptNotification(record.request, now()).accepted }) {
                cancel(record.request); records.remove(key); observations.save(records)
                return@withLock false
            }
            // Persist an at-most-once receipt before posting. This is not delivery proof.
            records[key] = record.copy(delivered = true)
            observations.save(records)
            cancelAlarm(record.request)
            ensureChannel()
            if (!allowed()) { notifications.cancel(key, 0); return@withLock false }
            val builder = NotificationCompat.Builder(context, CHANNEL)
                .setSmallIcon(R.drawable.ic_launcher_foreground)
                .setVisibility(NotificationCompat.VISIBILITY_PRIVATE)
                .setPublicVersion(NotificationCompat.Builder(context, CHANNEL)
                    .setSmallIcon(R.drawable.ic_launcher_foreground).setContentTitle(context.getString(R.string.app_name)).build())
                .setAutoCancel(true).setOnlyAlertOnce(true).setCategory(NotificationCompat.CATEGORY_REMINDER)
                .addExtras(android.os.Bundle().apply { putString(ID, record.request.id); putString(REVISION, record.request.sourceRevision) })
            when (val content = record.request.content) {
                is NotificationContent.Reminder -> {
                    val body = content.dueAtMs?.let { due ->
                        val formatter = DateTimeFormatter.ofPattern(if (android.text.format.DateFormat.is24HourFormat(context)) "HH:mm" else "h:mm a")
                        context.getString(R.string.due_time, Instant.ofEpochMilli(due.toLong()).atZone(ZoneId.systemDefault()).format(formatter))
                    } ?: context.getString(R.string.reminder)
                    val open = openIntent(key, content.taskId)
                    builder.setContentTitle(content.title).setContentText(if(Build.VERSION.SDK_INT < 31)
                        context.getString(R.string.reminder_body_unlock, body) else body).setContentIntent(open)
                        .addAction(0, context.getString(R.string.open_task), open)
                        .addAction(NotificationCompat.Action.Builder(0, context.getString(R.string.done), actionIntent(key, DONE))
                            .setAuthenticationRequired(true).build())
                        .addAction(NotificationCompat.Action.Builder(0, context.getString(R.string.snooze), actionIntent(key, SNOOZE))
                            .setAuthenticationRequired(true).build())
                }
                is NotificationContent.Summary -> builder.setContentTitle(context.getString(R.string.summary))
                    .setContentText(context.resources.getQuantityString(R.plurals.summary_count,
                        content.counts.total.toInt(), content.counts.total.toInt()))
                    .setContentIntent(openIntent(key, null))
            }
            notifications.notify(key, 0, builder.build())
            true
        }
    }

    private suspend fun allowed(): Boolean = repository.settings().notifications && notifications.areNotificationsEnabled() &&
        (Build.VERSION.SDK_INT < 33 || ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED) &&
        notifications.getNotificationChannel(CHANNEL)?.importance != NotificationManager.IMPORTANCE_NONE

    private fun ensureChannel() {
        notifications.createNotificationChannel(NotificationChannel(CHANNEL, context.getString(R.string.reminder), NotificationManager.IMPORTANCE_DEFAULT).apply {
            lockscreenVisibility = android.app.Notification.VISIBILITY_PRIVATE
            setShowBadge(true)
        })
    }

    private fun schedule(request: NotificationRequest) {
        val time = maxOf(request.fireAtMs.toLong(), System.currentTimeMillis() + 1_000)
        val intent = alarmIntent(request, PendingIntent.FLAG_UPDATE_CURRENT)!!
        if (Build.VERSION.SDK_INT < 31 || alarm.canScheduleExactAlarms()) {
            try { alarm.setExactAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, time, intent) }
            catch (_: SecurityException) { alarm.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, time, intent) }
        } else alarm.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, time, intent)
    }

    private fun cancelAlarm(request: NotificationRequest) {
        alarmIntent(request, PendingIntent.FLAG_NO_CREATE)?.let { alarm.cancel(it); it.cancel() }
    }
    private fun cancel(request: NotificationRequest) {
        cancelAlarm(request)
        notifications.cancel(ReminderIdentity.key(ReminderIdentity.observation(request)), 0)
    }
    private fun alarmIntent(request: NotificationRequest, flags: Int): PendingIntent? {
        val key = ReminderIdentity.key(ReminderIdentity.observation(request))
        return PendingIntent.getBroadcast(context, 0, Intent(context, ReminderReceiver::class.java)
            .setAction("com.codedbydan.momentum.REMINDER").setData(Uri.parse("momentum-private://reminder/$key"))
            .putExtra(KEY, key), flags or PendingIntent.FLAG_IMMUTABLE)
    }
    private fun actionIntent(key: String, action: String): PendingIntent = PendingIntent.getBroadcast(context, 0,
        Intent(context, ReminderReceiver::class.java).setAction("com.codedbydan.momentum.REMINDER_ACTION")
            .setData(Uri.parse("momentum-private://reminder/$key/$action")).putExtra(KEY, key).putExtra(ACTION, action),
        PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
    private fun openIntent(key: String, task: String?): PendingIntent = PendingIntent.getActivity(context, 0,
        Intent(context, MainActivity::class.java).setAction(Intent.ACTION_MAIN)
            .setFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP)
            .setData(Uri.parse("momentum-private://open/$key")).apply {
                if (task != null) putExtra("task-id", task) else putExtra("open-today", true)
            },
        PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)

    private fun now() = System.currentTimeMillis().coerceAtLeast(0).toULong()

    private companion object {
        const val CHANNEL = "momentum.reminders"
        const val ID = "momentum.notification.id"
        const val REVISION = "momentum.notification.revision"
        const val KEY = "momentum.notification.key"
        const val ACTION = "momentum.notification.action"
        const val DONE = "done"
        const val SNOOZE = "snooze"
        const val TimeUnitDays7 = 7L * 24 * 60 * 60 * 1_000
    }
}
