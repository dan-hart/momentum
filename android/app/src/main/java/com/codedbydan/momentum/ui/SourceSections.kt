// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import kotlin.math.abs

/** UTF-16 offsets, matching Android's text selection and the existing import limit. */
internal data class SourceSelection(val start: Int, val end: Int = start)
internal data class SourceInput(val text: String, val selection: SourceSelection = SourceSelection(0), val composition: SourceSelection? = null)
internal data class SourceSection(val start: Int, val end: Int, val splitsLongCharacter: Boolean = false)
internal interface SourceBoundaries {
    fun before(offset: Int): Int
    fun after(offset: Int): Int
}

/** No source text belongs in a Bundle. The existing private draft remains authoritative. */
internal class SourceSectionBuffer(
    source: String,
    private val boundaries: (String) -> SourceBoundaries,
) {
    companion object {
        const val SECTION_THRESHOLD = 32_768
        const val SECTION_SIZE = 4_096
        const val COMPOSING_SECTION_SIZE = SECTION_SIZE * 2
    }
    var source = source
        private set
    var sectioned = source.length > SECTION_THRESHOLD
        private set
    var section = if(sectioned) window(source, 0, 0) else SourceSection(0, source.length)
        private set
    var input = SourceInput(source.substring(section.start, section.end))
        private set
    private var compositionBudget: Int? = null

    data class Edit(val source: String, val sectioned: Boolean, val section: SourceSection, val input: SourceInput, val compositionBudget: Int?)

    /** Prepare without changing either source or caret; rejection therefore retains the whole edit. */
    fun prepare(proposed: SourceInput): Edit? {
        val complete = if(proposed.text == input.text) source
            else source.replaceRange(section.start, section.end, proposed.text)
        val paged = sectioned || complete.length > SECTION_THRESHOLD
        // Returning different text while the IME composes makes Compose restart InputConnection
        // and commit its composition. Keep proposed text unchanged within a finite temporary budget,
        // then rewindow on commit. Crossing from an ordinary field carries its larger initial budget.
        val budget = if(paged && proposed.composition != null) compositionBudget ?:
            if(sectioned) COMPOSING_SECTION_SIZE else SECTION_THRESHOLD + SECTION_SIZE else null
        if(budget != null && (abs(proposed.composition!!.end - proposed.composition.start) > SECTION_SIZE || proposed.text.length > budget)) return null
        val next = when {
            !paged -> SourceSection(0, complete.length)
            proposed.composition != null -> SourceSection(section.start, section.start + proposed.text.length, section.splitsLongCharacter)
            else -> window(complete, section.start, section.start + proposed.selection.end)
        }
        val offset = section.start - next.start
        fun remap(range: SourceSelection) = SourceSelection(
            (range.start + offset).coerceIn(0, next.end - next.start),
            (range.end + offset).coerceIn(0, next.end - next.start))
        return Edit(complete, paged, next, SourceInput(complete.substring(next.start, next.end),
            remap(proposed.selection), proposed.composition?.let(::remap)), budget)
    }

    fun accept(edit: Edit) {
        source = edit.source; sectioned = edit.sectioned; section = edit.section; input = edit.input
        compositionBudget = edit.compositionBudget
    }

    /** External full-source changes, including explicit Paste, keep the current nearby position. */
    fun synchronize(complete: String) {
        if(complete == source) return
        compositionBudget = null
        val focus = (section.start + input.selection.end).coerceAtMost(complete.length)
        source = complete
        sectioned = sectioned || complete.length > SECTION_THRESHOLD
        section = if(sectioned) window(complete, section.start, focus) else SourceSection(0, complete.length)
        input = SourceInput(complete.substring(section.start, section.end),
            SourceSelection((focus - section.start).coerceIn(0, section.end - section.start)))
    }

    fun next() {
        if(section.end >= source.length) return
        compositionBudget = null
        section = window(source, section.end, section.end)
        input = SourceInput(source.substring(section.start, section.end))
    }

    fun previous() {
        if(section.start == 0) return
        compositionBudget = null
        val end = section.start
        val target = (end - SECTION_SIZE).coerceAtLeast(0)
        val marks = boundaries(source)
        val boundary = marks.after(target)
        val split = boundary >= end
        val start = if(split) codePointAfter(source, target) else boundary
        section = SourceSection(start, end, split)
        input = SourceInput(source.substring(start, end), SourceSelection(end - start))
    }

    private fun window(text: String, preferredStart: Int, focus: Int): SourceSection {
        if(text.length <= SECTION_SIZE) return SourceSection(0, text.length)
        val marks = boundaries(text)
        var split = false
        val requestedStart = preferredStart.coerceIn(0, text.length)
        // Deleting the last section or replacing source with a shorter value keeps its caret at
        // EOF, but must show the retained preceding text rather than a nonempty source's empty page.
        val wantedStart = if(requestedStart == text.length) (text.length - SECTION_SIZE).coerceAtLeast(0) else requestedStart
        var start = marks.before(wantedStart)
        // A grapheme can contain arbitrarily many combining characters. Preserve every code point,
        // including surrogate pairs/CRLF, but split that exceptional grapheme explicitly and finitely.
        // A surrogate/CRLF-safe cut may be 4095 rather than 4096. Use actual reachable grapheme
        // progress, not a fixed distance, or the next section of one giant ZWJ cluster rewinds.
        if(wantedStart > start && marks.before((start + SECTION_SIZE).coerceAtMost(text.length)) <= wantedStart) {
            start = codePointBefore(text, wantedStart); split = true
        }
        var target = (start + SECTION_SIZE).coerceAtMost(text.length)
        var end = marks.before(target)
        if(end <= start) { end = codePointBefore(text, target); split = true }
        if(focus > end) {
            // A large paste may put the caret far beyond the old section. Show its end immediately.
            target = focus.coerceIn(0, text.length)
            end = marks.after(target)
            if(end - target > SECTION_SIZE / 2) { end = codePointAfter(text, target); split = true }
            val startTarget = (end - SECTION_SIZE).coerceAtLeast(0)
            start = marks.after(startTarget)
            if(start >= end) { start = codePointAfter(text, startTarget); split = true }
        }
        return SourceSection(start, end, split)
    }

}

/** Only the pathological oversized-grapheme fallback uses these boundaries. */
private fun codePointBefore(text: String, offset: Int): Int {
    var at = offset.coerceIn(0, text.length)
    if(at in 1 until text.length && Character.isHighSurrogate(text[at - 1]) && Character.isLowSurrogate(text[at])) at--
    if(at in 1 until text.length && text[at - 1] == '\r' && text[at] == '\n') at--
    return at
}
private fun codePointAfter(text: String, offset: Int): Int {
    var at = offset.coerceIn(0, text.length)
    if(at in 1 until text.length && Character.isHighSurrogate(text[at - 1]) && Character.isLowSurrogate(text[at])) at++
    if(at in 1 until text.length && text[at - 1] == '\r' && text[at] == '\n') at++
    return at
}
