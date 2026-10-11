// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import com.codedbydan.momentum.data.EditorDraft
import org.junit.Assert.*
import org.junit.Test

class TextImportInputTest {
    @Test fun pastePreservesExistingSourceAndClipboardLineOrder() {
        assertEquals("Existing\nFirst\nSecond", appendImportSource("Existing", listOf("First", "Second")))
        assertEquals("Existing\nFirst", appendImportSource("Existing\n", listOf("First")))
    }

    @Test fun sizeLimitAcceptsBoundaryAndRejectsEntirePasteWithoutTruncation() {
        val source = "s".repeat(EditorDraft.MAX_TEXT_IMPORT_CHARS - 2)
        assertEquals(EditorDraft.MAX_TEXT_IMPORT_CHARS, appendImportSource(source, listOf("x"))!!.length)
        assertNull(appendImportSource(source, listOf("xy")))
        assertNull(appendImportSource("Retained", listOf("x".repeat(EditorDraft.MAX_TEXT_IMPORT_CHARS))))
        assertEquals(EditorDraft.MAX_TEXT_IMPORT_CHARS - 2, source.length)
    }

    @Test fun laterClipboardItemCannotProduceAPartialImport() {
        assertNull(appendImportSource("Existing", listOf("First", "x".repeat(EditorDraft.MAX_TEXT_IMPORT_CHARS))))
    }
}
