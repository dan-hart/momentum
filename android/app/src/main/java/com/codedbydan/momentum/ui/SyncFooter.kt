// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.platform.LocalLocale
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.codedbydan.momentum.MomentumViewModel
import com.codedbydan.momentum.UiState
import com.codedbydan.momentum.R
import com.codedbydan.momentum.data.SyncProvider
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.time.format.FormatStyle

@Composable fun SyncFooter(state: UiState, vm: MomentumViewModel) {
    val locale = LocalLocale.current.platformLocale
    val snapshot = state.snapshot ?: return
    val status by vm.platform.state.collectAsStateWithLifecycle()
    if(snapshot.settings.provider == SyncProvider.Off && status.statusPersistenceError == null) return
    val last = if(snapshot.settings.provider == SyncProvider.Nextcloud) snapshot.sync.lastNextcloudMs else snapshot.sync.lastNearbyMs
    val text = when {
        status.error != null -> stringResource(R.string.sync_failed)
        status.syncing -> stringResource(R.string.syncing)
        last == 0uL -> stringResource(R.string.sync_never)
        else -> stringResource(R.string.last_sync, Instant.ofEpochMilli(last.toLong()).atZone(ZoneId.systemDefault())
            .format(DateTimeFormatter.ofLocalizedDateTime(FormatStyle.SHORT).withLocale(locale)))
    }
    Column(Modifier.fillMaxWidth()) {
        if(snapshot.settings.provider != SyncProvider.Off) {
            Row(Modifier.fillMaxWidth().clickable(onClickLabel = stringResource(R.string.sync), onClick = vm::openSync).padding(horizontal = 20.dp, vertical = 12.dp),
                horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                if(status.syncing) CircularProgressIndicator(Modifier.size(16.dp), strokeWidth = 2.dp)
                Text(text, style = MaterialTheme.typography.labelMedium,
                    color = if(status.error != null) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
        if(status.statusPersistenceError != null) {
            Column(Modifier.padding(horizontal = 20.dp, vertical = 8.dp)) {
                Text(stringResource(R.string.sync_status_save_failed), style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.error)
                TextButton(vm.platform::retryStatusPersistence) { Text(stringResource(R.string.sync_status_save_retry)) }
            }
        }
    }
}
