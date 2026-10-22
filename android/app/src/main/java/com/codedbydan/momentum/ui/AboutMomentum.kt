// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import com.codedbydan.momentum.R

internal fun appDetails(version: String, build: Long): String = "Momentum $version ($build)\nAndroid"
@Composable internal fun AboutMomentum() {
    val context = LocalContext.current
    val info = remember { context.packageManager.getPackageInfo(context.packageName, 0) }
    val details = appDetails(info.versionName.orEmpty(), androidx.core.content.pm.PackageInfoCompat.getLongVersionCode(info))
    var notices by remember { mutableStateOf(false) }
    androidx.compose.foundation.Image(androidx.compose.ui.res.painterResource(R.drawable.ic_launcher_foreground), null, Modifier.size(88.dp))
    SettingAnchor(R.string.about) { Text("Momentum", Modifier.padding(top = 24.dp).semantics { heading() }, style = MaterialTheme.typography.headlineLarge) }
    Text(stringResource(R.string.about_description), style = MaterialTheme.typography.bodyLarge)
    Text(details)
    SettingHeading(R.string.about_values)
    Text(stringResource(R.string.about_offline))
    Text(stringResource(R.string.about_privacy))
    Text(stringResource(R.string.about_freedom))
    SettingHeading(R.string.about_project)
    Text("Dan Hart")
    listOf(R.string.source_code to "https://github.com/dan-hart/momentum", R.string.report_issue to "https://github.com/dan-hart/momentum/issues").forEach { (label, url) ->
        SettingAnchor(label) { TextButton({ context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url))) }) { Text(stringResource(label)) } }
    }
    TextButton({ (context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager).setPrimaryClip(ClipData.newPlainText("Momentum", details)) }) { Text(stringResource(R.string.copy_app_details)) }
    SettingHeading(R.string.license)
    Text("GPL-3.0-or-later · LibreSync: AGPL-3.0-only")
    SettingAnchor(R.string.acknowledgments) { TextButton({ notices = !notices }) { Text(stringResource(R.string.acknowledgments)) } }
    if(notices) Text(remember { context.assets.open("THIRD_PARTY_NOTICES.txt").bufferedReader().use { it.readText() } }, style = MaterialTheme.typography.bodySmall)
}
