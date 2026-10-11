// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.data

import java.lang.ref.WeakReference

/**
 * Exact version-two private-draft admission without serializing the record.
 * One editor owner retains only the last admitted fields and at most 10,000 tag references.
 */
class DraftStorageBudget {
    private class Accepted(
        val fields: Array<String>,
        val fieldBytes: IntArray,
        val tagList: WeakReference<List<String>>,
        val tags: Array<String>,
        val tagBytes: Long,
    )

    private var accepted: Accepted? = null

    /** False leaves both the admitted cache and the caller's previous draft intact. */
    fun admit(draft: EditorDraft): Boolean {
        if (draft.tagIds.size > EditorDraft.MAX_TAG_IDS) return false
        val previous = accepted
        val fields = arrayOf(draft.id.orEmpty(), draft.title, draft.notes, draft.projectId,
            draft.dueDay, draft.time, draft.estimate, draft.newTags)
        val fieldBytes = IntArray(fields.size)
        // Version + eight length prefixes + reminder + tag count + import marker.
        var recordBytes = 49L
        for (index in fields.indices) {
            val size = if (previous != null && fields[index] === previous.fields[index]) {
                previous.fieldBytes[index]
            } else utf8Bytes(fields[index], EditorDraft.MAX_FIELD_BYTES)
            if (size > EditorDraft.MAX_FIELD_BYTES) return false
            fieldBytes[index] = size
            recordBytes += size
            if (recordBytes > EditorDraft.MAX_RECORD_BYTES) return false
        }

        // Kotlin List is not necessarily immutable: check captured element identities as well.
        val reuseTags = previous != null && draft.tagIds === previous.tagList.get() &&
            draft.tagIds.size == previous.tags.size &&
            draft.tagIds.indices.all { draft.tagIds[it] === previous.tags[it] }
        var tagBytes = if (reuseTags) previous.tagBytes else 0L
        if (!reuseTags) {
            for (tag in draft.tagIds) {
                val size = modifiedUtf8Bytes(tag, EditorDraft.MAX_TAG_MODIFIED_UTF8_BYTES)
                if (size > EditorDraft.MAX_TAG_MODIFIED_UTF8_BYTES) return false
                tagBytes += 2L + size
                if (recordBytes + tagBytes > EditorDraft.MAX_RECORD_BYTES) return false
            }
        }
        if (recordBytes + tagBytes > EditorDraft.MAX_RECORD_BYTES) return false
        // Do not retain an aliased mutable list that could later grow past the admitted bound.
        accepted = Accepted(fields, fieldBytes, WeakReference(draft.tagIds),
            if (reuseTags) previous.tags else draft.tagIds.toTypedArray(), tagBytes)
        return true
    }

    private companion object {
        // String.getBytes replacement differs between runtimes; derive each once, on this runtime.
        val highSurrogateBytes = "\uD800".toByteArray(Charsets.UTF_8).size
        val lowSurrogateBytes = "\uDC00".toByteArray(Charsets.UTF_8).size

        /** Capped count; no text-sized byte array or encoded record is allocated. */
        fun utf8Bytes(value: String, limit: Int): Int {
            var size = 0
            var index = 0
            while (index < value.length) {
                val code = value[index].code
                size += when {
                    code <= 0x7f -> 1
                    code <= 0x7ff -> 2
                    code in 0xd800..0xdbff -> {
                        if (index + 1 < value.length && value[index + 1].code in 0xdc00..0xdfff) {
                            index++
                            4
                        } else highSurrogateBytes
                    }
                    code in 0xdc00..0xdfff -> lowSurrogateBytes
                    else -> 3
                }
                if (size > limit) return limit + 1
                index++
            }
            return size
        }

        /** DataOutputStream.writeUTF encodes UTF-16 code units, including NUL and surrogates. */
        fun modifiedUtf8Bytes(value: String, limit: Int): Int {
            var size = 0
            var index = 0
            while (index < value.length) {
                size += when (value[index].code) {
                    in 1..0x7f -> 1
                    in 0..0x7ff -> 2
                    else -> 3
                }
                if (size > limit) return limit + 1
                index++
            }
            return size
        }
    }
}
