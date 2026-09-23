# SPDX-License-Identifier: GPL-3.0-or-later
"""Fast Apple catalog regression: real xcstringstool products and Foundation lookup."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]


def run_checked(command):
    result = subprocess.run(command, capture_output=True, text=True)
    if result.returncode:
        raise RuntimeError(result.stdout + result.stderr)


@unittest.skipUnless(sys.platform == "darwin", "Requires Apple Foundation and xcstringstool")
class RepeatCatalogTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temporary = tempfile.TemporaryDirectory(prefix="momentum-repeat-catalog-")
        cls.addClassCleanup(cls.temporary.cleanup)
        cls.folder = Path(cls.temporary.name)
        cls.verifier = cls.folder / "check-repeat-localization"
        run_checked(["xcrun", "swiftc", str(ROOT / "macos/scripts/check-repeat-localization.swift"),
                     "-o", str(cls.verifier)])

    def check_catalog(self, platform):
        destination = self.folder / platform
        destination.mkdir()
        catalog = ROOT / platform / "Momentum/Resources/Localizable.xcstrings"
        run_checked(["xcrun", "xcstringstool", "compile", str(catalog),
                     "--output-directory", str(destination)])
        result = subprocess.run([str(self.verifier), str(destination)], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_ios_repeat_intervals(self):
        self.check_catalog("ios")

    def test_macos_repeat_intervals(self):
        self.check_catalog("macos")


if __name__ == "__main__":
    unittest.main()
