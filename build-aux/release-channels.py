#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Dan Hart
"""Required install-channel credentials, publication and signed version readback."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
APP_ID = "io.github.dan_hart.Momentum"
ARCHITECTURES = ("x86_64", "aarch64")
CREDENTIALS = {
    "homebrew": ("HOMEBREW_TAP_SSH_KEY",),
    "flatpak": ("FLATPAK_GPG_PRIVATE_KEY", "FLATPAK_GPG_KEY_ID", "FLATPAK_GPG_FINGERPRINT"),
}
VERSION_RE = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\Z")


def checked(*args):
    return subprocess.run([str(a) for a in args], check=True, capture_output=True, text=True)


def version_tuple(value):
    if not VERSION_RE.fullmatch(value):
        raise ValueError("Expected a release version in X.Y.Z form")
    return tuple(int(part) for part in value.split("."))


def credentials(channel):
    names = sum(CREDENTIALS.values(), ()) if channel == "all" else CREDENTIALS[channel]
    missing = [name for name in names if not os.environ.get(name, "").strip()]
    if missing:
        raise ValueError("Required release configuration is missing: " + ", ".join(missing))
    if channel != "homebrew" and not re.fullmatch(r"[0-9A-Fa-f]{40}", os.environ["FLATPAK_GPG_FINGERPRINT"]):
        raise ValueError("FLATPAK_GPG_FINGERPRINT must be the full trusted public fingerprint")
    print("Required release configuration is present")


def git(repo, *args):
    return checked("git", "-C", repo, *args)


def publish_git(repo, branch, message, paths=None):
    git(repo, "check-ref-format", "--branch", branch)
    # Authenticate the receive operation even when a rerun has no new content.
    git(repo, "push", "--dry-run", "origin", f"HEAD:{branch}")
    if paths:
        git(repo, "add", "--", *paths)
    else:
        git(repo, "add", "--all")
    difference = subprocess.run(["git", "-C", str(repo), "diff", "--cached", "--quiet"])
    if difference.returncode == 0:
        # A previous failed push may have left a valid local commit ahead of origin.
        git(repo, "push", "origin", f"HEAD:{branch}")
        print("Install channel is already current")
        return
    if difference.returncode != 1:
        difference.check_returncode()
    git(repo, "commit", "-m", message)
    git(repo, "push", "origin", f"HEAD:{branch}")
    print("Published install channel")


def validate_assets(assets, version):
    manifest = assets / "SHA256SUMS.txt"
    if not manifest.is_file():
        raise ValueError("Missing SHA256SUMS.txt")
    digests = {}
    for line in manifest.read_text().splitlines():
        match = re.fullmatch(r"([0-9a-f]{64})  (\S+)", line)
        if not match or match[2] in digests:
            raise ValueError("Invalid or duplicate checksum entry in SHA256SUMS.txt")
        digests[match[2]] = match[1]
    names = (f"Momentum-v{version}-macos.zip", f"mo-v{version}-macos-universal.tar.gz",
             f"mo-v{version}-linux-x86_64.tar.gz", f"mo-v{version}-linux-aarch64.tar.gz")
    for name in names:
        path = assets / name
        if name not in digests or not path.is_file() or path.is_symlink():
            raise ValueError(f"Missing release asset or checksum: {name}")
        digest = hashlib.sha256()
        with path.open("rb") as file:
            for chunk in iter(lambda: file.read(1024 * 1024), b""):
                digest.update(chunk)
        if digest.hexdigest() != digests[name]:
            raise ValueError(f"Release asset checksum mismatch: {name}")


def homebrew(tap, assets, version, notarized):
    incoming = version_tuple(version)
    if git(tap, "status", "--porcelain").stdout.strip():
        raise ValueError("Refusing to modify a dirty Homebrew tap checkout")
    paths = ("Formula/momentum-cli.rb", "Casks/momentum.rb")
    for name in paths:
        path = tap / name
        if not path.exists():
            continue
        text = path.read_text()
        versions = re.findall(r'version\s+"([0-9]+\.[0-9]+\.[0-9]+)"', text)
        versions += re.findall(r'releases/download/v([0-9]+\.[0-9]+\.[0-9]+)/', text)
        if not versions:
            raise ValueError(f"Cannot determine existing Homebrew version: {name}")
        if any(version_tuple(current) > incoming for current in versions):
            raise ValueError("Refusing an older release that would downgrade Homebrew")
    validate_assets(assets, version)
    args = [sys.executable, ROOT / "build-aux/release.py", "homebrew", "--version", version,
            "--assets", assets, "--out", tap]
    if notarized:
        args.append("--notarized")
    checked(*args)
    publish_git(tap, "main", f"momentum {version}", paths)


def flatpak_ref(arch):
    return f"app/{APP_ID}/{arch}/stable"


def stored_version(repo, ref):
    xml = checked("ostree", f"--repo={repo}", "cat", ref,
                  f"/files/share/metainfo/{APP_ID}.metainfo.xml").stdout
    release = ET.fromstring(xml).find("releases/release")
    if release is None or not release.get("version"):
        raise ValueError("Hosted app metadata has no release version")
    version = release.get("version")
    version_tuple(version)
    return version


def check_flatpak(repo, version, arch=None):
    incoming = version_tuple(version)
    if arch:
        actual = stored_version(repo, flatpak_ref(arch))
        if actual != version:
            raise ValueError(f"Exported Flatpak version mismatch: {arch} has {actual}, expected {version}")
    else:
        refs = set(checked("ostree", f"--repo={repo}", "refs").stdout.splitlines())
        for architecture in ARCHITECTURES:
            ref = flatpak_ref(architecture)
            if ref in refs and version_tuple(stored_version(repo, ref)) > incoming:
                raise ValueError("Refusing an older release that would downgrade hosted Flatpak")
    print("Hosted Flatpak version check passed")


def primary_fingerprints(records):
    primary = set()
    next_primary = False
    for record in records:
        fields = record.split(":")
        if fields[0] in ("pub", "sec"):
            next_primary = True
        elif fields[0] in ("sub", "ssb"):
            next_primary = False
        elif fields[0] == "fpr" and next_primary:
            primary.add(fields[9].upper())
            next_primary = False
    return primary


def check_signing_key(key_id, fingerprint):
    if not re.fullmatch(r"[0-9A-Fa-f]{40}", fingerprint):
        raise ValueError("Expected the full trusted public signing fingerprint")
    records = checked("gpg", "--batch", "--with-colons", "--list-secret-keys", "--", key_id).stdout.splitlines()
    if primary_fingerprints(records) != {fingerprint.upper()}:
        raise ValueError("Imported signing identity does not match the trusted release key")
    print("Imported signing identity matches the trusted release key")


def verify_hosted(repo, remote, version, fingerprint, attempts, delay):
    version_tuple(version)
    if not re.fullmatch(r"[0-9A-Fa-f]{40}", fingerprint):
        raise ValueError("Expected the full trusted public signing fingerprint")
    if not re.fullmatch(r"[A-Za-z0-9_-]+", remote):
        raise ValueError("Invalid hosted remote name")
    if not 1 <= attempts <= 30 or not 0 <= delay <= 60:
        raise ValueError("Invalid bounded hosted verification retry settings")
    keyring = repo / f"{remote}.trustedkeys.gpg"
    records = checked("gpg", "--show-keys", "--with-colons", keyring).stdout.splitlines()
    if primary_fingerprints(records) != {fingerprint.upper()}:
        raise ValueError("Hosted signing fingerprint does not match the trusted release identity")
    for attempt in range(attempts):
        try:
            result = {}
            for arch in ARCHITECTURES:
                ref = flatpak_ref(arch)
                checked("ostree", f"--repo={repo}", "pull", "--subpath=/files/share/metainfo",
                        "--depth=0", "--http-header=Cache-Control=no-cache", remote, ref)
                qualified = f"{remote}:{ref}"
                actual = stored_version(repo, qualified)
                if actual != version:
                    raise ValueError(f"Hosted Flatpak version mismatch: {arch} has {actual}, expected {version}")
                signature = checked("ostree", f"--repo={repo}", "show",
                                    f"--gpg-verify-remote={remote}", qualified).stdout
                if "Good signature" not in signature:
                    raise ValueError(f"Hosted Flatpak signature verification failed: {arch}")
                commit = checked("ostree", f"--repo={repo}", "rev-parse", qualified).stdout.strip()
                result[arch] = {"version": actual, "commit": commit, "trusted_signature": True}
            print(json.dumps(result, indent=2))
            return
        except (ValueError, subprocess.CalledProcessError):
            if attempt + 1 == attempts:
                raise
            print(f"Waiting for hosted release propagation ({attempt + 1}/{attempts})", flush=True)
            time.sleep(delay)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    p = sub.add_parser("credentials")
    p.add_argument("--channel", choices=("all", "homebrew", "flatpak"), default="all")
    p = sub.add_parser("version")
    p.add_argument("--version", default="")
    p = sub.add_parser("homebrew")
    p.add_argument("--tap", type=Path, required=True)
    p.add_argument("--assets", type=Path, required=True)
    p.add_argument("--version", required=True)
    p.add_argument("--notarized", action="store_true")
    p = sub.add_parser("publish-git")
    p.add_argument("--repo", type=Path, required=True)
    p.add_argument("--branch", required=True)
    p.add_argument("--message", required=True)
    p.add_argument("--path", action="append", dest="paths")
    p = sub.add_parser("check-flatpak")
    p.add_argument("--repo", type=Path, required=True)
    p.add_argument("--version", required=True)
    p.add_argument("--arch", choices=ARCHITECTURES)
    p = sub.add_parser("check-signing-key")
    p.add_argument("--key-id", required=True)
    p.add_argument("--fingerprint", required=True)
    p = sub.add_parser("verify-hosted")
    p.add_argument("--repo", type=Path, required=True)
    p.add_argument("--remote", default="momentum")
    p.add_argument("--version", required=True)
    p.add_argument("--fingerprint", required=True)
    p.add_argument("--attempts", type=int, default=6)
    p.add_argument("--delay", type=int, default=20)
    args = parser.parse_args()
    if args.command == "credentials":
        credentials(args.channel)
    elif args.command == "version":
        version = args.version or checked(sys.executable, ROOT / "build-aux/release.py", "version").stdout.strip()
        version_tuple(version)
        print(version)
    elif args.command == "homebrew":
        homebrew(args.tap, args.assets, args.version, args.notarized)
    elif args.command == "publish-git":
        publish_git(args.repo, args.branch, args.message, args.paths)
    elif args.command == "check-flatpak":
        check_flatpak(args.repo, args.version, args.arch)
    elif args.command == "check-signing-key":
        check_signing_key(args.key_id, args.fingerprint)
    elif args.command == "verify-hosted":
        verify_hosted(args.repo, args.remote, args.version, args.fingerprint, args.attempts, args.delay)


if __name__ == "__main__":
    # CLI output from GPG/OSTree must be deterministic for signature verification.
    os.environ["LC_ALL"] = "C"
    try:
        main()
    except (ValueError, OSError, ET.ParseError, subprocess.CalledProcessError) as error:
        if isinstance(error, subprocess.CalledProcessError) and error.stderr:
            print(error.stderr.strip(), file=sys.stderr)
        else:
            print(str(error), file=sys.stderr)
        sys.exit(1)
