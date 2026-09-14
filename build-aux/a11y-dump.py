#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Dan Hart
"""Dump the accessibility tree of a running Momentum window and flag unnamed controls.

Orca can only announce what the toolkit exposes, so this is the automated half of the
accessibility check: every button, entry, check box and row must carry a name.

Run it on the host while the app is open (needs only `gdbus`, part of GLib):

    build-aux/a11y-dump.py [--all] [app-name]

`--all` prints the whole tree; the default prints only problems and a summary.
Exit status is 1 when a control has neither a name nor a description, 2 when the
application is not on the accessibility bus.
"""
import ast
import re
import subprocess
import sys

NEEDS_NAME = {
    "push button",
    "toggle button",
    "check box",
    "radio button",
    "menu button",
    "text",
    "entry",
    "password text",
    "spin button",
    "combo box",
    "check menu item",
    "radio menu item",
    "menu item",
    "list item",
    "tab",
    "switch",
}
ROOT = "/org/a11y/atspi/accessible/root"
IFACE = "org.a11y.atspi.Accessible"


def a11y_address():
    out = subprocess.check_output(
        ["gdbus", "call", "--session", "--dest", "org.a11y.Bus", "--object-path", "/org/a11y/bus",
         "--method", "org.a11y.Bus.GetAddress"], text=True)
    return ast.literal_eval(out.strip())[0]


class Bus:
    def __init__(self, address):
        self.address = address

    def call(self, dest, path, method, *args):
        cmd = ["gdbus", "call", "--address", self.address, "--dest", dest, "--object-path", path,
               "--method", method, *args]
        out = subprocess.run(cmd, text=True, capture_output=True)
        if out.returncode != 0:
            raise RuntimeError(out.stderr.strip())
        text = re.sub(r"@[a-z(){}]+ ", "", out.stdout.strip())  # drop type annotations: @a(so) []
        text = re.sub(r"\bobjectpath\b ", "", text)
        # Variant wrapper on properties; gdbus double-quotes strings holding an apostrophe.
        text = re.sub(r"""<('(?:[^'\\]|\\.)*'|"(?:[^"\\]|\\.)*")>""", r"\1", text)
        return ast.literal_eval(text)

    def prop(self, dest, path, name):
        v = self.call(dest, path, "org.freedesktop.DBus.Properties.Get", IFACE, name)
        return v[0] if isinstance(v, tuple) else v

    def role(self, dest, path):
        return self.call(dest, path, IFACE + ".GetRoleName")[0]

    def children(self, dest, path):
        return self.call(dest, path, IFACE + ".GetChildren")[0]


def walk(bus, dest, path, depth, out, problems, counts):
    try:
        role = bus.role(dest, path)
        name = bus.prop(dest, path, "Name") or ""
        desc = bus.prop(dest, path, "Description") or ""
        kids = bus.children(dest, path)
    except Exception as e:  # noqa: BLE001 — a vanished widget is not a finding
        out.append("  " * depth + f"<error {e}>")
        return
    counts[role] = counts.get(role, 0) + 1
    line = "  " * depth + f"{role}: {name!r}" + (f"  ({desc})" if desc else "")
    out.append(line)
    if role in NEEDS_NAME and not name and not desc:
        # A button whose child is a labelled widget is read via that child.
        labelled = False
        for kd, kp in kids:
            try:
                if bus.prop(kd, kp, "Name"):
                    labelled = True
            except Exception:  # noqa: BLE001
                pass
        if not labelled:
            problems.append(line.strip())
    for kd, kp in kids:
        walk(bus, kd, kp, depth + 1, out, problems, counts)


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    show_all = "--all" in sys.argv
    wanted = (args[0] if args else "momentum").lower()
    bus = Bus(a11y_address())
    apps = bus.children("org.a11y.atspi.Registry", ROOT)
    names = {}
    for dest, path in apps:
        try:
            names[(dest, path)] = bus.prop(dest, path, "Name") or ""
        except Exception:  # noqa: BLE001
            names[(dest, path)] = ""
    match = next((k for k, n in names.items() if wanted in n.lower()), None)
    if match is None:
        print(f"no application matching {wanted!r} on the accessibility bus; found: {sorted(set(names.values()))}")
        return 2
    out, problems, counts = [], [], {}
    walk(bus, match[0], match[1], 0, out, problems, counts)
    if show_all:
        print("\n".join(out))
    print(f"\n{sum(counts.values())} accessible objects: " + ", ".join(f"{k} {v}" for k, v in sorted(counts.items())))
    if problems:
        print(f"\n{len(problems)} unnamed controls:")
        print("\n".join("  " + p for p in problems))
        return 1
    print("every control has a name or description")
    return 0


if __name__ == "__main__":
    sys.exit(main())
