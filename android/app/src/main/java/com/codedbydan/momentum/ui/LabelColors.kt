// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import android.os.Build
import android.view.accessibility.AccessibilityManager
import androidx.annotation.RequiresApi
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.*
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext

internal val LocalColorfulLabels = staticCompositionLocalOf { true }
private val coreColorPattern = Regex("#[0-9a-fA-F]{6}")

internal fun colorfulLabelsEnabled(preference: Boolean, highContrastText: Boolean): Boolean =
    preference && !highContrastText

/** Rust owns normalization; its native color contract is exactly #rrggbb. */
internal fun coreLabelColor(hex: String?): Color? {
    if(hex == null || !hex.matches(coreColorPattern)) return null
    return Color(0xff000000L or hex.substring(1).toLong(16))
}

internal fun labelInk(hex: String?, scheme: ColorScheme, enabled: Boolean, selected: Boolean = false): Color {
    val fallback = if(selected) scheme.onSecondaryContainer else scheme.onSurfaceVariant
    val seed = coreLabelColor(hex)?.takeIf { enabled } ?: return fallback
    val surfaces = if(selected) listOf(scheme.secondaryContainer) else accentSurfaces(scheme)
    return readableAccent(seed, surfaces)
}

@Composable internal fun nativeLabelInk(hex: String?, selected: Boolean = false): Color {
    val scheme = MaterialTheme.colorScheme
    val enabled = LocalColorfulLabels.current
    return remember(hex, scheme, enabled, selected) { labelInk(hex, scheme, enabled, selected) }
}

@Composable internal fun nativeColorfulLabels(preference: Boolean): Boolean {
    val highContrast = if(Build.VERSION.SDK_INT >= 36) highContrastTextEnabled() else false
    return colorfulLabelsEnabled(preference, highContrast)
}

@RequiresApi(36)
@Composable private fun highContrastTextEnabled(): Boolean {
    val context = LocalContext.current
    val manager = remember(context) { context.getSystemService(AccessibilityManager::class.java) }
    var enabled by remember(manager) { mutableStateOf(manager.isHighContrastTextEnabled) }
    DisposableEffect(manager) {
        val listener = AccessibilityManager.HighContrastTextStateChangeListener { enabled = it }
        manager.addHighContrastTextStateChangeListener(context.mainExecutor, listener)
        // Cover a setting change between initial composition and listener registration.
        enabled = manager.isHighContrastTextEnabled
        onDispose { manager.removeHighContrastTextStateChangeListener(listener) }
    }
    return enabled
}
