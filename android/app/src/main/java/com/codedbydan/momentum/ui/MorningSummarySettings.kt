// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import android.app.TimePickerDialog
import android.icu.text.ListFormatter
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import com.codedbydan.momentum.MomentumViewModel
import com.codedbydan.momentum.R
import com.codedbydan.momentum.data.Snapshot
import com.codedbydan.momentum.core.ClockTime
import com.codedbydan.momentum.core.MorningSummary
import com.codedbydan.momentum.platform.SummarySection
import com.codedbydan.momentum.platform.morningSummaryText
import kotlinx.coroutines.CancellationException
import java.time.DayOfWeek
import java.time.format.TextStyle
import java.time.temporal.WeekFields

private const val EVERY_DAY = 127
private const val WEEKDAYS = 31

@Composable
internal fun MorningSummarySettings(snapshot: Snapshot, vm: MomentumViewModel) {
    val preferences = snapshot.preferences
    val context = LocalContext.current
    val locale = androidx.compose.ui.platform.LocalLocale.current.platformLocale
    val dayMask = preferences.morningSummaryWeekdays.toInt()
    val sections = preferences.morningSummarySections
    val firstDay = WeekFields.of(locale).firstDayOfWeek
    val days = List(7) { firstDay.plus(it.toLong()) }
    val selectedDays = when (dayMask) {
        EVERY_DAY -> stringResource(R.string.summary_every_day)
        WEEKDAYS -> stringResource(R.string.summary_weekdays)
        0 -> stringResource(R.string.summary_no_days)
        else -> ListFormatter.getInstance(locale).format(
            days.filter { dayMask and it.mask != 0 }.map { it.getDisplayName(TextStyle.SHORT_STANDALONE, locale) }
        )
    }
    var showDays by rememberSaveable { mutableStateOf(false) }
    var draftDays by rememberSaveable { mutableIntStateOf(dayMask) }
    val preview by produceState<MorningSummary?>(initialValue = null, key1 = snapshot) {
        value = if (preferences.morningSummaryEnabled && dayMask != 0 && sections != 0u.toUByte()) {
            try {
                vm.repository.read { morningSummaryPreview(sections) }
            } catch (error: CancellationException) {
                throw error
            } catch (_: Exception) {
                null
            }
        } else null
    }
    SettingSwitch(R.string.summary, preferences.morningSummaryEnabled) { enabled ->
        vm.preferences { it.copy(morningSummaryEnabled = enabled) }
    }
    if (!preferences.morningSummaryEnabled) return
    TextButton({
        val time = preferences.morningSummaryTime
        TimePickerDialog(context, { _, hour, minute ->
            vm.preferences { it.copy(morningSummaryTime = ClockTime(hour.toUInt(), minute.toUInt())) }
        }, time.hour.toInt(), time.minute.toInt(), android.text.format.DateFormat.is24HourFormat(context)).show()
    }, Modifier.heightIn(min = 48.dp)) {
        Text(stringResource(R.string.summary_time_option, clockText(preferences.morningSummaryTime)))
    }
    TextButton({
        draftDays = dayMask
        showDays = true
    }, Modifier.heightIn(min = 48.dp)) {
        Text(stringResource(R.string.summary_days_option, selectedDays))
    }
    if (showDays) {
        AlertDialog(
            onDismissRequest = { showDays = false },
            title = { Text(stringResource(R.string.summary_days)) },
            text = {
                Column(Modifier.heightIn(max = 360.dp).verticalScroll(rememberScrollState())) {
                    TextButton({ draftDays = EVERY_DAY }) { Text(stringResource(R.string.summary_every_day)) }
                    TextButton({ draftDays = WEEKDAYS }) { Text(stringResource(R.string.summary_weekdays)) }
                    days.forEach { day ->
                        SummaryOption(day.getDisplayName(TextStyle.FULL_STANDALONE, locale), draftDays and day.mask != 0) {
                            draftDays = draftDays xor day.mask
                        }
                    }
                }
            },
            confirmButton = {
                TextButton({
                    val chosenDays = draftDays.toUByte()
                    showDays = false
                    vm.preferences { it.copy(morningSummaryWeekdays = chosenDays) }
                }) { Text(stringResource(R.string.save)) }
            },
            dismissButton = { TextButton({ showDays = false }) { Text(stringResource(R.string.cancel)) } },
        )
    }
    SettingHeading(R.string.summary_sections)
    SummarySection.entries.forEach { section ->
        SummaryOption(stringResource(section.label), section.isSelected(sections)) {
            vm.preferences { it.copy(morningSummarySections = (it.morningSummarySections.toInt() xor section.bit).toUByte()) }
        }
    }
    SettingHeading(R.string.summary_preview)
    val previewText = preview?.takeIf { counts ->
        dayMask != 0 && counts.sections == sections && SummarySection.entries.any {
            it.isSelected(sections) && it.count(counts) > 0u
        }
    }?.let { morningSummaryText(context, it) }
    Text(previewText ?: stringResource(
        when {
            dayMask == 0 -> R.string.summary_preview_no_days
            sections == 0u.toUByte() -> R.string.summary_preview_no_sections
            else -> R.string.summary_preview_empty
        }
    ), style = MaterialTheme.typography.bodyMedium)
}

private val DayOfWeek.mask get() = 1 shl (value - 1)

@Composable
private fun SummaryOption(label: String, checked: Boolean, toggle: () -> Unit) {
    androidx.compose.material3.ListItem(
        headlineContent = { Text(label) },
        leadingContent = { androidx.compose.material3.Checkbox(checked, onCheckedChange = null) },
        modifier = Modifier.heightIn(min = 48.dp).toggleable(checked, role = Role.Checkbox, onValueChange = { toggle() }),
    )
}
