// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import com.codedbydan.momentum.R
import com.codedbydan.momentum.data.AppSettings
import com.codedbydan.momentum.data.NativeFont
import com.codedbydan.momentum.data.normalizedNativeTextScale
import kotlin.math.roundToInt

@Composable fun TypographySettings(settings: AppSettings, change: ((AppSettings) -> AppSettings) -> Unit) {
    SettingHeading(R.string.typography)
    Text(stringResource(R.string.typography_hint), style = MaterialTheme.typography.bodyMedium)
    Text(stringResource(R.string.font), style = MaterialTheme.typography.titleSmall)
    NativeFont.entries.forEach { choice ->
        val label = stringResource(when(choice) {
            NativeFont.System -> R.string.font_system
            NativeFont.Sans -> R.string.font_sans
            NativeFont.Serif -> R.string.font_serif
            NativeFont.Monospace -> R.string.font_monospace
        })
        Row(Modifier.fillMaxWidth().heightIn(min = 48.dp)
            .selectable(settings.font == choice, role = Role.RadioButton) { change { it.copy(font = choice) } },
            verticalAlignment = Alignment.CenterVertically) {
            RadioButton(selected = settings.font == choice, onClick = null)
            Text(label, Modifier.padding(horizontal = 12.dp, vertical = 8.dp))
        }
    }
    TypographyScale(R.string.content_text_size, settings.contentTextScale) { scale -> change { it.copy(contentTextScale = scale) } }
    TypographyScale(R.string.interface_text_size, settings.interfaceTextScale) { scale -> change { it.copy(interfaceTextScale = scale) } }
    Surface(color = MaterialTheme.colorScheme.surfaceContainer, shape = MaterialTheme.shapes.medium) {
        Column(Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(stringResource(R.string.interface_text_size), style = MaterialTheme.typography.labelLarge)
            Text(stringResource(R.string.typography_preview), style = MaterialTheme.typography.bodyLarge)
            Text(stringResource(R.string.content_text_size), style = MaterialTheme.typography.labelLarge)
            TaskContentText { Text(stringResource(R.string.typography_preview)) }
        }
    }
    OutlinedButton(onClick = { change(::resetNativeTypography) }, modifier = Modifier.heightIn(min = 48.dp)) {
        Text(stringResource(R.string.typography_reset))
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable private fun TypographyScale(label: Int, value: Float, commit: (Float) -> Unit) {
    val current = normalizedNativeTextScale(value)
    var preview by remember(current) { mutableFloatStateOf(current) }
    val title = stringResource(label)
    val interaction = remember { MutableInteractionSource() }
    Column {
        Text(title, style = MaterialTheme.typography.titleSmall)
        Text(stringResource(R.string.text_scale_percent, (preview * 100).roundToInt()), style = MaterialTheme.typography.bodyMedium)
        Slider(value = preview, onValueChange = { preview = (it * 20).roundToInt() / 20f },
            onValueChangeFinished = { commit(preview) }, valueRange = 0.75f..2f, steps = 24,
            interactionSource = interaction,
            // The native range node measures its thumb/track, rather than an outer min-height.
            thumb = { SliderDefaults.Thumb(interactionSource = interaction, modifier = Modifier.heightIn(min = 48.dp)) },
            modifier = Modifier.fillMaxWidth().semantics { contentDescription = title })
    }
}
