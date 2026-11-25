#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
set -euo pipefail
cd "$(dirname "$0")/../.."

: "${ASC_APP_ID:?Set the App Store Connect app ID}"
: "${TESTFLIGHT_INTERNAL_GROUP_ID:?Set the internal TestFlight group ID}"
: "${ASC_KEY_ID:?Configure the ASC_KEY_ID Actions secret}"
: "${ASC_ISSUER_ID:?Configure the ASC_ISSUER_ID Actions secret}"
: "${ASC_PRIVATE_KEY_B64:?Configure the ASC_PRIVATE_KEY_B64 Actions secret}"
: "${APPLE_TEAM_ID:?Configure the APPLE_TEAM_ID Actions secret}"

umask 077
work=$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/momentum-testflight.XXXXXX")
trap 'rm -rf "$work"' EXIT
export ASC_PRIVATE_KEY_PATH="$work/AuthKey_${ASC_KEY_ID}.p8"
printf '%s' "$ASC_PRIVATE_KEY_B64" | /usr/bin/base64 -D > "$ASC_PRIVATE_KEY_PATH"
unset ASC_PRIVATE_KEY_B64

# Use the project's checked-in metadata for retry safety and preserve it on upload.
version=$(sed -n 's/^    MARKETING_VERSION: "\([^"]*\)"$/\1/p' macos/project.yml)
build=$(sed -n 's/^    CURRENT_PROJECT_VERSION: "\([^"]*\)"$/\1/p' macos/project.yml)
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ && "$build" =~ ^[0-9]+$ ]]
selector=(--app "$ASC_APP_ID" --platform MAC_OS --version "$version" --build-number "$build")
builds=$(asc builds list "${selector[@]}" --processing-state all --limit 2)
uploads=$(asc builds uploads list --app "$ASC_APP_ID" --platform MAC_OS \
  --cf-bundle-short-version "$version" --cf-bundle-version "$build" --limit 2)
if jq -e '.data | length == 0' <<< "$builds" >/dev/null &&
   jq -e '.data | length == 0' <<< "$uploads" >/dev/null; then
  signing=(-allowProvisioningUpdates -authenticationKeyPath "$ASC_PRIVATE_KEY_PATH"
    -authenticationKeyID "$ASC_KEY_ID" -authenticationKeyIssuerID "$ASC_ISSUER_ID")
  python3 macos/scripts/build-env.py xcodebuild archive \
    -project macos/MomentumTestFlight.xcodeproj -scheme Momentum -configuration Release \
    -destination 'generic/platform=macOS' -archivePath "$work/Momentum.xcarchive" \
    -derivedDataPath "$work/DerivedData" "DEVELOPMENT_TEAM=$APPLE_TEAM_ID" "${signing[@]}"

  app="$work/Momentum.xcarchive/Products/Applications/Momentum.app"
  python3 macos/scripts/check-testflight-archive.py "$app" "$version" "$build"
  /usr/bin/codesign --verify --strict "$app"
  /usr/bin/codesign -d --entitlements :- "$app" > "$work/entitlements.plist"
  for entitlement in app-sandbox files.user-selected.read-write network.client network.server personal-information.calendars; do
    test "$(/usr/libexec/PlistBuddy -c "Print :com.apple.security.$entitlement" "$work/entitlements.plist")" = true
  done

  /usr/bin/python3 - "$work/ExportOptions.plist" "$APPLE_TEAM_ID" <<'PY'
import plistlib
import sys
with open(sys.argv[1], "wb") as output:
    plistlib.dump({
        "method": "app-store-connect",
        "destination": "upload",
        "signingStyle": "automatic",
        "teamID": sys.argv[2],
        "manageAppVersionAndBuildNumber": False,
        "testFlightInternalTestingOnly": True,
    }, output)
PY
  python3 macos/scripts/build-env.py xcodebuild -exportArchive \
    -archivePath "$work/Momentum.xcarchive" -exportPath "$work/export" \
    -exportOptionsPlist "$work/ExportOptions.plist" "${signing[@]}"
else
  printf 'Mac build %s (%s) already has an App Store Connect record. Checking its status.\n' "$version" "$build"
fi

asc builds wait "${selector[@]}" --timeout 20m --fail-on-invalid
builds=$(asc builds list "${selector[@]}" --processing-state VALID --limit 2)
build_id=$(jq -er 'select(.data | length == 1) | .data[0].id' <<< "$builds")
if ! asc testflight groups list --build-id "$build_id" |
     jq -e --arg id "$TESTFLIGHT_INTERNAL_GROUP_ID" '.complete and any(.groups[]; .id == $id)' >/dev/null; then
  asc builds add-groups --build-id "$build_id" --group "$TESTFLIGHT_INTERNAL_GROUP_ID"
  asc testflight groups list --build-id "$build_id" |
    jq -e --arg id "$TESTFLIGHT_INTERNAL_GROUP_ID" '.complete and any(.groups[]; .id == $id)' >/dev/null
fi
asc builds build-beta-detail view --build-id "$build_id" |
  jq -e '.data.attributes.internalBuildState == "IN_BETA_TESTING"'
