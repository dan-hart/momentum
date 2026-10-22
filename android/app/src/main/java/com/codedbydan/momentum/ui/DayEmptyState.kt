// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import android.provider.Settings
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.unit.dp
import com.codedbydan.momentum.R
import com.codedbydan.momentum.*
import com.codedbydan.momentum.core.*
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import java.time.LocalDateTime
import kotlin.random.Random

/** A visible state is stable; returning to it avoids the immediately previous message. */
internal class CompletionMessageSelection(private val random: (Int) -> Int = { Random.nextInt(it) }) {
    private val previous = mutableMapOf<String, UInt>()
    fun choose(context: String, eligible: List<UInt>): UInt? {
        if(eligible.isEmpty()) return null
        val options = eligible.filter { it != previous[context] }.ifEmpty { eligible }
        return options[random(options.size)].also { previous[context] = it }
    }
}
internal val messageSelection = CompletionMessageSelection()
internal enum class CelebrationFeedback { Hidden, Static, Burst }
internal fun celebrationFeedback(completion: DayCompletion, dynamic: Boolean, animationScale: Float): CelebrationFeedback = when {
    !dynamic || completion !in listOf(DayCompletion.PERIOD_COMPLETE, DayCompletion.DAY_COMPLETE) -> CelebrationFeedback.Hidden
    !animationScale.isFinite() || animationScale <= 0f -> CelebrationFeedback.Static
    else -> CelebrationFeedback.Burst
}
@Composable internal fun DayEmptyState(state: UiState) {
    val snapshot = state.snapshot ?: return
    val status = snapshot.dayStatus ?: return
    var now by remember { mutableStateOf(LocalDateTime.now()) }
    LaunchedEffect(Unit) { while(true) { delay(30_000); now = LocalDateTime.now() } }
    val eligible = completionMessageIds(state.view, status.completion, now.hour.toUInt())
    val message = remember(state.view, status.completion, eligible, now.toLocalDate(), snapshot.settings.dynamicEmptyMessages) {
        if(snapshot.settings.dynamicEmptyMessages) messageSelection.choose(state.view.toString(), eligible) else null
    }
    val context = LocalContext.current
    val completed = status.completion == DayCompletion.DAY_COMPLETE || status.completion == DayCompletion.PERIOD_COMPLETE
    val symbol = when(state.view) { View.Morning -> morningSymbol; View.Tonight -> eveningSymbol; else -> Icons.Default.CheckCircle }
    val scope = rememberCoroutineScope()
    val progress = remember(state.view, status.completion, snapshot.settings.dynamicEmptyMessages) { Animatable(1f) }
    var staticCelebration by remember(state.view, status.completion, snapshot.settings.dynamicEmptyMessages) { mutableStateOf(false) }
    Column(Modifier.fillMaxWidth().padding(32.dp), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Icon(symbol, null, Modifier.size(if(staticCelebration) 56.dp else 40.dp), tint = MaterialTheme.colorScheme.primary)
        Text(stringResource(if(completed) R.string.list_complete else if(status.completion == DayCompletion.PENDING) R.string.visible_list_clear else R.string.nothing_planned), style = MaterialTheme.typography.headlineSmall)
        val text = message?.let { context.resources.getStringArray(R.array.completion_messages).getOrNull(it.toInt() - 1) }
        Text(text ?: stringResource(if(completed) R.string.completed_hint else R.string.empty_hint), color = MaterialTheme.colorScheme.onSurfaceVariant)
        if(completed && message != null) TextButton({
            when(celebrationFeedback(status.completion, snapshot.settings.dynamicEmptyMessages,
                Settings.Global.getFloat(context.contentResolver, Settings.Global.ANIMATOR_DURATION_SCALE, 1f))) {
                CelebrationFeedback.Static -> staticCelebration = true
                CelebrationFeedback.Burst -> scope.launch { progress.snapTo(0f); progress.animateTo(1f, tween(850)) }
                CelebrationFeedback.Hidden -> Unit
            }
        }) { Text(stringResource(R.string.celebrate)) }
        val color = MaterialTheme.colorScheme.primary
        if(progress.value < 1f) Canvas(Modifier.fillMaxWidth().height(64.dp).clearAndSetSemantics {}) {
            repeat(18) { i ->
                val x = size.width * ((i * 37 % 101) / 101f)
                val y = size.height * progress.value
                drawCircle(color.copy(alpha = 1f - progress.value), 3.dp.toPx(), Offset(x, y))
            }
        }
    }
}
@Composable internal fun HiddenDayFooter(state: UiState, vm: MomentumViewModel) {
    val snapshot = state.snapshot ?: return
    val status = snapshot.dayStatus ?: return
    if(state.view != View.Today || snapshot.preferences.showDayPeriodTasksInToday) return
    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp), horizontalAlignment = Alignment.CenterHorizontally) {
        listOf(Triple(status.hiddenMorning, View.Morning, R.string.morning), Triple(status.hiddenEvening, View.Tonight, R.string.evening)).forEach { (count, view, label) ->
            if(count > 0u) TextButton({ vm.navigate(view) }, Modifier.heightIn(min = 48.dp)) {
                Text(androidx.compose.ui.res.pluralStringResource(R.plurals.hidden_period_tasks, count.toInt(), count.toInt(), stringResource(label)), style = MaterialTheme.typography.labelMedium)
            }
        }
    }
}
