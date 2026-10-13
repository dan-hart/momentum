// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import com.codedbydan.momentum.R

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.DateRange
import androidx.compose.material.icons.filled.Close
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.platform.LocalLocale
import androidx.compose.ui.semantics.*
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import com.codedbydan.momentum.*
import com.codedbydan.momentum.core.*
import java.time.DayOfWeek
import java.time.LocalDate
import java.time.Instant
import java.time.ZoneOffset
import java.time.format.TextStyle
import java.time.format.DateTimeFormatter
import java.time.format.FormatStyle
import java.time.temporal.WeekFields
import kotlinx.coroutines.launch

@OptIn(ExperimentalLayoutApi::class, ExperimentalMaterial3Api::class)
@Composable fun RepeatEditor(id: String, vm: MomentumViewModel, configuration: Boolean = false, dismiss: () -> Unit) {
    var draft by remember { mutableStateOf<RepeatDraft?>(null) }
    var every by remember { mutableStateOf("1") }
    var error by remember { mutableIntStateOf(0) }
    var calendar by remember { mutableStateOf(false) }
    var busy by remember { mutableStateOf(false) }
    val locale = LocalLocale.current.platformLocale
    val weekdayOrder = remember(locale) {
        val first = WeekFields.of(locale).firstDayOfWeek.value % 7
        List(7) { (first + it) % 7 }
    }
    val weekdayFocus = remember { FocusRequester() }
    val scope = rememberCoroutineScope()
    fun apply(block: Engine.() -> Outcome) {
        if(busy) return
        busy = true
        scope.launch {
            vm.action(onSuccess = dismiss, block = block).join()
            busy = false
            if(vm.state.value.error != null) error = R.string.save_failed
        }
    }
    LaunchedEffect(id) {
        try {
            draft = vm.repository.read { if(configuration) repeatConfigDraft(id) else repeatDraft(id) }
            if(draft == null) { dismiss(); return@LaunchedEffect }
            every = draft?.every?.toString() ?: "1"
        }
        catch(e: Exception) { vm.reportError(e); dismiss() }
    }
    val current = draft
    AlertDialog(onDismissRequest = { if(!busy) dismiss() }, title = { Text(stringResource(R.string.repeat)) },
        text = {
            if(current == null) CircularProgressIndicator()
            else Column(Modifier.verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    RepeatCycle.entries.forEach { cycle -> FilterChip(current.cycle == cycle, { error = 0; draft = current.copy(cycle = cycle) }, { Text(stringResource(when(cycle) {
                        RepeatCycle.DAILY -> R.string.daily; RepeatCycle.WEEKLY -> R.string.weekly; RepeatCycle.MONTHLY -> R.string.monthly; RepeatCycle.YEARLY -> R.string.yearly
                    })) }, enabled = !busy) }
                }
                OutlinedTextField(every, { every = it }, label = { Text(stringResource(R.string.every)) }, singleLine = true, enabled = !busy,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    isError = every.toUIntOrNull()?.let { it == 0u } != false,
                    supportingText = { if(every.toUIntOrNull()?.let { it > 0u } != true) Text(stringResource(R.string.repeat_interval_invalid)) })
                if(current.cycle == RepeatCycle.WEEKLY) {
                    Text(stringResource(R.string.repeat_weekdays), modifier = Modifier.semantics { heading() })
                    FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    weekdayOrder.forEach { index ->
                        val weekday = DayOfWeek.of(if(index == 0) 7 else index).getDisplayName(TextStyle.SHORT, locale)
                        val spoken = DayOfWeek.of(if(index == 0) 7 else index).getDisplayName(TextStyle.FULL, locale)
                        FilterChip(current.weekdays.getOrElse(index) { false }, {
                            error = 0
                            draft = current.copy(weekdays = List(7) { if(it == index) !current.weekdays.getOrElse(it) { false } else current.weekdays.getOrElse(it) { false } })
                        }, { Text(weekday) }, enabled = !busy,
                            modifier = (if(index == weekdayOrder.first()) Modifier.focusRequester(weekdayFocus) else Modifier)
                                .semantics { contentDescription = spoken })
                    }
                    }
                }
                if(current.cycle == RepeatCycle.MONTHLY) {
                    listOf(R.string.same_day to MonthlyRule.SameDay, R.string.last_day to MonthlyRule.LastDay,
                        R.string.nth_weekday to MonthlyRule.NthWeekday(1, 1u)).forEach { (label, rule) ->
                        ChoiceRow(stringResource(label), when(rule) { is MonthlyRule.NthWeekday -> current.monthly is MonthlyRule.NthWeekday; MonthlyRule.SameDay -> current.monthly == rule || current.monthly is MonthlyRule.DayOfMonth; else -> current.monthly == rule },
                            enabled = !busy) { draft = current.copy(monthly = rule) }
                    }
                    (current.monthly as? MonthlyRule.NthWeekday)?.let { monthly ->
                        Text(stringResource(R.string.repeat_week_of_month), modifier = Modifier.semantics { heading() })
                        FlowRow { listOf(1, 2, 3, 4, -1).forEach { week ->
                            FilterChip(week == monthly.week, { draft = current.copy(monthly = monthly.copy(week = week)) }, {
                                Text(stringResource(when(week) {
                                    1 -> R.string.repeat_first_week; 2 -> R.string.repeat_second_week
                                    3 -> R.string.repeat_third_week; 4 -> R.string.repeat_fourth_week
                                    else -> R.string.repeat_last_week
                                }))
                            }, enabled = !busy)
                        } }
                        Text(stringResource(R.string.repeat_weekdays), modifier = Modifier.semantics { heading() })
                        FlowRow { weekdayOrder.forEach { weekday ->
                            val day = DayOfWeek.of(if(weekday == 0) 7 else weekday)
                            val spoken = day.getDisplayName(TextStyle.FULL, locale)
                            FilterChip(weekday.toUInt() == monthly.weekday,
                            { draft = current.copy(monthly = monthly.copy(weekday = weekday.toUInt())) },
                            { Text(day.getDisplayName(TextStyle.SHORT, locale)) }, enabled = !busy,
                                modifier = Modifier.semantics { contentDescription = spoken })
                        } }
                    }
                }
                val displayedStart = remember(current.startDate, locale) {
                    current.startDate?.let { raw -> runCatching { LocalDate.parse(raw)
                        .format(DateTimeFormatter.ofLocalizedDate(FormatStyle.MEDIUM).withLocale(locale)) }.getOrDefault(raw) }.orEmpty()
                }
                OutlinedTextField(displayedStart, {}, readOnly = true, enabled = !busy,
                    label = { Text(stringResource(R.string.starts)) }, placeholder = { Text(stringResource(R.string.repeat_no_start_date)) },
                    isError = error == R.string.repeat_invalid_date,
                    trailingIcon = { Row {
                        if(current.startDate != null) IconButton({ draft = current.copy(startDate = null); error = 0 }, enabled = !busy) {
                            Icon(Icons.Default.Close, stringResource(R.string.clear))
                        }
                        IconButton({ calendar = true }, enabled = !busy) { Icon(Icons.Default.DateRange, stringResource(R.string.date)) }
                    } })
                SettingSwitch(R.string.paused, current.paused, enabled = !busy) { draft = current.copy(paused = it) }
                if(error != 0) Text(stringResource(error), color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.semantics { liveRegion = LiveRegionMode.Assertive })
                if(busy) LinearProgressIndicator(Modifier.fillMaxWidth())
                if(current.existing) TextButton({ apply { if(configuration) stopRepeatConfig(id) else stopRepeat(id) } }, enabled = !busy) { Text(stringResource(R.string.stop_repeat)) }
            }
        },
        confirmButton = { TextButton(onClick = {
            val value = current ?: return@TextButton
            val interval = every.toUIntOrNull() ?: return@TextButton
            if(value.cycle == RepeatCycle.WEEKLY && value.weekdays.none { it }) {
                error = R.string.pick_weekday
                weekdayFocus.requestFocus()
                return@TextButton
            }
            if(value.startDate != null && runCatching { LocalDate.parse(value.startDate) }.isFailure) { error = R.string.repeat_invalid_date; return@TextButton }
            apply { if(configuration) saveRepeatConfig(id, value.copy(every = interval)) else saveRepeat(id, value.copy(every = interval)) }
        }, enabled = !busy && current != null && every.toUIntOrNull()?.let { it > 0u } == true) { Text(stringResource(R.string.save)) } },
        dismissButton = { TextButton(dismiss, enabled = !busy) { Text(stringResource(R.string.cancel)) } })
    if(calendar && current != null) {
        val selected = runCatching { LocalDate.parse(current.startDate).atStartOfDay(ZoneOffset.UTC).toInstant().toEpochMilli() }.getOrNull()
        val picker = rememberDatePickerState(initialSelectedDateMillis = selected)
        DatePickerDialog(onDismissRequest = { calendar = false },
            confirmButton = { TextButton({
                picker.selectedDateMillis?.let { draft = current.copy(startDate = Instant.ofEpochMilli(it).atZone(ZoneOffset.UTC).toLocalDate().toString()); error = 0 }
                calendar = false
            }) { Text(stringResource(R.string.save)) } },
            dismissButton = { TextButton({ calendar = false }) { Text(stringResource(R.string.cancel)) } }) { DatePicker(picker) }
    }
}
