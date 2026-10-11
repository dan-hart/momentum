// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import org.junit.Assert.*
import org.junit.Test

class KeyboardCommandsTest {
    private val selected = listOf(KeyboardTask("visible", "Title", false, true))
    private fun context() = KeyboardContext(visibleTasks = selected, selectedIds = setOf("visible", "stale"), canUndo = true, canArchive = true)
    private fun stroke(key: KeyboardKey, modifiers: Int = KeyboardModifiers.CTRL) = KeyboardStroke(key, modifiers)

    @Test fun visibleSelectionIsAuthoritativeAndKeepsListingOrder() {
        val tasks = listOf(KeyboardTask("second", "Second", false, true), KeyboardTask("first", "First", true, false))
        val resolved = resolveKeyboardCommand(stroke(KeyboardKey.D), "Ctrl", context().copy(visibleTasks = tasks, selectedIds = setOf("first", "missing", "second")))!!
        assertEquals(listOf("second", "first"), resolved.taskIds)
    }
    @Test fun editingTextDoesNotSelectOrMutateTasks() {
        for(key in listOf(KeyboardKey.A, KeyboardKey.D, KeyboardKey.Z, KeyboardKey.DELETE)) {
            assertNull(resolveKeyboardCommand(stroke(key), "Ctrl", context().copy(textEditing = true)))
        }
        assertEquals(KeyboardAction.Search, resolveKeyboardCommand(stroke(KeyboardKey.F), "Ctrl", context().copy(textEditing = true))!!.action)
    }
    @Test fun extraModifiersRepeatedKeysAndKeyUpDoNotDispatch() {
        assertNull(resolveKeyboardCommand(stroke(KeyboardKey.D, KeyboardModifiers.CTRL or KeyboardModifiers.META), "Ctrl", context()))
        assertNull(resolveKeyboardCommand(stroke(KeyboardKey.D).copy(repeatCount = 1), "Ctrl", context()))
        assertNull(resolveKeyboardCommand(stroke(KeyboardKey.D).copy(down = false), "Ctrl", context()))
        assertNull(resolveKeyboardCommand(stroke(KeyboardKey.D).copy(unsupportedModifiers = true), "Ctrl", context()))
    }
    @Test fun configuredModifierChangesAppCommandsButKeepsNativeSelectAll() {
        assertNull(resolveKeyboardCommand(stroke(KeyboardKey.N), "Alt", context()))
        assertEquals(KeyboardAction.NewTask, resolveKeyboardCommand(stroke(KeyboardKey.N, KeyboardModifiers.ALT), "Alt", context())!!.action)
        assertEquals(KeyboardAction.SelectAll, resolveKeyboardCommand(stroke(KeyboardKey.A), "Meta", context())!!.action)
        assertEquals(KeyboardAction.NudgeUp, resolveKeyboardCommand(stroke(KeyboardKey.UP, KeyboardModifiers.CTRL or KeyboardModifiers.ALT), "Alt", context())!!.action)
    }
    @Test fun unavailableTaskContextsNeverDispatchDestructiveActions() {
        for(blocked in listOf(context().copy(archive = true), context().copy(restoring = true), context().copy(loading = true), context().copy(busy = true), context().copy(editorLoading = true), context().copy(editor = true), context().copy(modal = true), context().copy(locked = true))) {
            assertNull(resolveKeyboardCommand(stroke(KeyboardKey.DELETE), "Ctrl", blocked))
        }
    }
    @Test fun singleTaskCommandsRejectMultipleOrStaleSelections() {
        val many = context().copy(visibleTasks = selected + KeyboardTask("other", "Other", false, true), selectedIds = setOf("visible", "other"))
        assertNull(resolveKeyboardCommand(stroke(KeyboardKey.O), "Ctrl", many))
        assertNull(resolveKeyboardCommand(stroke(KeyboardKey.O), "Ctrl", context().copy(selectedIds = setOf("missing"))))
        assertNull(resolveKeyboardCommand(stroke(KeyboardKey.R, KeyboardModifiers.CTRL or KeyboardModifiers.SHIFT), "Ctrl", context().copy(visibleTasks = listOf(selected.single().copy(topLevel = false)))))
    }
    @Test fun archiveAllowsExistingUndoOnlyAndRetainsEverySafetyGuard() {
        val archive = context().copy(archive = true)
        assertEquals(KeyboardAction.Undo, resolveKeyboardCommand(stroke(KeyboardKey.Z), "Ctrl", archive)!!.action)
        for(blocked in listOf(archive.copy(canUndo = false), archive.copy(textEditing = true), archive.copy(modal = true), archive.copy(busy = true), archive.copy(restoring = true), archive.copy(loading = true), archive.copy(editorLoading = true), archive.copy(editor = true), archive.copy(settings = true))) {
            assertNull(resolveKeyboardCommand(stroke(KeyboardKey.Z), "Ctrl", blocked))
        }
        for(key in listOf(KeyboardKey.A, KeyboardKey.O, KeyboardKey.D, KeyboardKey.DELETE, KeyboardKey.E)) {
            assertNull(resolveKeyboardCommand(stroke(key), "Ctrl", archive))
        }
    }

    @Test fun saveAndCancelAreAvailableOnlyForAnUnblockedEditor() {
        val editor = context().copy(editor = true, textEditing = true)
        assertEquals(KeyboardAction.Save, resolveKeyboardCommand(stroke(KeyboardKey.ENTER), "Ctrl", editor)!!.action)
        assertEquals(KeyboardAction.Cancel, resolveKeyboardCommand(stroke(KeyboardKey.ESCAPE, 0), "Ctrl", editor)!!.action)
        assertNull(resolveKeyboardCommand(stroke(KeyboardKey.ENTER), "Ctrl", editor.copy(busy = true)))
        assertNull(resolveKeyboardCommand(stroke(KeyboardKey.ENTER), "Ctrl", editor.copy(canSave = false)))
    }
    @Test fun registryHasNoCollisionsForEverySupportedModifier() {
        for(modifier in listOf("Ctrl", "Alt", "Meta")) {
            val bindings = keyboardBindings(modifier)
            assertEquals(bindings.size, bindings.map { it.key to it.modifiers }.toSet().size)
            assertTrue(bindings.any { it.action == KeyboardAction.Help })
        }
    }
}
