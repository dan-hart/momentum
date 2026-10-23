// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import androidx.compose.material.ripple.RippleAlpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.compositeOver

/** Preserve Material's native ripple while bounding its worst overlapping state.
 * Material3 1.4 uses .16 dragged / .10 focused / .08 hovered / .10 pressed.
 * The indication draws the state layer then the pressed ripple; API 26/27 also
 * doubles the requested pressed alpha before handing it to RippleDrawable.
 */
internal fun celebrationRippleAlpha(container: Color, ink: Color): RippleAlpha {
    fun readable(scale: Float): Boolean {
        val state = ink.copy(alpha = 0.16f * scale).compositeOver(container)
        val pressed = ink.copy(alpha = 0.20f * scale).compositeOver(state)
        return colorContrast(ink, pressed) >= 4.55
    }
    var lower = 0f
    var upper = 1f
    if(readable(upper)) lower = upper else repeat(24) {
        val candidate = (lower + upper) / 2f
        if(readable(candidate)) lower = candidate else upper = candidate
    }
    return RippleAlpha(draggedAlpha = 0.16f * lower, focusedAlpha = 0.10f * lower,
        hoveredAlpha = 0.08f * lower, pressedAlpha = 0.10f * lower)
}
