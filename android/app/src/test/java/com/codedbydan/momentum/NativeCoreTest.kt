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
}
