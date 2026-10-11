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
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import com.codedbydan.momentum.R
import com.codedbydan.momentum.data.AppSettings
import com.codedbydan.momentum.platform.CompletionSoundChoice

@Composable fun CompletionSoundSettings(
    settings: AppSettings,
    change: ((AppSettings) -> AppSettings) -> Unit,
    preview: () -> Unit,
) {
    var choosing by rememberSaveable { mutableStateOf(false) }
    SettingHeading(R.string.completion_sound)
    SettingSwitch(R.string.completion_sound_enabled, settings.completionSound) { enabled ->
        change { it.copy(completionSound = enabled) }
    }
    Text(stringResource(R.string.completion_sound_hint), style = MaterialTheme.typography.bodyMedium)
    OutlinedButton({ choosing = true }, Modifier.fillMaxWidth().heightIn(min = 48.dp), enabled = settings.completionSound) {
        Text(stringResource(R.string.completion_sound_choice) + ": " + stringResource(settings.completionSoundChoice.label))
    }
    OutlinedButton(preview, Modifier.heightIn(min = 48.dp), enabled = settings.completionSound) { Text(stringResource(R.string.completion_sound_preview)) }
    if(choosing && settings.completionSound) AlertDialog(
        onDismissRequest = { choosing = false },
        title = { Text(stringResource(R.string.completion_sound_choice)) },
        text = {
            Column(Modifier.heightIn(max = 360.dp).verticalScroll(rememberScrollState()).selectableGroup()) {
                CompletionSoundChoice.entries.forEach { choice ->
                    val selected = choice == settings.completionSoundChoice
                    Row(Modifier.fillMaxWidth().heightIn(min = 48.dp).selectable(selected, role = Role.RadioButton) {
                        change { it.copy(completionSoundChoice = choice) }
                        choosing = false
                    }, verticalAlignment = Alignment.CenterVertically) {
                        RadioButton(selected, onClick = null)
                        Text(stringResource(choice.label), Modifier.weight(1f).padding(start = 12.dp, top = 10.dp, bottom = 10.dp))
                    }
                }
            }
        },
        confirmButton = { TextButton({ choosing = false }) { Text(stringResource(R.string.cancel)) } },
    )
}
