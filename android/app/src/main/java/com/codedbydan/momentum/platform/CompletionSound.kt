// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.platform

import android.content.Context
import android.media.AudioAttributes
import android.media.AudioManager
import android.media.SoundPool
import com.codedbydan.momentum.core.Message
import com.codedbydan.momentum.core.Outcome

fun Outcome.isInteractiveCompletion(): Boolean = changed && when(val result = message) {
    Message.TaskCompleted, Message.TaskCompletedArchived -> true
    is Message.TasksCompleted -> result.n > 0u
    is Message.TasksCompletedArchived -> result.n > 0u
    else -> false
}

interface CompletionSoundPlayer : AutoCloseable {
    fun play(choice: CompletionSoundChoice)
    /** Cancels queued loads and stops current feedback on mute or leaving the app. */
    fun cancelPending()
    override fun close()
}

/** Small asynchronous sample owner shared by the native adapter and lifecycle regression tests. */
internal class CompletionSoundPlayback(
    private val load: (CompletionSoundChoice) -> Int,
    private val playSample: (Int) -> Unit,
    private val stop: () -> Unit,
    private val release: () -> Unit,
    private val audible: () -> Boolean,
) : CompletionSoundPlayer {
    private val samples = mutableMapOf<CompletionSoundChoice, Int>()
    private val ready = mutableSetOf<Int>()
    private val pending = mutableListOf<Int>()
    private var closed = false

    @Synchronized override fun play(choice: CompletionSoundChoice) {
        if(closed || !audible()) return
        val id = samples[choice] ?: load(choice).takeIf { it != 0 }?.also { samples[choice] = it } ?: return
        if(id in ready) playSample(id) else pending.add(id)
    }

    @Synchronized fun loaded(id: Int, status: Int) {
        if(closed || id !in samples.values) return
        val requests = pending.count { it == id }
        pending.removeAll { it == id }
        if(status != 0) { samples.entries.removeAll { it.value == id }; return }
        ready.add(id)
        if(audible()) repeat(requests) { playSample(id) }
    }

    @Synchronized override fun cancelPending() {
        if(closed) return
        pending.clear()
        stop()
    }

    @Synchronized override fun close() {
        if(closed) return
        closed = true
        pending.clear(); samples.clear(); ready.clear()
        release()
    }
}

/** Sonification follows Android's system volume and explicitly honors silent/vibrate. */
class CompletionSound(context: Context) : CompletionSoundPlayer {
    private val application = context.applicationContext
    private val audio = application.getSystemService(AudioManager::class.java)
    private var pool: SoundPool? = null
    private var stream = 0
    private val playback: CompletionSoundPlayback = CompletionSoundPlayback(
        load = { choice ->
            val owner = pool ?: SoundPool.Builder().setMaxStreams(1).setAudioAttributes(
                AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_ASSISTANCE_SONIFICATION)
                    .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION).build()
            ).build().also { created ->
                created.setOnLoadCompleteListener { _, sample, status -> playback.loaded(sample, status) }
                pool = created
            }
            owner.load(application, choice.resource, 1)
        },
        playSample = { sample -> stream = pool?.play(sample, 1f, 1f, 1, 0, 1f) ?: 0 },
        stop = { if(stream != 0) pool?.stop(stream); stream = 0 },
        release = { pool?.setOnLoadCompleteListener(null); pool?.release(); pool = null; stream = 0 },
        audible = { audio.ringerMode == AudioManager.RINGER_MODE_NORMAL && audio.getStreamVolume(AudioManager.STREAM_SYSTEM) > 0 },
    )
    override fun play(choice: CompletionSoundChoice) { runCatching { playback.play(choice) } }
    override fun cancelPending() { playback.cancelPending() }
    override fun close() { playback.close() }
}
