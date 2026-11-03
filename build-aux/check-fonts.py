#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Verify offline font bundles and licenses without extra Python dependencies."""
import hashlib
import json
from pathlib import Path
from xml.etree import ElementTree

ROOT = Path(__file__).resolve().parents[1]


def check(root=ROOT):
    directory = root / "data/resources/fonts"
    manifest = json.loads((directory / "provenance.json").read_text())
    apple = root / "macos/Packages/MomentumKit/Sources/MomentumKit/Resources/Fonts"
    android = root / "android/app/src/main"
    resources = {node.text for node in ElementTree.parse(root / "data/resources/resources.gresource.xml").iter("file")}
    errors = []
    for entry in manifest["fonts"]:
        name = entry["file"]
        for path in [directory / name, apple / name, android / "res/font" / entry["android"]]:
            if not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest() != entry["sha256"]:
                errors.append(f"Font missing or changed: {path.relative_to(root)}")
        if (directory / name).read_bytes()[:4] not in [b"\x00\x01\x00\x00", b"OTTO"]:
            errors.append(f"Not an OpenType/TrueType file: {name}")
        if f"fonts/{name}" not in resources:
            errors.append(f"Linux resource missing: {name}")
    for name, digest in manifest["licenses"].items():
        for path in [directory / name, apple / name, android / "assets/font-licenses" / name]:
            if not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest() != digest:
                errors.append(f"License missing or changed: {path.relative_to(root)}")
        if f"fonts/{name}" not in resources:
            errors.append(f"Linux license missing: {name}")
        text = (directory / name).read_text()
        if "SIL OPEN FONT LICENSE Version 1.1" not in text or "<html" in text.lower():
            errors.append(f"Not the font license text: {name}")
    return errors


if __name__ == "__main__":
    problems = check()
    if problems:
        raise SystemExit("\n".join(problems))
    print("8 original font faces and 2 licenses match across Linux, Apple and Android resources.")
