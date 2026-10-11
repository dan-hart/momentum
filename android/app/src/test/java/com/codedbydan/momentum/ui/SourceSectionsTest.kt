// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import org.junit.Assert.*
import org.junit.Test
import java.text.BreakIterator
import java.util.Locale

class SourceSectionsTest {
    private fun boundaries(text: String): SourceBoundaries {
        val iterator = BreakIterator.getCharacterInstance(Locale.ROOT).apply { setText(text) }
        return object : SourceBoundaries {
            override fun before(offset: Int) = if(iterator.isBoundary(offset)) offset else iterator.preceding(offset).coerceAtLeast(0)
            override fun after(offset: Int) = if(iterator.isBoundary(offset)) offset else iterator.following(offset).let { if(it < 0) text.length else it }
        }
    }
    private fun model(text: String) = SourceSectionBuffer(text, ::boundaries)

    @Test fun ordinaryMultilineKeepsFullInputAndComposition() {
        val buffer = model("First\nSecond")
        val input = SourceInput("First\nSecond 📝", SourceSelection(15), SourceSelection(13, 15))
        buffer.accept(buffer.prepare(input)!!)
        assertFalse(buffer.sectioned); assertEquals(input, buffer.input); assertEquals(input.text, buffer.source)
    }

    @Test fun everySectionReassemblesSurrogatesAndCrLfWithoutLossOrLoops() {
        val source = "A📝\r\n".repeat(100_000)
        val buffer = model(source)
        val reconstructed = StringBuilder()
        var iterations = 0
        do {
            val section = buffer.section
            assertTrue(section.end > section.start); assertTrue(buffer.input.text.length <= 4096)
            assertFalse(buffer.input.text.first().isLowSurrogate())
            assertFalse(buffer.input.text.last().isHighSurrogate())
            assertFalse(buffer.input.text.last() == '\r')
            reconstructed.append(buffer.input.text); iterations++
            if(section.end == source.length) break
            buffer.next()
        } while(iterations < 1000)
        assertTrue("Finite traversal preserves all source units", reconstructed.toString() == source)
        buffer.previous(); assertTrue(buffer.section.start < source.length - 4096)
    }

    @Test fun hugePasteIsAdmittedCompletelyAndWindowRebalancesAroundItsCaret() {
        val buffer = model("Small")
        val pasted = "😀".repeat(276_480)
        val edit = buffer.prepare(SourceInput(pasted, SourceSelection(pasted.length)))!!
        buffer.accept(edit)
        assertTrue(buffer.source == pasted); assertTrue(buffer.sectioned)
        assertTrue(buffer.input.text.length <= 4096); assertEquals(pasted.length, buffer.section.end)
        assertEquals(buffer.input.text.length, buffer.input.selection.end)
        buffer.previous(); assertTrue(buffer.input.text.length <= 4096)
    }

    @Test fun editsReplaceOnlyVisibleSourceAndRetainActiveComposition() {
        val source = "a".repeat(50_000)
        val buffer = model(source); buffer.next()
        val composing = SourceInput("日本" + buffer.input.text.drop(2), SourceSelection(2), SourceSelection(0, 2))
        buffer.accept(buffer.prepare(composing)!!)
        assertEquals(SourceSelection(0, 2), buffer.input.composition)
        assertTrue(buffer.source == source.take(4096) + "日本" + source.drop(4098))
        assertEquals(SourceSelection(2), buffer.input.selection)
    }

    @Test fun smallCjkCompositionAtEitherEdgeOfAFullSectionIsAdmittedAndPreserved() {
        for(at in listOf(0, 4096)) {
            val original = "a".repeat(50_000); val buffer = model(original)
            val visible = buffer.input.text
            val proposed = SourceInput(visible.take(at) + "日" + visible.drop(at),
                SourceSelection(at + 1), SourceSelection(at, at + 1))
            buffer.accept(buffer.prepare(proposed)!!)
            assertTrue(buffer.source == original.take(at) + "日" + original.drop(at))
            assertEquals("Returning unchanged native text preserves InputConnection composition", proposed.text, buffer.input.text)
            assertTrue(buffer.input.text.length <= 8192)
            val composition = buffer.input.composition!!
            assertEquals("日", buffer.input.text.substring(composition.start, composition.end))
            assertEquals(composition.end, buffer.input.selection.end)
            buffer.accept(buffer.prepare(buffer.input.copy(composition = null))!!)
            assertNull(buffer.input.composition); assertTrue(buffer.input.text.length <= 4096)
        }
    }

    @Test fun composingAtEmojiCrLfBoundaryKeepsSurrogateAndNewlinePairsIntact() {
        val original = "📝\r\n".repeat(20_000); val buffer = model(original)
        val visible = buffer.input.text
        val proposed = SourceInput(visible + "日本", SourceSelection(visible.length + 2),
            SourceSelection(visible.length, visible.length + 2))
        buffer.accept(buffer.prepare(proposed)!!)
        val composition = buffer.input.composition!!
        assertEquals("日本", buffer.input.text.substring(composition.start, composition.end))
        assertFalse(buffer.input.text.first().isLowSurrogate()); assertFalse(buffer.input.text.last().isHighSurrogate())
        assertFalse(buffer.input.text.first() == '\n'); assertFalse(buffer.input.text.last() == '\r')
        assertTrue(buffer.source == visible + "日本" + original.drop(visible.length))
    }

    @Test fun crossingOrdinaryThresholdCarriesFiniteCompositionBudgetUntilCommit() {
        val original = "a".repeat(32_768); val buffer = model(original)
        for(composing in listOf("日", "日本")) {
            val proposed = SourceInput(original + composing, SourceSelection(original.length + composing.length),
                SourceSelection(original.length, original.length + composing.length))
            buffer.accept(buffer.prepare(proposed)!!)
            assertTrue(buffer.sectioned); assertEquals(proposed, buffer.input)
            assertTrue(buffer.source == proposed.text)
        }
        buffer.accept(buffer.prepare(buffer.input.copy(composition = null))!!)
        assertTrue(buffer.input.text.length <= 4096)
        assertTrue(buffer.source == original + "日本")
        assertNull(buffer.input.composition)
    }

    @Test fun finiteActiveFieldBudgetRejectsIncrementalGrowthWithoutChangingSourceOrCaret() {
        val buffer = model("a".repeat(50_000))
        val proposed = SourceInput("a".repeat(8191) + "日", SourceSelection(8192), SourceSelection(8191, 8192))
        buffer.accept(buffer.prepare(proposed)!!)
        assertEquals(proposed, buffer.input)
        val source = buffer.source
        assertNull(buffer.prepare(SourceInput(proposed.text + "本", SourceSelection(8193), SourceSelection(8191, 8193))))
        assertEquals(proposed, buffer.input); assertTrue(buffer.source == source)
        buffer.accept(buffer.prepare(proposed.copy(composition = null))!!)
        assertTrue(buffer.input.text.length <= 4096)
        val ordinary = model("a".repeat(32_768))
        val atBudget = SourceInput("a".repeat(36_863) + "日", SourceSelection(36_864), SourceSelection(36_863, 36_864))
        ordinary.accept(ordinary.prepare(atBudget)!!)
        assertNull(ordinary.prepare(SourceInput(atBudget.text + "本", SourceSelection(36_865), SourceSelection(36_863, 36_865))))
        assertEquals(atBudget, ordinary.input)
    }

    @Test fun rejectedFullSourceAdmissionAndLongCompositionLeaveCaretAndSourceIntact() {
        val buffer = model("x".repeat(50_000))
        val original = buffer.input; val section = buffer.section; val source = buffer.source
        val edit = buffer.prepare(SourceInput(original.text + "extra", SourceSelection(original.text.length)))!!
        // Caller deliberately rejects the complete source; no accept() means no UI/data mutation.
        assertTrue(edit.source.length > source.length)
        assertEquals(original, buffer.input); assertEquals(section, buffer.section); assertTrue(source == buffer.source)
        assertNull(buffer.prepare(SourceInput("x".repeat(5000), SourceSelection(5000), SourceSelection(0, 5000))))
        assertEquals(original, buffer.input)
    }

    @Test fun singleOversizedGraphemeHasFiniteLosslessCodePointFallback() {
        val source = "a" + "\u0301".repeat(50_000) + "\r\n📝"
        val buffer = model(source)
        val result = StringBuilder(); var iterations = 0
        while(true) {
            assertTrue(buffer.input.text.length <= 4096); assertTrue(buffer.section.end > buffer.section.start)
            result.append(buffer.input.text); iterations++
            if(buffer.section.end == source.length) break
            assertTrue(iterations < 100); buffer.next()
        }
        assertTrue(result.toString() == source)
    }

    @Test fun oversizedZwjSurrogateFallbackAdvancesAtActual4095Boundary() {
        val source = "😀" + "\u200D😀".repeat(20_000)
        // JVM Unicode versions differ from Android ICU. Explicitly model one indivisible grapheme.
        val buffer = SourceSectionBuffer(source) { text -> object : SourceBoundaries {
            override fun before(offset: Int) = if(offset == text.length) text.length else 0
            override fun after(offset: Int) = if(offset == 0) 0 else text.length
        } }
        assertEquals(4095, buffer.section.end)
        val result = StringBuilder(); var iterations = 0
        while(true) {
            assertEquals(result.length, buffer.section.start)
            assertTrue(buffer.section.end > buffer.section.start && buffer.input.text.length <= 4096)
            assertFalse(buffer.input.text.first().isLowSurrogate()); assertFalse(buffer.input.text.last().isHighSurrogate())
            result.append(buffer.input.text); iterations++
            if(buffer.section.end == source.length) break
            assertTrue(iterations < 100); buffer.next()
        }
        assertTrue(result.toString() == source)
        buffer.previous(); assertTrue(buffer.section.end > buffer.section.start)
    }

    @Test fun externalAppendKeepsSourceAndCurrentWindowAndDeletionKeepsPagingStable() {
        val source = "x".repeat(40_000); val buffer = model(source); buffer.next()
        val before = buffer.section.start
        buffer.synchronize(source + "tail"); assertEquals(before, buffer.section.start); assertTrue(buffer.source.endsWith("tail"))
        buffer.synchronize("tiny"); assertTrue(buffer.sectioned); assertEquals("tiny", buffer.input.text)
        buffer.accept(buffer.prepare(SourceInput("", SourceSelection(0)))!!)
        assertEquals("", buffer.source); assertTrue(buffer.sectioned); assertEquals(SourceSection(0, 0), buffer.section)
    }

    @Test fun deletingLastSectionShowsRetainedPrecedingTextWithCaretAtEnd() {
        val original = "a".repeat(50_000); val buffer = model(original)
        while(buffer.section.end < original.length) buffer.next()
        val retainedLength = buffer.section.start
        buffer.accept(buffer.prepare(SourceInput("", SourceSelection(0)))!!)
        assertTrue(buffer.source == original.take(retainedLength))
        assertTrue(buffer.section.start >= 0 && buffer.section.start < buffer.section.end)
        assertEquals(retainedLength, buffer.section.end)
        assertTrue(buffer.input.text.length <= 4096); assertEquals(buffer.input.text.length, buffer.input.selection.end)
        val visible = buffer.input.text
        buffer.accept(buffer.prepare(SourceInput(visible + "尾", SourceSelection(visible.length + 1)))!!)
        assertTrue(buffer.source == original.take(retainedLength) + "尾")
    }

    @Test fun externallyShortenedSourceKeepsNonemptyWindowAndCaretAtItsEnd() {
        val buffer = model("a".repeat(50_000))
        while(buffer.section.end < buffer.source.length) buffer.next()
        val retained = "b".repeat(6000); buffer.synchronize(retained)
        assertTrue(buffer.source == retained)
        assertTrue(buffer.section.start >= 0 && buffer.section.start < buffer.section.end)
        assertEquals(retained.length, buffer.section.end)
        assertTrue(buffer.input.text.length <= 4096); assertEquals(buffer.input.text.length, buffer.input.selection.end)
        buffer.previous(); assertTrue(buffer.section.start < buffer.section.end)
        buffer.next(); assertTrue(buffer.section.start < buffer.section.end)
    }
}
