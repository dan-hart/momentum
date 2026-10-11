// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import com.codedbydan.momentum.R

import android.content.pm.PackageManager
import android.os.Build
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalLocale
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.codedbydan.momentum.*
import com.codedbydan.momentum.core.NextcloudSettings
import com.codedbydan.momentum.core.SyncFailureKind
import com.codedbydan.momentum.data.SyncProvider
import kotlinx.coroutines.launch
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.time.format.FormatStyle

@Composable fun SyncScreen(state: UiState, vm: MomentumViewModel) {
    val settings = state.snapshot?.settings ?: return
    val context = LocalContext.current
    val locale = LocalLocale.current.platformLocale
    val platform = vm.platform
    val status by platform.state.collectAsStateWithLifecycle()
    val scope = rememberCoroutineScope()
    var server by remember { mutableStateOf("") }
    var user by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    var folder by remember { mutableStateOf("momentum") }
    var encryption by remember { mutableStateOf("") }
    var compress by remember { mutableStateOf(true) }
    var address by remember { mutableStateOf("") }
    var code by remember { mutableStateOf("") }
    var detail by remember { mutableStateOf(false) }
    var pendingProvider by remember { mutableStateOf<SyncProvider?>(null) }
    var localError by remember { mutableStateOf(false) }
    var unlink by remember { mutableStateOf<String?>(null) }
    var discovery by remember { mutableStateOf(false) }
    var replace by remember { mutableStateOf(false) }
    val permission = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { allowed ->
        val requested = pendingProvider; pendingProvider = null
        localError = !allowed
        if(allowed) scope.launch {
            if(requested != null) platform.selectProvider(requested)
            platform.foreground(true); vm.refresh()
        }
    }
    LaunchedEffect(Unit) {
        platform.connection()?.let { server = it.serverUrl; user = it.userName; password = it.password; folder = it.folder; compress = it.compress; encryption = it.encryptionPassword.orEmpty() }
    }
    DisposableEffect(Unit) { onDispose { platform.pairing(false) } }
    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        SettingHeading(R.string.provider)
        SyncProvider.entries.forEach { provider ->
            ChoiceRow(stringResource(when(provider) { SyncProvider.Off -> R.string.off; SyncProvider.Nextcloud -> R.string.nextcloud; SyncProvider.LibreSync -> R.string.libresync }),
                settings.provider == provider, !status.syncing) {
                if(provider == SyncProvider.LibreSync && Build.VERSION.SDK_INT >= 37 && context.checkSelfPermission("android.permission.ACCESS_LOCAL_NETWORK") != PackageManager.PERMISSION_GRANTED) {
                    pendingProvider = provider; permission.launch("android.permission.ACCESS_LOCAL_NETWORK")
                } else scope.launch { platform.selectProvider(provider); vm.refresh() }
            }
        }
        if(localError) Text(stringResource(R.string.local_network_denied), color = MaterialTheme.colorScheme.error)
        if(status.statusPersistenceError != null) {
            Text(stringResource(R.string.sync_status_save_failed), color = MaterialTheme.colorScheme.error)
            OutlinedButton(platform::retryStatusPersistence) { Text(stringResource(R.string.sync_status_save_retry)) }
        }
        if(settings.provider != SyncProvider.Off) {
            if(status.syncing) LinearProgressIndicator(Modifier.fillMaxWidth())
            status.error?.let { error ->
                Text(stringResource(syncRemedy(status.failureKind)), color = MaterialTheme.colorScheme.error)
                TextButton({ detail = !detail }) { Text(stringResource(R.string.details)) }
                if(detail) Text(status.errorDetails ?: error, style = MaterialTheme.typography.bodySmall)
                if(settings.provider == SyncProvider.Nextcloud && status.failureKind == SyncFailureKind.REMOTE_FILE_DAMAGED)
                    OutlinedButton({ replace = true }, enabled = !status.syncing) { Text(stringResource(R.string.replace_server)) }
            }
            status.information?.let { Text(stringResource(it)) }
            val sync = state.snapshot!!.sync
            val last = if(settings.provider == SyncProvider.Nextcloud) sync.lastNextcloudMs else sync.lastNearbyMs
            Text(if(last == 0uL) stringResource(R.string.sync_never) else stringResource(R.string.last_sync,
                Instant.ofEpochMilli(last.toLong()).atZone(ZoneId.systemDefault()).format(DateTimeFormatter.ofLocalizedDateTime(FormatStyle.SHORT).withLocale(locale))))
            Text(pluralStringResource(R.plurals.pending_changes, sync.pendingOps.toInt(), sync.pendingOps.toInt()))
            Button({ platform.syncNow(); vm.refresh() }, enabled = !status.syncing, modifier = Modifier.fillMaxWidth()) { Text(stringResource(R.string.sync_now)) }
        }
        if(settings.provider == SyncProvider.Nextcloud) {
            OutlinedTextField(server, { server = it }, label = { Text(stringResource(R.string.server)) }, modifier = Modifier.fillMaxWidth(), singleLine = true)
            OutlinedTextField(user, { user = it }, label = { Text(stringResource(R.string.username)) }, modifier = Modifier.fillMaxWidth(), singleLine = true)
            OutlinedTextField(password, { password = it }, label = { Text(stringResource(R.string.password)) }, modifier = Modifier.fillMaxWidth(), visualTransformation = PasswordVisualTransformation(), singleLine = true,
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password, autoCorrectEnabled = false))
            OutlinedTextField(folder, { folder = it }, label = { Text(stringResource(R.string.folder)) }, modifier = Modifier.fillMaxWidth(), singleLine = true)
            OutlinedTextField(encryption, { encryption = it }, label = { Text(stringResource(R.string.encryption_password)) }, modifier = Modifier.fillMaxWidth(), visualTransformation = PasswordVisualTransformation(), singleLine = true,
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password, autoCorrectEnabled = false))
            SettingSwitch(R.string.compress, compress) { compress = it }
            OutlinedButton({ scope.launch { platform.saveConnection(NextcloudSettings(server.trim(), user.trim(), password, folder.trim(), compress, encryption.takeIf { it.isNotEmpty() })); vm.refresh() } },
                Modifier.fillMaxWidth(), enabled = server.isNotBlank() && user.isNotBlank() && password.isNotEmpty() && folder.isNotBlank()) { Text(stringResource(R.string.save_connection)) }
            OutlinedButton({ platform.syncNow(test = true) }, Modifier.fillMaxWidth(), enabled = !status.syncing) { Text(stringResource(R.string.test_connection)) }
            SettingSwitch(R.string.automatic_sync, settings.automaticSync) { enabled -> vm.settings { it.copy(automaticSync = enabled) } }
            SettingSwitch(R.string.background_sync, settings.backgroundSync) { enabled -> vm.settings { it.copy(backgroundSync = enabled) } }
            if(Build.VERSION.SDK_INT >= 37) OutlinedButton({ permission.launch("android.permission.ACCESS_LOCAL_NETWORK") }, Modifier.fillMaxWidth()) { Text(stringResource(R.string.allow_local_network)) }
        }
        if(settings.provider == SyncProvider.LibreSync) {
            Text(stringResource(R.string.nearby_foreground), style = MaterialTheme.typography.bodyMedium)
            Button({ discovery = !discovery; platform.pairing(discovery) }, Modifier.fillMaxWidth()) { Text(stringResource(if(discovery) R.string.close else R.string.nearby)) }
            status.pairingCode?.let { Text(stringResource(R.string.pairing_code, it), style = MaterialTheme.typography.titleLarge) }
            status.devices.forEach { device -> ListItem(headlineContent = { Text(device.name) }, trailingContent = {
                TextButton({ unlink = device.deviceId }) { Text(stringResource(R.string.unlink)) }
            }) }
            if(discovery) {
                status.discovered.forEach { device -> TextButton({ address = device.address.orEmpty() }) { Text(device.name) } }
                OutlinedTextField(address, { address = it }, label = { Text(stringResource(R.string.device_address)) }, modifier = Modifier.fillMaxWidth())
                OutlinedTextField(code, { code = it }, label = { Text(stringResource(R.string.pair_code)) }, modifier = Modifier.fillMaxWidth())
                Button({ scope.launch { platform.link(address.trim(), code.trim()); vm.refresh() } }, enabled = address.isNotBlank() && code.matches(Regex("[0-9]{6}")), modifier = Modifier.fillMaxWidth()) { Text(stringResource(R.string.link)) }
            }
        }
        Spacer(Modifier.height(24.dp))
    }
    unlink?.let { id -> AlertDialog(onDismissRequest = { unlink = null }, title = { Text(stringResource(R.string.unlink)) },
        text = { Text(stringResource(R.string.unlink_hint)) }, confirmButton = { TextButton({ scope.launch { platform.unlink(id); vm.refresh() }; unlink = null }) { Text(stringResource(R.string.unlink)) } },
        dismissButton = { TextButton({ unlink = null }) { Text(stringResource(R.string.cancel)) } }) }
    if(replace) AlertDialog(onDismissRequest = { replace = false }, title = { Text(stringResource(R.string.replace_server_title)) },
        text = { Text(stringResource(R.string.replace_server_hint)) },
        confirmButton = { TextButton({ platform.replaceServerCopy(); replace = false }, enabled = !status.syncing && status.failureKind == SyncFailureKind.REMOTE_FILE_DAMAGED) { Text(stringResource(R.string.replace_server_confirm)) } },
        dismissButton = { TextButton({ replace = false }) { Text(stringResource(R.string.cancel)) } })
}

private fun syncRemedy(kind: SyncFailureKind?): Int = when(kind) {
    SyncFailureKind.UNREACHABLE -> R.string.sync_unreachable
    SyncFailureKind.UNAUTHORIZED -> R.string.sync_unauthorized
    SyncFailureKind.SERVER_ERROR -> R.string.sync_server_error
    SyncFailureKind.TIMED_OUT -> R.string.sync_timed_out
    SyncFailureKind.ENCRYPTION_PASSWORD_MISSING -> R.string.sync_encryption_missing
    SyncFailureKind.ENCRYPTION_PASSWORD_WRONG -> R.string.sync_encryption_wrong
    SyncFailureKind.REMOTE_FILE_DAMAGED -> R.string.sync_damaged
    SyncFailureKind.INCOMPATIBLE -> R.string.sync_incompatible
    SyncFailureKind.NOTHING_TO_START_FROM -> R.string.sync_empty
    SyncFailureKind.CONFLICT -> R.string.sync_conflict
    SyncFailureKind.CANCELLED -> R.string.sync_cancelled
    SyncFailureKind.UPLOAD_INCOMPLETE -> R.string.sync_upload_incomplete
    SyncFailureKind.OTHER, null -> R.string.sync_failed
}
