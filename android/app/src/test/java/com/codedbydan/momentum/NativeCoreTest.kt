// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import com.codedbydan.momentum.core.*
import org.junit.Assert.*
import org.junit.Test
import java.nio.file.Files

/** Exercises generated Kotlin ↔ Rust calls against disposable host stores. */
class NativeCoreTest {
    private fun draft(title: String) = TaskDraft(title, "INBOX_PROJECT", null, null, null, 0.0, "", emptyList(), emptyList())

    @Test fun morningAndEveningUseSharedSlotRulesAndPersist() {
        val directory = Files.createTempDirectory("momentum-kotlin-").toFile()
        var engine = Engine.openChecked(directory.absolutePath)
        try {
            val created = engine.createTaskWithId(draft("Slot task"), View.Morning)
            val id = created.id!!
            assertEquals(Slot.MORNING, engine.taskMenu(id)!!.slot)
            engine.toggleSlot(listOf(id), Slot.TONIGHT)
            assertEquals(Slot.TONIGHT, engine.taskMenu(id)!!.slot)
            assertFalse(engine.taskDetail(id)!!.tagIds.any { engine.tag(it)?.title == "Morning" })
            engine.destroy()
            engine = Engine.openChecked(directory.absolutePath)
            assertEquals(Slot.TONIGHT, engine.taskMenu(id)!!.slot)
        } finally { engine.destroy(); directory.deleteRecursively() }
    }

    @Test fun generatedDaySnapshotKeepsHiddenWorkAndCompletionInAgreement() {
        val directory = Files.createTempDirectory("momentum-day-status-").toFile()
        val engine = Engine.openChecked(directory.absolutePath)
        try {
            val id = engine.createTaskWithId(draft("Hidden morning").copy(dueDay = java.time.LocalDate.now().toString()), View.Morning).id!!
            engine.setDayPeriodTasksInToday(false)
            val hidden = engine.listingWithDayStatus(View.Today, 100u)
            assertEquals(1u, hidden.status.hiddenMorning)
            assertEquals(DayCompletion.PENDING, hidden.status.completion)
            assertTrue(completionMessageIds(View.Today, hidden.status.completion, 18u).isEmpty())
            engine.setDone(id, true)
            val complete = engine.listingWithDayStatus(View.Today, 100u)
            assertEquals(0u, complete.status.hiddenMorning)
            assertEquals(DayCompletion.DAY_COMPLETE, complete.status.completion)
            assertEquals(40, completionMessageIds(View.Today, complete.status.completion, 18u).size)
            engine.setDone(id, false)
            assertEquals(DayCompletion.PENDING, engine.dayStatus(View.Today).completion)
        } finally { engine.destroy(); directory.deleteRecursively() }
    }

    @Test fun autoarchivedPeriodRetainsItsOwnCompletionAlongsideUnfinishedToday() {
        val directory = Files.createTempDirectory("momentum-period-completion-").toFile()
        val engine = Engine.openChecked(directory.absolutePath)
        try {
            engine.setPreferences(engine.preferences().copy(autoArchive = true))
            val day = java.time.LocalDate.now().toString()
            val morning = engine.createTaskWithId(draft("Morning").copy(dueDay = day), View.Morning).id!!
            val today = engine.createTaskWithId(draft("Still today").copy(dueDay = day), View.Today).id!!
            engine.setDone(morning, true)
            val period = engine.listingWithDayStatus(View.Morning, 100u)
            assertEquals(DayCompletion.PERIOD_COMPLETE, period.status.completion)
            assertFalse(period.listing.sections.flatMap { it.rows }.filterIsInstance<Row.Task>().any { it.row.id == today })
            assertEquals(View.Morning, com.codedbydan.momentum.data.resolvedListingView(View.Morning, engine.viewExists(View.Morning), period.status.completion))
            assertEquals(DayCompletion.PENDING, engine.dayStatus(View.Today).completion)
        } finally { engine.destroy(); directory.deleteRecursively() }
    }

    @Test fun generatedDirectoryAndSearchResolveStableAndroidDestinations() {
        val available = settingsDirectory().filter { SettingsPlatform.ANDROID in it.platforms }
        assertEquals(available.size, available.map { it.id }.distinct().size)
        assertTrue(available.any { it.id == "tasks.dynamic-messages" })
        val metadata = listOf(SettingsEntry("tasks.dynamic-messages", "Tasks", "Dynamic empty-state messages", "", listOf("encouragement")),
            SettingsEntry("sounds.volume", "Sounds", "Volume", "Quiet feedback", listOf("loudness")))
        assertEquals("sounds.volume", searchSettings(" loudness ", metadata).single().id)
        assertEquals("tasks.dynamic-messages", searchSettings("empty messages", metadata).single().id)
        assertTrue(searchSettings("", metadata).isEmpty())
    }

    @Test fun generatedErrorRetainsDetailsAndNeverResetsDamagedStore() {
        val directory = Files.createTempDirectory("momentum-kotlin-").toFile()
        try {
            val journal = java.io.File(directory, ".momentum-transaction")
            journal.writeText("invalid test journal")
            val error = assertThrows(CoreException::class.java) { Engine.openChecked(directory.absolutePath) }
            assertFalse(error.message.isNullOrEmpty())
            assertEquals("invalid test journal", journal.readText())
        } finally { directory.deleteRecursively() }
    }

    @Test fun archiveIsReadOnlyAndExactBatchUndoRestoresFamily() {
        val directory = Files.createTempDirectory("momentum-kotlin-").toFile()
        val engine = Engine.openChecked(directory.absolutePath)
        try {
            val id = engine.createTaskWithId(draft("Parent"), View.Today).id!!
            engine.addSubtask(id, "Child")
            engine.setDone(id, true)
            val archive = engine.archiveDone()
            val rows = engine.listing(View.Archive, 100u).sections.flatMap { it.rows }.filterIsInstance<Row.Task>()
            assertTrue(rows.isNotEmpty())
            assertTrue(rows.all { it.row.archived })
            engine.undoBatch(archive.undo!!)
            assertEquals(1, engine.taskDetail(id)!!.subTasks.size)
            assertFalse(engine.taskRow(id)!!.archived)
        } finally { engine.destroy(); directory.deleteRecursively() }
    }

    @Test fun failedSaveDoesNotPublishHalfAnEditAndRetries() {
        val directory = Files.createTempDirectory("momentum-kotlin-").toFile()
        val engine = Engine.openChecked(directory.absolutePath)
        try {
            val original = draft("Original")
            val id = engine.createTaskWithId(original, View.Today).id!!
            val blocker = java.io.File(directory, ".momentum-transaction")
            blocker.writeText("test fault")
            val result = engine.saveTask(id, original.copy(title = "Changed"))
            assertTrue(result.message is Message.SaveFailed)
            assertEquals("Original", engine.taskDetail(id)!!.title)
            blocker.delete()
            assertTrue(engine.saveTask(id, original.copy(title = "Changed")).changed)
        } finally { engine.destroy(); directory.deleteRecursively() }
    }
    @Test fun generatedDayPeriodBoundaryPolicyHandlesPopulatedAndMissingMiddleSections() {
        val directory = Files.createTempDirectory("momentum-day-period-").toFile()
        val engine = Engine.openChecked(directory.absolutePath)
        try {
            engine.createTaskWithId(draft("Morning").copy(dueDay = java.time.LocalDate.now().toString()), View.Morning)
            engine.createTaskWithId(draft("Evening").copy(dueDay = java.time.LocalDate.now().toString()), View.Tonight)
            fun separators(): List<Int> {
                val listing = engine.listing(View.Today, 100u)
                return listing.sections.indices.filter { dayPeriodSeparatorBefore(listing, it.toUInt(), engine.preferences().groupBy) }
            }
            assertEquals(listOf(1), separators())
            val today = engine.createTaskWithId(draft("Today").copy(dueDay = java.time.LocalDate.now().toString()), View.Today).id!!
            assertEquals(listOf(1, 2), separators())
            engine.setDone(today, true)
            assertEquals(listOf(1), separators())
            engine.stagePortableSettings("""{"version":1,"values":{"grouping":"none"}}""")
            assertEquals(emptyList<Int>(), separators())
            val morning = engine.listing(View.Morning, 100u)
            assertTrue(morning.sections.indices.none { dayPeriodSeparatorBefore(morning, it.toUInt(), GroupBy.MORNING_NIGHT) })
        } finally { engine.destroy(); directory.deleteRecursively() }
    }

    @Test fun generatedDayPeriodBoundaryPolicyRejectsOutOfRangeIndices() {
        val directory = Files.createTempDirectory("momentum-day-period-bounds-").toFile()
        val engine = Engine.openChecked(directory.absolutePath)
        try {
            val empty = engine.listing(View.Today, 100u)
            assertFalse(dayPeriodSeparatorBefore(empty, 0u, GroupBy.MORNING_NIGHT))
            assertFalse(dayPeriodSeparatorBefore(empty, UInt.MAX_VALUE, GroupBy.MORNING_NIGHT))
            engine.createTaskWithId(draft("Morning"), View.Morning)
            val populated = engine.listing(View.Today, 100u)
            assertFalse(dayPeriodSeparatorBefore(populated, populated.sections.size.toUInt(), GroupBy.MORNING_NIGHT))
            assertFalse(dayPeriodSeparatorBefore(populated, UInt.MAX_VALUE, GroupBy.MORNING_NIGHT))
        } finally { engine.destroy(); directory.deleteRecursively() }
    }

    @Test fun generatedLocalVisibilitySetterSurvivesPortableChangesAndPreservesSlotDestinations() {
        val directory = Files.createTempDirectory("momentum-day-period-").toFile()
        val engine = Engine.openChecked(directory.absolutePath)
        try {
            assertTrue(engine.preferences().showDayPeriodTasksInToday)
            val morning = engine.createTaskWithId(draft("Morning").copy(dueDay = java.time.LocalDate.now().toString()), View.Morning).id!!
            val plain = engine.createTaskWithId(draft("Today").copy(dueDay = java.time.LocalDate.now().toString()), View.Today).id!!
            val evening = engine.createTaskWithId(draft("Evening").copy(dueDay = java.time.LocalDate.now().toString()), View.Tonight).id!!
            engine.setDone(evening, true)
            engine.setDayPeriodTasksInToday(false)
            for(group in listOf("morning-night", "none", "project", "tag", "estimate")) {
                engine.stagePortableSettings("""{"version":1,"values":{"grouping":"$group","sort":"title"}}""")
                assertFalse(engine.preferences().showDayPeriodTasksInToday)
                assertEquals(SortKey.TITLE, engine.preferences().sort)
                val rows = engine.listing(View.Today, 100u).sections.flatMap { it.rows }.filterIsInstance<Row.Task>()
                assertEquals(listOf(plain), rows.map { it.row.id })
            }
            assertTrue(engine.listing(View.Morning, 100u).sections.flatMap { it.rows }.filterIsInstance<Row.Task>().any { it.row.id == morning })
            assertTrue(engine.listing(View.Tonight, 100u).sections.flatMap { it.rows }.filterIsInstance<Row.Task>().any { it.row.id == evening })
            assertFalse(engine.portableSettingsJson().contains("show-day-period-tasks-in-today"))
            assertFalse(engine.portableSettingsJson().contains("showDayPeriodTasksInToday"))
            engine.setDayPeriodTasksInToday(true)
            assertEquals(setOf(morning, plain, evening), engine.listing(View.Today, 100u).sections.flatMap { it.rows }.filterIsInstance<Row.Task>().map { it.row.id }.toSet())
        } finally { engine.destroy(); directory.deleteRecursively() }
    }

}
