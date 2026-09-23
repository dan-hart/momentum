# Momentum for iOS

Native SwiftUI client of the shared Rust engine. Minimum iOS/iPadOS 26; development
uses Xcode 27 and XcodeGen. Implementation is in progress; see the
[parity checklist](../docs/IOS-PARITY-CHECKLIST.md) and [progress ledger](../docs/PROGRESS.md)
for actual verification and remaining features.

Current mobile sync choices are Off, Nextcloud, and LibreSync. Only the selected
provider owns the shared store or runs transport work.

## Build

Use the repository's Rust/LibreSync setup described in the [Mac build guide](../macos/README.md).
Install the Apple targets with rustup:

```sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
./ios/scripts/build-core.sh --debug
xcodegen generate --spec ios/project.yml
xcodebuild -project ios/Momentum.xcodeproj -scheme Momentum \
  -configuration Debug -destination 'generic/platform=iOS Simulator' \
  CODE_SIGNING_ALLOWED=NO build
```

The shared builder regenerates UniFFI and publishes one XCFramework containing Mac,
iOS device, and simulator libraries. The generated Xcode project and binary artifacts
are ignored. Device installation requires normal local signing configuration.

## Isolated verification

Use the [fast iOS testing guide](TESTING.md) for the incremental package lane,
in-process SwiftUI view tests, filtering, timing and optional coverage. The default
command is `python3 ios/scripts/test.py` after one initial `prepare` run. The
recommended quick gate is `python3 ios/scripts/test.py unit --destination
'platform=iOS Simulator,id=YOUR_TEST_SIMULATOR'`; it runs portable code tests and
real SwiftUI rendering without launching the app.

```sh
swift test --package-path ios/Packages/MomentumMobile
python3 ios/scripts/localize.py --sync /path/to/ios-derived-data
```

The iOS project deliberately has no XCUITest targets. Native UI coverage uses XCTest,
`UIHostingController`, production SwiftUI source and isolated stores/preferences in a
minimal simulator host. It does not use ViewInspector or a physical device. `--demo`
continues to provide the separate disposable preview store and must not enable
transport or use personal sync credentials.

## Nextcloud sync

1. Open **Settings → Sync → Sync Provider** and select **Nextcloud**.
2. Enter the server URL, username, app password and sync folder. If your other
   devices encrypt the sync file, enter the same encryption password here.
   Momentum requires HTTPS for remote servers and accepts HTTP only for an isolated
   localhost test server. Login details, query strings and fragments do not belong
   in the server URL.
3. Choose compression and automatic sync as needed, then **Save Connection**.
   **Connection Saved** confirms that the connection details were stored; it does
   not claim that a server exchange succeeded.
4. Choose **Test Connection** to verify the saved server and credentials without
   uploading or downloading tasks. A missing Momentum folder is accepted when the
   authenticated account root is reachable because the first sync can create it.
5. Choose **Sync Now**. Check **Last Successful Sync** and **Pending Changes**.
   If an exchange fails, correct the connection, save it and choose **Try Again**.

When sync is enabled, task lists end with one compact status link such as **Last
synced just now**, **Syncing…**, or **Sync needs attention**. Tap it to open Sync
settings and recovery. Detailed provider, pending-change and linked-device state stays
on the Sync screen instead of occupying primary task-list space.

Connection details are stored in the device Keychain. Automatic sync combines edits
and periodically checks for changes while the app is active. Exchanges stop when
the app leaves the foreground; local edits remain available offline. Selecting
**Off** retains both local tasks and the saved connection.

## LibreSync

1. Open **Settings → Sync → Sync Provider** and select **LibreSync**.
2. Open **Nearby Devices** on both Momentum devices. Each device shows a temporary
   six-digit pairing code and discovers reachable peers while this screen is open.
3. Select the other device, enter its code, and choose **Link**. Linked devices and
   last-sync state remain visible, and **Unlink** requires destructive confirmation.
4. Choose **Test Connection** to start or join an outbound exchange and wait for that
   exact cycle. The result cannot be satisfied by inbound work or a different sync.

LibreSync exchanges encrypted task data directly between linked peers while Momentum
is open. The lifecycle starts only for the selected foreground provider, stops and
drains on provider/background/restore transitions, reuses its Keychain-backed identity,
and coalesces local edits before syncing. Manual **Sync Now** remains available.

Low Power Mode pauses automatic sync and discretionary Spotlight/background-refresh
work. **Sync Now**, task editing and already scheduled local notifications remain
available. Automatic work resumes when Low Power Mode turns off.

Production-source setup/recovery, real Rust HTTP exchanges, and real two-peer
LibreSync sockets have passed on iOS 26.5 and 27 with isolated stores and Keychain
identities. The transport lane also contains an opt-in hosted HTTPS Nextcloud test;
it skips unless all `MOMENTUM_NEXTCLOUD_TEST_*` variables identify a disposable
folder. Physical-device acceptance and spoken VoiceOver checks remain open. Saved-connection layout and
rendered contrast pass the recorded simulator scope; see B-038/B-067 in the
[bug ledger](../docs/BUGS.md).

## Appearance

The default base accent is #FF6600. Custom accent choices come exclusively from
[DHFlatUIColors](https://github.com/dan-hart/DHFlatUIColors), pinned to revision
`821b077fc94ba45422ded8f38ee5b532dbabfd3e`, licensed GPL-3.0. Palette attribution:
[Flat UI Colors](https://flatuicolors.com/). No transitive package dependencies.
The picker offers AsNeeded’s nine curated colors in spectrum order, without country
groupings: Alizarin, Carrot, Orange, Emerald, Turquoise, Peter River, Amethyst,
Pomegranate, and Green Sea. Retained choices keep their saved IDs; retired choices
resolve to Momentum Default. The exact chosen base is shown in Settings. Momentum Default stays exactly
#FF6600 in both light and dark mode, including Increase Contrast, by user request.
Filled default actions use white text and symbols in both appearances. Increase
Contrast is the only exception and uses calculated contrasting ink. Exact orange text on light surfaces is not
a claim of WCAG text contrast. Custom colors adapt
to normal/increased contrast. Tests cover these policies and invalid saved choices. Body text uses semantic system colors, and selection includes a
checkmark and accessibility state.

The iOS icon is the editable [MomentumIcon.icon](Momentum/Resources/MomentumIcon.icon)
Icon Composer document. Its default appearance preserves the orange background and
white checkmark/motion motif. Dark appearance uses Apple's charcoal-to-black gradient
and an exact sRGB #FF6600 checkmark. The system provides edge treatment, corner masks,
and tinted/clear appearances; the artwork fills the background without baked margins.
See [icon design and export notes](Design/README.md) for previews and verification.
App/icon source remains under the repository's GPL-3.0-or-later notices.

## Automation

The iOS target compiles the same App Intents declarations as macOS: Create Task,
Find Tasks, Set Task Completed (including reopen), Plan Tasks for Today and Open
Task. Their parameters, task/project entities and results are shared. Task mutations
run on the app's existing engine actor and refresh visible lists and notifications;
Open Task waits for an in-progress capture/editor instead of replacing its draft.
Mobile actions require authentication. Dated native Shortcuts evidence is retained in
the progress ledger, but its former XCUITest/AppIntentsTesting targets were removed by
the 2026-09-18 testing decision. Current units cover action mapping, shared-engine
ownership and mutations. Shortcuts discovery/presentation, cold/background invocation
and locked-device behavior are manual system-acceptance boundaries.

Both `momentum://` and `superproductivity://` support `add`/`create-task` with
`title`, optional `notes`, `due` (`yyyy-MM-dd`) and comma-separated `tags`, plus
`complete-task?title=…`. Percent-encode parameter values. URL creation follows the
desktop convention: without `due`, the task is unscheduled. Receiving a URL while
Momentum is open preserves the selected tab.
