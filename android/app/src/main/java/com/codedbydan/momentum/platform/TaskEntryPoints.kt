// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.platform

import android.app.KeyguardManager
import android.content.Context
import android.content.Intent
import com.codedbydan.momentum.MomentumViewModel
import com.codedbydan.momentum.R
import com.codedbydan.momentum.Tab

/** Existing activity URL/share contract, also used by warm onNewIntent entry. */
internal class TaskEntryPoints(private val context: Context, private val vm: MomentumViewModel,
    private val deviceLocked: () -> Boolean = { context.getSystemService(KeyguardManager::class.java).isDeviceLocked },
) {
    fun capture(intent: Intent) {
        if(deviceLocked()) { captureWhileLocked(intent); return }
        when(intent.action) {
            "com.codedbydan.momentum.ADD_TASK" -> vm.newTask()
            "com.codedbydan.momentum.OPEN_TODAY" -> vm.navigate(com.codedbydan.momentum.core.View.Today)
            Intent.ACTION_SEARCH -> { vm.tab(Tab.Search); vm.search(intent.getStringExtra(android.app.SearchManager.QUERY).orEmpty()) }
            Intent.ACTION_SEND, Intent.ACTION_PROCESS_TEXT -> if(intent.type == "text/plain") {
                val extra = if(intent.action == Intent.ACTION_PROCESS_TEXT) Intent.EXTRA_PROCESS_TEXT else Intent.EXTRA_TEXT
                intent.getCharSequenceExtra(extra)?.toString()?.let { text ->
                    if(text.length <= 1_000_000) vm.captureText(text)
                    else vm.reportError(IllegalArgumentException(context.getString(R.string.capture_too_large)))
                }
            }
            Intent.ACTION_VIEW -> intent.data?.let { uri ->
                if(uri.scheme != "momentum") return
                if(uri.toString().length > 1_000_000) { vm.reportError(IllegalArgumentException(context.getString(R.string.capture_too_large))); return }
                when(uri.host) {
                    "add", "create-task" -> {
                        val title = uri.getQueryParameter("title") ?: return
                        vm.captureTask(title, uri.getQueryParameter("notes").orEmpty(),
                            uri.getQueryParameter("due").orEmpty(), uri.getQueryParameter("tags").orEmpty())
                    }
                    "complete-task" -> uri.getQueryParameter("title")?.let { title -> vm.action { check(!deviceLocked()) { context.getString(R.string.capture_unlock_required) }; completeByTitle(title) } }
                    "open-task" -> uri.getQueryParameter("id")?.let(vm::edit)
                    "system-search" -> uri.getQueryParameter("id")?.let(vm::openSystemSearchTask)
                    "find-tasks" -> { vm.tab(Tab.Search); vm.search(uri.getQueryParameter("query").orEmpty()) }
                    "reopen-task" -> uri.getQueryParameter("id")?.let { id -> vm.action { check(!deviceLocked()) { context.getString(R.string.capture_unlock_required) }; reopenTasks(listOf(id)) } }
                    "plan-task" -> uri.getQueryParameter("id")?.let { id -> vm.action { check(!deviceLocked()) { context.getString(R.string.capture_unlock_required) }; planForToday(listOf(id)) } }
                }
            }
        }
        intent.getStringExtra("task-id")?.let(vm::edit)
        if(intent.getBooleanExtra("open-today", false)) vm.navigate(com.codedbydan.momentum.core.View.Today)
        if(intent.getBooleanExtra("new-task", false)) vm.newTask()
    }

    /** Persist only reviewed draft payloads, never an Intent or a deferred arbitrary callback. */
    private fun captureWhileLocked(intent: Intent) {
        when(intent.action) {
            "com.codedbydan.momentum.ADD_TASK" -> { vm.newTask(); return }
            Intent.ACTION_SEND, Intent.ACTION_PROCESS_TEXT -> if(intent.type == "text/plain") {
                val extra = if(intent.action == Intent.ACTION_PROCESS_TEXT) Intent.EXTRA_PROCESS_TEXT else Intent.EXTRA_TEXT
                intent.getCharSequenceExtra(extra)?.toString()?.let { text ->
                    if(text.length <= 1_000_000) vm.captureText(text)
                    else vm.reportError(IllegalArgumentException(context.getString(R.string.capture_too_large)))
                }
                return
            }
            Intent.ACTION_VIEW -> intent.data?.let { uri ->
                if(uri.scheme != "momentum") return
                if(uri.toString().length > 1_000_000) { vm.reportError(IllegalArgumentException(context.getString(R.string.capture_too_large))); return }
                if(uri.host == "add" || uri.host == "create-task") {
                    uri.getQueryParameter("title")?.let { title ->
                        vm.captureTask(title, uri.getQueryParameter("notes").orEmpty(),
                            uri.getQueryParameter("due").orEmpty(), uri.getQueryParameter("tags").orEmpty())
                    }
                    return
                }
            }
        }
        if(intent.getBooleanExtra("new-task", false)) { vm.newTask(); return }
        if(intent.action != Intent.ACTION_MAIN || intent.hasExtra("task-id") || intent.hasExtra("open-today")) {
            vm.reportError(IllegalStateException(context.getString(R.string.capture_unlock_required)))
        }
    }

}
