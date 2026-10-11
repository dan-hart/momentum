// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import com.codedbydan.momentum.data.EditorDraft
import org.junit.Assert.*
import org.junit.Test
import java.util.Base64

class TextImportDraftTest {
    @Test fun versionOneTaskDraftRemainsReadableWithoutBecomingAnImport() {
        // An independently encoded v1 fixture: eight strings, reminder, then two tags.
        val draft = EditorDraft.decode("AAAAAQAAAAt0YXNrLWxlZ2FjeQAAAA5FeGlzdGluZyB0aXRsZQAAABxGaXJzdCBsaW5lClp3ZWl0ZSBaZWlsZSDwn5OdAAAAC3Byb2plY3Qtb2xkAAAACjIwMjYtMTAtMTAAAAAFMDg6MzAAAAACMWgAAAAFZnJlc2gAAAAAAAAADwAAAAIABWFscGhhAARiZXRh")!!
        assertFalse(draft.isTextImport)
        assertEquals("task-legacy", draft.id)
        assertEquals("Existing title", draft.title)
        assertEquals("First line\nZweite Zeile 📝", draft.notes)
        assertEquals("project-old", draft.projectId)
        assertEquals("2026-10-10", draft.dueDay)
        assertEquals("08:30", draft.time)
        assertEquals("1h", draft.estimate)
        assertEquals("fresh", draft.newTags)
        assertEquals(15L, draft.reminder)
        assertEquals(listOf("alpha", "beta"), draft.tagIds)
    }

    @Test fun versionTwoRetainsFullImportSourceAndTaskDrafts() {
        val source = "First line\n\nhttps://example.org/source\nPrivate Unicode 📝\n".repeat(5_000)
        val draft = EditorDraft(notes = source, isTextImport = true)
        assertEquals(draft, EditorDraft.decode(draft.encode()))
        val task = EditorDraft(title = "A normal task", tagIds = listOf("work"))
        assertEquals(task, EditorDraft.decode(task.encode()))
        assertFalse(EditorDraft.decode(task.encode())!!.isTextImport)
    }

    @Test fun incompleteOrInvalidVersionTwoMarkerDoesNotSilentlyChangeEditorMode() {
        val encoded = Base64.getDecoder().decode(EditorDraft(notes = "Retain source", isTextImport = true).encode())
        assertNull(EditorDraft.decode(Base64.getEncoder().encodeToString(encoded.copyOf(encoded.size - 1))))
        encoded[encoded.lastIndex] = 2
        assertNull(EditorDraft.decode(Base64.getEncoder().encodeToString(encoded)))
    }

    @Test fun oversizedFieldsAndCombinedDraftsCannotEncodeAnUnreadableFile() {
        assertThrows(IllegalArgumentException::class.java) { EditorDraft(notes = "x".repeat(4_000_001)).encode() }
        assertThrows(IllegalArgumentException::class.java) {
            EditorDraft(title = "x".repeat(2_500_000), notes = "y".repeat(2_500_000)).encode()
        }
        val maximumImport = EditorDraft(notes = "界".repeat(EditorDraft.MAX_TEXT_IMPORT_CHARS), isTextImport = true)
        assertEquals(maximumImport, EditorDraft.decode(maximumImport.encode()))
    }
}
