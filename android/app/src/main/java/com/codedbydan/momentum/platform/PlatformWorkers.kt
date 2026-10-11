// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.platform

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import androidx.work.*
import com.codedbydan.momentum.MomentumApplication
import kotlinx.coroutines.*

class NextcloudWorker(context: Context, parameters: WorkerParameters) : CoroutineWorker(context, parameters) {
    override suspend fun doWork(): Result = try {
        if ((applicationContext as MomentumApplication).platform.backgroundSync()) Result.success() else Result.retry()
    } catch (e: Exception) {
        if (e is CancellationException) throw e
        (applicationContext as MomentumApplication).platform.reportFailure(e)
        Result.retry()
    }
}

class ReminderWorker(context: Context, parameters: WorkerParameters) : CoroutineWorker(context, parameters) {
    override suspend fun doWork(): Result {
        return if ((applicationContext as MomentumApplication).platform.reconcileReminders(inputData.getBoolean("reset", false)))
            Result.success() else Result.retry()
    }
}

/** Cold callbacks always use the application's existing engine owner. */
class ReminderReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val pending = goAsync()
        CoroutineScope(SupervisorJob() + Dispatchers.IO).launch {
            var platform: PlatformCoordinator? = null
            try {
                val owner = (context.applicationContext as MomentumApplication).platform
                platform = owner
                // Rust/disk operations are short; longer plan replenishment uses WorkManager.
                withTimeout(8_000) { owner.reminder(intent) }
            } catch (e: Exception) {
                if (e !is CancellationException || e is TimeoutCancellationException) platform?.reportFailure(e)
            } finally {
                try { enqueueReminderRefresh(context) }
                catch (e: Exception) { platform?.reportFailure(e) }
                finally { pending.finish() }
            }
        }
    }
}

class BootReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        // Alarms disappear on boot; changed clocks/exact access also need a new schedule.
        if (intent.action !in setOf(Intent.ACTION_BOOT_COMPLETED, Intent.ACTION_TIME_CHANGED,
                Intent.ACTION_TIMEZONE_CHANGED, Intent.ACTION_DATE_CHANGED, android.app.AlarmManager.ACTION_SCHEDULE_EXACT_ALARM_PERMISSION_STATE_CHANGED)) return
        enqueueReminderRefresh(context, reset = true)
    }
}

internal fun enqueueReminderRefresh(context: Context, reset: Boolean = false) {
    WorkManager.getInstance(context).enqueueUniqueWork("momentum.reminder-refresh",
        ExistingWorkPolicy.APPEND_OR_REPLACE,
        OneTimeWorkRequestBuilder<ReminderWorker>().setInputData(workDataOf("reset" to reset)).build())
}
