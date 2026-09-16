#!/usr/bin/env swift
// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
// Read-only macOS counterpart of build-aux/a11y-dump.py. No XCUITest or app host.
import AppKit
import ApplicationServices

let controlRoles: Set<String> = ["AXButton", "AXCheckBox", "AXTextField", "AXTextArea",
    "AXPopUpButton", "AXComboBox", "AXMenuItem", "AXRadioButton", "AXSlider"]

func missingName(role: String, names: [String], subrole: String = "") -> Bool {
    // AppKit's window controls expose semantic subroles instead of app-defined labels.
    // VoiceOver names these itself; custom buttons still require an accessible name.
    let systemWindowControls: Set<String> = ["AXCloseButton", "AXMinimizeButton", "AXZoomButton", "AXFullScreenButton"]
    if role == "AXButton", systemWindowControls.contains(subrole) { return false }
    return controlRoles.contains(role) && !names.contains { !$0.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }
}

if CommandLine.arguments.contains("--self-test") {
    precondition(missingName(role: "AXButton", names: ["", "  "]))
    precondition(!missingName(role: "AXTextField", names: ["Title"]))
    precondition(!missingName(role: "AXButton", names: ["", "Dismiss"]))
    precondition(!missingName(role: "AXImage", names: []))
    precondition(!missingName(role: "AXButton", names: [], subrole: "AXCloseButton"))
    precondition(!missingName(role: "AXButton", names: [], subrole: "AXMinimizeButton"))
    precondition(missingName(role: "AXButton", names: [], subrole: "AXUnknown"))
    print("Accessibility audit checks passed")
    exit(0)
}

func attribute(_ element: AXUIElement, _ name: String) -> CFTypeRef? {
    var value: CFTypeRef?
    guard AXUIElementCopyAttributeValue(element, name as CFString, &value) == .success else { return nil }
    return value
}
func text(_ element: AXUIElement, _ name: String) -> String {
    attribute(element, name) as? String ?? ""
}

let args = CommandLine.arguments
let pid: pid_t
if let index = args.firstIndex(of: "--pid"), index + 1 < args.count, let value = Int32(args[index + 1]) {
    pid = value
} else if let app = NSWorkspace.shared.runningApplications.first(where: { $0.bundleIdentifier == "com.codedbydan.Momentum" }) {
    pid = app.processIdentifier
} else {
    fputs("Momentum is not running. Launch a demo instance first, or pass --pid PID.\n", stderr)
    exit(2)
}
guard AXIsProcessTrusted() else {
    fputs("The terminal running this script needs Accessibility access in System Settings. No permission was requested or changed.\n", stderr)
    exit(2)
}
let root = AXUIElementCreateApplication(pid)
guard let windows = attribute(root, kAXWindowsAttribute) as? [AXUIElement], !windows.isEmpty else {
    fputs("No readable windows. Open a Momentum window or sheet first.\n", stderr)
    exit(2)
}
var visited = Set<CFHashCode>()
var problems = 0
var controls = 0
func walk(_ element: AXUIElement, depth: Int) {
    guard depth < 60, visited.insert(CFHash(element)).inserted else { return }
    let role = text(element, kAXRoleAttribute)
    var names = [kAXTitleAttribute, kAXDescriptionAttribute, kAXHelpAttribute, "AXPlaceholderValue"].map { text(element, $0) }
    if let label = attribute(element, kAXTitleUIElementAttribute), CFGetTypeID(label) == AXUIElementGetTypeID() {
        let titleElement = unsafeBitCast(label, to: AXUIElement.self)
        names.append(text(titleElement, kAXValueAttribute))
        names.append(text(titleElement, kAXTitleAttribute))
    }
    let missing = missingName(role: role, names: names, subrole: text(element, kAXSubroleAttribute))
    if controlRoles.contains(role) { controls += 1 }
    if missing { problems += 1 }
    if missing || args.contains("--all") {
        let name = names.first { !$0.isEmpty } ?? "<unnamed>"
        print("\(String(repeating: "  ", count: depth))\(missing ? "FAIL " : "")\(role): \(name)")
    }
    for child in attribute(element, kAXChildrenAttribute) as? [AXUIElement] ?? [] { walk(child, depth: depth + 1) }
}
for window in windows { walk(window, depth: 0) }
print("Checked \(controls) controls; \(problems) unnamed.")
exit(problems == 0 && controls > 0 ? 0 : 1)
