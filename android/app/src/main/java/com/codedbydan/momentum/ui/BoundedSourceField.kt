// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import android.icu.text.BreakIterator
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.*
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.unit.dp
import com.codedbydan.momentum.R
import java.util.Locale

internal fun nativeSourceBoundaries(text: String): SourceBoundaries {
    val iterator = BreakIterator.getCharacterInstance(Locale.ROOT).apply { setText(text) }
    return object : SourceBoundaries {
        override fun before(offset: Int) = if(iterator.isBoundary(offset)) offset else iterator.preceding(offset).coerceAtLeast(0)
        override fun after(offset: Int) = if(iterator.isBoundary(offset)) offset else iterator.following(offset).let { if(it < 0) text.length else it }
    }
}

/**
 * Native IME field with synchronous whole-source admission. TextFieldValue retains selection and
 * composition; only the visible section is laid out. No text or field state is rememberSaveable.
 * Copy/Share/Save and the private draft continue to use the caller's complete source.
 */
@Composable fun BoundedSourceField(
    source: String,
    onSourceChange: (String) -> Boolean,
    label: String,
    enabled: Boolean = true,
    isError: Boolean = false,
    modifier: Modifier = Modifier,
    minLines: Int = 4,
    maxLines: Int = 10,
) {
    val buffer = remember { SourceSectionBuffer(source, ::nativeSourceBoundaries) }
    var value by remember { mutableStateOf(buffer.input.toTextFieldValue()) }
    // Adjacent Unicode sections can contain exactly the same text/caret. Their offsets still need
    // to recompose, rather than relying on TextFieldValue equality to refresh navigation semantics.
    var revision by remember { mutableIntStateOf(0) }
    var largeComposition by remember { mutableStateOf(false) }
    val fieldFocus = remember { FocusRequester() }
    var fieldFocused by remember { mutableStateOf(false) }
    var historyScope by remember { mutableIntStateOf(0) }
    var restoreFieldFocus by remember { mutableStateOf(false) }
    fun resetFieldHistory() {
        // The field's native UndoManager belongs to one visible source scope. Reusing it after
        // navigation/rewindowing can insert another section's old text into the complete draft.
        // Keyboard navigation keeps its focused button; only restore an already-focused field.
        restoreFieldFocus = fieldFocused
        historyScope++
    }
    // External source can change after explicit Paste or VM restoration. A matching admitted edit
    // keeps the original TextFieldValue, including the platform IME's active composition/caret.
    if(source != buffer.source) {
        resetFieldHistory()
        buffer.synchronize(source)
        value = buffer.input.toTextFieldValue()
        largeComposition = false
        revision++
    }
    LaunchedEffect(historyScope) {
        if(restoreFieldFocus) {
            restoreFieldFocus = false
            fieldFocus.requestFocus()
        }
    }
    val section = remember(revision, source) { buffer.section }
    val fieldScope = historyScope
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        if(buffer.sectioned) {
            Text(stringResource(R.string.source_section_hint), style = MaterialTheme.typography.bodySmall)
            Text(stringResource(R.string.source_section_position, section.start + if(source.isEmpty()) 0 else 1, section.end, source.length),
                style = MaterialTheme.typography.bodySmall,
                modifier = Modifier.testTag("source-section-position").semantics { liveRegion = LiveRegionMode.Polite })
        }
        key(fieldScope) { OutlinedTextField(value = value, onValueChange = fieldEdit@ { proposed ->
            // Disposing CoreTextField also delivers its old processor text through onBlur. That
            // callback belongs to the retired visible section, even after the next field is shown.
            if(fieldScope != historyScope) return@fieldEdit
            val edit = buffer.prepare(proposed.toSourceInput())
            if(edit == null) largeComposition = true
            else if(edit.source == buffer.source || onSourceChange(edit.source)) {
                largeComposition = false
                // A preserved native proposal keeps same-section undo and active composition.
                // Rewindowing can also change the text with identical section offsets, while a
                // short final section can safely change its end without changing source scope.
                if(edit.input.text != proposed.text || edit.section.start != buffer.section.start)
                    resetFieldHistory()
                buffer.accept(edit)
                value = edit.input.toTextFieldValue()
                revision++
            }
        }, label = { Text(label) }, enabled = enabled, isError = isError || largeComposition,
            textStyle = taskTypography().bodyLarge, minLines = minLines, maxLines = maxLines,
            modifier = modifier.focusRequester(fieldFocus).onFocusChanged { fieldFocused = it.isFocused }
                .testTag("bounded-source-field")) }
        if(largeComposition) Text(stringResource(R.string.source_section_composition),
            color = MaterialTheme.colorScheme.error,
            modifier = Modifier.semantics { liveRegion = LiveRegionMode.Assertive })
        if(buffer.sectioned) {
            if(section.splitsLongCharacter) Text(stringResource(R.string.source_section_long_character),
                style = MaterialTheme.typography.bodySmall)
            // Wrapping controls keep both actions usable with German and large system text.
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                OutlinedButton(onClick = {
                    resetFieldHistory()
                    buffer.previous(); value = buffer.input.toTextFieldValue(); revision++; largeComposition = false
                }, enabled = enabled && section.start > 0,
                    modifier = Modifier.fillMaxWidth().sizeIn(minHeight = 48.dp)) {
                    Text(stringResource(R.string.source_section_previous, label))
                }
                OutlinedButton(onClick = {
                    resetFieldHistory()
                    buffer.next(); value = buffer.input.toTextFieldValue(); revision++; largeComposition = false
                }, enabled = enabled && section.end < source.length,
                    modifier = Modifier.fillMaxWidth().sizeIn(minHeight = 48.dp)) {
                    Text(stringResource(R.string.source_section_next, label))
                }
            }
        }
    }
}

private fun SourceInput.toTextFieldValue() = TextFieldValue(text,
    TextRange(selection.start, selection.end), composition?.let { TextRange(it.start, it.end) })
private fun TextFieldValue.toSourceInput() = SourceInput(text,
    SourceSelection(selection.start, selection.end), composition?.let { SourceSelection(it.start, it.end) })
