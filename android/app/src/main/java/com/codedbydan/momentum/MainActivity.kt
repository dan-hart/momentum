// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum

import android.content.Intent
import android.os.Bundle
import android.app.KeyguardManager
import android.view.KeyEvent
import android.view.KeyboardShortcutGroup
import android.view.Menu
import androidx.activity.ComponentActivity
import androidx.compose.runtime.CompositionLocalProvider
import com.codedbydan.momentum.ui.KeyboardCommandBridge
import com.codedbydan.momentum.ui.LocalKeyboardCommandBridge
import com.codedbydan.momentum.ui.nativeKeyboardGroups
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.viewModels
import androidx.lifecycle.createSavedStateHandle
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.lifecycle.lifecycleScope
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.repeatOnLifecycle
import com.codedbydan.momentum.ui.MomentumApp
import kotlinx.coroutines.launch
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.flow.map

class MainActivity : ComponentActivity() {
    private val keyboardCommands = KeyboardCommandBridge()
    private val vm: MomentumViewModel by viewModels {
        viewModelFactory { initializer { MomentumViewModel(application, createSavedStateHandle()) } }
    }
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent { CompositionLocalProvider(LocalKeyboardCommandBridge provides keyboardCommands) { MomentumApp(vm) } }
        if(savedInstanceState == null) capture(intent)
        lifecycleScope.launch {
            repeatOnLifecycle(Lifecycle.State.STARTED) {
                launch {
                    (application as MomentumApplication).platform.state.map { it.revision }
                        .distinctUntilChanged().drop(1).collect { vm.refresh() }
                }
                // Day rollover and localized due labels refresh only while visible.
                launch { while(true) { delay(60_000); vm.refresh(); (application as MomentumApplication).platform.refreshTaskCount() } }
            }
        }
    }
    override fun onNewIntent(intent: Intent) { super.onNewIntent(intent); setIntent(intent); capture(intent) }
    override fun onResume() {
        super.onResume()
        if(!getSystemService(KeyguardManager::class.java).isDeviceLocked) vm.resumeQueuedCaptures()
    }
    override fun onStart() { super.onStart(); lifecycleScope.launch { (application as MomentumApplication).platform.foreground(true) }; vm.refresh() }
    override fun onStop() { lifecycleScope.launch { try { vm.persistDraft() } catch(e: Exception) { vm.reportError(e) } }; (application as MomentumApplication).platform.leaveForeground(); super.onStop() }

    private fun capture(intent: Intent) = com.codedbydan.momentum.platform.TaskEntryPoints(this, vm).capture(intent)

    override fun onKeyShortcut(keyCode: Int, event: KeyEvent): Boolean {
        if(!getSystemService(KeyguardManager::class.java).isDeviceLocked &&
            keyboardCommands.dispatch(event, currentFocus?.onCheckIsTextEditor() == true)) return true
        return super.onKeyShortcut(keyCode, event)
    }

    override fun onProvideKeyboardShortcuts(data: MutableList<KeyboardShortcutGroup>, menu: Menu?, deviceId: Int) {
        super.onProvideKeyboardShortcuts(data, menu, deviceId)
        data.addAll(nativeKeyboardGroups(this, vm.state.value.snapshot?.settings?.keyboardModifier?.name ?: "Ctrl"))
    }
}
