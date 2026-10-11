// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import com.codedbydan.momentum.data.EditorDraft
import org.junit.Assert.*
import org.junit.Test
import java.io.*
import java.util.Base64

class DraftStreamCodecTest {
    @Test fun canonicalVersionTwoBytesRemainIdenticalAndCallerOwnsOutputClose() {
        val draft = EditorDraft(title = "A", notes = "B", projectId = "", reminder = 15,
            tagIds = listOf("nul\u0000😀"), isTextImport = true)
        val expected = Base64.getEncoder().encode(binary(draft, 2))
        val output = OpenOutput()
        draft.writeEncodedTo(output)
        assertArrayEquals(expected, output.bytes.toByteArray())
        assertEquals(String(expected, Charsets.US_ASCII), draft.encode())
        assertFalse("AtomicFile must retain ownership of its file descriptor", output.closed)
        assertEquals(draft, EditorDraft.readEncodedFrom(ByteArrayInputStream(expected)))
    }

    @Test fun theIndependentLegacyVersionOneFixtureRemainsReadable() {
        val fixture = "AAAAAQAAAAt0YXNrLWxlZ2FjeQAAAA5FeGlzdGluZyB0aXRsZQAAABxGaXJzdCBsaW5lClp3ZWl0ZSBaZWlsZSDwn5OdAAAAC3Byb2plY3Qtb2xkAAAACjIwMjYtMTAtMTAAAAAFMDg6MzAAAAACMWgAAAAFZnJlc2gAAAAAAAAADwAAAAIABWFscGhhAARiZXRh"
        val expected = EditorDraft(id = "task-legacy", title = "Existing title",
            notes = "First line\nZweite Zeile 📝", projectId = "project-old", dueDay = "2026-10-10",
            time = "08:30", estimate = "1h", reminder = 15, tagIds = listOf("alpha", "beta"), newTags = "fresh")
        assertEquals(expected, EditorDraft.readEncodedFrom(ByteArrayInputStream(fixture.toByteArray(Charsets.US_ASCII))))
        assertEquals(expected, EditorDraft.decode(fixture))
        assertArrayEquals(binary(expected, 1), Base64.getDecoder().decode(fixture))
    }

    @Test fun shortReadsAndZeroAvailableStillParseTheCompleteRecordWithoutClosingInput() {
        val draft = EditorDraft(title = "Unicode 📝", notes = "A\r\n界", tagIds = listOf("nul\u0000", "😀"))
        val input = ShortInput(Base64.getEncoder().encode(binary(draft, 2)))
        assertEquals(draft, EditorDraft.readEncodedFrom(input))
        assertFalse(input.closed)
        assertEquals(-1, input.read())
    }

    @Test fun streamingOutputNeverSubmitsAWholeLargeEncodedRecord() {
        val draft = EditorDraft(notes = "a".repeat(4_000_000))
        var size = 0L
        var largest = 0
        val sink = object : OutputStream() {
            override fun write(value: Int) { size++ }
            override fun write(bytes: ByteArray, offset: Int, length: Int) {
                assertTrue("Encoded disk writes remain bounded chunks", length <= 65_536)
                size += length; largest = maxOf(largest, length)
            }
        }
        draft.writeEncodedTo(sink)
        assertEquals(4L * ((49L + 13L + 4_000_000L + 2) / 3), size)
        assertTrue(largest > 0)
    }

    @Test fun encodedPaddingAndUnpaddedCompatibilityRequireTheActualRawEnd() {
        val draft = EditorDraft(title = "abc", projectId = "")
        val encoded = Base64.getEncoder().encodeToString(binary(draft, 2))
        assertTrue(encoded.endsWith("="))
        assertEquals(draft, read(encoded))
        assertEquals(draft, read(encoded.trimEnd('=')))
        listOf("A", "AAAA", "\n", "=", "\u0000").forEach { suffix ->
            assertNull("Trailing raw bytes after Base64 padding are invalid", read(encoded + suffix))
            assertNull(EditorDraft.decode(encoded + suffix))
        }
        assertNull(read(encoded.dropLast(1)))
    }

    @Test fun trailingDecodedBytesAndEveryTruncationAreRejected() {
        val bytes = binary(EditorDraft(projectId = "", tagIds = listOf("a")), 2)
        for(size in 0 until bytes.size) assertNull(read(bytes.copyOf(size)))
        assertNull(read(bytes + 0))
        assertNull(read(bytes + byteArrayOf(1, 2, 3)))
    }

    @Test fun invalidVersionsFieldLengthsTagCountsModifiedUtfAndMarkersAreRejected() {
        val minimal = binary(EditorDraft(projectId = ""), 2)
        fun setInt(offset: Int, value: Int) = minimal.copyOf().also {
            it[offset] = (value ushr 24).toByte(); it[offset + 1] = (value ushr 16).toByte()
            it[offset + 2] = (value ushr 8).toByte(); it[offset + 3] = value.toByte()
        }
        listOf(0, 3, Int.MAX_VALUE).forEach { assertNull(read(setInt(0, it))) }
        listOf(-1, 4_000_001, Int.MAX_VALUE).forEach { assertNull(read(setInt(4, it))) }
        listOf(-1, 10_001, Int.MAX_VALUE).forEach { assertNull(read(setInt(44, it))) }
        assertNull(read(minimal.copyOf().also { it[it.lastIndex] = 2 }))
        val malformedTag = binary(EditorDraft(projectId = "", tagIds = listOf("\u0000")), 2)
        malformedTag[50] = 0xff.toByte()
        assertNull(read(malformedTag))
    }

    @Test fun maximumVersionOneRecordAndMaximumVersionTwoRecordRemainReadable() {
        val notes = "a".repeat(4_000_000)
        for(version in 1..2) {
            val draft = EditorDraft(projectId = "", notes = notes, title = "t".repeat(4_500_000 - notes.length - if(version == 1) 48 else 49))
            val bytes = binary(draft, version)
            assertEquals(4_500_000, bytes.size)
            assertTrue("The full maximum-size source remains readable in each version", draft == read(bytes))
            assertNull(read(binary(draft.copy(title = draft.title + "t"), version)))
        }
    }

    @Test fun oversizedRawInputStopsAtTheBoundWithoutConsumingItsWholeTail() {
        val draft = EditorDraft(projectId = "", notes = "n".repeat(4_000_000), title = "t".repeat(499_952))
        val encoded = Base64.getEncoder().encode(binary(draft, 2))
        val input = ShortInput(encoded)
        assertNull(EditorDraft.readEncodedFrom(input))
        assertTrue("Raw read cannot consume beyond the first over-limit byte", input.offset <= 6_000_001)
    }

    @Test fun maximumLegacyStringIsReadableWithoutChangingItsSourceOrEncodedForm() {
        val draft = EditorDraft(projectId = "", notes = "a".repeat(4_000_000), title = "t".repeat(499_952))
        val encoded = Base64.getEncoder().encodeToString(binary(draft, 1))
        assertEquals(6_000_000, encoded.length)
        assertTrue("Inline recovery retains all original legacy fields", draft == EditorDraft.decode(encoded))
        assertNull(EditorDraft.decode(encoded + "A"))
        assertNull(EditorDraft.decode("界" + encoded.drop(1)))
    }

    @Test fun modifiedUtfTagBoundaryAndTenThousandEmptyTagsRoundTripExactly() {
        val tag = "😀".repeat(10_922) + "abc"
        val draft = EditorDraft(tagIds = listOf(tag, "\u0000", "\uD800", "\uDC00"))
        assertTrue("Modified UTF-8 retains surrogate code units and NUL", draft == read(binary(draft, 2)))
        val maximumCount = EditorDraft(tagIds = List(10_000) { "" })
        assertTrue("The existing maximum tag count remains readable", maximumCount == read(binary(maximumCount, 2)))
    }

    @Test fun decoderDoesNotMisreportVmErrorsAsCorruptRecords() {
        val input = object : InputStream() { override fun read(): Int = throw AssertionError("fixture VM error") }
        assertThrows(AssertionError::class.java) { EditorDraft.readEncodedFrom(input) }
    }

    private fun read(encoded: String) = EditorDraft.readEncodedFrom(ByteArrayInputStream(encoded.toByteArray(Charsets.US_ASCII)))
    private fun read(binary: ByteArray) = EditorDraft.readEncodedFrom(ByteArrayInputStream(Base64.getEncoder().encode(binary)))
    private fun binary(draft: EditorDraft, version: Int): ByteArray = ByteArrayOutputStream().also { bytes ->
        DataOutputStream(bytes).use { output ->
            output.writeInt(version)
            listOf(draft.id.orEmpty(), draft.title, draft.notes, draft.projectId, draft.dueDay, draft.time,
                draft.estimate, draft.newTags).forEach { field ->
                val encoded = field.toByteArray(Charsets.UTF_8)
                output.writeInt(encoded.size); output.write(encoded)
            }
            output.writeLong(draft.reminder ?: -1); output.writeInt(draft.tagIds.size)
            draft.tagIds.forEach { output.writeUTF(it) }
            if(version == 2) output.writeBoolean(draft.isTextImport)
        }
    }.toByteArray()

    private class OpenOutput : OutputStream() {
        val bytes = ByteArrayOutputStream()
        var closed = false
        override fun write(value: Int) = bytes.write(value)
        override fun write(value: ByteArray, offset: Int, length: Int) = bytes.write(value, offset, length)
        override fun close() { closed = true }
    }

    private class ShortInput(private val bytes: ByteArray) : InputStream() {
        var offset = 0
        var closed = false
        override fun available() = 0
        override fun read(): Int = if(offset == bytes.size) -1 else bytes[offset++].toInt() and 0xff
        override fun read(target: ByteArray, offset: Int, length: Int): Int {
            if(length == 0) return 0
            if(this.offset == bytes.size) return -1
            val count = minOf(length, 13, bytes.size - this.offset)
            bytes.copyInto(target, offset, this.offset, this.offset + count); this.offset += count
            return count
        }
        override fun close() { closed = true }
    }
}
