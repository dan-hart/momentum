# SPDX-License-Identifier: GPL-3.0-or-later
import importlib.util
import os
from pathlib import Path
import plistlib
import tempfile
import sys
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("mobile_test_runner", Path(__file__).parents[1] / "test.py")
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class CoreFreshnessTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.source = self.root / "crates/example/src/lib.rs"
        self.source.parent.mkdir(parents=True)
        self.source.write_text("pub fn value() -> u8 { 1 }")
        patcher = patch.object(runner, "ROOT", self.root)
        patcher.start()
        self.addCleanup(patcher.stop)

    def test_source_edit_is_detected_even_with_unchanged_timestamp_and_size(self):
        before = runner.core_fingerprint()
        timestamp = self.source.stat()
        self.source.write_text("pub fn value() -> u8 { 2 }")
        os.utime(self.source, ns=(timestamp.st_atime_ns, timestamp.st_mtime_ns))
        self.assertNotEqual(runner.core_fingerprint(), before)

    def test_new_untracked_source_and_deletion_are_detected(self):
        before = runner.core_fingerprint()
        added = self.source.parent / "new.rs"
        added.write_text("pub fn added() {}")
        self.assertNotEqual(runner.core_fingerprint(), before)
        added.unlink()
        self.assertEqual(runner.core_fingerprint(), before)

    def test_generated_rust_build_output_does_not_invalidate_source_stamp(self):
        before = runner.core_fingerprint()
        generated = self.root / "libresync-src/target/debug/generated.rs"
        generated.parent.mkdir(parents=True)
        generated.write_text("generated build output")
        self.assertEqual(runner.core_fingerprint(), before)

    def test_local_path_dependency_edit_invalidates_source_stamp(self):
        dependency = self.root / "libresync-src/src/lib.rs"
        dependency.parent.mkdir(parents=True)
        dependency.write_text("pub fn dependency() {}")
        before = runner.core_fingerprint()
        dependency.write_text("pub fn changed_dependency() {}")
        self.assertNotEqual(runner.core_fingerprint(), before)

    def test_replaced_generated_library_is_detected(self):
        library = self.root / "momentum_ffi.xcframework/ios/libmomentum.a"
        library.parent.mkdir(parents=True)
        library.write_bytes(b"old library")
        with patch.object(runner, "CORE", self.root):
            before = runner.artifacts()
            library.write_bytes(b"replacement library")
            self.assertNotEqual(runner.artifacts(), before)


class TestSelectionTests(unittest.TestCase):
    def run_log(self, label, text):
        with tempfile.TemporaryDirectory() as directory, patch.object(runner, "OUTPUT", Path(directory)):
            timings = {}
            runner.run(label, [sys.executable, "-c", f"print({text!r})"], timings)
            return timings[label]

    def test_zero_test_filter_is_failure_for_every_lane(self):
        for label in ("fast", "views", "transport"):
            with self.subTest(label=label), self.assertRaisesRegex(RuntimeError, "no executed tests"):
                self.run_log(label, "Test run with 0 tests passed\nExecuted 0 tests, with 0 failures")

    def test_swift_testing_results_override_empty_xctest_preamble(self):
        result = self.run_log("fast", "Executed 0 tests\nTest run with 67 tests in 12 suites passed")
        self.assertEqual(result["tests"], 67)

    def test_native_test_count_is_detected(self):
        result = self.run_log("views", "Executed 8 tests, with 0 failures")
        self.assertEqual(result["tests"], 8)

    def test_non_test_commands_do_not_require_test_counts(self):
        self.assertEqual(self.run_log("generate", "Generated project")["exitCode"], 0)


class ProjectShapeTests(unittest.TestCase):
    def test_production_info_plist_uses_project_version_settings(self):
        with (runner.ROOT / "ios/Momentum/Info.plist").open("rb") as source:
            info = plistlib.load(source)
        self.assertEqual(info["CFBundleShortVersionString"], "$(MARKETING_VERSION)")
        self.assertEqual(info["CFBundleVersion"], "$(CURRENT_PROJECT_VERSION)")

    def test_ios_project_has_no_xcui_targets_or_sources(self):
        project = (runner.ROOT / "ios/project.yml").read_text()
        self.assertNotIn("bundle.ui-testing", project)
        for name in ("MomentumUITests", "MomentumSyncUITests", "MomentumIntentTests", "MomentumSyncTestHost"):
            self.assertFalse((runner.ROOT / "ios" / name).exists(), name)
        for source in (runner.ROOT / "ios").rglob("*.swift"):
            if "DerivedData" not in source.parts:
                self.assertNotIn("XCUIApplication", source.read_text(), str(source))

    def test_ios_project_has_no_viewinspector_dependency_or_imports(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "ios/Sample").mkdir(parents=True)
            (root / "ios/project.yml").write_text(
                "packages:\n  ViewInspector: https://example.invalid\n"
            )
            (root / "ios/Package.swift").write_text(".package(url: \"ViewInspector\")")
            (root / "ios/Sample/View.swift").write_text("import ViewInspector\n")
            violations = runner.ios_project_shape_violations(root)
            self.assertEqual(len(violations), 3)
            self.assertTrue(all("ViewInspector" in violation for violation in violations))


if __name__ == "__main__":
    unittest.main()
