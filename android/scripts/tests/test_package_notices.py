# SPDX-License-Identifier: GPL-3.0-or-later
import importlib.util
import io
import hashlib
import json
import tempfile
import unittest
import zipfile
from pathlib import Path

SCRIPT = Path(__file__).parents[1] / "package_notices.py"
spec = importlib.util.spec_from_file_location("package_notices", SCRIPT)
notices = importlib.util.module_from_spec(spec)
spec.loader.exec_module(notices)


class PackageNoticesTests(unittest.TestCase):
    def test_retains_aar_and_nested_jar_notices_without_private_paths(self):
        jar = io.BytesIO()
        with zipfile.ZipFile(jar, "w") as archive:
            archive.writestr("META-INF/LICENSE", "Copyright nested author\nMIT license")
            archive.writestr("example.class", b"not a license")
        with tempfile.TemporaryDirectory() as directory:
            aar = Path(directory) / "library.aar"
            with zipfile.ZipFile(aar, "w") as archive:
                archive.writestr("META-INF/NOTICE.txt", "Copyright AAR author")
                archive.writestr("classes.jar", jar.getvalue())
            extracted = notices.archive_notices(aar)
        self.assertEqual(extracted, [("META-INF/NOTICE.txt", b"Copyright AAR author"),
                                    ("classes.jar/META-INF/LICENSE", b"Copyright nested author\nMIT license")])

    def test_inventory_excludes_dev_edges_but_preserves_native_build_inputs(self):
        packages = [{"id": name, "name": name} for name in ["momentum-ffi", "core", "build-tool", "test-helper"]]
        edges = [{"pkg": name, "dep_kinds": [{"kind": kind}]} for name, kind in
                 [("core", None), ("build-tool", "build"), ("test-helper", "dev")]]
        nodes = [{"id": name, "deps": edges if name == "momentum-ffi" else []} for name in
                 ["momentum-ffi", "core", "build-tool", "test-helper"]]
        metadata = {"packages": packages, "resolve": {"nodes": nodes}}
        self.assertEqual({p["name"] for p in notices.rust_dependencies(metadata)}, {"momentum-ffi", "core", "build-tool"})

    def test_pom_license_declarations_are_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            pom = Path(directory) / "library.pom"
            pom.write_text('<project xmlns="http://maven.apache.org/POM/4.0.0"><licenses><license>'
                           '<name>Apache-2.0</name><url>https://www.apache.org/licenses/LICENSE-2.0.txt</url>'
                           '</license></licenses></project>')
            self.assertEqual(notices.pom_licenses(pom), [("Apache-2.0", "https://www.apache.org/licenses/LICENSE-2.0.txt")])

    def test_inherited_maven_license_is_read_from_cached_parent(self):
        with tempfile.TemporaryDirectory() as directory:
            cache = Path(directory) / "files-2.1"
            parent = cache / "example/parent/1/hash/parent-1.pom"
            child = cache / "example/child/1/hash/child-1.pom"
            parent.parent.mkdir(parents=True)
            child.parent.mkdir(parents=True)
            parent.write_text('<project xmlns="http://maven.apache.org/POM/4.0.0"><licenses><license>'
                              '<name>Apache-2.0</name><url>https://example.org/license</url>'
                              '</license></licenses></project>')
            child.write_text('<project xmlns="http://maven.apache.org/POM/4.0.0"><parent>'
                             '<groupId>example</groupId><artifactId>parent</artifactId><version>1</version>'
                             '</parent></project>')
            self.assertEqual(notices.pom_licenses(child), [("Apache-2.0", "https://example.org/license")])

    def test_duplicate_notice_text_keeps_all_attributions_without_absolute_paths(self):
        text = notices.render([], [], [("first component/LICENSE", b"Copyright author"),
                                       ("second component/LICENSE", b"Copyright author")])
        self.assertEqual(text.count("Copyright author"), 1)
        self.assertIn("first component/LICENSE", text)
        self.assertIn("second component/LICENSE", text)
        self.assertNotIn(str(Path.home()), text)

    def test_rebuilt_graphics_jni_preserves_source_identity_and_license_attribution(self):
        pin = {"version": "1.1.0", "source_commit": "reviewed-source-commit",
               "source_url": "https://android.googlesource.com/platform/frameworks/support/+/reviewed-source-commit/",
               "aar_sha256": "original-aar-hash", "classes_sha256": "unchanged-kotlin-hash"}
        text = notices.render([], [], [
            ("AndroidX Graphics Path LICENSE.txt", b"Original Apache copyright and license"),
            ("AndroidX Graphics Path JNI source provenance", notices.graphics_rebuild_notice(pin)),
        ])
        for expected in ["Original Apache copyright and license", "reviewed-source-commit",
                         "original-aar-hash", "unchanged-kotlin-hash", "16 KiB", "unchanged C++ sources"]:
            self.assertIn(expected, text)
        self.assertNotIn(str(Path.home()), text)

    def test_pinned_standard_library_inventory_and_full_licenses_are_rendered(self):
        directory = notices.ANDROID / "licenses"
        provenance = json.loads((directory / "provenance.json").read_text())
        supplied = []
        for item in provenance:
            data = (directory / item["file"]).read_bytes()
            self.assertEqual(item["sha256"], hashlib.sha256(data).hexdigest())
            supplied.append((item["file"] + " (" + item["url"] + ")", data))
        text = notices.render([], [], supplied)
        for attribution in ["Copyright notices for The Rust Standard Library",
                            "library/core/src/unicode", "1991-2024 Unicode, Inc",
                            "The Rust Project Developers (see https://thanks.rust-lang.org)",
                            "2016 The Fuchsia Authors"]:
            self.assertTrue(attribution in text, "Missing standard-library inventory attribution: " + attribution)
        version = json.loads((notices.ANDROID / "toolchain.json").read_text())["rust"]
        names = [f"Rust-{version}-COPYRIGHT-library.html"] + [
            f"Rust-{version}-{license}.txt" for license in
            ["Apache-2.0", "MIT", "BSD-2-Clause", "Unicode-3.0", "LLVM-exception"]]
        for name in names:
            matching = [item for item in provenance if item["file"] == name]
            self.assertEqual(1, len(matching), "Pinned standard-library source provenance is retained")
            self.assertTrue((directory / name).read_text() in text, "Complete upstream notice bytes are rendered: " + name)
            self.assertNotIn(str(Path.home()), matching[0]["url"])
        self.assertNotIn(str(Path.home()), text)


if __name__ == "__main__":
    unittest.main()
