// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.selection.toggleable
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Checkbox
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.unit.dp
import com.codedbydan.momentum.MomentumViewModel
import com.codedbydan.momentum.R
import com.codedbydan.momentum.core.InboxCleanupChange
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch

private data class CleanupPreview(
    val changes: List<InboxCleanupChange>,
    val tagTitles: Map<String, String>,
)

private fun InboxCleanupChange.hasChanges() =
    title != originalTitle || links.isNotEmpty() || tagIds.isNotEmpty()

@Composable
internal fun InboxCleanupAction(vm: MomentumViewModel, enabled: Boolean) {
    var preview by remember { mutableStateOf<CleanupPreview?>(null) }
    var preparing by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    Button(
        onClick = {
            if (preparing) return@Button
            preparing = true
            scope.launch {
                try {
                    preview = vm.repository.read {
                        val changes = previewInboxCleanup().filter { it.hasChanges() }
                        CleanupPreview(changes, tags().associate { it.id to it.title })
                    }
                } catch (error: CancellationException) {
                    throw error
                } catch (error: Exception) {
                    vm.reportError(error)
                } finally {
                    preparing = false
                }
            }
        },
        enabled = enabled && !preparing,
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp).heightIn(min = 48.dp),
    ) {
        Icon(Icons.Default.Edit, contentDescription = null, modifier = Modifier.size(ButtonDefaults.IconSize))
        Spacer(Modifier.width(ButtonDefaults.IconSpacing))
        Text(stringResource(if (preparing) R.string.inbox_cleanup_preparing else R.string.inbox_cleanup))
    }
    preview?.let { pending ->
        CleanupPreviewDialog(
            preview = pending,
            enabled = enabled,
            dismiss = { preview = null },
            apply = { selected ->
                if (preview === pending) {
                    preview = null
                    vm.action { applyInboxCleanup(selected) }
                }
            },
        )
    }
}

@Composable
private fun CleanupPreviewDialog(
    preview: CleanupPreview,
    enabled: Boolean,
    dismiss: () -> Unit,
    apply: (List<InboxCleanupChange>) -> Unit,
) {
    var excludedTasks by remember(preview) { mutableStateOf(emptySet<String>()) }
    var excludedTags by remember(preview) { mutableStateOf(emptySet<Pair<String, String>>()) }
    val selectedChanges = preview.changes.filter { it.taskId !in excludedTasks }.map { change ->
        change.copy(tagIds = change.tagIds.filter { tag -> (change.taskId to tag) !in excludedTags })
    }.filter { it.hasChanges() }
    AlertDialog(
        onDismissRequest = dismiss,
        title = { Text(stringResource(R.string.inbox_cleanup)) },
        text = {
            if (preview.changes.isEmpty()) {
                Text(stringResource(R.string.inbox_cleanup_empty))
            } else {
                LazyColumn(
                    modifier = Modifier.fillMaxWidth().heightIn(max = 440.dp),
                    contentPadding = PaddingValues(bottom = 8.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    item {
                        Text(stringResource(R.string.inbox_cleanup_review), style = MaterialTheme.typography.bodyMedium)
                    }
                    items(preview.changes, key = { it.taskId }) { change ->
                        val included = change.taskId !in excludedTasks
                        HorizontalDivider()
                        CleanupToggle(included, enabled = enabled, toggle = {
                            excludedTasks = if (it) excludedTasks - change.taskId else excludedTasks + change.taskId
                        }) {
                            Column(Modifier.weight(1f)) {
                                if (change.title != change.originalTitle) {
                                    Text(
                                        change.originalTitle,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        textDecoration = TextDecoration.LineThrough,
                                    )
                                }
                                Text(change.title, style = MaterialTheme.typography.titleSmall)
                                if (change.links.isNotEmpty()) {
                                    Text(pluralStringResource(R.plurals.inbox_cleanup_links, change.links.size, change.links.size))
                                }
                            }
                        }
                        change.tagIds.forEach { tagId ->
                            val tagTitle = preview.tagTitles[tagId] ?: return@forEach
                            val key = change.taskId to tagId
                            CleanupToggle(key !in excludedTags, enabled = enabled && included, toggle = {
                                excludedTags = if (it) excludedTags - key else excludedTags + key
                            }, modifier = Modifier.padding(start = 28.dp)) {
                                Text(stringResource(R.string.inbox_cleanup_add_tag, tagTitle))
                            }
                        }
                    }
                }
            }
        },
        confirmButton = {
            if (preview.changes.isEmpty()) {
                TextButton(dismiss) { Text(stringResource(R.string.close)) }
            } else {
                TextButton(
                    onClick = { apply(selectedChanges) },
                    enabled = enabled && selectedChanges.isNotEmpty(),
                ) { Text(stringResource(R.string.inbox_cleanup_apply)) }
            }
        },
        dismissButton = {
            if (preview.changes.isNotEmpty()) {
                TextButton(dismiss) { Text(stringResource(R.string.cancel)) }
            }
        },
    )
}

@Composable
private fun CleanupToggle(
    checked: Boolean,
    enabled: Boolean,
    toggle: (Boolean) -> Unit,
    modifier: Modifier = Modifier,
    label: @Composable RowScope.() -> Unit,
) {
    Row(
        modifier.fillMaxWidth().heightIn(min = 48.dp).toggleable(checked, enabled, Role.Checkbox, onValueChange = toggle),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Checkbox(checked = checked, onCheckedChange = null, enabled = enabled)
        label()
    }
}
