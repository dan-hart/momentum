#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Dan Hart
"""Verify the approved completion masters, short PCM WAV shape, and native copies."""
import hashlib
import json
from pathlib import Path
import re
import struct
import sys
from urllib.parse import quote
import wave
from xml.etree import ElementTree

ROOT = Path(__file__).resolve().parents[1]
REVISION = "c1ea7bcc3c7309650ab0da9d15c9cd1fbc4a4c7e"
# Immutable hashes of the ten previews approved on 2026-10-11.
APPROVED = {
    "wood-pop": "e38ab4a1d0216bac2e88aad767d5027ab6339caa83300a86947b956d2e594b28",
    "double-check": "6d6ad0d6f3a7cac492294b46b3e319af1f3250eeb25e41ed912a4348c1863368",
    "rounded-lift": "1c62315ecef6cca1bcd09e605e61124d36baf5c47ea1b00121dd8a8cb41566bc",
    "pocket-piano": "7ff557a44654bb3ef8d769c48eaa31ea16d7c7dbe628792882392426b0bf0721",
    "string-flick": "4bfd34325bbbc7f2baea000335dcfd89bb376f8b0acf695b298c976687c5e64d",
    "bright-finish": "87903557cd5d790e37019358b70ce213373e7988a6421b90f8097b1ba243527f",
    "harp-snap": "b8e065b15e28e1b486face48e2ca3dfb3db623d4845f1d19afb768838e60dd45",
    "click-and-glow": "a86dbb5689e7b31489849255e597bc3ae7aa4474f98d3b96e2a76ea7c898762b",
    "tiny-ta-da": "58ad4aed4ad6ebd5f737604f5595ec4efa26f196ee1132602fba7d1d5145c9a2",
    "warm-spark": "3e3ba756e1679e51465842450c58daf0ad39be82eeeff124a756c302c51fdd6e",
}


def check(root=ROOT):
    errors = []
    directory = root / "data/resources/sounds"
    manifest = json.loads((directory / "provenance.json").read_text())
    options = manifest["options"]
    if manifest.get("revision") != REVISION or manifest.get("license") != "CC0-1.0":
        errors.append("The VCSL revision and CC0 license must stay pinned")
    if manifest.get("default") != "bright-finish":
        errors.append("The default must remain option 6, Bright Finish")
    if [o["slug"] for o in options] != list(APPROVED) or [o["id"] for o in options] != list(range(1, 11)):
        errors.append("The approved numbered catalog must contain exactly ten sounds in order")
    if len(set(APPROVED.values())) != 10:
        errors.append("Each approved option must have a distinct hash")
    if "/Users/" in json.dumps(manifest):
        errors.append("Provenance must not contain private local paths")
    source_paths = manifest.get("sources", {})
    for path, source in source_paths.items():
        expected_url = f"https://raw.githubusercontent.com/sgossner/VCSL/{REVISION}/{quote(path)}"
        if source.get("path") != path or source.get("url") != expected_url:
            errors.append(f"{path}: source URL must identify the pinned recording")
        if not re.fullmatch(r"[0-9a-f]{64}", source.get("sha256", "")) or not re.fullmatch(r"[0-9a-f]{40}", source.get("git_blob", "")):
            errors.append(f"{path}: missing source SHA256 or Git blob identity")
    for option in options:
        slug = option["slug"]
        if option.get("sha256") != APPROVED.get(slug):
            errors.append(f"{slug}: provenance differs from approved master")
        if any(event[0] not in source_paths for event in option["events"]):
            errors.append(f"{slug}: source recording has no pinned provenance")
        copies = [directory / f"{slug}.wav",
                  root / "macos/Packages/MomentumKit/Sources/MomentumKit/Resources" / f"{slug}.wav",
                  root / "android/app/src/main/res/raw" / f"{slug.replace('-', '_')}.wav"]
        for file in copies:
            label = str(file.relative_to(root))
            if not file.is_file():
                errors.append(f"Missing {label}")
                continue
            data = file.read_bytes()
            if hashlib.sha256(data).hexdigest() != APPROVED.get(slug):
                errors.append(f"{label}: bytes differ from approved preview")
            try:
                with wave.open(str(file)) as audio:
                    if (audio.getnchannels(), audio.getsampwidth(), audio.getframerate(), audio.getcomptype()) != (1, 2, 44100, "NONE"):
                        errors.append(f"{label}: expected uncompressed PCM16 mono at 44.1 kHz")
                    duration = audio.getnframes() / audio.getframerate()
                    if not 0.1 <= duration <= 0.6 or abs(duration - option["duration"]) > 1 / 44100:
                        errors.append(f"{label}: wrong short duration {duration}")
                    frames = audio.readframes(audio.getnframes())
                    if len(frames) != audio.getnframes() * 2 or not any(struct.unpack(f"<{len(frames) // 2}h", frames)):
                        errors.append(f"{label}: truncated or silent PCM samples")
            except (wave.Error, EOFError, ValueError, struct.error) as error:
                errors.append(f"{label}: invalid WAV ({error})")
    for old in (directory / "task-complete.wav",
                root / "macos/Packages/MomentumKit/Sources/MomentumKit/Resources/task-complete.wav"):
        if old.exists():
            errors.append(f"Obsolete clip remains: {old.relative_to(root)}")
    resources = ElementTree.parse(root / "data/resources/resources.gresource.xml")
    bundled = [node.text for node in resources.iter("file") if (node.text or "").startswith("sounds/")]
    if bundled != [f"sounds/{slug}.wav" for slug in APPROVED]:
        errors.append("Linux gresource must bundle exactly the ten approved WAVs in catalog order")
    swift = (root / "macos/Packages/MomentumKit/Sources/MomentumKit/CompletionSoundChoice.swift").read_text()
    if re.findall(r'case \w+ = "([^"]+)"', swift) != list(APPROVED):
        errors.append("Apple catalog slugs must match the approved order")
    kotlin_catalogs = list((root / "android/app/src/main").rglob("CompletionSoundChoice.kt"))
    if len(kotlin_catalogs) != 1:
        errors.append("Android must have one CompletionSoundChoice catalog")
    elif re.findall(r'^\s*[A-Za-z][A-Za-z0-9_]*\("([^"]+)"', kotlin_catalogs[0].read_text(), re.MULTILINE) != list(APPROVED):
        errors.append("Android catalog slugs must match the approved order")
    return errors


def main():
    try:
        errors = check()
    except (OSError, KeyError, ValueError, ElementTree.ParseError) as error:
        errors = [str(error)]
    for error in errors:
        print(f"ERROR: {error}", file=sys.stderr)
    if errors:
        return 1
    print("Completion sounds: all 10 approved PCM16 masters match across Linux, Apple and Android")
    return 0


if __name__ == "__main__":
    sys.exit(main())
