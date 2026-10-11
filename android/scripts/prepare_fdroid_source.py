#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Dan Hart
"""Prune reviewed unused binary inputs from an Android F-Droid vendor copy.

Never run this on a shared Cargo registry. Source, Cargo.lock, package checksums,
and remaining file checksums stay unchanged. Only removed-file checksum inventory
entries are removed. Unknown inputs stay for the scanner to review. See
android/fdroid/SOURCE_PREPARATION.md.
"""
import argparse
import hashlib
import json
import re
from pathlib import Path


POLICIES = {
    ("winapi-i686-pc-windows-gnu", "0.4.0"): {
        "build_sha256": "4344ce95f51197e1377429cd9ef7a8e3f97d39546776705857991f2f9416f545",
        "library_pattern": r"lib/libwinapi_[A-Za-z0-9_.-]+\.a"},
    ("winapi-x86_64-pc-windows-gnu", "0.4.0"): {
        "build_sha256": "b4ab421e1dc98159dd4440813cdaf7d9ea98278324bc6118d0f7cf378933f371",
        "library_pattern": r"lib/libwinapi_[A-Za-z0-9_.-]+\.a"},
}
for version, build_sha in [
    ("0.48.5", "63fb190082cf5ca1a64158013408353c9a4b9b88ac4a0181f8ffc7d81a846202"),
    ("0.52.6", "6d40bd2c0ed4cbea5126dfcd89d72f229c7d986540cbf0dc34acc1017f1de20f"),
]:
    architectures = ["aarch64_gnullvm", "aarch64_msvc", "i686_gnu", "i686_msvc",
                     "x86_64_gnu", "x86_64_gnullvm", "x86_64_msvc"]
    if version == "0.52.6":
        architectures.append("i686_gnullvm")
    for architecture in architectures:
        if version == "0.48.5":
            build_sha = {
                "aarch64_gnullvm": "63fb190082cf5ca1a64158013408353c9a4b9b88ac4a0181f8ffc7d81a846202",
                "aarch64_msvc": "cb87ced2a369513d4593ceec97e176dd38335501ada9f960cb72c454979335a2",
                "i686_gnu": "000703af30e0803750ab7d9a087728f109f058144368e8f53532ddcae317ab73",
                "i686_msvc": "d5df812ba7add22771644473db37d7de40c1e7479a30f81ae3ccb0d7be3fabe4",
                "x86_64_gnu": "d1d816121af1bba70471e982bf98fa9cd2d8cf695d15d2837f592458410ff597",
                "x86_64_gnullvm": "f909646de7a03d6998bf644e9aea2b732c3f0705e24b296f78ff323d4bf74137",
                "x86_64_msvc": "c801bf00de0978fd5252930993878c0b0eed65336c82c89289efaf9a9ad5ac22",
            }[architecture]
        library_version = "0.48.5" if version == "0.48.5" else "0.52.0"
        filename = (f"windows.{library_version}.lib" if architecture.endswith("msvc")
                    else f"libwindows.{library_version}.a")
        POLICIES[("windows_" + architecture, version)] = {
            "build_sha256": build_sha,
            "library_pattern": re.escape("lib/" + filename),
        }


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def table_value(table, name):
    match = re.search(r'^\s*' + re.escape(name) + r'\s*=\s*"([^"\n]+)"\s*$', table, re.M)
    if not match:
        raise ValueError(f"Missing {name} in Cargo metadata")
    return match.group(1)


def prepare(vendor, lock, policies=None, fixture_policies=None):
    policies = POLICIES if policies is None else policies
    if fixture_policies is None:
        fixture_policies = json.loads((Path(__file__).parents[1] / "fdroid/source-pruning.json").read_text())["packages"]
    vendor, lock = Path(vendor), Path(lock)
    if vendor.is_symlink() or not vendor.is_dir():
        raise ValueError("Expected a real, isolated versioned Cargo vendor directory")
    lock_bytes = lock.read_bytes()
    locked = {}
    for table in re.split(r"^\[\[package\]\]\s*$", lock_bytes.decode(), flags=re.M)[1:]:
        if re.search(r'^checksum\s*=', table, re.M):
            locked[(table_value(table, "name"), table_value(table, "version"))] = table_value(table, "checksum")
    pending, metadata, inspected_packages = [], [], {}

    def inspect_package(name, version):
        if (name, version) in inspected_packages:
            return inspected_packages[(name, version)]
        package = vendor / f"{name}-{version}"
        if package.is_symlink():
            raise ValueError(f"Symlinked package: {package.name}")
        if not package.exists():
            return None
        manifest = package / "Cargo.toml"
        if manifest.is_symlink():
            raise ValueError(f"Symlinked manifest: {package.name}")
        tables = re.split(r"^\[package\]\s*$", manifest.read_text(), flags=re.M)
        if len(tables) != 2:
            raise ValueError(f"Missing package table: {package.name}")
        package_table = re.split(r"^\[", tables[1], maxsplit=1, flags=re.M)[0]
        if (table_value(package_table, "name"), table_value(package_table, "version")) != (name, version):
            raise ValueError(f"Unexpected package identity: {package.name}")
        checksum_path = package / ".cargo-checksum.json"
        if checksum_path.is_symlink():
            raise ValueError(f"Symlinked checksum metadata: {package.name}")
        checksum_bytes = checksum_path.read_bytes()
        checksums = json.loads(checksum_bytes)
        if not locked.get((name, version)) or checksums["package"] != locked[(name, version)]:
            raise ValueError(f"Vendor checksum differs from Cargo.lock: {package.name}")
        checked_file(package, "Cargo.toml", checksums["files"].get("Cargo.toml"))
        metadata.append({"package": package.name, "crate_checksum": checksums["package"],
                         "original_checksum_metadata_sha256": hashlib.sha256(checksum_bytes).hexdigest()})
        inspected_packages[(name, version)] = package, checksums
        return package, checksums

    def checked_file(package, relative, expected):
        parts = Path(relative).parts
        if not parts or Path(relative).is_absolute() or ".." in parts:
            raise ValueError(f"Unsafe source path: {relative}")
        for index in range(1, len(parts) + 1):
            if package.joinpath(*parts[:index]).is_symlink():
                raise ValueError(f"Symlinked source input: {package.name}/{relative}")
        path = package / relative
        if not expected or not path.is_file() or sha256(path) != expected:
            raise ValueError(f"Changed source input: {package.name}/{relative}")
        return path
    # Preflight the complete list before removing anything. A partial deletion
    # after an unexpected input would make provenance unnecessarily ambiguous.
    for (name, version), policy in sorted(policies.items()):
        inspected = inspect_package(name, version)
        if inspected is None:
            continue
        package, checksums = inspected
        build_script = checked_file(package, "build.rs", checksums["files"].get("build.rs"))
        if sha256(build_script) != policy["build_sha256"]:
            raise ValueError(f"Unreviewed build script: {package.name}")
        library_dir = package / "lib"
        if library_dir.is_symlink():
            raise ValueError(f"Symlinked import directory: {package.name}/lib")
        for path in sorted(library_dir.glob("*")):
            relative = path.relative_to(package).as_posix()
            if not re.fullmatch(policy["library_pattern"], relative):
                continue
            checked_file(package, relative, checksums["files"].get(relative))
            pending.append((path, {"path": f"{package.name}/{relative}", "sha256": sha256(path),
                                   "reason": "Windows target import library; not used by Android or Linux host"}))
    for policy in fixture_policies:
        inspected = inspect_package(policy["name"], policy["version"])
        if inspected is None:
            continue
        package, checksums = inspected
        if checksums["package"] != policy["crate_checksum"]:
            raise ValueError(f"Unreviewed fixture crate: {package.name}")
        for relative, expected in policy["evidence"].items():
            if checksums["files"].get(relative) != expected:
                raise ValueError(f"Unreviewed target/test guard: {package.name}/{relative}")
            checked_file(package, relative, expected)
        for relative, expected in sorted(policy["files"].items()):
            path = package / relative
            if not path.exists() and not path.is_symlink() and relative not in checksums["files"]:
                continue  # Idempotent preparation; deleted paths no longer inventoried.
            if checksums["files"].get(relative) != expected:
                raise ValueError(f"Unreviewed fixture: {package.name}/{relative}")
            checked_file(package, relative, expected)
            pending.append((path, {"path": f"{package.name}/{relative}", "sha256": expected,
                                   "reason": policy["reason"]}))
    # Verify every original registry file, including code, licenses, and inputs
    # outside the pruning policies. Never replace trust in original file hashes
    # with freshly calculated hashes of potentially altered source.
    for checksum_path in sorted(vendor.glob("*/.cargo-checksum.json")):
        package = checksum_path.parent
        manifest = (package / "Cargo.toml").read_text()
        table = re.split(r"^\[package\]\s*$", manifest, flags=re.M)[1]
        table = re.split(r"^\[", table, maxsplit=1, flags=re.M)[0]
        inspected = inspect_package(table_value(table, "name"), table_value(table, "version"))
        if inspected is None or inspected[0] != package:
            raise ValueError(f"Expected a versioned vendor package: {package.name}")
        _, checksums = inspected
        actual = {path.relative_to(package).as_posix() for path in package.rglob("*")
                  if path.is_file() or path.is_symlink()}
        if actual != set(checksums["files"]) | {".cargo-checksum.json"}:
            raise ValueError(f"Missing or unknown files in vendor package: {package.name}")
        for relative, expected in checksums["files"].items():
            checked_file(package, relative, expected)

    changed_metadata = {}
    for path, entry in pending:
        path.unlink()
        package_name, relative = entry["path"].split("/", 1)
        checksum_path = vendor / package_name / ".cargo-checksum.json"
        if checksum_path not in changed_metadata:
            changed_metadata[checksum_path] = json.loads(checksum_path.read_text())
        del changed_metadata[checksum_path]["files"][relative]
    for checksum_path, checksums in changed_metadata.items():
        # Cargo verifies every listed file in a compiled directory-source crate,
        # even fixtures unused by the build. Retain its original package checksum
        # and every remaining hash; remove only explicitly verified deleted keys.
        temporary = checksum_path.with_suffix(".json.tmp")
        temporary.write_text(json.dumps(checksums, sort_keys=True) + "\n")
        temporary.replace(checksum_path)
    for item in metadata:
        item["prepared_checksum_metadata_sha256"] = sha256(vendor / item["package"] / ".cargo-checksum.json")
    return {"format": 1, "cargo_lock_sha256": hashlib.sha256(lock_bytes).hexdigest(),
            "packages": metadata, "removed": [entry for _, entry in pending]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--vendor", type=Path, required=True)
    parser.add_argument("--lock", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    if args.report.resolve().is_relative_to(args.vendor.resolve()):
        parser.error("Write the report outside the vendor source directory")
    report = prepare(args.vendor, args.lock)
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2) + "\n")
    print(f"Removed {len(report['removed'])} reviewed unused inputs; Cargo.lock/package/remaining-file checksums preserved")


if __name__ == "__main__":
    main()
