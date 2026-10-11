// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import com.codedbydan.momentum.core.*
import org.junit.Assert.*
import org.junit.Test
import java.nio.file.Files

/** Generated Kotlin ↔ Rust imports, with disposable stores and no duplicated parser rules. */
class TextImportCoreTest {
    @Test fun multilineImportUsesSharedParserAndDestinationContext() {
        val directory = Files.createTempDirectory("momentum-text-import-").toFile()
        val engine = Engine.openChecked(directory.absolutePath)
        try {
            val project = engine.addProject("Source project")!!
            assertTrue(engine.addFromText("First task\n\nSecond task\n", View.Project(project)).changed)
            val tasks = engine.allTasks()
            assertEquals(setOf("First task", "Second task"), tasks.map { it.title }.toSet())
            assertTrue(tasks.all { engine.taskDetail(it.id)!!.projectId == project })
            assertTrue(engine.addFromText("https://example.org/source/", View.Morning).changed)
            val link = engine.allTasks().single { it.id !in tasks.map { task -> task.id } }
            assertEquals("example.org/source", link.title)
            assertEquals("https://example.org/source/", link.notes)
            assertEquals(Slot.MORNING, engine.taskMenu(link.id)!!.slot)
        } finally { engine.destroy(); directory.deleteRecursively() }
    }

    @Test fun failedMultilineImportRollsBackAndRetryCreatesOneBatch() {
        val directory = Files.createTempDirectory("momentum-text-import-").toFile()
        val engine = Engine.openChecked(directory.absolutePath)
        try {
            val blocker = java.io.File(directory, ".momentum-transaction").apply { writeText("isolated test fault") }
            val source = "First task\nSecond task"
            assertTrue(engine.addFromText(source, View.Tonight).message is Message.SaveFailed)
            assertTrue(engine.allTasks().isEmpty())
            assertTrue(blocker.delete())
            assertTrue(engine.addFromText(source, View.Tonight).changed)
            val tasks = engine.allTasks()
            assertEquals(2, tasks.size)
            assertTrue(tasks.all { engine.taskMenu(it.id)!!.slot == Slot.TONIGHT })
        } finally { engine.destroy(); directory.deleteRecursively() }
    }
}
