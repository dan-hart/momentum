#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Check that the archive belongs to the native Mac TestFlight channel."""
import pathlib
import plistlib
import subprocess
import sys


def main(app, version, build):
    contents = pathlib.Path(app) / "Contents"
    with (contents / "Info.plist").open("rb") as source:
        info = plistlib.load(source)
    expected = {
        "CFBundleIdentifier": "com.codedbydan.Momentum",
        "MomentumDistributionChannel": "testflight",
        "CFBundleShortVersionString": version,
        "CFBundleVersion": build,
        "ITSAppUsesNonExemptEncryption": False,
    }
    for key, value in expected.items():
        if info.get(key) != value:
            raise SystemExit(f"Unexpected {key} in Mac TestFlight archive: {info.get(key)!r}")
    if info.get("SUFeedURL"):
        raise SystemExit("Mac TestFlight archive has a Sparkle feed")
    for path in ("Resources/Info.plist", "Frameworks/Sparkle.framework", "MacOS/mo", "MacOS/MomentumRelaunch"):
        if (contents / path).exists():
            raise SystemExit(f"TestFlight must not ship {path}")
    executable = contents / "MacOS" / info["CFBundleExecutable"]
    architectures = subprocess.check_output(["lipo", "-archs", executable], text=True).split()
    if set(architectures) != {"arm64", "x86_64"}:
        raise SystemExit(f"Unexpected Mac TestFlight architectures: {architectures}")
    linked = subprocess.check_output(["otool", "-L", executable], text=True)
    if "Sparkle.framework" in linked:
        raise SystemExit("Mac TestFlight executable links Sparkle")
    print(f"Verified universal native Mac TestFlight archive {version} ({build})")


if __name__ == "__main__":
    main(*sys.argv[1:])
