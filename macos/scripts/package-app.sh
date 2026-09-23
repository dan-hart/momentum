#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Dan Hart
#
# Builds the release app and packages it the way a GitHub Release ships it:
#   <out>/Momentum-vX.Y.Z-macos.zip           the app, mo inside Contents/MacOS
#   <out>/mo-vX.Y.Z-macos-universal.tar.gz    the command line on its own, for Homebrew
#
#   macos/scripts/package-app.sh [out-dir]    # default: dist
#
# Signing: ad hoc unless MACOS_SIGN_IDENTITY (e.g. "Developer ID Application") and
# APPLE_TEAM_ID are set. Notarization runs when APPLE_ID, APPLE_TEAM_ID and
# APPLE_APP_PASSWORD are all set; the ticket is stapled to the app before zipping.
# REQUIRE_UNIVERSAL=1 fails the build unless both Apple architectures are in the binary.
set -eu
cd "$(dirname "$0")/../.."
ROOT=$PWD
# A release needs rustup's toolchain: the second Apple target for a universal binary, and
# a cargo that builds with an explicit --target (a Homebrew cargo is not guaranteed to).
export PATH="$HOME/.cargo/bin:$PATH"
OUT=$(mkdir -p "${1:-dist}" && cd "${1:-dist}" && pwd)
VERSION=$(build-aux/release.py version)
IDENTITY=${MACOS_SIGN_IDENTITY:--}
TEAM=${APPLE_TEAM_ID:-}
DERIVED="$ROOT/build/macos-release"
APP="$DERIVED/Build/Products/Release/Momentum.app"

# Xcode resolves the MomentumCore package before any build phase runs, so the
# xcframework and bindings must exist first: release profile, every installed target.
echo "== Momentum $VERSION: build the core"
macos/scripts/build-core.sh

# Xcode's Release configuration links arm64 and x86_64; build only what the core has, so
# a Mac without the second Rust target still packages (REQUIRE_UNIVERSAL catches it in CI).
CORE_ARCHS=$(lipo -archs "$ROOT/target/xcframework-stage/lib/libmomentum_ffi.a")
echo "   core architectures: $CORE_ARCHS"

echo "== xcodegen"
(cd macos && xcodegen generate >/dev/null)

echo "== xcodebuild Release (identity: $IDENTITY)"
SIGN_FLAGS="CODE_SIGN_STYLE=Manual CODE_SIGN_IDENTITY=$IDENTITY"
if [ "$IDENTITY" != "-" ]; then
  [ -n "$TEAM" ] || { echo "APPLE_TEAM_ID is required with MACOS_SIGN_IDENTITY" >&2; exit 1; }
  SIGN_FLAGS="$SIGN_FLAGS DEVELOPMENT_TEAM=$TEAM OTHER_CODE_SIGN_FLAGS=--timestamp"
fi
mkdir -p "$DERIVED"
LOG="$DERIVED/xcodebuild.log"
# shellcheck disable=SC2086
if ! xcodebuild -project macos/Momentum.xcodeproj -scheme Momentum -configuration Release \
  -derivedDataPath "$DERIVED" ARCHS="$CORE_ARCHS" ONLY_ACTIVE_ARCH=NO $SIGN_FLAGS build >"$LOG" 2>&1; then
  echo "xcodebuild failed; full log: $LOG" >&2
  grep -E 'error:|\*\* BUILD' "$LOG" >&2 || tail -n 60 "$LOG" >&2
  exit 1
fi
[ -d "$APP" ] || { echo "no app at $APP" >&2; exit 1; }

echo "== verify the bundle"
BUNDLE_VERSION=$(/usr/libexec/PlistBuddy -c 'Print CFBundleShortVersionString' "$APP/Contents/Info.plist")
[ "$BUNDLE_VERSION" = "$VERSION" ] || { echo "app says $BUNDLE_VERSION, workspace says $VERSION" >&2; exit 1; }
ARCHS=$(lipo -archs "$APP/Contents/MacOS/Momentum")
echo "   architectures: $ARCHS"
if [ "${REQUIRE_UNIVERSAL:-0}" = 1 ]; then
  case "$ARCHS" in *arm64*x86_64*|*x86_64*arm64*) ;; *) echo "not a universal binary: $ARCHS" >&2; exit 1 ;; esac
fi
CLI_VERSION=$("$APP/Contents/MacOS/mo" --version | awk '{print $2}')
[ "$CLI_VERSION" = "$VERSION" ] || { echo "mo says $CLI_VERSION, workspace says $VERSION" >&2; exit 1; }
codesign --verify --deep --strict --verbose=1 "$APP"

if [ -n "${APPLE_ID:-}" ] && [ -n "$TEAM" ] && [ -n "${APPLE_APP_PASSWORD:-}" ]; then
  echo "== notarize"
  [ "$IDENTITY" != "-" ] || { echo "notarization needs a Developer ID signature, not ad hoc" >&2; exit 1; }
  NOTARY_ZIP="$DERIVED/notarize.zip"
  ditto -c -k --norsrc --keepParent "$APP" "$NOTARY_ZIP"
  xcrun notarytool submit "$NOTARY_ZIP" --apple-id "$APPLE_ID" --team-id "$TEAM" \
    --password "$APPLE_APP_PASSWORD" --wait
  xcrun stapler staple "$APP"
  spctl --assess --type execute --verbose=2 "$APP"
fi

echo "== package"
ZIP="$OUT/Momentum-v$VERSION-macos.zip"
rm -f "$ZIP"
# --norsrc: without it, ditto stores extended attributes as AppleDouble entries that
# Info-ZIP's unzip (which Homebrew uses) extracts as stray ._ files inside the bundle,
# breaking the code signature's resource seal. The signature seals no xattrs.
ditto -c -k --norsrc --keepParent "$APP" "$ZIP"

STAGE="$DERIVED/cli"
rm -rf "$STAGE" && mkdir -p "$STAGE"
cp "$ROOT/target/xcframework-stage/mo" "$STAGE/mo"
if [ "$IDENTITY" = "-" ]; then
  codesign --force --options runtime --sign - "$STAGE/mo"
else
  codesign --force --options runtime --timestamp --sign "$IDENTITY" "$STAGE/mo"
fi
TAR="$OUT/mo-v$VERSION-macos-universal.tar.gz"
rm -f "$TAR"
tar -czf "$TAR" -C "$STAGE" mo

echo "== done"
ls -l "$ZIP" "$TAR"
