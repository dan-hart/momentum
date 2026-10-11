// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import com.codedbydan.momentum.data.capturedDraft
import org.junit.Assert.*
import org.junit.Test

class CapturedDraftTest {
    @Test fun captureUsesSharedShortSyntaxAndKeepsFullNotes() {
        val draft = capturedDraft("Write Android docs #work 1h 30m", "Source\nsecond line 📝", "2026-10-10", "work, review")
        assertEquals("Write Android docs", draft.title)
        assertEquals("1h 30m", draft.estimate)
        assertEquals("work, review", draft.newTags)
        assertEquals("Source\nsecond line 📝", draft.notes)
        assertEquals("2026-10-10", draft.dueDay)
    }
    @Test fun ordinaryWordsRemainLiteralAndMalformedDateCanBeCorrected() {
        val draft = capturedDraft("Read warmth math", "", "not-a-date", "")
        assertEquals("Read warmth math", draft.title)
        assertEquals("", draft.estimate)
        assertEquals(com.codedbydan.momentum.data.EditorDraft.Problem.Date, draft.problem())
    }
}
