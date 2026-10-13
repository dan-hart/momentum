// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.platform.LocalLocale
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import com.codedbydan.momentum.R
import com.codedbydan.momentum.MomentumViewModel
import com.codedbydan.momentum.UiState
import com.codedbydan.momentum.core.SettingsBootstrapChoice
import com.codedbydan.momentum.data.PortableAppSettings
import com.codedbydan.momentum.data.SyncProvider
import com.codedbydan.momentum.platform.CompletionSoundChoice
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.time.format.FormatStyle

@Composable internal fun AppSettingsSync(state: UiState, vm: MomentumViewModel) {
    val snapshot = state.snapshot ?: return
    val status = snapshot.settingsSync ?: return
    var details by remember { mutableStateOf(false) }
    SettingHeading(R.string.app_settings)
    SettingSwitch(R.string.sync_app_settings, status.enabled, !state.busy) { vm.settingsSync(it) }
    Text(stringResource(R.string.settings_sync_hint), style = MaterialTheme.typography.bodyMedium)
    if(status.enabled) {
        if(snapshot.settings.provider == SyncProvider.Off) Text(stringResource(R.string.settings_sync_provider_off))
        Text(stringResource(R.string.settings_sync_pending, status.pendingKeys.toInt()))
        if(status.lastSuccessMs > 0uL) {
            val locale = LocalLocale.current.platformLocale
            val time = Instant.ofEpochMilli(status.lastSuccessMs.toLong()).atZone(ZoneId.systemDefault())
                .format(DateTimeFormatter.ofLocalizedDateTime(FormatStyle.SHORT).withLocale(locale))
            Text(stringResource(R.string.settings_last_sync, time), style = MaterialTheme.typography.bodySmall)
        }
        if(status.bootstrapRequired && status.remoteSettingsJson != null && status.reviewId != null) {
            val displayedId = status.reviewId!!
            Text(stringResource(R.string.settings_sync_review), style = MaterialTheme.typography.titleSmall)
            SettingsSummary(R.string.settings_this_device, PortableAppSettings.encode(snapshot.settings, snapshot.preferences))
            SettingsSummary(R.string.settings_synced_device, status.remoteSettingsJson!!)
            OutlinedButton({ vm.chooseSettings(SettingsBootstrapChoice.USE_LOCAL, displayedId) }, Modifier.fillMaxWidth()) {
                Text(stringResource(R.string.settings_use_local))
            }
            OutlinedButton({ vm.chooseSettings(SettingsBootstrapChoice.USE_REMOTE, displayedId) }, Modifier.fillMaxWidth()) {
                Text(stringResource(R.string.settings_use_remote))
            }
        }
        status.error?.let { error ->
            Text(stringResource(R.string.settings_sync_failed), color = MaterialTheme.colorScheme.error,
                modifier = Modifier.semantics { liveRegion = LiveRegionMode.Polite })
            Row {
                TextButton({ vm.platform.syncNow(); vm.refresh() }, enabled = snapshot.settings.provider != SyncProvider.Off) { Text(stringResource(R.string.retry)) }
                TextButton({ details = !details }) { Text(stringResource(R.string.details)) }
            }
            if(details) Text(error, style = MaterialTheme.typography.bodySmall)
        }
    }
}

/** The concrete review uses native labels, never the transport document. */
@Composable internal fun SettingsSummary(heading: Int, json: String) {
    val values = remember(json) { runCatching { PortableAppSettings.values(json) }.getOrNull() } ?: return
    Column(Modifier.fillMaxWidth().padding(vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text(stringResource(heading), style = MaterialTheme.typography.titleSmall)
        val labels = listOf("grouping" to R.string.group, "sort" to R.string.sort, "direction" to R.string.sort,
            "upcomingDays" to R.string.days_ahead, "autoArchive" to R.string.auto_archive, "rowDensity" to R.string.row_layout,
            "colorfulLabels" to R.string.colorful_labels, "completionSoundEnabled" to R.string.completion_sound_enabled,
            "completionSoundChoice" to R.string.completion_sound_choice)
        labels.forEach { (key, label) ->
            if(values.has(key)) {
                val value = values.get(key)
                val text = when(value) {
                    is Boolean -> stringResource(if(value) R.string.settings_summary_on else R.string.settings_summary_off)
                    is Number -> value.toInt().toString()
                    else -> {
                        val resource = when(value) {
                            "morning-night" -> R.string.morning_evening; "none" -> R.string.none; "project" -> R.string.project
                            "tag" -> R.string.tags; "estimate" -> R.string.estimate; "manual" -> R.string.manual
                            "title" -> R.string.title; "due" -> R.string.due; "created" -> R.string.created
                            "ascending" -> R.string.ascending; "descending" -> R.string.descending
                            "compact" -> R.string.row_compact; "regular" -> R.string.row_regular
                            else -> CompletionSoundChoice.entries.firstOrNull { it.key == value }?.label
                        }
                        if(resource != null) stringResource(resource) else stringResource(R.string.none)
                    }
                }
                Text(stringResource(R.string.settings_summary_item, stringResource(label), text), style = MaterialTheme.typography.bodyMedium)
            }
        }
    }
}
