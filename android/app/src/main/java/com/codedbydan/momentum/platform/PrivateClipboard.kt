// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.platform

import android.content.ClipData
import android.content.ClipDescription
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.ParcelFileDescriptor
import android.os.Build
import androidx.core.content.FileProvider
import com.codedbydan.momentum.R
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import java.io.File
import java.io.FileNotFoundException
import java.io.IOException
import java.util.UUID

/** Only explicit user copies publish private text. Full large payloads travel through a stream. */
internal object PrivateClipboard {
    private const val MAX_BYTES = 4_000_000
    private const val MAX_FILES = 32
    private const val INLINE_CHARS = 64 * 1024
    private const val DIRECTORY = "clipboard-payloads"
    private val filename = Regex("[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\\.txt")
    private val mutex = Mutex()
    private fun authority(context: Context) = "${context.packageName}.clipboard"

    fun owns(context: Context, uri: Uri): Boolean = uri.scheme == "content" &&
        uri.authority == authority(context) && uri.query == null && uri.fragment == null &&
        uri.pathSegments.size == 2 && uri.pathSegments[0] == "text" && filename.matches(uri.pathSegments[1])

    suspend fun copy(context: Context, label: String, text: String) = mutex.withLock {
        // Android 8's clipboard service asks for URI grants as system UID; its activity manager
        // refuses those grants. Reject before touching the current clip or its private files.
        if(Build.VERSION.SDK_INT <= 27 && text.length > INLINE_CHARS) throw ClipboardUriUnsupportedException()
        val bytes = withContext(Dispatchers.IO) {
            text.toByteArray(Charsets.UTF_8).also { require(it.size <= MAX_BYTES) }
        }
        val directory = File(context.filesDir, DIRECTORY)
        val file = if(text.length <= INLINE_CHARS) null else withContext(Dispatchers.IO) {
            check(directory.exists() || directory.mkdirs())
            // Never evict a potentially active clip merely because background reads are unavailable.
            check(directory.listFiles().orEmpty().size < MAX_FILES)
            File(directory, "${UUID.randomUUID()}.txt").also { it.writeBytes(bytes) }
        }
        val uri = file?.let { FileProvider.getUriForFile(context, authority(context), it) }
        val clip = if(uri == null) sensitivePlainText(label, text)
            else ClipData(label, arrayOf(ClipDescription.MIMETYPE_TEXT_PLAIN), ClipData.Item(uri)).markSensitive()
        withContext(Dispatchers.Main.immediate) {
            // API 26's service constructor creates a Handler on the calling thread.
            val manager = context.getSystemService(ClipboardManager::class.java)
            manager.setPrimaryClip(clip)
            val current = manager.primaryClip
            // A system policy/focus rejection must not remove the previous clip's backing file.
            check(current?.itemCount == 1 && if(uri == null) current.getItemAt(0).text?.toString() == text
                else current.getItemAt(0).uri == uri)
        }
        // Successful explicit replacement expires old payloads/grants. Process-death orphans stay
        // bounded until a verified replacement; no automatic/background clipboard inspection occurs.
        withContext(Dispatchers.IO) {
            directory.listFiles().orEmpty().filter { it != file && it.isFile && filename.matches(it.name) }.forEach { previous ->
                val previousUri = FileProvider.getUriForFile(context, authority(context), previous)
                context.revokeUriPermission(previousUri, Intent.FLAG_GRANT_READ_URI_PERMISSION)
                previous.delete()
            }
        }
    }

    /** Explicit paste only. Never coerce an arbitrary provider or resolve a remote URI. */
    suspend fun readOwned(context: Context, uri: Uri, maxChars: Int): String = withContext(Dispatchers.IO) {
        require(owns(context, uri) && maxChars >= 0)
        context.contentResolver.openInputStream(uri)?.bufferedReader(Charsets.UTF_8)?.use { input ->
            val result = StringBuilder(minOf(maxChars, 8192))
            val buffer = CharArray(8192)
            while(true) {
                val count = input.read(buffer)
                if(count < 0) break
                if(count > maxChars - result.length) throw ClipboardTextTooLargeException()
                result.append(buffer, 0, count)
            }
            result.toString()
        } ?: throw FileNotFoundException()
    }
}

internal class ClipboardTextTooLargeException : IOException()
internal class ClipboardUriUnsupportedException : IOException()

/** A URI read grant never permits changing the file or exposing the rest of private storage. */
class ClipboardFileProvider : FileProvider(R.xml.clipboard_paths) {
    override fun openFile(uri: Uri, mode: String): ParcelFileDescriptor {
        val app = requireNotNull(context)
        if(mode != "r" || !(PrivateClipboard.owns(app, uri) || PrivateNotesShare.owns(app, uri))) throw FileNotFoundException()
        return super.openFile(uri, mode) ?: throw FileNotFoundException()
    }
    override fun delete(uri: Uri, selection: String?, selectionArgs: Array<out String>?): Int =
        throw UnsupportedOperationException()
}
