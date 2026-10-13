#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Dan Hart
"""Refresh gettext resources with stable AppStream and GTK callback extraction."""
import argparse
import ast
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]

# System-selected AppStream ITS rules vary: gettext's fallback omits releases,
# and older versions escape XML entities unless told otherwise. Keep the
# translatable metadata fields explicit so every maintainer extracts the same
# keys, including current and historical release notes, on gettext 0.21 onward.
APPSTREAM_ITS = """<?xml version="1.0"?>
<its:rules xmlns:its="http://www.w3.org/2005/11/its"
           xmlns:gt="https://www.gnu.org/s/gettext/ns/its/extensions/1.0" version="2.0">
  <its:translateRule selector="/component" translate="no"/>
  <its:translateRule selector="/component/name | /component/summary | /component/description |
                               /component/developer_name | /component/developer/name |
                               /component/name_variant_suffix | /component/keywords/keyword |
                               /component/screenshots/screenshot/caption |
                               /component/releases/release/description |
                               /component/agreement/agreement_section/name |
                               /component/agreement/agreement_section/description" translate="yes"/>
  <its:withinTextRule selector="/component//description//em |
                               /component//description//code" withinText="yes"/>
  <its:translateRule selector="/component/name[@translate = 'no'] |
                               /component/developer_name[@translate = 'no'] |
                               /component/developer/name[@translate = 'no'] |
                               /component/name_variant_suffix[@translate = 'no'] |
                               /component/keywords/keyword[@translate = 'no'] |
                               /component/releases/release/description[@translate = 'no'] |
                               /component/agreement/agreement_section/name[@translate = 'no'] |
                               /component/agreement/agreement_section/description[@translate = 'no']" translate="no"/>
  <gt:escapeRule selector="/component" escape="no"/>
</its:rules>
"""


def message_ids(text):
    """Read complete msgids without confusing source references or plurals."""
    result = set()
    current = None
    for line in text.splitlines() + [""]:
        if line.startswith("msgid "):
            current = ast.literal_eval(line[6:])
        elif line.startswith('"') and current is not None:
            current += ast.literal_eval(line)
        elif current is not None:
            if current:
                result.add(current)
            current = None
    return result


def generate(root, output):
    files = [line.strip() for line in (root / "po/POTFILES.in").read_text().splitlines()
             if line.strip() and not line.lstrip().startswith("#")]
    args = ["xgettext", "--package-name=momentum", "--from-code=UTF-8",
            "--add-comments", "--keyword=_", "--keyword=N_",
            "--keyword=C_:1c,2", "--keyword=NC_:1c,2", "--keyword=gettext",
            "--keyword=ngettext:1,2", "--output=" + str(output)]
    metainfo = [file for file in files if file.endswith(
        (".metainfo.xml", ".metainfo.xml.in", ".metainfo.xml.in.in"))]
    sources = [file for file in files if file not in metainfo]
    if sources:
        subprocess.run([*args, *sources], cwd=root, check=True)
    # gettext >= 0.24 recognizes Rust but skips arbitrary procedural macro
    # bodies. GTK's glib::clone! contains ordinary gettext calls, so retain the
    # earlier C-parser extraction as an additional pass. Older gettext already
    # falls back to that parser for .rs; joining the same entries is harmless.
    rust = [file for file in files if file.endswith(".rs")]
    if rust:
        subprocess.run([*args, "--language=C", "--join-existing", *rust],
                       cwd=root, check=True)
    if metainfo:
        with tempfile.TemporaryDirectory(prefix="momentum-appstream-its-") as directory:
            rules = Path(directory) / "metainfo.its"
            rules.write_text(APPSTREAM_ITS)
            join = ["--join-existing"] if sources else []
            subprocess.run([*args, "--its=" + str(rules), *join, *metainfo],
                           cwd=root, check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true",
                        help="verify the checked-in template includes every extracted key")
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="momentum-translations-") as directory:
        generated = Path(directory) / "momentum.pot"
        generate(ROOT, generated)
        template = ROOT / "po/momentum.pot"
        if args.check:
            missing = message_ids(generated.read_text()) - message_ids(template.read_text())
            if missing:
                for key in sorted(missing):
                    print(f"Missing from po/momentum.pot: {key}")
                raise SystemExit(1)
            print("po/momentum.pot includes every extracted key, including GTK callbacks")
            return
        updated = []
        for line in (ROOT / "po/LINGUAS").read_text().splitlines():
            if line.strip() and not line.lstrip().startswith("#"):
                for language in line.split():
                    catalog = ROOT / "po" / f"{language}.po"
                    merged = subprocess.run(["msgmerge", "--quiet", str(catalog), str(generated)],
                                            check=True, capture_output=True, text=True)
                    updated.append((catalog, merged.stdout))
        template.write_text(generated.read_text())
        for catalog, text in updated:
            catalog.write_text(text)
        print(f"Updated po/momentum.pot and {len(updated)} translation catalog(s)")


if __name__ == "__main__":
    main()
