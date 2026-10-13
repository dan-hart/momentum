// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.platform

import android.app.Notification
import android.app.NotificationManager
import android.content.Intent
import android.os.Build
import androidx.test.platform.app.InstrumentationRegistry
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.MomentumRepository
import kotlinx.coroutines.*
import org.junit.*
import java.io.File
import java.time.LocalDate
import java.util.UUID

/** Explicitly opt-in on disposable emulators: exercises real OS notification APIs. */
class ReminderOsTest {
    private val context get() = InstrumentationRegistry.getInstrumentation().targetContext
    private val notifications get() = context.getSystemService(NotificationManager::class.java)
    private lateinit var directory: File
    private lateinit var repo: MomentumRepository
    private lateinit var scheduler: ReminderScheduler
    private lateinit var request: NotificationRequest
    private lateinit var taskId: String
    private val key get() = ReminderIdentity.key(ReminderIdentity.observation(request))

    @Before fun prepare() = runBlocking {
        Assume.assumeTrue("OS tests require an explicitly disposable emulator",
            InstrumentationRegistry.getArguments().getString("momentum.disposableOsTests") == "true" &&
                Build.HARDWARE in listOf("ranchu", "goldfish"))
        Assume.assumeTrue("Grant notification permission in the disposable test environment", notifications.areNotificationsEnabled())
        directory = File(context.cacheDir, "reminder-os-${UUID.randomUUID()}")
        repo = MomentumRepository(context, directory)
        repo.updateSettings { it.copy(notifications = true) }
        taskId = repo.create(TaskDraft("Private OS fixture", "INBOX_PROJECT", LocalDate.now().plusDays(1).toString(),
            ClockTime(12u, 0u), 0u, 0.0, "Private notes never in notifications", emptyList(), emptyList()), View.Today).id!!
        request = repo.read { notificationPlan(System.currentTimeMillis().toULong()).requests.single() }
        scheduler = ReminderScheduler(context, repo, isolated = false)
        scheduler.reconcile()
    }
    @After fun cleanup() = runBlocking {
        if(::scheduler.isInitialized) scheduler.cancelAndDrain()
        if(::repo.isInitialized) repo.close()
        if(::directory.isInitialized) directory.deleteRecursively()
        Unit
    }
    private suspend fun delivered() {
        Assert.assertTrue(scheduler.receive(Intent().putExtra("momentum.notification.key", key)))
        withTimeout(5_000) { while(notifications.activeNotifications.none { it.tag == key }) delay(20) }
    }
    private suspend fun awaitDismissed() {
        // Android cancellation is queued on its notification worker, like posting.
        withTimeout(5_000) { while(notifications.activeNotifications.any { it.tag == key }) delay(20) }
        Assert.assertTrue(notifications.activeNotifications.none { it.tag == key })
    }
    private fun action(name: String) = Intent().putExtra("momentum.notification.key", key)
        .putExtra("momentum.notification.action", name)

    @Test fun postedReminderIsPrivateAndDoneIsAppliedExactlyOnce() = runBlocking {
        delivered()
        val posted = notifications.activeNotifications.single { it.tag == key }.notification
        Assert.assertEquals(Notification.VISIBILITY_PRIVATE, posted.visibility)
        Assert.assertEquals("Private OS fixture", posted.extras.getString(Notification.EXTRA_TITLE))
        Assert.assertEquals(context.getString(com.codedbydan.momentum.R.string.app_name), posted.publicVersion.extras.getString(Notification.EXTRA_TITLE))
        Assert.assertFalse(posted.extras.toString().contains("Private notes never in notifications"))
        Assert.assertEquals(3, posted.actions.size)
        if(Build.VERSION.SDK_INT >= 31) {
            Assert.assertTrue(posted.actions.all { it.actionIntent.isImmutable })
            Assert.assertTrue(posted.actions.drop(1).all { it.isAuthenticationRequired })
        }
        Assert.assertTrue(scheduler.receive(action("done")))
        Assert.assertTrue(repo.detail(taskId)!!.isDone)
        Assert.assertFalse(scheduler.receive(action("done")))
        awaitDismissed()
        Assert.assertTrue(repo.mutate { undo() }.changed)
        Assert.assertFalse(repo.detail(taskId)!!.isDone)
    }

    @Test fun lockedActionsKeepTheReceiptAndDisplayedReminderForUnlockedRetry() = runBlocking {
        delivered()
        var locked = true
        val guarded = ReminderScheduler(context, repo, isolated = false, deviceLocked = { locked })
        val observations = File(directory, "android-reminder-schedule.json")
        val previous = observations.readBytes()
        for(name in listOf("done", "snooze")) {
            val error = runCatching { guarded.receive(action(name)) }.exceptionOrNull()
            Assert.assertEquals(context.getString(com.codedbydan.momentum.R.string.reminder_unlock_required), error?.message)
            Assert.assertFalse(repo.detail(taskId)!!.isDone)
            Assert.assertArrayEquals(previous, observations.readBytes())
            Assert.assertTrue(notifications.activeNotifications.any { it.tag == key })
        }
        locked = false
        Assert.assertTrue(guarded.receive(action("done")))
        Assert.assertTrue(repo.detail(taskId)!!.isDone)
        Assert.assertFalse(guarded.receive(action("done")))
    }

    @Test fun oneHourSnoozeCancelsThePostedReminderAndRejectsItsOldSource() = runBlocking {
        delivered()
        val start = System.currentTimeMillis()
        Assert.assertTrue(scheduler.receive(action("snooze")))
        Assert.assertFalse(scheduler.receive(action("snooze")))
        val replacement = repo.read { notificationPlan(System.currentTimeMillis().toULong()).requests.single() }
        Assert.assertTrue(replacement.fireAtMs.toLong() - start in 3_600_000..3_605_000)
        awaitDismissed()
    }

    @Test fun failedDurableActionKeepsDisplayedSourceAndCanBeRetried() = runBlocking {
        delivered()
        val blocker = File(directory, "pending.json.tmp").also { Assert.assertTrue(it.mkdir()) }
        Assert.assertTrue(runCatching { scheduler.receive(action("done")) }.exceptionOrNull() is CoreException.Io)
        Assert.assertFalse(repo.detail(taskId)!!.isDone)
        Assert.assertTrue(notifications.activeNotifications.any { it.tag == key })
        Assert.assertTrue(blocker.delete())
        Assert.assertTrue(scheduler.receive(action("done")))
        Assert.assertTrue(repo.detail(taskId)!!.isDone)
    }

    @Test fun optOutHidesPostedReminderEvenWhenObservationsAreUnreadable() = runBlocking {
        delivered()
        val file = File(directory, "android-reminder-schedule.json")
        val valid = file.readBytes()
        val malformed = "{bad observations}".toByteArray()
        file.writeBytes(malformed)
        try {
            repo.updateSettings { it.copy(notifications = false) }
            Assert.assertNotNull(runCatching { scheduler.reconcile() }.exceptionOrNull())
            awaitDismissed()
            Assert.assertArrayEquals(malformed, file.readBytes())
            Assert.assertFalse(repo.detail(taskId)!!.isDone)
        } finally { file.writeBytes(valid) }
    }
}
