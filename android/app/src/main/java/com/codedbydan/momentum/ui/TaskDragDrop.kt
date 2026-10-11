// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import android.content.ClipData
import androidx.compose.foundation.draganddrop.dragAndDropSource
import androidx.compose.foundation.draganddrop.dragAndDropTarget
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Modifier
import androidx.compose.ui.draganddrop.DragAndDropEvent
import androidx.compose.ui.draganddrop.DragAndDropTarget
import androidx.compose.ui.draganddrop.DragAndDropTransferData
import androidx.compose.ui.draganddrop.mimeTypes
import androidx.compose.ui.draganddrop.toAndroidDragEvent
import com.codedbydan.momentum.core.View
import java.net.URI

/**
 * Native, application-local drag of selected task IDs in their supplied order.
 * Apply only to selected rows while selection mode is active. [dragLabel] should
 * be a localized generic label (no title/notes). No global drag flag is granted.
 * Rust must validate task existence again in the receiving callback.
 */
fun Modifier.taskDragSource(
    selectedIds: List<String>,
    dragLabel: String,
    enabled: Boolean = true,
): Modifier = if (!enabled) this else dragAndDropSource(transferData = { _ ->
    taskTransferData(selectedIds, dragLabel)
})

/**
 * Native drop target for trusted local task IDs or external plain text/HTTP(S) URLs.
 * External data is never read through ContentResolver, coerced from an Intent, or
 * fetched from a URI; no transient URI permission is needed for these value types.
 * The stable remembered target uses the latest destination/callbacks throughout
 * a drag. Archive refuses external content, including a destination changed mid-drag.
 *
 * Callbacks return whether they accepted the transfer, and should enqueue their
 * Rust work off the main thread. For IDs, validate `isTaskIdList(ids.joinToString("\n"))`
 * then call `dropTasks` for a destination or `reorderTasks` for a row. Forward text
 * directly to `addFromText`; task parsing and mutations remain in Rust.
 * See https://developer.android.com/develop/ui/compose/touch-input/user-interactions/drag-and-drop
 */
@Composable
fun Modifier.taskDropTarget(
    destination: View,
    onTaskIds: (List<String>) -> Boolean,
    onExternalText: (String) -> Boolean,
    enabled: Boolean = true,
    onHoverChanged: (Boolean) -> Unit = {},
): Modifier {
    val current by rememberUpdatedState(TaskDropOptions(destination, enabled, onTaskIds, onExternalText, onHoverChanged))
    val target = remember {
        object : DragAndDropTarget {
            override fun onEntered(event: DragAndDropEvent) {
                if (acceptsEvent(event, current)) current.onHoverChanged(true)
            }

            override fun onExited(event: DragAndDropEvent) = current.onHoverChanged(false)
            override fun onEnded(event: DragAndDropEvent) = current.onHoverChanged(false)

            override fun onDrop(event: DragAndDropEvent): Boolean {
                current.onHoverChanged(false)
                if (!acceptsEvent(event, current)) return false
                val androidEvent = event.toAndroidDragEvent()
                val clip = androidEvent.clipData ?: return false
                if (TaskDragTransport.PRIVATE_MIME in event.mimeTypes()) {
                    val local = androidEvent.localState as? LocalTaskDrag ?: return false
                    if (clip.itemCount != 1) return false
                    val ids = TaskDragTransport.decodeIds(clip.getItemAt(0).text) ?: return false
                    if (ids != local.ids) return false
                    return current.onTaskIds(ids)
                }
                if (clip.itemCount !in 1..TaskDragTransport.MAX_EXTERNAL_ITEMS) return false
                val items = (0 until clip.itemCount).map { index ->
                    val item = clip.getItemAt(index)
                    ExternalTaskDropItem(text = item.text, uri = item.uri?.toString())
                }
                val text = TaskDragTransport.externalText(items) ?: return false
                return current.onExternalText(text)
            }
        }
    }
    return dragAndDropTarget(shouldStartDragAndDrop = { acceptsEvent(it, current) }, target = target)
}

private data class TaskDropOptions(
    val destination: View,
    val enabled: Boolean,
    val onTaskIds: (List<String>) -> Boolean,
    val onExternalText: (String) -> Boolean,
    val onHoverChanged: (Boolean) -> Unit,
)

// localState is an in-process object, not transferred between applications.
private class LocalTaskDrag(val ids: List<String>)

internal fun taskTransferData(selectedIds: List<String>, dragLabel: String): DragAndDropTransferData? {
    val ids = TaskDragTransport.validIds(selectedIds) ?: return null
    return DragAndDropTransferData(
        clipData = ClipData(dragLabel, arrayOf(TaskDragTransport.PRIVATE_MIME), ClipData.Item(ids.joinToString("\n"))),
        localState = LocalTaskDrag(ids),
        flags = 0,
    )
}

private fun acceptsEvent(event: DragAndDropEvent, options: TaskDropOptions): Boolean = options.enabled &&
    TaskDragTransport.accepts(
        mimeTypes = event.mimeTypes(),
        trustedLocal = (event.toAndroidDragEvent().localState as? LocalTaskDrag)?.ids
            ?.let { TaskDragTransport.validIds(it) != null } == true,
        allowExternal = options.destination != View.Archive,
    )

internal data class ExternalTaskDropItem(val text: CharSequence? = null, val uri: String? = null)

/** Transfer validation only; IDs, ordering, task parsing and mutations belong to Rust. */
internal object TaskDragTransport {
    const val PRIVATE_MIME = "application/vnd.com.codedbydan.momentum.task-ids"
    const val MAX_EXTERNAL_ITEMS = 32
    private const val MAX_IDS = 1000
    private const val MAX_ID_LENGTH = 128
    private const val MAX_EXTERNAL_BYTES = 65536
    private val identifier = Regex("[A-Za-z0-9_-]+")

    fun validIds(ids: List<String>): List<String>? {
        if (ids.size !in 1..MAX_IDS || ids.any { it.length !in 1..MAX_ID_LENGTH || !identifier.matches(it) }) return null
        if (ids.toSet().size != ids.size) return null
        return ids.toList()
    }

    fun decodeIds(encoded: CharSequence?): List<String>? {
        if (encoded == null || encoded.length !in 1..(MAX_IDS * (MAX_ID_LENGTH + 1))) return null
        return validIds(encoded.toString().split('\n'))
    }

    fun accepts(mimeTypes: Set<String>, trustedLocal: Boolean, allowExternal: Boolean): Boolean =
        if (PRIVATE_MIME in mimeTypes) trustedLocal
        else allowExternal && mimeTypes.any { it == "text/plain" || it == "text/uri-list" }

    fun externalText(items: List<ExternalTaskDropItem>): String? {
        if (items.size !in 1..MAX_EXTERNAL_ITEMS) return null
        val parts = ArrayList<String>(items.size)
        var bytes = 0
        for (item in items) {
            val raw = item.text ?: item.uri?.takeIf { isHttpUrl(it) } ?: return null
            if (raw.length > MAX_EXTERNAL_BYTES) return null
            val text = raw.toString()
            bytes += text.toByteArray(Charsets.UTF_8).size + if (parts.isEmpty()) 0 else 1
            if (bytes > MAX_EXTERNAL_BYTES) return null
            parts += text
        }
        return parts.joinToString("\n").takeUnless { it.isBlank() }
    }

    private fun isHttpUrl(value: String): Boolean {
        if (value.length > MAX_EXTERNAL_BYTES) return false
        val uri = runCatching { URI(value) }.getOrNull() ?: return false
        return uri.scheme?.lowercase() in setOf("http", "https") && !uri.host.isNullOrEmpty()
    }
}
