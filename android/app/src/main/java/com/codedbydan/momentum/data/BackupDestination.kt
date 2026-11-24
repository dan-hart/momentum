// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.data

import java.io.File
import java.io.IOException
import java.io.InputStream
import java.security.MessageDigest

internal enum class BackupDestination { Verified, ReadbackUnavailable, Mismatch }

internal fun verifyBackupDestination(source: File, openDestination: () -> InputStream?): BackupDestination {
    val expected = source.inputStream().use { it.sha256() }
    val actual = try {
        openDestination()?.use { it.sha256() }
    } catch(_: SecurityException) {
        null
    } catch(_: IOException) {
        null
    }
    return when {
        actual == null -> BackupDestination.ReadbackUnavailable
        expected.contentEquals(actual) -> BackupDestination.Verified
        else -> BackupDestination.Mismatch
    }
}

private fun InputStream.sha256(): ByteArray {
    val digest = MessageDigest.getInstance("SHA-256")
    val buffer = ByteArray(8192)
    while(true) {
        val count = read(buffer)
        if(count == -1) return digest.digest()
        digest.update(buffer, 0, count)
    }
}
