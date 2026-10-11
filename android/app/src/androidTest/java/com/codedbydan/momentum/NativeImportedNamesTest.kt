// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import android.content.res.Configuration
import android.content.res.Resources
import androidx.activity.ComponentActivity
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalResources
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.viewModelScope
import androidx.test.core.app.ApplicationProvider
import androidx.test.espresso.Espresso
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.filters.SdkSuppress
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.MomentumRepository
import com.codedbydan.momentum.ui.MomentumApp
import kotlinx.coroutines.cancel
import kotlinx.coroutines.runBlocking
import org.junit.*
import org.junit.runner.RunWith
import java.io.File
import java.time.LocalDate
import java.util.Locale
import java.util.UUID

/** Real Rust backup import followed by native presentation and mutations; no task-data caps. */
@RunWith(AndroidJUnit4::class)
class NativeImportedNamesTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()
    private lateinit var directory: File
    private lateinit var repository: MomentumRepository
    private lateinit var vm: MomentumViewModel
    private lateinit var strings: Resources
    private var locales: NativeAppLocaleFixture? = null
    private val projectName = oversized("ImportedProjekt ")
    private val tagName = oversized("ImportedTag ")
    private val taskTitle = oversized("Aufgabe ")
    private val shortProjectName = "Kurzes Ziel"
    private lateinit var project: String
    private lateinit var tag: String
    private lateinit var shortProject: String
    private lateinit var task: String

    @Before fun prepare() {
        directory = File(app.cacheDir, "imported-names-${UUID.randomUUID()}").also { it.mkdirs() }
        repository = MomentumRepository(app, File(directory, "target"))
        strings = compose.activity.resources
    }

    @After fun cleanup() = runBlocking {
        locales?.let { it.restore(); compose.waitUntil(15_000) { it.isApplied } }
        if(::vm.isInitialized) compose.runOnUiThread { vm.viewModelScope.cancel() }
        repository.close()
        directory.deleteRecursively()
        Unit
    }

    private fun importAndShow(inInbox: Boolean = false, germanLargeText: Boolean = false) {
        runBlocking {
            val source = MomentumRepository(app, File(directory, "source"))
            val backup = File(directory, "import.json")
            try {
                project = source.read { addProject(projectName)!! }
                tag = source.read { addTag(tagName)!! }
                shortProject = source.read { addProject(shortProjectName)!! }
                task = source.create(TaskDraft(taskTitle, if(inInbox) "INBOX_PROJECT" else project,
                    LocalDate.now().toString(), null, null, 0.0, "", listOf(tag), emptyList()), View.Today).id!!
                source.read { exportBackup(backup.absolutePath) }
            } finally { source.close() }
            repository.mutate { importBackup(backup.absolutePath) }
            if(germanLargeText) repository.updateSettings { it.copy(interfaceTextScale = 2f, contentTextScale = 2f) }
        }
        val fixture = if(germanLargeText) {
            val configuration = Configuration(compose.activity.resources.configuration).apply {
                setLocale(Locale.GERMAN)
                fontScale = 2f
            }
            compose.activity.createConfigurationContext(configuration)
        } else null
        strings = fixture?.resources ?: compose.activity.resources
        compose.runOnUiThread { vm = MomentumViewModel(app, SavedStateHandle(), repository, {}, {}) }
        compose.setContent {
            if(fixture == null) MomentumApp(vm)
            else {
                // Keep the real Activity context for window/display services and
                // use Android's font converter, including non-linear OS scaling.
                CompositionLocalProvider(LocalResources provides fixture.resources,
                    LocalConfiguration provides fixture.resources.configuration,
                    LocalDensity provides Density(fixture)) { MomentumApp(vm) }
            }
        }
        awaitSnapshot(View.Today)
        assertImportedNames()
    }

    private fun awaitSnapshot(view: View) = compose.waitUntil(15_000) {
        !vm.state.value.loading && vm.state.value.snapshot?.listing?.view == view
    }

    private fun assertImportedNames(expectedProject: String = projectName) = runBlocking {
        Assert.assertEquals(expectedProject, repository.read { projects().single { it.id == project }.title })
        Assert.assertEquals(tagName, repository.read { tags().single { it.id == tag }.title })
        Assert.assertEquals(taskTitle, repository.detail(task)!!.title)
    }

    private fun boundedText(node: SemanticsNodeInteraction, completeText: String, maximumLines: Int) {
        node.assertIsDisplayed()
        val layouts = mutableListOf<TextLayoutResult>()
        node.performSemanticsAction(SemanticsActions.GetTextLayoutResult) { it(layouts) }
        Assert.assertEquals(completeText, layouts.single().layoutInput.text.text)
        val layout = layouts.single()
        Assert.assertTrue("Imported name remains visually bounded: lines=${layout.lineCount}, " +
            "maxLines=${layout.layoutInput.maxLines}, softWrap=${layout.layoutInput.softWrap}, " +
            "layoutSize=${layout.size}, constraints=${layout.layoutInput.constraints}, " +
            "visibleBounds=${node.fetchSemanticsNode().boundsInRoot}", layout.lineCount <= maximumLines)
    }

    private fun touchTarget(node: SemanticsNodeInteraction): SemanticsNodeInteraction {
        node.assertIsDisplayed().assertHasClickAction()
        val semantics = node.fetchSemanticsNode()
        val bounds = semantics.touchBoundsInRoot
        val minimum = with(semantics.layoutInfo.density) { 48.dp.toPx() }
        // Material's visual bounds can be 40dp while its native hit region is
        // 48dp. Verify the actual target, using this node's Android density.
        Assert.assertTrue("Native touch width remains at least 48dp", bounds.width + 0.5f >= minimum)
        Assert.assertTrue("Native touch height remains at least 48dp", bounds.height + 0.5f >= minimum)
        return node
    }

    private fun boundedSingleLineEditor(node: SemanticsNodeInteraction, completeText: String) {
        fun bounds(expected: String) = run {
            node.performScrollTo().assertIsDisplayed()
            Assert.assertEquals(expected,
                node.fetchSemanticsNode().config[SemanticsProperties.EditableText].text)
            val layouts = mutableListOf<TextLayoutResult>()
            node.performSemanticsAction(SemanticsActions.GetTextLayoutResult) { it(layouts) }
            Assert.assertEquals(expected, layouts.single().layoutInput.text.text)
            val field = node.getUnclippedBoundsInRoot()
            val window = compose.onNode(isDialog()).getUnclippedBoundsInRoot()
            Assert.assertTrue("The complete editable value stays inside the native dialog viewport",
                field.left >= window.left - 1.dp && field.right <= window.right + 1.dp &&
                    field.top >= window.top - 1.dp && field.bottom <= window.bottom + 1.dp)
            field
        }

        val longBounds = bounds(completeText)
        node.performTextReplacement("Kurzer Name")
        Espresso.closeSoftKeyboard()
        val shortBounds = bounds("Kurzer Name")
        node.performTextReplacement(completeText)
        Espresso.closeSoftKeyboard()
        val restoredBounds = bounds(completeText)
        // CoreTextField keeps a horizontally scrolled, one-line viewport while
        // its internal TextLayoutResult may contain more than one paragraph line.
        Assert.assertTrue("An imported name cannot grow the single-line editor: " +
            "long=$longBounds short=$shortBounds restored=$restoredBounds",
            kotlin.math.abs((longBounds.bottom - longBounds.top).value -
                (shortBounds.bottom - shortBounds.top).value) <= 1f &&
                kotlin.math.abs((restoredBounds.bottom - restoredBounds.top).value -
                    (shortBounds.bottom - shortBounds.top).value) <= 1f)
    }

    private fun dialogNode(matcher: SemanticsMatcher, unmerged: Boolean = false) =
        compose.onNode(matcher and hasAnyAncestor(isDialog()), useUnmergedTree = unmerged)

    private fun dialogButton(label: Int) = dialogNode(hasText(strings.getString(label)) and hasClickAction())

    private fun openListActions(name: String) {
        touchTarget(compose.onNodeWithContentDescription(strings.getString(R.string.list_actions, name))
            .performScrollTo()).performClick()
    }

    private fun drawerEditorAndWarning() {
        compose.onNodeWithContentDescription(strings.getString(R.string.lists)).performClick()
        boundedText(compose.onNodeWithText(projectName, useUnmergedTree = true).performScrollTo(), projectName, 2)
        openListActions(projectName)
        compose.onNodeWithText(strings.getString(R.string.edit_task)).performClick()
        val name = dialogNode(hasSetTextAction() and hasText(strings.getString(R.string.name)))
        name.assertIsDisplayed()
        Assert.assertEquals(projectName, name.fetchSemanticsNode().config[SemanticsProperties.EditableText].text)
        boundedSingleLineEditor(name, projectName)
        dialogNode(hasSetTextAction() and hasText(strings.getString(R.string.color))).performScrollTo().assertIsDisplayed()
        touchTarget(dialogButton(R.string.save))
        val renamed = "$projectName Ende"
        name.performScrollTo().performTextReplacement(renamed)
        Espresso.closeSoftKeyboard()
        dialogButton(R.string.save).performClick()
        compose.waitUntil(15_000) { vm.state.value.snapshot?.projects?.any { it.id == project && it.title == renamed } == true }
        openListActions(renamed)
        compose.onNodeWithText(strings.getString(R.string.delete)).performClick()
        val warning = strings.getQuantityString(R.plurals.delete_project_scope, 1, 1)
        compose.waitUntil(15_000) { compose.onAllNodes(hasText(warning) and hasAnyAncestor(isDialog())).fetchSemanticsNodes().isNotEmpty() }
        boundedText(dialogNode(hasText(strings.getString(R.string.delete_organization, renamed))),
            strings.getString(R.string.delete_organization, renamed), 2)
        dialogNode(hasText(warning)).performScrollTo().assertIsDisplayed()
        touchTarget(dialogButton(R.string.delete)).assertIsEnabled()
        touchTarget(dialogButton(R.string.cancel)).performClick()
        assertImportedNames(renamed)
    }

    @Test fun importedDrawerNamesCanBeEditedWithoutBuryingColorOrDestructiveScope() {
        importAndShow()
        drawerEditorAndWarning()
    }

    @SdkSuppress(minSdkVersion = 33)
    @Test fun GermanCombinedLargeTextKeepsImportedListEditorAndWarningReachable() {
        locales = NativeAppLocaleFixture(compose.activity).also { it.apply(Locale.GERMAN) }
        compose.waitUntil(15_000) { locales!!.isApplied }
        importAndShow(germanLargeText = true)
        drawerEditorAndWarning()
    }

    @Test fun importedTaskActionTitleLeavesActionsReachableAndMoveKeepsCompleteNames() {
        importAndShow()
        compose.onNodeWithTag("task-list").performScrollToNode(hasContentDescription(strings.getString(R.string.task_actions, taskTitle)))
        compose.onNodeWithContentDescription(strings.getString(R.string.task_actions, taskTitle)).performClick()
        boundedText(compose.onNode(hasText(taskTitle) and !hasClickAction()), taskTitle, 3)
        compose.onNodeWithText(strings.getString(R.string.edit_task)).assertIsDisplayed().assertHasClickAction()
        compose.onNodeWithText(strings.getString(R.string.move_to)).performScrollTo().performClick()
        boundedText(dialogNode(hasText(projectName), unmerged = true).performScrollTo(), projectName, 2)
        touchTarget(dialogNode(hasText(shortProjectName) and hasClickAction()).performScrollTo()).performClick()
        compose.waitUntil(15_000) {
            !vm.state.value.busy && !vm.state.value.loading && vm.state.value.snapshot?.listing?.sections.orEmpty()
                .flatMap { it.rows }.filterIsInstance<Row.Task>().any { it.row.id == task && it.row.project?.id == shortProject }
        }
        Assert.assertEquals(shortProject, runBlocking { repository.detail(task) }!!.projectId)
        assertImportedNames()
    }

    @Test fun importedDestinationChipsFitTheNativeViewportAndMoveTheRealTask() {
        importAndShow(inInbox = true)
        compose.runOnUiThread { vm.select(task) }
        compose.onNodeWithTag("task-list").performScrollToNode(hasTestTag("task-destinations"))
        val destinations = compose.onNodeWithTag("task-destinations")
        val parentBounds = destinations.getUnclippedBoundsInRoot()
        val parentWidth = parentBounds.right - parentBounds.left
        for(name in listOf(tagName, projectName)) {
            destinations.performScrollToNode(hasText(name))
            val target = compose.onNode(hasText(name) and hasAnyAncestor(hasTestTag("task-destinations")))
            touchTarget(target)
            val targetBounds = target.getUnclippedBoundsInRoot()
            Assert.assertTrue("A full imported name cannot make its destination wider than the native viewport",
                targetBounds.right - targetBounds.left <= parentWidth + 1.dp)
            boundedText(compose.onNode(hasText(name) and hasAnyAncestor(hasTestTag("task-destinations")), useUnmergedTree = true), name, 1)
        }
        compose.onNode(hasText(projectName) and hasAnyAncestor(hasTestTag("task-destinations"))).performClick()
        compose.waitUntil(15_000) {
            !vm.state.value.busy && !vm.state.value.loading && vm.state.value.snapshot?.listing?.sections.orEmpty()
                .flatMap { it.rows }.filterIsInstance<Row.Task>().any { it.row.id == task && it.row.project?.id == project }
        }
        Assert.assertEquals(project, runBlocking { repository.detail(task) }!!.projectId)
        assertImportedNames()
    }

    @Test fun importedSearchAndGroupingNamesStayBoundedAndNavigateByTheirRealIds() {
        importAndShow()
        compose.runOnUiThread { vm.navigate(View.Search); vm.search("Imported") }
        compose.waitUntil(15_000) {
            !vm.state.value.loading && vm.state.value.snapshot?.listing?.sections.orEmpty().flatMap { it.rows }
                .any { it is Row.Project && it.item.id == project }
        }
        val inList = hasAnyAncestor(hasTestTag("task-list"))
        boundedText(compose.onNode(hasText(projectName) and inList, useUnmergedTree = true).performScrollTo(), projectName, 2)
        boundedText(compose.onNode(hasText("#$tagName") and inList, useUnmergedTree = true).performScrollTo(), "#$tagName", 2)
        compose.onNode(hasText("#$tagName") and inList and hasClickAction()).performClick()
        awaitSnapshot(View.Tag(tag))
        Assert.assertEquals(View.Tag(tag), vm.state.value.view)
        compose.runOnUiThread { vm.navigate(View.Today); vm.preferences { it.copy(groupBy = GroupBy.PROJECT) } }
        compose.waitUntil(15_000) { !vm.state.value.loading && vm.state.value.snapshot?.preferences?.groupBy == GroupBy.PROJECT }
        val heading = strings.getString(R.string.section_count, projectName, 1)
        compose.onNodeWithTag("task-list").performScrollToNode(hasText(heading))
        boundedText(compose.onNodeWithText(heading), heading, 2)
        touchTarget(compose.onNodeWithContentDescription(strings.getString(R.string.select_shown_group, projectName))).performClick()
        compose.waitUntil(15_000) { vm.state.value.selected == setOf(task) }
        Assert.assertEquals(setOf(task), vm.state.value.selected)
        assertImportedNames()
    }

    private fun oversized(prefix: String) = (prefix + "Veröffentlichungsvorbereitung und gemeinsame Dokumentation ".repeat(160)).take(8191) + "X"
}
