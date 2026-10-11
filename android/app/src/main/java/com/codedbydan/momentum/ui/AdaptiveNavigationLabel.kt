// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.foundation.layout.widthIn
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.codedbydan.momentum.Tab

/** Native items keep their targets; only the visual label is ellipsized. */
@Composable internal fun AdaptiveNavigationLabel(text: String) {
    val density = LocalDensity.current
    val width = with(density) { LocalWindowInfo.current.containerSize.width.toDp() }
    val labelWidth = (width / Tab.entries.size - 16.dp).coerceIn(48.dp, 160.dp)
    Text(text, modifier = Modifier.widthIn(max = labelWidth), maxLines = 1, overflow = TextOverflow.Ellipsis)
}
