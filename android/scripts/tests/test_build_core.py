# SPDX-License-Identifier: GPL-3.0-or-later
import importlib.util
import io
import tarfile
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).parents[1] / "build_core.py"


class BuildCoreTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if not SCRIPT.exists():
            return
        spec = importlib.util.spec_from_file_location("build_core", SCRIPT)
        cls.core = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.core)

    def test_source_build_script_exists(self):
        self.assertTrue(SCRIPT.exists(), "the source-built Android pipeline is missing")

    def core_module(self):
        self.assertTrue(SCRIPT.exists(), "the source-built Android pipeline is missing")
        return self.core

    def test_source_digest_detects_changed_or_added_source(self):
        core = self.core_module()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text("original")
            original = core.source_digest(root)
            (root / "Cargo.toml").write_text("changed")
            self.assertNotEqual(original, core.source_digest(root))
            (root / "Cargo.toml").write_text("original")
            (root / ".git").mkdir()
            (root / ".git" / "HEAD").write_text("local git metadata")
            self.assertEqual(original, core.source_digest(root))
            (root / "extra.rs").write_text("unreviewed source")
            self.assertNotEqual(original, core.source_digest(root))

    def test_archive_rejects_path_escape_without_touching_existing_source(self):
        core = self.core_module()
        archive = io.BytesIO()
        with tarfile.open(fileobj=archive, mode="w:gz") as tar:
            member = tarfile.TarInfo("LibreSync/../outside")
            member.size = 1
            tar.addfile(member, io.BytesIO(b"x"))
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "source"
            with self.assertRaises(ValueError):
                core.extract_source(archive.getvalue(), output)
            self.assertFalse((Path(directory) / "outside").exists())

    def test_ndk_environment_pins_abi_and_16k_alignment(self):
        core = self.core_module()
        env = core.target_environment(Path("/sdk/ndk"), "aarch64-linux-android", 26)
        self.assertTrue(env["CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER"].endswith("aarch64-linux-android26-clang"))
        self.assertIn("max-page-size=16384", env["CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS"])
        self.assertIn("common-page-size=16384", env["CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS"])
        self.assertEqual(env["CC_aarch64_linux_android"], env["CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER"])

    def test_elf_verifier_rejects_small_load_alignment(self):
        core = self.core_module()
        good = "LOAD off 0x0 align 2**14\nLOAD off 0x4000 align 2**14"
        core.check_load_alignment(good)
        with self.assertRaises(ValueError):
            core.check_load_alignment("LOAD off 0x0 align 2**12")
        with self.assertRaises(ValueError):
            core.check_load_alignment("not an ELF file")

    def test_relro_verifier_rejects_partial_16k_page(self):
        core = self.core_module()
        self.assertTrue(hasattr(core, "check_relro_alignment"), "RELRO alignment verification is missing")
        core.check_relro_alignment("GNU_RELRO 0x1000 0x4000 0x4000 0x2000 0x4000 R 0x1")
        with self.assertRaises(ValueError):
            core.check_relro_alignment("GNU_RELRO 0x1000 0x4000 0x4000 0x2000 0x2000 R 0x1")


if __name__ == "__main__":
    unittest.main()
