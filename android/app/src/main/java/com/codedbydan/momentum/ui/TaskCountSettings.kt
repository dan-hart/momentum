// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.provider.Settings
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.codedbydan.momentum.*
import com.codedbydan.momentum.R
import com.codedbydan.momentum.core.TaskCountMode

@Composable fun TaskCountSettings(state: UiState, vm: MomentumViewModel) {
    val mode = state.snapshot?.settings?.taskCountMode ?: return
    val context = LocalContext.current
    val status by vm.platform.state.collectAsStateWithLifecycle()
    var details by remember { mutableStateOf(false) }
    val permission = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) {
        vm.platform.refreshTaskCount(); vm.refresh()
    }
    SettingHeading(R.string.task_count_title)
    TaskCountMode.entries.forEach { choice ->
        ChoiceRow(stringResource(when(choice) {
            TaskCountMode.NONE -> R.string.task_count_off
            TaskCountMode.DUE_TODAY -> R.string.task_count_due_mode
            TaskCountMode.TODAY_INCLUDING_OVERDUE -> R.string.task_count_overdue_mode
        }), mode == choice) {
            vm.settings { it.copy(taskCountMode = choice) }
            if(choice != TaskCountMode.NONE && Build.VERSION.SDK_INT >= 33 &&
                context.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED)
                permission.launch(Manifest.permission.POST_NOTIFICATIONS)
        }
    }
    Text(stringResource(R.string.task_count_hint), style = MaterialTheme.typography.bodyMedium)
    if(mode != TaskCountMode.NONE && status.taskCountBlocked) {
        Text(stringResource(R.string.task_count_denied), color = MaterialTheme.colorScheme.error)
        TextButton({ context.startActivity(Intent(Settings.ACTION_CHANNEL_NOTIFICATION_SETTINGS)
            .putExtra(Settings.EXTRA_APP_PACKAGE, context.packageName)
            .putExtra(Settings.EXTRA_CHANNEL_ID, "momentum.task-count")) }, Modifier.heightIn(min = 48.dp)) {
            Text(stringResource(R.string.settings))
        }
    }
    if(status.taskCountError != null) {
        Text(stringResource(R.string.task_count_failed), color = MaterialTheme.colorScheme.error,
            modifier = Modifier.semantics { liveRegion = LiveRegionMode.Assertive })
        Row {
            TextButton(vm.platform::refreshTaskCount) { Text(stringResource(R.string.retry)) }
            TextButton({ details = !details }) { Text(stringResource(R.string.details)) }
        }
        if(details) Text(status.taskCountError!!, style = MaterialTheme.typography.bodySmall)
    }
}
