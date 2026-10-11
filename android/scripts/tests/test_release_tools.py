# SPDX-License-Identifier: GPL-3.0-or-later
import base64
import importlib.util
import shutil
import subprocess
import tarfile
import tempfile
import unittest
import zipfile
from pathlib import Path

SCRIPTS = Path(__file__).parents[1]


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class ReleaseToolsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.package = load("package_release", SCRIPTS / "package_release.py")
        cls.signing = load("configure_signing", SCRIPTS / "configure_signing.py")
        cls.release = load("release", SCRIPTS.parents[1] / "build-aux/release.py")

    def test_android_codes_increase_at_patch_minor_and_major_rollovers(self):
        code = self.package.version_code
        versions = ["0.4.9", "0.4.10", "0.4.999", "0.5.0", "0.999.999", "1.0.0"]
        values = [code(version) for version in versions]
        self.assertEqual(values, sorted(set(values)))
        self.assertEqual(4009, values[0])
        self.assertLessEqual(code("2099.999.999"), 2_100_000_000)

    def test_android_codes_reject_invalid_or_overflowing_versions(self):
        for value in ["0.0.0", "2100.0.0", "1.1000.0", "1.0.1000", "1.2", "1.2.3-beta", "-1.2.3"]:
            with self.subTest(version=value), self.assertRaises(ValueError):
                self.package.version_code(value)

    def badging(self):
        return ("package: name='com.codedbydan.momentum' versionCode='4009' versionName='0.4.9'\n"
                "minSdkVersion:'26'\ntargetSdkVersion:'37'\n")

    def test_metadata_refuses_wrong_version_id_sdk_and_debug_artifacts(self):
        good = self.badging()
        self.package.verify_badging(good, "0.4.9")
        self.package.verify_badging(good.replace("minSdkVersion", "sdkVersion"), "0.4.9")
        for bad in [good.replace("4009", "4005"), good.replace("0.4.9", "0.4.5"),
                    good.replace("com.codedbydan.momentum", "other.app"),
                    good.replace("minSdkVersion:'26'", "minSdkVersion:'28'"),
                    good.replace("targetSdkVersion:'37'", "targetSdkVersion:'36'"),
                    good + "application-debuggable\n"]:
            with self.subTest(badging=bad), self.assertRaises(ValueError):
                self.package.verify_badging(bad, "0.4.9")

    def test_bundle_metadata_refuses_version_or_debug_mismatch(self):
        good = ('<manifest xmlns:android="http://schemas.android.com/apk/res/android" '
                'package="com.codedbydan.momentum" android:versionCode="4009" android:versionName="0.4.9">'
                '<uses-sdk android:minSdkVersion="26" android:targetSdkVersion="37"/>'
                '<application android:debuggable="false"/></manifest>')
        self.package.verify_bundle_manifest(good, "0.4.9")
        for bad in [good.replace("4009", "4005"), good.replace('debuggable="false"', 'debuggable="true"')]:
            with self.assertRaises(ValueError):
                self.package.verify_bundle_manifest(bad, "0.4.9")

    def test_no_signing_configuration_creates_no_key_or_environment_entries(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            env_file = folder / "env"
            env_file.write_text("EXISTING=value\n")
            self.assertFalse(self.signing.configure({}, folder, env_file))
            self.assertEqual("EXISTING=value\n", env_file.read_text())
            self.assertEqual([env_file], list(folder.iterdir()))

    def test_partial_or_invalid_signing_configuration_fails_before_writing(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            for values in [{"ANDROID_KEYSTORE_BASE64": "a2V5"},
                           dict(self.signing_values(), ANDROID_KEYSTORE_BASE64="not base64!")]:
                with self.assertRaises(ValueError):
                    self.signing.configure(values, folder, folder / "env")
                self.assertEqual([], list(folder.iterdir()))

    def signing_values(self):
        return {"ANDROID_KEYSTORE_BASE64": base64.b64encode(b"fixture key bytes").decode(),
                "ANDROID_KEYSTORE_PASSWORD": "fixture\npassword", "ANDROID_KEY_ALIAS": "fixture alias",
                "ANDROID_KEY_PASSWORD": "second fixture password"}

    def test_signing_key_is_private_and_multiline_values_use_safe_environment_blocks(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            self.assertTrue(self.signing.configure(self.signing_values(), folder, folder / "env"))
            keys = list(folder.glob("momentum-android-key-*.jks"))
            self.assertEqual(1, len(keys))
            self.assertEqual(b"fixture key bytes", keys[0].read_bytes())
            self.assertEqual(0o600, keys[0].stat().st_mode & 0o777)
            text = (folder / "env").read_text()
            self.assertIn("MOMENTUM_KEYSTORE<<", text)
            self.assertIn("fixture\npassword\n", text)
            self.assertNotIn("ANDROID_KEYSTORE_BASE64", text)

    def test_checksums_include_mobile_and_metadata_assets_and_exclude_themselves(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            names = ["app.apk", "app.aab", "source.tar.gz", "notices.txt", "provenance.json", "desktop.zip"]
            for name in names:
                (folder / name).write_bytes(name.encode())
            (folder / "SHA256SUMS.txt").write_text("old checksum\n")
            first = self.release.write_checksums(folder)
            second = self.release.write_checksums(folder)
            self.assertEqual(first, second)
            self.assertEqual(sorted(names), [line.split("  ", 1)[1] for line in first.splitlines()])
            for line in first.splitlines():
                digest, name = line.split("  ", 1)
                self.assertEqual(digest, self.release.sha256_of(folder / name))

    def test_apk_signer_must_match_the_configured_certificate(self):
        expected = "a" * 64
        good = f"Signer #1 certificate SHA-256 digest: {expected}\n"
        self.package.verify_apk_certificate(good, expected)
        for bad in ["", good.replace(expected, "b" * 64),
                    good + f"Signer #2 certificate SHA-256 digest: {expected}\n"]:
            with self.assertRaises(ValueError):
                self.package.verify_apk_certificate(bad, expected)

    @unittest.skipUnless(all(shutil.which(command) for command in ["java", "keytool", "jarsigner"]), "JDK 17 required")
    def test_bundle_rejects_wrong_signer_unsigned_append_and_modified_signed_payload(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            for alias in ["configured", "wrong"]:
                subprocess.run(["keytool", "-genkeypair", "-keystore", str(folder / f"{alias}.jks"),
                                "-storepass", "fixture", "-alias", alias, "-dname", f"CN={alias}",
                                "-keyalg", "RSA", "-keysize", "2048", "-validity", "1", "-noprompt"],
                               check=True, capture_output=True)
            certificate = folder / "expected.der"
            subprocess.run(["keytool", "-exportcert", "-keystore", str(folder / "configured.jks"),
                            "-storepass", "fixture", "-alias", "configured", "-file", str(certificate)],
                           check=True, capture_output=True)
            checker = ["java", str(SCRIPTS / "VerifyBundleSignature.java")]
            for alias in ["configured", "wrong"]:
                bundle = folder / f"{alias}.aab"
                with zipfile.ZipFile(bundle, "w") as archive:
                    archive.writestr("base/manifest/AndroidManifest.xml", b"fixture payload")
                subprocess.run(["jarsigner", "-keystore", str(folder / f"{alias}.jks"),
                                "-storepass", "fixture", str(bundle), alias], check=True, capture_output=True)
                result = subprocess.run([*checker, str(bundle), str(certificate)], capture_output=True)
                self.assertEqual(0 if alias == "configured" else 1, result.returncode)
            good = folder / "configured.aab"
            unsigned = folder / "appended.aab"; shutil.copyfile(good, unsigned)
            with zipfile.ZipFile(unsigned, "a") as archive:
                archive.writestr("base/assets/unsigned.txt", b"must be refused")
            self.assertNotEqual(0, subprocess.run([*checker, str(unsigned), str(certificate)], capture_output=True).returncode)
            modified = folder / "modified.aab"
            with zipfile.ZipFile(good) as original, zipfile.ZipFile(modified, "w") as archive:
                for name in original.namelist():
                    archive.writestr(name, b"tampered" if name.endswith("AndroidManifest.xml") else original.read(name))
            self.assertNotEqual(0, subprocess.run([*checker, str(modified), str(certificate)], capture_output=True).returncode)

    def test_committed_snapshot_excludes_stale_same_version_outputs_and_untracked_code(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory); root = folder / "repository"; root.mkdir()
            subprocess.run(["git", "init", "-q", str(root)], check=True)
            (root / "Cargo.toml").write_text('[workspace.package]\nversion = "0.4.9"\n')
            source = root / "android/app/src/main/Task.kt"; source.parent.mkdir(parents=True)
            source.write_text("committed first version")
            def commit():
                subprocess.run(["git", "add", "."], cwd=root, check=True)
                subprocess.run(["git", "-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid",
                                "-c", "commit.gpgsign=false", "-c", "core.hooksPath=/dev/null",
                                "commit", "-qm", "fixture"], cwd=root, check=True)
            commit()
            source.write_text("committed changed implementation with same release version")
            commit()
            stale = root / "android/app/build/outputs/apk/release/app-release.apk"
            stale.parent.mkdir(parents=True); stale.write_bytes(b"stale binary from first commit")
            source.with_name("Untracked.kt").write_text("untracked build input")
            stage = folder / "stage"
            revision = self.package.committed_snapshot(root, stage)
            self.assertEqual(subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip(), revision)
            self.assertEqual(source.read_bytes(), (stage / source.relative_to(root)).read_bytes())
            self.assertFalse((stage / stale.relative_to(root)).exists())
            self.assertFalse((stage / source.with_name("Untracked.kt").relative_to(root)).exists())
            source.write_text("dirty changes")
            with self.assertRaises(ValueError):
                self.package.committed_snapshot(root, folder / "refused")

    def test_source_archive_contains_committed_app_and_exact_dependency_sources_only(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory); root = folder / "repository"; root.mkdir()
            subprocess.run(["git", "init", "-q", str(root)], check=True)
            (root / "README.md").write_text("source fixture")
            subprocess.run(["git", "add", "README.md"], cwd=root, check=True)
            subprocess.run(["git", "-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid",
                            "-c", "commit.gpgsign=false", "-c", "core.hooksPath=/dev/null",
                            "commit", "-qm", "fixture"], cwd=root, check=True)
            (root / "private-untracked.txt").write_text("must never ship")
            dependency = folder / "dependency"; dependency.mkdir()
            (dependency / "LICENSE").write_text("AGPL fixture")
            (dependency / ".git").mkdir(); (dependency / ".git/config").write_text("private git config")
            vendor = folder / "vendor"; vendor.mkdir(); (vendor / "crate.rs").write_text("locked Rust source")
            archive = folder / "source.tar.gz"
            self.package.source_archive(root, dependency, vendor, archive, "momentum-v0.4.9")
            with tarfile.open(archive) as source:
                names = source.getnames()
                self.assertIn("momentum-v0.4.9/README.md", names)
                self.assertIn("momentum-v0.4.9/libresync-src/LICENSE", names)
                self.assertIn("momentum-v0.4.9/vendor/crate.rs", names)
                self.assertFalse(any("private" in name or "/.git/" in name for name in names))
            (root / "README.md").write_text("dirty source")
            with self.assertRaises(ValueError):
                self.package.source_archive(root, dependency, vendor, folder / "dirty.tar.gz", "fixture")


if __name__ == "__main__":
    unittest.main()
