// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import androidx.activity.ComponentActivity
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.ViewModelStore
import androidx.test.core.app.ApplicationProvider
import com.codedbydan.momentum.data.DraftStore
import com.codedbydan.momentum.data.EditorDraft
import com.codedbydan.momentum.data.MomentumRepository
import com.codedbydan.momentum.ui.DraftRecoveryStatus
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import java.io.File
import java.util.UUID

/** Native recovery affordance, with real private draft files and an isolated VM. */
class NativeDraftRecoveryUiTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()

    @Test fun retryRemainsDisabledForAnActiveEditorThenRecoversTheRetainedDraft() = runBlocking {
        val app = ApplicationProvider.getApplicationContext<MomentumApplication>()
        val directory = File(app.cacheDir, "recovery-ui-${UUID.randomUUID()}")
        val repository = MomentumRepository(app, directory)
        val owners = ViewModelStore()
        val id = UUID.randomUUID().toString()
        val recovered = EditorDraft(title = "Recovered private draft", notes = "Preserved notes")
        DraftStore(directory).write(id, recovered)
        val payload = File(directory, "android-drafts/$id.draft")
        val original = payload.readBytes()
        payload.writeText("isolated draft read failure")
        val saved = SavedStateHandle(mapOf("editor-id" to id))
        lateinit var vm: MomentumViewModel
        try {
            compose.runOnUiThread {
                vm = MomentumViewModel(app, saved, repository, {}, {}, resumeAfterRestore = {})
                owners.put("recovery-ui", vm)
            }
            compose.setContent {
                val state by vm.state.collectAsState()
                MaterialTheme { DraftRecoveryStatus(state, vm) }
            }
            compose.waitUntil(15_000) { !vm.state.value.loading && !vm.state.value.editorLoading }
            val retry = compose.activity.getString(R.string.retry)
            compose.onNodeWithText(compose.activity.getString(R.string.draft_recovery_pending)).assertIsDisplayed()
            compose.runOnUiThread { vm.newTask("Active draft stays open") }
            compose.onNodeWithText(retry).assertIsNotEnabled()
            payload.writeBytes(original)
            compose.runOnUiThread { vm.dismissEditor() }
            compose.onNodeWithText(retry).assertIsEnabled().performClick()
            compose.waitUntil(15_000) { vm.state.value.editor == recovered && !vm.state.value.editorLoading }
            assertFalse(vm.state.value.draftRecoveryPending)
            assertEquals(id, saved.get<String>("editor-id"))
            assertArrayEquals(original, payload.readBytes())
        } finally {
            compose.runOnUiThread { owners.clear() }
            repository.close()
            directory.deleteRecursively()
        }
    }
}
