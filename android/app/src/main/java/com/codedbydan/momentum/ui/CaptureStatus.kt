// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.*
import androidx.compose.ui.unit.dp
import com.codedbydan.momentum.R
import com.codedbydan.momentum.UiState

/** Acknowledges disk-backed capture without replacing the draft under review. */
@Composable fun CaptureStatus(state: UiState) {
    val captureError = state.error?.takeIf { error -> error in listOf(
        stringResource(R.string.capture_queue_full), stringResource(R.string.capture_unlock_required), stringResource(R.string.capture_too_large)) }
    if(state.queuedCaptures > 0 || captureError != null) {
        Surface(color = if(captureError != null) MaterialTheme.colorScheme.errorContainer else MaterialTheme.colorScheme.secondaryContainer) {
            Column(Modifier.fillMaxWidth().padding(12.dp).semantics { liveRegion = LiveRegionMode.Polite }) {
                if(state.queuedCaptures > 0) Text(pluralStringResource(R.plurals.queued_captures, state.queuedCaptures, state.queuedCaptures))
                captureError?.let { Text(it) }
            }
        }
    }
}
