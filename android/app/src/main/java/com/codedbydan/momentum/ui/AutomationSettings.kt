// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import com.codedbydan.momentum.R

@Composable fun AutomationSettings() {
    var help by rememberSaveable { mutableStateOf(false) }
    SettingHeading(R.string.automation_title)
    Text(stringResource(R.string.automation_hint), style = MaterialTheme.typography.bodyMedium)
    OutlinedButton({ help = true }, Modifier.heightIn(min = 48.dp)) { Text(stringResource(R.string.automation_help)) }
    if(help) Dialog(onDismissRequest = { help = false }) {
        Surface(Modifier.widthIn(max = 720.dp).fillMaxWidth(), shape = MaterialTheme.shapes.extraLarge) {
            Column(Modifier.verticalScroll(rememberScrollState()).padding(20.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Text(stringResource(R.string.automation_help), style = MaterialTheme.typography.headlineSmall,
                    modifier = Modifier.semantics { heading() })
                Text(stringResource(R.string.automation_security_hint))
                listOf(R.string.automation_create, R.string.automation_find, R.string.automation_open,
                    R.string.automation_complete, R.string.automation_reopen, R.string.automation_plan).forEach { Text(stringResource(it)) }
                Text(stringResource(R.string.automation_parameters), style = MaterialTheme.typography.bodySmall)
                TextButton({ help = false }, Modifier.heightIn(min = 48.dp)) { Text(stringResource(R.string.close)) }
            }
        }
    }
}
