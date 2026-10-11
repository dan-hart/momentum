// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.platform

import androidx.test.platform.app.InstrumentationRegistry
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.MomentumRepository
import kotlinx.coroutines.*
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import java.io.File
import java.time.LocalDate
import java.util.UUID
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicInteger

/** Only day observation is injected; instance creation and projections use real Rust. */
class CoordinatorLocalDayTest {
    @Test fun backgroundProjectionCreatesOneMissedRepeatBeforeCountAndReminderReads() = runBlocking {
        val fixture = Fixture()
        try {
            val occurrence = fixture.missedRepeat()
            assertEquals(0u, fixture.repository.read { taskCount(TaskCountMode.DUE_TODAY) })
            fixture.dayChanged.set(true)
            val revision = fixture.platform.state.value.revision
            assertTrue(fixture.platform.reconcileReminders())
            assertEquals(1u, fixture.repository.read { taskCount(TaskCountMode.DUE_TODAY) })
            assertNotNull(fixture.repository.detail(occurrence))
            assertTrue(fixture.platform.state.value.revision > revision)
            fixture.assertTodaySummaryUsesActualOccurrence(occurrence)
            assertTrue(fixture.platform.reconcileReminders())
            assertEquals(1, fixture.repository.read { allTasks().count { it.id == occurrence } })
            assertEquals(1, fixture.repository.snapshot(View.Today).listing.sections.flatMap { it.rows }
                .filterIsInstance<Row.Task>().count { it.row.id == occurrence })
        } finally { fixture.close() }
    }

    @Test fun restoreRetainsPendingDayUntilTheOwnerExplicitlyResumes() = runBlocking {
        val fixture = Fixture()
        try {
            val occurrence = fixture.missedRepeat()
            fixture.platform.pauseForRestore()
            fixture.dayChanged.set(true)
            val observations = fixture.observations.get()
            assertTrue(fixture.platform.reconcileReminders())
            assertEquals(observations, fixture.observations.get())
            assertNull(fixture.repository.detail(occurrence))
            assertTrue(fixture.dayChanged.get())
            fixture.platform.resumeAfterRestore()
            assertNotNull(fixture.repository.detail(occurrence))
            assertEquals(1u, fixture.repository.read { taskCount(TaskCountMode.DUE_TODAY) })
        } finally { fixture.close() }
    }

    @Test fun queuedProjectionRechecksAdmissionBeforeConsumingTheCoreDaySignal() = runBlocking {
        val fixture = Fixture()
        try {
            val occurrence = fixture.missedRepeat()
            val entered = CompletableDeferred<Unit>()
            val release = CompletableDeferred<Unit>()
            val owner = async { fixture.repository.read { entered.complete(Unit); release.await() } }
            entered.await()
            fixture.dayChanged.set(true)
            val admitted = AtomicBoolean(true)
            val observations = fixture.observations.get()
            val queued = async(Dispatchers.IO, start = CoroutineStart.UNDISPATCHED) {
                fixture.repository.readProjection({ admitted.get() }) { taskCount(TaskCountMode.DUE_TODAY) }
            }
            admitted.set(false)
            release.complete(Unit)
            owner.await()
            assertNull(queued.await())
            assertEquals(observations, fixture.observations.get())
            assertTrue(fixture.dayChanged.get())
            assertNull(fixture.repository.detail(occurrence))
            assertEquals(1u, fixture.repository.readProjection { taskCount(TaskCountMode.DUE_TODAY) })
            fixture.assertTodaySummaryUsesActualOccurrence(occurrence)
        } finally { fixture.close() }
    }

    private class Fixture {
        private val context = InstrumentationRegistry.getInstrumentation().targetContext
        private val directory = File(context.cacheDir, "local-day-${UUID.randomUUID()}")
        val dayChanged = AtomicBoolean(false)
        val observations = AtomicInteger()
        val repository = MomentumRepository(context, directory) {
            observations.incrementAndGet()
            dayChanged.getAndSet(false)
        }
        val platform = PlatformCoordinator(context, repository)
        private val today = LocalDate.now()

        suspend fun missedRepeat(): String {
            val seed = repository.create(TaskDraft("Private daily fixture", "INBOX_PROJECT", today.toString(),
                null, null, 0.0, "Private repeat notes", emptyList(), emptyList()), View.Today).id!!
            assertTrue(repository.mutate { saveRepeat(seed, repeatDraft(seed)!!.copy(cycle = RepeatCycle.DAILY,
                startDate = today.minusDays(7).toString())) }.changed)
            assertTrue(repository.mutate { setDone(seed, true) }.changed)
            repository.updatePreferences { it.copy(morningSummaryEnabled = true, morningSummaryTime = ClockTime(0u, 0u)) }
            val backup = File(directory, "isolated-missed-repeat.json")
            repository.read { exportBackup(backup.absolutePath) }
            val json = JSONObject(backup.readText())
            val configs = json.getJSONObject("data").getJSONObject("taskRepeatCfg").getJSONObject("entities")
            val cfg = configs.getJSONObject(configs.keys().next())
            cfg.put("lastTaskCreationDay", today.minusDays(1).toString())
            backup.writeText(json.toString())
            repository.read { importBackup(backup.absolutePath) }
            return "rpt_${cfg.getString("id")}_$today"
        }

        suspend fun assertTodaySummaryUsesActualOccurrence(occurrence: String) {
            val plan = repository.read { notificationPlan(System.currentTimeMillis().toULong()) }
            val summary = plan.requests.map { it.content }.filterIsInstance<NotificationContent.Summary>().single { it.day == today.toString() }
            assertEquals(1u, summary.counts.total)
            assertEquals(listOf(occurrence), summary.taskIds)
            assertTrue(plan.projectedOccurrences.none { it.taskId == occurrence })
        }

        suspend fun close() {
            platform.pauseForRestore()
            repository.close()
            directory.deleteRecursively()
        }
    }
}
