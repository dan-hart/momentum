// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import android.content.ClipboardManager
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Check
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.input.key.*
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import com.codedbydan.momentum.MomentumViewModel
import com.codedbydan.momentum.R
import com.codedbydan.momentum.UiState
import com.codedbydan.momentum.data.EditorDraft
import com.codedbydan.momentum.platform.PrivateClipboard
import com.codedbydan.momentum.platform.ClipboardTextTooLargeException
import kotlinx.coroutines.launch
import kotlinx.coroutines.CancellationException

/** Source text is a private disk-backed editor draft. Only Rust interprets its task content. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable fun TextImportEditor(state: UiState, vm: MomentumViewModel) {
    val draft = state.editor?.takeIf { it.isTextImport } ?: return
    val context = LocalContext.current
    val importNoChanges = stringResource(R.string.import_no_changes)
    val focus = remember { FocusRequester() }
    var discard by rememberSaveable { mutableStateOf(false) }
    var feedback by rememberSaveable { mutableIntStateOf(0) }
    var pasting by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    val tooLarge = draft.notes.length > EditorDraft.MAX_TEXT_IMPORT_CHARS
    val canImport = !state.busy && !tooLarge && draft.notes.isNotBlank()
    fun change(source: String): Boolean {
        if(vm.state.value.busy) return false
        if(source.length > EditorDraft.MAX_TEXT_IMPORT_CHARS) {
            feedback = R.string.import_text_too_large
            return false
        }
        feedback = 0
        val proposed = draft.copy(notes = source)
        vm.setEditor(proposed)
        return vm.state.value.editor == proposed
    }
    fun close() {
        if(!state.busy) { if(draft.hasContent) discard = true else vm.dismissEditor() }
    }
    fun paste() {
        if(state.busy || pasting) return
        // Read only on explicit Paste. A private text clip streams only from our narrow provider.
        try {
            val clip = context.getSystemService(ClipboardManager::class.java)?.primaryClip
            if(clip == null || clip.itemCount == 0) { feedback = R.string.import_clipboard_empty; return }
            val original = draft
            pasting = true
            scope.launch {
                try {
                    val items = (0 until clip.itemCount).map { index ->
                        val item = clip.getItemAt(index)
                        item.text ?: item.uri?.let { uri ->
                            if(PrivateClipboard.owns(context, uri)) PrivateClipboard.readOwned(context, uri, EditorDraft.MAX_TEXT_IMPORT_CHARS)
                            else uri.toString()
                        }
                    }
                    if(items.any { it == null } || items.all { it.isNullOrEmpty() }) feedback = R.string.import_clipboard_empty
                    else {
                        val current = vm.state.value
                        // Async reading must not replace text typed meanwhile or a different editor.
                        if(current.editor == original && !current.busy) {
                            val source = appendImportSource(original.notes, items.filterNotNull())
                            if(source == null) feedback = R.string.import_text_too_large else change(source)
                        } else feedback = R.string.import_clipboard_unavailable
                    }
                } catch(e: CancellationException) { throw e }
                catch(_: ClipboardTextTooLargeException) { feedback = R.string.import_text_too_large }
                catch(_: Exception) { feedback = R.string.import_clipboard_unavailable }
                finally { pasting = false }
            }
        } catch(_: SecurityException) { feedback = R.string.import_clipboard_unavailable }
        catch(_: RuntimeException) { feedback = R.string.import_clipboard_unavailable }
    }
    Dialog(onDismissRequest = ::close,
        properties = DialogProperties(usePlatformDefaultWidth = false, decorFitsSystemWindows = false)) {
        Box(Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.safeDrawing).imePadding().padding(12.dp),
            contentAlignment = Alignment.Center) {
            Surface(Modifier.widthIn(max = 720.dp).fillMaxWidth(), shape = MaterialTheme.shapes.extraLarge) {
                Column(Modifier.editorKeyboardShortcuts(vm, canSave = canImport, blocked = discard,
                    save = vm::importEditorText, cancel = ::close)) {
                    TopAppBar(title = { AdaptiveTopAppBarTitle(stringResource(R.string.import_text)) },
                        expandedHeight = adaptiveTopAppBarHeight(),
                        navigationIcon = { IconButton(onClick = ::close, enabled = !state.busy,
                            modifier = Modifier.sizeIn(minWidth = 48.dp, minHeight = 48.dp)) {
                            Icon(Icons.Default.Close, stringResource(R.string.cancel))
                        } }, actions = { IconButton(onClick = vm::importEditorText, enabled = canImport,
                            modifier = Modifier.sizeIn(minWidth = 48.dp, minHeight = 48.dp)) {
                            Icon(Icons.Default.Check, stringResource(R.string.create_from_text))
                        } })
                    Column(Modifier.verticalScroll(rememberScrollState()).padding(16.dp),
                        verticalArrangement = Arrangement.spacedBy(12.dp)) {
                        CaptureStatus(state)
                        DraftRecoveryStatus(state, vm)
                        if(state.editorCapacityError) Text(stringResource(R.string.draft_edit_capacity), color = MaterialTheme.colorScheme.error,
                            modifier = Modifier.semantics { liveRegion = LiveRegionMode.Assertive })
                        Text(stringResource(R.string.import_text_hint), style = MaterialTheme.typography.bodyMedium)
                        if(feedback != 0 || tooLarge) Text(stringResource(if(tooLarge) R.string.import_text_too_large else feedback),
                            color = MaterialTheme.colorScheme.error,
                            modifier = Modifier.semantics { liveRegion = LiveRegionMode.Assertive })
                        if(state.editorError) Text(stringResource(R.string.import_source_required), color = MaterialTheme.colorScheme.error,
                            modifier = Modifier.semantics { liveRegion = LiveRegionMode.Assertive })
                        if(state.error != null) Text(
                            if(state.error == importNoChanges) state.error else stringResource(R.string.save_failed),
                            color = MaterialTheme.colorScheme.error,
                            modifier = Modifier.semantics { liveRegion = LiveRegionMode.Assertive })
                        BoundedSourceField(source = draft.notes, onSourceChange = ::change, enabled = !state.busy,
                            label = stringResource(R.string.import_source), minLines = 4, maxLines = 10,
                            modifier = Modifier.fillMaxWidth().focusRequester(focus),
                            isError = tooLarge || state.editorError || feedback == R.string.import_text_too_large)
                        OutlinedButton(onClick = ::paste, enabled = !state.busy && !pasting) { Text(stringResource(R.string.paste_text)) }
                        if(state.busy) LinearProgressIndicator(Modifier.fillMaxWidth())
                    }
                }
            }
        }
        LaunchedEffect(Unit) { focus.requestFocus() }
    }
    if(discard) AlertDialog(onDismissRequest = { if(!state.busy) discard = false },
        title = { Text(stringResource(R.string.discard_title)) }, text = { Text(stringResource(R.string.discard_hint)) },
        confirmButton = { TextButton(onClick = { if(!state.busy) { vm.dismissEditor(); discard = false } }, enabled = !state.busy) {
            Text(stringResource(R.string.discard))
        } }, dismissButton = { TextButton(onClick = { discard = false }) { Text(stringResource(R.string.cancel)) } })
}

/** Preserve line order and reject the whole paste instead of silently truncating private source. */
internal fun appendImportSource(source: String, items: List<CharSequence>): String? {
    if(source.length > EditorDraft.MAX_TEXT_IMPORT_CHARS) return null
    val result = StringBuilder(source)
    for(item in items) {
        val separator = if(result.isNotEmpty() && result.last() != '\n') 1 else 0
        if(item.length > EditorDraft.MAX_TEXT_IMPORT_CHARS - result.length - separator) return null
        if(separator == 1) result.append('\n')
        result.append(item)
    }
    return result.toString()
}
