# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Dan Hart
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location(
    "measure_launch", Path(__file__).resolve().parents[1] / "measure-launch.py",
)
launch = importlib.util.module_from_spec(spec)
spec.loader.exec_module(launch)


class ReadinessTests(unittest.TestCase):
    def test_ios_requires_both_enabled_elements(self):
        capture = {"AXUniqueId": "floating-add-task", "AXLabel": "Add task"}
        row = {"AXLabel": "Open Momentum launch fixture 0001"}
        self.assertTrue(launch.ios_ready([{"children": [capture, row]}]))
        self.assertFalse(launch.ios_ready([capture, {"AXLabel": "Loading Momentum"}]))
        self.assertFalse(launch.ios_ready([capture, {**row, "AXEnabled": False}]))
        self.assertFalse(launch.ios_ready([{**capture, "AXHidden": True}, row]))

    def test_android_requires_the_app_and_a_usable_edit_field_on_a_populated_list(self):
        capture = '<node class="android.widget.EditText" enabled="true" package="com.codedbydan.momentum"/>'
        row = ('<node content-desc="Mark as done: Momentum launch fixture 0001" '
               'enabled="true" package="com.codedbydan.momentum"/>')
        self.assertTrue(launch.android_ready(f"<hierarchy>{capture}{row}</hierarchy>"))
        self.assertFalse(launch.android_ready(f"<hierarchy>{capture}</hierarchy>"))
        self.assertFalse(launch.android_ready(f"<hierarchy>{row}</hierarchy>"))
        self.assertFalse(launch.android_ready(f"<hierarchy>{capture.replace('true', 'false')}{row}</hierarchy>"))
        self.assertFalse(launch.android_ready(f"<hierarchy>{capture.replace('momentum', 'other')}{row}</hierarchy>"))


if __name__ == "__main__":
    unittest.main()
