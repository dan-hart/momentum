// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.platform

import android.content.ActivityNotFoundException
import android.content.ClipData
import android.content.ClipDescription
import android.content.Context
import android.content.Intent
import android.net.Uri
import androidx.core.content.FileProvider
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import java.io.File
import java.io.IOException
import java.util.UUID

/** Explicit full-text sharing uses a distinct cache, never clipboard payload cleanup. */
internal object PrivateNotesShare {
    private const val DIRECTORY = "shared-notes"
    private const val MAX_FILES = 32
    private const val MAX_BYTES = 4_000_000
    private val filename = Regex("[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\\.txt")
    private val mutex = Mutex()

    fun owns(context: Context, uri: Uri): Boolean = uri.scheme == "content" &&
        uri.authority == "${context.packageName}.clipboard" && uri.query == null && uri.fragment == null &&
        uri.pathSegments.size == 2 && uri.pathSegments[0] == "notes" && filename.matches(uri.pathSegments[1])

    suspend fun prepare(context: Context, label: String, text: String): Intent = mutex.withLock {
        val uri = withContext(Dispatchers.IO) {
            val bytes = text.toByteArray(Charsets.UTF_8)
            require(bytes.size <= MAX_BYTES)
            val directory = File(context.cacheDir, DIRECTORY)
            check(directory.exists() || directory.mkdirs())
            if(directory.listFiles().orEmpty().size >= MAX_FILES) throw NotesShareCapacityException()
            val file = File(directory, "${UUID.randomUUID()}.txt")
            try {
                file.writeBytes(bytes)
                FileProvider.getUriForFile(context, "${context.packageName}.clipboard", file)
            } catch(error: Exception) { file.delete(); throw error }
        }
        Intent(Intent.ACTION_SEND).setType(ClipDescription.MIMETYPE_TEXT_PLAIN)
            .putExtra(Intent.EXTRA_STREAM, uri).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            .apply { clipData = ClipData(label, arrayOf(ClipDescription.MIMETYPE_TEXT_PLAIN), ClipData.Item(uri)).markSensitive() }
    }

    suspend fun send(context: Context, label: String, text: String) {
        val intent = prepare(context, label, text)
        try {
            withContext(Dispatchers.Main.immediate) {
                context.startActivity(Intent.createChooser(intent, label).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
            }
        } catch(error: ActivityNotFoundException) { discardUnsent(context, intent); throw error }
        catch(error: SecurityException) { discardUnsent(context, intent); throw error }
        // A dismissed chooser has no dependable grant-lifetime callback. Retain these bounded
        // files rather than deleting a recipient's active stream; Android Settings can clear cache.
    }

    private suspend fun discardUnsent(context: Context, intent: Intent) = withContext(Dispatchers.IO) {
        val uri = intent.clipData?.getItemAt(0)?.uri ?: return@withContext
        if(owns(context, uri)) File(File(context.cacheDir, DIRECTORY), uri.lastPathSegment!!).delete()
    }
}

internal class NotesShareCapacityException : IOException()
