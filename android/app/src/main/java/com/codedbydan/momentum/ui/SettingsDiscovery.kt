// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.codedbydan.momentum.*
import com.codedbydan.momentum.R
import com.codedbydan.momentum.core.*

internal val settingsLabels = linkedMapOf(
    "tasks.grouping" to R.string.group, "tasks.sort" to R.string.sort, "tasks.sort-direction" to R.string.ascending,
    "tasks.upcoming" to R.string.days_ahead, "tasks.day-period-visibility" to R.string.show_day_period_tasks_in_today,
    "tasks.archive" to R.string.auto_archive, "tasks.dynamic-messages" to R.string.dynamic_empty_messages,
    "appearance.theme" to R.string.appearance, "appearance.accent" to R.string.accent_color,
    "appearance.colors" to R.string.colorful_labels, "appearance.compact" to R.string.compact,
    "appearance.font" to R.string.font, "appearance.content-size" to R.string.content_text_size,
    "appearance.interface-size" to R.string.interface_text_size, "appearance.reset" to R.string.typography_reset,
    "sounds.enabled" to R.string.completion_sound_enabled, "sounds.selection" to R.string.completion_sound_choice,
    "sounds.volume" to R.string.sound_volume, "notifications.permission" to R.string.enable_notifications,
    "notifications.reminders" to R.string.reminder, "notifications.morning-summary" to R.string.summary,
    "notifications.exact-alarm" to R.string.exact_alarms, "sync.provider" to R.string.sync,
    "sync.nextcloud" to R.string.nextcloud, "sync.libresync" to R.string.libresync,
    "sync.automatic" to R.string.automatic_sync, "sync.background" to R.string.background_sync, "sync.status" to R.string.sync_status_heading, "sync.compression" to R.string.compress,
    "sync.folder-copy" to R.string.folder_migration_action, "sync.devices" to R.string.nearby,
    "sync.settings" to R.string.sync_app_settings,
    "data.summary" to R.string.automation_title, "data.tasks-import" to R.string.import_backup, "data.tasks-export" to R.string.export_backup,
    "data.settings-import" to R.string.import_app_settings, "data.settings-export" to R.string.export_app_settings,
    "integrations.shortcuts" to R.string.keyboard_shortcuts, "integrations.count" to R.string.task_count_title,
    "integrations.system-search" to R.string.system_search, "about.app" to R.string.about,
    "about.source" to R.string.source_code, "about.issues" to R.string.report_issue,
    "about.license" to R.string.license, "about.acknowledgments" to R.string.acknowledgments,
)
internal val settingsCategoryLabels = linkedMapOf("tasks" to R.string.settings_tasks, "appearance" to R.string.appearance,
    "sounds" to R.string.settings_sounds, "notifications" to R.string.notifications, "sync" to R.string.settings_sync,
    "data" to R.string.settings_data, "integrations" to R.string.settings_integrations, "about" to R.string.settings_about)

internal val settingsCategoryDescriptions = mapOf("tasks" to R.string.settings_tasks_description, "appearance" to R.string.settings_appearance_description,
    "sounds" to R.string.settings_sounds_description, "notifications" to R.string.settings_notifications_description,
    "sync" to R.string.settings_sync_description, "data" to R.string.settings_data_description,
    "integrations" to R.string.settings_integrations_description, "about" to R.string.settings_about_description)

@Composable internal fun settingsEntries(): List<SettingsEntry> {
    val resources = androidx.compose.ui.platform.LocalResources.current
    val language = androidx.compose.ui.platform.LocalLocale.current.platformLocale.language
    return settingsDirectory().filter { SettingsPlatform.ANDROID in it.platforms && it.id in settingsLabels && (it.id != "notifications.exact-alarm" || android.os.Build.VERSION.SDK_INT >= 31) }.map {
        SettingsEntry(it.id, resources.getString(settingsCategoryLabels.getValue(it.category)),
            resources.getString(settingsLabels.getValue(it.id)), when(it.id) {
                "sounds.enabled", "sounds.selection", "sounds.volume" -> resources.getString(R.string.completion_sound_hint)
                "integrations.system-search" -> resources.getString(R.string.system_search_hint)
                "notifications.permission", "notifications.reminders" -> resources.getString(R.string.notification_hint)
                "data.tasks-import", "data.settings-import", "data.settings-export" -> resources.getString(R.string.app_settings_scope)
                else -> ""
            }, when(it.id) {
                "appearance.theme" -> listOf(R.string.dynamic_color, R.string.system, R.string.light, R.string.dark).map(resources::getString)
                "tasks.sort-direction" -> listOf(resources.getString(R.string.descending))
                "notifications.morning-summary" -> listOf(resources.getString(R.string.time))
                "integrations.shortcuts" -> listOf(resources.getString(R.string.keyboard_modifier_hint))
                "tasks.day-period-visibility" -> listOf(resources.getString(R.string.morning), resources.getString(R.string.evening), if(language == "de") "Nacht" else "night")
                "appearance.font", "appearance.content-size", "appearance.interface-size" -> listOf(resources.getString(R.string.font), resources.getString(R.string.content_text_size))
                "data.tasks-import", "data.tasks-export", "data.settings-import", "data.settings-export" -> listOf(resources.getString(R.string.backups), resources.getString(R.string.restore))
                else -> emptyList()
            })
    }
}

@Composable internal fun SettingsSearch(state: UiState, vm: MomentumViewModel) {
    OutlinedTextField(state.settingsQuery, vm::searchSettings, Modifier.fillMaxWidth(),
        label = { Text(stringResource(R.string.search_settings)) }, singleLine = true,
        trailingIcon = { if(state.settingsQuery.isNotBlank()) TextButton({ vm.searchSettings("") }) { Text(stringResource(R.string.clear)) } })
    if(state.settingsQuery.isNotBlank()) {
        val matches = searchSettings(state.settingsQuery, settingsEntries())
        if(matches.isEmpty()) Text(stringResource(R.string.no_settings_results))
        Column(Modifier.heightIn(max = 320.dp).verticalScroll(rememberScrollState())) {
        matches.forEach { result -> TextButton({ vm.searchSettings(""); vm.openSettingsDestination(result.id) }, Modifier.fillMaxWidth()) {
            Text("${result.title} · ${result.category}")
        } }
        }
    }
}
@Composable internal fun SettingsBrowse(state: UiState, vm: MomentumViewModel) {
    androidx.activity.compose.BackHandler(state.view == View.Search) { vm.tab(Tab.Search) }
    Column(Modifier.widthIn(max = 720.dp).fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        SettingsSearch(state, vm)
        if(state.settingsQuery.isBlank()) settingsCategoryLabels.forEach { (id, label) ->
            OutlinedButton({ vm.settingsPage(id) }, Modifier.fillMaxWidth().heightIn(min = 56.dp)) {
                Column(Modifier.fillMaxWidth().padding(vertical = 6.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Text(stringResource(label))
                    Text(stringResource(settingsCategoryDescriptions.getValue(id)), style = MaterialTheme.typography.bodySmall)
                }
            }
        }
    }
}
@Composable internal fun PlanningSettings(state: UiState, vm: MomentumViewModel) {
    val prefs = state.snapshot!!.preferences
    SettingHeading(R.string.group)
    GroupBy.entries.forEach { group -> ChoiceRow(stringResource(when(group) {
        GroupBy.MORNING_NIGHT -> R.string.morning_evening; GroupBy.NONE -> R.string.none
        GroupBy.PROJECT -> R.string.project; GroupBy.TAG -> R.string.tags; GroupBy.ESTIMATE -> R.string.time_estimate
    }), prefs.groupBy == group) { vm.preferences { it.copy(groupBy = group) } } }
    SettingHeading(R.string.sort)
    SortKey.entries.forEach { sort -> ChoiceRow(stringResource(when(sort) {
        SortKey.MANUAL -> R.string.manual; SortKey.TITLE -> R.string.title; SortKey.DUE -> R.string.due
        SortKey.ESTIMATE -> R.string.estimate; SortKey.CREATED -> R.string.created
    }), prefs.sort == sort) { vm.preferences { it.copy(sort = sort) } } }
    SettingAnchor(R.string.ascending) { Column {
    ChoiceRow(stringResource(R.string.ascending), prefs.direction == SortDirection.ASCENDING) { vm.preferences { it.copy(direction = SortDirection.ASCENDING) } }
    ChoiceRow(stringResource(R.string.descending), prefs.direction == SortDirection.DESCENDING) { vm.preferences { it.copy(direction = SortDirection.DESCENDING) } }
    } }
    SettingHeading(R.string.days_ahead)
    listOf(7u, 30u).forEach { days -> ChoiceRow(days.toString(), prefs.upcomingDays == days) { vm.preferences { it.copy(upcomingDays = days) } } }
}

/** Revalidate a shared destination without performing any preference or permission action. */
internal fun settingsRoute(id: String): String {
    val category = id.substringBefore('.')
    return category.takeIf { it in settingsCategoryLabels } ?: "main"
}
