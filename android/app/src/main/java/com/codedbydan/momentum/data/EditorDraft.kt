// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.data

import java.io.*
import java.time.LocalDate
import java.util.Base64

/** Presentation draft only. Rust validates and commits task rules. */
data class EditorDraft(
    val id: String? = null,
    val title: String = "",
    val notes: String = "",
    val projectId: String = "INBOX_PROJECT",
    val dueDay: String = "",
    val time: String = "",
    val estimate: String = "",
    val reminder: Long? = null,
    val tagIds: List<String> = emptyList(),
    val newTags: String = "",
    val isTextImport: Boolean = false,
) {
    enum class Problem { Title, Date, Time, Estimate }
    fun problem(): Problem? = when {
        title.isBlank() -> Problem.Title
        dueDay.isNotBlank() && runCatching { LocalDate.parse(dueDay) }.isFailure -> Problem.Date
        else -> null
    }
    val hasContent: Boolean get() = title.isNotBlank() || notes.isNotBlank() || newTags.isNotBlank()

    // Full versioned drafts stay on disk; SavedStateHandle contains only their ID/context.
    fun encode(): String = ByteArrayOutputStream().also { writeEncodedTo(it) }.toString(Charsets.US_ASCII.name())

    /** Finish Base64 padding, but leave the raw stream open for AtomicFile's fsync/close. */
    internal fun writeEncodedTo(output: OutputStream) {
        Base64.getEncoder().wrap(DraftNonClosingOutput(output)).use { encoded ->
            val counted = DraftBoundedOutput(encoded, MAX_RECORD_BYTES)
            val out = DataOutputStream(counted)
            require(tagIds.size <= MAX_TAG_IDS) { "The task draft contains too many tags" }
            out.writeInt(2)
            listOf(id.orEmpty(), title, notes, projectId, dueDay, time, estimate, newTags).forEach {
                // Independent per-field conversion preserves the existing UTF-8 replacement
                // behavior, including a dangling surrogate at a field boundary.
                val bytes = it.toByteArray(Charsets.UTF_8)
                require(bytes.size <= MAX_FIELD_BYTES) { "A task draft field is too large to save" }
                require(counted.bytes + 4L + bytes.size <= MAX_RECORD_BYTES) { "The task draft is too large to save" }
                out.writeInt(bytes.size); out.write(bytes)
            }
            out.writeLong(reminder ?: -1)
            out.writeInt(tagIds.size)
            tagIds.forEach { out.writeUTF(it) }
            out.writeBoolean(isTextImport)
            out.flush()
        }
    }

    companion object {
        const val MAX_TEXT_IMPORT_CHARS = 1_000_000
        const val MAX_FIELD_BYTES = 4_000_000
        const val MAX_RECORD_BYTES = 4_500_000
        const val MAX_ENCODED_CHARS = 6_000_000
        const val MAX_TAG_IDS = 10_000
        const val MAX_TAG_MODIFIED_UTF8_BYTES = 65_535
        fun decode(value: String): EditorDraft? = try {
            require(value.length <= MAX_ENCODED_CHARS)
            readEncodedFrom(DraftStringInput(value))
        } catch(_: Exception) { null }

        /** Caller owns the input stream. Both encoded and decoded byte counts are bounded. */
        internal fun readEncodedFrom(source: InputStream): EditorDraft? = try {
            // Buffer after bounding the raw source: Decoder.wrap reads bytes individually,
            // and the same buffer must retain any bytes read ahead beyond Base64 padding.
            val raw = BufferedInputStream(DraftBoundedInput(source, MAX_ENCODED_CHARS), 8192)
            val decoded = DraftBoundedInput(Base64.getDecoder().wrap(raw), MAX_RECORD_BYTES)
            val input = DataInputStream(decoded)
            val version = input.readInt(); require(version == 1 || version == 2)
            val fields = List(8) {
                val size = input.readInt()
                require(size in 0..MAX_FIELD_BYTES && size <= decoded.remaining)
                ByteArray(size).also { input.readFully(it) }.toString(Charsets.UTF_8)
            }
            val reminder = input.readLong().takeIf { it >= 0 }
            val count = input.readInt(); require(count in 0..MAX_TAG_IDS)
            val tags = List(count) { input.readUTF() }
            val isTextImport = if(version == 2) {
                val marker = input.readUnsignedByte(); require(marker == 0 || marker == 1)
                marker == 1
            } else false
            require(input.read() == -1)
            // Decoder.wrap stops at padding; its EOF alone does not reject trailing raw data.
            require(raw.read() == -1)
            EditorDraft(fields[0].takeIf { it.isNotEmpty() }, fields[1], fields[2], fields[3],
                fields[4], fields[5], fields[6], reminder, tags, fields[7], isTextImport)
        } catch(_: Exception) { null }
    }
}

/** Existing inline SavedState records already own a String; do not copy another six MB. */
private class DraftStringInput(private val value: String) : InputStream() {
    private var offset = 0
    override fun read(): Int {
        if(offset == value.length) return -1
        val code = value[offset++].code
        // Base64's alphabet is ASCII; a non-ASCII code unit must remain invalid.
        return if(code <= 0x7f) code else 0
    }
    override fun read(target: ByteArray, offset: Int, length: Int): Int {
        if(offset < 0 || length < 0 || offset > target.size - length) throw IndexOutOfBoundsException()
        if(length == 0) return 0
        if(this.offset == value.length) return -1
        val count = minOf(length, value.length - this.offset)
        for(index in 0 until count) target[offset + index] = read().toByte()
        return count
    }
}

private class DraftBoundedInput(private val input: InputStream, private val limit: Int) : InputStream() {
    private var bytes = 0
    val remaining: Int get() = limit - bytes
    override fun read(): Int = input.read().also { value ->
        if(value != -1) {
            bytes++
            require(bytes <= limit) { "The existing task draft is too large to read" }
        }
    }
    override fun read(target: ByteArray, offset: Int, length: Int): Int {
        if(offset < 0 || length < 0 || offset > target.size - length) throw IndexOutOfBoundsException()
        if(length == 0) return 0
        val count = input.read(target, offset, minOf(length, remaining + 1))
        if(count > 0) {
            bytes += count
            require(bytes <= limit) { "The existing task draft is too large to read" }
        }
        return count
    }
}

private class DraftBoundedOutput(private val output: OutputStream, private val limit: Int) : OutputStream() {
    var bytes = 0
        private set
    override fun write(value: Int) {
        require(bytes < limit) { "The task draft is too large to save" }
        output.write(value); bytes++
    }
    override fun write(value: ByteArray, offset: Int, length: Int) {
        require(bytes + length.toLong() <= limit) { "The task draft is too large to save" }
        output.write(value, offset, length); bytes += length
    }
    override fun flush() = output.flush()
}

private class DraftNonClosingOutput(private val output: OutputStream) : OutputStream() {
    override fun write(value: Int) = output.write(value)
    override fun write(value: ByteArray, offset: Int, length: Int) = output.write(value, offset, length)
    override fun flush() = output.flush()
    override fun close() = flush()
}
