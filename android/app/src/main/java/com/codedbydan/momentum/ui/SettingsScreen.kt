// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import com.codedbydan.momentum.R

import android.Manifest
import android.app.TimePickerDialog
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.provider.Settings
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.BackHandler
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalLocale
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.semantics.*
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.codedbydan.momentum.*
import com.codedbydan.momentum.core.ClockTime
import com.codedbydan.momentum.data.*
import kotlinx.coroutines.launch
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.time.format.FormatStyle

@Composable fun SettingsScreen(state: UiState, vm: MomentumViewModel) {
    val settings = state.snapshot?.settings ?: return
    val preferences = state.snapshot!!.preferences
    val page = state.settingsPage
    val context = LocalContext.current
    val locale = LocalLocale.current.platformLocale
    val searchStatus by vm.platform.state.collectAsStateWithLifecycle()
    val scope = rememberCoroutineScope()
    var confirmSearch by remember { mutableStateOf(false) }
    var searchDetails by remember { mutableStateOf(false) }
    val notifications = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { allowed ->
        // Retain the explicit opt-in; Android permission remains the delivery gate.
        vm.settings { it.copy(notifications = true) }
    }
    var includeSettings by rememberSaveable { mutableStateOf(false) }
    var restoreSettings by rememberSaveable { mutableStateOf(false) }
    val export = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("application/json")) { it?.let { uri -> vm.export(uri, includeSettings) } }
    val restore = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { it?.let { uri -> vm.prepareImport(uri) } }
    val settingsExport = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("application/json")) { it?.let { uri -> vm.export(uri, settingsOnly = true) } }
    val settingsImport = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { it?.let { uri -> vm.prepareImport(uri, settingsOnly = true) } }
    if(page == "sync") {
        BackHandler { vm.settingsPage("main") }
        Column { TextButton({ vm.settingsPage("main") }) { Text(stringResource(R.string.settings)) }; SyncScreen(state, vm) }
        return
    }
    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
        state.snapshot?.appSettingsError?.let {
            Text(stringResource(R.string.app_settings_apply_failed), color = MaterialTheme.colorScheme.error)
            TextButton(vm::refresh) { Text(stringResource(R.string.retry)) }
        }
        SettingHeading(R.string.appearance)
        Appearance.entries.forEach { appearance -> ChoiceRow(stringResource(when(appearance) {
            Appearance.System -> R.string.system; Appearance.Light -> R.string.light; Appearance.Dark -> R.string.dark
        }), appearance == settings.appearance) { vm.settings { it.copy(appearance = appearance) } } }
        SettingSwitch(R.string.dynamic_color, settings.dynamicColor, Build.VERSION.SDK_INT >= 31) { enabled -> vm.settings { it.copy(dynamicColor = enabled) } }
        AccentSettings(settings, vm::settings)
        SettingSwitch(R.string.colorful_labels, settings.colorfulLabels) { enabled -> vm.settings { it.copy(colorfulLabels = enabled) } }
        Text(stringResource(R.string.colorful_labels_hint), style = MaterialTheme.typography.bodyMedium)
        if(Build.VERSION.SDK_INT >= 36) Text(stringResource(R.string.colorful_labels_high_contrast_hint), style = MaterialTheme.typography.bodyMedium)
        SettingSwitch(R.string.compact, settings.compact) { enabled -> vm.settings { it.copy(compact = enabled) } }
        TypographySettings(settings, vm::settings)
        KeyboardModifierSettings(settings, vm::settings)
        SettingHeading(R.string.lists)
        SettingSwitch(R.string.show_day_period_tasks_in_today, preferences.showDayPeriodTasksInToday) { visible ->
            vm.preferences { it.copy(showDayPeriodTasksInToday = visible) }
        }
        SettingSwitch(R.string.auto_archive, preferences.autoArchive) { enabled -> vm.preferences { it.copy(autoArchive = enabled) } }
        CompletionSoundSettings(settings, vm::settings, vm::previewCompletionSound)
        SettingHeading(R.string.system_search)
        SettingSwitch(R.string.system_search_enabled, settings.systemSearch,
            Build.VERSION.SDK_INT >= 31 && (!searchStatus.searchUpdating || settings.systemSearch)) { enabled ->
            if(enabled) confirmSearch = true
            else scope.launch { vm.platform.setSystemSearch(false); vm.refresh() }
        }
        Text(stringResource(R.string.system_search_hint), style = MaterialTheme.typography.bodyMedium)
        if(Build.VERSION.SDK_INT < 31) Text(stringResource(R.string.system_search_unavailable))
        if(searchStatus.searchUpdating) LinearProgressIndicator(Modifier.fillMaxWidth())
        if(searchStatus.searchError != null) {
            Text(stringResource(R.string.system_search_failed), color = MaterialTheme.colorScheme.error,
                modifier = Modifier.semantics { liveRegion = LiveRegionMode.Assertive })
            Row {
                TextButton(vm.platform::retrySystemSearch) { Text(stringResource(R.string.retry)) }
                TextButton({ searchDetails = !searchDetails }) { Text(stringResource(R.string.details)) }
            }
            if(searchDetails) Text(searchStatus.searchError!!, style = MaterialTheme.typography.bodySmall)
        } else if(searchStatus.searchUpdatedAt > 0) {
            if(!settings.systemSearch && searchStatus.searchCleared) Text(stringResource(R.string.system_search_removed))
            else Text(pluralStringResource(R.plurals.system_search_last_update, searchStatus.searchIndexed,
                Instant.ofEpochMilli(searchStatus.searchUpdatedAt).atZone(ZoneId.systemDefault()).format(DateTimeFormatter.ofLocalizedDateTime(FormatStyle.SHORT).withLocale(locale)),
                searchStatus.searchIndexed), style = MaterialTheme.typography.bodySmall)
        }
        SettingHeading(R.string.sync)
        OutlinedButton(vm::openSync, Modifier.fillMaxWidth()) { Text(stringResource(R.string.sync)) }
        SettingHeading(R.string.notifications)
        SettingSwitch(R.string.enable_notifications, settings.notifications) { enabled ->
            if(enabled && Build.VERSION.SDK_INT >= 33 && context.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
                notifications.launch(Manifest.permission.POST_NOTIFICATIONS)
            } else vm.settings { it.copy(notifications = enabled) }
        }
        Text(stringResource(R.string.notification_hint), style = MaterialTheme.typography.bodyMedium)
        if(settings.notifications && (!androidx.core.app.NotificationManagerCompat.from(context).areNotificationsEnabled() ||
            (Build.VERSION.SDK_INT >= 33 && context.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED))) {
            Text(stringResource(R.string.notification_denied), color = MaterialTheme.colorScheme.error)
            TextButton({ context.startActivity(Intent(Settings.ACTION_APP_NOTIFICATION_SETTINGS).putExtra(Settings.EXTRA_APP_PACKAGE, context.packageName)) }) { Text(stringResource(R.string.settings)) }
        }
        if(Build.VERSION.SDK_INT >= 31) TextButton({
            context.startActivity(Intent(Settings.ACTION_REQUEST_SCHEDULE_EXACT_ALARM, Uri.parse("package:${context.packageName}")))
        }) { Text(stringResource(R.string.exact_alarms)) }
        if(Build.VERSION.SDK_INT >= 31 && !context.getSystemService(android.app.AlarmManager::class.java).canScheduleExactAlarms())
            Text(stringResource(R.string.exact_hint), style = MaterialTheme.typography.bodyMedium)
        SettingSwitch(R.string.summary, preferences.morningSummaryEnabled) { enabled -> vm.preferences { it.copy(morningSummaryEnabled = enabled) } }
        TextButton({ TimePickerDialog(context, { _, hour, minute -> vm.preferences { it.copy(morningSummaryTime = ClockTime(hour.toUInt(), minute.toUInt())) } },
            preferences.morningSummaryTime.hour.toInt(), preferences.morningSummaryTime.minute.toInt(), android.text.format.DateFormat.is24HourFormat(context)).show()
        }) { Text(stringResource(R.string.time) + ": " + clockText(preferences.morningSummaryTime)) }
        TaskCountSettings(state, vm)
        AutomationSettings()
        SettingHeading(R.string.backups)
        if(state.importPreparing) {
            Text(stringResource(R.string.backup_reading), modifier = Modifier.semantics { liveRegion = LiveRegionMode.Polite })
            LinearProgressIndicator(Modifier.fillMaxWidth())
            TextButton(vm::cancelImport, enabled = !state.restoring) { Text(stringResource(R.string.cancel)) }
        }
        TransferCheck(R.string.include_app_settings, includeSettings, !state.busy) { includeSettings = it }
        OutlinedButton({ export.launch("momentum-backup.json") }, Modifier.fillMaxWidth()) { Text(stringResource(R.string.export_backup)) }
        OutlinedButton({ restore.launch(arrayOf("application/json", "text/plain")) }, Modifier.fillMaxWidth(), enabled = state.editor == null) { Text(stringResource(R.string.import_backup)) }
        Text(stringResource(R.string.app_settings_scope), style = MaterialTheme.typography.bodyMedium)
        OutlinedButton({ settingsExport.launch("momentum-settings.json") }, Modifier.fillMaxWidth(), enabled = !state.busy) { Text(stringResource(R.string.export_app_settings)) }
        OutlinedButton({ settingsImport.launch(arrayOf("application/json", "text/plain")) }, Modifier.fillMaxWidth(), enabled = !state.busy && !state.importPreparing) { Text(stringResource(R.string.import_app_settings)) }
        SettingHeading(R.string.about)
        Text(stringResource(R.string.about_text), style = MaterialTheme.typography.bodyMedium)
        Spacer(Modifier.height(32.dp))
    }
    if(confirmSearch) AlertDialog(onDismissRequest = { confirmSearch = false },
        title = { Text(stringResource(R.string.system_search_confirm_title)) },
        text = { Text(stringResource(R.string.system_search_confirm_hint)) },
        confirmButton = { TextButton({ confirmSearch = false; scope.launch { vm.platform.setSystemSearch(true); vm.refresh() } }) {
            Text(stringResource(R.string.system_search_confirm))
        } }, dismissButton = { TextButton({ confirmSearch = false }) { Text(stringResource(R.string.cancel)) } })
    if(state.importReady) AlertDialog(onDismissRequest = vm::cancelImport, title = { Text(stringResource(R.string.restore_title)) },
        text = { Column { Text(stringResource(if(state.editor == null) R.string.restore_hint else R.string.draft_restore_hint)); TransferCheck(R.string.restore_app_settings, restoreSettings, !state.restoring) { restoreSettings = it }; if(state.restoring) LinearProgressIndicator(Modifier.fillMaxWidth().padding(top = 12.dp)) } },
        confirmButton = { TextButton({ vm.restore(restoreSettings) }, enabled = state.editor == null && !state.editorLoading && !state.busy) { Text(stringResource(R.string.restore)) } },
        dismissButton = { TextButton(vm::cancelImport, enabled = !state.restoring) { Text(stringResource(R.string.cancel)) } })
}

@Composable fun SettingHeading(label: Int) = Text(stringResource(label), Modifier.padding(top = 16.dp).semantics { heading() }, style = MaterialTheme.typography.titleMedium)
@Composable fun SettingSwitch(label: Int, checked: Boolean, enabled: Boolean = true, change: (Boolean) -> Unit) {
    val title = stringResource(label)
    ListItem(headlineContent = { Text(title) }, trailingContent = {
        Switch(checked, change, enabled = enabled, modifier = Modifier.sizeIn(minHeight = 48.dp).semantics { contentDescription = title })
    })
}

@Composable internal fun TransferCheck(label: Int, checked: Boolean, enabled: Boolean = true, change: (Boolean) -> Unit) {
    val title = stringResource(label)
    ListItem(headlineContent = { Text(title) }, leadingContent = {
        Checkbox(checked, change, enabled = enabled, modifier = Modifier.sizeIn(minWidth = 48.dp, minHeight = 48.dp).semantics { contentDescription = title })
    })
}
