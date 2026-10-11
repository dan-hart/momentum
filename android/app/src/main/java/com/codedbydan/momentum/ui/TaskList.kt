// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import com.codedbydan.momentum.R
import androidx.compose.ui.platform.testTag

import androidx.compose.foundation.*
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.Alignment
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.*
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.codedbydan.momentum.*
import com.codedbydan.momentum.core.*

@Composable fun TaskList(state: UiState, vm: MomentumViewModel, modifier: Modifier = Modifier, menu: (TaskRow) -> Unit) {
    val snapshot = state.snapshot
    if(snapshot == null) {
        if(state.loading) { InitialTaskLoading(modifier); return }
        Box(modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            Button(onClick = vm::refresh) { Text(stringResource(R.string.retry)) }
        }; return
    }
    val listing = snapshot.listing
    LazyColumn(modifier.fillMaxSize().testTag("task-list").taskDropTarget(state.view,
        onTaskIds = { vm.drop(it, state.view); true }, onExternalText = { vm.importText(it, state.view); true },
        enabled = state.view != View.Archive), contentPadding = PaddingValues(bottom = 96.dp)) {
        if(state.selecting) item("drop-destinations") { TaskDestinations(state, vm) }
        else if(state.view != View.Archive && state.view != View.Search) item("quick-add") { QuickAdd(state, vm) }
        if (listing.empty != null) item("empty") {
            Column(Modifier.fillMaxWidth().padding(32.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                Text(emptyText(listing.empty), style = MaterialTheme.typography.headlineSmall)
                Spacer(Modifier.height(12.dp))
                Text(stringResource(if(state.view == View.Search) R.string.empty_search_hint else R.string.empty_hint),
                    style = MaterialTheme.typography.bodyLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
        listing.sections.forEachIndexed { index, section ->
            item("heading-$index") {
                sectionText(section)?.let {
                    val groupIds = section.rows.filterIsInstance<Row.Task>().filter { row -> !row.row.archived }.map { row -> row.row.id }
                    Row(Modifier.fillMaxWidth().padding(start = 20.dp, end = 8.dp), verticalAlignment = Alignment.CenterVertically) {
                        Text(stringResource(R.string.section_count, it, section.count.toInt()), Modifier.weight(1f).padding(vertical = 12.dp).semantics { heading() },
                            style = MaterialTheme.typography.titleSmall, color = nativeLabelInk(when(val group = section.group) {
                                is TaskGroup.Project -> group.color; is TaskGroup.Tag -> group.color; else -> null
                            }), maxLines = 2, overflow = TextOverflow.Ellipsis)
                        if(groupIds.isNotEmpty()) IconButton({ vm.selectGroup(groupIds) }) {
                            Icon(Icons.Default.CheckCircle, stringResource(R.string.select_shown_group, it))
                        }
                    }
                }
            }
            items(section.rows, key = { row -> when(row) { is Row.Task -> "t-${row.row.id}"; is Row.Project -> "p-${row.item.id}"; is Row.Tag -> "g-${row.item.id}" } }) { row ->
                when(row) {
                    is Row.Task -> TaskItem(row.row, state, vm, menu)
                    is Row.Project -> ListItem(headlineContent = { TaskContentText {
                        Text(row.item.title, color = nativeLabelInk(row.item.color), maxLines = 2, overflow = TextOverflow.Ellipsis)
                    } }, modifier = Modifier.clickable { vm.navigate(View.Project(row.item.id)) })
                    is Row.Tag -> ListItem(headlineContent = { TaskContentText {
                        Text("#${row.item.title}", color = nativeLabelInk(row.item.color), maxLines = 2, overflow = TextOverflow.Ellipsis)
                    } }, modifier = Modifier.clickable { vm.navigate(View.Tag(row.item.id)) })
                }
            }
            section.note?.let { note -> item("note-$index") {
                Text(stringResource(if(note.suggestNarrowing) R.string.search_limit else R.string.results_limit,
                    note.shown.toInt(), note.total.toInt()), Modifier.padding(20.dp), style = MaterialTheme.typography.bodyMedium)
            } }
        }
        if(listing.moreAvailable > 0u) item("more") {
            TextButton(vm::moreArchive, Modifier.fillMaxWidth()) { Text(stringResource(R.string.show_more)) }
        }
    }
}

@OptIn(ExperimentalFoundationApi::class)
@Composable private fun TaskItem(row: TaskRow, state: UiState, vm: MomentumViewModel, menu: (TaskRow) -> Unit) {
    val selected = row.id in state.selected
    val actionDescription = stringResource(if(row.isDone) R.string.reopen else R.string.complete)
    val openDescription = stringResource(R.string.open_task)
    val metadata = taskMetadata(row, state, selected)
    val compact = state.snapshot!!.settings.compact
    val notes = row.notesPreview.takeUnless { compact }
    var modifier = Modifier.fillMaxWidth().padding(start = if(row.isSubtask) 20.dp else 0.dp)
    if(!row.archived) modifier = modifier.combinedClickable(
        onClickLabel = if(state.selecting) stringResource(R.string.select_tasks) else openDescription,
        onClick = { if(state.selecting) vm.select(row.id) else vm.edit(row.id) },
        onLongClick = { if(!state.selecting || !selected) vm.select(row.id) },
    )
    val orderedSelection = state.snapshot?.listing?.sections.orEmpty().flatMap { it.rows }.filterIsInstance<Row.Task>()
        .map { it.row.id }.filter { it in state.selected }
    modifier = modifier.taskDragSource(orderedSelection, stringResource(R.string.select_tasks), enabled = state.selecting && selected && !row.archived)
        .taskDropTarget(state.view, onTaskIds = { vm.reorder(it, row.id, state.view); true },
            onExternalText = { vm.importText(it, state.view); true }, enabled = !row.archived && state.snapshot?.preferences?.sort == SortKey.MANUAL)
    if(state.selecting) modifier = modifier.semantics { this.selected = selected }
    val container = if(selected) MaterialTheme.colorScheme.secondaryContainer else MaterialTheme.colorScheme.surface
    val supportingInk = nativeLabelInk(null, selected)
    val headlineInk = if(selected) MaterialTheme.colorScheme.onSecondaryContainer else MaterialTheme.colorScheme.onSurface
    val headline: @Composable () -> Unit = {
        TaskContentText { Text(row.title, maxLines = if(compact) 2 else 3, overflow = TextOverflow.Ellipsis, color = headlineInk,
            textDecoration = if(row.isDone) TextDecoration.LineThrough else null) }
    }
    val supporting: (@Composable () -> Unit)? = if(metadata.isNotEmpty() || notes != null) {{
            TaskContentText(taskTypography().bodyMedium) {
                Column(verticalArrangement = Arrangement.spacedBy(if(compact) 1.dp else 3.dp)) {
                    if(metadata.isNotEmpty()) Text(metadata, maxLines = if(compact) 1 else 2, overflow = TextOverflow.Ellipsis)
                    notes?.let { Text(it, maxLines = 2, overflow = TextOverflow.Ellipsis, color = supportingInk) }
                }
            }
        }} else null
    val leading: (@Composable () -> Unit)? = if(!row.archived) {{
            Checkbox(checked = if(state.selecting) selected else row.isDone,
                onCheckedChange = if(state.selecting) null else {{ vm.action { toggleDone(row.id) } }},
                modifier = Modifier.sizeIn(minWidth = 48.dp, minHeight = 48.dp).semantics {
                    contentDescription = (if(state.selecting) "" else actionDescription + ": ") + row.title
                })
        }} else null
    val trailing: (@Composable () -> Unit)? = if(!row.archived && !state.selecting) {{
            IconButton(onClick = { menu(row) }, modifier = Modifier.sizeIn(minWidth = 48.dp, minHeight = 48.dp)) {
                Icon(Icons.Default.MoreVert, stringResource(R.string.task_actions, row.title))
            }
        }} else null
    if(compact) {
        Surface(modifier = modifier, color = container) {
            Row(Modifier.fillMaxWidth().heightIn(min = 48.dp).padding(horizontal = 16.dp, vertical = 2.dp),
                verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                leading?.invoke()
                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(1.dp)) {
                    headline()
                    supporting?.invoke()
                }
                trailing?.invoke()
            }
        }
    } else ListItem(modifier = modifier,
        colors = ListItemDefaults.colors(containerColor = container),
        headlineContent = headline, supportingContent = supporting, leadingContent = leading, trailingContent = trailing)
    HorizontalDivider(Modifier.padding(start = if(row.archived) 16.dp else 72.dp), color = MaterialTheme.colorScheme.outlineVariant)
}

private data class MetadataLabel(val text: String, val color: String? = null)

/** Styled spans retain the same full text for screen readers. */
@Composable private fun taskMetadata(row: TaskRow, state: UiState, selected: Boolean): AnnotatedString {
    val parts = buildList {
        row.project?.let { add(MetadataLabel(it.title, it.color)) }
        if(row.estimateMs > 0) add(MetadataLabel(stringResource(R.string.estimate_minutes, (row.estimateMs / 60_000).toInt())))
        row.day?.let { add(MetadataLabel(dayText(it))) }
        row.time?.let { add(MetadataLabel(clockText(it))) }
        if(row.repeat != null) add(MetadataLabel(stringResource(R.string.repeat)))
        if(row.reminder != null) add(MetadataLabel(stringResource(R.string.reminder)))
        addAll(row.tags.map { MetadataLabel("#${it.title}", it.color) })
        if(state.snapshot?.settings?.compact == true && row.notesPreview != null) add(MetadataLabel(stringResource(R.string.has_notes)))
        row.parentId?.let { id -> add(MetadataLabel(stringResource(R.string.subtask_of,
            state.snapshot?.parentTitles?.get(id) ?: stringResource(R.string.task_unavailable_short)))) }
    }
    val inks = parts.map { nativeLabelInk(it.color, selected) }
    val separatorInk = nativeLabelInk(null, selected)
    return buildAnnotatedString {
        parts.forEachIndexed { index, part ->
            if(index > 0) withStyle(SpanStyle(color = separatorInk)) { append(" · ") }
            withStyle(SpanStyle(color = inks[index])) { append(part.text) }
        }
    }
}

@Composable private fun TaskDestinations(state: UiState, vm: MomentumViewModel) {
    val contexts = state.snapshot?.sidebar?.let { it.fixed + it.projects + it.tags }.orEmpty()
        .filter { it.view == View.Today || it.view == View.Morning || it.view == View.Tonight || it.view is View.Project || it.view is View.Tag }
    BoxWithConstraints(Modifier.fillMaxWidth()) {
        // A horizontal lazy list measures children with unbounded width. Keep a
        // destination inside this native content viewport without changing its name.
        val maximumWidth = (maxWidth - 32.dp).coerceIn(48.dp, 240.dp)
        LazyRow(Modifier.fillMaxWidth().testTag("task-destinations"), contentPadding = PaddingValues(horizontal = 16.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            items(contexts, key = { it.view.toString() }) { entry ->
                var hover by remember { mutableStateOf(false) }
                val title = when(entry.view) { View.Today -> stringResource(R.string.today); View.Morning -> stringResource(R.string.morning);
                    View.Tonight -> stringResource(R.string.evening); else -> entry.title }
                FilterChip(selected = hover, onClick = { vm.drop(state.selected.toList(), entry.view) },
                    label = { Text(title, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                    modifier = Modifier.widthIn(min = 48.dp, max = maximumWidth).heightIn(min = 48.dp).taskDropTarget(entry.view,
                        onTaskIds = { vm.drop(it, entry.view); true }, onExternalText = { vm.importText(it, entry.view); true }, onHoverChanged = { hover = it }))
            }
        }
    }
}
