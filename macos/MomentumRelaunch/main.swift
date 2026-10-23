// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
import AppKit
import Darwin

// Launch only the containing app, after the original process exits. Registering the
// kernel process event before the READY handshake prevents PID reuse races and keeps
// old/new task stores, CLI sockets and background services from overlapping.
guard CommandLine.arguments.count == 2, let originalPID = Int32(CommandLine.arguments[1]), originalPID > 1 else { exit(64) }
let application = URL(fileURLWithPath: CommandLine.arguments[0]).resolvingSymlinksInPath()
    .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
guard application.pathExtension == "app", Bundle(url: application)?.bundleIdentifier == "com.codedbydan.Momentum" else { exit(65) }
let queue = kqueue()
guard queue >= 0 else { exit(71) }
var event = kevent(ident: UInt(originalPID), filter: Int16(EVFILT_PROC), flags: UInt16(EV_ADD | EV_ENABLE | EV_ONESHOT),
                   fflags: UInt32(NOTE_EXIT), data: 0, udata: nil)
guard kevent(queue, &event, 1, nil, 0, nil) == 0 else { close(queue); exit(71) }
FileHandle.standardOutput.write(Data("READY\n".utf8))
let deadline = ProcessInfo.processInfo.systemUptime + 60
var exited = false
while ProcessInfo.processInfo.systemUptime < deadline {
    var timeout = timespec(tv_sec: 1, tv_nsec: 0)
    let count = kevent(queue, nil, 0, &event, 1, &timeout)
    if count == 1 && event.fflags & UInt32(NOTE_EXIT) != 0 { exited = true; break }
    if count < 0 && errno != EINTR { break }
}
close(queue)
guard exited else { exit(75) }
let configuration = NSWorkspace.OpenConfiguration()
NSWorkspace.shared.openApplication(at: application, configuration: configuration) { _, error in exit(error == nil ? 0 : 1) }
RunLoop.main.run()
