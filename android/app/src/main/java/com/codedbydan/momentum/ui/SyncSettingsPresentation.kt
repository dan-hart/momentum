// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import com.codedbydan.momentum.core.NextcloudSettings

/** Saved-only actions must never suggest that an edited connection has been tested. */
internal fun savedConnectionMatchesDraft(saved: NextcloudSettings?, draft: NextcloudSettings): Boolean =
    saved != null && saved.serverUrl.trim() == draft.serverUrl.trim() &&
        saved.userName.trim() == draft.userName.trim() && saved.folder.trim() == draft.folder.trim() &&
        saved.password == draft.password && saved.compress == draft.compress &&
        saved.encryptionPassword.orEmpty() == draft.encryptionPassword.orEmpty()

internal fun revealAdvancedNextcloud(destination: String?, migrating: Boolean, migrationFailed: Boolean): Boolean =
    destination in setOf("sync.compression", "sync.folder-copy") || migrating || migrationFailed
