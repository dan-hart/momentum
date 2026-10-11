// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.platform

import com.codedbydan.momentum.R

/** Device-local stable slugs shared by the native clients. */
enum class CompletionSoundChoice(val key: String, val label: Int, val resource: Int) {
    WOOD_POP("wood-pop", R.string.sound_wood_pop, R.raw.wood_pop),
    DOUBLE_CHECK("double-check", R.string.sound_double_check, R.raw.double_check),
    ROUNDED_LIFT("rounded-lift", R.string.sound_rounded_lift, R.raw.rounded_lift),
    POCKET_PIANO("pocket-piano", R.string.sound_pocket_piano, R.raw.pocket_piano),
    STRING_FLICK("string-flick", R.string.sound_string_flick, R.raw.string_flick),
    BRIGHT_FINISH("bright-finish", R.string.sound_bright_finish, R.raw.bright_finish),
    HARP_SNAP("harp-snap", R.string.sound_harp_snap, R.raw.harp_snap),
    CLICK_AND_GLOW("click-and-glow", R.string.sound_click_and_glow, R.raw.click_and_glow),
    TINY_TA_DA("tiny-ta-da", R.string.sound_tiny_ta_da, R.raw.tiny_ta_da),
    WARM_SPARK("warm-spark", R.string.sound_warm_spark, R.raw.warm_spark);

    companion object {
        val defaultChoice = BRIGHT_FINISH
        fun resolve(key: String?): CompletionSoundChoice = entries.firstOrNull { it.key == key } ?: defaultChoice
    }
}
