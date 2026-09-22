#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Dan Hart
#
# Runs the whole test suite: the core crates (pure Rust) and the app's logic and UI tests.
# UI tests need GTK's runtime assets and a display, so this script compiles the Blueprint
# UI, the resource bundle and the GSettings schema into target/test-assets, points the
# tests at them, and starts a headless Broadway display when no display is available.
# Settings changes never leave the process (GSETTINGS_BACKEND=memory).
#
#   build-aux/run-tests.sh                 # everything
#   build-aux/run-tests.sh -p sp-sync      # extra args go to cargo test
#
# Requires: cargo, blueprint-compiler, glib-compile-resources, glib-compile-schemas,
# gtk4-broadwayd (all in org.gnome.Sdk; build-aux/test.sh enters that sandbox for you).
set -eu
cd "$(dirname "$0")/.."
OUT=${MOMENTUM_TEST_ASSETS:-$PWD/target/test-assets}
mkdir -p "$OUT/ui" "$OUT/schemas"

blueprint-compiler batch-compile "$OUT" "$PWD/data/resources" data/resources/ui/*.blp >/dev/null
glib-compile-resources --sourcedir="$OUT" --sourcedir="$PWD/data/resources" \
  --target="$OUT/resources.gresource" data/resources/resources.gresource.xml
sed 's/@app-id@/io.github.dan_hart.Momentum.Devel/g; s/@gettext-package@/momentum/g' \
  data/io.github.dan_hart.Momentum.gschema.xml.in > "$OUT/schemas/io.github.dan_hart.Momentum.Devel.gschema.xml"
glib-compile-schemas "$OUT/schemas"

export MOMENTUM_TEST_RESOURCES="$OUT/resources.gresource"
export GSETTINGS_SCHEMA_DIR="$OUT/schemas"
export GSETTINGS_BACKEND=memory
# Never touch a real store, bus or server from tests.
unset MOMENTUM_DATA_DIR MOMENTUM_DEMO
export DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent/momentum-tests"

if [ "${MOMENTUM_TEST_BROADWAY:-}" = 1 ] || { [ -z "${WAYLAND_DISPLAY:-}" ] && [ -z "${DISPLAY:-}" ]; }; then
  export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-$OUT/run}"
  mkdir -p "$XDG_RUNTIME_DIR" && chmod 700 "$XDG_RUNTIME_DIR"
  DISPLAY_NO=${MOMENTUM_TEST_DISPLAY:-7}
  gtk4-broadwayd ":$DISPLAY_NO" >/dev/null 2>&1 &
  BROADWAY_PID=$!
  trap 'kill $BROADWAY_PID 2>/dev/null || true' EXIT INT TERM
  export GDK_BACKEND=broadway BROADWAY_DISPLAY=":$DISPLAY_NO"
  sleep 0.3
fi

# GTK, libadwaita and GLib warn on every frame on a headless display and in the narrow
# windows the UI tests open. Meson echoes only the last 100 lines of a failing test, so
# that noise buries the failure itself. Print everything else, keep the whole log on disk,
# and report cargo's own exit status rather than the filter's.
NOISE='Adwaita-WARNING|Gtk-WARNING|GLib-GIO-CRITICAL|IBUS-WARNING|MESA-EGL|Unable to acquire session bus'
LOG="$OUT/cargo-test.log"
STATUS_FILE="$OUT/cargo-test.status"
rm -f "$STATUS_FILE"
# `|| CODE=$?` keeps set -e from killing this subshell before the status is recorded.
{ CODE=0; cargo test --workspace "$@" 2>&1 || CODE=$?; echo "$CODE" >"$STATUS_FILE"; } |
  tee "$LOG" | { grep -vE "$NOISE" || true; }
STATUS=$(cat "$STATUS_FILE")
[ "$STATUS" -eq 0 ] || echo "Filtered GTK warnings; full test log: $LOG" >&2
exit "$STATUS"
