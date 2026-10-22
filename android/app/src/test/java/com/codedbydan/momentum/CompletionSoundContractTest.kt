// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import com.codedbydan.momentum.data.AppSettings
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.platform.CompletionSoundChoice
import com.codedbydan.momentum.platform.isInteractiveCompletion
import org.junit.Assert.*
import org.junit.Test

class CompletionSoundContractTest {
    @Test fun soundIsOffAndLittleLiftIsTheDefaultChoice() {
        assertFalse(AppSettings().completionSound)
        assertEquals(CompletionSoundChoice.LITTLE_LIFT, AppSettings().completionSoundChoice)
    }

    @Test fun allTenApprovedStableChoicesExistInTheirNumberedOrder() {
        assertEquals(listOf("wood-pop", "double-check", "rounded-lift", "pocket-piano", "string-flick",
            "bright-finish", "harp-snap", "click-and-glow", "tiny-ta-da", "warm-spark"),
            CompletionSoundChoice.entries.take(10).map { it.key })
        assertEquals(20, CompletionSoundChoice.entries.map { it.resource }.distinct().size)
    }

    @Test fun unknownOrAbsentChoiceFallsBackAndTogglingPreservesChoice() {
        for(key in listOf(null, "", "future-sound", "BRIGHT_FINISH"))
            assertEquals(CompletionSoundChoice.LITTLE_LIFT, CompletionSoundChoice.resolve(key))
        for(choice in CompletionSoundChoice.entries) {
            assertEquals(choice, CompletionSoundChoice.resolve(choice.key))
            val selected = AppSettings(completionSound = true, completionSoundChoice = choice)
            assertEquals(choice, selected.copy(completionSound = false).copy(completionSound = true).completionSoundChoice)
        }
    }

    @Test fun onlyChangedSuccessfulCompletionOutcomesQualify() {
        val completed = listOf(Message.TaskCompleted, Message.TaskCompletedArchived,
            Message.TasksCompleted(2u), Message.TasksCompletedArchived(3u))
        for(message in completed) {
            assertTrue(Outcome(true, message, null, false).isInteractiveCompletion())
            assertFalse(Outcome(false, message, null, false).isInteractiveCompletion())
        }
        for(message in listOf(null, Message.SaveFailed("fixture failure"), Message.TaskAdded, Message.TaskDeleted,
            Message.TasksCompleted(0u), Message.TasksCompletedArchived(0u)))
            assertFalse(Outcome(true, message, null, false).isInteractiveCompletion())
    }
}
