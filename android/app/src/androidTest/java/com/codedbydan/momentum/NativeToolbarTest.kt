// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import android.content.res.Configuration
import androidx.activity.ComponentActivity
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalResources
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.platform.WindowInfo
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import androidx.lifecycle.SavedStateHandle
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.filters.SdkSuppress
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.*
import com.codedbydan.momentum.ui.MomentumApp
import kotlinx.coroutines.runBlocking
import org.junit.*
import org.junit.runner.RunWith
import java.io.File
import java.util.Locale
import java.util.UUID

@RunWith(AndroidJUnit4::class)
@SdkSuppress(minSdkVersion = 33)
class NativeToolbarTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()
    private lateinit var repository: MomentumRepository
    private lateinit var directory: File
    private lateinit var vm: MomentumViewModel
    private lateinit var locales: NativeAppLocaleFixture
    private val german by lazy {
        val configuration = Configuration(app.resources.configuration).apply {
            setLocale(Locale.GERMAN)
            fontScale = 2f
        }
        compose.activity.createConfigurationContext(configuration)
    }

    @Before fun prepare() = runBlocking {
        locales = NativeAppLocaleFixture(compose.activity)
        locales.apply(Locale.GERMAN)
        compose.waitUntil(15_000) { locales.isApplied }
        directory = File(app.cacheDir, "toolbar-${UUID.randomUUID()}")
        repository = MomentumRepository(app, directory)
        repository.updateSettings { it.copy(interfaceTextScale = 2f, contentTextScale = 2f) }
        compose.runOnUiThread { vm = MomentumViewModel(app, SavedStateHandle(), repository, {}, {}) }
    }

    @After fun cleanup() = runBlocking {
        if(::locales.isInitialized) {
            locales.restore()
            compose.waitUntil(15_000) { locales.isApplied }
        }
        if(::repository.isInitialized) repository.close()
        if(::directory.isInitialized) directory.deleteRecursively()
        Unit
    }

    private fun showApp(windowSize: DpSize? = null) {
        // Use Android's context-backed converter, including non-linear system font scaling.
        val density = Density(german)
        compose.setContent {
            val actualWindow = LocalWindowInfo.current
            val fixtureWindow = if(windowSize == null) actualWindow else object : WindowInfo by actualWindow {
                override val containerDpSize: DpSize = requireNotNull(windowSize)
                override val containerSize = with(density) {
                    IntSize(windowSize.width.roundToPx(), windowSize.height.roundToPx())
                }
            }
            // Keep the host Activity context for native window/display services.
            CompositionLocalProvider(LocalResources provides german.resources,
                LocalConfiguration provides german.resources.configuration, LocalDensity provides density,
                LocalWindowInfo provides fixtureWindow) {
                MomentumApp(vm)
            }
        }
        compose.waitUntil(15_000) { !vm.state.value.loading && vm.state.value.snapshot?.settings?.interfaceTextScale == 2f }
    }

    private fun assertTitleFits(node: SemanticsNodeInteraction) {
        node.assertIsDisplayed()
        val layouts = mutableListOf<TextLayoutResult>()
        node.performSemanticsAction(SemanticsActions.GetTextLayoutResult) { it(layouts) }
        Assert.assertEquals(1, layouts.single().lineCount)
        Assert.assertTrue("Title must retain its full measured line height",
            node.fetchSemanticsNode().boundsInRoot.height + 1f >= layouts.single().size.height)
    }

    private fun assertAction(label: Int) = compose.onNodeWithContentDescription(german.getString(label))
        .assertIsDisplayed().assertWidthIsAtLeast(48.dp).assertHeightIsAtLeast(48.dp)

    @Test fun longGermanRootAndEditorTitlesKeepActionsAndScrollableContent() {
        val projectTitle = "Veröffentlichungsvorbereitung für die gemeinsame Aufgabenverwaltung und Dokumentation"
        val project = runBlocking { repository.read { addProject(projectTitle)!! } }
        compose.runOnUiThread { vm.navigate(View.Project(project)) }
        showApp()
        // The drawer entry has a click action; the toolbar title keeps the complete label.
        assertTitleFits(compose.onNode(hasText(projectTitle) and !hasClickAction()))
        assertAction(R.string.refresh)
        assertAction(R.string.options)

        compose.runOnUiThread { vm.newTask("Langer deutscher Aufgabentitel für den Editor") }
        assertTitleFits(compose.onNodeWithText(german.getString(R.string.add_task)))
        assertAction(R.string.save)
        assertAction(R.string.cancel)
        compose.onNodeWithText(german.getString(R.string.notes)).performScrollTo().assertIsDisplayed()
        assertAction(R.string.save)
    }

    @Test fun GermanImportToolbarRetainsCreateActionWhileSourceContentScrolls() {
        showApp()
        compose.runOnUiThread { vm.openTextImport() }
        assertTitleFits(compose.onNodeWithText(german.getString(R.string.import_text)))
        assertAction(R.string.create_from_text)
        assertAction(R.string.cancel)
        compose.onNodeWithText(german.getString(R.string.paste_text)).performScrollTo().assertIsDisplayed()
        assertAction(R.string.create_from_text)
    }

    @Test fun GermanLargestCombinedScaleKeepsAllNavigationLabelsInsideWindow() {
        showApp()
        val window = compose.onRoot().fetchSemanticsNode().boundsInRoot
        val tabRole = SemanticsMatcher.expectValue(SemanticsProperties.Role, Role.Tab)
        for(label in listOf(R.string.today, R.string.upcoming, R.string.search, R.string.settings)) {
            val name = german.getString(label)
            val targets = compose.onAllNodes(hasText(name) and tabRole)
            val visible = targets.fetchSemanticsNodes().withIndex().filter { (_, node) ->
                window.contains(node.boundsInRoot.center)
            }
            Assert.assertEquals("One visible native navigation item for $name", 1, visible.size)
            targets[visible.single().index].assertIsDisplayed().assertWidthIsAtLeast(48.dp).assertHeightIsAtLeast(48.dp)
            val labels = compose.onAllNodes(hasText(name) and hasAnyAncestor(tabRole), useUnmergedTree = true)
            val shown = labels.fetchSemanticsNodes().withIndex().filter { (_, node) -> window.contains(node.boundsInRoot.center) }
            Assert.assertEquals("One visible complete semantic label for $name", 1, shown.size)
            val bounds = shown.single().value.boundsInRoot
            Assert.assertTrue("$name stays within the left window edge", bounds.left >= window.left - 1f)
            Assert.assertTrue("$name stays within the right window edge", bounds.right <= window.right + 1f)
            Assert.assertTrue("$name stays within the bottom window edge", bounds.bottom <= window.bottom + 1f)
            assertTitleFits(labels[shown.single().index])
        }
    }

    @Test fun GermanTabletRailKeepsCompleteLabelsInsideNativeItemsAtCombinedScale() {
        // Public adaptive-window data selects the real native rail. The host
        // Activity/window services and Android's font converter remain intact;
        // this fixture does not resize or reconfigure the running test device.
        showApp(DpSize(800.dp, 1000.dp))
        val root = compose.onRoot().fetchSemanticsNode().boundsInRoot
        val tabRole = SemanticsMatcher.expectValue(SemanticsProperties.Role, Role.Tab)
        val inWindow = SemanticsMatcher("inside the actual host window") { root.contains(it.boundsInRoot.center) }
        val centers = mutableListOf<androidx.compose.ui.geometry.Offset>()
        for(label in listOf(R.string.today, R.string.upcoming, R.string.search, R.string.settings)) {
            val name = german.getString(label)
            val target = compose.onNode(hasText(name) and tabRole and inWindow)
            target.assertIsDisplayed().assertWidthIsAtLeast(48.dp).assertHeightIsAtLeast(48.dp)
            val item = target.fetchSemanticsNode().boundsInRoot
            centers += item.center
            val text = compose.onNode(hasText(name) and hasAnyAncestor(tabRole) and inWindow, useUnmergedTree = true)
            text.assertIsDisplayed()
            val bounds = text.fetchSemanticsNode().boundsInRoot
            Assert.assertTrue("$name label stays inside its native item at the left", bounds.left >= item.left - 1f)
            Assert.assertTrue("$name label stays inside its native item at the right", bounds.right <= item.right + 1f)
            Assert.assertTrue("$name item stays inside the real window at the left", item.left >= root.left - 1f)
            Assert.assertTrue("$name item stays inside the real window at the right", item.right <= root.right + 1f)
            Assert.assertTrue("$name item stays inside the real window at the bottom", item.bottom <= root.bottom + 1f)
            assertTitleFits(text)
        }
        Assert.assertTrue("The adaptive fixture must display a vertical native rail",
            centers.zipWithNext().all { (before, after) -> after.y > before.y && kotlin.math.abs(after.x - before.x) <= 1f })
    }
}
