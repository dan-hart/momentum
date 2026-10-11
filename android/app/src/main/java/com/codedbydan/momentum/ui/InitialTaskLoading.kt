// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import com.codedbydan.momentum.R

/** Static system-surface placeholders; only the loading status has accessibility content. */
@Composable fun InitialTaskLoading(modifier: Modifier = Modifier) {
    Column(modifier.fillMaxSize().padding(20.dp), verticalArrangement = Arrangement.spacedBy(18.dp)) {
        Text(stringResource(R.string.loading), style = MaterialTheme.typography.bodyMedium,
            modifier = Modifier.semantics { liveRegion = LiveRegionMode.Polite })
        repeat(4) {
            Row(Modifier.fillMaxWidth().height(64.dp).clearAndSetSemantics {}, horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                Box(Modifier.size(36.dp).background(MaterialTheme.colorScheme.surfaceContainerHigh, CircleShape))
                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    Box(Modifier.fillMaxWidth(if(it % 2 == 0) 0.85f else 0.65f).height(18.dp)
                        .background(MaterialTheme.colorScheme.surfaceContainerHigh, MaterialTheme.shapes.small))
                    Box(Modifier.fillMaxWidth(0.55f).height(12.dp)
                        .background(MaterialTheme.colorScheme.surfaceContainerHigh, MaterialTheme.shapes.small))
                }
            }
        }
    }
}
