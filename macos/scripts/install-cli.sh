#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Link the companion to the app so updating Momentum also updates mo.
set -eu
APP=${1:-/Applications/Momentum.app}
if [ ! -d "$APP" ]; then
  echo "App not found: $APP" >&2
  exit 1
fi
APP=$(cd "$APP" && pwd -P)
BINARY="$APP/Contents/MacOS/mo"
if [ ! -x "$BINARY" ]; then
  echo "This Momentum build does not contain mo: $APP" >&2
  exit 1
fi
BIN_DIR=${MOMENTUM_CLI_BIN_DIR:-"$HOME/.local/bin"}
mkdir -p "$BIN_DIR"
DEST="$BIN_DIR/mo"
if [ -e "$DEST" ] || [ -L "$DEST" ]; then
  if [ -L "$DEST" ] && [ "$(readlink "$DEST")" = "$BINARY" ]; then
    echo "Already installed: $DEST"
    exit 0
  fi
  echo "Not replacing an existing command: $DEST" >&2
  echo "Move it aside or set MOMENTUM_CLI_BIN_DIR to another directory." >&2
  exit 1
fi
ln -s "$BINARY" "$DEST"
echo "Installed: $DEST"
echo "Keep Momentum at $APP; add $BIN_DIR to PATH if it is not already present."
