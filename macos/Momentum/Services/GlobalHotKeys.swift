// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// The two system-wide shortcuts the GNOME app takes through the GlobalShortcuts portal:
// ⌃⌥T adds a task from anywhere, ⌃⌥M brings the window forward. Carbon's hot-key API is
// still the only way to register one without an accessibility grant.
import AppKit
import Carbon.HIToolbox

@MainActor
final class GlobalHotKeys {
    private var refs: [EventHotKeyRef?] = []
    private var handler: EventHandlerRef?
    private static var actions: [UInt32: () -> Void] = [:]
    private static let signature = OSType(0x4D4F4D4E) // 'MOMN'

    init(quickAdd: @escaping () -> Void, show: @escaping () -> Void) {
        register(id: 1, keyCode: UInt32(kVK_ANSI_T), action: quickAdd)
        register(id: 2, keyCode: UInt32(kVK_ANSI_M), action: show)
        installHandler()
    }

    private func register(id: UInt32, keyCode: UInt32, action: @escaping () -> Void) {
        var ref: EventHotKeyRef?
        let hotKeyID = EventHotKeyID(signature: Self.signature, id: id)
        let modifiers = UInt32(controlKey | optionKey)
        let status = RegisterEventHotKey(keyCode, modifiers, hotKeyID, GetApplicationEventTarget(), 0, &ref)
        if status == noErr {
            Self.actions[id] = action
            refs.append(ref)
        } else {
            // Another app already owns the combination; the menu items still work.
            NSLog("global shortcut \(id) unavailable (status \(status))")
        }
    }

    private func installHandler() {
        var spec = EventTypeSpec(eventClass: OSType(kEventClassKeyboard), eventKind: UInt32(kEventHotKeyPressed))
        InstallEventHandler(GetApplicationEventTarget(), { _, event, _ -> OSStatus in
            var id = EventHotKeyID()
            let status = GetEventParameter(event, EventParamName(kEventParamDirectObject),
                                           EventParamType(typeEventHotKeyID), nil,
                                           MemoryLayout<EventHotKeyID>.size, nil, &id)
            guard status == noErr else { return status }
            let key = id.id
            DispatchQueue.main.async {
                MainActor.assumeIsolated { GlobalHotKeys.actions[key]?() }
            }
            return noErr
        }, 1, &spec, nil, &handler)
    }

    /// Registered for the life of the process, like any menu shortcut: there is no point
    /// in the app at which it still runs but should stop answering ⌃⌥T.
}
