#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Dan Hart
"""Sync compiler-extracted catalogs, import confirmed gettext, and audit German.

Uses only Python's standard library. Never use lightweight Swift extraction: it
loses interpolation types, creating keys that Foundation will not look up.
"""
import argparse
import ast
from collections import Counter
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
CATALOGS = {
    "Momentum": ROOT / "macos/Momentum/Resources/Localizable.xcstrings",
    "MomentumKit": ROOT / "macos/Packages/MomentumKit/Sources/MomentumKit/Resources/Localizable.xcstrings",
}
FORMAT = re.compile(r"%(?:(\d+)\$)?(?:[-+ #0]*)(?:\d+)?(?:\.\d+)?(lld|llu|ld|lu|[diuoxXfFeEgGaAcCsSp@])")


def parse_po(text):
    entries, entry, field = [], {}, None
    for line in text.splitlines() + [""]:
        if not line.strip():
            if entry.get("msgid"):
                entries.append(entry)
            entry, field = {}, None
        elif line.startswith("#, "):
            entry["fuzzy"] = "fuzzy" in line[3:].split(", ")
        elif line.startswith("#"):
            continue
        elif line.startswith('"') and field:
            entry[field] += ast.literal_eval(line)
        else:
            match = re.match(r'(msgctxt|msgid_plural|msgid|msgstr(?:\[\d+\])?) (".*")$', line)
            if match:
                field = match[1]
                entry[field] = ast.literal_eval(match[2])
    return entries


def confirmed_translations(entries):
    return {e["msgid"]: e["msgstr"] for e in entries
            if e.get("msgstr") and not e.get("fuzzy") and not e.get("msgctxt") and not e.get("msgid_plural")}


def import_gettext(catalog, translations):
    count = 0
    for key, entry in catalog["strings"].items():
        if "de" in entry.get("localizations", {}):
            continue  # Preserve catalog edits and plural variants.
        translation = translations.get(key)
        if translation is None:
            placeholders = list(FORMAT.finditer(key))
            normalized = FORMAT.sub("{}", key)
            candidate = translations.get(normalized)
            if candidate is not None and candidate.count("{}") == len(placeholders):
                translation = candidate
                for placeholder in placeholders:
                    translation = translation.replace("{}", placeholder[0], 1)
        if translation and placeholder_signature(key) == placeholder_signature(translation):
            entry.setdefault("localizations", {})["de"] = {
                "stringUnit": {"state": "translated", "value": translation}}
            count += 1
    return count


def placeholder_signature(text):
    # Associate each type with its argument index, accepting explicit reordering.
    return Counter((int(m[1]) if m[1] else i, m[2])
                   for i, m in enumerate(FORMAT.finditer(text.replace("%%", "")), 1))


def units(value):
    if isinstance(value, dict):
        if "stringUnit" in value:
            yield value["stringUnit"]
        for key, child in value.items():
            if key != "stringUnit":
                yield from units(child)


def validate(catalog):
    errors, missing = [], []
    active = 0
    for key, entry in catalog["strings"].items():
        if "%arg" in key:
            errors.append(f"Untyped lightweight-extraction key: {key}")
        if entry.get("extractionState") == "stale" or entry.get("shouldTranslate") is False:
            continue
        active += 1
        german = list(units(entry.get("localizations", {}).get("de", {})))
        if not german or any(u.get("state") != "translated" or not u.get("value") for u in german):
            missing.append(key)
        for unit in german:
            if placeholder_signature(key) != placeholder_signature(unit.get("value", "")):
                errors.append(f"German placeholder mismatch: {key!r} -> {unit.get('value')!r}")
            # App Intent parameter-summary placeholders are not printf arguments.
            if Counter(re.findall(r"\$\{[^}]+\}", key)) != Counter(re.findall(r"\$\{[^}]+\}", unit.get("value", ""))):
                errors.append(f"Intent parameter mismatch: {key}")
    return {"active": active, "translated": active - len(missing), "missing": missing, "errors": errors}


def sync_catalogs(derived_data):
    intermediates = derived_data / "Build/Intermediates.noindex"
    for target, path in CATALOGS.items():
        data = sorted((intermediates / f"{target}.build").rglob("*.stringsdata"))
        if not data:
            raise SystemExit(f"No compiler stringsdata for {target}; build with SWIFT_EMIT_LOC_STRINGS=YES first.")
        subprocess.run(["xcrun", "xcstringstool", "sync", str(path), "--stringsdata", *map(str, data)], check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sync", type=Path, metavar="DERIVED_DATA", help="sync keys from a completed Xcode compiler build")
    parser.add_argument("--import-gettext", action="store_true", help="fill missing German entries from non-fuzzy po/de.po")
    parser.add_argument("--require-complete", action="store_true", help="fail if any active German entry remains untranslated")
    args = parser.parse_args()
    if args.sync:
        sync_catalogs(args.sync)
    translations = confirmed_translations(parse_po((ROOT / "po/de.po").read_text()))
    failed = False
    catalogs = {**CATALOGS, "Momentum InfoPlist": ROOT / "macos/Momentum/Resources/InfoPlist.xcstrings",
                "Momentum AppShortcuts": ROOT / "macos/Momentum/Resources/AppShortcuts.xcstrings"}
    for target, path in catalogs.items():
        catalog = json.loads(path.read_text())
        if args.import_gettext:
            count = import_gettext(catalog, translations)
            path.write_text(json.dumps(catalog, ensure_ascii=False, indent=2, sort_keys=True) + "\n")
            print(f"{target}: imported {count} confirmed gettext translations")
        result = validate(catalog)
        print(f"{target}: {result['translated']}/{result['active']} German entries translated; {len(result['missing'])} missing")
        for error in result["errors"]:
            print(f"ERROR: {error}")
        for key in result["missing"]:
            print(f"MISSING: {key}")
        failed |= bool(result["errors"] or (args.require_complete and result["missing"]))
    raise SystemExit(1 if failed else 0)


if __name__ == "__main__":
    main()
