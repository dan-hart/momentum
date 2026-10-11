// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.*
import androidx.compose.ui.unit.dp
import com.codedbydan.momentum.MomentumViewModel
import com.codedbydan.momentum.R
import com.codedbydan.momentum.UiState

/** Recovery exposes no draft contents and never replaces the editor being reviewed. */
@Composable fun DraftRecoveryStatus(state: UiState, vm: MomentumViewModel) {
    if(!state.draftRecoveryPending) return
    Surface(color = MaterialTheme.colorScheme.errorContainer) {
        Column(Modifier.fillMaxWidth().padding(12.dp).semantics { liveRegion = LiveRegionMode.Polite },
            verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(stringResource(R.string.draft_recovery_pending))
            if(state.editor != null || state.busy || state.restoring)
                Text(stringResource(R.string.draft_recovery_wait))
            if(state.editorLoading) LinearProgressIndicator(Modifier.fillMaxWidth())
            TextButton({ vm.retryDraftRecovery() }, enabled = state.editor == null && !state.editorLoading && !state.busy && !state.restoring,
                modifier = Modifier.heightIn(min = 48.dp)) { Text(stringResource(R.string.retry)) }
        }
    }
}
