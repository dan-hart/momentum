// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.platform

import org.junit.Assert.*
import org.junit.Test

class CompletionSoundPlaybackTest {
    private class Fixture {
        var audible = true
        var loads = 0
        var stops = 0
        var releases = 0
        val played = mutableListOf<Int>()
        val playback = CompletionSoundPlayback({ ++loads }, played::add, { stops++ }, { releases++ }, { audible })
    }

    @Test fun aSuccessfulLoadPlaysEachRequestedCompletionOnceAndCachesTheSample() {
        val f = Fixture()
        f.playback.play(CompletionSoundChoice.BRIGHT_FINISH)
        f.playback.play(CompletionSoundChoice.BRIGHT_FINISH)
        assertTrue(f.played.isEmpty())
        f.playback.loaded(1, 0)
        assertEquals(listOf(1, 1), f.played)
        f.playback.loaded(1, 0)
        assertEquals(listOf(1, 1), f.played)
        f.playback.play(CompletionSoundChoice.BRIGHT_FINISH)
        assertEquals(listOf(1, 1, 1), f.played)
        assertEquals(1, f.loads)
    }

    @Test fun mutingClearsPendingAudioAndReenablingDoesNotReplayIt() {
        val f = Fixture()
        f.playback.play(CompletionSoundChoice.WOOD_POP)
        f.playback.cancelPending()
        f.playback.loaded(1, 0)
        assertTrue(f.played.isEmpty())
        f.playback.play(CompletionSoundChoice.WOOD_POP)
        assertEquals(listOf(1), f.played)
        assertEquals(1, f.stops)
    }

    @Test fun releaseIsIdempotentAndLateLoadsCannotPlay() {
        val f = Fixture()
        f.playback.play(CompletionSoundChoice.WARM_SPARK)
        f.playback.close()
        f.playback.loaded(1, 0)
        f.playback.play(CompletionSoundChoice.WOOD_POP)
        f.playback.close()
        assertTrue(f.played.isEmpty())
        assertEquals(1, f.loads)
        assertEquals(1, f.releases)
    }

    @Test fun loadFailureAndSystemSilencingSuppressAudio() {
        val f = Fixture()
        f.playback.play(CompletionSoundChoice.WOOD_POP)
        f.playback.loaded(1, 1)
        assertTrue(f.played.isEmpty())
        f.audible = false
        f.playback.play(CompletionSoundChoice.WOOD_POP)
        assertEquals(1, f.loads)
        f.audible = true
        f.playback.play(CompletionSoundChoice.WOOD_POP)
        assertEquals(2, f.loads)
        f.audible = false
        f.playback.loaded(2, 0)
        assertTrue(f.played.isEmpty())
        f.audible = true
        f.playback.play(CompletionSoundChoice.WOOD_POP)
        assertEquals(listOf(2), f.played)
    }
}
