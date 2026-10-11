// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Check
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.unit.dp
import com.codedbydan.momentum.R
import com.codedbydan.momentum.data.AppSettings
import com.codedbydan.momentum.data.NativeAccent

@Composable fun AccentSettings(settings: AppSettings, change: ((AppSettings) -> AppSettings) -> Unit) {
    SettingHeading(R.string.accent_color)
    Text(stringResource(if(settings.dynamicColor) R.string.accent_dynamic_hint else R.string.accent_hint),
        style = MaterialTheme.typography.bodyMedium)
    NativeAccent.entries.forEach { choice ->
        val selected = settings.accent == choice
        val name = stringResource(when(choice) {
            NativeAccent.Momentum -> R.string.accent_momentum
            NativeAccent.Alizarin -> R.string.accent_alizarin
            NativeAccent.Carrot -> R.string.accent_carrot
            NativeAccent.Orange -> R.string.accent_orange
            NativeAccent.Emerald -> R.string.accent_emerald
            NativeAccent.Turquoise -> R.string.accent_turquoise
            NativeAccent.PeterRiver -> R.string.accent_peter_river
            NativeAccent.Amethyst -> R.string.accent_amethyst
            NativeAccent.Pomegranate -> R.string.accent_pomegranate
            NativeAccent.GreenSea -> R.string.accent_green_sea
        })
        Row(Modifier.fillMaxWidth().heightIn(min = 48.dp)
            .selectable(selected, role = Role.RadioButton) { change { it.copy(accent = choice) } },
            verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Box(Modifier.padding(4.dp).size(32.dp).background(choice.seed, CircleShape).clearAndSetSemantics {}, contentAlignment = Alignment.Center) {
                if(selected) Icon(Icons.Default.Check, null, tint = accentInk(choice.seed))
            }
            Text(name, Modifier.weight(1f).padding(vertical = 10.dp))
            RadioButton(selected, onClick = null)
        }
    }
}
