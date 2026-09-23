#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Sync the iOS compiler catalog and reuse Momentum's German translation validator."""
import argparse
import importlib.util
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
CATALOG = ROOT / "ios/Momentum/Resources/Localizable.xcstrings"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sync", type=Path, metavar="DERIVED_DATA")
    args = parser.parse_args()
    spec = importlib.util.spec_from_file_location("momentum_localization", ROOT / "macos/scripts/localize.py")
    shared = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(shared)
    if args.sync:
        folder = args.sync / "Build/Intermediates.noindex/Momentum.build"
        data = sorted(folder.rglob("arm64/*.stringsdata"))
        if not data:
            raise SystemExit("Build the iOS app with SWIFT_EMIT_LOC_STRINGS=YES before syncing.")
        subprocess.run(["xcrun", "xcstringstool", "sync", str(CATALOG), "--stringsdata", *map(str, data)], check=True)
    catalog = json.loads(CATALOG.read_text())
    result = shared.validate(catalog)
    print(f"iOS: {result['translated']}/{result['active']} German entries translated")
    for message in result["errors"]:
        print(f"ERROR: {message}")
    for key in result["missing"]:
        print(f"MISSING: {key}")
    raise SystemExit(bool(result["errors"] or result["missing"]))


if __name__ == "__main__":
    main()
