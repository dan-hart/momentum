# SPDX-License-Identifier: GPL-3.0-or-later
import hashlib
import importlib.util
import tempfile
import unittest
import zipfile
from pathlib import Path

SCRIPT = Path(__file__).parents[1] / "build_graphics_path.py"


class GraphicsPathTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if SCRIPT.exists():
            spec = importlib.util.spec_from_file_location("build_graphics_path", SCRIPT)
            cls.graphics = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(cls.graphics)

    def module(self):
        self.assertTrue(SCRIPT.exists(), "the source-built graphics-path pipeline is missing")
        return self.graphics

    def test_unchanged_upstream_sources_are_present_and_pinned(self):
        graphics = self.module()
        graphics.verify_source()

    def test_source_pin_rejects_changed_extra_and_symlinked_inputs(self):
        graphics = self.module()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "original.cpp"
            source.write_bytes(b"original")
            pin = {"original.cpp": {"sha256": hashlib.sha256(b"original").hexdigest()}}
            graphics.verify_files(root, pin)
            source.write_bytes(b"changed")
            with self.assertRaises(ValueError):
                graphics.verify_files(root, pin)
            source.write_bytes(b"original")
            extra = root / "unreviewed.cpp"
            extra.write_bytes(b"extra")
            with self.assertRaises(ValueError):
                graphics.verify_files(root, pin)
            extra.unlink()
            source.unlink()
            source.symlink_to(root / "outside")
            with self.assertRaises(ValueError):
                graphics.verify_files(root, pin)

    def test_build_command_preserves_exports_and_enables_both_page_flags_and_relro(self):
        graphics = self.module()
        command = graphics.compile_command(Path("/ndk"), "x86_64", Path("/output.so"))
        self.assertTrue(command[0].endswith("x86_64-linux-android26-clang++"))
        self.assertIn("-Wl,-z,max-page-size=16384", command)
        self.assertIn("-Wl,-z,common-page-size=16384", command)
        self.assertIn("-Wl,-z,relro,-z,now", command)
        self.assertIn("-Wl,-soname,libandroidx.graphics.path.so", command)
        self.assertTrue(any("--version-script=" in item for item in command))
        self.assertIn("-nostdlib++", command)

    def test_repack_replaces_only_jni_and_preserves_classes_resources_notices(self):
        graphics = self.module()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            original, output = root / "upstream.aar", root / "rebuilt.aar"
            contents = {"classes.jar": b"unchanged public classes", "AndroidManifest.xml": b"manifest",
                        "META-INF/LICENSE.txt": b"license", "res/values/values.xml": b"resources",
                        "jni/arm64-v8a/libandroidx.graphics.path.so": b"old-arm",
                        "jni/x86_64/libandroidx.graphics.path.so": b"old-x86",
                        "jni/armeabi-v7a/libandroidx.graphics.path.so": b"unsupported"}
            with zipfile.ZipFile(original, "w") as archive:
                for name, data in contents.items():
                    archive.writestr(name, data)
            native = {"arm64-v8a": b"new-arm", "x86_64": b"new-x86"}
            graphics.repack_archive(original, output, native)
            with zipfile.ZipFile(output) as archive:
                self.assertNotIn("jni/armeabi-v7a/libandroidx.graphics.path.so", archive.namelist())
                for name, data in contents.items():
                    if not name.startswith("jni/"):
                        self.assertEqual(data, archive.read(name))
                for abi, data in native.items():
                    self.assertEqual(data, archive.read(f"jni/{abi}/libandroidx.graphics.path.so"))
            second = root / "second.aar"
            graphics.repack_archive(original, second, native)
            self.assertEqual(output.read_bytes(), second.read_bytes())

    def test_repack_rejects_unexpected_native_library(self):
        graphics = self.module()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            original = root / "upstream.aar"
            with zipfile.ZipFile(original, "w") as archive:
                archive.writestr("jni/x86_64/unreviewed.so", b"unknown")
            with self.assertRaises(ValueError):
                graphics.repack_archive(original, root / "out.aar", {"x86_64": b"new"})


if __name__ == "__main__":
    unittest.main()
