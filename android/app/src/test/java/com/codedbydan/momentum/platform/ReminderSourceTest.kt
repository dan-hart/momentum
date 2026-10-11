// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.platform

import com.codedbydan.momentum.core.*
import org.junit.Assert.*
import org.junit.Test
import java.nio.file.Files
import java.io.File
import java.time.LocalDate

/** Uses the generated boundary that the cold Android receiver validates through. */
class ReminderSourceTest {
    @Test fun nativeActionPersistenceFailureThrowsWithoutConsumingTheSourceAndCanRetry() {
        for (action in listOf("done", "snooze")) {
            val directory = Files.createTempDirectory("momentum-reminder-write-test").toFile()
            val engine = Engine.openChecked(directory.absolutePath)
            try {
                val now = System.currentTimeMillis().toULong()
                val id = engine.createTaskWithId(TaskDraft("Private fixture", "INBOX_PROJECT",
                    LocalDate.now().plusDays(1).toString(), ClockTime(12u, 0u), 0u,
                    0.0, "", emptyList(), emptyList()), View.Today).id!!
                val request = engine.notificationPlan(now).requests.single()
                assertTrue(engine.acceptNotification(request, now).accepted)
                val before = engine.taskDetail(id)!!
                val blockedWrite = File(directory, "pending.json.tmp")
                assertTrue(blockedWrite.mkdir())
                val failure = runCatching { ReminderActions.performIfCurrent(engine, request, now, action) }.exceptionOrNull()
                assertTrue("Native persistence failures must propagate through the receiver adapter", failure is CoreException.Io)
                assertEquals(before, engine.taskDetail(id))
                assertTrue(blockedWrite.delete())
                assertTrue("The same accepted source must remain retryable", ReminderActions.performIfCurrent(engine, request, now, action)!!.changed)
            } finally { engine.destroy(); directory.deleteRecursively() }
        }
    }

    @Test fun editedSourceCannotApplyDoneOrSnoozeToTheNewTaskRevision() {
        val directory = Files.createTempDirectory("momentum-reminder-action-test").toFile()
        val engine = Engine.openChecked(directory.absolutePath)
        try {
            val now = System.currentTimeMillis().toULong()
            val draft = TaskDraft("Original", "INBOX_PROJECT", LocalDate.now().plusDays(1).toString(), ClockTime(12u, 0u), 0u,
                0.0, "", emptyList(), emptyList())
            val id = engine.createTaskWithId(draft, View.Today).id!!
            val request = engine.notificationPlan(now).requests.single()
            assertTrue(engine.acceptNotification(request, now).accepted)
            engine.saveTask(id, draft.copy(title = "Edited"))
            assertNull(ReminderActions.performIfCurrent(engine, request, now, "done"))
            assertNull(ReminderActions.performIfCurrent(engine, request, now, "snooze"))
            val unchanged = engine.taskDetail(id)!!
            assertEquals("Edited", unchanged.title)
            assertFalse(unchanged.isDone)
            assertEquals(request.scheduledAtMs, engine.notificationPlan(now).requests.single().scheduledAtMs)
        } finally { engine.destroy(); directory.deleteRecursively() }
    }

    @Test fun notificationSnoozeUsesTheNativeOneHourPolicyAndInvalidatesTheOldSource() {
        val directory = Files.createTempDirectory("momentum-snooze-test").toFile()
        val engine = Engine.openChecked(directory.absolutePath)
        try {
            val now = System.currentTimeMillis()
            val draft = TaskDraft("Reminder", "INBOX_PROJECT", LocalDate.now().plusDays(1).toString(), ClockTime(12u, 0u), 0u,
                0.0, "", emptyList(), emptyList())
            engine.createTaskWithId(draft, View.Today)
            val request = engine.notificationPlan(now.toULong()).requests.single()
            assertTrue(engine.acceptNotification(request, now.toULong()).accepted)
            assertTrue(ReminderActions.performIfCurrent(engine, request, now.toULong(), "snooze")!!.changed)
            val replacement = engine.notificationPlan(System.currentTimeMillis().toULong()).requests.single()
            assertTrue(replacement.fireAtMs.toLong() - now in 3_600_000..3_605_000)
            assertFalse(engine.acceptNotification(request, System.currentTimeMillis().toULong()).accepted)
            assertNull(ReminderActions.performIfCurrent(engine, replacement, now.toULong(), "unknown"))
        } finally { engine.destroy(); directory.deleteRecursively() }
    }

    @Test fun editedSourceRejectsTheAlreadyScheduledRevisionAcrossRestart() {
        val directory = Files.createTempDirectory("momentum-reminder-test").toFile()
        val now = System.currentTimeMillis().toULong()
        val tomorrow = LocalDate.now().plusDays(1)
        val draft = TaskDraft("Original", "INBOX_PROJECT", tomorrow.toString(), ClockTime(12u, 0u), 0u,
            0.0, "", emptyList(), emptyList())
        var engine = Engine.openChecked(directory.absolutePath)
        try {
            val creation = engine.createTaskWithId(draft, View.Today)
            val request = engine.notificationPlan(now).requests.single()
            assertTrue(engine.acceptNotification(request, now).accepted)
            assertTrue(engine.saveTask(creation.id!!, draft.copy(title = "Edited")).changed)
            engine.destroy()
            engine = Engine.openChecked(directory.absolutePath)
            val rejection = engine.acceptNotification(request, now)
            assertFalse(rejection.accepted)
            assertEquals(ReminderIdentity.observation(request), rejection.cancellation)
            val replacement = engine.reconcileNotifications(now, listOf(ReminderIdentity.observation(request)), emptyList()).requests.single()
            assertEquals(request.id, replacement.id)
            assertNotEquals(ReminderIdentity.key(ReminderIdentity.observation(request)), ReminderIdentity.key(ReminderIdentity.observation(replacement)))
            assertTrue(engine.acceptNotification(replacement, now).accepted)
        } finally { engine.destroy(); directory.deleteRecursively() }
    }
}
