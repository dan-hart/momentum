// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.platform

import com.codedbydan.momentum.R

/** Stable sound slugs shared by portable native app settings. */
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
    WARM_SPARK("warm-spark", R.string.sound_warm_spark, R.raw.warm_spark),
    SOFT_KNOCK("soft-knock", R.string.sound_soft_knock, R.raw.soft_knock),
    LITTLE_LIFT("little-lift", R.string.sound_little_lift, R.raw.little_lift),
    FELT_FINISH("felt-finish", R.string.sound_felt_finish, R.raw.felt_finish),
    WARM_THUMB("warm-thumb", R.string.sound_warm_thumb, R.raw.warm_thumb),
    GENTLE_PAIR("gentle-pair", R.string.sound_gentle_pair, R.raw.gentle_pair),
    SMALL_BLOOM("small-bloom", R.string.sound_small_bloom, R.raw.small_bloom),
    SUNNY_TAP("sunny-tap", R.string.sound_sunny_tap, R.raw.sunny_tap),
    POCKET_HARP("pocket-harp", R.string.sound_pocket_harp, R.raw.pocket_harp),
    EASY_LANDING("easy-landing", R.string.sound_easy_landing, R.raw.easy_landing),
    QUIET_CHEER("quiet-cheer", R.string.sound_quiet_cheer, R.raw.quiet_cheer);

    companion object {
        val defaultChoice = LITTLE_LIFT
        val candidates = entries.takeLast(10)
        fun resolve(key: String?): CompletionSoundChoice = entries.firstOrNull { it.key == key } ?: defaultChoice
    }
}
