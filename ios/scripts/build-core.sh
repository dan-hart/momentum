#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
set -eu
export PATH="$HOME/.cargo/bin:$PATH:/opt/homebrew/bin"
for target in aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios; do
  if ! rustup target list --installed | grep -qx "$target"; then
    echo "Missing Rust target: run rustup target add $target" >&2
    exit 1
  fi
done
exec "$(dirname "$0")/../../macos/scripts/build-core.sh" "$@"
