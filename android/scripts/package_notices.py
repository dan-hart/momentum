#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Package source licenses, artifact notices and an exact dependency inventory."""
import argparse
import hashlib
import io
import json
import re
import subprocess
import xml.etree.ElementTree as ET
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ANDROID = ROOT / "android"
NOTICE_NAME = re.compile(r"^(licen[sc]e|copying|notice)(?:$|[-_.])", re.IGNORECASE)


def archive_notices(path):
    """AARs can contain notices at their root and inside classes.jar."""
    def collect(archive, prefix=""):
        notices = []
        for name in sorted(archive.namelist()):
            if not name.endswith("/") and NOTICE_NAME.match(Path(name).name):
                notices.append((prefix + name, archive.read(name)))
        if "classes.jar" in archive.namelist():
            with zipfile.ZipFile(io.BytesIO(archive.read("classes.jar"))) as nested:
                notices.extend(collect(nested, prefix + "classes.jar/"))
        return notices
    with zipfile.ZipFile(path) as archive:
        return collect(archive)


def pom_licenses(path, visited=None):
    path = Path(path)
    visited = set() if visited is None else visited
    if path in visited:
        raise ValueError(f"Cyclic Maven parent POM: {path.name}")
    visited.add(path)
    namespace = {"m": "http://maven.apache.org/POM/4.0.0"}
    document = ET.parse(path)
    licenses = [((item.findtext("m:name", default="", namespaces=namespace)),
                 (item.findtext("m:url", default="", namespaces=namespace)))
                for item in document.findall("./m:licenses/m:license", namespace)]
    if not licenses:
        parent = document.find("./m:parent", namespace)
        cache = next((directory for directory in path.parents if directory.name == "files-2.1"), None)
        if parent is not None and cache:
            coordinate = [parent.findtext("m:" + key, namespaces=namespace) for key in ["groupId", "artifactId", "version"]]
            if all(coordinate):
                parent_path = cache.joinpath(*coordinate)
                candidates = list(parent_path.glob("*/*.pom"))
                if len(candidates) == 1:
                    licenses = pom_licenses(candidates[0], visited)
    return licenses


def rust_dependencies(metadata):
    packages = {package["id"]: package for package in metadata["packages"]}
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    pending = [key for key, package in packages.items() if package["name"] == "momentum-ffi"]
    if len(pending) != 1:
        raise ValueError("Expected one momentum-ffi dependency root")
    visited = set()
    while pending:
        key = pending.pop()
        if key in visited:
            continue
        visited.add(key)
        pending.extend(dependency["pkg"] for dependency in nodes[key]["deps"]
                       if any(kind["kind"] != "dev" for kind in dependency["dep_kinds"]))
    return [packages[key] for key in sorted(visited)]


def source_notices(package):
    root = Path(package["manifest_path"]).parent
    files = {path for path in root.rglob("*") if path.is_file() and NOTICE_NAME.match(path.name)}
    if package.get("license_file"):
        files.add(root / package["license_file"])
    # Momentum workspace crates inherit the top-level GPL notice/license.
    if root.is_relative_to(ROOT / "crates"):
        files.add(ROOT / "LICENSE")
    return [(str(path.relative_to(root)) if path.is_relative_to(root) else "Momentum/LICENSE", path.read_bytes())
            for path in sorted(files)]


def graphics_rebuild_notice(pin):
    """Retain matching source attribution for the JNI rebuilt from unchanged AndroidX code."""
    source = pin["source_url"]
    return (f"AndroidX Graphics Path {pin['version']} native source-build provenance\n"
            f"Source commit: {pin['source_commit']}\n"
            f"Matching source: {source}graphics/graphics-path/\n"
            f"Original Maven AAR SHA-256: {pin['aar_sha256']}\n"
            f"Unchanged classes.jar SHA-256: {pin['classes_sha256']}\n"
            "The original Maven Kotlin/API classes and other non-JNI entries are retained.\n"
            "Only JNI libraries are rebuilt from the pinned, unchanged C++ sources with\n"
            "the pinned Android NDK and 16 KiB maximum/common page-size linker options.\n"
            "The source license and every original per-file copyright header are retained.\n").encode()


def render(gradle_artifacts, rust_packages, supplied_notices):
    sections = ["Momentum Android third-party licenses and notices\n"
                "Momentum source remains GPL-3.0-or-later; LibreSync is AGPL-3.0-only.\n"
                "JNA uses the Apache-2.0 license option. Some entries are build tools.\n"
                "Matching source: https://github.com/dan-hart/momentum\n"]
    notices = {}

    def include(label, data):
        key = hashlib.sha256(data).hexdigest()
        entry = notices.setdefault(key, {"labels": [], "data": data})
        entry["labels"].append(label)

    for label, data in supplied_notices:
        include(label, data)
    for artifact in sorted(gradle_artifacts, key=lambda value: value["id"]):
        licenses = pom_licenses(artifact["pom"])
        if not licenses:
            raise ValueError(f"No declared POM license for {artifact['id']}")
        sections.append("Maven: " + artifact["id"] + "\n" +
                        "\n".join(f"  License: {name}; {url}" for name, url in licenses) + "\n")
        for name, data in archive_notices(artifact["path"]):
            include(f"{artifact['id']}: {name}", data)
    for package in sorted(rust_packages, key=lambda value: (value["name"], value["version"])):
        name = package["name"] + " " + package["version"]
        if not package.get("license") and not package.get("license_file"):
            raise ValueError(f"No declared Rust license for {name}")
        sections.append(f"Rust: {name}\n  License: {package.get('license') or package['license_file']}\n"
                        f"  Authors: {', '.join(package.get('authors', []))}\n"
                        f"  Source: {package.get('repository') or package.get('homepage') or 'Momentum workspace'}\n")
        for path, data in source_notices(package):
            include(f"{name}: {path}", data)
    for key in sorted(notices):
        entry = notices[key]
        data = entry["data"]
        try:
            content = data.decode("utf-8-sig")
        except UnicodeDecodeError:
            content = data.decode("latin-1")
        sections.append("\n" + "=" * 72 + "\n" + "\n".join(sorted(set(entry["labels"]))) +
                        "\nSHA-256: " + key + "\n\n" + content + "\n")
    return "\n".join(sections)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--gradle-input", type=Path, required=True)
    parser.add_argument("--output", type=Path, default=ANDROID / "app/build/generated/notices/THIRD_PARTY_NOTICES.txt")
    args = parser.parse_args()
    toolchain = json.loads((ANDROID / "toolchain.json").read_text())
    gradle = json.loads(args.gradle_input.read_text())
    jna = [artifact for artifact in gradle if artifact["id"] == f"net.java.dev.jna:jna:{toolchain['jna']['version']}"]
    if len(jna) != 1 or hashlib.sha256(Path(jna[0]["path"]).read_bytes()).hexdigest() != toolchain["jna"]["aar_sha256"]:
        raise ValueError("JNA AAR differs from the reviewed Maven artifact")
    supplied = [("Momentum/LICENSE", (ROOT / "LICENSE").read_bytes()),
                ("LibreSync/LICENSE", (ROOT / "libresync-src/LICENSE").read_bytes())]
    provenance = json.loads((ANDROID / "licenses/provenance.json").read_text())
    for item in provenance:
        data = (ANDROID / "licenses" / item["file"]).read_bytes()
        if hashlib.sha256(data).hexdigest() != item["sha256"]:
            raise ValueError(f"Altered upstream notice: {item['file']}")
        supplied.append((item["file"] + " (" + item["url"] + ")", data))
    supplied.append(("AndroidX Graphics Path JNI source provenance", graphics_rebuild_notice(toolchain["graphics_path"])))
    rust = {}
    for triple in toolchain["abis"].values():
        command = ["rustup", "run", toolchain["rust"], "cargo", "metadata", "--locked", "--format-version", "1",
                   "--filter-platform", triple]
        metadata = json.loads(subprocess.check_output(command, cwd=ROOT))
        rust.update((package["id"], package) for package in rust_dependencies(metadata))
    text = render(gradle, list(rust.values()), supplied)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(text, encoding="utf-8")
    print(f"Packaged notices: {len(gradle)} Maven artifacts, {len(rust)} Rust packages -> {args.output}")


if __name__ == "__main__":
    main()
