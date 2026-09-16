#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Dan Hart
#
# Builds the Rust core for macOS and packages it for the Swift package:
#   macos/Packages/MomentumCore/momentum_ffi.xcframework   (static library + C header)
#   macos/Packages/MomentumCore/Sources/MomentumCore/momentum.swift   (UniFFI bindings)
#
# Runs as an Xcode build phase (so a plain `xcodebuild` works) and by hand:
#   macos/scripts/build-core.sh            # release, every Apple target rustup has installed
#   macos/scripts/build-core.sh --debug    # faster, this machine's architecture only
#
# Requires cargo (a rustup toolchain with `aarch64-apple-darwin` and, for universal
# binaries, `x86_64-apple-darwin`; a Homebrew cargo builds the host architecture only).
set -eu
cd "$(dirname "$0")/../.."
ROOT=$PWD
PKG="$ROOT/macos/Packages/MomentumCore"
OUT="$PKG/momentum_ffi.xcframework"
PROFILE=release
CARGO_FLAGS="--release"
for arg in "$@"; do
  case "$arg" in
    --debug) PROFILE=debug; CARGO_FLAGS="" ;;
  esac
done
# Xcode's build environment points cargo at the wrong SDK and linker; clear it. The
# caller's cargo comes first (one toolchain must own target/); Homebrew and rustup are
# fallbacks for the minimal PATH Xcode gives a build phase.
unset IPHONEOS_DEPLOYMENT_TARGET LIBRARY_PATH
# Scheme actions and target phases have different compiler search environments.
# Pin the macOS SDK so C dependencies also find system headers under Xcode's clang.
export SDKROOT="$(xcrun --sdk macosx --show-sdk-path)"
export PATH="$PATH:/opt/homebrew/bin:/usr/local/bin:$HOME/.cargo/bin"
export MACOSX_DEPLOYMENT_TARGET=26.0

HOST=$(rustc -vV | sed -n 's/^host: //p')
TARGETS="$HOST"
if [ "$PROFILE" = release ] && command -v rustup >/dev/null 2>&1; then
  for t in aarch64-apple-darwin x86_64-apple-darwin; do
    if rustup target list --installed 2>/dev/null | grep -qx "$t" && [ "$t" != "$HOST" ]; then
      TARGETS="$TARGETS $t"
    fi
  done
fi

LIBS=""
CLIS=""
for t in $TARGETS; do
  echo "== cargo build momentum-ffi ($t, $PROFILE)"
  cargo build -p momentum-ffi -p mo $CARGO_FLAGS --target "$t"
  LIBS="$LIBS $ROOT/target/$t/$PROFILE/libmomentum_ffi.a"
  CLIS="$CLIS $ROOT/target/$t/$PROFILE/mo"
done

# One library for the xcframework: lipo the slices together when there are several.
STAGE="$ROOT/target/xcframework-stage"
rm -rf "$STAGE" && mkdir -p "$STAGE/lib" "$STAGE/include/momentumFFI"
set -- $LIBS
if [ $# -gt 1 ]; then
  lipo -create "$@" -output "$STAGE/lib/libmomentum_ffi.a"
else
  cp "$1" "$STAGE/lib/libmomentum_ffi.a"
fi

# Package the companion from exactly the same source, profile and architectures.
set -- $CLIS
if [ $# -gt 1 ]; then
  lipo -create "$@" -output "$STAGE/mo"
else
  cp "$1" "$STAGE/mo"
fi

echo "== uniffi-bindgen (swift)"
FIRST_TARGET=${TARGETS%% *}
# The bindings are read from the compiled library's metadata, so they always match it.
cargo run -q -p uniffi-bindgen -- generate \
  --library "$ROOT/target/$FIRST_TARGET/$PROFILE/libmomentum_ffi.a" \
  --language swift --out-dir "$STAGE/bindings" >/dev/null
cp "$STAGE/bindings/momentumFFI.h" "$STAGE/include/momentumFFI/momentumFFI.h"
cat > "$STAGE/include/momentumFFI/module.modulemap" <<MM
module momentumFFI {
    header "momentumFFI.h"
    export *
}
MM
mkdir -p "$PKG/Sources/MomentumCore"
cp "$STAGE/bindings/momentum.swift" "$PKG/Sources/MomentumCore/momentum.swift"

echo "== xcodebuild -create-xcframework"
rm -rf "$OUT"
xcodebuild -create-xcframework \
  -library "$STAGE/lib/libmomentum_ffi.a" -headers "$STAGE/include" \
  -output "$OUT" >/dev/null
echo "== done: $OUT ($TARGETS)"
