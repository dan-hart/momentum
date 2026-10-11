// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import org.junit.Assert.*
import org.junit.Test

class TaskDragTransportTest {
    @Test fun taskIdsKeepSuppliedBatchOrder() {
        val ids = listOf("second_task", "first-task", "third")
        assertEquals(ids, TaskDragTransport.validIds(ids))
        assertEquals(ids, TaskDragTransport.decodeIds("second_task\nfirst-task\nthird"))
    }

    @Test fun rejectsEmptyDuplicateOversizedAndNonIdentifierPayloads() {
        assertNull(TaskDragTransport.validIds(emptyList()))
        assertNull(TaskDragTransport.validIds(listOf("same", "same")))
        assertNull(TaskDragTransport.validIds(listOf("task\ninjected")))
        assertNull(TaskDragTransport.validIds(listOf("Task title with private notes")))
        assertNull(TaskDragTransport.validIds(listOf("https://private.example/task")))
        assertNull(TaskDragTransport.validIds(listOf("a".repeat(129))))
        assertNull(TaskDragTransport.validIds(List(1001) { "task_$it" }))
        assertNull(TaskDragTransport.decodeIds("task\n"))
    }

    @Test fun externalTextPreservesContentForRustParsing() {
        val items = listOf(ExternalTaskDropItem(text = "  first line\nsecond line  "),
            ExternalTaskDropItem(uri = "https://example.org/page?item=one"))
        assertEquals("  first line\nsecond line  \nhttps://example.org/page?item=one",
            TaskDragTransport.externalText(items))
    }

    @Test fun refusesUriReadsOrIntentFallbackAndBoundsTotalBytes() {
        assertNull(TaskDragTransport.externalText(listOf(ExternalTaskDropItem(uri = "content://provider/private"))))
        assertNull(TaskDragTransport.externalText(listOf(ExternalTaskDropItem(uri = "file:///private/tasks"))))
        assertNull(TaskDragTransport.externalText(listOf(ExternalTaskDropItem())))
        assertNull(TaskDragTransport.externalText(listOf(ExternalTaskDropItem(text = " "))))
        assertNull(TaskDragTransport.externalText(List(33) { ExternalTaskDropItem(text = "task") }))
        assertNull(TaskDragTransport.externalText(listOf(ExternalTaskDropItem(text = "x".repeat(65537)))))
        assertNull(TaskDragTransport.externalText(listOf(ExternalTaskDropItem(text = "€".repeat(22000)))))
    }

    @Test fun externalArchiveDropsAndForgedPrivateMimeAreRejected() {
        assertFalse(TaskDragTransport.accepts(setOf(TaskDragTransport.PRIVATE_MIME), false, false))
        assertTrue(TaskDragTransport.accepts(setOf(TaskDragTransport.PRIVATE_MIME), true, false))
        assertFalse(TaskDragTransport.accepts(setOf("text/plain"), false, false))
        assertTrue(TaskDragTransport.accepts(setOf("text/plain"), false, true))
        assertTrue(TaskDragTransport.accepts(setOf("text/uri-list"), false, true))
        assertFalse(TaskDragTransport.accepts(setOf("image/png"), false, true))
        assertFalse(TaskDragTransport.accepts(setOf(TaskDragTransport.PRIVATE_MIME, "text/plain"), false, true))
    }
}
