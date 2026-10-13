# SPDX-License-Identifier: GPL-3.0-or-later
"""Resolve changed native feature copy through compiled Foundation resources."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]


@unittest.skipUnless(sys.platform == "darwin", "Requires Apple Foundation and xcstringstool")
class NativeFeatureCatalogTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temporary = tempfile.TemporaryDirectory(prefix="momentum-native-feature-catalog-")
        cls.addClassCleanup(cls.temporary.cleanup)
        cls.folder = Path(cls.temporary.name)
        cls.verifier = cls.folder / "check-native-feature-localization"
        subprocess.run(["xcrun", "swiftc", str(ROOT / "macos/scripts/check-native-feature-localization.swift"),
                        "-o", str(cls.verifier)], check=True, capture_output=True, text=True)

    def check_catalog(self, source, scope):
        destination = self.folder / scope
        destination.mkdir()
        subprocess.run(["xcrun", "xcstringstool", "compile", str(ROOT / source),
                        "--output-directory", str(destination)], check=True, capture_output=True, text=True)
        result = subprocess.run([str(self.verifier), str(destination), scope], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        print(result.stdout.strip())

    def test_macos_settings_and_feedback(self):
        self.check_catalog("macos/Momentum/Resources/Localizable.xcstrings", "app")

    def test_shared_settings_and_feedback(self):
        self.check_catalog("macos/Packages/MomentumKit/Sources/MomentumKit/Resources/Localizable.xcstrings", "shared")

    def test_ios_settings_share_and_feedback(self):
        catalog = json.loads((ROOT / "ios/Momentum/Resources/Localizable.xcstrings").read_text())
        self.assertNotEqual(catalog["strings"]["Momentum plans the task for today when it next opens."].get("extractionState"), "stale")
        self.check_catalog("ios/Momentum/Resources/Localizable.xcstrings", "ios")


if __name__ == "__main__":
    unittest.main()
