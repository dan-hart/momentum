// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

/** Leave room for the actual title line, including Android's non-linear sp conversion. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable fun adaptiveTopAppBarHeight(): Dp {
    val title = MaterialTheme.typography.titleLarge
    val lineHeight = with(LocalDensity.current) { title.lineHeight.toDp() }
    return maxOf(TopAppBarDefaults.TopAppBarExpandedHeight, lineHeight + 16.dp)
}

/** Visual truncation keeps the complete native Text semantics for screen readers. */
@Composable fun AdaptiveTopAppBarTitle(text: String, modifier: Modifier = Modifier) {
    Text(text, modifier, maxLines = 1, overflow = TextOverflow.Ellipsis)
}
