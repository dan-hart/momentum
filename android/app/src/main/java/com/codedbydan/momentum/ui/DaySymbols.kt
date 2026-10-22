// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.path
import androidx.compose.ui.unit.dp

internal val morningSymbol = ImageVector.Builder("Morning", 24.dp, 24.dp, 24f, 24f).apply {
    path(fill = SolidColor(Color.Black)) {
        moveTo(12f, 7f); arcTo(5f, 5f, 0f, true, true, 12f, 17f); arcTo(5f, 5f, 0f, true, true, 12f, 7f); close()
        moveTo(11f, 1f); lineTo(13f, 1f); lineTo(13f, 5f); lineTo(11f, 5f); close()
        moveTo(11f, 19f); lineTo(13f, 19f); lineTo(13f, 23f); lineTo(11f, 23f); close()
        moveTo(1f, 11f); lineTo(5f, 11f); lineTo(5f, 13f); lineTo(1f, 13f); close()
        moveTo(19f, 11f); lineTo(23f, 11f); lineTo(23f, 13f); lineTo(19f, 13f); close()
        moveTo(3f, 4f); lineTo(4f, 3f); lineTo(7f, 6f); lineTo(6f, 7f); close()
        moveTo(17f, 18f); lineTo(18f, 17f); lineTo(21f, 20f); lineTo(20f, 21f); close()
        moveTo(3f, 20f); lineTo(6f, 17f); lineTo(7f, 18f); lineTo(4f, 21f); close()
        moveTo(17f, 6f); lineTo(20f, 3f); lineTo(21f, 4f); lineTo(18f, 7f); close()
    }
}.build()
internal val eveningSymbol = ImageVector.Builder("Evening", 24.dp, 24.dp, 24f, 24f).apply {
    path(fill = SolidColor(Color.Black)) {
        moveTo(20f, 16f); curveTo(11f, 19f, 5f, 10f, 10f, 3f)
        curveTo(-2f, 7f, 3f, 25f, 15f, 21f); curveTo(18f, 20f, 20f, 18f, 20f, 16f); close()
    }
}.build()
