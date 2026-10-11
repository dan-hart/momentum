// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.data

import android.util.AtomicFile
import com.codedbydan.momentum.core.View
import java.io.*
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap

data class QueuedCapture(val id: String, val draft: EditorDraft, val origin: View)

/** Private native review queue. The bounded index contains references, never shared text. */
class CaptureQueue(directory: File) {
    class Full : IllegalStateException()
    private data class Reference(val id: String, val origin: View)
    private val index = AtomicFile(File(directory, "android-drafts/capture-queue"))
    private val drafts = DraftStore(directory)
    private val lock = locks.computeIfAbsent(directory.absolutePath) { Any() }

    fun enqueue(draft: EditorDraft, origin: View): String = synchronized(lock) {
        val references = readIndex()
        if(references.size >= CAPACITY) throw Full()
        val id = UUID.randomUUID().toString()
        drafts.write(id, draft)
        try { writeIndex(references + Reference(id, origin)) }
        catch(e: Exception) { drafts.delete(id); throw e }
        id
    }
    fun contains(id: String): Boolean = synchronized(lock) { readIndex().any { it.id == id } }
    fun size(): Int = synchronized(lock) { readIndex().size }
    fun head(): QueuedCapture? = synchronized(lock) {
        readIndex().firstOrNull()?.let {
            QueuedCapture(it.id, requireNotNull(drafts.read(it.id)) { "A queued capture could not be read" }, it.origin)
        }
    }
    /** Keep the active head until review ends; a process restart can recover it without a Bundle. */
    fun complete(id: String) = synchronized(lock) {
        val references = readIndex()
        if(references.any { it.id == id }) {
            writeIndex(references.filterNot { it.id == id })
            drafts.delete(id)
        }
    }

    private fun readIndex(): List<Reference> {
        if(!index.baseFile.exists() && !File(index.baseFile.path + ".bak").exists()) return emptyList()
        val bytes = index.openRead().use { input ->
            val output = ByteArrayOutputStream()
            val buffer = ByteArray(4096)
            while(true) {
                val size = input.read(buffer)
                if(size < 0) break
                require(output.size() + size <= 256_000) { "The capture queue index is too large" }
                output.write(buffer, 0, size)
            }
            output.toByteArray()
        }
        return DataInputStream(ByteArrayInputStream(bytes)).use { input ->
            require(input.readInt() == 1) { "The capture queue index could not be read" }
            val count = input.readInt(); require(count in 0..CAPACITY)
            val references = List(count) {
                val id = input.readUTF(); require(UUID.fromString(id).toString() == id)
                val kind = input.readUTF()
                val context = input.readUTF(); require(context.length <= 4096)
                val origin = when(kind) {
                    "today" -> View.Today; "morning" -> View.Morning; "evening" -> View.Tonight
                    "upcoming" -> View.Upcoming; "archive" -> View.Archive; "search" -> View.Search
                    "project" -> View.Project(context); "tag" -> View.Tag(context)
                    else -> error("The capture queue context could not be read")
                }
                Reference(id, origin)
            }
            require(input.available() == 0 && references.map { it.id }.distinct().size == count)
            references
        }
    }
    private fun writeIndex(references: List<Reference>) {
        val bytes = ByteArrayOutputStream().also { buffer ->
            DataOutputStream(buffer).use { out ->
                out.writeInt(1); out.writeInt(references.size)
                for(reference in references) {
                    val (kind, context) = when(val view = reference.origin) {
                        View.Today -> "today" to ""; View.Morning -> "morning" to ""; View.Tonight -> "evening" to ""
                        View.Upcoming -> "upcoming" to ""; View.Archive -> "archive" to ""; View.Search -> "search" to ""
                        is View.Project -> "project" to view.id; is View.Tag -> "tag" to view.id
                    }
                    require(context.length <= 4096)
                    out.writeUTF(reference.id); out.writeUTF(kind); out.writeUTF(context)
                }
            }
        }.toByteArray()
        check(index.baseFile.parentFile!!.exists() || index.baseFile.parentFile!!.mkdirs())
        val output = index.startWrite()
        try { output.write(bytes); index.finishWrite(output) }
        catch(e: Exception) { index.failWrite(output); throw e }
    }
    companion object {
        const val CAPACITY = 32
        private val locks = ConcurrentHashMap<String, Any>()
    }
}
