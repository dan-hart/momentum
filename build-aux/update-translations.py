#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Dan Hart
"""Refresh gettext resources, including strings inside GTK callback macros."""
import argparse
import ast
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


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
    subprocess.run([*args, *files], cwd=root, check=True)
    # gettext >= 0.24 recognizes Rust but skips arbitrary procedural macro
    # bodies. GTK's glib::clone! contains ordinary gettext calls, so retain the
    # earlier C-parser extraction as an additional pass. Older gettext already
    # falls back to that parser for .rs; joining the same entries is harmless.
    rust = [file for file in files if file.endswith(".rs")]
    if rust:
        subprocess.run([*args, "--language=C", "--join-existing", *rust],
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
