// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.*
import androidx.compose.ui.unit.dp
import com.codedbydan.momentum.R
import com.codedbydan.momentum.data.AppSettings
import com.codedbydan.momentum.platform.CompletionSoundChoice

@Composable fun CompletionSoundSettings(
    settings: AppSettings,
    change: ((AppSettings) -> AppSettings) -> Unit,
    preview: (CompletionSoundChoice) -> Unit,
    cancelPreview: () -> Unit,
) {
    DisposableEffect(Unit) { onDispose(cancelPreview) }
    var choosing by rememberSaveable { mutableStateOf(false) }
    SettingHeading(R.string.completion_sound)
    SettingSwitch(R.string.completion_sound_enabled, settings.completionSound) { enabled ->
        change { it.copy(completionSound = enabled) }
    }
    val raw = settings.completionSoundRaw
    val legacy = raw != null && CompletionSoundChoice.candidates.none { it.key == raw }
    if(legacy) Text(stringResource(R.string.legacy_sound) + ": " + raw)
    Text(stringResource(R.string.completion_sound_hint), style = MaterialTheme.typography.bodyMedium)
    SettingAnchor(R.string.completion_sound_choice) { OutlinedButton({ choosing = true }, Modifier.fillMaxWidth().heightIn(min = 48.dp)) {
        Text(stringResource(R.string.completion_sound_choice) + ": " + (if(legacy) raw.orEmpty() else stringResource(settings.completionSoundChoice.label)))
    }
    }
    SettingAnchor(R.string.sound_volume) { Text(stringResource(R.string.sound_volume) + ": ${(settings.completionSoundVolume * 100).toInt()}%") }
    val volumeLabel = stringResource(R.string.sound_volume)
    Slider(settings.completionSoundVolume, { volume -> change { it.copy(completionSoundVolume = volume) } },
        Modifier.semantics { contentDescription = volumeLabel; stateDescription = "${(settings.completionSoundVolume * 100).toInt()}%" })
    if(choosing) AlertDialog(
        onDismissRequest = { choosing = false; cancelPreview() },
        title = { Text(stringResource(R.string.completion_sound_choice)) },
        text = {
            Column(Modifier.heightIn(max = 360.dp).verticalScroll(rememberScrollState()).selectableGroup()) {
                (CompletionSoundChoice.candidates + listOf(settings.completionSoundChoice).filter { it !in CompletionSoundChoice.candidates }).forEach { choice ->
                    val selected = choice == settings.completionSoundChoice && (raw == null || raw == choice.key)
                    Row(Modifier.fillMaxWidth().heightIn(min = 48.dp).selectable(selected, role = Role.RadioButton) {
                        change { it.copy(completionSoundChoice = choice, completionSoundRaw = choice.key) }
                        cancelPreview()
                        choosing = false
                    }, verticalAlignment = Alignment.CenterVertically) {
                        RadioButton(selected, onClick = null)
                        Text(stringResource(choice.label), Modifier.weight(1f).padding(start = 12.dp, top = 10.dp, bottom = 10.dp))
                        val previewLabel = stringResource(R.string.completion_sound_preview) + ": " + stringResource(choice.label)
                        TextButton({ preview(choice) }, Modifier.semantics { contentDescription = previewLabel }) { Text(stringResource(R.string.completion_sound_preview)) }
                    }
                }
            }
        },
        confirmButton = { TextButton({ choosing = false; cancelPreview() }) { Text(stringResource(R.string.cancel)) } },
    )
}
