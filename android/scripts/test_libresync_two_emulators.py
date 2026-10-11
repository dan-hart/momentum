#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Opt-in, private-cache LibreSync acceptance. Does not build/install or change app data."""
import argparse
import json
import os
from pathlib import Path
import re
import select
import shlex
import socket
import subprocess
import tempfile
import threading
import time
import uuid

PACKAGE = "com.codedbydan.momentum"
TEST = PACKAGE + ".platform.LibreSyncTwoEmulatorTest"
RUNNER = PACKAGE + ".test/androidx.test.runner.AndroidJUnitRunner"
LAN = "android.permission.ACCESS_LOCAL_NETWORK"


class FixtureError(Exception):
    """Messages contain only generic phase/runtime information, never child output."""


class Device:
    def __init__(self, adb, serial, role, run):
        self.adb, self.serial, self.role, self.run = adb, serial, role, run
        self.root = f"cache/libresync-two-{run}-{role}"
        self.sequence = 0
        self.process = None
        self.output = tempfile.TemporaryFile()
        self.sdk = 0
        self.restore_lan = False

    def call(self, *arguments, data=None, tolerate=False, timeout=30):
        result = subprocess.run([self.adb, "-s", self.serial, *arguments], input=data,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=timeout)
        if result.returncode and not tolerate:
            raise FixtureError(f"{self.role}: adb operation failed")
        return result

    def validate(self):
        hardware = self.call("shell", "getprop", "ro.hardware").stdout.decode().strip()
        if hardware not in {"ranchu", "goldfish"}:
            raise FixtureError(f"{self.role}: disposable emulator hardware required")
        self.sdk = int(self.call("shell", "getprop", "ro.build.version.sdk").stdout.decode().strip())
        if self.sdk < 26:
            raise FixtureError(f"{self.role}: API26 or later required")
        self.call("shell", "run-as", PACKAGE, "pwd")  # Requires an already installed debug test app.
        if self.sdk >= 37:
            package = self.call("shell", "dumpsys", "package", PACKAGE).stdout.decode()
            match = re.search(re.escape(LAN) + r": granted=(true|false)", package)
            if not match:
                raise FixtureError(f"{self.role}: cannot establish prior LAN permission")
            self.restore_lan = match.group(1) == "false"

    def instrumentation(self, cleanup=False):
        args = ["shell", "am", "instrument", "-w", "-r", "-e", "class", TEST,
                "-e", "momentum.disposableLibreSync", "true",
                "-e", "momentum.fixture.run", self.run, "-e", "momentum.fixture.role", self.role]
        if cleanup:
            args += ["-e", "momentum.fixture.cleanup", "true"]
        return [self.adb, "-s", self.serial, *args, RUNNER]

    def start(self):
        self.process = subprocess.Popen(self.instrumentation(), stdout=self.output, stderr=self.output)

    def read(self, name, deadline=None):
        if name not in {"status.json", "pairing.json"}:
            raise FixtureError("Invalid private fixture file")
        path = f"{self.root}/{name}"
        deadline = deadline if deadline is not None else time.monotonic() + 50
        # exec-out merges remote stderr and does not propagate cat's exit status.
        # Missing initial files must produce no bytes, never a cat error as JSON.
        command = f"if [ -f {path} ]; then cat {path}; fi"
        for attempt in range(8):
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise FixtureError(f"{self.role}: fixture publication deadline exceeded")
            result = self.call("exec-out", "run-as", PACKAGE, "sh", "-c", command,
                               tolerate=True, timeout=min(30, remaining))
            if result.returncode or not result.stdout:
                return None
            if len(result.stdout) > 4096:
                raise FixtureError(f"{self.role}: invalid fixture status bounds")
            try:
                value = json.loads(result.stdout)
            except (ValueError, UnicodeError):
                # API26 AtomicFile writes the initial base in place. Retry only
                # its short publication window; persistent corruption still fails.
                if attempt == 7 or time.monotonic() >= deadline:
                    raise FixtureError(f"{self.role}: invalid fixture status") from None
                time.sleep(min(0.05, max(0, deadline - time.monotonic())))
                continue
            if not isinstance(value, dict):
                raise FixtureError(f"{self.role}: invalid fixture status types")
            if name == "status.json" and (not isinstance(value.get("phase"), str) or
                    type(value.get("sequence")) is not int):
                raise FixtureError(f"{self.role}: invalid fixture status types")
            return value

    def wait(self, phase, timeout=50):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            value = self.read("status.json", deadline=deadline)
            if value and value.get("phase") == "failed":
                raise FixtureError(f"{self.role}: native fixture phase failed")
            if value and value.get("phase") == phase and value.get("sequence") == self.sequence:
                return value
            if self.process.poll() is not None:
                raise FixtureError(f"{self.role}: native fixture exited before {phase}")
            time.sleep(0.15)
        raise FixtureError(f"{self.role}: {phase} deadline exceeded")

    def command(self, operation, phase=None, **parameters):
        self.sequence += 1
        data = json.dumps(dict(sequence=self.sequence, operation=operation, **parameters)).encode()
        if len(data) > 4096:
            raise FixtureError("Fixture command exceeds bounds")
        # Paths consist only of our canonical UUID and fixed role/name. Sensitive
        # command values travel on stdin, never argv or printed subprocess output.
        # mksh heredocs can use /data/local temp files, which run-as cannot access.
        # Its built-in printf requires no temp file; shell quoting also handles
        # any literal apostrophe without turning a parameter into shell code.
        shell = (f"printf '%s' {shlex.quote(data.decode())} > {self.root}/command.tmp && "
                 f"mv {self.root}/command.tmp {self.root}/command.json\n").encode()
        # Shell v2 sends a distinct stdin-close frame and waits for remote exit.
        # Raw exec-in can close its socket before the final write/rename completes.
        self.call("shell", "-T", "run-as", PACKAGE, "sh", data=shell)
        if phase:
            self.wait(phase)

    def finish(self):
        self.command("stop")
        try:
            self.process.wait(timeout=30)
        except subprocess.TimeoutExpired:
            raise FixtureError(f"{self.role}: native drain deadline exceeded") from None
        self.output.seek(0)
        result = self.output.read()
        if self.process.returncode or b"OK (1 test)" not in result or b"FAILURES!!!" in result:
            raise FixtureError(f"{self.role}: native assertions or cleanup failed")

    def cleanup(self):
        good = True
        try:
            if self.process and self.process.poll() is None:
                try:
                    self.command("stop")
                    self.process.wait(timeout=60)
                except (FixtureError, subprocess.SubprocessError):
                    # Only the explicitly owned emulator's test application is stopped.
                    self.call("shell", "am", "force-stop", PACKAGE, tolerate=True)
                    try:
                        self.process.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        self.process.kill()
            if self.process:
                result = subprocess.run(self.instrumentation(cleanup=True), stdout=subprocess.PIPE,
                                        stderr=subprocess.PIPE, timeout=45)
                good = result.returncode == 0 and b"OK (1 test)" in result.stdout
        except (FixtureError, OSError, subprocess.SubprocessError):
            good = False
        finally:
            try:
                if self.process and self.restore_lan:
                    result = self.call("shell", "pm", "revoke", PACKAGE, LAN, tolerate=True)
                    good = good and result.returncode == 0
            except (FixtureError, OSError, subprocess.SubprocessError):
                good = False
            finally:
                self.output.close()
        return good


class Bridge:
    """Bounded loopback TCP -> private Unix adb forward; encrypted bytes stay opaque."""
    def __init__(self, device, root):
        self.device = device
        self.unix = str(root / "alpha-adb.sock")
        if len(self.unix.encode()) > 99:
            raise FixtureError("Private bridge socket path exceeds platform bounds")
        self.spec = "localfilesystem:" + self.unix
        self.routed = False
        self.stop = threading.Event()
        self.active = set()
        self.lock = threading.Lock()
        self.slots = threading.BoundedSemaphore(8)
        self.listener = socket.socket()
        self.listener.bind(("127.0.0.1", 0))
        self.listener.listen(8)
        self.listener.settimeout(0.25)
        self.port = self.listener.getsockname()[1]
        self.thread = threading.Thread(target=self.accept, daemon=True)
        self.thread.start()

    def route(self, port):
        if not isinstance(port, int) or not 1 <= port <= 65535:
            raise FixtureError("Invalid native listener port")
        self.device.call("forward", self.spec, f"tcp:{port}")
        self.routed = True

    def accept(self):
        while not self.stop.is_set():
            try:
                client, _ = self.listener.accept()
            except socket.timeout:
                continue
            except OSError:
                return
            if not self.slots.acquire(blocking=False):
                client.close()
                continue
            threading.Thread(target=self.relay, args=(client,), daemon=True).start()

    def relay(self, client):
        remote = socket.socket(socket.AF_UNIX)
        sockets = (client, remote)
        with self.lock:
            self.active.update(sockets)
        try:
            remote.settimeout(5)
            remote.connect(self.unix)
            for endpoint in sockets:
                endpoint.settimeout(5)
            deadline = time.monotonic() + 45
            while not self.stop.is_set() and time.monotonic() < deadline:
                ready, _, _ = select.select(sockets, [], [], 0.25)
                for endpoint in ready:
                    data = endpoint.recv(65536)
                    if not data:
                        return
                    (remote if endpoint is client else client).sendall(data)
        except (OSError, ValueError):
            pass  # The native fixture checks exchange success; no packet/body logging.
        finally:
            for endpoint in sockets:
                endpoint.close()
            with self.lock:
                self.active.difference_update(sockets)
            self.slots.release()

    def close(self):
        self.stop.set()
        self.listener.close()
        with self.lock:
            for endpoint in self.active:
                endpoint.close()
        self.thread.join(timeout=2)
        if self.routed:
            result = self.device.call("forward", "--remove", self.spec, tolerate=True)
            if result.returncode:
                raise FixtureError("Scoped bridge forward removal failed")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--adb", default="adb")
    parser.add_argument("--alpha", required=True)
    parser.add_argument("--beta", required=True)
    parser.add_argument("--disposable-emulators", action="store_true")
    args = parser.parse_args()
    if not args.disposable_emulators or args.alpha == args.beta or any(
            not re.fullmatch(r"emulator-[0-9]+", serial) for serial in (args.alpha, args.beta)):
        parser.error("Two distinct explicitly disposable emulator serials are required")
    run = str(uuid.uuid4())
    devices = [Device(args.adb, args.alpha, "alpha", run), Device(args.adb, args.beta, "beta", run)]
    bridge = None
    passed = False
    failure = None
    try:
        # Validate BOTH endpoints before any instrumentation/permission mutation.
        for device in devices:
            device.validate()
        with tempfile.TemporaryDirectory(prefix="momentum-libresync-") as temporary:
            os.chmod(temporary, 0o700)
            alpha, beta = devices
            for device in devices:
                device.start()
            for device in devices:
                device.wait("ready")
            private_pairing = alpha.read("pairing.json")
            code = private_pairing.get("code")
            if not isinstance(code, str) or not re.fullmatch(r"[0-9]{6}", code):
                raise FixtureError("Invalid private pairing state")
            bridge = Bridge(alpha, Path(temporary))
            bridge.route(private_pairing.get("port"))
            beta.command("link", "linked", address=f"10.0.2.2:{bridge.port}", code=code)
            del code, private_pairing
            alpha.command("linked", "linked")
            beta.command("sync", "synced")
            for device in devices:
                device.command("initial", "initial")
            for device in devices:
                device.command("offline", "offline")
            for device in devices:
                device.command("edit", "edited")
            alpha.command("online", "online")
            bridge.route(alpha.read("pairing.json").get("port"))
            beta.command("online", "online")
            beta.command("sync", "synced")
            for device in devices:
                device.command("verify", "converged")
            for device in devices:
                device.command("reopen", "reopened")
            for device in devices:
                device.finish()
            bridge.close()
            bridge = None
            passed = True
    except (FixtureError, OSError, ValueError, subprocess.SubprocessError) as error:
        failure = str(error) if isinstance(error, FixtureError) else type(error).__name__
    finally:
        if bridge:
            try:
                bridge.close()
            except (FixtureError, subprocess.SubprocessError):
                passed = False
        for device in devices:
            try:
                if not device.cleanup():
                    passed = False
                    failure = "Scoped fixture/permission cleanup failed"
            except (FixtureError, OSError, subprocess.SubprocessError):
                passed = False
                failure = "Scoped fixture/permission cleanup failed"
    if not passed:
        print("LibreSync two-emulator acceptance FAILED: " + (failure or "cleanup failed"))
        return 1
    print(f"LibreSync two-emulator acceptance PASS: API{devices[0].sdk}/API{devices[1].sdk}; "
          "2 isolated stores; 5 converged tasks; Keystore reopen; scoped cleanup complete. "
          "NAT TCP bridge; discovery and symmetric routing unverified.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
