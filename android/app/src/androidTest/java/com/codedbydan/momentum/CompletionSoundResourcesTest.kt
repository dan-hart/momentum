// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import android.media.AudioAttributes
import android.media.SoundPool
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import com.codedbydan.momentum.platform.CompletionSoundChoice
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

@RunWith(AndroidJUnit4::class)
class CompletionSoundResourcesTest {
    @Test fun allTenBundledWavsDecodeThroughNativeSoundPoolWithoutPlaying() {
        val app = ApplicationProvider.getApplicationContext<MomentumApplication>()
        val loaded = CountDownLatch(10)
        val statuses = ConcurrentHashMap<Int, Int>()
        val ids = mutableListOf<Int>()
        lateinit var pool: SoundPool
        InstrumentationRegistry.getInstrumentation().runOnMainSync {
            pool = SoundPool.Builder().setMaxStreams(1).setAudioAttributes(AudioAttributes.Builder()
                .setUsage(AudioAttributes.USAGE_ASSISTANCE_SONIFICATION)
                .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION).build()).build()
            pool.setOnLoadCompleteListener { _, id, status -> statuses[id] = status; loaded.countDown() }
            for(choice in CompletionSoundChoice.entries) {
                app.resources.openRawResource(choice.resource).use { source ->
                    val header = ByteArray(12)
                    assertEquals(12, source.read(header))
                    assertEquals("RIFF", header.copyOfRange(0, 4).toString(Charsets.US_ASCII))
                    assertEquals("WAVE", header.copyOfRange(8, 12).toString(Charsets.US_ASCII))
                }
                ids.add(pool.load(app, choice.resource, 1))
            }
        }
        try {
            assertFalse(ids.contains(0))
            assertEquals(10, ids.distinct().size)
            assertTrue("All ten native samples finish decoding", loaded.await(20, TimeUnit.SECONDS))
            assertEquals(ids.toSet(), statuses.keys)
            assertEquals(listOf(0), statuses.values.distinct())
        } finally { InstrumentationRegistry.getInstrumentation().runOnMainSync { pool.release() } }
    }
}
