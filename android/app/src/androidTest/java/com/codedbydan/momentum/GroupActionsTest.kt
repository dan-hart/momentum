// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.lifecycle.SavedStateHandle
import androidx.test.core.app.ApplicationProvider
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.MomentumRepository
import com.codedbydan.momentum.ui.MomentumApp
import kotlinx.coroutines.runBlocking
import org.junit.*
import java.io.File
import java.time.LocalDate
import java.time.format.DateTimeFormatter
import java.util.UUID

class GroupActionsTest {
    @get:Rule val compose = createComposeRule()

    @Test fun shownGroupCanMoveToANamedWeekdayWithOneExactUndo() {
        val app = ApplicationProvider.getApplicationContext<MomentumApplication>()
        val directory = File(app.cacheDir, "group-actions-${UUID.randomUUID()}")
        val repo = MomentumRepository(app, directory)
        try {
            fun create(title: String, view: View) = runBlocking { repo.create(TaskDraft(title, "INBOX_PROJECT", LocalDate.now().toString(),
                null, null, 0.0, "", emptyList(), emptyList()), view).id!! }
            val morning = listOf(create("Morning one", View.Morning), create("Morning two", View.Morning))
            val evening = create("Evening unaffected", View.Tonight)
            lateinit var vm: MomentumViewModel
            compose.runOnUiThread { vm = MomentumViewModel(app, SavedStateHandle(), repo, {}, {}) }
            compose.setContent { MomentumApp(vm) }
            compose.waitUntil(15_000) { !vm.state.value.loading }
            compose.onNodeWithContentDescription(app.getString(R.string.select_shown_group, app.getString(R.string.morning))).performClick()
            compose.runOnIdle { Assert.assertEquals(morning.toSet(), vm.state.value.selected) }
            compose.onNodeWithContentDescription(app.getString(R.string.selection_actions)).performClick()
            val day = LocalDate.now().plusDays(2)
            val name = day.format(DateTimeFormatter.ofPattern("EEEE", app.resources.configuration.locales[0]))
            compose.onNodeWithText(name).performClick()
            compose.waitUntil(15_000) { !vm.state.value.selecting && vm.state.value.notice?.undo != null && !vm.state.value.loading }
            val undo = vm.state.value.notice!!.undo!!
            Assert.assertEquals(listOf(day.toString(), day.toString()), runBlocking { morning.map { repo.detail(it)!!.dueDay } })
            Assert.assertEquals(LocalDate.now().toString(), runBlocking { repo.detail(evening)!!.dueDay })
            Assert.assertTrue(runBlocking { repo.mutate { undoBatch(undo) }.changed })
            Assert.assertEquals(listOf(LocalDate.now().toString(), LocalDate.now().toString()), runBlocking { morning.map { repo.detail(it)!!.dueDay } })
        } finally { runBlocking { repo.close() }; directory.deleteRecursively() }
    }
}
