// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.lifecycle.SavedStateHandle
import androidx.test.core.app.ApplicationProvider
import com.codedbydan.momentum.core.Row
import com.codedbydan.momentum.core.TaskDraft
import com.codedbydan.momentum.core.View
import com.codedbydan.momentum.data.MomentumRepository
import com.codedbydan.momentum.ui.MomentumApp
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import java.io.File
import java.util.UUID

class InboxCleanupUiTest {
    @get:Rule val compose = createComposeRule()
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()

    @Test fun reviewCanOmitATagAndATaskAndUndoTheAppliedBatch(): Unit = runBlocking {
        val repo = MomentumRepository(app, File(app.cacheDir, "inbox-cleanup-${UUID.randomUUID()}"))
        var mounted: MomentumViewModel? = null
        val inbox = View.Project("INBOX_PROJECT")
        val link = "https://example.com/work-agenda"
        val title = "review work agenda $link"
        val omittedTitle = "review the draft https://example.com/draft"
        try {
            val workTag = repo.read { addTag("Work") }!!
            val updatedId = repo.create(TaskDraft(title, inbox.id, null, null, null, 0.0, "Keep these notes", emptyList(), emptyList()), inbox).id!!
            val omittedId = repo.create(TaskDraft(omittedTitle, inbox.id, null, null, null, 0.0, "", emptyList(), emptyList()), inbox).id!!
            val vm = compose.runOnUiThread {
                MomentumViewModel(app, SavedStateHandle(), repo, {}, {}).also { mounted = it }
            }
            compose.setContent { MomentumApp(vm) }
            compose.waitUntil(10_000) { !vm.state.value.loading }
            val cleanup = app.getString(R.string.inbox_cleanup)
            compose.onNodeWithText(cleanup).assertDoesNotExist()

            compose.runOnUiThread { vm.navigate(inbox) }
            compose.waitUntil(10_000) { vm.state.value.snapshot?.listing?.view == inbox }
            compose.onNodeWithText(cleanup).assertIsDisplayed().performClick()
            val apply = app.getString(R.string.inbox_cleanup_apply)
            compose.waitUntil(10_000) { compose.onAllNodesWithText(apply).fetchSemanticsNodes().isNotEmpty() }
            compose.onNode(hasText(app.getString(R.string.inbox_cleanup_add_tag, "Work")) and isToggleable())
                .performScrollTo().assertIsOn().performClick()
            compose.onNode(hasText(omittedTitle) and isToggleable()).performScrollTo().assertIsOn().performClick()
            assertEquals(title, repo.detail(updatedId)?.title)
            compose.onNodeWithText(apply).performClick()

            compose.waitUntil(10_000) { vm.state.value.notice?.undo != null }
            val updated = repo.detail(updatedId)!!
            assertFalse(updated.title.contains(link))
            assertTrue(updated.notes.contains(link))
            assertTrue(updated.notes.contains("Keep these notes"))
            assertFalse(workTag in updated.tagIds)
            assertEquals(omittedTitle, repo.detail(omittedId)?.title)

            compose.onNodeWithText(app.getString(R.string.undo)).performClick()
            compose.waitUntil(10_000) {
                vm.state.value.snapshot?.listing?.sections.orEmpty().any { section ->
                    section.rows.any { row -> row is Row.Task && row.row.title == title }
                }
            }
            assertEquals("Keep these notes", repo.detail(updatedId)?.notes)
        } finally {
            mounted?.platform?.foreground(false)
            repo.close()
            repo.directory.deleteRecursively()
        }
    }
}
