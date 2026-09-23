# SPDX-License-Identifier: GPL-3.0-or-later
"""Fast regression for the shared Apple catalog using compiled Foundation resources."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]


@unittest.skipUnless(sys.platform == "darwin", "Requires Apple Foundation and xcstringstool")
class SharedPluralTests(unittest.TestCase):
    def test_english_and_german_shared_plurals(self):
        with tempfile.TemporaryDirectory(prefix="momentum-shared-plurals-") as temporary:
            folder = Path(temporary)
            verifier = folder / "check-shared-plurals"
            commands = [
                ["xcrun", "swiftc", str(ROOT / "macos/scripts/check-shared-plurals.swift"), "-o", str(verifier)],
                ["xcrun", "xcstringstool", "compile",
                 str(ROOT / "macos/Packages/MomentumKit/Sources/MomentumKit/Resources/Localizable.xcstrings"),
                 "--output-directory", str(folder)],
                [str(verifier), str(folder)],
            ]
            for command in commands:
                result = subprocess.run(command, capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn("168 compiled shared plural checks; 0 failures", result.stdout)
            print(result.stdout.strip())


if __name__ == "__main__":
    unittest.main()
