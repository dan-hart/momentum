#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Dan Hart
# Single public entry point: sanitize before taking the shared Apple build lock.
set -eu
cd "$(dirname "$0")/../.."
ROOT=$PWD
mkdir -p "$ROOT/target"
exec /usr/bin/python3 "$ROOT/macos/scripts/build-env.py" /usr/bin/python3 - "$ROOT/macos/scripts/build-core-impl.sh" "$@" <<'PYLOCK'
import fcntl, subprocess, sys
with open("target/apple-build.lock", "w") as lock:
    fcntl.flock(lock, fcntl.LOCK_EX)
    raise SystemExit(subprocess.call(["/bin/sh", *sys.argv[1:]]))
PYLOCK
