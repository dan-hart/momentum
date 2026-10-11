// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import androidx.compose.material3.ColorScheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.graphics.luminance
import com.codedbydan.momentum.data.NativeAccent

/** The same ten palette seeds as iOS; Android renders contrast-safe native color roles. */
internal val NativeAccent.seed: Color get() = Color(when(this) {
    NativeAccent.Momentum -> 0xFFFF6600
    NativeAccent.Alizarin -> 0xFFE74C3C
    NativeAccent.Carrot -> 0xFFE67E22
    NativeAccent.Orange -> 0xFFF39C12
    NativeAccent.Emerald -> 0xFF2ECC71
    NativeAccent.Turquoise -> 0xFF1ABC9C
    NativeAccent.PeterRiver -> 0xFF3498DB
    NativeAccent.Amethyst -> 0xFF9B59B6
    NativeAccent.Pomegranate -> 0xFFC0392B
    NativeAccent.GreenSea -> 0xFF16A085
})

internal fun colorContrast(first: Color, second: Color): Double {
    val a = first.luminance().toDouble()
    val b = second.luminance().toDouble()
    return (maxOf(a, b) + 0.05) / (minOf(a, b) + 0.05)
}

internal fun accentInk(background: Color): Color =
    if(colorContrast(Color.Black, background) >= colorContrast(Color.White, background)) Color.Black else Color.White

internal fun accentSurfaces(scheme: ColorScheme): List<Color> = listOf(
    scheme.background, scheme.surface, scheme.surfaceVariant, scheme.surfaceDim, scheme.surfaceBright,
    scheme.surfaceContainerLowest, scheme.surfaceContainerLow, scheme.surfaceContainer,
    scheme.surfaceContainerHigh, scheme.surfaceContainerHighest,
)

internal fun readableAccent(seed: Color, surfaces: List<Color>): Color {
    fun readable(color: Color) = surfaces.all { colorContrast(color, it) >= 4.55 }
    if(readable(seed)) return seed
    val candidates = listOf(Color.Black, Color.White).filter(::readable).map { endpoint ->
        var lower = 0f
        var upper = 1f
        repeat(28) {
            val fraction = (lower + upper) / 2
            if(readable(lerp(seed, endpoint, fraction))) upper = fraction else lower = fraction
        }
        upper to lerp(seed, endpoint, upper)
    }
    // Native light/dark surface families always admit at least one opaque endpoint.
    return candidates.minBy { it.first }.second
}

internal fun nativeColorScheme(accent: NativeAccent, dark: Boolean): ColorScheme {
    val base = if(dark) darkColorScheme() else lightColorScheme()
    val primary = readableAccent(accent.seed, accentSurfaces(base))
    val container = lerp(accent.seed, base.surface, if(dark) 0.7f else 0.85f)
    val ink = accentInk(primary)
    val containerInk = accentInk(container)
    val fixed = lerp(accent.seed, Color.White, 0.85f)
    val fixedDim = lerp(accent.seed, Color.White, 0.65f)
    return base.copy(
        primary = primary, onPrimary = ink, primaryContainer = container, onPrimaryContainer = containerInk,
        secondary = primary, onSecondary = ink, secondaryContainer = container, onSecondaryContainer = containerInk,
        tertiary = primary, onTertiary = ink, tertiaryContainer = container, onTertiaryContainer = containerInk,
        primaryFixed = fixed, primaryFixedDim = fixedDim, onPrimaryFixed = Color.Black, onPrimaryFixedVariant = Color.Black,
        secondaryFixed = fixed, secondaryFixedDim = fixedDim, onSecondaryFixed = Color.Black, onSecondaryFixedVariant = Color.Black,
        tertiaryFixed = fixed, tertiaryFixedDim = fixedDim, onTertiaryFixed = Color.Black, onTertiaryFixedVariant = Color.Black,
        surfaceTint = primary, inversePrimary = readableAccent(accent.seed, listOf(base.inverseSurface)),
    )
}
