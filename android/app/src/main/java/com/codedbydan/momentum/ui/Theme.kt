// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import com.codedbydan.momentum.R
import com.codedbydan.momentum.data.AppSettings
import com.codedbydan.momentum.data.Appearance
import com.codedbydan.momentum.data.NativeFont
import com.codedbydan.momentum.data.normalizedNativeTextScale

// Packaged faces keep offline rendering and real bold/italic variants available.
private val atkinsonHyperlegibleMono = FontFamily(
    Font(R.font.atkinson_hyperlegible_mono_regular, FontWeight.Normal, FontStyle.Normal),
    Font(R.font.atkinson_hyperlegible_mono_bold, FontWeight.Bold, FontStyle.Normal),
    Font(R.font.atkinson_hyperlegible_mono_italic, FontWeight.Normal, FontStyle.Italic),
    Font(R.font.atkinson_hyperlegible_mono_bold_italic, FontWeight.Bold, FontStyle.Italic),
)
private val openDyslexic = FontFamily(
    Font(R.font.opendyslexic_regular, FontWeight.Normal, FontStyle.Normal),
    Font(R.font.opendyslexic_bold, FontWeight.Bold, FontStyle.Normal),
    Font(R.font.opendyslexic_italic, FontWeight.Normal, FontStyle.Italic),
    Font(R.font.opendyslexic_bold_italic, FontWeight.Bold, FontStyle.Italic),
)

internal data class NativeTypography(val interfaceStyles: Typography, val contentStyles: Typography)

/** Scale sp values before Android's native conversion; never replace LocalDensity. */
internal fun nativeTypography(settings: AppSettings, base: Typography = Typography()): NativeTypography {
    fun scale(factor: Float): Typography {
        val amount = normalizedNativeTextScale(factor)
        fun TextStyle.scaled(): TextStyle = copy(
            fontFamily = when(settings.font) {
                NativeFont.System -> fontFamily
                NativeFont.Sans -> FontFamily.SansSerif
                NativeFont.Serif -> FontFamily.Serif
                NativeFont.Monospace -> FontFamily.Monospace
                NativeFont.AtkinsonHyperlegibleMono -> atkinsonHyperlegibleMono
                NativeFont.OpenDyslexic -> openDyslexic
            },
            fontSize = if(fontSize.isSp) fontSize * amount else fontSize,
            lineHeight = if(lineHeight.isSp) lineHeight * amount else lineHeight,
            letterSpacing = if(letterSpacing.isSp) letterSpacing * amount else letterSpacing,
        )
        return base.copy(
            displayLarge = base.displayLarge.scaled(), displayMedium = base.displayMedium.scaled(), displaySmall = base.displaySmall.scaled(),
            headlineLarge = base.headlineLarge.scaled(), headlineMedium = base.headlineMedium.scaled(), headlineSmall = base.headlineSmall.scaled(),
            titleLarge = base.titleLarge.scaled(), titleMedium = base.titleMedium.scaled(), titleSmall = base.titleSmall.scaled(),
            bodyLarge = base.bodyLarge.scaled(), bodyMedium = base.bodyMedium.scaled(), bodySmall = base.bodySmall.scaled(),
            labelLarge = base.labelLarge.scaled(), labelMedium = base.labelMedium.scaled(), labelSmall = base.labelSmall.scaled(),
        )
    }
    // Each area starts from the same unscaled base; content never compounds interface scale.
    return NativeTypography(scale(settings.interfaceTextScale), scale(settings.contentTextScale))
}

private val LocalTaskTypography = staticCompositionLocalOf { Typography() }
@Composable fun taskTypography(): Typography = LocalTaskTypography.current

@Composable fun TaskContentText(style: TextStyle = taskTypography().bodyLarge, content: @Composable () -> Unit) {
    ProvideTextStyle(style, content)
}

fun resetNativeTypography(settings: AppSettings): AppSettings = settings.copy(
    font = NativeFont.System, contentTextScale = 1f, interfaceTextScale = 1f,
)

@Composable fun MomentumTheme(settings: AppSettings, content: @Composable () -> Unit) {
    val dark = when(settings.appearance) { Appearance.System -> isSystemInDarkTheme(); Appearance.Light -> false; Appearance.Dark -> true }
    val scheme = if (settings.dynamicColor && Build.VERSION.SDK_INT >= 31) {
        if (dark) dynamicDarkColorScheme(LocalContext.current) else dynamicLightColorScheme(LocalContext.current)
    } else remember(settings.accent, dark) { nativeColorScheme(settings.accent, dark) }
    val typography = remember(settings.font, settings.contentTextScale, settings.interfaceTextScale) { nativeTypography(settings) }
    val colorfulLabels = nativeColorfulLabels(settings.colorfulLabels)
    CompositionLocalProvider(LocalTaskTypography provides typography.contentStyles, LocalColorfulLabels provides colorfulLabels) {
        MaterialTheme(colorScheme = scheme, typography = typography.interfaceStyles, content = content)
    }
}
