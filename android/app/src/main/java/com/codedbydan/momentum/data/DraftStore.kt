// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.data

import android.util.AtomicFile
import java.io.File
import java.util.UUID

/** Full private drafts stay on disk; saved Android state contains only a small ID. */
class DraftStore(private val directory: File) {
    private fun file(id: String): AtomicFile {
        require(UUID.fromString(id).toString() == id)
        return AtomicFile(File(directory, "android-drafts/$id.draft"))
    }
    fun read(id: String): EditorDraft? {
        val file = file(id)
        if(!file.baseFile.exists() && !File(file.baseFile.path + ".bak").exists()) return null
        return file.openRead().use {
            requireNotNull(EditorDraft.readEncodedFrom(it)) { "The existing task draft could not be read" }
        }
    }
    fun write(id: String, draft: EditorDraft) {
        // Validate before opening an atomic write so rejected input never touches the saved file.
        require(DraftStorageBudget().admit(draft)) { "The task draft cannot be saved" }
        val file = file(id)
        check(file.baseFile.parentFile!!.exists() || file.baseFile.parentFile!!.mkdirs())
        val output = file.startWrite()
        try { draft.writeEncodedTo(output); file.finishWrite(output) }
        catch(e: Throwable) { file.failWrite(output); throw e }
    }
    fun delete(id: String) { file(id).delete() }
}
