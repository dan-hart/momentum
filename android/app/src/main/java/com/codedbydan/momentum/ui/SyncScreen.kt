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
    var upgrade by remember { mutableStateOf(false) }
    var upgradeIdentity by remember { mutableStateOf("") }
    var storedConnection by remember { mutableStateOf<NextcloudSettings?>(null) }
    var migrationSource by remember { mutableStateOf<NextcloudSettings?>(null) }
    var migrationError by remember { mutableStateOf(false) }
    val permission = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { allowed ->
        val requested = pendingProvider; pendingProvider = null
        localError = !allowed
        if(allowed) scope.launch {
            if(requested != null) platform.selectProvider(requested)
            platform.foreground(true); vm.refresh()
        }
    }
    LaunchedEffect(Unit) {
        platform.connection()?.let { storedConnection = it; server = it.serverUrl; user = it.userName; password = it.password; folder = it.folder; compress = it.compress; encryption = it.encryptionPassword.orEmpty() }
    }
    val sameSavedTarget = storedConnection?.let {
        com.codedbydan.momentum.core.nextcloudConnectionIdentity(server, user, folder) ==
            com.codedbydan.momentum.core.nextcloudConnectionIdentity(it.serverUrl, it.userName, it.folder)
    } == true
    val optimized = sameSavedTarget && storedConnection?.optimized == true
    DisposableEffect(Unit) { onDispose { platform.pairing(false); platform.cancelFolderMigration() } }
    Column(Modifier.widthIn(max = 720.dp).fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        if((state.settingsDestination in listOf("sync.nextcloud", "sync.automatic", "sync.background", "sync.compression", "sync.folder-copy") && settings.provider != SyncProvider.Nextcloud) ||
            (state.settingsDestination in listOf("sync.libresync", "sync.devices") && settings.provider != SyncProvider.LibreSync)) Text(stringResource(R.string.inactive_provider_hint))
        SettingHeading(R.string.provider)
        SyncProvider.entries.forEach { provider ->
            ChoiceRow(stringResource(when(provider) { SyncProvider.Off -> R.string.off; SyncProvider.Nextcloud -> R.string.nextcloud; SyncProvider.LibreSync -> R.string.libresync }),
                settings.provider == provider, !status.syncing && !status.migratingFolder) {
                if(provider == SyncProvider.LibreSync && Build.VERSION.SDK_INT >= 37 && context.checkSelfPermission("android.permission.ACCESS_LOCAL_NETWORK") != PackageManager.PERMISSION_GRANTED) {
                    pendingProvider = provider; permission.launch("android.permission.ACCESS_LOCAL_NETWORK")
                } else scope.launch { platform.selectProvider(provider); vm.refresh() }
            }
        }
        AppSettingsSync(state, vm)
        if(localError) Text(stringResource(R.string.local_network_denied), color = MaterialTheme.colorScheme.error)
        if(status.statusPersistenceError != null) {
            Text(stringResource(R.string.sync_status_save_failed), color = MaterialTheme.colorScheme.error)
            OutlinedButton(platform::retryStatusPersistence) { Text(stringResource(R.string.sync_status_save_retry)) }
        }
        if(settings.provider != SyncProvider.Off) {
            SettingHeading(R.string.sync_status_heading)
            if(status.syncing) LinearProgressIndicator(Modifier.fillMaxWidth())
            status.error?.let { error ->
                Text(stringResource(syncRemedy(status.failureKind)), color = MaterialTheme.colorScheme.error)
                TextButton({ detail = !detail }) { Text(stringResource(R.string.details)) }
                if(detail) Text(status.errorDetails ?: error, style = MaterialTheme.typography.bodySmall)
                if(settings.provider == SyncProvider.Nextcloud && status.failureKind == SyncFailureKind.REMOTE_FILE_DAMAGED)
                    OutlinedButton({ replace = true }, enabled = !status.syncing && !status.migratingFolder) { Text(stringResource(R.string.replace_server)) }
            }
            status.information?.let { Text(stringResource(it)) }
            val sync = state.snapshot!!.sync
            val last = if(settings.provider == SyncProvider.Nextcloud) sync.lastNextcloudMs else sync.lastNearbyMs
            Text(if(last == 0uL) stringResource(R.string.sync_never) else stringResource(R.string.last_sync,
                Instant.ofEpochMilli(last.toLong()).atZone(ZoneId.systemDefault()).format(DateTimeFormatter.ofLocalizedDateTime(FormatStyle.SHORT).withLocale(locale))))
            Text(pluralStringResource(R.plurals.pending_changes, sync.pendingOps.toInt(), sync.pendingOps.toInt()))
            Button({ platform.syncNow(); vm.refresh() }, enabled = !status.syncing && !status.migratingFolder, modifier = Modifier.fillMaxWidth()) { Text(stringResource(R.string.sync_now)) }
        }
        if(settings.provider == SyncProvider.Nextcloud) {
            SettingHeading(R.string.nextcloud)
            OutlinedTextField(server, { server = it }, label = { Text(stringResource(R.string.server)) }, modifier = Modifier.fillMaxWidth(), singleLine = true)
            OutlinedTextField(user, { user = it }, label = { Text(stringResource(R.string.username)) }, modifier = Modifier.fillMaxWidth(), singleLine = true)
            OutlinedTextField(password, { password = it }, label = { Text(stringResource(R.string.password)) }, modifier = Modifier.fillMaxWidth(), visualTransformation = PasswordVisualTransformation(), singleLine = true,
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password, autoCorrectEnabled = false))
            OutlinedTextField(folder, { folder = it }, label = { Text(stringResource(R.string.folder)) }, modifier = Modifier.fillMaxWidth(), singleLine = true)
            OutlinedTextField(encryption, { encryption = it }, label = { Text(stringResource(R.string.encryption_password)) }, modifier = Modifier.fillMaxWidth(), visualTransformation = PasswordVisualTransformation(), singleLine = true,
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password, autoCorrectEnabled = false))
            SettingSwitch(R.string.compress, compress) { compress = it }
            if (optimized) Text(stringResource(R.string.optimized_sync_enabled))
            else OutlinedButton({ upgradeIdentity = com.codedbydan.momentum.core.nextcloudConnectionIdentity(server, user, folder); upgrade = true }, enabled = sameSavedTarget && storedConnection != null &&
                !status.syncing && !status.migratingFolder && storedConnection?.password == password &&
                storedConnection?.compress == compress && storedConnection?.encryptionPassword.orEmpty() == encryption) {
                Text(stringResource(R.string.optimized_sync_upgrade))
            }
            Text(stringResource(R.string.optimized_sync_hint), style = MaterialTheme.typography.bodySmall)

            OutlinedButton({ scope.launch { platform.saveConnection(NextcloudSettings(server.trim(), user.trim(), password, folder.trim(), compress, encryption.takeIf { it.isNotEmpty() }, optimized)); storedConnection = platform.connection(); vm.refresh() } },
                Modifier.fillMaxWidth(), enabled = !status.migratingFolder && server.isNotBlank() && user.isNotBlank() && password.isNotEmpty() && folder.isNotBlank()) { Text(stringResource(R.string.save_connection)) }
            if(state.settingsDestination == "sync.folder-copy") {
                SettingHeading(R.string.folder_migration_action)
                if(storedConnection?.folder == null || storedConnection?.folder == "momentum") Text(stringResource(R.string.folder_copy_unavailable))
            }
            if(storedConnection?.folder != null && storedConnection?.folder != "momentum") OutlinedButton({ migrationSource = storedConnection },
                Modifier.fillMaxWidth(), enabled = !status.syncing && !status.migratingFolder) { Text(stringResource(R.string.folder_migration_action)) }
            if(status.migratingFolder) {
                Text(stringResource(R.string.folder_migration_progress))
                LinearProgressIndicator(Modifier.fillMaxWidth())
                TextButton(platform::cancelFolderMigration) { Text(stringResource(R.string.cancel)) }
            }
            if(migrationError) Text(stringResource(R.string.folder_migration_failed), color = MaterialTheme.colorScheme.error)
            OutlinedButton({ platform.syncNow(test = true) }, Modifier.fillMaxWidth(), enabled = !status.syncing && !status.migratingFolder) { Text(stringResource(R.string.test_connection)) }
            SettingSwitch(R.string.automatic_sync, settings.automaticSync) { enabled -> vm.settings { it.copy(automaticSync = enabled) } }
            SettingSwitch(R.string.background_sync, settings.backgroundSync) { enabled -> vm.settings { it.copy(backgroundSync = enabled) } }
            if(Build.VERSION.SDK_INT >= 37) OutlinedButton({ permission.launch("android.permission.ACCESS_LOCAL_NETWORK") }, Modifier.fillMaxWidth()) { Text(stringResource(R.string.allow_local_network)) }
        }
        if(settings.provider == SyncProvider.LibreSync) {
            SettingHeading(R.string.libresync)
            Text(stringResource(R.string.nearby_foreground), style = MaterialTheme.typography.bodyMedium)
            Button({ discovery = !discovery; platform.pairing(discovery) }, Modifier.fillMaxWidth()) { Text(stringResource(if(discovery) R.string.close else R.string.nearby)) }
            status.pairingCode?.let { Text(stringResource(R.string.pairing_code, it), style = MaterialTheme.typography.titleLarge) }
            SettingHeading(R.string.nearby)
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
    migrationSource?.let { source -> AlertDialog(onDismissRequest = { migrationSource = null },
        title = { Text(stringResource(R.string.folder_migration_action)) },
        text = { Text(stringResource(R.string.folder_migration_hint, source.folder)) },
        confirmButton = { TextButton({ migrationSource = null; migrationError = false
            scope.launch {
                try { val result = platform.migrateConnection(source); folder = result.targetFolder; storedConnection = platform.connection(); vm.refresh() }
                catch(error: Exception) { migrationError = true; vm.refresh() }
            }
        }) { Text(stringResource(R.string.folder_migration_confirm)) } },
        dismissButton = { TextButton({ migrationSource = null }) { Text(stringResource(R.string.cancel)) } }) }
    unlink?.let { id -> AlertDialog(onDismissRequest = { unlink = null }, title = { Text(stringResource(R.string.unlink)) },
        text = { Text(stringResource(R.string.unlink_hint)) }, confirmButton = { TextButton({ scope.launch { platform.unlink(id); vm.refresh() }; unlink = null }) { Text(stringResource(R.string.unlink)) } },
        dismissButton = { TextButton({ unlink = null }) { Text(stringResource(R.string.cancel)) } }) }
    if(upgrade) AlertDialog(onDismissRequest = { upgrade = false },
        title = { Text(stringResource(R.string.optimized_sync_title)) },
        text = { Text(stringResource(R.string.optimized_sync_hint)) },
        confirmButton = { TextButton({
            upgrade = false
            val connection = storedConnection
            if(connection != null && sameSavedTarget && com.codedbydan.momentum.core.nextcloudConnectionIdentity(connection.serverUrl, connection.userName, connection.folder) == upgradeIdentity && !status.syncing && !status.migratingFolder) scope.launch {
                platform.saveConnection(connection.copy(optimized = true))
                storedConnection = platform.connection()
                if(storedConnection?.optimized == true) platform.syncNow()
                vm.refresh()
            }
        }) { Text(stringResource(R.string.upgrade)) } },
        dismissButton = { TextButton({ upgrade = false }) { Text(stringResource(R.string.cancel)) } })
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
