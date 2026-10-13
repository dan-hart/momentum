#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Dan Hart
"""The real extraction pipeline must retain nested native callback messages."""
import importlib.util
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("translations", ROOT / "build-aux/update-translations.py")
translations = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(translations)

CALLBACK_KEYS = {
    "Export app settings", "Import app settings", "Choose a local file",
    "App settings saved", "App settings imported", "Notes copied",
    "Import Momentum backup", "Restore included app settings", "Replace all tasks?",
    "This replaces tasks on this device and discards unsynced task changes. App settings are optional; credentials and device controls stay local.",
    "Restore Backup", "Include app settings", "Export backup",
    "Save tasks, projects, tags, and repeat schedules. App settings are optional; credentials and device controls stay local.",
    "Continue", "A sync operation is already running", "Copying and verifying files…",
    "The original folder stays intact. You can cancel the copy.",
    "Sync folder copied and verified. This device now uses momentum.",
    "The saved connection changed. The folder was not switched.",
    "Folder copy could not finish.",
}


@unittest.skipUnless(shutil.which("xgettext"), "GNU gettext is required")
class ExtractionTests(unittest.TestCase):
    def test_current_native_callbacks_survive_extraction(self):
        with tempfile.TemporaryDirectory(prefix="momentum-extraction-test-") as directory:
            output = Path(directory) / "momentum.pot"
            translations.generate(ROOT, output)
            self.assertFalse(CALLBACK_KEYS - translations.message_ids(output.read_text()))

    def test_nested_macro_and_blueprint_both_survive(self):
        with tempfile.TemporaryDirectory(prefix="momentum-extraction-fixture-") as directory:
            root = Path(directory)
            (root / "po").mkdir()
            (root / "po/POTFILES.in").write_text("native.rs\nview.blp\n")
            (root / "native.rs").write_text('fn feedback() { glib::clone!(move || { gettext("Notes copied"); }); }\n')
            (root / "view.blp").write_text('Label { label: _("App settings"); }\n')
            output = root / "messages.pot"
            translations.generate(root, output)
            self.assertEqual(translations.message_ids(output.read_text()), {"Notes copied", "App settings"})

    @unittest.skipUnless(shutil.which("meson") and shutil.which("ninja"), "Meson and Ninja are required")
    def test_normal_meson_maintainer_target_uses_supplemental_pass(self):
        with tempfile.TemporaryDirectory(prefix="momentum-meson-extraction-") as directory:
            root = Path(directory)
            (root / "po").mkdir()
            (root / "build-aux").mkdir()
            (root / "meson.build").write_text("project('momentum-localization-test')\ni18n = import('i18n')\ngettext_package = 'momentum'\nsubdir('po')\n")
            shutil.copy(ROOT / "po/meson.build", root / "po/meson.build")
            shutil.copy(ROOT / "build-aux/update-translations.py", root / "build-aux/update-translations.py")
            (root / "po/POTFILES.in").write_text("native.rs\nview.blp\n")
            (root / "po/LINGUAS").write_text("de\n")
            (root / "po/de.po").write_text('msgid ""\nmsgstr ""\n"Language: de\\n"\n"Content-Type: text/plain; charset=UTF-8\\n"\n')
            (root / "native.rs").write_text('fn feedback() { glib::clone!(move || { gettext("Notes copied"); }); }\n')
            (root / "view.blp").write_text('Label { label: _("App settings"); }\n')
            for command in (["meson", "setup", str(root / "build"), str(root)],
                            ["meson", "compile", "-C", str(root / "build"), "momentum-update-translations"]):
                result = subprocess.run(command, capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertEqual(translations.message_ids((root / "po/momentum.pot").read_text()),
                             {"Notes copied", "App settings"})


if __name__ == "__main__":
    unittest.main()
