// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.compositeOver
import com.codedbydan.momentum.data.NativeAccent
import org.junit.Assert.*
import org.junit.Test

class CelebrationRippleTest {
    private fun assertAllIndications(container: Color, ink: Color) {
        val alpha = celebrationRippleAlpha(container, ink)
        assertTrue(alpha.pressedAlpha > 0f)
        assertTrue(alpha.pressedAlpha <= 0.10f)
        for (state in listOf(0f, alpha.hoveredAlpha, alpha.focusedAlpha, alpha.draggedAlpha)) {
            for (platformMultiplier in listOf(1f, 2f)) {
                val stateBackground = ink.copy(alpha = state).compositeOver(container)
                val pressedBackground = ink.copy(alpha = alpha.pressedAlpha * platformMultiplier).compositeOver(stateBackground)
                assertTrue("$container / $ink state=$state multiplier=$platformMultiplier",
                    colorContrast(ink, pressedBackground) >= 4.5)
            }
        }
    }

    @Test fun nativePalettesKeepContrastWhenStateAndPressedRippleOverlap() {
        for (accent in NativeAccent.entries) for (dark in listOf(false, true)) {
            val scheme = nativeColorScheme(accent, dark)
            assertAllIndications(scheme.primary, scheme.onPrimary)
        }
    }

    @Test fun resolvedThemeRolesBoundIndicationIncludingNearContrastThreshold() {
        // Android's documented Material3 light/dark roles plus a low-headroom
        // resolved role pair. The helper receives the actual theme roles at use.
        for ((container, ink) in listOf(
            Color(0xFF476810) to Color.White,
            Color(0xFFACD370) to Color(0xFF213600),
            Color(0xFF777777) to Color.Black,
        )) assertAllIndications(container, ink)
    }
}
