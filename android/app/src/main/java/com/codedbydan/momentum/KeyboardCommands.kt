// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

internal object KeyboardModifiers { const val CTRL = 1; const val ALT = 2; const val META = 4; const val SHIFT = 8 }
internal enum class KeyboardKey { N,F,Z,SLASH,COMMA,A,O,D,DELETE,C,T,M,RIGHT,DOWN,R,UP,E,ONE,TWO,THREE,FOUR,LEFT_BRACKET,RIGHT_BRACKET,ENTER,ESCAPE }
internal enum class KeyboardGroup { Global, Tasks, Editor }
internal enum class KeyboardAction { NewTask,NewProject,Search,Today,Upcoming,SearchTab,SettingsTab,NextTab,PreviousTab,Settings,Sync,Help,SelectAll,DeselectAll,Open,Done,Duplicate,CopyTitle,Delete,Archive,Undo,PlanToday,Morning,Evening,Tomorrow,NextWeek,Move,Repeat,NudgeUp,NudgeDown,Save,Cancel }
internal data class KeyboardStroke(val key: KeyboardKey, val modifiers: Int, val repeatCount: Int = 0, val down: Boolean = true, val unsupportedModifiers: Boolean = false)
internal data class KeyboardBinding(val action: KeyboardAction, val group: KeyboardGroup, val key: KeyboardKey, val modifiers: Int)
internal data class KeyboardTask(val id: String, val title: String, val done: Boolean, val topLevel: Boolean)
internal data class KeyboardContext(val visibleTasks: List<KeyboardTask> = emptyList(), val selectedIds: Set<String> = emptySet(), val canUndo: Boolean = false, val canArchive: Boolean = false, val canSync: Boolean = false, val canSave: Boolean = true, val loading: Boolean = false, val busy: Boolean = false, val restoring: Boolean = false, val editorLoading: Boolean = false, val locked: Boolean = false, val modal: Boolean = false, val editor: Boolean = false, val textEditing: Boolean = false, val archive: Boolean = false, val settings: Boolean = false, val manualSort: Boolean = true)
internal data class ResolvedKeyboardCommand(val action: KeyboardAction, val taskIds: List<String> = emptyList())
/** Native text selection/settings keep Ctrl; other app commands use the stored modifier. */
internal fun keyboardBindings(modifier: String): List<KeyboardBinding> {
    val base = when(modifier) { "Alt" -> KeyboardModifiers.ALT; "Meta" -> KeyboardModifiers.META; else -> KeyboardModifiers.CTRL }
    val shift = base or KeyboardModifiers.SHIFT
    val nudge = base or if(base == KeyboardModifiers.ALT) KeyboardModifiers.CTRL else KeyboardModifiers.ALT
    return buildList {
        fun add(action: KeyboardAction, group: KeyboardGroup, key: KeyboardKey, modifiers: Int = base) { add(KeyboardBinding(action, group, key, modifiers)) }
        add(KeyboardAction.NewTask, KeyboardGroup.Global, KeyboardKey.N)
        add(KeyboardAction.NewProject, KeyboardGroup.Global, KeyboardKey.N, shift)
        add(KeyboardAction.Search, KeyboardGroup.Global, KeyboardKey.F)
        add(KeyboardAction.Today, KeyboardGroup.Global, KeyboardKey.ONE)
        add(KeyboardAction.Upcoming, KeyboardGroup.Global, KeyboardKey.TWO)
        add(KeyboardAction.SearchTab, KeyboardGroup.Global, KeyboardKey.THREE)
        add(KeyboardAction.SettingsTab, KeyboardGroup.Global, KeyboardKey.FOUR)
        add(KeyboardAction.NextTab, KeyboardGroup.Global, KeyboardKey.RIGHT_BRACKET, shift)
        add(KeyboardAction.PreviousTab, KeyboardGroup.Global, KeyboardKey.LEFT_BRACKET, shift)
        add(KeyboardAction.Settings, KeyboardGroup.Global, KeyboardKey.COMMA, KeyboardModifiers.CTRL)
        add(KeyboardAction.Sync, KeyboardGroup.Global, KeyboardKey.R)
        add(KeyboardAction.Help, KeyboardGroup.Global, KeyboardKey.SLASH)
        add(KeyboardAction.SelectAll, KeyboardGroup.Tasks, KeyboardKey.A, KeyboardModifiers.CTRL)
        add(KeyboardAction.DeselectAll, KeyboardGroup.Tasks, KeyboardKey.A, shift)
        add(KeyboardAction.Open, KeyboardGroup.Tasks, KeyboardKey.O)
        add(KeyboardAction.Done, KeyboardGroup.Tasks, KeyboardKey.D)
        add(KeyboardAction.Duplicate, KeyboardGroup.Tasks, KeyboardKey.D, shift)
        add(KeyboardAction.CopyTitle, KeyboardGroup.Tasks, KeyboardKey.C, shift)
        add(KeyboardAction.Delete, KeyboardGroup.Tasks, KeyboardKey.DELETE)
        add(KeyboardAction.Archive, KeyboardGroup.Tasks, KeyboardKey.E)
        add(KeyboardAction.Undo, KeyboardGroup.Tasks, KeyboardKey.Z)
        add(KeyboardAction.PlanToday, KeyboardGroup.Tasks, KeyboardKey.T)
        add(KeyboardAction.Morning, KeyboardGroup.Tasks, KeyboardKey.M, shift)
        add(KeyboardAction.Evening, KeyboardGroup.Tasks, KeyboardKey.T, shift)
        add(KeyboardAction.Tomorrow, KeyboardGroup.Tasks, KeyboardKey.RIGHT, shift)
        add(KeyboardAction.NextWeek, KeyboardGroup.Tasks, KeyboardKey.DOWN, shift)
        add(KeyboardAction.Move, KeyboardGroup.Tasks, KeyboardKey.M)
        add(KeyboardAction.Repeat, KeyboardGroup.Tasks, KeyboardKey.R, shift)
        add(KeyboardAction.NudgeUp, KeyboardGroup.Tasks, KeyboardKey.UP, nudge)
        add(KeyboardAction.NudgeDown, KeyboardGroup.Tasks, KeyboardKey.DOWN, nudge)
        add(KeyboardAction.Save, KeyboardGroup.Editor, KeyboardKey.ENTER)
        add(KeyboardAction.Cancel, KeyboardGroup.Editor, KeyboardKey.ESCAPE, 0)
    }
}

/** Presentation availability only. Rust still validates and executes every task mutation. */
internal fun resolveKeyboardCommand(stroke: KeyboardStroke, modifier: String, context: KeyboardContext): ResolvedKeyboardCommand? {
    if(!stroke.down || stroke.repeatCount != 0 || stroke.unsupportedModifiers || context.locked || context.busy || context.restoring || context.editorLoading || context.modal) return null
    val binding = keyboardBindings(modifier).firstOrNull { it.key == stroke.key && it.modifiers == stroke.modifiers } ?: return null
    if(context.editor) return when(binding.action) {
        KeyboardAction.Save -> if(context.canSave) ResolvedKeyboardCommand(binding.action) else null
        KeyboardAction.Cancel -> ResolvedKeyboardCommand(binding.action)
        else -> null
    }
    if(context.loading || binding.group == KeyboardGroup.Editor) return null
    if(binding.group == KeyboardGroup.Global) {
        if(binding.action == KeyboardAction.Sync && !context.canSync) return null
        return ResolvedKeyboardCommand(binding.action)
    }
    if(context.textEditing || context.settings ||
        (context.archive && binding.action != KeyboardAction.Undo)) return null
    val selected = context.visibleTasks.filter { it.id in context.selectedIds }
    val ids = selected.map { it.id }
    val enabled = when(binding.action) {
        KeyboardAction.SelectAll -> context.visibleTasks.isNotEmpty()
        KeyboardAction.DeselectAll, KeyboardAction.Done, KeyboardAction.Delete,
        KeyboardAction.PlanToday, KeyboardAction.Morning, KeyboardAction.Evening,
        KeyboardAction.Tomorrow, KeyboardAction.NextWeek -> selected.isNotEmpty()
        KeyboardAction.Open, KeyboardAction.Duplicate, KeyboardAction.CopyTitle -> selected.size == 1
        KeyboardAction.Repeat -> selected.size == 1 && selected.single().topLevel
        KeyboardAction.Move -> selected.any { it.topLevel }
        KeyboardAction.NudgeUp, KeyboardAction.NudgeDown -> selected.size == 1 && context.manualSort
        KeyboardAction.Archive -> context.canArchive
        KeyboardAction.Undo -> context.canUndo
        else -> false
    }
    if(!enabled) return null
    return ResolvedKeyboardCommand(binding.action, if(binding.action == KeyboardAction.Move) selected.filter { it.topLevel }.map { it.id } else ids)
}
