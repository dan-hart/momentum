#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Build unchanged pinned AndroidX Graphics Path JNI; replace only upstream JNI."""
import argparse
import fcntl
import hashlib
import importlib.util
import json
import os
import subprocess
import tempfile
import zipfile
from pathlib import Path

ANDROID = Path(__file__).resolve().parents[1]
ROOT = ANDROID.parent
TOOLS = json.loads((ANDROID / "toolchain.json").read_text())
PIN = TOOLS["graphics_path"]
SOURCE = ANDROID / "vendor/graphics-path"
OUTPUT = ANDROID / "app/build/generated/graphics-path/jni"
LIBRARY = "libandroidx.graphics.path.so"
core_spec = importlib.util.spec_from_file_location("momentum_android_build_core", ANDROID / "scripts/build_core.py")
core = importlib.util.module_from_spec(core_spec)
core_spec.loader.exec_module(core)


def verify_files(root, files):
    if root.is_symlink() or not root.is_dir():
        raise ValueError("Graphics Path source must be a real directory")
    entries = list(root.rglob("*"))
    if any(path.is_symlink() for path in entries):
        raise ValueError("Symlinked Graphics Path input")
    actual = {path.relative_to(root).as_posix() for path in entries if path.is_file()}
    if actual != set(files):
        raise ValueError("Graphics Path source inventory differs from the reviewed pin")
    for relative, pin in files.items():
        if hashlib.sha256((root / relative).read_bytes()).hexdigest() != pin["sha256"]:
            raise ValueError(f"Changed Graphics Path input: {relative}")


def verify_source():
    verify_files(SOURCE, PIN["files"])


def compile_command(ndk, abi, output):
    triple = TOOLS["abis"][abi]
    cpp = SOURCE / "src/main/cpp"
    # Match upstream build.gradle/CMakeLists compiler options and export map.
    # Add common-page-size as required for RELRO as well as LOAD alignment.
    return [str(core.ndk_bin(ndk) / f"{triple}{TOOLS['min_sdk']}-clang++"),
            "-shared", "-fPIC", "-O2", "-DNDEBUG", "-g0", "-std=c++17",
            "-fno-exceptions", "-fno-unwind-tables", "-fno-asynchronous-unwind-tables", "-fno-rtti",
            "-ffast-math", "-ffp-contract=fast", "-fvisibility-inlines-hidden", "-fvisibility=hidden",
            "-fomit-frame-pointer", "-ffunction-sections", "-fdata-sections", "-nostdlib++",
            f"-ffile-prefix-map={SOURCE}=androidx/graphics-path", "-Wl,--hash-style=both",
            "-Wl,--gc-sections", "-Wl,-Bsymbolic-functions", "-Wl,--no-undefined",
            "-Wl,--build-id=sha1", "-Wl,-z,relro,-z,now", "-Wl,-z,max-page-size=16384",
            "-Wl,-z,common-page-size=16384", f"-Wl,--version-script={cpp / 'libandroidx.graphics.path.map'}",
            f"-Wl,-soname,{LIBRARY}", str(cpp / "Conic.cpp"), str(cpp / "PathIterator.cpp"),
            str(cpp / "pathway.cpp"), "-lm", "-o", str(output)]


def check_library(path, ndk):
    core.check_load_alignment(subprocess.check_output([core.ndk_bin(ndk) / "llvm-objdump", "-p", path], text=True))
    headers = subprocess.check_output([core.ndk_bin(ndk) / "llvm-readelf", "-Wl", path], text=True)
    if "GNU_RELRO" not in headers:
        raise ValueError("Graphics Path must retain RELRO protection")
    core.check_relro_alignment(headers)
    exports = subprocess.check_output([core.ndk_bin(ndk) / "llvm-nm", "-D", "--defined-only", path], text=True)
    symbols = [line.split()[-1].split("@")[0] for line in exports.splitlines() if line.split()]
    if symbols != ["JNI_OnLoad"]:
        raise ValueError(f"Unexpected Graphics Path public exports: {symbols}")


def build(output=OUTPUT):
    verify_source()
    output = output.resolve()
    ndk = core.sdk_root() / "ndk" / TOOLS["ndk"]
    if not (ndk / "source.properties").is_file():
        raise ValueError(f"Install pinned NDK {TOOLS['ndk']}")
    revision = next((line.split("=", 1)[1].strip() for line in (ndk / "source.properties").read_text().splitlines()
                     if line.startswith("Pkg.Revision")), None)
    if revision != TOOLS["ndk"]:
        raise ValueError("Graphics Path NDK revision differs from the pinned toolchain")
    for abi in TOOLS["abis"]:
        destination = output / abi / LIBRARY
        destination.parent.mkdir(parents=True, exist_ok=True)
        temporary = destination.with_suffix(".so.tmp")
        subprocess.run(compile_command(ndk, abi, temporary), check=True, cwd=SOURCE,
                       env={**os.environ, "SOURCE_DATE_EPOCH": "0"})
        subprocess.run([core.ndk_bin(ndk) / "llvm-strip", "--strip-unneeded", temporary], check=True)
        check_library(temporary, ndk)
        temporary.replace(destination)
        print(f"Built {abi}/{LIBRARY}: {hashlib.sha256(destination.read_bytes()).hexdigest()}")


def repack_archive(original, output, native):
    """Retain every non-JNI entry byte-for-byte; never patch an ELF binary."""
    with zipfile.ZipFile(original) as archive:
        infos = archive.infolist()
        names = [info.filename for info in infos]
        if len(names) != len(set(names)):
            raise ValueError("Duplicate AAR entries")
        retained = {}
        for info in infos:
            name = info.filename
            if name.startswith("jni/"):
                if not name.endswith("/") and Path(name).name != LIBRARY:
                    raise ValueError(f"Unreviewed JNI input: {name}")
            else:
                retained[name] = archive.read(info)
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(dir=output.parent, suffix=".aar", delete=False) as staged:
        temporary = Path(staged.name)
    try:
        with zipfile.ZipFile(temporary, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
            entries = {**retained, **{f"jni/{abi}/{LIBRARY}": data for abi, data in native.items()}}
            for name, data in sorted(entries.items()):
                info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
                info.compress_type = zipfile.ZIP_DEFLATED
                info.external_attr = (0o755 if name.endswith("/") else 0o644) << 16
                archive.writestr(info, data, compresslevel=9)
        temporary.replace(output)
    finally:
        temporary.unlink(missing_ok=True)


def repack(original, output, native_root=OUTPUT):
    verify_source()
    if hashlib.sha256(original.read_bytes()).hexdigest() != PIN["aar_sha256"]:
        raise ValueError("Graphics Path AAR differs from reviewed 1.1.0 artifact")
    with zipfile.ZipFile(original) as archive:
        if hashlib.sha256(archive.read("classes.jar")).hexdigest() != PIN["classes_sha256"]:
            raise ValueError("Graphics Path Kotlin API classes differ from the release")
    ndk = core.sdk_root() / "ndk" / TOOLS["ndk"]
    native = {}
    for abi in TOOLS["abis"]:
        library = native_root / abi / LIBRARY
        check_library(library, ndk)
        native[abi] = library.read_bytes()
    repack_archive(original, output, native)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--verify-source", action="store_true")
    parser.add_argument("--output-dir", type=Path, default=OUTPUT)
    parser.add_argument("--repack-aar", type=Path)
    parser.add_argument("--output-aar", type=Path)
    args = parser.parse_args()
    if bool(args.repack_aar) != bool(args.output_aar):
        parser.error("--repack-aar and --output-aar must be supplied together")
    if args.verify_source:
        verify_source()
        print(f"Verified unchanged Graphics Path {PIN['version']} source {PIN['source_commit']}")
    elif args.repack_aar:
        repack(args.repack_aar, args.output_aar, args.output_dir)
    else:
        lock = ANDROID / "rust-target/graphics-path-build.lock"
        lock.parent.mkdir(parents=True, exist_ok=True)
        with lock.open("w") as handle:
            fcntl.flock(handle, fcntl.LOCK_EX)
            build(args.output_dir)


if __name__ == "__main__":
    main()
