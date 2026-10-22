// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import com.codedbydan.momentum.data.AppSettings
import com.codedbydan.momentum.platform.*
import com.codedbydan.momentum.ui.*
import org.junit.Assert.*
import org.junit.Test

class NativeOverhaulContractTest {
    @Test fun volumeIsQuietFiniteAndBounded() {
        assertEquals(0.35f, AppSettings().completionSoundVolume)
        assertEquals(0f, boundedSoundVolume(-1f))
        assertEquals(1f, boundedSoundVolume(2f))
        assertEquals(0.35f, boundedSoundVolume(Float.NaN))
        assertTrue(AppSettings().dynamicEmptyMessages)
    }
    @Test fun candidateCatalogPreservesAllLegacyIds() {
        assertEquals(listOf("soft-knock", "little-lift", "felt-finish", "warm-thumb", "gentle-pair", "small-bloom", "sunny-tap", "pocket-harp", "easy-landing", "quiet-cheer"), CompletionSoundChoice.candidates.map { it.key })
        assertEquals(20, CompletionSoundChoice.entries.map { it.key }.distinct().size)
    }
    @Test fun messageChoiceAvoidsImmediateRepeatsAndKeepsScopesSeparate() {
        val selection = CompletionMessageSelection { 0 }
        assertEquals(1u, selection.choose("morning", listOf(1u, 2u)))
        assertEquals(2u, selection.choose("morning", listOf(1u, 2u)))
        assertEquals(1u, selection.choose("today", listOf(1u, 2u)))
        assertNull(selection.choose("pending", emptyList()))
        assertEquals(4u, selection.choose("single", listOf(4u)))
        assertEquals(4u, selection.choose("single", listOf(4u)))
    }
    @Test fun exactRoutesFallbackWithoutChangingSettings() {
        assertEquals("tasks", settingsRoute("tasks.dynamic-messages"))
        assertEquals("sync", settingsRoute("sync.nextcloud"))
        assertEquals("main", settingsRoute("unsupported.secret"))
        assertEquals("main", settingsRoute("main"))
    }
    @Test fun portableSoundValuesPreserveUnknownsButDeliberateKnownSelectionWins() {
        val unknown = AppSettings(completionSoundRaw = "future-sound")
        assertEquals("future-sound", com.codedbydan.momentum.data.portableSoundValue(unknown))
        assertEquals("soft-knock", com.codedbydan.momentum.data.portableSoundValue(unknown.copy(completionSoundChoice = CompletionSoundChoice.SOFT_KNOCK)))
        assertEquals("quiet-cheer", com.codedbydan.momentum.data.portableSoundValue(AppSettings(completionSoundRaw = "bright-finish", completionSoundChoice = CompletionSoundChoice.QUIET_CHEER)))
    }
    @Test fun exactSearchRouteChoosesOnlyOneAnchorAndInactiveProvidersFallbackSafely() {
        assertEquals(R.string.sound_volume, settingsAnchorLabel("sounds.volume", com.codedbydan.momentum.data.SyncProvider.Off))
        assertEquals(R.string.completion_sound_choice, settingsAnchorLabel("sounds.selection", com.codedbydan.momentum.data.SyncProvider.Off))
        assertEquals(R.string.provider, settingsAnchorLabel("sync.compression", com.codedbydan.momentum.data.SyncProvider.Off))
        assertEquals(R.string.compress, settingsAnchorLabel("sync.compression", com.codedbydan.momentum.data.SyncProvider.Nextcloud))
    }
    @Test fun aCompletedPeriodRemainsVisibleAfterItsLastTaskDisappears() {
        assertEquals(com.codedbydan.momentum.core.View.Morning, com.codedbydan.momentum.data.resolvedListingView(com.codedbydan.momentum.core.View.Morning, false, com.codedbydan.momentum.core.DayCompletion.PERIOD_COMPLETE))
        assertEquals(com.codedbydan.momentum.core.View.Today, com.codedbydan.momentum.data.resolvedListingView(com.codedbydan.momentum.core.View.Morning, false, com.codedbydan.momentum.core.DayCompletion.UNPLANNED))
        assertEquals(com.codedbydan.momentum.core.View.Today, com.codedbydan.momentum.data.resolvedListingView(com.codedbydan.momentum.core.View.Project("missing"), false, com.codedbydan.momentum.core.DayCompletion.DAY_COMPLETE))
    }
    @Test fun reducedMotionAndStaticPreferenceNeverProduceParticles() {
        val complete = com.codedbydan.momentum.core.DayCompletion.DAY_COMPLETE
        assertEquals(CelebrationFeedback.Static, celebrationFeedback(complete, true, 0f))
        assertEquals(CelebrationFeedback.Burst, celebrationFeedback(complete, true, 1f))
        assertEquals(CelebrationFeedback.Hidden, celebrationFeedback(complete, false, 1f))
        assertEquals(CelebrationFeedback.Hidden, celebrationFeedback(com.codedbydan.momentum.core.DayCompletion.UNPLANNED, true, 1f))
        assertEquals(CelebrationFeedback.Hidden, celebrationFeedback(com.codedbydan.momentum.core.DayCompletion.PENDING, true, 0f))
    }
    @Test fun bothNativeCatalogsContainOneHundredNonemptyDistinctMessages() {
        val resources = listOf(java.io.File("src/main/res"), java.io.File("app/src/main/res"), java.io.File("android/app/src/main/res")).first { it.isDirectory }
        for(locale in listOf("values", "values-de")) {
            val document = javax.xml.parsers.DocumentBuilderFactory.newInstance().newDocumentBuilder()
                .parse(java.io.File(resources, "$locale/completion_messages.xml"))
            val items = document.getElementsByTagName("item")
            val messages = (0 until items.length).map { items.item(it).textContent }
            assertEquals(100, messages.size)
            assertEquals(100, messages.toSet().size)
            assertTrue(messages.all { it.isNotBlank() })
        }
    }
    @Test fun copiedDetailsContainOnlyVisibleAppMetadata() {
        assertEquals("Momentum 0.4.11 (4011)\nAndroid", appDetails("0.4.11", 4011))
    }
}
