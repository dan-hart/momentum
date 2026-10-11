// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.platform

import androidx.test.platform.app.InstrumentationRegistry
import com.codedbydan.momentum.core.*
import org.junit.Assert.*
import org.junit.Test
import java.io.File
import java.io.RandomAccessFile
import java.util.UUID

/** Scheduler storage only: never reads or changes the OS notification collection. */
class ReminderObservationsTest {
    @Test fun roundTripRetainsPendingAndDeliveredExactSourcesAndActions() = fixture { file, observations ->
        val reminder = request("reminder", "one", NotificationContent.Reminder("fixture-id", "Private title", 3_000uL))
        val summary = request("summary", "two", NotificationContent.Summary("2026-10-11", MorningSummary(2u, 1u, 1u), listOf("a", "b")))
        val records = records(ReminderRecord(reminder), ReminderRecord(summary, delivered = true, actions = setOf("done")))
        observations.save(records)
        assertEquals(records, ReminderObservations(file).load())
        val original = file.readBytes()
        assertThrows(IllegalStateException::class.java) {
            observations.save(records(ReminderRecord(reminder, actions = setOf("invalid"))))
        }
        assertArrayEquals(original, file.readBytes())
    }

    @Test fun capacityFailurePreservesAllExistingPendingRecords() = fixture { file, observations ->
        val pending = (0 until ReminderObservations.MAX_RECORDS).map {
            ReminderRecord(request("reminder-$it", "revision-$it", NotificationContent.Reminder("task-$it", "Fixture", null)))
        }
        val full = records(*pending.toTypedArray())
        observations.save(full)
        val original = file.readBytes()
        val overflow = records(*pending.toTypedArray(), ReminderRecord(request("additional", "revision", NotificationContent.Reminder("additional", "Fixture", null))))
        assertThrows(IllegalStateException::class.java) { observations.save(overflow) }
        assertArrayEquals(original, file.readBytes())
        assertEquals(full, observations.load())
    }

    @Test fun malformedAndOversizedInputAreRejectedWithoutOverwritingTheFile() = fixture { file, observations ->
        val malformed = "{\"version\":2,\"records\":[]}".toByteArray()
        file.writeBytes(malformed)
        assertThrows(IllegalStateException::class.java) { observations.load() }
        assertArrayEquals(malformed, file.readBytes())
        RandomAccessFile(file, "rw").use { it.setLength(ReminderObservations.MAX_BYTES.toLong() + 1) }
        assertThrows(IllegalStateException::class.java) { observations.load() }
        assertEquals(ReminderObservations.MAX_BYTES.toLong() + 1, file.length())
        assertArrayEquals(malformed, file.inputStream().use { input -> ByteArray(malformed.size).also { assertEquals(it.size, input.read(it)) } })
    }

    private fun request(id: String, source: String, content: NotificationContent) = NotificationRequest(id, source, 1_000uL, 2_000uL, content)
    private fun records(vararg values: ReminderRecord) = values.associateByTo(linkedMapOf()) {
        ReminderIdentity.key(ReminderIdentity.observation(it.request))
    }
    private fun fixture(block: (File, ReminderObservations) -> Unit) {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val directory = File(context.cacheDir, "reminder-observations-test-${UUID.randomUUID()}").also { it.mkdirs() }
        val file = File(directory, "observations.json")
        try { block(file, ReminderObservations(file)) } finally { directory.deleteRecursively() }
    }
}
