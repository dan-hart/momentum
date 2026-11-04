// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import com.codedbydan.momentum.core.NextcloudSettings
import org.junit.Assert.*
import org.junit.Test

class SyncSettingsPresentationTest {
    private val saved = NextcloudSettings("https://cloud.example.com", "alex", "app-password", "momentum", true, null, false)

    @Test fun savedActionsRequireEveryDraftFieldToMatch() {
        assertTrue(savedConnectionMatchesDraft(saved, saved))
        assertFalse(savedConnectionMatchesDraft(null, saved))
        listOf(saved.copy(serverUrl = "https://other.example.com"), saved.copy(userName = "other"),
            saved.copy(password = "edited"), saved.copy(folder = "other"), saved.copy(compress = false),
            saved.copy(encryptionPassword = "secret")).forEach {
            assertFalse(savedConnectionMatchesDraft(saved, it))
        }
        assertTrue(savedConnectionMatchesDraft(saved, saved.copy(serverUrl = " ${saved.serverUrl} ", userName = " alex ", folder = " momentum ")))
    }

    @Test fun advancedSearchAndRecoveryRevealHiddenOptions() {
        assertFalse(revealAdvancedNextcloud("sync.nextcloud", false, false))
        assertFalse(revealAdvancedNextcloud("sync.automatic", false, false))
        assertTrue(revealAdvancedNextcloud("sync.compression", false, false))
        assertTrue(revealAdvancedNextcloud("sync.folder-copy", false, false))
        assertTrue(revealAdvancedNextcloud(null, true, false))
        assertTrue(revealAdvancedNextcloud(null, false, true))
    }
}
