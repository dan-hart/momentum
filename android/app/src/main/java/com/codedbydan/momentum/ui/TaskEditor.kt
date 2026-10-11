// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import com.codedbydan.momentum.R

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Check
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.semantics.*
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import androidx.compose.ui.input.key.*
import com.codedbydan.momentum.*
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.EditorDraft
import com.codedbydan.momentum.platform.PrivateClipboard
import com.codedbydan.momentum.platform.ClipboardUriUnsupportedException
import com.codedbydan.momentum.platform.PrivateNotesShare
import com.codedbydan.momentum.platform.NotesShareCapacityException
import java.time.*
import java.util.Locale
import kotlinx.coroutines.launch
import kotlinx.coroutines.CancellationException

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable fun TaskEditor(state: UiState, vm: MomentumViewModel) {
    val draft = state.editor ?: return
    if(draft.isTextImport) { TextImportEditor(state, vm); return }
    var discard by remember { mutableStateOf(false) }
    var calendar by remember { mutableStateOf(false) }
    var subtask by remember { mutableStateOf(false) }
    var repeat by remember { mutableStateOf(false) }
    var projectMenu by remember { mutableStateOf(false) }
    var reminderMenu by remember { mutableStateOf(false) }
    var pickerTime by remember { mutableStateOf<ClockTime?>(null) }
    var preparingTime by remember { mutableStateOf(false) }
    var copyingNotes by remember { mutableStateOf(false) }
    var copyNotesFailure by remember { mutableStateOf<Int?>(null) }
    var sharingNotes by remember { mutableStateOf(false) }
    var shareNotesFailure by remember { mutableStateOf<Int?>(null) }
    var detail by remember { mutableStateOf<TaskDetail?>(null) }
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val clipboardNotes = stringResource(R.string.notes)
    val subtaskLabel = stringResource(R.string.subtask)
    val containerWidth = LocalWindowInfo.current.containerSize.width
    val expanded = with(LocalDensity.current) { containerWidth.toDp() >= 600.dp }
    val focus = remember { FocusRequester() }
    LaunchedEffect(draft.id, state.snapshot) {
        try { draft.id?.let { detail = vm.repository.detail(it) } }
        catch(e: Exception) { vm.reportError(e) }
    }
    fun change(value: EditorDraft) { if(!state.busy) vm.setEditor(value) }
    fun close() { if(!state.busy) { if(draft.hasContent) discard = true else vm.dismissEditor() } }
    Dialog(onDismissRequest = ::close, properties = DialogProperties(usePlatformDefaultWidth = expanded, decorFitsSystemWindows = false)) {
        Surface(Modifier.fillMaxSize()) {
            Scaffold(Modifier.windowInsetsPadding(WindowInsets.safeDrawing).imePadding().editorKeyboardShortcuts(vm,
                canSave = draft.problem() == null && !state.editorUnavailable,
                blocked = discard || calendar || subtask || repeat || projectMenu || reminderMenu || preparingTime || pickerTime != null,
                save = vm::saveEditor, cancel = ::close),
                topBar = { TopAppBar(title = { AdaptiveTopAppBarTitle(stringResource(if(draft.id == null) R.string.add_task else R.string.edit_task)) },
                    expandedHeight = adaptiveTopAppBarHeight(),
                    navigationIcon = { IconButton(onClick = ::close, enabled = !state.busy,
                        modifier = Modifier.sizeIn(minWidth = 48.dp, minHeight = 48.dp)) { Icon(Icons.Default.Close, stringResource(R.string.cancel)) } },
                    actions = { IconButton(onClick = vm::saveEditor, enabled = draft.problem() == null && !state.busy,
                        modifier = Modifier.sizeIn(minWidth = 48.dp, minHeight = 48.dp)) { Icon(Icons.Default.Check, stringResource(R.string.save)) } }) },
            ) { padding ->
                Column(Modifier.padding(padding).verticalScroll(rememberScrollState()).padding(16.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
                    CaptureStatus(state)
                    DraftRecoveryStatus(state, vm)
                    if(state.editorCapacityError) Text(stringResource(R.string.draft_edit_capacity), color = MaterialTheme.colorScheme.error,
                        modifier = Modifier.semantics { liveRegion = LiveRegionMode.Assertive })
                    if(state.editorError) Text(stringResource(R.string.invalid_editor), color = MaterialTheme.colorScheme.error,
                        modifier = Modifier.semantics { liveRegion = LiveRegionMode.Assertive })
                    if(state.editorUnavailable) { Text(stringResource(R.string.task_unavailable), color = MaterialTheme.colorScheme.error); OutlinedButton(vm::saveAsNew, enabled = !state.busy) { Text(stringResource(R.string.copy_draft_new)) } }
                    else if(state.error != null) Text(stringResource(R.string.save_failed), color = MaterialTheme.colorScheme.error,
                        modifier = Modifier.semantics { liveRegion = LiveRegionMode.Assertive })
                    OutlinedTextField(enabled = !state.busy, value = draft.title, onValueChange = { change(draft.copy(title = it)) }, label = { Text(stringResource(R.string.title)) },
                        modifier = Modifier.fillMaxWidth().focusRequester(focus), singleLine = true, textStyle = taskTypography().bodyLarge,
                        isError = state.editorError && draft.title.isBlank())
                    ExposedDropdownMenuBox(expanded = projectMenu, onExpandedChange = { projectMenu = it }) {
                        val projects = state.snapshot?.projects.orEmpty()
                        OutlinedTextField(projects.firstOrNull { it.id == draft.projectId }?.title ?: stringResource(R.string.inbox), {},
                            readOnly = true, singleLine = true, enabled = !state.busy, label = { Text(stringResource(R.string.project)) },
                            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(projectMenu) },
                            modifier = Modifier.menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable).fillMaxWidth())
                        ExposedDropdownMenu(projectMenu, { projectMenu = false }) {
                            projects.forEach { project -> DropdownMenuItem(text = { Text(project.title, maxLines = 2, overflow = TextOverflow.Ellipsis) }, onClick = {
                                change(draft.copy(projectId = project.id)); projectMenu = false
                            }) }
                        }
                    }
                    OutlinedTextField(enabled = !state.busy, value = draft.dueDay, onValueChange = { change(draft.copy(dueDay = it)) },
                        label = { Text(stringResource(R.string.date)) }, placeholder = { Text(stringResource(R.string.date_hint)) },
                        singleLine = true, modifier = Modifier.fillMaxWidth(),
                        trailingIcon = { TextButton({ calendar = true }) { Text(stringResource(R.string.date)) } },
                        isError = state.editorError && draft.problem() == EditorDraft.Problem.Date)
                    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        listOf(R.string.today to LocalDate.now().toString(), R.string.tomorrow to LocalDate.now().plusDays(1).toString(), R.string.none to "").forEach { (label, day) ->
                            AssistChip(enabled = !state.busy, onClick = { change(draft.copy(dueDay = day)) }, label = { Text(stringResource(label)) })
                        }
                    }
                    OutlinedTextField(enabled = !state.busy, value = draft.time, onValueChange = { change(draft.copy(time = it, reminder = if(it.isBlank()) null else draft.reminder)) },
                        label = { Text(stringResource(R.string.time)) }, placeholder = { Text(stringResource(R.string.time_hint)) },
                        singleLine = true, modifier = Modifier.fillMaxWidth(), isError = state.editorError && draft.time.isNotBlank(),
                        trailingIcon = { IconButton(enabled = !state.busy && !preparingTime, onClick = {
                            preparingTime = true
                            scope.launch {
                                try {
                                    val parsed = vm.repository.read { parseTime(draft.time) }
                                    if(vm.state.value.editor == draft && !vm.state.value.busy) {
                                        val now = LocalTime.now()
                                        pickerTime = parsed ?: ClockTime(now.hour.toUInt(), now.minute.toUInt())
                                    }
                                } catch(e: Exception) { if(e is CancellationException) throw e; vm.reportError(e) }
                                finally { preparingTime = false }
                            }
                        }) { Icon(painterResource(R.drawable.ic_time), stringResource(R.string.choose_time)) } })
                    ExposedDropdownMenuBox(reminderMenu, { if(draft.time.isNotBlank()) reminderMenu = it }) {
                        OutlinedTextField(reminderText(draft.reminder), {}, readOnly = true,
                            label = { Text(stringResource(R.string.reminder)) }, enabled = draft.time.isNotBlank() && !state.busy,
                            modifier = Modifier.menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable).fillMaxWidth(),
                            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(reminderMenu) })
                        ExposedDropdownMenu(reminderMenu, { reminderMenu = false }) {
                            listOf(null, 0L, 5L, 10L, 15L, 30L, 60L, 1440L).forEach { value ->
                                DropdownMenuItem(text = { Text(reminderText(value)) }, onClick = { change(draft.copy(reminder = value)); reminderMenu = false })
                            }
                        }
                    }
                    OutlinedTextField(enabled = !state.busy, value = draft.estimate, onValueChange = { change(draft.copy(estimate = it)) }, label = { Text(stringResource(R.string.estimate)) },
                        placeholder = { Text(stringResource(R.string.estimate_hint)) }, singleLine = true, modifier = Modifier.fillMaxWidth())
                    Text(stringResource(R.string.tags), style = MaterialTheme.typography.titleSmall, modifier = Modifier.semantics { heading() })
                    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        state.snapshot?.tags.orEmpty().forEach { tag ->
                            FilterChip(enabled = !state.busy, selected = tag.id in draft.tagIds, onClick = { change(draft.copy(tagIds = if(tag.id in draft.tagIds) draft.tagIds - tag.id else draft.tagIds + tag.id)) }, label = { Text(tag.title, maxLines = 1, overflow = TextOverflow.Ellipsis) })
                        }
                    }
                    OutlinedTextField(enabled = !state.busy, value = draft.newTags, onValueChange = { change(draft.copy(newTags = it)) }, label = { Text(stringResource(R.string.new_tags)) }, modifier = Modifier.fillMaxWidth())
                    BoundedSourceField(source = draft.notes, onSourceChange = {
                        if(state.busy || vm.state.value.busy) false else {
                            val proposed = draft.copy(notes = it)
                            change(proposed)
                            vm.state.value.editor == proposed
                        }
                    }, label = stringResource(R.string.notes), enabled = !state.busy, modifier = Modifier.fillMaxWidth())
                    TextButton(onClick = {
                        copyingNotes = true; copyNotesFailure = null
                        scope.launch {
                            try { PrivateClipboard.copy(context, clipboardNotes, draft.notes) }
                            catch(e: CancellationException) { throw e }
                            catch(_: ClipboardUriUnsupportedException) { copyNotesFailure = R.string.clipboard_large_unsupported }
                            catch(_: Exception) { copyNotesFailure = R.string.clipboard_copy_failed }
                            finally { copyingNotes = false }
                        }
                    }, enabled = draft.notes.isNotEmpty() && !copyingNotes && !state.busy) { Text(stringResource(R.string.copy_notes)) }
                    copyNotesFailure?.let { Text(stringResource(it), color = MaterialTheme.colorScheme.error,
                        modifier = Modifier.semantics { liveRegion = LiveRegionMode.Assertive }) }
                    TextButton(onClick = {
                        sharingNotes = true; shareNotesFailure = null
                        scope.launch {
                            try { PrivateNotesShare.send(context, clipboardNotes, draft.notes) }
                            catch(e: CancellationException) { throw e }
                            catch(_: NotesShareCapacityException) { shareNotesFailure = R.string.share_notes_capacity }
                            catch(_: Exception) { shareNotesFailure = R.string.share_notes_failed }
                            finally { sharingNotes = false }
                        }
                    }, enabled = draft.notes.isNotEmpty() && !sharingNotes && !state.busy) { Text(stringResource(R.string.share_notes)) }
                    shareNotesFailure?.let { Text(stringResource(it), color = MaterialTheme.colorScheme.error,
                        modifier = Modifier.semantics { liveRegion = LiveRegionMode.Assertive })
                    }
                    if(draft.id != null) {
                        if(detail?.parentId == null) {
                            detail?.subTasks.orEmpty().forEach { child ->
                                ListItem(headlineContent = { TaskContentText { Text(child.title, maxLines = 2, overflow = TextOverflow.Ellipsis) } }, leadingContent = {
                                    Checkbox(child.isDone, { vm.action { toggleDone(child.id) } },
                                        Modifier.semantics { contentDescription = subtaskLabel + ": " + child.title })
                                })
                            }
                            OutlinedButton({ subtask = true }, Modifier.fillMaxWidth()) { Text(stringResource(R.string.subtask)) }
                            OutlinedButton({ vm.saveEditorThen { repeat = true } }, Modifier.fillMaxWidth(), enabled = !state.busy && draft.problem() == null) { Text(stringResource(R.string.repeat)) }
                        }
                        Text(stringResource(R.string.save_before_action), style = MaterialTheme.typography.bodySmall)
                        OutlinedButton({ vm.saveEditorThen { id -> vm.action { duplicateTask(id) } } }, Modifier.fillMaxWidth(), enabled = !state.busy && draft.problem() == null) { Text(stringResource(R.string.duplicate)) }
                    }
                    Spacer(Modifier.height(16.dp))
                }
            }
        }
        LaunchedEffect(Unit) { if(draft.id == null) focus.requestFocus() }
    }
    if(discard) AlertDialog(onDismissRequest = { if(!state.busy) discard = false }, title = { Text(stringResource(R.string.discard_title)) },
        text = { Text(stringResource(R.string.discard_hint)) },
        confirmButton = { TextButton({ if(!state.busy) { vm.dismissEditor(); discard = false } }, enabled = !state.busy) { Text(stringResource(R.string.discard)) } },
        dismissButton = { TextButton({ discard = false }) { Text(stringResource(R.string.cancel)) } })
    if(calendar) {
        val selected = runCatching { LocalDate.parse(draft.dueDay).atStartOfDay(ZoneOffset.UTC).toInstant().toEpochMilli() }.getOrNull()
        val picker = rememberDatePickerState(initialSelectedDateMillis = selected)
        DatePickerDialog(onDismissRequest = { calendar = false },
            confirmButton = { TextButton({ picker.selectedDateMillis?.let { change(draft.copy(dueDay = Instant.ofEpochMilli(it).atZone(ZoneOffset.UTC).toLocalDate().toString())) }; calendar = false }) { Text(stringResource(R.string.save)) } },
            dismissButton = { TextButton({ calendar = false }) { Text(stringResource(R.string.cancel)) } }) { DatePicker(picker) }
    }
    pickerTime?.let { initial -> NativeTimePicker(initial, onSet = { selected ->
        if(vm.state.value.editor == draft && !vm.state.value.busy) change(draft.copy(
            time = String.format(Locale.ROOT, "%02d:%02d", selected.hour.toInt(), selected.minute.toInt())))
        pickerTime = null
    }, onDismiss = { pickerTime = null }) }
    if(subtask) TextEntryDialog(R.string.subtask, onDismiss = { subtask = false }) {
        vm.action { addSubtask(draft.id!!, it) }; subtask = false
    }
    if(repeat) RepeatEditor(draft.id!!, vm) { repeat = false }
}

@Composable private fun reminderText(minutes: Long?): String = when(minutes) {
    null -> stringResource(R.string.reminder_none)
    0L -> stringResource(R.string.reminder_at)
    60L -> stringResource(R.string.hour_before)
    1440L -> stringResource(R.string.day_before)
    else -> pluralStringResource(R.plurals.reminder_minutes, minutes.toInt(), minutes.toInt())
}

@Composable fun TextEntryDialog(label: Int, initial: String = "", onDismiss: () -> Unit, onSave: (String) -> Unit) {
    var text by remember { mutableStateOf(initial) }
    AlertDialog(onDismissRequest = onDismiss, title = { Text(stringResource(label)) },
        text = { OutlinedTextField(text, { text = it }, label = { Text(stringResource(R.string.title)) }) },
        confirmButton = { TextButton({ onSave(text.trim()) }, enabled = text.isNotBlank()) { Text(stringResource(R.string.save)) } },
        dismissButton = { TextButton(onDismiss) { Text(stringResource(R.string.cancel)) } })
}
