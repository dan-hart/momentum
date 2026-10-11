// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import com.codedbydan.momentum.data.EditorDraft
import org.junit.Assert.*
import org.junit.Test

class EditorValidationTest {
    @Test fun rejectsEmptyTitleAndInvalidCalendarDay() {
        assertEquals(EditorDraft.Problem.Title, EditorDraft(title = "  ").problem())
        assertEquals(EditorDraft.Problem.Date, EditorDraft(title = "Task", dueDay = "2026-02-30").problem())
        assertNull(EditorDraft(title = "Task", dueDay = "2028-02-29").problem())
    }

    @Test fun roundTripPreservesDraftAndSelectedTagOrder() {
        val draft = EditorDraft(id = "task", title = "Übermorgen", notes = "line one\nline two",
            projectId = "project", dueDay = "2026-10-10", time = "14:30", estimate = "1h",
            tagIds = listOf("second", "first"), newTags = "new", reminder = 15)
        assertEquals(draft, EditorDraft.decode(draft.encode()))
    }

    @Test fun cancelledBlankDraftIsNotTaskContent() {
        assertFalse(EditorDraft().hasContent)
        assertTrue(EditorDraft(notes = "Do not discard").hasContent)
    }
}
