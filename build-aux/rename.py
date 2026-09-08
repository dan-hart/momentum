#!/usr/bin/env python3
"""Rename the project: display name, app ID, binary/crate name, author.

Run from the repository root. Rewrites file contents and file names in place.
"""
import re
import subprocess
import sys
from pathlib import Path

CUR = {
    "name": "Momentum",
    "project": "momentum",
    "app_id": "io.github.dan_hart.Momentum",
    "author": "Dan Hart",
    "email": "race-unbent-water@duck.com",
    "repo": "https://github.com/dan-hart/momentum",
}

root = Path(__file__).resolve().parent.parent
if not (root / "meson.build").exists():
    sys.exit("run from the repository root")

new = {}
for key, cur in CUR.items():
    new[key] = input(f"{key} [{cur}]: ").strip() or cur
if any("-" in seg for seg in new["app_id"].split(".")[:-1]):
    sys.exit("app id may only contain '-' in the last segment")

paths = {
    "app_path": "/" + "/".join(CUR["app_id"].split(".")) + "/",
}
new_paths = {"app_path": "/" + "/".join(new["app_id"].split(".")) + "/"}
subs = [
    (paths["app_path"], new_paths["app_path"]),
    (CUR["app_id"], new["app_id"]),
    (CUR["repo"], new["repo"]),
    (CUR["email"], new["email"]),
    (CUR["author"], new["author"]),
    (CUR["name"], new["name"]),
    (CUR["project"], new["project"]),
    (CUR["project"].replace("-", "_"), new["project"].replace("-", "_")),
]

tracked = subprocess.run(
    ["git", "ls-files"], cwd=root, capture_output=True, text=True, check=True
).stdout.split()

for rel in tracked:
    p = root / rel
    if p.suffix in {".png", ".lock"} or not p.is_file():
        continue
    try:
        text = p.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        continue
    out = text
    for a, b in subs:
        out = out.replace(a, b)
    if out != text:
        p.write_text(out, encoding="utf-8")

for rel in sorted(tracked, key=len, reverse=True):
    p = root / rel
    name = p.name
    for a, b in ((CUR["app_id"], new["app_id"]), (CUR["project"], new["project"])):
        name = name.replace(a, b)
    if name != p.name:
        subprocess.run(["git", "mv", str(p), str(p.with_name(name))], cwd=root, check=True)

print("Done. Review `git diff`, then regenerate Cargo.lock with `cargo generate-lockfile`.")
