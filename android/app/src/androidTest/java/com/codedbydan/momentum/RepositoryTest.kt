// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.MomentumRepository
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.util.UUID

@RunWith(AndroidJUnit4::class)
class RepositoryTest {
    @Test fun editsPersistOfflineAndExactUndoRestoresCompletion() = runBlocking {
        val context = ApplicationProvider.getApplicationContext<android.content.Context>()
        val directory = File(context.cacheDir, "core-test-${UUID.randomUUID()}")
        val repo = MomentumRepository(context, directory)
        try {
            val task = repo.create(TaskDraft("Offline task", "INBOX_PROJECT", java.time.LocalDate.now().toString(), null, null,
                0.0, "Private notes", emptyList(), listOf("Tag")), View.Today)
            assertNotNull(task.id)
            val done = repo.mutate { setDone(task.id!!, true) }
            assertTrue(repo.detail(task.id!!)!!.isDone)
            repo.mutate { undoBatch(done.undo!!) }
            assertFalse(repo.detail(task.id!!)!!.isDone)
            repo.close()
            val reopened = MomentumRepository(context, directory)
            try {
                assertEquals("Private notes", reopened.detail(task.id!!)!!.notes)
                assertEquals(1, reopened.snapshot(View.Today).listing.sections.flatMap { it.rows }.size)
            } finally { reopened.close() }
        } finally { repo.close(); directory.deleteRecursively() }
    }

    @Test fun failedSaveRetainsCommittedStateAndCanRetry() = runBlocking {
        val context = ApplicationProvider.getApplicationContext<android.content.Context>()
        val directory = File(context.cacheDir, "core-test-${UUID.randomUUID()}")
        val repo = MomentumRepository(context, directory)
        try {
            val original = TaskDraft("Original", "INBOX_PROJECT", null, null, null,
                0.0, "", emptyList(), emptyList())
            val task = repo.create(original, View.Today)
            val blocker = File(directory, ".momentum-transaction")
            blocker.writeText("isolated invalid journal")
            val outcome = repo.mutate { saveTask(task.id!!, original.copy(title = "New")) }
            assertTrue(outcome.message is Message.SaveFailed)
            assertEquals("Original", repo.detail(task.id!!)!!.title)
            blocker.delete()
            assertTrue(repo.mutate { saveTask(task.id!!, original.copy(title = "New")) }.changed)
            assertEquals("New", repo.detail(task.id!!)!!.title)
        } finally { repo.close(); directory.deleteRecursively() }
    }
}
