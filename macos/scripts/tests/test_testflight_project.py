# SPDX-License-Identifier: GPL-3.0-or-later
import pathlib
import plistlib
import shutil
import subprocess
import tempfile
import unittest

MACOS = pathlib.Path(__file__).resolve().parents[2]


class TestFlightProjectTests(unittest.TestCase):
    def test_generated_beta_and_stable_projects(self):
        with tempfile.TemporaryDirectory() as directory:
            macos = pathlib.Path(directory) / "macos"
            shutil.copytree(MACOS, macos, ignore=shutil.ignore_patterns(
                "*.xcodeproj", ".build", "DerivedData", "*.xcframework",
                "MomentumTestFlight-Info.plist", "__pycache__"))
            for spec in ("project.yml", "project-testflight.yml"):
                subprocess.run(["xcodegen", "generate", "--spec", spec], cwd=macos, check=True)
            stable = (macos / "Momentum.xcodeproj/project.pbxproj").read_text()
            beta = (macos / "MomentumTestFlight.xcodeproj/project.pbxproj").read_text()
            self.assertIn("Sparkle in Frameworks", stable)
            self.assertIn("Bundle mo command line companion", stable)
            self.assertNotIn("Sparkle in Frameworks", beta)
            self.assertNotIn("Bundle mo command line companion", beta)
            scheme = (macos / "MomentumTestFlight.xcodeproj/xcshareddata/xcschemes/Momentum.xcscheme").read_text()
            self.assertNotIn("MomentumRelaunch", scheme)
            self.assertIn("MOMENTUM_TESTFLIGHT", beta)
            self.assertIn("MomentumTestFlight.entitlements", beta)
            info = plistlib.loads((macos / "MomentumTestFlight-Info.plist").read_bytes())
            self.assertEqual(info["CFBundleIdentifier"], "com.codedbydan.Momentum")
            self.assertFalse(info.get("SUFeedURL"))
            entitlements = plistlib.loads((macos / "MomentumTestFlight.entitlements").read_bytes())
            for key in ("app-sandbox", "network.client", "network.server", "files.user-selected.read-write"):
                self.assertIs(entitlements["com.apple.security." + key], True)


if __name__ == "__main__":
    unittest.main()
