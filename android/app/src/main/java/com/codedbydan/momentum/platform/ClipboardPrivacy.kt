// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.platform

import android.content.ClipData
import android.content.ClipDescription
import android.os.Build
import android.os.PersistableBundle

/** Explicit copying still works; Android hides private task content in its copy preview. */
internal fun sensitivePlainText(label: CharSequence, text: CharSequence): ClipData =
    ClipData.newPlainText(label, text).markSensitive()

internal fun ClipData.markSensitive(): ClipData = apply {
        description.extras = PersistableBundle().apply {
            putBoolean(
                if (Build.VERSION.SDK_INT >= 33) ClipDescription.EXTRA_IS_SENSITIVE
                else "android.content.extra.IS_SENSITIVE",
                true,
            )
        }
    }
