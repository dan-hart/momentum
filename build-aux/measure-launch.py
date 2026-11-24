#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Dan Hart
"""Sample process start to an enabled capture control and a synthetic Today row."""
import argparse
import contextlib
import hashlib
import json
import os
import plistlib
import re
import shutil
import statistics
import subprocess
import tempfile
import time
import uuid
from pathlib import Path
from xml.etree import ElementTree

ROOT = Path(__file__).resolve().parents[1]
TITLE = "Momentum launch fixture "
ANDROID_BUNDLE = "com.codedbydan.momentum"
STORE_FILES = ("state.json", "pending.json", "meta.json")
SIZES = (10, 1000)


def run(args, *, env=None, input=None, statuses=(0,), timeout=30):
    result = subprocess.run(
        args, env=env, input=input, capture_output=True, timeout=timeout, check=False,
    )
    if result.returncode not in statuses:
        raise RuntimeError(f"{args[0]} failed ({result.returncode}): "
                           f"{result.stderr.decode(errors='replace').strip()}")
    return result.stdout.decode(errors="replace")


def elapsed_ms(started):
    return round((time.monotonic() - started) * 1000, 1)


def sample(launch, ready):
    started = time.monotonic()
    launch()
    launch_return_ms = elapsed_ms(started)
    deadline = started + 30
    polls = 0
    while time.monotonic() < deadline:
        polls += 1
        if ready():
            return {"actionable_ms": elapsed_ms(started), "launch_return_ms": launch_return_ms, "polls": polls}
        time.sleep(0.075)
    raise RuntimeError("Today never exposed the synthetic task row and capture control within 30 seconds")


def descendants(value):
    if isinstance(value, list):
        for element in value:
            yield from descendants(element)
    elif isinstance(value, dict):
        yield value
        yield from descendants(value.get("children", []))


def ios_ready(tree):
    elements = list(descendants(tree))
    enabled = [
        element for element in elements if element.get("enabled", True) and element.get("isEnabled", True)
        and element.get("AXEnabled", True) and not element.get("AXHidden", False)
    ]
    return (
        any(element.get("AXUniqueId") == "floating-add-task" or element.get("AXLabel") == "Add task"
            for element in enabled)
        and any((element.get("AXLabel") or "").startswith("Open " + TITLE) for element in enabled)
    )


def android_ready(xml):
    if not xml.strip():
        return False
    nodes = list(ElementTree.fromstring(xml).iter("node"))
    enabled = [node.attrib for node in nodes
               if node.get("enabled") == "true" and node.get("package") == ANDROID_BUNDLE]
    return (
        any(node.get("class") == "android.widget.EditText" for node in enabled)
        and any((node.get("content-desc", "") or node.get("text", "")).startswith(
            ("Mark as done: " + TITLE, TITLE)
        ) for node in enabled)
    )


def copy_store(source, destination):
    destination.mkdir()
    for filename in STORE_FILES:
        shutil.copyfile(source / filename, destination / filename)


@contextlib.contextmanager
def ios_target(args):
    sim = ["xcrun", "simctl"]
    devices = json.loads(run([*sim, "list", "devices", "-j"]))["devices"]
    target = next(
        ((runtime, device) for runtime, group in devices.items()
         for device in group if device["udid"] == args.target),
        None,
    )
    if not target or not target[1].get("isAvailable") or target[1]["state"] != "Booted":
        raise RuntimeError("Choose an explicitly identified, booted disposable simulator")
    runtime, device = target
    info = plistlib.loads((args.app / "Info.plist").read_bytes())
    bundle = info["CFBundleIdentifier"]
    binary = args.app / info["CFBundleExecutable"]
    probe = ["axe", "describe-ui", "--minimal-keys", "--udid", args.target]
    run([*sim, "install", args.target, str(args.app)])
    container = Path(run([*sim, "get_app_container", args.target, bundle, "data"]).strip())
    def stop():
        # simctl uses ESRCH (3) when the app is already stopped.
        run([*sim, "terminate", args.target, bundle], statuses=(0, 3))

    with tempfile.TemporaryDirectory(prefix="momentum-launch-", dir=container / "tmp") as folder:
        def measure(fixture):
            stop()
            destination = Path(folder) / fixture.name
            copy_store(fixture, destination)
            env = {**os.environ, "SIMCTL_CHILD_MOMENTUM_DATA_DIR": str(destination)}
            return sample(
                lambda: run([*sim, "launch", "--terminate-running-process", args.target, bundle,
                             "-AppleLanguages", "(en)", "-AppleLocale", "en_US",
                             "-sync-method", "off"], env=env),
                lambda: ios_ready(json.loads(run(probe, timeout=10) or "[]")),
            )

        try:
            yield measure, binary, {"target": device["name"], "runtime": runtime, "probe": "AXe"}
        finally:
            stop()


@contextlib.contextmanager
def android_target(args):
    if not re.fullmatch(r"emulator-\d+", args.target):
        raise RuntimeError("Choose an explicit disposable emulator serial")
    adb = ["adb", "-s", args.target]
    shell = [*adb, "shell"]
    run([*adb, "emu", "avd", "name"])
    existing = run([*shell, "pm", "path", ANDROID_BUNDLE], statuses=(0, 1))
    if existing.strip():
        raise RuntimeError("Use a disposable emulator without Momentum installed so the fixture starts with default settings")
    run([*adb, "install", str(args.app)], timeout=90)
    app = [*shell, "run-as", ANDROID_BUNDLE]
    dump = f"/data/local/tmp/momentum-launch-{uuid.uuid4().hex}.xml"
    cases = 0

    def stop():
        run([*shell, "am", "force-stop", ANDROID_BUNDLE])

    def measure(fixture):
        nonlocal cases
        # Both fixture sizes start cold. Replace only the fixture store created
        # here, keeping the emulator, app, settings and build constant.
        stop()
        if cases:
            run([*app, "rm", "-rf", "no_backup/tasks"])
        run([*app, "mkdir", "-p", "no_backup/tasks"])
        cases += 1
        for filename in STORE_FILES:
            command = f"run-as {ANDROID_BUNDLE} sh -c 'cat > no_backup/tasks/{filename}'"
            run([*adb, "exec-in", command], input=(fixture / filename).read_bytes())

        def ready():
            run([*shell, "rm", "-f", dump])
            run([*shell, "uiautomator", "dump", dump], timeout=10, statuses=(0, 1))
            return android_ready(run([*shell, "cat", dump], statuses=(0, 1)))

        return sample(
            lambda: run([*shell, "am", "start", "-n", f"{ANDROID_BUNDLE}/.MainActivity"]),
            ready,
        )

    try:
        runtime = run([*shell, "getprop", "ro.build.fingerprint"]).strip()
        yield measure, args.app, {"target": args.target, "runtime": runtime, "probe": "UI Automator"}
    finally:
        stop()
        run([*shell, "rm", "-f", dump])
        if cases:
            run([*app, "rm", "-rf", "no_backup/tasks"])


@contextlib.contextmanager
def macos_target(args):
    info = plistlib.loads((args.app / "Contents" / "Info.plist").read_bytes())
    binary = args.app / "Contents" / "MacOS" / info["CFBundleExecutable"]
    with tempfile.TemporaryDirectory(prefix="momentum-launch-ax-") as folder:
        probe = Path(folder) / "ax-audit"
        run(["xcrun", "swiftc", str(ROOT / "macos/scripts/a11y-audit.swift"), "-o", str(probe)], timeout=90)

        def measure(fixture):
            env = {**os.environ, "MOMENTUM_DATA_DIR": str(fixture)}
            env.pop("MOMENTUM_DEMO", None)
            process = None

            def launch():
                nonlocal process
                process = subprocess.Popen(
                    [binary, "-AppleLanguages", "(en)", "-AppleLocale", "en_US", "-sync-method", "off"],
                    env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                )

            def ready():
                if process.poll() is not None:
                    raise RuntimeError("Momentum exited before Today became actionable")
                output = run([probe, "--launch-ready", "--pid", str(process.pid)], statuses=(0, 2), timeout=10)
                return output.strip() == "momentum-launch-ready"

            try:
                return sample(launch, ready)
            finally:
                if process and process.poll() is None:
                    process.kill()
                    process.wait()

        yield measure, binary, {"target": "isolated macOS user session",
                                "runtime": run(["sw_vers", "-productVersion"]).strip(), "probe": "native AX"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("platform", choices=("ios", "android", "macos"))
    parser.add_argument("--app", required=True, type=lambda value: Path(value).resolve(strict=True))
    parser.add_argument("--target", help="Exact booted iOS UDID or Android emulator serial")
    parser.add_argument("--samples", type=int, default=5)
    parser.add_argument("--confirm-disposable", required=True, action="store_true",
                        help="Confirm this is an owned disposable English simulator/emulator or isolated macOS desktop and user")
    args = parser.parse_args()
    if args.samples < 1 or (args.platform != "macos" and not args.target):
        parser.error("a positive sample count and an explicit simulator/emulator target are required")
    runners = {"ios": ios_target, "android": android_target, "macos": macos_target}
    output = {"platform": args.platform, "mode": "cold process, OS file cache retained", "fixtures": {}}
    with tempfile.TemporaryDirectory(prefix="momentum-launch-fixtures-") as folder:
        fixture_paths = {
            (count, index): Path(folder) / f"{count}-{index}"
            for count in SIZES for index in range(args.samples)
        }
        for (count, _), path in fixture_paths.items():
            run(["cargo", "run", "--locked", "--quiet", "--manifest-path", str(ROOT / "Cargo.toml"),
                 "-p", "momentum-core", "--example", "launch_fixture", "--", str(path), str(count)], timeout=300)
        with runners[args.platform](args) as (measure, binary, metadata):
            output.update(metadata)
            digest = hashlib.sha256()
            with binary.open("rb") as artifact:
                for chunk in iter(lambda: artifact.read(1 << 20), b""):
                    digest.update(chunk)
            output["sha256"] = digest.hexdigest()
            for count in SIZES:
                samples = []
                for index in range(args.samples):
                    try:
                        samples.append(measure(fixture_paths[count, index]))
                    except (RuntimeError, subprocess.TimeoutExpired) as error:
                        raise RuntimeError(f"{args.platform}, {count} tasks, sample {index + 1}: {error}") from error
                output["fixtures"][count] = {
                    "samples": samples,
                    "median_actionable_ms": statistics.median(sample["actionable_ms"] for sample in samples),
                }
    print(json.dumps(output, indent=2))


if __name__ == "__main__":
    main()
