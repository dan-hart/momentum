// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import android.app.KeyguardManager
import android.content.Context
import android.view.KeyEvent as NativeKeyEvent
import android.view.KeyboardShortcutGroup
import android.view.KeyboardShortcutInfo
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.key.onKeyEvent
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import com.codedbydan.momentum.*
import com.codedbydan.momentum.R
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.AppSettings
import com.codedbydan.momentum.data.KeyboardModifier
import com.codedbydan.momentum.data.SyncProvider
import com.codedbydan.momentum.platform.PrivateClipboard
import com.codedbydan.momentum.platform.ClipboardUriUnsupportedException
import kotlinx.coroutines.launch
import kotlinx.coroutines.CancellationException

/** Activity fallback and Compose bubbling share this bridge; the first consumer owns a key. */
internal class KeyboardCommandBridge {
    var appHandler: ((KeyboardStroke, Boolean) -> Boolean)? = null
    var editorHandler: ((KeyboardStroke, Boolean) -> Boolean)? = null
    private var handledEvent: List<Long>? = null
    fun dispatch(event: NativeKeyEvent, textEditing: Boolean): Boolean {
        val stroke = event.keyboardStroke() ?: return false
        val identity = listOf(event.deviceId.toLong(), event.downTime, event.eventTime,
            event.keyCode.toLong(), event.metaState.toLong(), event.repeatCount.toLong(), event.action.toLong())
        // Some windows send the same key through Compose and Activity shortcut fallback.
        if(identity == handledEvent) return true
        val handled = (editorHandler ?: appHandler)?.invoke(stroke, textEditing) == true
        if(handled) handledEvent = identity
        return handled
    }
}
internal val LocalKeyboardCommandBridge = staticCompositionLocalOf<KeyboardCommandBridge?> { null }

@Composable internal fun rememberAppKeyboardCommands(
    vm: MomentumViewModel,
    modal: Boolean,
    newProject: () -> Unit,
    search: () -> Unit,
    move: (List<String>) -> Unit,
    repeat: (String) -> Unit,
    help: () -> Unit,
): KeyboardCommandBridge {
    val inherited = LocalKeyboardCommandBridge.current
    val bridge = inherited ?: remember { KeyboardCommandBridge() }
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val clipboardTitle = stringResource(R.string.title)
    val clipboardFailure = stringResource(R.string.clipboard_copy_failed)
    val clipboardUnsupported = stringResource(R.string.clipboard_large_unsupported)
    val rootView = LocalView.current
    val callbacks by rememberUpdatedState(object {
        fun dispatch(stroke: KeyboardStroke, textEditing: Boolean): Boolean {
            // Read fresh VM state before every key, including a second press before recomposition.
            val state = vm.state.value
            val snapshot = state.snapshot
            val rows = snapshot?.listing?.sections.orEmpty().flatMap { it.rows }.filterIsInstance<Row.Task>()
                .map { it.row }.filter { !it.archived }
            val available = KeyboardContext(
                visibleTasks = rows.map { KeyboardTask(it.id, it.title, it.isDone, !it.isSubtask) },
                selectedIds = state.selected, canUndo = snapshot?.canUndo == true,
                canArchive = snapshot?.listing?.canArchive == true,
                canSync = snapshot?.settings?.provider?.let { it != SyncProvider.Off } == true && !vm.platform.state.value.syncing,
                loading = state.loading, busy = state.busy, restoring = state.restoring,
                editorLoading = state.editorLoading, editor = state.editor != null,
                locked = context.getSystemService(KeyguardManager::class.java).isDeviceLocked,
                modal = modal || !rootView.hasWindowFocus(), textEditing = textEditing, archive = state.view == View.Archive,
                settings = state.tab == Tab.Settings, manualSort = snapshot?.preferences?.sort == SortKey.MANUAL,
            )
            // Dialogs register their own save/cancel handler, retaining their discard confirmation.
            if(available.editor) return false
            val command = resolveKeyboardCommand(stroke, snapshot?.settings?.keyboardModifier?.name ?: "Ctrl", available) ?: return false
            val ids = command.taskIds
            when(command.action) {
                KeyboardAction.NewTask -> { if(state.view == View.Archive || state.tab == Tab.Settings) vm.navigate(View.Today); vm.newTask() }
                KeyboardAction.NewProject -> newProject()
                KeyboardAction.Search, KeyboardAction.SearchTab -> search()
                KeyboardAction.Today -> vm.tab(Tab.Today)
                KeyboardAction.Upcoming -> vm.tab(Tab.Upcoming)
                KeyboardAction.SettingsTab, KeyboardAction.Settings -> vm.tab(Tab.Settings)
                KeyboardAction.NextTab -> vm.tab(Tab.entries[(state.tab.ordinal + 1) % Tab.entries.size])
                KeyboardAction.PreviousTab -> vm.tab(Tab.entries[(state.tab.ordinal + Tab.entries.size - 1) % Tab.entries.size])
                KeyboardAction.Sync -> vm.platform.syncNow()
                KeyboardAction.Help -> help()
                KeyboardAction.SelectAll -> vm.selectAll()
                KeyboardAction.DeselectAll -> vm.cancelSelection()
                KeyboardAction.Open -> vm.edit(ids.single())
                KeyboardAction.Done -> vm.action { if(ids.size == 1) toggleDone(ids.single()) else bulkDone(ids) }
                KeyboardAction.Duplicate -> vm.action { duplicateTask(ids.single()) }
                KeyboardAction.CopyTitle -> scope.launch {
                    try { PrivateClipboard.copy(context, clipboardTitle, rows.single { it.id == ids.single() }.title) }
                    catch(e: CancellationException) { throw e }
                    catch(_: ClipboardUriUnsupportedException) { vm.reportError(IllegalStateException(clipboardUnsupported)) }
                    catch(_: Exception) { vm.reportError(IllegalStateException(clipboardFailure)) }
                }
                KeyboardAction.Delete -> vm.action { bulkDelete(ids) }
                KeyboardAction.Archive -> vm.action { archiveDone() }
                KeyboardAction.Undo -> vm.action { undo() }
                KeyboardAction.PlanToday -> vm.action { if(ids.size == 1) toggleToday(ids.single()) else planForToday(ids) }
                KeyboardAction.Morning -> vm.action { toggleSlot(ids, Slot.MORNING) }
                KeyboardAction.Evening -> vm.action { toggleSlot(ids, Slot.TONIGHT) }
                KeyboardAction.Tomorrow -> vm.action { moveToTomorrow(ids) }
                KeyboardAction.NextWeek -> vm.action { moveToNextWeek(ids) }
                KeyboardAction.Move -> move(ids)
                KeyboardAction.Repeat -> repeat(ids.single())
                KeyboardAction.NudgeUp -> vm.action { nudge(ids.single(), -1, state.view) }
                KeyboardAction.NudgeDown -> vm.action { nudge(ids.single(), 1, state.view) }
                KeyboardAction.Save, KeyboardAction.Cancel -> return false
            }
            return true
        }
    })
    val handler = remember { { stroke: KeyboardStroke, editing: Boolean -> callbacks.dispatch(stroke, editing) } }
    DisposableEffect(bridge) {
        bridge.appHandler = handler
        onDispose { if(bridge.appHandler === handler) bridge.appHandler = null }
    }
    return bridge
}

/** Bubble after native controls have handled text editing, then check Compose's public editor flag. */
@Composable internal fun Modifier.appKeyboardShortcuts(bridge: KeyboardCommandBridge): Modifier {
    val view = LocalView.current
    return onKeyEvent { bridge.dispatch(it.nativeKeyEvent, (view.findFocus() ?: view).onCheckIsTextEditor()) }
}

@Composable internal fun Modifier.editorKeyboardShortcuts(
    vm: MomentumViewModel,
    canSave: Boolean,
    blocked: Boolean,
    save: () -> Unit,
    cancel: () -> Unit,
): Modifier {
    val context = LocalContext.current
    val bridge = LocalKeyboardCommandBridge.current
    val current by rememberUpdatedState(object {
        fun dispatch(stroke: KeyboardStroke): Boolean {
            val state = vm.state.value
            val available = KeyboardContext(editor = state.editor != null, canSave = canSave,
                busy = state.busy, restoring = state.restoring, editorLoading = state.editorLoading,
                modal = blocked, locked = context.getSystemService(KeyguardManager::class.java).isDeviceLocked)
            val command = resolveKeyboardCommand(stroke, state.snapshot?.settings?.keyboardModifier?.name ?: "Ctrl", available) ?: return false
            when(command.action) { KeyboardAction.Save -> save(); KeyboardAction.Cancel -> cancel(); else -> return false }
            return true
        }
    })
    val handler = remember { { stroke: KeyboardStroke, _: Boolean -> current.dispatch(stroke) } }
    DisposableEffect(bridge) {
        bridge?.editorHandler = handler
        onDispose { bridge?.let { if(it.editorHandler === handler) it.editorHandler = null } }
    }
    return onPreviewKeyEvent { event ->
        bridge?.dispatch(event.nativeKeyEvent, true)
            ?: (event.nativeKeyEvent.keyboardStroke()?.let { stroke -> current.dispatch(stroke) } == true)
    }
}

@Composable fun KeyboardModifierSettings(settings: AppSettings, change: ((AppSettings) -> AppSettings) -> Unit) {
    var help by rememberSaveable { mutableStateOf(false) }
    Text(stringResource(R.string.keyboard_shortcuts), style = MaterialTheme.typography.titleMedium,
        modifier = Modifier.padding(top = 12.dp).semantics { heading() })
    Text(stringResource(R.string.keyboard_modifier_hint), style = MaterialTheme.typography.bodyMedium)
    KeyboardModifier.entries.forEach { modifier ->
        Row(Modifier.fillMaxWidth().heightIn(min = 48.dp).selectable(settings.keyboardModifier == modifier,
            role = Role.RadioButton, onClick = { change { it.copy(keyboardModifier = modifier) } }),
            verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
            RadioButton(selected = settings.keyboardModifier == modifier, onClick = null)
            Text(stringResource(when(modifier) { KeyboardModifier.Ctrl -> R.string.keyboard_ctrl; KeyboardModifier.Alt -> R.string.keyboard_alt; KeyboardModifier.Meta -> R.string.keyboard_meta }), Modifier.padding(start = 12.dp))
        }
    }
    OutlinedButton({ help = true }, Modifier.heightIn(min = 48.dp)) { Text(stringResource(R.string.keyboard_help)) }
    if(help) KeyboardHelp(settings.keyboardModifier.name) { help = false }
}

@Composable internal fun KeyboardHelp(modifier: String, close: () -> Unit) {
    val context = LocalContext.current
    Dialog(onDismissRequest = close) {
        Surface(Modifier.widthIn(max = 720.dp).fillMaxWidth(), shape = MaterialTheme.shapes.extraLarge) {
            Column(Modifier.verticalScroll(rememberScrollState()).padding(20.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Text(stringResource(R.string.keyboard_help), style = MaterialTheme.typography.headlineSmall, modifier = Modifier.semantics { heading() })
                Text(stringResource(R.string.keyboard_help_hint))
                KeyboardGroup.entries.forEach { group ->
                    Text(stringResource(group.label()), style = MaterialTheme.typography.titleMedium, modifier = Modifier.semantics { heading() })
                    keyboardBindings(modifier).filter { it.group == group }.forEach { binding ->
                        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                            Text(stringResource(binding.action.label()), Modifier.weight(1f))
                            Text(binding.display(context), Modifier.widthIn(max = 180.dp), style = MaterialTheme.typography.labelLarge)
                        }
                    }
                }
                TextButton(close, Modifier.heightIn(min = 48.dp)) { Text(stringResource(R.string.close)) }
            }
        }
    }
}

internal fun nativeKeyboardGroups(context: Context, modifier: String): List<KeyboardShortcutGroup> = KeyboardGroup.entries.map { group ->
    KeyboardShortcutGroup(context.getString(group.label()), keyboardBindings(modifier).filter { it.group == group }.map {
        KeyboardShortcutInfo(context.getString(it.action.label()), it.key.nativeCode(), it.modifiers.nativeModifiers())
    })
}

private fun KeyboardGroup.label() = when(this) { KeyboardGroup.Global -> R.string.keyboard_global; KeyboardGroup.Tasks -> R.string.keyboard_tasks; KeyboardGroup.Editor -> R.string.keyboard_editor }
private fun KeyboardAction.label(): Int = when(this) {
    KeyboardAction.NewTask -> R.string.add_task; KeyboardAction.NewProject -> R.string.new_project
    KeyboardAction.Search -> R.string.search; KeyboardAction.Today -> R.string.today; KeyboardAction.Upcoming -> R.string.upcoming
    KeyboardAction.SearchTab -> R.string.keyboard_search_tab; KeyboardAction.SettingsTab -> R.string.keyboard_settings_tab
    KeyboardAction.NextTab -> R.string.keyboard_next_tab; KeyboardAction.PreviousTab -> R.string.keyboard_previous_tab
    KeyboardAction.Settings -> R.string.settings; KeyboardAction.Sync -> R.string.keyboard_sync; KeyboardAction.Help -> R.string.keyboard_help
    KeyboardAction.SelectAll -> R.string.keyboard_select_all; KeyboardAction.DeselectAll -> R.string.finish_selection
    KeyboardAction.Open -> R.string.edit_task; KeyboardAction.Done -> R.string.keyboard_done_reopen
    KeyboardAction.Duplicate -> R.string.duplicate; KeyboardAction.CopyTitle -> R.string.copy_title; KeyboardAction.Delete -> R.string.delete
    KeyboardAction.Archive -> R.string.keyboard_archive_done; KeyboardAction.Undo -> R.string.undo; KeyboardAction.PlanToday -> R.string.keyboard_plan_today
    KeyboardAction.Morning -> R.string.morning; KeyboardAction.Evening -> R.string.evening; KeyboardAction.Tomorrow -> R.string.tomorrow
    KeyboardAction.NextWeek -> R.string.next_week; KeyboardAction.Move -> R.string.move_to; KeyboardAction.Repeat -> R.string.repeat
    KeyboardAction.NudgeUp -> R.string.move_up; KeyboardAction.NudgeDown -> R.string.move_down; KeyboardAction.Save -> R.string.save; KeyboardAction.Cancel -> R.string.cancel
}
private fun KeyboardBinding.display(context: Context): String {
    val parts = buildList {
        if(modifiers and KeyboardModifiers.CTRL != 0) add(context.getString(R.string.keyboard_ctrl))
        if(modifiers and KeyboardModifiers.ALT != 0) add(context.getString(R.string.keyboard_alt))
        if(modifiers and KeyboardModifiers.META != 0) add(context.getString(R.string.keyboard_meta))
        if(modifiers and KeyboardModifiers.SHIFT != 0) add(context.getString(R.string.keyboard_shift))
        add(when(key) {
            KeyboardKey.DELETE -> context.getString(R.string.keyboard_delete_key); KeyboardKey.ENTER -> context.getString(R.string.keyboard_enter_key)
            KeyboardKey.ESCAPE -> "Esc"; KeyboardKey.SLASH -> "/"; KeyboardKey.COMMA -> ","; KeyboardKey.UP -> "↑"; KeyboardKey.DOWN -> "↓"
            KeyboardKey.RIGHT -> "→"; KeyboardKey.LEFT_BRACKET -> "["; KeyboardKey.RIGHT_BRACKET -> "]"
            KeyboardKey.ONE -> "1"; KeyboardKey.TWO -> "2"; KeyboardKey.THREE -> "3"; KeyboardKey.FOUR -> "4"; else -> key.name
        })
    }
    return parts.joinToString(" + ")
}
private fun Int.nativeModifiers(): Int =
    (if(this and KeyboardModifiers.CTRL != 0) NativeKeyEvent.META_CTRL_ON else 0) or
    (if(this and KeyboardModifiers.ALT != 0) NativeKeyEvent.META_ALT_ON else 0) or
    (if(this and KeyboardModifiers.META != 0) NativeKeyEvent.META_META_ON else 0) or
    (if(this and KeyboardModifiers.SHIFT != 0) NativeKeyEvent.META_SHIFT_ON else 0)
private fun KeyboardKey.nativeCode(): Int = when(this) {
    KeyboardKey.N -> NativeKeyEvent.KEYCODE_N; KeyboardKey.F -> NativeKeyEvent.KEYCODE_F; KeyboardKey.Z -> NativeKeyEvent.KEYCODE_Z
    KeyboardKey.SLASH -> NativeKeyEvent.KEYCODE_SLASH; KeyboardKey.COMMA -> NativeKeyEvent.KEYCODE_COMMA; KeyboardKey.A -> NativeKeyEvent.KEYCODE_A
    KeyboardKey.O -> NativeKeyEvent.KEYCODE_O; KeyboardKey.D -> NativeKeyEvent.KEYCODE_D; KeyboardKey.DELETE -> NativeKeyEvent.KEYCODE_FORWARD_DEL
    KeyboardKey.C -> NativeKeyEvent.KEYCODE_C; KeyboardKey.T -> NativeKeyEvent.KEYCODE_T; KeyboardKey.M -> NativeKeyEvent.KEYCODE_M
    KeyboardKey.RIGHT -> NativeKeyEvent.KEYCODE_DPAD_RIGHT; KeyboardKey.DOWN -> NativeKeyEvent.KEYCODE_DPAD_DOWN; KeyboardKey.R -> NativeKeyEvent.KEYCODE_R
    KeyboardKey.UP -> NativeKeyEvent.KEYCODE_DPAD_UP; KeyboardKey.E -> NativeKeyEvent.KEYCODE_E; KeyboardKey.ONE -> NativeKeyEvent.KEYCODE_1
    KeyboardKey.TWO -> NativeKeyEvent.KEYCODE_2; KeyboardKey.THREE -> NativeKeyEvent.KEYCODE_3; KeyboardKey.FOUR -> NativeKeyEvent.KEYCODE_4
    KeyboardKey.LEFT_BRACKET -> NativeKeyEvent.KEYCODE_LEFT_BRACKET; KeyboardKey.RIGHT_BRACKET -> NativeKeyEvent.KEYCODE_RIGHT_BRACKET
    KeyboardKey.ENTER -> NativeKeyEvent.KEYCODE_ENTER; KeyboardKey.ESCAPE -> NativeKeyEvent.KEYCODE_ESCAPE
}
private fun NativeKeyEvent.keyboardStroke(): KeyboardStroke? {
    val key = KeyboardKey.entries.firstOrNull { it.nativeCode() == keyCode } ?: return null
    val modifiers = (if(isCtrlPressed) KeyboardModifiers.CTRL else 0) or
        (if(isAltPressed) KeyboardModifiers.ALT else 0) or (if(isMetaPressed) KeyboardModifiers.META else 0) or
        (if(isShiftPressed) KeyboardModifiers.SHIFT else 0)
    return KeyboardStroke(key, modifiers, repeatCount, action == NativeKeyEvent.ACTION_DOWN, isSymPressed || isFunctionPressed)
}
