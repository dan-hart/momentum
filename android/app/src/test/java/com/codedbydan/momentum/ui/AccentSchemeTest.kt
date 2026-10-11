// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import androidx.compose.ui.graphics.Color
import com.codedbydan.momentum.data.NativeAccent
import org.junit.Assert.*
import org.junit.Test

class AccentSchemeTest {
    @Test fun opaqueAccentsMeetNormalTextContrastInBothAppearances() {
        for (choice in NativeAccent.entries) for (dark in listOf(false, true)) {
            val scheme = nativeColorScheme(choice, dark)
            val surfaces = accentSurfaces(scheme)
            for (surface in surfaces) assertTrue("$choice dark=$dark primary on surface",
                colorContrast(scheme.primary, surface) >= 4.5)
            for ((foreground, background) in listOf(
                scheme.onPrimary to scheme.primary, scheme.onPrimaryContainer to scheme.primaryContainer,
                scheme.onSecondary to scheme.secondary, scheme.onSecondaryContainer to scheme.secondaryContainer,
                scheme.onTertiary to scheme.tertiary, scheme.onTertiaryContainer to scheme.tertiaryContainer,
                scheme.onSurface to scheme.surface, scheme.onSurfaceVariant to scheme.surfaceVariant,
                scheme.onBackground to scheme.background,
            )) {
                assertEquals(1f, foreground.alpha, 0f)
                assertEquals(1f, background.alpha, 0f)
                assertTrue("$choice dark=$dark foreground/background", colorContrast(foreground, background) >= 4.5)
            }
        }
    }

    @Test fun swatchSelectionInkIsReadableForEverySeed() {
        for (choice in NativeAccent.entries) assertTrue(colorContrast(accentInk(choice.seed), choice.seed) >= 4.5)
    }

    @Test fun contrastHasTheKnownBlackWhiteEndpointsAndIsSymmetric() {
        assertEquals(21.0, colorContrast(Color.Black, Color.White), 0.001)
        assertEquals(1.0, colorContrast(Color.Black, Color.Black), 0.001)
        val color = Color(0xFF3498DB)
        assertEquals(colorContrast(color, Color.White), colorContrast(Color.White, color), 0.001)
    }

    @Test fun nativeTonesAdaptWhileKeepingTheSavedPaletteChoiceDistinct() {
        val light = nativeColorScheme(NativeAccent.Momentum, false)
        val dark = nativeColorScheme(NativeAccent.Momentum, true)
        assertEquals(Color(0xFFFF6600), NativeAccent.Momentum.seed)
        assertNotEquals(light.primary, dark.primary)
        assertNotEquals(nativeColorScheme(NativeAccent.Alizarin, false).primary,
            nativeColorScheme(NativeAccent.PeterRiver, false).primary)
    }

    @Test fun fixedColorRolesRemainStableAndLegibleAcrossAppearances() {
        for (choice in NativeAccent.entries) {
            val light = nativeColorScheme(choice, false)
            val dark = nativeColorScheme(choice, true)
            assertEquals(light.primaryFixed, dark.primaryFixed)
            assertEquals(light.primaryFixedDim, dark.primaryFixedDim)
            for ((foreground, background) in listOf(
                light.onPrimaryFixed to light.primaryFixed, light.onPrimaryFixed to light.primaryFixedDim,
                light.onSecondaryFixedVariant to light.secondaryFixedDim,
                light.onTertiaryFixedVariant to light.tertiaryFixedDim,
            )) assertTrue(colorContrast(foreground, background) >= 4.5)
        }
    }
}
