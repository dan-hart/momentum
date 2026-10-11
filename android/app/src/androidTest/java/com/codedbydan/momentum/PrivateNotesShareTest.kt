// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import android.content.*
import android.net.Uri
import android.os.Build
import android.os.SystemClock
import android.view.View
import android.view.MotionEvent
import android.view.accessibility.AccessibilityNodeInfo
import android.graphics.Rect
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import com.codedbydan.momentum.platform.*
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import org.junit.*
import org.junit.runner.RunWith
import java.io.File
import java.util.UUID
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

@RunWith(AndroidJUnit4::class)
class PrivateNotesShareTest {
    @get:Rule val compose = createComposeRule()
    private val app get() = ApplicationProvider.getApplicationContext<MomentumApplication>()
    private lateinit var senderView: View
    private val created = mutableListOf<File>()
    private val shareDirectory get() = File(app.cacheDir, "shared-notes")
    private var disposable = false
    private var priorFiles: Set<String> = emptySet()

    @Before fun foreground() {
        Assume.assumeTrue("Sharing requires an explicitly owned emulator and disposable recipient",
            InstrumentationRegistry.getArguments().getString("momentum.disposableOsTests") == "true" &&
                Build.HARDWARE in listOf("ranchu", "goldfish"))
        disposable = true
        priorFiles = shareDirectory.listFiles().orEmpty().map { it.name }.toSet()
        compose.setContent {
            senderView = LocalView.current
            val scope = rememberCoroutineScope()
            TextButton(onClick = { scope.launch { PrivateNotesShare.send(app, "Private fixture", "📝".repeat(270 * 1024)) } }) {
                Text("Share disposable full notes")
            }
        }
        compose.waitUntil(10_000) { senderView.hasWindowFocus() }
    }
    @After fun cleanup() {
        if(!disposable) return
        // Only this explicitly owned fixture creates notes shares while it runs. Include its
        // newly prepared chooser file even if target selection fails; preserve all prior files.
        val fixtureFiles = created + shareDirectory.listFiles().orEmpty().filter { it.name !in priorFiles }
        for(file in fixtureFiles.distinctBy { it.name }) {
            val uri = androidx.core.content.FileProvider.getUriForFile(app, "${app.packageName}.clipboard", file)
            app.revokeUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION)
            file.delete()
        }
    }
    private fun rememberUri(uri: Uri) {
        Assert.assertTrue(PrivateNotesShare.owns(app, uri))
        created += File(shareDirectory, uri.lastPathSegment!!)
    }
    private fun prepare(text: String) = runBlocking { PrivateNotesShare.prepare(app, "Private fixture", text) }.also {
        rememberUri(it.clipData!!.getItemAt(0).uri!!)
    }
    private fun receive(action: String, invoke: () -> Unit): Intent {
        val latch = CountDownLatch(1)
        var result: Intent? = null
        val receiver = object : BroadcastReceiver() {
            override fun onReceive(context: Context, intent: Intent) { result = intent; latch.countDown() }
        }
        if(Build.VERSION.SDK_INT >= 33) app.registerReceiver(receiver, IntentFilter(action), Context.RECEIVER_EXPORTED)
        else app.registerReceiver(receiver, IntentFilter(action))
        try {
            invoke()
            Assert.assertTrue("Separate recipient returned full-text status", latch.await(30, TimeUnit.SECONDS))
            compose.waitUntil(10_000) { senderView.hasWindowFocus() }
            return result!!
        } finally { app.unregisterReceiver(receiver) }
    }
    private fun assertReadOnlyExact(received: Intent, expectedLength: Int) {
        Assert.assertNull(received.getStringExtra("failure"))
        Assert.assertNotEquals(app.applicationInfo.uid, received.getIntExtra("uid", -1))
        Assert.assertTrue("Intent grants read before recipient access", received.getBooleanExtra("readBefore", false))
        Assert.assertTrue("Read grant stays available", received.getBooleanExtra("readAfter", false))
        Assert.assertFalse("Sharing never grants write", received.getBooleanExtra("writeAfter", true))
        Assert.assertTrue("Complete source text received", received.getBooleanExtra("exact", false))
        Assert.assertEquals(expectedLength, received.getIntExtra("length", -1))
    }

    @Test fun fullAsciiAndUnicodeShareAcrossAppUidWithReadOnlyGrantAndReconstructedContext() {
        for((fixture, text) in listOf("ascii" to "A".repeat(2 * 1024 * 1024), "unicode" to "📝".repeat(270 * 1024))) {
            val intent = prepare(text)
            Assert.assertFalse("No full text travels in the Intent", intent.hasExtra(Intent.EXTRA_TEXT))
            val uri = intent.clipData!!.getItemAt(0).uri!!
            val recreated = app.createPackageContext(app.packageName, 0)
            Assert.assertEquals(text, recreated.contentResolver.openInputStream(uri)!!.bufferedReader().use { it.readText() })
            Assert.assertThrows(Exception::class.java) { app.contentResolver.openOutputStream(uri)!!.use { it.write(0) } }
            val action = "com.codedbydan.momentum.test.NOTES_RESULT_${UUID.randomUUID()}"
            val testPackage = InstrumentationRegistry.getInstrumentation().context.packageName
            val result = receive(action) {
                app.startActivity(intent.setComponent(ComponentName(testPackage, ClipboardRecipientActivity::class.java.name))
                    .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK).putExtra("resultAction", action)
                    .putExtra("targetPackage", app.packageName).putExtra("fixture", fixture))
            }
            assertReadOnlyExact(result, text.length)
        }
    }

    @Test fun nativeChooserSharesFullUnicodeOnlyAfterExplicitUserSelection() {
        val text = "📝".repeat(270 * 1024)
        val result = receive("com.codedbydan.momentum.test.NOTES_SHARE_RESULT") {
            compose.onNodeWithText("Share disposable full notes").performClick()
            val automation = InstrumentationRegistry.getInstrumentation().uiAutomation
            fun visibleRecipient() = automation.rootInActiveWindow
                ?.findAccessibilityNodeInfosByText("Disposable notes recipient")
                ?.firstOrNull { it.isVisibleToUser && it.isEnabled && it.text?.toString() == "Disposable notes recipient" }
            var recipient: AccessibilityNodeInfo? = null
            compose.waitUntil(10_000) {
                automation.rootInActiveWindow?.packageName?.toString() in
                    setOf("android", "com.android.intentresolver", "com.google.android.intentresolver")
            }
            automation.waitForIdle(250, 10_000)
            recipient = visibleRecipient()
            if(Build.VERSION.SDK_INT >= 29 || recipient == null) {
                // Modern Sharesheets expose full accessibility labels even when their tile is
                // clipped by the collapsed sheet/navigation bar. Expand the real native sheet
                // before selection, including when that label is already accessible.
                val screen = Rect().also { automation.rootInActiveWindow!!.getBoundsInScreen(it) }
                val start = SystemClock.uptimeMillis()
                val x = screen.exactCenterX()
                val from = screen.top + screen.height() * .72f
                val to = screen.top + screen.height() * .30f
                fun touch(action: Int, y: Float) {
                    val event = MotionEvent.obtain(start, SystemClock.uptimeMillis(), action, x, y, 0)
                    event.source = android.view.InputDevice.SOURCE_TOUCHSCREEN
                    try { Assert.assertTrue("Native chooser expansion gesture is delivered", automation.injectInputEvent(event, true)) }
                    finally { event.recycle() }
                }
                touch(MotionEvent.ACTION_DOWN, from)
                for(step in 1..12) {
                    SystemClock.sleep(16)
                    touch(MotionEvent.ACTION_MOVE, from + (to - from) * step / 12)
                }
                touch(MotionEvent.ACTION_UP, to)
            }
            compose.waitUntil(10_000) { recipient = visibleRecipient(); recipient != null }
            // The label may already be accessible while the Sharesheet is moving on screen.
            // Wait for native accessibility idle, then fetch its current touch bounds again.
            automation.waitForIdle(250, 10_000)
            recipient = visibleRecipient()
            Assert.assertNotNull("Disposable recipient remains visible after native chooser settles", recipient)
            var tile = recipient
            while(tile != null && !tile.isClickable) tile = tile.parent
            Assert.assertNotNull("Exact recipient label belongs to a clickable native tile", tile)
            val target = requireNotNull(tile)
            Assert.assertTrue("Exact recipient tile remains enabled and visible", target.isEnabled && target.isVisibleToUser)
            val bounds = Rect().also { target.getBoundsInScreen(it) }
            val window = Rect().also { automation.rootInActiveWindow!!.getBoundsInScreen(it) }
            Assert.assertTrue("Expanded recipient tile fits fully inside the native chooser window",
                !bounds.isEmpty && window.contains(bounds))
            // Android 17's Sharesheet can report a clickable ancestor whose accessibility
            // ACTION_CLICK is unhandled. Deliver a real touchscreen gesture to its tile center;
            // the two-line accessibility label's center may lie outside a collapsed touch area.
            val time = SystemClock.uptimeMillis()
            for((action, whenMs) in listOf(MotionEvent.ACTION_DOWN to time, MotionEvent.ACTION_UP to time + 30)) {
                val event = MotionEvent.obtain(time, whenMs, action, bounds.exactCenterX(), bounds.exactCenterY(), 0)
                event.source = android.view.InputDevice.SOURCE_TOUCHSCREEN
                try { Assert.assertTrue("Native recipient touch is delivered", automation.injectInputEvent(event, true)) }
                finally { event.recycle() }
            }
        }
        rememberUri(Uri.parse(result.getStringExtra("uri")))
        assertReadOnlyExact(result, text.length)
    }

    @Test fun capacityAndByteLimitFailureKeepEveryPriorPayloadAndNeverGrantWrite() {
        val intent = prepare("Retained full text")
        Assert.assertEquals(0, intent.flags and Intent.FLAG_GRANT_WRITE_URI_PERMISSION)
        shareDirectory.mkdirs()
        repeat(32 - shareDirectory.listFiles().orEmpty().size) {
            File(shareDirectory, "${UUID.randomUUID()}.txt").also { it.writeText("Disposable prior share"); created += it }
        }
        val before = shareDirectory.listFiles()!!.associate { it.name to it.readBytes().toList() }
        Assert.assertThrows(NotesShareCapacityException::class.java) { prepare("Rejected new notes") }
        Assert.assertThrows(IllegalArgumentException::class.java) { prepare("📝".repeat(1_000_001)) }
        Assert.assertEquals(before, shareDirectory.listFiles()!!.associate { it.name to it.readBytes().toList() })
        val uri = intent.clipData!!.getItemAt(0).uri!!
        Assert.assertFalse(PrivateNotesShare.owns(app, uri.buildUpon().appendQueryParameter("unexpected", "x").build()))
        Assert.assertEquals("Retained full text", app.contentResolver.openInputStream(uri)!!.bufferedReader().use { it.readText() })
    }
}
