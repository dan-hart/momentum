// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import androidx.lifecycle.SavedStateHandle
import androidx.test.core.app.ApplicationProvider
import androidx.test.platform.app.InstrumentationRegistry
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.MomentumRepository
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import java.io.File
import java.util.UUID

class ProjectNavigationTest {
    private fun main(block: () -> Unit) = InstrumentationRegistry.getInstrumentation().runOnMainSync(block)
    @Test fun firstProjectNavigationAndRapidChangesResolveWithoutASecondSelection() = runBlocking {
        val app = ApplicationProvider.getApplicationContext<MomentumApplication>()
        val directory = File(app.cacheDir, "project-navigation-${UUID.randomUUID()}")
        val repo = MomentumRepository(app, directory)
        try {
            val first = repo.read { addProject("First") }!!; val second = repo.read { addProject("Second") }!!
            repo.create(TaskDraft("Second task", second, null, null, null, 0.0, "", emptyList(), emptyList()), View.Project(second))
            lateinit var vm: MomentumViewModel
            main { vm = MomentumViewModel(app, SavedStateHandle(), repo, {}, {}) }
            withTimeout(10_000) { while(vm.state.value.loading) delay(20) }
            main { vm.navigate(View.Project(first)) }
            withTimeout(10_000) { while(vm.state.value.loading) delay(20) }
            assertEquals(View.Project(first), vm.state.value.snapshot!!.listing.view)
            val entered = CompletableDeferred<Unit>(); val release = CompletableDeferred<Unit>()
            val holding = launch(Dispatchers.IO) { repo.read { entered.complete(Unit); release.await() } }
            entered.await()
            main { vm.navigate(View.Project(first)); vm.navigate(View.Project(second)) }
            assertTrue(vm.state.value.loading)
            release.complete(Unit); holding.join()
            withTimeout(10_000) { while(vm.state.value.loading) delay(20) }
            assertEquals(View.Project(second), vm.state.value.view)
            assertEquals(listOf("Second task"), vm.state.value.snapshot!!.listing.sections.flatMap { it.rows }.filterIsInstance<Row.Task>().map { it.row.title })
            repo.mutate { deleteProject(second) }
            main { vm.refresh() }
            withTimeout(10_000) { while(vm.state.value.loading) delay(20) }
            assertEquals(View.Today, vm.state.value.view)
        } finally { repo.close(); directory.deleteRecursively() }
    }
}
