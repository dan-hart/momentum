#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Dan Hart
"""Source-built Android core and Kotlin bindings, isolated from Apple build outputs."""
import argparse
import fcntl
import hashlib
import io
import json
import os
import platform
import re
import shutil
import subprocess
import tarfile
import tempfile
import urllib.request
import zipfile
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[2]
ANDROID = ROOT / "android"
TOOLS = json.loads((ANDROID / "toolchain.json").read_text())
TARGET = ANDROID / "rust-target"
GENERATED = ANDROID / "app/build/generated"


def source_digest(root):
    digest = hashlib.sha256()
    for path in sorted(root.rglob("*"), key=lambda value: value.relative_to(root).as_posix()):
        relative = path.relative_to(root)
        if any(part in {".git", "target", "__pycache__", ".DS_Store"} for part in relative.parts):
            continue
        if path.is_symlink():
            raise ValueError(f"Unexpected symlink in dependency source: {relative}")
        if path.is_file():
            digest.update(relative.as_posix().encode() + b"\0" + hashlib.sha256(path.read_bytes()).digest())
    return digest.hexdigest()


def extract_source(archive, destination):
    # Validate every member before creating any files, including on Python 3.9.
    with tarfile.open(fileobj=io.BytesIO(archive), mode="r:gz") as source:
        members = []
        for member in source.getmembers():
            parts = PurePosixPath(member.name).parts
            if member.name.startswith("/") or ".." in parts or not (member.isfile() or member.isdir()):
                raise ValueError(f"Unsafe source archive member: {member.name}")
            if len(parts) > 1:
                members.append((member, Path(*parts[1:])))
        for member, relative in members:
            path = destination / relative
            if member.isdir():
                path.mkdir(parents=True, exist_ok=True)
            else:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(source.extractfile(member).read())
                path.chmod(member.mode & 0o777)


def verify_source():
    source = ROOT / "libresync-src"
    if not source.is_dir():
        raise ValueError("Missing LibreSync source. Run python3 android/scripts/build_core.py --prepare-source first.")
    if source_digest(source) != TOOLS["libresync"]["source_sha256"]:
        raise ValueError("LibreSync source differs from pinned v0.6.0. Preserve local edits and prepare a clean pinned checkout; no files were replaced.")


def prepare_source():
    destination = ROOT / "libresync-src"
    if destination.exists() or destination.is_symlink():
        verify_source()
        return
    pin = TOOLS["libresync"]
    archive = urllib.request.urlopen(pin["url"], timeout=60).read()
    if hashlib.sha256(archive).hexdigest() != pin["archive_sha256"]:
        raise ValueError("LibreSync archive checksum mismatch")
    with tempfile.TemporaryDirectory(prefix="libresync-", dir=TARGET) as directory:
        stage = Path(directory) / "source"
        extract_source(archive, stage)
        if source_digest(stage) != pin["source_sha256"]:
            raise ValueError("LibreSync extracted source checksum mismatch")
        stage.rename(destination)


def sdk_root():
    for key in ["ANDROID_HOME", "ANDROID_SDK_ROOT"]:
        if os.environ.get(key):
            return Path(os.environ[key])
    properties = ANDROID / "local.properties"
    if properties.exists():
        for line in properties.read_text().splitlines():
            if line.startswith("sdk.dir="):
                return Path(line.split("=", 1)[1].replace("\\:", ":").replace("\\\\", "\\"))
    conventional = Path.home() / ("Library/Android/sdk" if platform.system() == "Darwin" else "Android/Sdk")
    if conventional.is_dir():
        return conventional
    raise ValueError("Set ANDROID_HOME to an installed Android SDK")


def ndk_bin(ndk):
    host = "darwin-x86_64" if platform.system() == "Darwin" else "linux-x86_64"
    return ndk / "toolchains/llvm/prebuilt" / host / "bin"


def target_environment(ndk, triple, api):
    tools = ndk_bin(ndk)
    clang = tools / f"{triple}{api}-clang"
    key = triple.upper().replace("-", "_")
    underscore = triple.replace("-", "_")
    return {
        f"CARGO_TARGET_{key}_LINKER": str(clang),
        f"CARGO_TARGET_{key}_RUSTFLAGS": "-C link-arg=-Wl,-z,max-page-size=16384 -C link-arg=-Wl,-z,common-page-size=16384",
        f"CC_{underscore}": str(clang),
        f"AR_{underscore}": str(tools / "llvm-ar"),
        f"CFLAGS_{underscore}": "-fPIC",
    }


def check_load_alignment(output):
    aligns = re.findall(r"LOAD[^\n]*align 2\*\*(\d+)", output)
    if not aligns or any(int(value) < 14 for value in aligns):
        raise ValueError("Native ELF LOAD segments must be aligned to at least 16 KiB")


def check_relro_alignment(output):
    for line in output.splitlines():
        fields = line.split()
        if fields and fields[0] == "GNU_RELRO":
            if len(fields) < 6 or (int(fields[2], 16) + int(fields[5], 16)) % 16384:
                raise ValueError("Native ELF RELRO must end at a 16 KiB page boundary")


def run(args, env):
    subprocess.run(args, cwd=ROOT, env=env, check=True)


def check_apk(apk):
    sdk = sdk_root()
    tools = ndk_bin(sdk / "ndk" / TOOLS["ndk"])
    subprocess.run([str(sdk / "build-tools" / TOOLS["build_tools"] / "zipalign"), "-c", "-P", "16", "4", str(apk)], check=True)
    with zipfile.ZipFile(apk) as archive, tempfile.TemporaryDirectory(prefix="apk-libs-", dir=TARGET) as directory:
        libraries = [name for name in archive.namelist() if name.startswith("lib/") and name.endswith(".so")]
        if not libraries:
            raise ValueError("APK contains no native libraries")
        for index, name in enumerate(libraries):
            library = Path(directory) / f"{index}.so"
            library.write_bytes(archive.read(name))
            check_load_alignment(subprocess.check_output([str(tools / "llvm-objdump"), "-p", str(library)], text=True))
            check_relro_alignment(subprocess.check_output([str(tools / "llvm-readelf"), "-Wl", str(library)], text=True))
            print(f"16 KiB LOAD/RELRO/ZIP alignment passed: {name}")


def build(profile, abis, bindings_only):
    verify_source()
    rustup = shutil.which("rustup") or str(Path.home() / ".cargo/bin/rustup")
    cargo = [rustup, "run", TOOLS["rust"], "cargo"]
    env = dict(os.environ, CARGO_TARGET_DIR=str(TARGET))
    for key in ["SDKROOT", "LIBRARY_PATH", "IPHONEOS_DEPLOYMENT_TARGET", "MACOSX_DEPLOYMENT_TARGET", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS"]:
        env.pop(key, None)
    env["PATH"] = str(Path(rustup).parent) + os.pathsep + env.get("PATH", "")
    run([*cargo, "build", "--locked", "-p", "momentum-ffi", "-p", "uniffi-bindgen"], env)
    extension = "dylib" if platform.system() == "Darwin" else "so"
    with tempfile.TemporaryDirectory(prefix="bindings-", dir=TARGET) as directory:
        run([str(TARGET / "debug/uniffi-bindgen"), "generate", "--library", str(TARGET / f"debug/libmomentum_ffi.{extension}"),
             "--language", "kotlin", "--config", str(ANDROID / "uniffi.toml"), "--out-dir", directory, "--no-format"], env)
        output = GENERATED / "uniffi"
        if output.exists():
            shutil.rmtree(output)
        shutil.copytree(directory, output)
    if bindings_only:
        return
    ndk = sdk_root() / "ndk" / TOOLS["ndk"]
    if not ndk_bin(ndk).is_dir():
        raise ValueError(f"Install Android NDK {TOOLS['ndk']}")
    for abi in abis:
        triple = TOOLS["abis"][abi]
        target_env = dict(env, **target_environment(ndk, triple, TOOLS["min_sdk"]))
        flags = ["--release"] if profile == "release" else []
        run([*cargo, "build", "--locked", "-p", "momentum-ffi", "--target", triple, *flags], target_env)
        library = TARGET / triple / profile / "libmomentum_ffi.so"
        result = subprocess.check_output([str(ndk_bin(ndk) / "llvm-objdump"), "-p", str(library)], text=True)
        check_load_alignment(result)
        result = subprocess.check_output([str(ndk_bin(ndk) / "llvm-readelf"), "-Wl", str(library)], text=True)
        check_relro_alignment(result)
        output = GENERATED / "jniLibs" / abi
        output.mkdir(parents=True, exist_ok=True)
        shutil.copy2(library, output / library.name)
        print(f"Built {abi}: {output / library.name} (16 KiB LOAD and RELRO alignment)", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prepare-source", action="store_true")
    parser.add_argument("--verify-source", action="store_true")
    parser.add_argument("--bindings-only", action="store_true")
    parser.add_argument("--check-apk", type=Path)
    parser.add_argument("--profile", choices=["debug", "release"], default="release")
    parser.add_argument("--abis", default=",".join(TOOLS["abis"]))
    args = parser.parse_args()
    abis = args.abis.split(",")
    if not abis or any(abi not in TOOLS["abis"] for abi in abis):
        parser.error("Supported ABIs: arm64-v8a,x86_64")
    TARGET.mkdir(parents=True, exist_ok=True)
    try:
        with (TARGET / "android-build.lock").open("w") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            if args.check_apk:
                check_apk(args.check_apk.resolve())
            elif args.prepare_source:
                prepare_source()
            elif args.verify_source:
                verify_source()
            else:
                build(args.profile, abis, args.bindings_only)
    except (ValueError, subprocess.CalledProcessError, OSError) as error:
        parser.exit(1, f"Android source build failed: {error}\n")


if __name__ == "__main__":
    main()
