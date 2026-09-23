#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Incremental iOS test lanes. Run from any directory; no third-party Python packages."""
import argparse
import fcntl
import hashlib
import json
import os
import re
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
OUTPUT = ROOT / "ios/DerivedData/TestReports"
STAMP = OUTPUT / "core-prepared.json"
PACKAGE = ROOT / "ios/Packages/MomentumMobile"
CORE = ROOT / "macos/Packages/MomentumCore"


def ios_project_shape_violations(root=ROOT):
    """Reject slow UI automation targets and view-introspection dependencies."""
    ios = root / "ios"
    violations = []
    project = ios / "project.yml"
    if project.exists():
        text = project.read_text(errors="replace")
        for forbidden in ("bundle.ui-testing", "ViewInspector"):
            if forbidden in text:
                violations.append(f"{project}: forbidden {forbidden}")
    for name in ("MomentumUITests", "MomentumSyncUITests", "MomentumIntentTests", "MomentumSyncTestHost"):
        path = ios / name
        if path.exists():
            violations.append(f"{path}: forbidden UI automation source directory")
    for manifest in ios.rglob("Package.swift"):
        if "DerivedData" not in manifest.parts and "ViewInspector" in manifest.read_text(errors="replace"):
            violations.append(f"{manifest}: forbidden ViewInspector dependency")
    for source in ios.rglob("*.swift"):
        if "DerivedData" in source.parts:
            continue
        text = source.read_text(errors="replace")
        for forbidden in ("XCUIApplication", "import ViewInspector"):
            if forbidden in text:
                violations.append(f"{source}: forbidden {forbidden}")
    return violations


def core_fingerprint():
    """Catch edits (including untracked Rust sources) before using generated bindings."""
    paths = [ROOT / "Cargo.toml", ROOT / "Cargo.lock",
             ROOT / "macos/scripts/build-core.sh", ROOT / "ios/scripts/build-core.sh"]
    for folder in (ROOT / "crates", ROOT / "libresync-src", ROOT / ".cargo"):
        paths.extend(p for p in folder.rglob("*") if p.is_file()
                     and p.suffix in (".rs", ".toml", ".lock", ".udl")
                     and not {"target", ".git"}.intersection(p.relative_to(folder).parts))
    digest = hashlib.sha256()
    for path in sorted(paths):
        if path.exists():
            digest.update(str(path.relative_to(ROOT)).encode())
            digest.update(path.read_bytes())
    digest.update(os.environ.get("DEVELOPER_DIR", "").encode())
    return digest.hexdigest()


def artifacts():
    paths = list((CORE / "momentum_ffi.xcframework").rglob("*.a"))
    paths += [CORE / "Sources/MomentumCore/momentum.swift"]
    return {str(p.relative_to(CORE)): [p.stat().st_size, p.stat().st_mtime_ns]
            for p in sorted(paths) if p.is_file()}


def run(label, command, timings):
    log = OUTPUT / f"{label}.log"
    print(f"{label}: running (log: {log})", flush=True)
    started = time.monotonic()
    with log.open("w") as output:
        result = subprocess.run(command, cwd=ROOT, stdout=output, stderr=subprocess.STDOUT)
    timings[label] = {"seconds": round(time.monotonic() - started, 3), "exitCode": result.returncode}
    lines = log.read_text(errors="replace").splitlines()
    if result.returncode:
        print(f"{label}: FAIL in {timings[label]['seconds']}s")
        print("\n".join(lines[-50:]))
        raise subprocess.CalledProcessError(result.returncode, command)
    if label in ("fast", "views", "transport"):
        counts = [int(count) for count in re.findall(r"(?:Test run with|Executed)\s+(\d+)\s+tests?", "\n".join(lines))]
        if not counts or max(counts) == 0:
            timings[label]["exitCode"] = 1
            raise RuntimeError(f"{label}: no executed tests found; check the filter and {log}")
        timings[label]["tests"] = max(counts)
    print(f"{label}: PASS in {timings[label]['seconds']}s")
    for line in lines:
        if "Test run with" in line or ("Executed " in line and "Executed 0 tests" not in line) or "** TEST" in line:
            print(line)
    return log


def summarize_package_coverage(timings):
    log = run("coverage-path", ["swift", "test", "--package-path", str(PACKAGE),
                                "--show-codecov-path"], timings)
    path = Path(log.read_text().strip().splitlines()[-1])
    data = json.loads(path.read_text())
    rows = []
    for binary in data["data"]:
        for file in binary["files"]:
            if "/Sources/MomentumMobile/" in file["filename"]:
                rows.append({"file": str(Path(file["filename"]).relative_to(ROOT)),
                             **file["summary"]["lines"]})
    report = OUTPUT / "mobile-coverage.json"
    report.write_text(json.dumps(rows, indent=2) + "\n")
    print(f"Mobile source line coverage: {report} (generated FFI/tests excluded)")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("lane", choices=["prepare", "fast", "views", "unit", "transport", "all"], nargs="?", default="fast")
    parser.add_argument("--destination", help="xcodebuild destination, required for native lanes")
    parser.add_argument("--filter", help="Swift test regex for fast, XCTest identifier for native lanes")
    parser.add_argument("--coverage", action="store_true", help="collect coverage (slower; off by default)")
    args = parser.parse_args()
    if args.lane in ("views", "unit", "transport", "all") and not args.destination:
        parser.error("native lanes require --destination 'platform=iOS Simulator,id=YOUR_TEST_SIMULATOR'")
    if args.destination and "platform=iOS Simulator" not in [item.strip() for item in args.destination.split(",")]:
        parser.error("the native unit lane requires an iOS Simulator destination")
    if args.filter and args.lane in ("prepare", "all"):
        parser.error("--filter requires one test lane: fast, views or transport")
    if args.coverage and args.lane == "prepare":
        parser.error("--coverage requires a test lane")
    OUTPUT.mkdir(parents=True, exist_ok=True)
    timings = {}
    # Serialize this runner's preparation/build/test operations; do not overwrite
    # generated package artifacts while another invocation is testing them.
    with (OUTPUT / "runner.lock").open("w") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        try:
            fingerprint = core_fingerprint()
            shape_violations = ios_project_shape_violations()
            if shape_violations:
                raise RuntimeError("Invalid iOS test shape:\n" + "\n".join(shape_violations))
            if args.lane == "prepare":
                run("prepare", [str(ROOT / "ios/scripts/build-core.sh"), "--debug"], timings)
                if fingerprint != core_fingerprint():
                    raise RuntimeError("Rust sources changed during preparation; rerun prepare.")
                STAMP.write_text(json.dumps({"source": fingerprint, "artifacts": artifacts()}))
                return 0
            prepared = json.loads(STAMP.read_text()) if STAMP.exists() else {}
            if prepared.get("source") != fingerprint or prepared.get("artifacts") != artifacts():
                raise RuntimeError("Rust sources/bindings are unprepared or changed. Run: python3 ios/scripts/test.py prepare")
            if args.lane in ("fast", "unit", "all"):
                command = ["swift", "test", "--package-path", str(PACKAGE)]
                command += ["--enable-code-coverage" if args.coverage else "--disable-code-coverage"]
                if args.filter:
                    command += ["--filter", args.filter]
                run("fast", command, timings)
                if args.coverage:
                    summarize_package_coverage(timings)
            if args.lane in ("views", "unit", "transport", "all"):
                run("generate", ["xcodegen", "generate", "--spec", "ios/project.yml"], timings)
                lanes = (("views", "transport") if args.lane == "all" else
                         ("views",) if args.lane == "unit" else (args.lane,))
                for lane in lanes:
                    target = "MomentumViewTests" if lane == "views" else "MomentumTransportTests"
                    scheme = "MomentumFastTests" if lane == "views" else "MomentumTransportTests"
                    # Always use test, never test-without-building: edited views must rebuild.
                    result = OUTPUT / f"{lane}-{time.time_ns()}.xcresult"
                    command = ["xcodebuild", "-project", "ios/Momentum.xcodeproj",
                               "-scheme", scheme, "-configuration", "Debug",
                               "-destination", args.destination,
                               "-derivedDataPath", "ios/DerivedData/FastTests",
                               "-parallel-testing-enabled", "NO",
                               "-collect-test-diagnostics", "never",
                               "-enableCodeCoverage", "YES" if args.coverage else "NO",
                               "-resultBundlePath", str(result),
                               # Local simulator signing supplies the simulated application
                               # entitlement needed by Security.framework. No certificate,
                               # provisioning profile or personal team is required.
                               "CODE_SIGNING_ALLOWED=YES", "CODE_SIGN_IDENTITY=-", "DEVELOPMENT_TEAM=", "test"]
                    if args.filter:
                        command += [f"-only-testing:{target}/{args.filter}"]
                    run(lane, command, timings)
                    print(f"Native results{' and coverage' if args.coverage else ''}: {result}")
                    if args.coverage:
                        run(f"{lane}-coverage", ["xcrun", "xccov", "view", "--report", "--json", str(result)], timings)
            return 0
        except (RuntimeError, OSError, ValueError, subprocess.CalledProcessError) as error:
            print(f"Error: {error}", file=sys.stderr)
            return 1
        finally:
            (OUTPUT / "timings.json").write_text(json.dumps(timings, indent=2) + "\n")


if __name__ == "__main__":
    sys.exit(main())
