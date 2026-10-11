// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import androidx.compose.runtime.Composable
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalLocale
import com.codedbydan.momentum.R
import com.codedbydan.momentum.core.*
import java.time.LocalDate
import java.time.LocalTime
import java.time.format.DateTimeFormatter
import java.time.format.FormatStyle

@Composable fun clockText(time: ClockTime): String {
    val context = LocalContext.current
    val locale = LocalLocale.current.platformLocale
    val skeleton = if(android.text.format.DateFormat.is24HourFormat(context)) "Hm" else "hm"
    val pattern = android.text.format.DateFormat.getBestDateTimePattern(locale, skeleton)
    return LocalTime.of(time.hour.toInt(), time.minute.toInt()).format(DateTimeFormatter.ofPattern(pattern, locale))
}

@Composable fun titleText(title: ViewTitle): String = when(title) {
    ViewTitle.Today -> stringResource(R.string.today)
    ViewTitle.Morning -> stringResource(R.string.morning)
    ViewTitle.Tonight -> stringResource(R.string.evening)
    ViewTitle.ComingUp -> stringResource(R.string.upcoming)
    ViewTitle.Archive -> stringResource(R.string.archive)
    ViewTitle.Search -> stringResource(R.string.search)
    is ViewTitle.Named -> title.name
}

@Composable fun sectionText(section: Section): String? = section.group?.let { group ->
    when(group) {
        TaskGroup.Today -> stringResource(R.string.today)
        TaskGroup.Morning -> stringResource(R.string.morning)
        TaskGroup.Evening -> stringResource(R.string.evening)
        is TaskGroup.Project -> group.title
        is TaskGroup.Tag -> "#${group.title}"
        is TaskGroup.Estimate -> when(group.range) {
            EstimateRange.UP_TO15_MINUTES -> "≤ 15 min"
            EstimateRange.UP_TO30_MINUTES -> "15–30 min"
            EstimateRange.UP_TO60_MINUTES -> "30–60 min"
            EstimateRange.UP_TO2_HOURS -> "1–2 h"
            EstimateRange.OVER2_HOURS -> "> 2 h"
            EstimateRange.NO_ESTIMATE -> stringResource(R.string.reminder_none).let { stringResource(R.string.estimate) + ": " + stringResource(R.string.none) }
        }
        TaskGroup.NoProject -> stringResource(R.string.no_project)
        TaskGroup.Untagged -> stringResource(R.string.untagged)
    }
} ?: when(val kind = section.kind) {
    SectionKind.Plain -> null
    SectionKind.Overdue -> stringResource(R.string.overdue)
    SectionKind.Morning -> stringResource(R.string.morning)
    SectionKind.Today -> stringResource(R.string.today)
    SectionKind.Tonight -> stringResource(R.string.evening)
    is SectionKind.Day -> dayText(kind.label)
    SectionKind.Completed -> stringResource(R.string.completed)
    SectionKind.SearchTasks -> stringResource(R.string.search)
    SectionKind.SearchProjects -> stringResource(R.string.projects)
    SectionKind.SearchTags -> stringResource(R.string.tags)
    SectionKind.SearchArchived -> stringResource(R.string.search_archived)
}

@Composable fun dayText(day: DayLabel): String {
    val locale = LocalLocale.current.platformLocale
    return when(day.relation) {
        DayRelation.TODAY -> stringResource(R.string.today)
        DayRelation.TOMORROW -> stringResource(R.string.tomorrow)
        else -> runCatching { LocalDate.parse(day.day).format(DateTimeFormatter.ofLocalizedDate(FormatStyle.MEDIUM).withLocale(locale)) }.getOrDefault(day.day)
    }
}

@Composable fun emptyText(empty: EmptyState?): String = stringResource(when(empty) {
    EmptyState.Today, EmptyState.Morning, EmptyState.Tonight -> R.string.empty_today
    EmptyState.Search -> R.string.empty_search
    EmptyState.NoResults -> R.string.no_results
    EmptyState.Archive -> R.string.empty_archive
    is EmptyState.Upcoming -> R.string.empty_upcoming
    is EmptyState.Project, is EmptyState.Tag, null -> R.string.empty_list
})

@Composable fun messageText(message: Message): String {
    val locale = LocalLocale.current.platformLocale
    val movedDay = when(message) { is Message.MovedToDay -> message.day; is Message.TasksMovedToDay -> message.day; else -> null }
    if(movedDay != null) {
        val day = runCatching { LocalDate.parse(movedDay).format(DateTimeFormatter.ofLocalizedDate(FormatStyle.MEDIUM).withLocale(locale)) }.getOrDefault(movedDay)
        return stringResource(R.string.moved_to_day, day)
    }
    return stringResource(when(message) {
    is Message.SaveFailed -> R.string.save_failed
    Message.TaskCompleted, Message.TaskCompletedArchived, is Message.TasksCompleted, is Message.TasksCompletedArchived -> R.string.action_completed
    Message.TaskDeleted, is Message.TasksDeleted -> R.string.action_deleted
    Message.TaskAdded, is Message.TasksAdded -> R.string.action_added
    Message.TaskDuplicated -> R.string.duplicate
    Message.MovedToTonight, Message.PlannedForTonight, is Message.TasksMovedToTonight -> R.string.evening
    Message.MovedToMorning, Message.PlannedForMorning, is Message.TasksMovedToMorning -> R.string.morning
    Message.MovedToToday, Message.PlannedForToday, is Message.TasksMovedToToday, is Message.TasksPlannedForToday -> R.string.today
    Message.MovedToTomorrow, is Message.TasksMovedToTomorrow -> R.string.tomorrow
    Message.MovedToNextWeek, is Message.TasksMovedToNextWeek -> R.string.next_week
    is Message.MovedToDay, is Message.TasksMovedToDay, Message.RemovedFromToday,
    is Message.MovedToProject, is Message.Tagged, is Message.TasksTagged, is Message.Snoozed -> R.string.action_changed
    is Message.Archived -> R.string.action_archived
    Message.Undone -> R.string.action_undone
    Message.NothingToUndo -> R.string.nothing_undo
    Message.ManualOrderOnly -> R.string.manual_only
    Message.PickAWeekday -> R.string.pick_weekday
    is Message.NoLongerRepeats -> R.string.no_longer_repeat
    is Message.RepeatSaved -> R.string.repeat_saved
    Message.BackupImported -> R.string.backup_restored
    Message.BackupExported -> R.string.backup_exported
    is Message.Synced -> R.string.sync
    is Message.LinkedWith -> R.string.linked
    Message.Linking -> R.string.linking
    Message.LinkFailed, Message.LinkDeclined, is Message.IdentityChanged, is Message.NearbyStartFailed -> R.string.link_failed
    })
}
