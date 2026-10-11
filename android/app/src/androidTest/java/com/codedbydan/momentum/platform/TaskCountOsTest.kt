// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.platform

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.os.Build
import androidx.test.platform.app.InstrumentationRegistry
import com.codedbydan.momentum.R
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.MomentumRepository
import kotlinx.coroutines.*
import org.junit.*
import java.io.File
import java.time.LocalDate
import java.util.UUID

/** Real framework notifications; unique store, channel and tag on a disposable emulator. */
class TaskCountOsTest {
    private val context get() = InstrumentationRegistry.getInstrumentation().targetContext
    private val manager get() = context.getSystemService(NotificationManager::class.java)
    private lateinit var repository: MomentumRepository
    private lateinit var directory: File
    private lateinit var notifier: TaskCountNotifier
    private val channel = "momentum.count-test.${UUID.randomUUID()}"
    private val blockedChannel = "$channel.blocked"
    private val tag = channel
    @Before fun prepare() = runBlocking {
        Assume.assumeTrue(InstrumentationRegistry.getArguments().getString("momentum.disposableOsTests") == "true" &&
            Build.HARDWARE in listOf("ranchu", "goldfish"))
        Assume.assumeTrue(manager.areNotificationsEnabled())
        directory = File(context.cacheDir, "task-count-${UUID.randomUUID()}")
        repository = MomentumRepository(context, directory)
        notifier = TaskCountNotifier(context, repository, isolated = false, channelId = channel, tag = tag)
    }
    @After fun cleanup() = runBlocking {
        if(::notifier.isInitialized) notifier.close()
        manager.deleteNotificationChannel(channel)
        manager.deleteNotificationChannel(blockedChannel)
        if(::repository.isInitialized) repository.close()
        if(::directory.isInitialized) directory.deleteRecursively()
        Unit
    }
    private suspend fun posted(expectedCount: Int): Notification = withTimeout(5_000) {
        var notification = manager.activeNotifications.singleOrNull { it.tag == tag }?.notification
        var pendingChecks = 0
        // notify() also queues OS publication. An older notification with the same
        // tag is not evidence that the requested core projection has been posted.
        while(notification == null || notification.number != expectedCount) {
            pendingChecks++
            delay(20)
            notification = manager.activeNotifications.singleOrNull { it.tag == tag }?.notification
        }
        android.util.Log.i("TaskCountOsTest", "Owned notification publication pending checks=$pendingChecks count=$expectedCount")
        notification
    }
    private suspend fun cancelled() {
        var pendingChecks = 0
        // Android 8 NotificationManagerService queues cancellation on its worker handler.
        // A returning cancel() is not a barrier for activeNotifications readback.
        withTimeout(5_000) {
            while(manager.activeNotifications.any { it.tag == tag }) {
                pendingChecks++
                delay(20)
            }
        }
        android.util.Log.i("TaskCountOsTest", "Owned notification removal pending checks=$pendingChecks")
        Assert.assertTrue("The exact owned notification is removed", manager.activeNotifications.none { it.tag == tag })
    }
    private suspend fun create(title: String, day: LocalDate) = repository.create(TaskDraft(title, "INBOX_PROJECT", day.toString(),
        null, null, 0.0, "Private source notes", emptyList(), emptyList()), View.Today).id!!

    @Test fun optionalCountUsesCoreModesAndPrivateQuietNotificationWithoutTaskContents() = runBlocking {
        val first = create("Private count fixture title", LocalDate.now())
        create("Private overdue title", LocalDate.now().minusDays(1))
        Assert.assertEquals(TaskCountMode.NONE, repository.settings().taskCountMode)
        Assert.assertTrue(notifier.reconcile())
        Assert.assertTrue("Notification cancellation must preserve the calling job", currentCoroutineContext().isActive)
        cancelled()
        for(mode in listOf(TaskCountMode.DUE_TODAY, TaskCountMode.TODAY_INCLUDING_OVERDUE)) {
            repository.updateSettings { it.copy(taskCountMode = mode) }
            Assert.assertFalse(repository.settings().notifications) // Count opt-in is independent.
            Assert.assertTrue(notifier.reconcile())
            Assert.assertTrue("Notification publication must preserve the calling job", currentCoroutineContext().isActive)
            val expectedCount = repository.read { taskCount(mode) }.toInt()
            Assert.assertEquals(expectedCount, notifier.state.value.count)
            val notification = posted(expectedCount)
            Assert.assertEquals(expectedCount, notification.number)
            Assert.assertEquals(Notification.VISIBILITY_PRIVATE, notification.visibility)
            Assert.assertEquals(context.getString(R.string.app_name), notification.publicVersion.extras.getString(Notification.EXTRA_TITLE))
            Assert.assertNull(notification.publicVersion.extras.getString(Notification.EXTRA_TEXT))
            Assert.assertFalse(notification.extras.toString().contains("Private"))
            Assert.assertEquals(NotificationManager.IMPORTANCE_LOW, manager.getNotificationChannel(channel).importance)
            Assert.assertNull(manager.getNotificationChannel(channel).sound)
            Assert.assertFalse(manager.getNotificationChannel(channel).shouldVibrate())
            if(Build.VERSION.SDK_INT >= 31) Assert.assertTrue(notification.contentIntent.isImmutable)
        }
        repository.mutate { setDone(first, true) }
        Assert.assertTrue(notifier.reconcile())
        Assert.assertTrue("Notification publication must preserve the calling job", currentCoroutineContext().isActive)
        val completedCount = repository.read { taskCount(TaskCountMode.TODAY_INCLUDING_OVERDUE) }.toInt()
        Assert.assertEquals(completedCount, notifier.state.value.count)
        Assert.assertEquals(completedCount, posted(completedCount).number)
        repository.updateSettings { it.copy(taskCountMode = TaskCountMode.NONE) }
        Assert.assertTrue(notifier.reconcile())
        Assert.assertTrue("Notification cancellation must preserve the calling job", currentCoroutineContext().isActive)
        cancelled()
    }

    @Test fun zeroRestoreAndBlockedChannelCancelTheCountWithoutChangingTaskState() = runBlocking {
        val id = create("Private policy fixture", LocalDate.now())
        repository.updateSettings { it.copy(taskCountMode = TaskCountMode.DUE_TODAY) }
        Assert.assertTrue(notifier.reconcile()); posted(1)
        notifier.pause(); notifier.cancelAndDrain()
        Assert.assertTrue("Restore drain must preserve the calling job", currentCoroutineContext().isActive)
        cancelled()
        Assert.assertTrue(notifier.reconcile())
        Assert.assertTrue("Notification cancellation must preserve the calling job", currentCoroutineContext().isActive)
        cancelled()
        notifier.resume(); Assert.assertTrue(notifier.reconcile()); posted(1)
        repository.mutate { setDone(id, true) }
        Assert.assertTrue(notifier.reconcile())
        Assert.assertTrue("Notification cancellation must preserve the calling job", currentCoroutineContext().isActive)
        cancelled()
        repository.mutate { undo() }
        Assert.assertTrue(notifier.reconcile()); posted(1)
        val previousCount = manager.activeNotifications.single { it.tag == tag }
        notifier.close()
        // API26 ignores an app's importance update for an existing channel. Start the
        // separate fixture blocked, then ensure adapter channel recreation preserves it.
        manager.createNotificationChannel(NotificationChannel(blockedChannel, "Disposable blocked count", NotificationManager.IMPORTANCE_NONE))
        Assert.assertEquals(NotificationManager.IMPORTANCE_NONE, manager.getNotificationChannel(blockedChannel).importance)
        notifier = TaskCountNotifier(context, repository, isolated = false, channelId = blockedChannel, tag = tag)
        // Retain a real count posted to the earlier allowed channel under our owned
        // tag: blocked reconciliation must cancel it, not merely avoid new delivery.
        manager.notify(tag, previousCount.id, previousCount.notification)
        posted(1)
        Assert.assertTrue(notifier.reconcile())
        Assert.assertEquals(NotificationManager.IMPORTANCE_NONE, manager.getNotificationChannel(blockedChannel).importance)
        Assert.assertTrue(notifier.state.value.blocked)
        Assert.assertTrue("Blocked cancellation must preserve the calling job", currentCoroutineContext().isActive)
        cancelled()
        Assert.assertFalse(repository.detail(id)!!.isDone)
        Assert.assertEquals("Private source notes", repository.detail(id)!!.notes)
    }

    @Test fun countModePersistsAndResetPreservesOtherSettingsAcrossRepositoryReopen() = runBlocking {
        Assert.assertEquals(TaskCountMode.NONE, repository.settings().taskCountMode)
        repository.updateSettings { it.copy(taskCountMode = TaskCountMode.TODAY_INCLUDING_OVERDUE, compact = true) }
        repository.close()
        repository = MomentumRepository(context, directory)
        Assert.assertEquals(TaskCountMode.TODAY_INCLUDING_OVERDUE, repository.settings().taskCountMode)
        repository.updateSettings { it.copy(taskCountMode = TaskCountMode.NONE) }
        Assert.assertTrue(repository.settings().compact)
        Assert.assertFalse(repository.settings().notifications)
        Assert.assertEquals(TaskCountMode.NONE, repository.settings().taskCountMode)
    }
}
