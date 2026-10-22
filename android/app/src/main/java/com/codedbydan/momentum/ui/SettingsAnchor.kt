// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import androidx.compose.foundation.focusable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.relocation.BringIntoViewRequester
import androidx.compose.foundation.relocation.bringIntoViewRequester
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import com.codedbydan.momentum.R
import com.codedbydan.momentum.data.SyncProvider

internal val LocalSettingsAnchorLabel = staticCompositionLocalOf<Int?> { null }
/** Exactly one native anchor, with category fallback for unavailable provider controls. */
internal fun settingsAnchorLabel(destination: String?, provider: SyncProvider): Int? = when(destination) {
    "notifications.reminders" -> R.string.notifications
    "sync.provider" -> R.string.provider
    "sync.status" -> if(provider != SyncProvider.Off) R.string.sync_status_heading else R.string.provider
    "sync.nextcloud", "sync.folder-copy" -> if(provider == SyncProvider.Nextcloud) settingsLabels[destination] else R.string.provider
    "sync.libresync", "sync.devices" -> if(provider == SyncProvider.LibreSync) settingsLabels[destination] else R.string.provider
    "sync.automatic", "sync.background", "sync.compression" -> if(provider == SyncProvider.Nextcloud) settingsLabels[destination] else R.string.provider
    else -> settingsLabels[destination]
}
@Composable internal fun SettingAnchor(label: Int, content: @Composable () -> Unit) {
    val target = LocalSettingsAnchorLabel.current
    val focus = remember { FocusRequester() }
    val reveal = remember { BringIntoViewRequester() }
    val matches = label == target
    Box(Modifier.bringIntoViewRequester(reveal).focusRequester(focus).focusable(enabled = matches)) {
        content()
        LaunchedEffect(target, matches) { if(matches) { reveal.bringIntoView(); focus.requestFocus() } }
    }
}
