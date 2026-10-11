# SPDX-License-Identifier: GPL-3.0-or-later
import importlib.util
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import threading
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).parents[1] / "test_libresync_two_emulators.py"
spec = importlib.util.spec_from_file_location("libresync_two_fixture", SCRIPT)
fixture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixture)


class PrivateBridgeTest(unittest.TestCase):
    def test_loopback_bridge_transports_opaque_bytes_both_ways_and_removes_only_its_forward(self):
        class ScopedForward:
            def __init__(self):
                self.calls = []

            def call(self, *args, **kwargs):
                self.calls.append(args)
                return subprocess.CompletedProcess(args, 0)

        with tempfile.TemporaryDirectory(prefix="mo-p2p-test-") as directory:
            root = Path(directory)
            owner = ScopedForward()
            listener = socket.socket(socket.AF_UNIX)
            listener.bind(str(root / "alpha-adb.sock"))
            listener.listen(1)
            listener.settimeout(5)
            outgoing, incoming = os.urandom(131_073), os.urandom(98_305)
            received = bytearray()
            failures = []

            def receive():
                try:
                    connection, _ = listener.accept()
                    with connection:
                        connection.settimeout(5)
                        while len(received) < len(outgoing):
                            data = connection.recv(8192)
                            if not data:
                                raise AssertionError("Premature bridge EOF")
                            received.extend(data)
                        connection.sendall(incoming)
                except BaseException as failure:
                    failures.append(failure)

            worker = threading.Thread(target=receive)
            worker.start()
            bridge = fixture.Bridge(owner, root)
            try:
                self.assertEqual("127.0.0.1", bridge.listener.getsockname()[0])
                bridge.route(52345)
                with socket.create_connection(("127.0.0.1", bridge.port), timeout=5) as client:
                    client.sendall(outgoing)
                    returned = bytearray()
                    while len(returned) < len(incoming):
                        data = client.recv(8192)
                        self.assertTrue(data)
                        returned.extend(data)
                worker.join(timeout=5)
                self.assertFalse(worker.is_alive())
                self.assertFalse(failures)
                self.assertEqual(outgoing, received)
                self.assertEqual(incoming, returned)
            finally:
                bridge.close()
                listener.close()
                worker.join(timeout=5)
            self.assertEqual([
                ("forward", "localfilesystem:" + str(root / "alpha-adb.sock"), "tcp:52345"),
                ("forward", "--remove", "localfilesystem:" + str(root / "alpha-adb.sock")),
            ], owner.calls)

    def test_no_permission_restore_is_attempted_before_instrumentation_has_started(self):
        device = fixture.Device("unused-adb", "emulator-1000", "alpha", "00000000-0000-0000-0000-000000000000")
        device.restore_lan = True
        def forbidden(*args, **kwargs):
            self.fail("Validation failure must not mutate any emulator")
        device.call = forbidden
        self.assertTrue(device.cleanup())

    def test_native_cleanup_timeout_still_restores_prior_lan_permission_and_closes_private_output(self):
        device = fixture.Device("unused-adb", "emulator-1000", "alpha", "00000000-0000-0000-0000-000000000000")
        device.process = type("FinishedInstrumentation", (), {"poll": lambda self: 0})()
        device.restore_lan = True
        calls = []
        def capture(*args, **kwargs):
            calls.append(args)
            return subprocess.CompletedProcess(args, 0)
        device.call = capture
        with patch.object(fixture.subprocess, "run", side_effect=subprocess.TimeoutExpired("fixture", 45)):
            self.assertFalse(device.cleanup())
        self.assertTrue(device.output.closed)
        self.assertEqual([("shell", "pm", "revoke", fixture.PACKAGE, fixture.LAN)], calls)

    def test_private_status_reader_retries_partial_atomic_file_publication(self):
        device = fixture.Device("unused-adb", "emulator-1000", "alpha", "00000000-0000-0000-0000-000000000000")
        responses = iter([b'{"phase":', b'{"phase":"ready","sequence":0}'])
        device.call = lambda *args, **kwargs: subprocess.CompletedProcess(args, 0, next(responses))
        try:
            self.assertEqual({"phase": "ready", "sequence": 0}, device.read("status.json"))
        finally:
            device.cleanup()

    def test_private_status_reader_rejects_stable_corruption_oversize_and_unexpected_types(self):
        device = fixture.Device("unused-adb", "emulator-1000", "alpha", "00000000-0000-0000-0000-000000000000")
        try:
            for data in [b'corrupt', b'x' * 4097, b'[]', b'{"phase":true,"sequence":0}']:
                device.call = lambda *args, **kwargs: subprocess.CompletedProcess(args, 0, data)
                with self.subTest(length=len(data)), self.assertRaises(fixture.FixtureError):
                    device.read("status.json")
        finally:
            device.cleanup()

    def test_command_publishes_json_atomically_with_literal_shell_characters_only_on_stdin(self):
        device = fixture.Device("unused-adb", "emulator-1000", "alpha", "00000000-0000-0000-0000-000000000000")
        with tempfile.TemporaryDirectory(prefix="mo-command-test-") as directory:
            owned = Path(directory) / device.root
            owned.mkdir(parents=True)
            calls = []
            def execute(*args, data=None, **kwargs):
                calls.append(args)
                result = subprocess.run(["sh"], input=data, cwd=directory, capture_output=True, timeout=5)
                self.assertEqual(0, result.returncode)
                return result
            device.call = execute
            try:
                literal = "quoted ' $(printf unsafe) fixture"
                device.command("link", code="000000", address="10.0.2.2:12345", fixture=literal)
                self.assertEqual(literal, fixture.json.loads((owned / "command.json").read_text())["fixture"])
                self.assertFalse((owned / "command.tmp").exists())
                self.assertNotIn("000000", " ".join(calls[0]))
                self.assertNotIn(literal, " ".join(calls[0]))
            finally:
                device.cleanup()


if __name__ == "__main__":
    unittest.main()
