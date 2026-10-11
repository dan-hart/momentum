// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import com.codedbydan.momentum.data.DraftStorageBudget
import com.codedbydan.momentum.data.EditorDraft
import org.junit.Assert.*
import org.junit.Test
import java.io.ByteArrayOutputStream
import java.io.DataOutputStream
import java.util.Base64

class DraftStorageBudgetTest {
    @Test fun everyOrdinaryFieldAcceptsExactlyFourMillionAsciiBytesAndRejectsOneMore() {
        val exact = "a".repeat(4_000_000)
        val tooLarge = exact + "b"
        val fields: List<(String) -> EditorDraft> = listOf(
            { EditorDraft(id = it, projectId = "") },
            { EditorDraft(title = it, projectId = "") },
            { EditorDraft(notes = it, projectId = "") },
            { EditorDraft(projectId = it) },
            { EditorDraft(dueDay = it, projectId = "") },
            { EditorDraft(time = it, projectId = "") },
            { EditorDraft(estimate = it, projectId = "") },
            { EditorDraft(newTags = it, projectId = "") },
        )
        fields.forEach { field ->
            val budget = DraftStorageBudget()
            assertTrue(budget.admit(field(exact)))
            assertFalse(budget.admit(field(tooLarge)))
        }
    }

    @Test fun multibyteAndSupplementaryFieldsUseEncodedBytesRatherThanCharacterCount() {
        listOf("é".repeat(2_000_000), "界".repeat(1_333_333) + "a", "😀".repeat(1_000_000)).forEach { exact ->
            assertEquals(4_000_000, exact.toByteArray(Charsets.UTF_8).size)
            val budget = DraftStorageBudget()
            assertTrue(budget.admit(EditorDraft(notes = exact)))
            assertFalse(budget.admit(EditorDraft(notes = exact + "a")))
        }
    }

    @Test fun malformedSurrogatesFollowTheActualPlatformUtf8Replacement() {
        listOf("\uD800", "\uDC00", "\uD800\uD800", "\uDC00\uDC00", "\uD800a\uDC00", "\uD800\uDC00\uD800").forEach { suffix ->
            val prefix = "x".repeat(4_000_000 - suffix.toByteArray(Charsets.UTF_8).size)
            val exact = EditorDraft(notes = prefix + suffix)
            assertTrue(DraftStorageBudget().admit(exact))
            assertTrue(runCatching { exact.encode() }.isSuccess)
            assertFalse(DraftStorageBudget().admit(exact.copy(notes = exact.notes + "a")))
        }
    }

    @Test fun theCompleteRecordAcceptsExactlyFourPointFiveMillionBytesAndRejectsOneMore() {
        val exact = EditorDraft(projectId = "", notes = "n".repeat(4_000_000), title = "t".repeat(499_951),
            reminder = 123L, isTextImport = true)
        val encoded = exact.encode()
        assertEquals(4_500_000, Base64.getDecoder().decode(encoded).size)
        assertEquals(6_000_000, encoded.length)
        val budget = DraftStorageBudget()
        assertTrue(budget.admit(exact))
        assertFalse(budget.admit(exact.copy(title = exact.title + "t")))
        assertTrue(budget.admit(exact.copy(reminder = Long.MAX_VALUE, isTextImport = false)))
    }

    @Test fun nulAndSupplementaryTagIdsUseModifiedUtf8IncludingTheirLengthPrefixes() {
        val tag = "nul\u0000😀"
        val tagBytes = modifiedRecord(tag).size
        assertEquals(13, tagBytes)
        val exact = EditorDraft(projectId = "", notes = "n".repeat(4_000_000),
            title = "t".repeat(499_951 - tagBytes), tagIds = listOf(tag))
        assertEquals(4_500_000, Base64.getDecoder().decode(exact.encode()).size)
        assertTrue(DraftStorageBudget().admit(exact))
        assertFalse(DraftStorageBudget().admit(exact.copy(title = exact.title + "t")))
    }

    @Test fun eachTagAcceptsTheWriteUtfByteMaximumAndRejectsOneMore() {
        listOf("a".repeat(65_535), "\u0000".repeat(32_767) + "a",
            "😀".repeat(10_922) + "abc", "\uD800".repeat(21_845)).forEach { exact ->
            assertEquals(65_537, modifiedRecord(exact).size)
            assertTrue(DraftStorageBudget().admit(EditorDraft(tagIds = listOf(exact))))
            assertFalse(DraftStorageBudget().admit(EditorDraft(tagIds = listOf(exact + "a"))))
        }
    }

    @Test fun tagCountIncludesEmptyTagsWithoutExceedingItsLimit() {
        assertTrue(DraftStorageBudget().admit(EditorDraft(tagIds = List(10_000) { "" })))
        assertFalse(DraftStorageBudget().admit(EditorDraft(tagIds = List(10_001) { "" })))
    }

    @Test fun admissionAgreesWithTheExistingVersionTwoEncoder() {
        val random = java.util.Random(91)
        val alphabet = listOf("a", "\u0000", "é", "界", "😀", "\uD800", "\uDC00")
        val examples = List(100) {
            val text = buildString { repeat(random.nextInt(50)) { append(alphabet[random.nextInt(alphabet.size)]) } }
            EditorDraft(title = text, notes = text.reversed(), newTags = text,
                tagIds = listOf(text, text.reversed()), isTextImport = random.nextBoolean())
        } + listOf(
            EditorDraft(notes = "x".repeat(4_000_001)),
            EditorDraft(title = "x".repeat(2_500_000), notes = "y".repeat(2_500_000)),
            EditorDraft(tagIds = listOf("x".repeat(65_536))),
            EditorDraft(tagIds = List(10_001) { "" }),
        )
        val budget = DraftStorageBudget()
        examples.forEach { assertEquals(runCatching { it.encode() }.isSuccess, budget.admit(it)) }
    }

    @Test fun aRejectedProposalDoesNotPreventFurtherEditsWithTheSameLargeNotes() {
        val notes = "😀".repeat(1_000_000)
        val budget = DraftStorageBudget()
        val admitted = EditorDraft(title = "Original", notes = notes)
        assertTrue(budget.admit(admitted))
        assertFalse(budget.admit(admitted.copy(notes = notes + "😀")))
        assertTrue(budget.admit(admitted.copy(title = "Changed", tagIds = listOf("work"))))
        assertTrue(budget.admit(admitted.copy(title = "Changed again")))
    }

    @Test fun changingAnAliasedTagListCannotReuseAnOutdatedAdmission() {
        val tags = mutableListOf("work")
        val budget = DraftStorageBudget()
        val draft = EditorDraft(notes = "Retained", tagIds = tags)
        assertTrue(budget.admit(draft))
        tags[0] = "x".repeat(65_536)
        assertFalse(budget.admit(draft.copy(title = "Changed")))
        tags[0] = "next"
        assertTrue(budget.admit(draft.copy(title = "Changed")))
        tags.addAll(List(10_000) { "" })
        assertFalse(budget.admit(draft))
    }

    private fun modifiedRecord(value: String): ByteArray = ByteArrayOutputStream().also { bytes ->
        DataOutputStream(bytes).use { it.writeUTF(value) }
    }.toByteArray()
}
