// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.data

import com.codedbydan.momentum.core.formatEstimate
import com.codedbydan.momentum.core.parseQuickAdd

/** Native review state; all short-syntax rules come from the shared parser. */
fun capturedDraft(title: String, notes: String, due: String, tags: String): EditorDraft {
    val parsed = parseQuickAdd(title)
    return EditorDraft(title = parsed.title, notes = notes, dueDay = due,
        estimate = if(parsed.estimateMs > 0.0) formatEstimate(parsed.estimateMs) else "",
        newTags = (parsed.tags + tags.split(',').map(String::trim).filter(String::isNotEmpty)).distinct().joinToString(", "))
}
