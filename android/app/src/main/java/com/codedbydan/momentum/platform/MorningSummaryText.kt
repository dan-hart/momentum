// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.platform

import android.content.Context
import android.icu.text.ListFormatter
import com.codedbydan.momentum.R
import com.codedbydan.momentum.core.MorningSummary

internal enum class SummarySection(val bit: Int, val label: Int, val countLabel: Int) {
    Today(1, R.string.today, R.string.summary_today_count),
    Morning(2, R.string.morning, R.string.summary_morning_count),
    Evening(4, R.string.evening, R.string.summary_evening_count),
    Overdue(8, R.string.overdue, R.string.summary_overdue_count);

    fun isSelected(mask: UByte): Boolean = mask.toInt() and bit != 0

    fun count(summary: MorningSummary): UInt = when (this) {
        Today -> summary.total
        Morning -> summary.morning
        Evening -> summary.tonight
        Overdue -> summary.overdue
    }
}

internal fun morningSummaryText(context: Context, summary: MorningSummary): String {
    val selected = SummarySection.entries.filter { it.isSelected(summary.sections) }
    return ListFormatter.getInstance(context.resources.configuration.locales[0]).format(
        selected.map { context.getString(it.countLabel, it.count(summary).toLong()) }
    )
}
