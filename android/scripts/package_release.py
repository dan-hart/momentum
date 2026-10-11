#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Verify Android release metadata and package matching, committed source.

Builds a fresh committed-source snapshot before packaging; existing outputs are
never release inputs. This command never uploads artifacts or creates a distribution key.
"""
import argparse
import gzip
import hashlib
import io
import json
import os
import re
import shutil
import subprocess
import tarfile
import tempfile
import xml.etree.ElementTree as ET
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
APP_ID = "com.codedbydan.momentum"
ANDROID_XML = "{http://schemas.android.com/apk/res/android}"


def version_code(version):
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        raise ValueError("Android releases require a stable MAJOR.MINOR.PATCH version")
    major, minor, patch = map(int, version.split("."))
    if major > 2099 or minor > 999 or patch > 999:
        raise ValueError("Version exceeds Android's monotonic versionCode bounds")
    result = major * 1_000_000 + minor * 1000 + patch
    if result == 0:
        raise ValueError("Android versionCode must be positive")
    return result


def verify_badging(badging, version):
    expected = {"name": APP_ID, "versionName": version, "versionCode": str(version_code(version))}
    package = next((line for line in badging.splitlines() if line.startswith("package:")), "")
    for key, value in expected.items():
        if f"{key}='{value}'" not in package:
            raise ValueError(f"APK {key} does not match the release")
    if "sdkVersion:'26'" not in badging or "targetSdkVersion:'37'" not in badging:
        raise ValueError("APK SDK metadata does not match the Android baseline")
    if "application-debuggable" in badging:
        raise ValueError("Debuggable APK cannot be packaged as a release")


def verify_bundle_manifest(manifest, version):
    root = ET.fromstring(manifest)
    sdk, application = root.find("uses-sdk"), root.find("application")
    expected = {"package": APP_ID, ANDROID_XML + "versionName": version,
                ANDROID_XML + "versionCode": str(version_code(version))}
    if any(root.get(key) != value for key, value in expected.items()):
        raise ValueError("AAB application/version metadata does not match the release")
    if sdk is None or sdk.get(ANDROID_XML + "minSdkVersion") != "26" or sdk.get(ANDROID_XML + "targetSdkVersion") != "37":
        raise ValueError("AAB SDK metadata does not match the Android baseline")
    if application is None or application.get(ANDROID_XML + "debuggable", "false") != "false":
        raise ValueError("Debuggable AAB cannot be packaged as a release")


def source_archive(root, dependency, vendor, destination, prefix, cargo_config=None, revision="HEAD"):
    root = Path(root)
    if subprocess.run(["git", "diff", "--quiet", "HEAD", "--"], cwd=root).returncode:
        raise ValueError("Commit tracked changes before packaging their corresponding source")
    original = subprocess.check_output(["git", "archive", "--format=tar", f"--prefix={prefix}/", revision], cwd=root)
    timestamp = int(subprocess.check_output(["git", "show", "-s", "--format=%ct", revision], cwd=root))
    with Path(destination).open("wb") as raw, gzip.GzipFile(filename="", fileobj=raw, mode="wb", mtime=0) as compressed:
        with tarfile.open(fileobj=compressed, mode="w") as output, tarfile.open(fileobj=io.BytesIO(original)) as source:
            for member in source:
                if cargo_config is not None and member.name == f"{prefix}/.cargo/config.toml":
                    continue
                output.addfile(member, source.extractfile(member) if member.isfile() else None)
            for folder, name in ((Path(dependency), "libresync-src"), (Path(vendor), "vendor")):
                if folder.is_symlink() or not folder.is_dir():
                    raise ValueError("Source archive requires real dependency source directories")
                for path in sorted(folder.rglob("*")):
                    relative = path.relative_to(folder)
                    if any(part in {".git", "target", "__pycache__", ".DS_Store"} for part in relative.parts):
                        continue
                    if path.is_symlink():
                        raise ValueError(f"Unexpected symlink in {name} source")
                    if path.is_file():
                        member = tarfile.TarInfo(f"{prefix}/{name}/{relative.as_posix()}")
                        member.size = path.stat().st_size
                        member.mode = 0o755 if path.stat().st_mode & 0o111 else 0o644
                        member.mtime = timestamp
                        with path.open("rb") as content:
                            output.addfile(member, content)
            if cargo_config is not None:
                data = cargo_config.encode()
                member = tarfile.TarInfo(f"{prefix}/.cargo/config.toml")
                member.size, member.mode, member.mtime = len(data), 0o644, timestamp
                output.addfile(member, io.BytesIO(data))


def sha256(path):
    with Path(path).open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest() if hasattr(hashlib, "file_digest") else hashlib.sha256(source.read()).hexdigest()


def committed_snapshot(root, destination):
    """Exclude all local/untracked inputs and bind this build to one Git commit."""
    root, destination = Path(root), Path(destination)
    if subprocess.run(["git", "diff", "--quiet", "HEAD", "--"], cwd=root).returncode:
        raise ValueError("Commit tracked changes before building their release artifacts")
    commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
    content = subprocess.check_output(["git", "archive", "--format=tar", commit], cwd=root)
    with tarfile.open(fileobj=io.BytesIO(content)) as archive:
        for member in archive:
            path = Path(member.name)
            if path.is_absolute() or ".." in path.parts or not (member.isfile() or member.isdir()):
                raise ValueError("Release source must contain only ordinary committed files/directories")
            output = destination / path
            if member.isdir():
                output.mkdir(parents=True, exist_ok=True)
            else:
                output.parent.mkdir(parents=True, exist_ok=True)
                output.write_bytes(archive.extractfile(member).read())
                output.chmod(member.mode & 0o777)
    return commit


def verify_apk_certificate(output, expected):
    fingerprints = re.findall(r"^Signer #\d+ certificate SHA-256 digest: ([0-9a-fA-F]+)$", output, re.MULTILINE)
    if [value.lower() for value in fingerprints] != [expected.lower()]:
        raise ValueError("APK signer differs from the configured distribution certificate")


def signing_certificate(destination):
    subprocess.run(["keytool", "-exportcert", "-keystore", os.environ["MOMENTUM_KEYSTORE"],
                    "-alias", os.environ["MOMENTUM_KEY_ALIAS"], "-storepass:env", "MOMENTUM_KEYSTORE_PASSWORD",
                    "-file", str(destination)], check=True, capture_output=True)
    return sha256(destination)


def build_snapshot(root, stage, dependency):
    commit = committed_snapshot(root, stage)
    shutil.copytree(dependency, stage / "libresync-src",
                    ignore=shutil.ignore_patterns(".git", "target", "__pycache__", ".DS_Store"))
    # No original app/build, rust-target, local.properties or untracked source is
    # present. Public dependency caches may be reused by the pinned build tools.
    subprocess.run([str(stage / "android/gradlew"), "--no-daemon", ":app:testDebugUnitTest",
                    ":app:assembleRelease", ":app:bundleRelease", ":app:lintRelease"],
                   cwd=stage / "android", check=True)
    return commit


def verify_bundle_apk(bundle, bundletool, stage, check_apk):
    """Check the APK generated from the actual packaged AAB, with a disposable key."""
    key = stage / "validation.jks"
    subprocess.run(["keytool", "-genkeypair", "-keystore", str(key), "-storepass", "android",
                    "-keypass", "android", "-alias", "validation", "-dname", "CN=Temporary Bundle Validation",
                    "-keyalg", "RSA", "-keysize", "2048", "-validity", "1", "-noprompt"], check=True)
    output = stage / "validation.apks"
    subprocess.run(["java", "-jar", str(bundletool), "build-apks", f"--bundle={bundle}",
                    f"--output={output}", "--mode=universal", f"--ks={key}", "--ks-key-alias=validation",
                    "--ks-pass=pass:android", "--key-pass=pass:android"], check=True)
    apk = stage / "validation.apk"
    with zipfile.ZipFile(output) as archive:
        apk.write_bytes(archive.read("universal.apk"))
    check_apk(apk)


def main():
    import build_core
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--bundletool", type=Path, required=True)
    args = parser.parse_args()
    build_core.verify_source()
    pin = build_core.TOOLS
    if sha256(args.bundletool) != pin["bundletool"]["sha256"]:
        raise ValueError("bundletool checksum mismatch")
    if args.out.exists() and any(args.out.iterdir()):
        raise ValueError("Release output directory must be empty; preserve prior artifacts in a separate directory")
    with tempfile.TemporaryDirectory(prefix="momentum-android-release-") as temporary:
        stage = Path(temporary) / "source"
        commit = build_snapshot(ROOT, stage, ROOT / "libresync-src")
        package(args, build_core, pin, stage, commit)


def package(args, build_core, pin, stage, commit):
    version = subprocess.check_output(["python3", "build-aux/release.py", "version"], cwd=stage, text=True).strip()
    version_code(version)
    signed = bool(os.environ.get("MOMENTUM_KEYSTORE"))
    apk = stage / "android/app/build/outputs/apk/release" / ("app-release.apk" if signed else "app-release-unsigned.apk")
    bundle = stage / "android/app/build/outputs/bundle/release/app-release.aab"
    tools = build_core.sdk_root() / "build-tools" / pin["build_tools"]
    verify_badging(subprocess.check_output([str(tools / "aapt2"), "dump", "badging", str(apk)], text=True), version)
    signature = subprocess.run([str(tools / "apksigner"), "verify", "--print-certs", str(apk)], capture_output=True, text=True)
    if (signature.returncode == 0) != signed:
        raise ValueError("APK signature does not match the declared signing configuration")
    if signed and "CN=Android Debug" in signature.stdout:
        raise ValueError("The Android debug key cannot sign official distribution artifacts")
    certificate = stage.parent / "distribution.der"
    fingerprint = signing_certificate(certificate) if signed else None
    if signed:
        verify_apk_certificate(signature.stdout, fingerprint)
    build_core.check_apk(apk)
    bundle_command = ["java", "-jar", str(args.bundletool.resolve())]
    subprocess.run([*bundle_command, "validate", f"--bundle={bundle}"], check=True)
    config = subprocess.check_output([*bundle_command, "dump", "config", f"--bundle={bundle}"], text=True)
    if "PAGE_ALIGNMENT_16K" not in config:
        raise ValueError("AAB must request 16 KiB APK alignment")
    verify_bundle_manifest(subprocess.check_output([*bundle_command, "dump", "manifest", f"--bundle={bundle}", "--module=base"], text=True), version)
    with zipfile.ZipFile(apk) as archive, zipfile.ZipFile(bundle) as bundle_archive:
        notices = archive.read("assets/THIRD_PARTY_NOTICES.txt")
        if notices != bundle_archive.read("base/assets/THIRD_PARTY_NOTICES.txt"):
            raise ValueError("APK/AAB dependency notices differ")
        for required in [b"LibreSync/LICENSE", b"JNA-libffi-LICENSE.txt", b"Rust-1.99.0-COPYRIGHT-library.html"]:
            if required not in notices:
                raise ValueError("Release artifact is missing required dependency notices")
        bundle_signed = any(re.fullmatch(r"META-INF/[^/]+\.(RSA|DSA|EC)", name) for name in bundle_archive.namelist())
    if bundle_signed != signed:
        raise ValueError("AAB signature does not match the declared signing configuration")
    if signed:
        # JarFile verifies every byte while reading; every payload entry must have
        # exactly the configured certificate, including entries appended later.
        subprocess.run(["java", str(stage / "android/scripts/VerifyBundleSignature.java"),
                        str(bundle), str(certificate)], check=True)
    verify_bundle_apk(bundle, args.bundletool.resolve(), stage.parent, build_core.check_apk)
    if subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip() != commit:
        raise ValueError("Committed source changed during the release build")
    if subprocess.run(["git", "diff", "--quiet", "HEAD", "--"], cwd=ROOT).returncode:
        raise ValueError("Tracked source changed during the release build")
    args.out.mkdir(parents=True, exist_ok=True)
    suffix = "" if signed else "-unsigned"
    prefix = f"Momentum-v{version}-android"
    shutil.copyfile(apk, args.out / f"{prefix}{suffix}.apk")
    shutil.copyfile(bundle, args.out / f"{prefix}{suffix}.aab")
    (args.out / f"{prefix}-notices.txt").write_bytes(notices)
    mapping = stage / "android/app/build/outputs/mapping/release/mapping.txt"
    shutil.copyfile(mapping, args.out / f"{prefix}-mapping.txt")
    with tempfile.TemporaryDirectory(prefix="momentum-release-source-") as temporary:
        vendor = Path(temporary) / "vendor"
        config = subprocess.check_output(["rustup", "run", pin["rust"], "cargo", "vendor", "--locked", "--versioned-dirs", str(vendor)], cwd=stage, text=True)
        config = re.sub(r'directory = "[^"\n]+"', 'directory = "vendor"', config)
        source_archive(ROOT, stage / "libresync-src", vendor, args.out / f"{prefix}-source.tar.gz", f"momentum-v{version}", config, commit)
    readme = f"""Momentum {version} Android distribution artifacts
Application ID: {APP_ID}; versionCode: {version_code(version)}; API 26+; arm64-v8a / x86_64.
Signing: {'configured distribution key' if signed else 'UNSIGNED: the APK cannot be installed until signed; the AAB is build input, not an installable app'}.
No Google Play or F-Droid upload is performed by this workflow.
Keep the same app signing identity for updates. F-Droid normally signs its own builds.
The source archive includes this exact committed Momentum revision, pinned LibreSync,
original locked Rust crate sources, native Graphics Path source, licenses and build scripts.
Cargo can use the included vendor directory offline; Android SDK/NDK, Rust toolchain and
public Maven/Gradle dependencies are still required as documented in android/README.md.
F-Droid admission, independent reproducibility and physical-device acceptance remain separate gates.
R8 mapping is included for diagnosing crashes from this build.
"""
    (args.out / f"{prefix}-distribution.txt").write_text(readme)
    provenance = {"version": version, "version_code": version_code(version), "application_id": APP_ID,
                  "commit": commit, "build_source": "fresh git archive of the recorded commit",
                  "signed": signed, "signing_certificate_sha256": fingerprint, "toolchain": pin,
                  "artifacts": {path.name: sha256(path) for path in sorted(args.out.iterdir()) if path.is_file()}}
    (args.out / f"{prefix}-provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")
    print(f"Verified and packaged Android {version} ({version_code(version)}), {'signed' if signed else 'unsigned'}")


if __name__ == "__main__":
    main()
