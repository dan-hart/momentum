// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import android.app.Application
import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.ViewModelStore
import androidx.test.core.app.ApplicationProvider
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.MomentumRepository
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.first
import org.junit.Assert.*
import org.junit.Test
import java.io.File
import java.util.UUID

class TaskSearchRoutingTest {
    @Test fun systemResultUsesCurrentTitleAndRejectsCompletedDeletedOrArchivedIds() = runBlocking {
        fixture { repository, vm ->
            val id = repository.create(draft("Original indexed title"), View.Today).id!!
            repository.mutate { saveTask(id, draft("Current title")) }
            withContext(Dispatchers.Main) { vm.openSystemSearchTask(id) }!!.join()
            assertEquals("Current title", vm.state.value.editor!!.title)
            withContext(Dispatchers.Main) { vm.dismissEditor() }
            repository.mutate { setDone(id, true) }
            withContext(Dispatchers.Main) { vm.openSystemSearchTask(id) }!!.join()
            assertNull(vm.state.value.editor)
            repository.mutate { archiveDone() }
            withContext(Dispatchers.Main) { vm.openSystemSearchTask(id) }!!.join()
            assertNull(vm.state.value.editor)
            val deleted = repository.create(draft("Deleted result"), View.Today).id!!
            repository.mutate { deleteTask(deleted) }
            withContext(Dispatchers.Main) { vm.openSystemSearchTask(deleted) }!!.join()
            assertNull(vm.state.value.editor)
        }
    }

    @Test fun warmSystemActivationPreservesTheActiveDraftAndOpensOnlyAfterDismissal() = runBlocking {
        fixture { repository, vm ->
            val id = repository.create(draft("Indexed target"), View.Today).id!!
            withContext(Dispatchers.Main) {
                vm.newTask("Unfinished private draft")
                assertNull(vm.openSystemSearchTask(id))
                assertEquals("Unfinished private draft", vm.state.value.editor!!.title)
                vm.dismissEditor()
            }
            withTimeout(10_000) { vm.state.first { it.editor?.id == id } }
            assertEquals("Indexed target", vm.state.value.editor!!.title)
        }
    }

    private suspend fun fixture(block: suspend (MomentumRepository, MomentumViewModel) -> Unit) {
        val application = ApplicationProvider.getApplicationContext<Application>()
        val directory = File(application.cacheDir, "search-route-${UUID.randomUUID()}").also { it.mkdirs() }
        val repository = MomentumRepository(application, directory)
        val vm = withContext(Dispatchers.Main) { MomentumViewModel(application, SavedStateHandle(), repository, {}, {}) }
        val owners = ViewModelStore().also { withContext(Dispatchers.Main) { it.put("fixture", vm) } }
        try { withTimeout(20_000) { vm.state.first { !it.loading && !it.editorLoading }; block(repository, vm) } }
        finally { withContext(Dispatchers.Main) { owners.clear() }; repository.close(); directory.deleteRecursively() }
    }

    private fun draft(title: String) = TaskDraft(title, "INBOX_PROJECT", null, null, null, 0.0, "", emptyList(), emptyList())
}
