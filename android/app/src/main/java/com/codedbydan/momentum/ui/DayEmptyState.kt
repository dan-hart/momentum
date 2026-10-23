// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import android.provider.Settings
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.animation.core.LinearEasing
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.Star
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.unit.dp
import com.codedbydan.momentum.R
import com.codedbydan.momentum.*
import com.codedbydan.momentum.core.*
import kotlinx.coroutines.delay
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import java.time.LocalDateTime
import kotlin.random.Random
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.sin
import kotlin.math.min

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
@OptIn(ExperimentalMaterial3Api::class)
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
    val lifecycleOwner = LocalLifecycleOwner.current
    // Leaving the completion context disposes the effect; taps restart one animation.
    key(state.view, status.completion, snapshot.settings.dynamicEmptyMessages) {
        val progress = remember { Animatable(1f) }
        var feedback by remember { mutableStateOf(CelebrationFeedback.Hidden) }
        var replay by remember { mutableIntStateOf(0) }
        DisposableEffect(lifecycleOwner) {
            val observer = LifecycleEventObserver { _, event ->
                if(event == Lifecycle.Event.ON_STOP) feedback = CelebrationFeedback.Hidden
            }
            lifecycleOwner.lifecycle.addObserver(observer)
            onDispose { lifecycleOwner.lifecycle.removeObserver(observer) }
        }
        LaunchedEffect(replay, feedback) {
            if(feedback == CelebrationFeedback.Burst) {
                progress.snapTo(0f)
                progress.animateTo(1f, tween(1_200, easing = LinearEasing))
                feedback = CelebrationFeedback.Hidden
            }
        }
        Box(Modifier.fillMaxWidth().padding(32.dp), contentAlignment = Alignment.Center) {
            Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Icon(symbol, null, Modifier.size(40.dp), tint = MaterialTheme.colorScheme.primary)
                Text(stringResource(if(completed) R.string.list_complete else if(status.completion == DayCompletion.PENDING) R.string.visible_list_clear else R.string.nothing_planned), style = MaterialTheme.typography.headlineSmall)
                val text = message?.let { context.resources.getStringArray(R.array.completion_messages).getOrNull(it.toInt() - 1) }
                Text(text ?: stringResource(if(completed) R.string.completed_hint else R.string.empty_hint), color = MaterialTheme.colorScheme.onSurfaceVariant)
                if(completed && message != null) {
                    val scheme = MaterialTheme.colorScheme
                    val rippleAlpha = remember(scheme.primary, scheme.onPrimary) {
                        celebrationRippleAlpha(scheme.primary, scheme.onPrimary)
                    }
                    CompositionLocalProvider(LocalRippleConfiguration provides RippleConfiguration(rippleAlpha = rippleAlpha)) {
                        Button(
                            onClick = {
                                feedback = celebrationFeedback(status.completion, snapshot.settings.dynamicEmptyMessages,
                                    Settings.Global.getFloat(context.contentResolver, Settings.Global.ANIMATOR_DURATION_SCALE, 1f))
                                replay++
                            },
                            modifier = Modifier.heightIn(min = 48.dp),
                            contentPadding = PaddingValues(horizontal = 24.dp, vertical = 12.dp),
                        ) {
                            Icon(Icons.Default.Star, null, Modifier.size(20.dp))
                            Spacer(Modifier.width(8.dp))
                            Text(stringResource(R.string.celebrate), style = MaterialTheme.typography.labelLarge)
                        }
                    }
                }
            }
            if(feedback != CelebrationFeedback.Hidden) {
                CelebrationFlourish(progress.value, feedback == CelebrationFeedback.Static, Modifier.matchParentSize())
            }
        }
    }
}

/** Decoration only: the overlay never changes layout or intercepts the button. */
@Composable private fun CelebrationFlourish(progress: Float, reducedMotion: Boolean, modifier: Modifier) {
    val colors = listOf(MaterialTheme.colorScheme.primary, MaterialTheme.colorScheme.tertiary,
        Color(0xFFFFB300), Color(0xFFFF6F61), Color(0xFF42A5F5), Color(0xFFAB47BC))
    Canvas(modifier.clearAndSetSemantics {}) {
        val origin = Offset(size.width / 2f, 20.dp.toPx())
        val radius = (size.width / 2f - 8.dp.toPx()).coerceAtLeast(0f)
        if(reducedMotion) {
            repeat(10) { i ->
                val angle = i * 2.0 * PI / 10
                val point = origin + Offset(cos(angle).toFloat() * min(radius, 58.dp.toPx()), sin(angle).toFloat() * 38.dp.toPx())
                drawSparkle(point, 5.dp.toPx(), colors[i % colors.size])
            }
        } else {
            val expansion = 1f - (1f - progress) * (1f - progress)
            val alpha = ((1f - progress) * 1.5f).coerceIn(0f, 1f)
            drawCircle(colors[0].copy(alpha = alpha * 0.55f),
                14.dp.toPx() + expansion * min(radius * 0.6f, 90.dp.toPx()), origin, style = Stroke(3.dp.toPx()))
            repeat(64) { i ->
                val angle = i * 2.0 * PI / 64 + (i % 3) * 0.07
                val distance = min(radius, 188.dp.toPx()) * (0.45f + (i * 37 % 112) / 112f * 0.55f) * expansion
                val point = origin + Offset(cos(angle).toFloat() * distance,
                    sin(angle).toFloat() * distance * 0.75f + 72.dp.toPx() * progress * progress)
                val color = colors[i % colors.size].copy(alpha = alpha)
                if(i % 4 == 0) drawSparkle(point, (6 + i % 3).dp.toPx(), color)
                else rotate(i * 29f + progress * 210f, point) {
                    drawRect(color, point - Offset(3.dp.toPx(), 6.dp.toPx()), Size(6.dp.toPx(), 12.dp.toPx()))
                }
            }
        }
    }
}

private fun DrawScope.drawSparkle(center: Offset, radius: Float, color: Color) {
    val path = Path().apply {
        repeat(8) { i ->
            val angle = i * PI / 4 - PI / 2
            val length = if(i % 2 == 0) radius else radius * 0.3f
            val x = center.x + cos(angle).toFloat() * length
            val y = center.y + sin(angle).toFloat() * length
            if(i == 0) moveTo(x, y) else lineTo(x, y)
        }
        close()
    }
    drawPath(path, color)
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
