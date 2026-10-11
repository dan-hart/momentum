# SPDX-License-Identifier: GPL-3.0-or-later
"""Regression coverage over actual UniFFI output, never handwritten bindings."""
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


class CodegenTests(unittest.TestCase):
    def test_error_payload_does_not_redeclare_throwable_message(self):
        generator = ROOT / "android/rust-target/debug/uniffi-bindgen"
        library = ROOT / "android/rust-target/debug/libmomentum_ffi.dylib"
        if not library.exists():
            library = library.with_suffix(".so")
        if not generator.exists() or not library.exists():
            self.skipTest("Run build_core.py --bindings-only to prepare the host generator")
        with tempfile.TemporaryDirectory() as directory:
            subprocess.run([str(generator), "generate", "--library", str(library),
                            "--language", "kotlin", "--config", str(ROOT / "android/uniffi.toml"),
                            "--out-dir", directory, "--no-format"], cwd=ROOT, check=True)
            generated = (Path(directory) / "com/codedbydan/momentum/core/momentum.kt").read_text()
        error = generated.split("sealed class CoreException", 1)[1].split("public object FfiConverterTypeCoreError", 1)[0]
        self.assertNotIn("val `message`:", error, "Payload shadows Throwable.message and its generated override")
        self.assertEqual(error.count("val `detail`: kotlin.String"), 5)
        self.assertIn("override val message", error)
        self.assertIn("detail=${ `detail` }", error)


if __name__ == "__main__":
    unittest.main()
