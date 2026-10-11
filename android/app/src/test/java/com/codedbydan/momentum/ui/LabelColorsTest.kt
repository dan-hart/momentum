// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import androidx.compose.ui.graphics.Color
import com.codedbydan.momentum.data.NativeAccent
import org.junit.Assert.*
import org.junit.Test

class LabelColorsTest {
    @Test fun coreColorsAcceptOnlyOpaqueNormalizedHexContract() {
        assertEquals(Color(0xFFFF6600), coreLabelColor("#ff6600"))
        assertEquals(Color(0xFFABCDEF), coreLabelColor("#ABCDEF"))
        for(invalid in listOf(null, "", "#fff", "#ffffff00", "red", "#gggggg", " #ffffff")) assertNull(coreLabelColor(invalid))
    }

    @Test fun labelsMeetContrastOnNormalAndSelectedNativeSurfaces() {
        for(accent in NativeAccent.entries) for(dark in listOf(false, true)) {
            val scheme = nativeColorScheme(accent, dark)
            for(hex in listOf("#000000", "#ffffff", "#ff6600", "#ffff00", "#00ffff", "#ff00ff", "#808080")) {
                val normal = labelInk(hex, scheme, true)
                assertEquals(1f, normal.alpha, 0f)
                for(surface in accentSurfaces(scheme)) assertTrue("$hex $accent dark=$dark", colorContrast(normal, surface) >= 4.5)
                assertTrue(colorContrast(labelInk(hex, scheme, true, selected = true), scheme.secondaryContainer) >= 4.5)
            }
        }
    }

    @Test fun disabledOrInvalidColorsUseSemanticTextRoles() {
        for(dark in listOf(false, true)) {
            val scheme = nativeColorScheme(NativeAccent.Momentum, dark)
            assertEquals(scheme.onSurfaceVariant, labelInk("#ff6600", scheme, false))
            assertEquals(scheme.onSecondaryContainer, labelInk("#ff6600", scheme, false, selected = true))
            assertEquals(scheme.onSurfaceVariant, labelInk("not a core color", scheme, true))
        }
    }

    @Test fun highContrastTextDisablesCustomLabelsWithoutChangingPreference() {
        assertTrue(colorfulLabelsEnabled(true, false))
        assertFalse(colorfulLabelsEnabled(false, false))
        assertFalse(colorfulLabelsEnabled(true, true))
        assertFalse(colorfulLabelsEnabled(false, true))
    }
}
