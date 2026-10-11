// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import android.app.TimePickerDialog
import android.view.ContextThemeWrapper
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.*
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.platform.LocalContext
import com.codedbydan.momentum.core.ClockTime

/** Compose owns the framework dialog's lifetime; Android owns locale, clock and accessibility. */
@Composable internal fun NativeTimePicker(
    initial: ClockTime,
    onSet: (ClockTime) -> Unit,
    onDismiss: () -> Unit,
) {
    val context = LocalContext.current
    val dark = MaterialTheme.colorScheme.surface.luminance() < 0.5f
    val clock24 = android.text.format.DateFormat.is24HourFormat(context)
    val currentSet by rememberUpdatedState(onSet)
    val currentDismiss by rememberUpdatedState(onDismiss)
    DisposableEffect(context, initial, dark, clock24) {
        val themed = ContextThemeWrapper(context, if(dark) android.R.style.Theme_Material_Dialog_Alert
            else android.R.style.Theme_Material_Light_Dialog_Alert)
        val dialog = TimePickerDialog(themed, { _, hour, minute ->
            currentSet(ClockTime(hour.toUInt(), minute.toUInt()))
        }, initial.hour.toInt(), initial.minute.toInt(), clock24)
        dialog.setOnDismissListener { currentDismiss() }
        dialog.show()
        onDispose {
            dialog.setOnDismissListener(null)
            dialog.dismiss()
        }
    }
}
