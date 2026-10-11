// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.platform

import androidx.test.platform.app.InstrumentationRegistry
import com.codedbydan.momentum.core.NextcloudSettings
import org.junit.Assert.*
import org.junit.Test
import java.io.File
import java.security.KeyStore
import java.util.UUID

class SecretVaultTest {
    @Test fun invalidStoredBase64LatchesFailureAndRefusesReplacementOrDeletion() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val directory = File(context.cacheDir, "vault-test-${UUID.randomUUID()}").also { it.mkdirs() }
        val vault = SecretVault(directory)
        try {
            assertTrue(vault.set("app-key", "malformed-existing-secret"))
            val file = File(directory, "android-secrets.bin")
            val original = file.readBytes()
            var reported = false
            val secrets = VaultSecretStore(vault) { reported = true }
            assertNull(secrets.get("app-key"))
            assertTrue(reported)
            assertFalse(secrets.set("app-key", "cmVwbGFjZW1lbnQ="))
            secrets.delete("app-key")
            assertArrayEquals(original, file.readBytes())
            assertEquals("malformed-existing-secret", vault.get("app-key"))
        } finally {
            KeyStore.getInstance("AndroidKeyStore").apply { load(null); deleteEntry(vault.keyAlias) }
            directory.deleteRecursively()
        }
    }

    @Test fun roundTripAndTamperPreserveOriginalCiphertext() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val directory = File(context.cacheDir, "vault-test-${UUID.randomUUID()}").also { it.mkdirs() }
        val vault = SecretVault(directory)
        try {
            val settings = NextcloudSettings("https://example.org", "test", "test-password", "Momentum", true, "test-encryption")
            vault.saveConnection(settings)
            vault.set("p2p-test", "test-key")
            assertEquals(settings, SecretVault(directory).connection())
            assertEquals("test-key", SecretVault(directory).get("p2p-test"))
            val file = File(directory, "android-secrets.bin")
            assertFalse(file.readText(Charsets.ISO_8859_1).contains("test-password"))
            val tampered = file.readBytes().also { it[it.lastIndex] = (it.last().toInt() xor 1).toByte() }
            file.writeBytes(tampered)
            assertThrows(Exception::class.java) { vault.connection() }
            assertThrows(Exception::class.java) { vault.saveConnection(settings.copy(password = "replacement")) }
            assertFalse(vault.set("another", "replacement"))
            assertArrayEquals(tampered, file.readBytes())
        } finally {
            KeyStore.getInstance("AndroidKeyStore").apply { load(null); deleteEntry(vault.keyAlias) }
            directory.deleteRecursively()
        }
    }
}
