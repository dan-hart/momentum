// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.platform

import android.Manifest
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
import com.codedbydan.momentum.core.TaskCountMode
import com.codedbydan.momentum.data.MomentumRepository
import kotlinx.coroutines.*
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import java.time.LocalDate

internal data class TaskCountStatus(val count: Int = 0, val blocked: Boolean = false, val error: String? = null)

/** Optional count projection. Task membership and family rules belong to Rust. */
internal class TaskCountNotifier(private val context: Context, private val repository: MomentumRepository,
    private val isolated: Boolean,
    private val channelId: String = CHANNEL,
    private val tag: String = TAG,
) {
    private val manager = context.getSystemService(NotificationManager::class.java)
    private val lock = Mutex()
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val requests = Channel<Unit>(Channel.CONFLATED)
    private val mutable = MutableStateFlow(TaskCountStatus())
    val state: StateFlow<TaskCountStatus> = mutable.asStateFlow()
    @Volatile private var paused = false
    private data class Projection(val mode: TaskCountMode, val count: Int, val day: LocalDate, val locale: String)
    private var lastPosted: Projection? = null

    init { scope.launch { for(request in requests) reconcile() } }
    fun requestReconcile() { requests.trySend(Unit) }
    fun pause() { paused = true }
    fun resume() { paused = false; requestReconcile() }
    suspend fun cancelAndDrain() = withContext(Dispatchers.IO) { lock.withLock { cancelPostedNotification() } }

    suspend fun reconcile(): Boolean = withContext(Dispatchers.IO) {
        lock.withLock {
            try {
                if(isolated || paused) return@withLock true
                val mode = repository.settings().taskCountMode
                if(mode == TaskCountMode.NONE) { cancelPostedNotification(); mutable.value = TaskCountStatus(); return@withLock true }
                ensureChannel()
                if(!allowed()) { cancelPostedNotification(); mutable.value = TaskCountStatus(blocked = true); return@withLock true }
                val count = (repository.readProjection({ !paused }) { taskCount(mode) }
                    ?: return@withLock true).coerceAtMost(Int.MAX_VALUE.toUInt()).toInt()
                // Opt-out/restore may arrive while the core projection is queued.
                if(paused || repository.settings().taskCountMode != mode || !allowed()) {
                    cancelPostedNotification(); requestReconcile(); return@withLock true
                }
                if(count == 0) { cancelPostedNotification(); mutable.value = TaskCountStatus(); return@withLock true }
                val projection = Projection(mode, count, LocalDate.now(), context.resources.configuration.locales[0].toLanguageTag())
                // Respect dismissal until the count, policy, day or locale changes.
                if(projection != lastPosted) {
                    val open = PendingIntent.getActivity(context, 0,
                        Intent(context, MainActivity::class.java).setAction(Intent.ACTION_MAIN)
                            .setData(Uri.parse("momentum-private://task-count/today"))
                            .setFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP)
                            .putExtra("open-today", true), PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
                    val text = context.resources.getQuantityString(if(mode == TaskCountMode.DUE_TODAY)
                        R.plurals.task_count_due_today else R.plurals.task_count_today_overdue, count, count)
                    manager.notify(tag, ID, NotificationCompat.Builder(context, channelId)
                        .setSmallIcon(R.drawable.ic_launcher_foreground)
                        .setContentTitle(context.getString(R.string.task_count_title)).setContentText(text)
                        .setNumber(count).setContentIntent(open).setAutoCancel(true).setOnlyAlertOnce(true)
                        .setSilent(true).setCategory(NotificationCompat.CATEGORY_STATUS)
                        .setVisibility(NotificationCompat.VISIBILITY_PRIVATE)
                        .setPublicVersion(NotificationCompat.Builder(context, channelId)
                            .setSmallIcon(R.drawable.ic_launcher_foreground)
                            .setContentTitle(context.getString(R.string.app_name)).build()).build())
                    lastPosted = projection
                }
                mutable.value = TaskCountStatus(count = count)
                true
            } catch(e: Exception) {
                if(e is CancellationException) throw e
                cancelPostedNotification()
                mutable.value = TaskCountStatus(error = (e.message ?: e.javaClass.simpleName).take(16_384))
                false
            }
        }
    }
    suspend fun close() { pause(); cancelAndDrain(); scope.cancel() }
    private fun cancelPostedNotification() { if(!isolated) manager.cancel(tag, ID); lastPosted = null }
    private fun allowed() = manager.areNotificationsEnabled() &&
        (Build.VERSION.SDK_INT < 33 || ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED) &&
        manager.getNotificationChannel(channelId)?.importance != NotificationManager.IMPORTANCE_NONE
    private fun ensureChannel() {
        // Existing channel behavior remains under the user's control.
        manager.createNotificationChannel(NotificationChannel(channelId, context.getString(R.string.task_count_title), NotificationManager.IMPORTANCE_LOW).apply {
            description = context.getString(R.string.task_count_channel_hint)
            lockscreenVisibility = android.app.Notification.VISIBILITY_PRIVATE
            setSound(null, null); enableVibration(false); setShowBadge(true)
        })
    }
    companion object { const val CHANNEL = "momentum.task-count"; const val TAG = "momentum.task-count"; const val ID = 0 }
}
