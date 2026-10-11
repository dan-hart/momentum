// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.platform

import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.*
import org.junit.Assert.*
import org.junit.Test

class PlatformPolicyTest {
    @Test fun backgroundAdmissionRequiresTheSelectedProviderAndBothAutomaticOptIns() {
        for (provider in SyncProvider.entries) for (automatic in listOf(false, true)) for (background in listOf(false, true)) {
            assertEquals(provider == SyncProvider.Nextcloud && automatic && background,
                AppSettings(provider = provider, automaticSync = automatic, backgroundSync = background).backgroundSyncEligible())
        }
    }

    @Test fun typedCoreFailureKeepsTheSentenceSeparateFromTheEnumName() {
        val transient = PlatformFailure.from(CoreException.Transient(SyncFailureKind.UNAUTHORIZED, "The server refused this app password"))
        assertEquals(SyncFailureKind.UNAUTHORIZED, transient.kind)
        assertEquals("The server refused this app password", transient.details)
        val actionable = PlatformFailure.from(CoreException.Actionable(SyncFailureKind.REMOTE_FILE_DAMAGED, "The server copy is damaged"))
        assertEquals(SyncFailureKind.REMOTE_FILE_DAMAGED, actionable.kind)
        assertEquals("The server copy is damaged", actionable.details)
    }
}
