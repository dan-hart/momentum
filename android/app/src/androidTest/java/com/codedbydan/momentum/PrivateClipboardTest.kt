// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import android.content.BroadcastReceiver
import android.content.ClipboardManager
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.Build
import android.view.View
import androidx.compose.material3.Text
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import com.codedbydan.momentum.platform.ClipboardTextTooLargeException
import com.codedbydan.momentum.platform.PrivateClipboard
import com.codedbydan.momentum.platform.ClipboardUriUnsupportedException
import kotlinx.coroutines.runBlocking
import org.junit.*
import org.junit.runner.RunWith
import java.io.File
import java.util.UUID
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

@RunWith(AndroidJUnit4::class)
class PrivateClipboardTest {
    @get:Rule val compose = createComposeRule()
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()
    private lateinit var clipboard: ClipboardManager
    private lateinit var senderView: View
    private var disposable = false
    @Before fun foreground() {
        Assume.assumeTrue("Clipboard mutation requires an explicitly owned emulator",
            InstrumentationRegistry.getArguments().getString("momentum.disposableOsTests") == "true" &&
                Build.HARDWARE in listOf("ranchu", "goldfish"))
        disposable = true
        compose.runOnUiThread { clipboard = app.getSystemService(ClipboardManager::class.java) }
        compose.setContent { senderView = LocalView.current; Text("Disposable clipboard sender") }
        compose.waitUntil(10_000) { senderView.hasWindowFocus() }
    }
    @After fun cleanup() {
        if(!disposable || !::senderView.isInitialized) return
        compose.waitUntil(10_000) { senderView.hasWindowFocus() }
        runBlocking { PrivateClipboard.copy(app, "Fixture cleared", "") }
    }

    @Test fun fullAsciiAndUnicodeAboveBinderLimitPasteInAnotherAppWithReadOnlyGrant() {
        Assume.assumeTrue("Android 8 has a confirmed system clipboard URI grant bug", Build.VERSION.SDK_INT >= 28)
        for((fixture, text) in listOf("ascii" to "A".repeat(2 * 1024 * 1024), "unicode" to "📝".repeat(270 * 1024))) {
            runBlocking { PrivateClipboard.copy(app, "Private fixture", text) }
            val clip = clipboard.primaryClip!!
            Assert.assertTrue(clip.description.extras!!.getBoolean("android.content.extra.IS_SENSITIVE"))
            Assert.assertNull(clip.getItemAt(0).text)
            val uri = clip.getItemAt(0).uri!!
            val action = "com.codedbydan.momentum.test.CLIP_RESULT_${UUID.randomUUID()}"
            val latch = CountDownLatch(1)
            var result: Intent? = null
            val receiver = object : BroadcastReceiver() {
                override fun onReceive(context: Context, intent: Intent) { result = intent; latch.countDown() }
            }
            if(Build.VERSION.SDK_INT >= 33) app.registerReceiver(receiver, IntentFilter(action), Context.RECEIVER_EXPORTED)
            else app.registerReceiver(receiver, IntentFilter(action))
            try {
                val testPackage = InstrumentationRegistry.getInstrumentation().context.packageName
                app.startActivity(Intent().setComponent(ComponentName(testPackage, ClipboardRecipientActivity::class.java.name))
                    .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK).putExtra("resultAction", action)
                    .putExtra("targetPackage", app.packageName).putExtra("uri", uri.toString()).putExtra("fixture", fixture))
                Assert.assertTrue("Separate recipient returned a result", latch.await(30, TimeUnit.SECONDS))
                val received = result!!
                Assert.assertNull(received.getStringExtra("failure"))
                Assert.assertNotEquals("Recipient uses a separate app UID", app.applicationInfo.uid, received.getIntExtra("uid", -1))
                Assert.assertFalse("No prior URI grant", received.getBooleanExtra("readBefore", true))
                Assert.assertTrue("Clipboard grants read", received.getBooleanExtra("readAfter", false))
                Assert.assertFalse("Clipboard never grants write", received.getBooleanExtra("writeAfter", true))
                Assert.assertTrue("Complete source text pasted", received.getBooleanExtra("exact", false))
                Assert.assertEquals(text.length, received.getIntExtra("length", -1))
            } finally { app.unregisterReceiver(receiver) }
            compose.waitUntil(10_000) { senderView.hasWindowFocus() }
        }
    }

    @Test fun reconstructionRetainsFullClipAndProviderRejectsWritesAndUnboundedPaste() {
        Assume.assumeTrue("Android 8 uses explicit full-notes sharing instead", Build.VERSION.SDK_INT >= 28)
        val text = "📝".repeat(270 * 1024)
        runBlocking { PrivateClipboard.copy(app, "Private fixture", text) }
        val uri = clipboard.primaryClip!!.getItemAt(0).uri!!
        val recreatedContext = app.createPackageContext(app.packageName, 0)
        Assert.assertEquals(text, runBlocking { PrivateClipboard.readOwned(recreatedContext, uri, text.length) })
        Assert.assertThrows(Exception::class.java) { app.contentResolver.openOutputStream(uri)!!.use { it.write(0) } }
        Assert.assertThrows(ClipboardTextTooLargeException::class.java) {
            runBlocking { PrivateClipboard.readOwned(app, uri, text.length - 1) }
        }
        Assert.assertEquals(text, runBlocking { PrivateClipboard.readOwned(app, uri, text.length) })
    }

    @Test fun capacityFailurePreservesActiveClipAndEveryEarlierPayload() {
        Assume.assumeTrue("Android 8 rejects URI clipboard copies before allocating files", Build.VERSION.SDK_INT >= 28)
        val text = "Retained full private text\n".repeat(4000)
        runBlocking { PrivateClipboard.copy(app, "Private fixture", text) }
        val original = clipboard.primaryClip!!.getItemAt(0).uri!!
        val directory = File(app.filesDir, "clipboard-payloads")
        repeat(31) { File(directory, "${UUID.randomUUID()}.txt").writeText("Disposable orphan $it") }
        val before = directory.listFiles()!!.associate { it.name to it.readBytes().toList() }
        Assert.assertThrows(IllegalStateException::class.java) {
            runBlocking { PrivateClipboard.copy(app, "Rejected new fixture", "Different text\n".repeat(9000)) }
        }
        Assert.assertEquals(original, clipboard.primaryClip!!.getItemAt(0).uri)
        Assert.assertEquals(before, directory.listFiles()!!.associate { it.name to it.readBytes().toList() })
        Assert.assertEquals(text, runBlocking { PrivateClipboard.readOwned(app, original, text.length) })
    }

    @Test fun androidEightRejectsLargeCopyBeforeChangingClipboardOrAllocatingPayloads() {
        Assume.assumeTrue(Build.VERSION.SDK_INT <= 27)
        runBlocking { PrivateClipboard.copy(app, "Retained fixture", "Previous clipboard") }
        val directory = File(app.filesDir, "clipboard-payloads")
        val before = directory.listFiles().orEmpty().associate { it.name to it.readBytes().toList() }
        Assert.assertThrows(ClipboardUriUnsupportedException::class.java) {
            runBlocking { PrivateClipboard.copy(app, "Rejected fixture", "📝".repeat(270 * 1024)) }
        }
        Assert.assertEquals("Previous clipboard", clipboard.primaryClip!!.getItemAt(0).text.toString())
        Assert.assertEquals(before, directory.listFiles().orEmpty().associate { it.name to it.readBytes().toList() })
    }
}
