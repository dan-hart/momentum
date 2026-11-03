// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import androidx.compose.material3.Typography
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontListFontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.em
import androidx.compose.ui.unit.sp
import com.codedbydan.momentum.data.AppSettings
import com.codedbydan.momentum.data.NativeFont
import com.codedbydan.momentum.data.normalizedNativeTextScale
import org.junit.Assert.*
import org.junit.Test

class NativeTypographyTest {
    @Test fun changingContentScaleDoesNotChangeInterfaceTypography() {
        val baseline = nativeTypography(AppSettings())
        val changed = nativeTypography(AppSettings(contentTextScale = 1.5f))
        assertEquals(baseline.interfaceStyles, changed.interfaceStyles)
        assertEquals(baseline.contentStyles.bodyLarge.fontSize.value * 1.5f,
            changed.contentStyles.bodyLarge.fontSize.value, 0.001f)
        assertTrue(changed.contentStyles.bodyLarge.fontSize.isSp)
    }

    @Test fun changingInterfaceScaleDoesNotChangeContentTypography() {
        val baseline = nativeTypography(AppSettings())
        val changed = nativeTypography(AppSettings(interfaceTextScale = 1.75f))
        assertEquals(baseline.contentStyles, changed.contentStyles)
        assertEquals(baseline.interfaceStyles.labelLarge.fontSize.value * 1.75f,
            changed.interfaceStyles.labelLarge.fontSize.value, 0.001f)
        assertEquals(baseline.interfaceStyles.bodyLarge.lineHeight.value * 1.75f,
            changed.interfaceStyles.bodyLarge.lineHeight.value, 0.001f)
    }

    @Test fun nativeFontChoicesPreserveSizesAndUseBuiltInFamilies() {
        val families = mapOf(NativeFont.Sans to FontFamily.SansSerif,
            NativeFont.Serif to FontFamily.Serif, NativeFont.Monospace to FontFamily.Monospace)
        val baseline = nativeTypography(AppSettings())
        assertEquals(Typography(), baseline.interfaceStyles)
        assertEquals(Typography(), baseline.contentStyles)
        for ((choice, family) in families) {
            val changed = nativeTypography(AppSettings(font = choice))
            assertEquals(family, changed.interfaceStyles.bodyLarge.fontFamily)
            assertEquals(family, changed.contentStyles.titleMedium.fontFamily)
            assertEquals(baseline.contentStyles.bodyLarge.fontSize, changed.contentStyles.bodyLarge.fontSize)
        }
    }

    @Test fun bundledChoicesUseFourRealFacesThroughoutBothTypographyAreas() {
        val bundledNames = setOf("AtkinsonHyperlegibleMono", "OpenDyslexic")
        assertTrue("Bundled choices must be selectable", NativeFont.entries.map { it.name }.containsAll(bundledNames))
        val base = Typography()
        for (choice in NativeFont.entries.filter { it.name in bundledNames }) {
            val changed = nativeTypography(AppSettings(font = choice, contentTextScale = 1.5f, interfaceTextScale = 0.8f))
            val family = changed.interfaceStyles.bodyLarge.fontFamily
            assertTrue("Bundled choices must use packaged faces", family is FontListFontFamily)
            val faces = (family as FontListFontFamily).fonts
            assertEquals(setOf(FontWeight.Normal to FontStyle.Normal, FontWeight.Bold to FontStyle.Normal,
                FontWeight.Normal to FontStyle.Italic, FontWeight.Bold to FontStyle.Italic),
                faces.map { it.weight to it.style }.toSet())
            assertEquals(4, faces.size)
            val styles = listOf(changed.interfaceStyles.displayLarge, changed.interfaceStyles.headlineSmall,
                changed.interfaceStyles.titleMedium, changed.interfaceStyles.labelSmall,
                changed.contentStyles.displaySmall, changed.contentStyles.headlineLarge,
                changed.contentStyles.bodySmall, changed.contentStyles.labelLarge)
            assertTrue(styles.all { it.fontFamily == family })
            assertEquals(base.bodyLarge.fontSize.value * 0.8f, changed.interfaceStyles.bodyLarge.fontSize.value, 0.001f)
            assertEquals(base.bodyLarge.fontSize.value * 1.5f, changed.contentStyles.bodyLarge.fontSize.value, 0.001f)
            assertEquals(AppSettings(compact = true), resetNativeTypography(AppSettings(font = choice,
                contentTextScale = 1.5f, interfaceTextScale = 0.8f, compact = true)))
        }
    }

    @Test fun invalidScalesAreNormalizedWithoutAffectingTheOtherScale() {
        assertEquals(1f, normalizedNativeTextScale(Float.NaN), 0f)
        assertEquals(1f, normalizedNativeTextScale(Float.POSITIVE_INFINITY), 0f)
        assertEquals(1f, normalizedNativeTextScale(Float.NEGATIVE_INFINITY), 0f)
        assertEquals(0.75f, normalizedNativeTextScale(-50f), 0f)
        assertEquals(2f, normalizedNativeTextScale(50f), 0f)
        val baseline = nativeTypography(AppSettings(contentTextScale = 1.25f))
        val invalid = nativeTypography(AppSettings(contentTextScale = 1.25f, interfaceTextScale = Float.NaN))
        assertEquals(baseline, invalid)
    }

    @Test fun relativeLineHeightAndTrackingStayRelativeToTheScaledFont() {
        val base = Typography(bodyLarge = TextStyle(fontSize = 16.sp, lineHeight = 1.5.em, letterSpacing = 0.02.em))
        val changed = nativeTypography(AppSettings(contentTextScale = 2f), base)
        assertEquals(32.sp, changed.contentStyles.bodyLarge.fontSize)
        assertEquals(1.5.em, changed.contentStyles.bodyLarge.lineHeight)
        assertEquals(0.02.em, changed.contentStyles.bodyLarge.letterSpacing)
    }
}
