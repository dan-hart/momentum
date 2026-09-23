#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Dan Hart
"""One version for every app, and the files a release derives from it.

Cargo.toml's workspace version is the source of truth. The GTK app, the macOS app, the
Flatpak metadata and the changelog each carry a copy, and this tool keeps them equal:

  build-aux/release.py version                 print the version
  build-aux/release.py check [--tag vX.Y.Z]    every copy agrees (and matches the tag)
  build-aux/release.py bump X.Y.Z [--summary]  set every copy, close the changelog section,
                                               add the metainfo <release>
  build-aux/release.py notes X.Y.Z             the changelog section, for the GitHub Release
  build-aux/release.py homebrew ...            render the tap formula and cask from assets
  build-aux/release.py flathub ...             render the Flathub manifest for a tag

Runs on the system Python of macOS and of the GNOME SDK: standard library only.
"""
import argparse
import datetime as dt
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from xml.sax.saxutils import escape

ROOT = Path(__file__).resolve().parent.parent
CARGO_TOML = ROOT / "Cargo.toml"
MESON_BUILD = ROOT / "meson.build"
PROJECT_YML = ROOT / "macos" / "project.yml"
METAINFO = ROOT / "data" / "io.github.dan_hart.Momentum.metainfo.xml.in.in"
CHANGELOG = ROOT / "CHANGELOG.md"
FLATHUB_MANIFEST = ROOT / "build-aux" / "flathub" / "io.github.dan_hart.Momentum.json"
HOMEBREW_TEMPLATES = ROOT / "build-aux" / "homebrew"
REPO_URL = "https://github.com/dan-hart/momentum"

SEMVER = re.compile(r"^\d+\.\d+\.\d+$")


def fail(message):
    print(f"release.py: {message}", file=sys.stderr)
    sys.exit(1)


def read(path):
    return path.read_text(encoding="utf-8")


def write(path, text):
    path.write_text(text, encoding="utf-8")


def sub_once(pattern, replacement, text, where):
    """Replace exactly one match, or refuse: a silent no-op here would ship a wrong version."""
    new, count = re.subn(pattern, replacement, text, count=1, flags=re.M)
    if count != 1:
        fail(f"could not find the version in {where}")
    return new


# ---- where each copy of the version lives --------------------------------------------

def cargo_version(text):
    m = re.search(r"^\[workspace\.package\]\n(?:.*\n)*?version = \"([^\"]+)\"", text, re.M)
    return m.group(1) if m else None


def meson_version(text):
    m = re.search(r"^\s*version: '([^']+)',", text, re.M)
    return m.group(1) if m else None


def marketing_version(text):
    m = re.search(r"^\s*MARKETING_VERSION: \"([^\"]+)\"", text, re.M)
    return m.group(1) if m else None


def build_number(text):
    m = re.search(r"^\s*CURRENT_PROJECT_VERSION: \"([^\"]+)\"", text, re.M)
    return m.group(1) if m else None


def metainfo_version(text):
    m = re.search(r"<release version=\"([^\"]+)\"", text)
    return m.group(1) if m else None


def metainfo_screenshot_tags(text):
    return set(re.findall(r"raw\.githubusercontent\.com/dan-hart/momentum/v([^/]+)/", text))


def changelog_latest(text):
    m = re.search(r"^## \[(\d+\.\d+\.\d+)\]", text, re.M)
    return m.group(1) if m else None


def expected_build_number(version):
    """CFBundleVersion must only ever go up; derive it from the version so it cannot be forgotten."""
    major, minor, patch = (int(p) for p in version.split("."))
    return str(major * 10000 + minor * 100 + patch)


def current_version():
    version = cargo_version(read(CARGO_TOML))
    if not version:
        fail("no workspace version in Cargo.toml")
    return version


# ---- changelog ------------------------------------------------------------------------

def changelog_section(text, heading_re):
    """Body of the first section whose `## ` heading matches, up to the next `## `."""
    m = re.search(rf"^## {heading_re}[^\n]*\n(.*?)(?=^## |\Z)", text, re.M | re.S)
    return m.group(1) if m else None


def changelog_bullets(section):
    """Top-level bullets, with their wrapped continuation lines joined."""
    bullets = []
    for line in section.splitlines():
        if line.startswith("- "):
            bullets.append(line[2:].strip())
        elif line.startswith("  ") and bullets and line.strip():
            bullets[-1] += " " + line.strip()
    return bullets


def markdown_to_plain(text):
    text = re.sub(r"`([^`]*)`", r"\1", text)
    text = re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", text)
    return text


# ---- commands -------------------------------------------------------------------------

def cmd_version(_args):
    print(current_version())


def collect_mismatches(version, tag=None):
    problems = []
    if not SEMVER.match(version):
        problems.append(f"Cargo.toml version {version!r} is not MAJOR.MINOR.PATCH")

    meson = meson_version(read(MESON_BUILD))
    if meson != version:
        problems.append(f"meson.build says {meson}")

    project = read(PROJECT_YML)
    marketing = marketing_version(project)
    if marketing != version:
        problems.append(f"macos/project.yml MARKETING_VERSION says {marketing}")
    build = build_number(project)
    if build != expected_build_number(version):
        problems.append(
            f"macos/project.yml CURRENT_PROJECT_VERSION is {build}, "
            f"expected {expected_build_number(version)} for {version}"
        )

    metainfo = read(METAINFO)
    newest = metainfo_version(metainfo)
    if newest != version:
        problems.append(f"metainfo's newest <release> is {newest}")
    tags = metainfo_screenshot_tags(metainfo)
    if tags and tags != {version}:
        problems.append(f"metainfo screenshots point at {', '.join(sorted(tags))}")

    latest = changelog_latest(read(CHANGELOG))
    if latest != version:
        problems.append(f"CHANGELOG.md's newest released section is {latest}")

    if tag is not None and tag != f"v{version}":
        problems.append(f"tag {tag} does not match v{version}")
    return problems


def cmd_check(args):
    version = current_version()
    problems = collect_mismatches(version, args.tag)
    if problems:
        print(f"Cargo.toml is at {version}, but:", file=sys.stderr)
        for p in problems:
            print(f"  - {p}", file=sys.stderr)
        print("Run build-aux/release.py bump X.Y.Z to align them.", file=sys.stderr)
        sys.exit(1)
    print(f"every version is {version}")


def cmd_bump(args):
    new = args.version
    if not SEMVER.match(new):
        fail(f"{new!r} is not MAJOR.MINOR.PATCH")
    old = current_version()
    if tuple(map(int, new.split("."))) <= tuple(map(int, old.split("."))):
        fail(f"{new} is not newer than the current {old}")
    date = args.date or dt.date.today().isoformat()

    changelog = read(CHANGELOG)
    unreleased = changelog_section(changelog, r"\[Unreleased\]")
    if unreleased is None:
        fail("CHANGELOG.md has no [Unreleased] section")
    bullets = changelog_bullets(unreleased)
    if not bullets and not args.allow_empty:
        fail("CHANGELOG.md's [Unreleased] section is empty; add the notes first, or pass --allow-empty")

    # Cargo.toml: the source of truth.
    write(CARGO_TOML, sub_once(
        r"^(\[workspace\.package\]\n(?:.*\n)*?version = )\"[^\"]+\"",
        f'\\g<1>"{new}"', read(CARGO_TOML), "Cargo.toml"))
    # meson.build
    write(MESON_BUILD, sub_once(r"^(\s*version: )'[^']+',", rf"\g<1>'{new}',", read(MESON_BUILD), "meson.build"))
    # macOS
    project = read(PROJECT_YML)
    project = sub_once(r"^(\s*MARKETING_VERSION: )\"[^\"]+\"", f'\\g<1>"{new}"', project, "macos/project.yml")
    project = sub_once(r"^(\s*CURRENT_PROJECT_VERSION: )\"[^\"]+\"",
                       f'\\g<1>"{expected_build_number(new)}"', project, "macos/project.yml")
    write(PROJECT_YML, project)

    # CHANGELOG: close the section, open a fresh one.
    header = f"## [{new}] - {date}"
    changelog = changelog.replace("## [Unreleased]", f"## [Unreleased]\n\n{header}", 1)
    write(CHANGELOG, changelog)

    # metainfo: a <release> built from the changelog bullets, and screenshots from the tag.
    metainfo = read(METAINFO)
    items = "\n".join(f"          <li>{escape(markdown_to_plain(b))}</li>" for b in bullets)
    summary = f"        <p>{escape(args.summary)}</p>\n" if args.summary else ""
    body = f"{summary}        <ul>\n{items}\n        </ul>\n" if bullets else summary
    release = (
        f"    <release version=\"{new}\" date=\"{date}\">\n"
        f"      <description>\n{body}      </description>\n"
        f"    </release>\n"
    )
    if "<releases>\n" not in metainfo:
        fail("metainfo has no <releases> element")
    metainfo = metainfo.replace("<releases>\n", "<releases>\n" + release, 1)
    metainfo = re.sub(r"(raw\.githubusercontent\.com/dan-hart/momentum/)v[^/]+/", rf"\g<1>v{new}/", metainfo)
    write(METAINFO, metainfo)

    # Cargo.lock records the workspace crates' versions too; refresh only those.
    try:
        subprocess.run(["cargo", "update", "--workspace", "--offline", "--quiet"],
                       cwd=ROOT, check=True, capture_output=True)
    except (OSError, subprocess.CalledProcessError):
        print("note: run `cargo update --workspace` to refresh Cargo.lock", file=sys.stderr)

    problems = collect_mismatches(new)
    if problems:
        fail("bump left a mismatch: " + "; ".join(problems))
    print(f"{old} -> {new}: Cargo.toml, meson.build, macos/project.yml, CHANGELOG.md, metainfo")
    print(f"Review the diff, commit, then: git tag -a v{new} -m 'Momentum {new}' && git push origin main v{new}")


def cmd_notes(args):
    version = args.version or current_version()
    section = changelog_section(read(CHANGELOG), rf"\[{re.escape(version)}\]")
    if section is None:
        fail(f"CHANGELOG.md has no section for {version}")
    print(section.strip())


def sha256_of(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def cmd_homebrew(args):
    version = args.version or current_version()
    assets = Path(args.assets)
    names = {
        "SHA_MACOS_APP": f"Momentum-v{version}-macos.zip",
        "SHA_MO_MACOS": f"mo-v{version}-macos-universal.tar.gz",
        "SHA_MO_LINUX_X86_64": f"mo-v{version}-linux-x86_64.tar.gz",
        "SHA_MO_LINUX_AARCH64": f"mo-v{version}-linux-aarch64.tar.gz",
    }
    values = {"VERSION": version}
    for key, name in names.items():
        path = assets / name
        if not path.is_file():
            fail(f"missing release asset {name} in {assets}")
        values[key] = sha256_of(path)
    if args.notarized:
        values["CAVEATS"] = ""
    else:
        values["CAVEATS"] = (
            "\n  caveats <<~EOS\n"
            "    This build is signed ad hoc, not notarized by Apple. macOS will refuse to open it\n"
            "    unless you install with --no-quarantine or allow it under System Settings > Privacy & Security.\n"
            "  EOS\n"
        )
    out = Path(args.out)
    rendered = []
    for template, dest in (("momentum-cli.rb.in", "Formula/momentum-cli.rb"), ("momentum.rb.in", "Casks/momentum.rb")):
        text = read(HOMEBREW_TEMPLATES / template)
        # The caveats block takes a whole line, or none at all when there is nothing to say.
        text = text.replace("@CAVEATS@\n", values["CAVEATS"])
        for key, value in values.items():
            text = text.replace(f"@{key}@", value)
        leftover = re.findall(r"@[A-Z_]+@", text)
        if leftover:
            fail(f"{template} still has placeholders {leftover}")
        target = out / dest
        target.parent.mkdir(parents=True, exist_ok=True)
        write(target, text)
        rendered.append(str(target))
    print("\n".join(rendered))


def cmd_flathub(args):
    manifest = json.loads(read(FLATHUB_MANIFEST))
    module = next(m for m in manifest["modules"] if m["name"] == "momentum")
    for source in module["sources"]:
        if not isinstance(source, dict) or source.get("type") != "git":
            continue
        if source["url"].endswith("/momentum.git"):
            source["tag"] = args.tag
            source["commit"] = args.commit
        elif source["url"].endswith("/LibreSync.git"):
            if args.libresync_ref:
                source["tag"] = args.libresync_ref
            if args.libresync_commit:
                source["commit"] = args.libresync_commit
    text = json.dumps(manifest, indent=4) + "\n"
    if args.out:
        write(Path(args.out), text)
    else:
        sys.stdout.write(text)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)

    sub.add_parser("version", help="print the workspace version").set_defaults(func=cmd_version)

    p = sub.add_parser("check", help="every copy of the version agrees")
    p.add_argument("--tag", help="a git tag that must equal v<version>")
    p.set_defaults(func=cmd_check)

    p = sub.add_parser("bump", help="set a new version everywhere and close the changelog")
    p.add_argument("version")
    p.add_argument("--date", help="release date, default today (YYYY-MM-DD)")
    p.add_argument("--summary", help="one-sentence tagline for the metainfo release entry")
    p.add_argument("--allow-empty", action="store_true", help="bump with an empty [Unreleased] section")
    p.set_defaults(func=cmd_bump)

    p = sub.add_parser("notes", help="print the changelog section for a version")
    p.add_argument("version", nargs="?")
    p.set_defaults(func=cmd_notes)

    p = sub.add_parser("homebrew", help="render the tap's formula and cask from downloaded release assets")
    p.add_argument("--version")
    p.add_argument("--assets", required=True, help="directory holding the release assets")
    p.add_argument("--out", required=True, help="tap checkout to write Formula/ and Casks/ into")
    p.add_argument("--notarized", action="store_true", help="the app was notarized; omit the Gatekeeper caveat")
    p.set_defaults(func=cmd_homebrew)

    p = sub.add_parser("flathub", help="render the Flathub manifest pinned to a tag")
    p.add_argument("--tag", required=True)
    p.add_argument("--commit", required=True)
    p.add_argument("--libresync-ref")
    p.add_argument("--libresync-commit")
    p.add_argument("--out")
    p.set_defaults(func=cmd_flathub)

    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
