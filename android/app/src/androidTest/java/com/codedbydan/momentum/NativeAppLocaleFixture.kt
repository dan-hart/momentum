// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import android.app.LocaleManager
import android.os.LocaleList
import androidx.activity.ComponentActivity
import androidx.annotation.RequiresApi
import androidx.test.platform.app.InstrumentationRegistry
import java.util.Locale

/** Changes only this disposable app's locales and restores the original selection. */
@RequiresApi(33)
internal class NativeAppLocaleFixture(private val activity: ComponentActivity) {
    private val manager = activity.getSystemService(LocaleManager::class.java)
    private val originalLocales = manager.applicationLocales
    private val originalEffectiveTag = activity.resources.configuration.locales[0].toLanguageTag()
    private var expectedTag = originalEffectiveTag

    val isApplied: Boolean
        get() = activity.resources.configuration.locales[0].toLanguageTag() == expectedTag

    fun apply(locale: Locale) {
        expectedTag = locale.toLanguageTag()
        InstrumentationRegistry.getInstrumentation().runOnMainSync {
            manager.applicationLocales = LocaleList(locale)
        }
    }

    fun restore() {
        expectedTag = originalEffectiveTag
        InstrumentationRegistry.getInstrumentation().runOnMainSync {
            manager.applicationLocales = originalLocales
        }
    }
}
