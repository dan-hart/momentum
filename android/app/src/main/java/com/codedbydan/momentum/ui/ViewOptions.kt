// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import com.codedbydan.momentum.R

import androidx.compose.foundation.clickable
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.compose.ui.semantics.Role
import com.codedbydan.momentum.*
import com.codedbydan.momentum.core.*

@Composable fun ViewOptions(state: UiState, vm: MomentumViewModel, importText: () -> Unit, dismiss: () -> Unit) {
    val snapshot = state.snapshot ?: return
    val prefs = snapshot.preferences
    AlertDialog(onDismissRequest = dismiss, title = { Text(stringResource(R.string.options)) },
        text = { Column(Modifier.verticalScroll(rememberScrollState())) {
            Text(stringResource(R.string.row_layout), style = MaterialTheme.typography.titleSmall)
            ChoiceRow(stringResource(R.string.row_regular), !snapshot.settings.compact) { vm.settings { it.copy(compact = false) } }
            ChoiceRow(stringResource(R.string.row_compact), snapshot.settings.compact) { vm.settings { it.copy(compact = true) } }
            Text(stringResource(R.string.group), style = MaterialTheme.typography.titleSmall)
            GroupBy.entries.forEach { group ->
                ChoiceRow(stringResource(when(group) { GroupBy.MORNING_NIGHT -> R.string.morning_evening; GroupBy.NONE -> R.string.none;
                    GroupBy.PROJECT -> R.string.project; GroupBy.TAG -> R.string.tags; GroupBy.ESTIMATE -> R.string.time_estimate }),
                    prefs.groupBy == group, state.view != View.Upcoming) { vm.preferences { it.copy(groupBy = group) } }
            }
            Text(stringResource(R.string.sort), style = MaterialTheme.typography.titleSmall)
            SortKey.entries.forEach { sort ->
                ChoiceRow(stringResource(when(sort) { SortKey.MANUAL -> R.string.manual; SortKey.TITLE -> R.string.title;
                    SortKey.DUE -> R.string.due; SortKey.ESTIMATE -> R.string.estimate; SortKey.CREATED -> R.string.created }), prefs.sort == sort) {
                    vm.preferences { it.copy(sort = sort) }
                }
            }
            ChoiceRow(stringResource(R.string.ascending), prefs.direction == SortDirection.ASCENDING) { vm.preferences { it.copy(direction = SortDirection.ASCENDING) } }
            ChoiceRow(stringResource(R.string.descending), prefs.direction == SortDirection.DESCENDING) { vm.preferences { it.copy(direction = SortDirection.DESCENDING) } }
            if(state.view == View.Upcoming) { Text(stringResource(R.string.days_ahead)); listOf(7u, 30u).forEach { days ->
                ChoiceRow(days.toString(), prefs.upcomingDays == days) { vm.preferences { it.copy(upcomingDays = days) } }
            } }
            TextButton(onClick = { vm.selectAll(); dismiss() }, enabled = state.view != View.Archive) { Text(stringResource(R.string.select_tasks)) }
            TextButton(onClick = { vm.action { archiveDone() }; dismiss() }, enabled = snapshot.listing.canArchive) { Text(stringResource(R.string.archive_completed)) }
            TextButton(onClick = { vm.action { undo() }; dismiss() }, enabled = snapshot.canUndo) { Text(stringResource(R.string.undo)) }
            TextButton(onClick = { dismiss(); importText() }, enabled = state.view != View.Archive) { Text(stringResource(R.string.import_text)) }
        } }, confirmButton = { TextButton(dismiss) { Text(stringResource(R.string.close)) } })
}

@Composable fun ChoiceRow(label: String, selected: Boolean, enabled: Boolean = true, click: () -> Unit) {
    Row(Modifier.fillMaxWidth().selectable(selected, enabled = enabled, role = Role.RadioButton, onClick = click).padding(vertical = 4.dp)) {
        RadioButton(selected, onClick = null, enabled = enabled)
        Text(label, Modifier.padding(12.dp), color = if(enabled) MaterialTheme.colorScheme.onSurface else MaterialTheme.colorScheme.onSurfaceVariant)
    }
}
