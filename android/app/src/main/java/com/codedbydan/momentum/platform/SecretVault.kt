// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.platform

import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.AtomicFile
import com.codedbydan.momentum.core.NextcloudSettings
import com.codedbydan.momentum.core.SecretStore
import org.json.JSONObject
import java.io.File
import java.net.URI
import java.security.KeyStore
import java.security.MessageDigest
import java.util.Base64
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/** No hostname resolution or credentials in the URL. Cleartext is only for test stores. */
internal object ConnectionPolicy {
    fun accepts(value: String, isolated: Boolean): Boolean = runCatching {
        val uri = URI(value.trim())
        val host = uri.host?.lowercase() ?: return false
        if (uri.userInfo != null || uri.fragment != null || uri.query != null || (uri.port != -1 && uri.port !in 1..65535)) return false
        when (uri.scheme?.lowercase()) {
            "https" -> true
            "http" -> isolated && host in setOf("127.0.0.1", "localhost", "[::1]", "::1")
            else -> false
        }
    }.getOrDefault(false)

    fun isLocalServer(value: String): Boolean = runCatching {
        val host = URI(value.trim()).host?.lowercase()?.removePrefix("[")?.removeSuffix("]") ?: return false
        if (host == "localhost" || host.endsWith(".localhost") || host.endsWith(".local") || host == "::1" ||
            host.startsWith("fc") && ':' in host || host.startsWith("fd") && ':' in host || host.startsWith("fe80:") ) return true
        val parts = host.split('.')
        if (parts.size != 4) return false
        val octets = parts.map { it.toIntOrNull()?.takeIf { value -> value in 0..255 } ?: return false }
        octets.size == 4 && (octets[0] in setOf(0, 10, 127) || octets.take(2) == listOf(169, 254) ||
            octets.take(2) == listOf(192, 168) || octets[0] == 172 && octets[1] in 16..31)
    }.getOrDefault(false)
}

/** Authenticated private ciphertext. Failed reads/saves preserve both data and key. */
internal class SecretVault(private val directory: File) {
    val keyAlias = "momentum.secrets." + digest(directory.canonicalPath)
    private val file = AtomicFile(File(directory, "android-secrets.bin"))
    private val aad = keyAlias.toByteArray(Charsets.UTF_8)

    @Synchronized fun checkReadable() {
        val secrets = load().optJSONObject("secrets") ?: return
        val cert = secrets.has("device-cert-der")
        val key = secrets.has("device-key-der")
        check(cert == key && (!cert || secrets.has("app-key"))) { "Stored nearby credentials are incomplete; existing encrypted data was preserved" }
    }
    @Synchronized fun get(name: String): String? = load().optJSONObject("secrets")?.let {
        if (it.has(name)) it.getString(name) else null
    }
    @Synchronized fun set(name: String, value: String): Boolean = runCatching {
        val document = load()
        val secrets = document.optJSONObject("secrets") ?: JSONObject().also { document.put("secrets", it) }
        secrets.put(name, value)
        save(document)
        true
    }.getOrDefault(false)
    @Synchronized fun delete(name: String) {
        val document = load()
        document.optJSONObject("secrets")?.remove(name)
        save(document)
    }
    @Synchronized fun connection(): NextcloudSettings? {
        val document = load()
        val json = document.optJSONObject("nextcloud") ?: return null
        val identity = com.codedbydan.momentum.core.nextcloudConnectionIdentity(json.getString("server"), json.getString("user"), json.getString("folder"))
        val approved = document.optJSONArray("optimizedConnections")
        val remembered = approved != null && (0 until approved.length()).any { approved.optString(it) == identity }
        return NextcloudSettings(json.getString("server"), json.getString("user"), json.getString("password"),
            json.getString("folder"), json.getBoolean("compress"),
            if (json.isNull("encryption")) null else json.getString("encryption"), json.optBoolean("optimized", false) || remembered)
    }
    @Synchronized fun saveConnection(settings: NextcloudSettings) {
        val document = load() // Validate existing ciphertext BEFORE replacing it.
        val identity = com.codedbydan.momentum.core.nextcloudConnectionIdentity(settings.serverUrl, settings.userName, settings.folder)
        val approved = document.optJSONArray("optimizedConnections") ?: org.json.JSONArray()
        val remembered = (0 until approved.length()).any { approved.optString(it) == identity }
        if (settings.optimized && !remembered) approved.put(identity)
        document.put("optimizedConnections", approved)
        document.put("nextcloud", JSONObject().put("server", settings.serverUrl).put("user", settings.userName)
            .put("password", settings.password).put("folder", settings.folder).put("compress", settings.compress)
            .put("encryption", settings.encryptionPassword ?: JSONObject.NULL).put("optimized", settings.optimized || remembered))
        save(document)
    }

    private fun load(): JSONObject {
        if (!file.baseFile.exists() && !File(file.baseFile.path + ".bak").exists()) return JSONObject().put("version", 1)
        val bytes = file.openRead().use { stream ->
            check(stream.available() in 30..8_388_608) { "Encrypted credentials are invalid; existing data was preserved" }
            stream.readBytes()
        }
        check(bytes.take(7).toByteArray().contentEquals("MOMSEC1".toByteArray())) { "Unsupported encrypted credentials; existing data was preserved" }
        val ivLength = bytes[7].toInt() and 255
        check(ivLength == 12 && bytes.size > 8 + ivLength + 15)
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.DECRYPT_MODE, key(create = false), GCMParameterSpec(128, bytes.copyOfRange(8, 8 + ivLength)))
        cipher.updateAAD(aad)
        val plain = cipher.doFinal(bytes.copyOfRange(8 + ivLength, bytes.size))
        val document = JSONObject(plain.toString(Charsets.UTF_8))
        check(document.getInt("version") == 1) { "Unsupported encrypted credentials; existing data was preserved" }
        if (document.has("secrets")) document.getJSONObject("secrets")
        if (document.has("nextcloud")) document.getJSONObject("nextcloud")
        return document
    }

    private fun save(document: JSONObject) {
        check(directory.exists() || directory.mkdirs())
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, key(create = true))
        cipher.updateAAD(aad)
        val encrypted = cipher.doFinal(document.toString().toByteArray(Charsets.UTF_8))
        val stream = file.startWrite()
        try {
            stream.write("MOMSEC1".toByteArray()); stream.write(cipher.iv.size); stream.write(cipher.iv); stream.write(encrypted)
            file.finishWrite(stream)
        } catch (e: Exception) { file.failWrite(stream); throw e }
    }

    private fun key(create: Boolean): SecretKey {
        val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        (store.getKey(keyAlias, null) as? SecretKey)?.let { return it }
        check(create) { "Android credential key is unavailable; existing encrypted data was preserved" }
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore").apply {
            init(KeyGenParameterSpec.Builder(keyAlias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM).setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setRandomizedEncryptionRequired(true).setKeySize(256).build())
        }.generateKey()
    }
}

/** Optional reads cannot represent failure. Latch it so Rust cannot replace bad keys. */
internal class VaultSecretStore(private val vault: SecretVault, private val report: (Throwable) -> Unit) : SecretStore {
    @Volatile var failure: Throwable? = null
        private set
    @Synchronized override fun get(name: String): String? {
        if (failure != null) return null
        return try {
            vault.get(name)?.filterNot { it.isWhitespace() }?.also {
                check(Base64.getDecoder().decode(it).isNotEmpty()) { "Stored nearby credentials are invalid; existing encrypted data was preserved" }
            }
        } catch (e: Exception) { fail(e); null }
    }
    @Synchronized override fun set(name: String, value: String): Boolean {
        if (failure != null) return false
        return if (vault.set(name, value)) true else {
            fail(IllegalStateException("Android credentials could not be saved; existing encrypted data was preserved"))
            false
        }
    }
    @Synchronized override fun delete(name: String) {
        if (failure != null) return
        try { vault.delete(name) } catch (e: Exception) { fail(e) }
    }
    private fun fail(error: Throwable) { failure = error; report(error) }
}

internal fun digest(value: String): String = MessageDigest.getInstance("SHA-256").digest(value.toByteArray(Charsets.UTF_8))
    .joinToString("") { "%02x".format(it) }
