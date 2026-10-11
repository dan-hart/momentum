# SPDX-License-Identifier: GPL-3.0-or-later
import hashlib
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).parents[1] / "prepare_fdroid_source.py"
spec = importlib.util.spec_from_file_location("prepare_fdroid_source", SCRIPT)
prepare = importlib.util.module_from_spec(spec)
spec.loader.exec_module(prepare)


class PrepareFdroidSourceTests(unittest.TestCase):
    def fixture(self, root, name="winapi-x86_64-pc-windows-gnu", version="0.4.0"):
        package = root / f"{name}-{version}"
        package.mkdir(parents=True)
        files = {"Cargo.toml": f'[package]\nname = "{name}"\nversion = "{version}"\n'.encode(),
                 "build.rs": b"reviewed build script", "src/lib.rs": b"// preserve source",
                 "lib/libwinapi_kernel32.a": b"windows import archive",
                 "tests/fixture.bin": b"preserve unrelated test fixture"}
        for relative, content in files.items():
            path = package / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(content)
        package_hash = "1" * 64
        checksums = {"package": package_hash, "files": {
            key: hashlib.sha256(value).hexdigest() for key, value in files.items()}}
        (package / ".cargo-checksum.json").write_text(json.dumps(checksums))
        lock = root.parent / "Cargo.lock"
        lock.write_text(f'[[package]]\nname = "{name}"\nversion = "{version}"\nchecksum = "{package_hash}"\n')
        policy = {(name, version): {"build_sha256": checksums["files"]["build.rs"],
                                    "library_pattern": r"lib/libwinapi_[A-Za-z0-9_.-]+\.a"}}
        return package, lock, policy

    def test_only_reviewed_windows_archives_and_checksum_entries_removed(self):
        with tempfile.TemporaryDirectory() as directory:
            vendor = Path(directory) / "vendor"
            package, lock, policy = self.fixture(vendor)
            expected_checksums = json.loads((package / ".cargo-checksum.json").read_text())
            del expected_checksums["files"]["lib/libwinapi_kernel32.a"]
            lock_bytes = lock.read_bytes()
            report = prepare.prepare(vendor, lock, policy)
            self.assertEqual([item["path"] for item in report["removed"]],
                             ["winapi-x86_64-pc-windows-gnu-0.4.0/lib/libwinapi_kernel32.a"])
            self.assertFalse((package / "lib/libwinapi_kernel32.a").exists())
            self.assertTrue((package / "tests/fixture.bin").exists())
            self.assertEqual((package / "src/lib.rs").read_bytes(), b"// preserve source")
            self.assertEqual(json.loads((package / ".cargo-checksum.json").read_text()), expected_checksums)
            self.assertEqual(lock.read_bytes(), lock_bytes)
            self.assertEqual(prepare.prepare(vendor, lock, policy)["removed"], [])

    def test_changed_bytes_rejected_before_any_deletion(self):
        with tempfile.TemporaryDirectory() as directory:
            vendor = Path(directory) / "vendor"
            package, lock, policy = self.fixture(vendor)
            original = package / "lib/libwinapi_kernel32.a"
            other = package / "lib/libwinapi_user32.a"
            other.write_bytes(b"unreviewed archive")
            with self.assertRaises(ValueError):
                prepare.prepare(vendor, lock, policy)
            self.assertTrue(original.exists())
            self.assertTrue(other.exists())

    def test_changed_build_script_or_package_identity_rejected(self):
        for relative, content in [("build.rs", b"changed logic"),
                                  ("Cargo.toml", b'[package]\nname="different"\nversion="0.4.0"')]:
            with self.subTest(relative=relative), tempfile.TemporaryDirectory() as directory:
                vendor = Path(directory) / "vendor"
                package, lock, policy = self.fixture(vendor)
                (package / relative).write_bytes(content)
                with self.assertRaises(ValueError):
                    prepare.prepare(vendor, lock, policy)
                self.assertTrue((package / "lib/libwinapi_kernel32.a").exists())

    def test_symlink_cannot_delete_outside_vendor(self):
        with tempfile.TemporaryDirectory() as directory:
            vendor = Path(directory) / "vendor"
            package, lock, policy = self.fixture(vendor)
            external = Path(directory) / "external.a"
            external.write_bytes(b"private file")
            archive = package / "lib/libwinapi_kernel32.a"
            archive.unlink()
            archive.symlink_to(external)
            with self.assertRaises(ValueError):
                prepare.prepare(vendor, lock, policy)
            self.assertEqual(external.read_bytes(), b"private file")

    def test_unreviewed_version_and_nonwindows_libraries_stay(self):
        with tempfile.TemporaryDirectory() as directory:
            vendor = Path(directory) / "vendor"
            package, lock, _ = self.fixture(vendor, version="0.5.0")
            nonwindows = vendor / "unreviewed_crypto_lib-1.0.0" / "libneeded.a"
            nonwindows.parent.mkdir()
            nonwindows.write_bytes(b"required input")
            self.assertEqual(prepare.prepare(vendor, lock)["removed"], [])
            self.assertTrue((package / "lib/libwinapi_kernel32.a").exists())
            self.assertTrue(nonwindows.exists())

    def test_exact_unused_fixture_removed_but_runtime_input_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            vendor = Path(directory) / "vendor"
            package, lock, _ = self.fixture(vendor)
            sums = json.loads((package / ".cargo-checksum.json").read_text())
            policy = [{"name": "winapi-x86_64-pc-windows-gnu", "version": "0.4.0",
                       "crate_checksum": sums["package"], "reason": "integration test only",
                       "evidence": {"src/lib.rs": sums["files"]["src/lib.rs"]},
                       "files": {"tests/fixture.bin": sums["files"]["tests/fixture.bin"]}}]
            expected_metadata = json.loads((package / ".cargo-checksum.json").read_text())
            del expected_metadata["files"]["tests/fixture.bin"]
            report = prepare.prepare(vendor, lock, policies={}, fixture_policies=policy)
            self.assertEqual(len(report["removed"]), 1)
            self.assertFalse((package / "tests/fixture.bin").exists())
            self.assertTrue((package / "lib/libwinapi_kernel32.a").exists())
            self.assertEqual(json.loads((package / ".cargo-checksum.json").read_text()), expected_metadata)
            self.assertEqual(prepare.prepare(vendor, lock, policies={}, fixture_policies=policy)["removed"], [])

    def test_changed_target_guard_prevents_fixture_deletion(self):
        with tempfile.TemporaryDirectory() as directory:
            vendor = Path(directory) / "vendor"
            package, lock, _ = self.fixture(vendor)
            sums = json.loads((package / ".cargo-checksum.json").read_text())
            policy = [{"name": "winapi-x86_64-pc-windows-gnu", "version": "0.4.0",
                       "crate_checksum": sums["package"], "reason": "integration test only",
                       "evidence": {"src/lib.rs": sums["files"]["src/lib.rs"]},
                       "files": {"tests/fixture.bin": sums["files"]["tests/fixture.bin"]}}]
            (package / "src/lib.rs").write_text("changed runtime use")
            with self.assertRaises(ValueError):
                prepare.prepare(vendor, lock, policies={}, fixture_policies=policy)
            self.assertTrue((package / "tests/fixture.bin").exists())

    def test_altered_code_or_license_anywhere_rejected_before_deletion(self):
        for relative in ["src/lib.rs", "LICENSE"]:
            with self.subTest(relative=relative), tempfile.TemporaryDirectory() as directory:
                vendor = Path(directory) / "vendor"
                package, lock, policy = self.fixture(vendor)
                if relative == "LICENSE":
                    original = b"original license"
                    checksums = json.loads((package / ".cargo-checksum.json").read_text())
                    checksums["files"][relative] = hashlib.sha256(original).hexdigest()
                    (package / ".cargo-checksum.json").write_text(json.dumps(checksums))
                (package / relative).write_bytes(b"altered source or license")
                with self.assertRaises(ValueError):
                    prepare.prepare(vendor, lock, policy)
                self.assertTrue((package / "lib/libwinapi_kernel32.a").exists())


if __name__ == "__main__":
    unittest.main()
