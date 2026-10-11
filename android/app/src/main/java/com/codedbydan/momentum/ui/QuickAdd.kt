// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.codedbydan.momentum.MomentumViewModel
import com.codedbydan.momentum.R
import com.codedbydan.momentum.UiState
import com.codedbydan.momentum.core.*
import kotlinx.coroutines.launch

@OptIn(ExperimentalLayoutApi::class)
@Composable fun QuickAdd(state: UiState, vm: MomentumViewModel) {
    var input by rememberSaveable(state.view, stateSaver = TextFieldValue.Saver) { mutableStateOf(TextFieldValue()) }
    var limitFeedback by remember { mutableStateOf(false) }
    var busy by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    fun submit() {
        if(input.text.isBlank() || parseQuickAdd(input.text).title.isBlank() || busy) return
        val source = input.text
        val job = vm.quickAdd(source) { if(input.text == source) input = TextFieldValue() } ?: return
        scope.launch { busy = true; try { job.join() } finally { busy = false } }
    }
    Column(Modifier.padding(horizontal = 16.dp, vertical = 8.dp)) {
        OutlinedTextField(input, {
            if(it.text.length > 4096) limitFeedback = true
            else {
                // Caret/composition callbacks do not acknowledge an input-length error.
                if(it.text != input.text) limitFeedback = false
                input = it
            }
        },
            modifier = Modifier.fillMaxWidth(), label = { Text(stringResource(R.string.quick_add)) }, singleLine = true,
            enabled = !busy, isError = limitFeedback,
            textStyle = taskTypography().bodyLarge,
            supportingText = { Text(stringResource(if(limitFeedback) R.string.quick_add_limit else R.string.quick_add_hint)) },
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done), keyboardActions = KeyboardActions(onDone = { submit() }),
            trailingIcon = { IconButton(::submit, enabled = input.text.isNotBlank() && parseQuickAdd(input.text).title.isNotBlank() && !busy) { Icon(Icons.Default.Add, stringResource(R.string.quick_add_submit)) } })
        val word = if(input.selection.collapsed && input.text.length <= 4096) hashWordAt(input.text, input.text.codePointCount(0, input.selection.end).toUInt()) else null
        val suggestions = word?.let { token -> state.snapshot?.tags.orEmpty().filter { it.title.startsWith(token.prefix, ignoreCase = true) }.take(6) }.orEmpty()
        if(suggestions.isNotEmpty()) FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            suggestions.forEach { tag -> AssistChip(onClick = {
                completeHashWord(input.text, input.text.codePointCount(0, input.selection.end).toUInt(), tag.title)?.let { completed ->
                    if(completed.text.length > 4096) limitFeedback = true
                    else {
                        input = TextFieldValue(completed.text, TextRange(completed.text.offsetByCodePoints(0, completed.cursor.toInt())))
                        limitFeedback = false
                    }
                }
            }, label = { Text("#${tag.title}", maxLines = 1, overflow = TextOverflow.Ellipsis) }) }
        }
    }
}
