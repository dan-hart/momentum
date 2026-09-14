#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Dan Hart
# Runs build-aux/run-tests.sh inside the GNOME SDK sandbox, which has every tool the
# tests need and no access to your real display, bus or data. Arguments go to cargo test.
set -eu
cd "$(dirname "$0")/.."
exec flatpak run --devel --share=network --filesystem=home --command=bash org.gnome.Sdk//50 -c '
  export PATH=/usr/lib/sdk/rust-stable/bin:$PATH CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-target-sdk}
  cd "$0" && exec build-aux/run-tests.sh "$@"' "$PWD" "$@"
