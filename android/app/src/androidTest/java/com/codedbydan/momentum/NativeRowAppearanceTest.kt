// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.unit.dp
import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.viewModelScope
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.*
import com.codedbydan.momentum.ui.MomentumApp
import kotlinx.coroutines.cancel
import kotlinx.coroutines.runBlocking
import org.junit.*
import org.junit.runner.RunWith
import java.io.File
import java.time.LocalDate
import java.util.UUID

@RunWith(AndroidJUnit4::class)
class NativeRowAppearanceTest {
    @get:Rule val compose = createComposeRule()
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()
    private lateinit var repository: MomentumRepository
    private lateinit var directory: File
    private lateinit var vm: MomentumViewModel

    @Before fun prepare() {
        directory = File(app.cacheDir, "row-appearance-${UUID.randomUUID()}")
        repository = MomentumRepository(app, directory)
    }

    @After fun cleanup() = runBlocking {
        if(::vm.isInitialized) compose.runOnUiThread { vm.viewModelScope.cancel() }
        repository.close()
        directory.deleteRecursively()
        Unit
    }

    private fun showTask(title: String, projectTitle: String = "Projekt", tagTitle: String = "Dinge"): String {
        val id = runBlocking {
            val project = repository.read { addProject(projectTitle)!! }
            val tag = repository.read { addTag(tagTitle)!! }
            repository.mutate { updateProject(project, projectTitle, "#ffff00") }
            repository.mutate { updateTag(tag, tagTitle, "#00ffff") }
            repository.create(TaskDraft(title, project, LocalDate.now().toString(), null, null,
                0.0, "", listOf(tag), emptyList()), View.Today).id!!
        }
        compose.runOnUiThread { vm = MomentumViewModel(app, SavedStateHandle(), repository, {}, {}) }
        compose.setContent { MomentumApp(vm) }
        compose.waitUntil(15_000) { !vm.state.value.loading && vm.state.value.snapshot != null }
        compose.onNodeWithText(title).assertIsDisplayed()
        return id
    }

    @Test fun colorfulPreferencePersistsWithoutChangingOtherPresentationSettings() = runBlocking {
        repository.updateSettings { it.copy(colorfulLabels = false, compact = true, accent = NativeAccent.Emerald,
            keyboardModifier = KeyboardModifier.Alt, contentTextScale = 1.5f) }
        repository.close()
        repository = MomentumRepository(app, directory)
        val saved = repository.settings()
        Assert.assertFalse(saved.colorfulLabels)
        Assert.assertTrue(saved.compact)
        Assert.assertEquals(NativeAccent.Emerald, saved.accent)
        Assert.assertEquals(KeyboardModifier.Alt, saved.keyboardModifier)
        Assert.assertEquals(1.5f, saved.contentTextScale, 0f)
        repository.updateSettings { it.copy(colorfulLabels = true) }
        Assert.assertEquals(saved.copy(colorfulLabels = true), repository.settings())
    }

    @Test fun viewOptionsCompactShrinksAnOrdinaryRowAndKeepsMetadataAndTargets() {
        val title = "Kurze Aufgabe"
        showTask(title)
        val regular = compose.onNodeWithText(title).fetchSemanticsNode().boundsInRoot.height
        compose.onNodeWithContentDescription(app.getString(R.string.options)).performClick()
        compose.onNodeWithText(app.getString(R.string.row_compact)).performScrollTo().performClick()
        compose.waitUntil(15_000) { vm.state.value.snapshot?.settings?.compact == true }
        compose.onNodeWithText(app.getString(R.string.close)).performClick()
        val compact = compose.onNodeWithText(title).fetchSemanticsNode().boundsInRoot.height
        Assert.assertTrue("Compact must reduce actual row height even without notes", compact + 1f < regular)
        compose.onNodeWithText("Projekt · #Dinge").assertIsDisplayed()
        compose.onNodeWithContentDescription(app.getString(R.string.complete) + ": " + title)
            .assertWidthIsAtLeast(48.dp).assertHeightIsAtLeast(48.dp)
        compose.onNodeWithContentDescription(app.getString(R.string.task_actions, title))
            .assertWidthIsAtLeast(48.dp).assertHeightIsAtLeast(48.dp)
        Assert.assertTrue(runBlocking { repository.settings() }.compact)
    }

    @Test fun longGermanColoredMetadataKeepsCompleteTextAndSelectionStateAfterOptOut() {
        val title = "Veröffentlichungsvorbereitung für die gemeinsame Aufgabenverwaltung und Dokumentation"
        val project = "Gemeinsame Veröffentlichungsvorbereitungen und Dokumentationsaufgaben"
        val tag = "BarrierefreieZusammenarbeit"
        val metadata = "$project · #$tag"
        val id = showTask(title, project, tag)
        compose.runOnUiThread { vm.settings { it.copy(compact = true) } }
        compose.waitUntil(15_000) { vm.state.value.snapshot?.settings?.compact == true }
        val label = compose.onNodeWithText(metadata, useUnmergedTree = true)
        val layouts = mutableListOf<TextLayoutResult>()
        label.performSemanticsAction(SemanticsActions.GetTextLayoutResult) { it(layouts) }
        Assert.assertEquals(metadata, layouts.single().layoutInput.text.text)
        Assert.assertEquals(1, layouts.single().lineCount)
        Assert.assertTrue("Project and tag retain separately styled spans", layouts.single().layoutInput.text.spanStyles.size >= 3)
        compose.onNodeWithText(title).assertIsDisplayed()
        compose.runOnUiThread { vm.settings { it.copy(colorfulLabels = false) } }
        compose.waitUntil(15_000) { vm.state.value.snapshot?.settings?.colorfulLabels == false }
        layouts.clear()
        label.performSemanticsAction(SemanticsActions.GetTextLayoutResult) { it(layouts) }
        Assert.assertEquals(1, layouts.single().layoutInput.text.spanStyles.map { it.item.color }.distinct().size)
        compose.runOnUiThread { vm.select(id) }
        compose.waitUntil(15_000) { id in vm.state.value.selected }
        compose.onNode(hasText(title) and isSelected()).assertIsSelected()
        compose.onNodeWithText(metadata).assertIsDisplayed()
    }
}
