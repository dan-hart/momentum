#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Dan Hart
# Single producer of the Apple XCFramework. Mac builds retain installed iOS slices.
set -eu
cd "$(dirname "$0")/../.."
ROOT=$PWD
mkdir -p "$ROOT/target"
# Keep package regeneration serial across Xcode schemes and manual invocations.
# flock is released by the OS even if the build is interrupted.
if [ "${MOMENTUM_APPLE_BUILD_LOCKED:-}" != 1 ]; then
  exec /usr/bin/python3 - "$ROOT/macos/scripts/build-core.sh" "$@" <<'PYLOCK'
import fcntl, os, subprocess, sys
with open("target/apple-build.lock", "w") as lock:
    fcntl.flock(lock, fcntl.LOCK_EX)
    env = dict(os.environ, MOMENTUM_APPLE_BUILD_LOCKED="1")
    raise SystemExit(subprocess.call(["/bin/sh", *sys.argv[1:]], env=env))
PYLOCK
fi
PKG="$ROOT/macos/Packages/MomentumCore"
OUT="$PKG/momentum_ffi.xcframework"
PROFILE=release
CARGO_FLAGS=--release
for arg in "$@"; do
  case "$arg" in
    --debug) PROFILE=debug; CARGO_FLAGS= ;;
    *) echo "Unknown argument: $arg" >&2; exit 2 ;;
  esac
done
# Use the rustup toolchain owning the cross-target standard libraries.
if [ -x "$HOME/.cargo/bin/rustup" ]; then
  export PATH="$HOME/.cargo/bin:$PATH"
else
  export PATH="$PATH:/opt/homebrew/bin:/usr/local/bin"
fi
unset SDKROOT LIBRARY_PATH IPHONEOS_DEPLOYMENT_TARGET MACOSX_DEPLOYMENT_TARGET
unset SWIFT_DEBUG_INFORMATION_FORMAT SWIFT_DEBUG_INFORMATION_VERSION
# Release builds strip debuginfo by running Xcode's `strip`; Xcode 27's leaves the
# proc-macro dylibs with a mis-aligned LINKEDIT string pool that macOS 27's dyld refuses
# to load, and cargo then reports "can't find crate for `serde_derive`". Keep the symbols;
# Xcode strips the app and mo is small either way.
export CARGO_PROFILE_RELEASE_STRIP="${CARGO_PROFILE_RELEASE_STRIP:-none}"
HOST=$(rustc -vV | sed -n 's/^host: //p')
MAC_TARGETS="$HOST"
IOS_TARGETS=""
INSTALLED=$(rustup target list --installed 2>/dev/null || true)
if [ "$PROFILE" = release ]; then
  for t in aarch64-apple-darwin x86_64-apple-darwin; do
    if echo "$INSTALLED" | grep -qx "$t" && [ "$t" != "$HOST" ]; then
      MAC_TARGETS="$MAC_TARGETS $t"
    fi
  done
fi
for t in aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios; do
  if echo "$INSTALLED" | grep -qx "$t"; then IOS_TARGETS="$IOS_TARGETS $t"; fi
done
for t in $MAC_TARGETS; do
  echo "== Rust macOS ($t, $PROFILE)"
  SDKROOT=$(xcrun --sdk macosx --show-sdk-path) MACOSX_DEPLOYMENT_TARGET=26.0 \
    cargo build -p momentum-ffi -p mo $CARGO_FLAGS --target "$t"
done
for t in $IOS_TARGETS; do
  case "$t" in *-sim|x86_64-apple-ios) SDK=iphonesimulator ;; *) SDK=iphoneos ;; esac
  echo "== Rust iOS ($t, $PROFILE)"
  SDKROOT=$(xcrun --sdk "$SDK" --show-sdk-path) IPHONEOS_DEPLOYMENT_TARGET=26.0 \
    cargo build -p momentum-ffi $CARGO_FLAGS --target "$t"
done
STAGE=$(mktemp -d "$ROOT/target/apple-stage.XXXXXX")
trap 'rm -rf "$STAGE"' EXIT HUP INT TERM
mkdir -p "$STAGE/include/momentumFFI" "$ROOT/target/xcframework-stage"
# Generate metadata with the host SDK, not the caller's iPhone linker environment.
SDKROOT=$(xcrun --sdk macosx --show-sdk-path) MACOSX_DEPLOYMENT_TARGET=26.0 \
  cargo run -q -p uniffi-bindgen -- generate \
    --library "$ROOT/target/$HOST/$PROFILE/libmomentum_ffi.a" \
    --language swift --out-dir "$STAGE/bindings" >/dev/null
cp "$STAGE/bindings/momentumFFI.h" "$STAGE/include/momentumFFI/momentumFFI.h"
cat > "$STAGE/include/momentumFFI/module.modulemap" <<'MM'
module momentumFFI {
    header "momentumFFI.h"
    export *
}
MM
set --
for t in $MAC_TARGETS; do set -- "$@" "$ROOT/target/$t/$PROFILE/libmomentum_ffi.a"; done
if [ "$#" -gt 1 ]; then
  lipo -create "$@" -output "$STAGE/libmomentum_ffi.a"
else
  cp "$1" "$STAGE/libmomentum_ffi.a"
fi
set --
for t in $MAC_TARGETS; do set -- "$@" "$ROOT/target/$t/$PROFILE/mo"; done
if [ "$#" -gt 1 ]; then
  lipo -create "$@" -output "$STAGE/mo"
else
  cp "$1" "$STAGE/mo"
fi
set -- -create-xcframework -library "$STAGE/libmomentum_ffi.a" -headers "$STAGE/include"
if echo "$INSTALLED" | grep -qx aarch64-apple-ios; then
  set -- "$@" -library "$ROOT/target/aarch64-apple-ios/$PROFILE/libmomentum_ffi.a" -headers "$STAGE/include"
fi
SIM_ARM="$ROOT/target/aarch64-apple-ios-sim/$PROFILE/libmomentum_ffi.a"
SIM_INTEL="$ROOT/target/x86_64-apple-ios/$PROFILE/libmomentum_ffi.a"
if echo "$INSTALLED" | grep -qx aarch64-apple-ios-sim; then
  if echo "$INSTALLED" | grep -qx x86_64-apple-ios; then
    lipo -create "$SIM_ARM" "$SIM_INTEL" -output "$STAGE/libmomentum_sim.a"
  else
    cp "$SIM_ARM" "$STAGE/libmomentum_sim.a"
  fi
  set -- "$@" -library "$STAGE/libmomentum_sim.a" -headers "$STAGE/include"
fi
xcodebuild "$@" -output "$STAGE/momentum_ffi.xcframework" >/dev/null
mkdir -p "$PKG/Sources/MomentumCore"
cp "$STAGE/bindings/momentum.swift" "$PKG/Sources/MomentumCore/momentum.swift"
cp "$STAGE/mo" "$ROOT/target/xcframework-stage/mo"
rm -rf "$OUT"
mv "$STAGE/momentum_ffi.xcframework" "$OUT"
echo "== done: $OUT ($MAC_TARGETS $IOS_TARGETS)"
