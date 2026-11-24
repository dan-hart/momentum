// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

internal fun replacementTaskFocus(before: List<String>, after: List<String>, affected: Set<String>): String? {
    val anchor = before.indexOfFirst { it in affected }
    if (anchor < 0) return null
    val eligible = after.toSet() - affected
    return before.subList(anchor + 1, before.size).firstOrNull { it in eligible }
        ?: before.subList(0, anchor).lastOrNull { it in eligible }
        ?: after.firstOrNull { it in eligible }
        ?: after.firstOrNull { it in affected }
}
