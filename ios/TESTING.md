# Fast iOS unit tests

Momentum’s iOS regression suite uses unit tests only. The project contains no
XCUITest target, UI automation source, or `XCUIApplication` dependency.

The portable lane runs with Swift Testing on macOS. The native lane runs XCTest in
a minimal iOS simulator host and uses Apple’s public `UIHostingController` APIs to
measure and render the real SwiftUI source. It does not use ViewInspector, private
view-tree reflection, screenshots as golden files, app launching, or arbitrary sleeps.

## Everyday commands

Run from the repository root:

```sh
export DEVELOPER_DIR=/Applications/Xcode-27.0.0.app/Contents/Developer

# Initially, and after Rust, UniFFI, or Apple core artifacts change.
python3 ios/scripts/test.py prepare

# Fastest logic loop. This is also the default command.
python3 ios/scripts/test.py fast
python3 ios/scripts/test.py fast --filter EditorValidationTests

# Recommended pre-handoff unit lane: code plus rendered SwiftUI.
python3 ios/scripts/test.py unit \
  --destination 'platform=iOS Simulator,id=YOUR_TEST_SIMULATOR'

# Rendered SwiftUI only, optionally focused to one XCTest method.
python3 ios/scripts/test.py views \
  --destination 'platform=iOS Simulator,id=YOUR_TEST_SIMULATOR'
python3 ios/scripts/test.py views \
  --destination 'platform=iOS Simulator,id=YOUR_TEST_SIMULATOR' \
  --filter ViewRenderingTests/testTaskRowWrapsLongTitleRatherThanGrowingPastContainer

# Real loopback HTTP, LibreSync sockets, Keychain, and sync-unit integration.
python3 ios/scripts/test.py transport \
  --destination 'platform=iOS Simulator,id=YOUR_TEST_SIMULATOR'

# Every unit lane: portable code, rendered views, and transport.
python3 ios/scripts/test.py all \
  --destination 'platform=iOS Simulator,id=YOUR_TEST_SIMULATOR'
```

Use `xcrun simctl list devices available` to find a simulator UDID. Native lanes
reject physical-device destinations. Xcode manages simulator startup; the runner
never erases the simulator. The test host has its own bundle identifier, uses local
ad-hoc simulator signing, and never opens a production task store.

## What each lane proves

| Lane | Coverage | Boundary |
|---|---|---|
| `fast` | Shared-core adapters, persistence, undo, search, organization, scheduling, notifications, automation projection, editor validation, backup state, appearance, and sync coordination | Runs portable mobile code on macOS; it does not prove UIKit rendering or OS integrations |
| `views` | Real SwiftUI layout and pixels, four-tab metadata, task rows, Settings labels, Dynamic Type, English/German layouts, semantic color resolution, default orange/white action ink, backup/model integration, mutation wiring, lifecycle boundaries, and Spotlight projections | In-process component and integration tests; it does not tap, swipe, launch another app, or validate spoken assistive technology |
| `unit` | `fast` followed by `views` | Recommended quick regression gate; it intentionally excludes network transport |
| `transport` | Real iOS Rust/HTTP exchange through an ephemeral loopback WebDAV server, Keychain configuration, conflict recovery, cancellation, plus two real LibreSync peers with pairing, bidirectional convergence, restart identity and unlink | Hosted Nextcloud/TLS is opt-in; no background suspension or physical-device claim |
| `all` | `fast`, `views`, and `transport` | Complete automated iOS unit suite |

The native target compiles the production SwiftUI files directly, excluding the app
entry point and delegate. Its host contains only an empty UIKit scene and the real
string catalog. Each model fixture receives a UUID-scoped directory and preferences
domain. Transport fixtures bind only to `127.0.0.1` and remove their temporary
Keychain records and files. The hosted TLS case runs only when URL, username,
password and disposable folder are supplied as `MOMENTUM_NEXTCLOUD_TEST_URL`,
`_USERNAME`, `_PASSWORD`, and `_FOLDER`; it deletes its visible fixture task after
proving two-device convergence, while normal local runs report it as skipped.

## Testing SwiftUI without ViewInspector

`MomentumViewTests/HostingFixture.swift` mounts a production view in a real
`UIHostingController`, supplies explicit appearance, contrast, locale, Dynamic Type,
and content-scale traits, then asks SwiftUI for its fitted size and UIKit for rendered
pixels. Tests assert behavior that users can observe, including:

- wrapping and geometry at compact widths and accessibility text sizes;
- distinct SF Symbol output and white ink over the default `#FF6600` action fill;
- stable tab order, labels, symbols, and accessibility identifiers;
- localized view sizing and prevention of clipped Settings labels;
- model-driven mutations, backup state, sync state, and task-family behavior.

The fixture disables incidental animation for deterministic assertions and removes
its hosted controller and isolated defaults after each render. Explicit trait
overrides keep results independent of the simulator’s current light/dark and Increase
Contrast settings.

Add UI coverage by testing the smallest production component or screen that exposes
the behavior. Assert meaningful geometry, pixels, accessibility metadata, or a state
mutation; constructing a view or checking that an image is non-nil is insufficient.
Use controlled continuations for async behavior and never sleep for presumed completion.

## Deliberate manual boundaries

Unit tests cannot establish gesture recognition, full keyboard routing, system
Shortcuts presentation, notification delivery, Files picker interaction, VoiceOver
speech, Voice Control, Switch Control, Full Keyboard Access, a spatial cross-app
drag gesture, or a system-scheduled background launch. Hosted Nextcloud/TLS runs only
when the explicit disposable-server environment is configured. Verify those flows manually on a disposable simulator when their code
changes, and record the exact scope in the project ledgers. Simulator-only evidence
does not establish physical-device behavior.

No XCUI replacement should re-create a second application driver inside unit tests.
Keep business rules in Rust or portable Swift, keep presentation state injectable,
and keep SwiftUI views small enough to render directly.

## Results, filtering, and coverage

The runner fingerprints Rust sources, manifests, build scripts, generated bindings,
and static libraries. It refuses to test stale core artifacts and fails an empty
filtered selection. Invocations serialize preparation/build operations so generated
artifacts cannot change under a running suite.

Logs, `timings.json`, coverage reports, and `.xcresult` bundles are written under the
ignored `ios/DerivedData/TestReports` directory. Coverage is opt-in because it slows
the edit loop:

```sh
python3 ios/scripts/test.py fast --coverage
python3 ios/scripts/test.py views --coverage \
  --destination 'platform=iOS Simulator,id=YOUR_TEST_SIMULATOR'
```

`fast --coverage` reports production `MomentumMobile` lines and excludes generated
bindings and tests. `views --coverage` retains Xcode’s native report. Coverage counts
executed lines; it is not a substitute for behavioral assertions.

Runner regression tests are host-only and require no simulator:

```sh
python3 -m unittest ios/scripts/tests/test_runner.py
```

That suite also guards the project shape: adding an iOS UI-testing bundle,
`XCUIApplication`, or one of the retired XCUI source directories fails immediately.

## Opt-in task-list performance guard

Large fixtures stay out of the everyday lanes. The dedicated Release scheme imports 1,000
Today tasks, closes the seeding scope, then measures a cold `EngineWorker.openChecked` plus
the first snapshot: persisted-store parsing, startup recurrence checks, actor hop, Rust
listing, sidebar/list metadata and native value projection. It then retains the lightweight
repeated presentation-read metric. The cold load must remain below 100 ms and must not
eagerly build Search/Archive's invalidation-aware index.

```sh
xcodegen generate --spec ios/project.yml
xcodebuild -project ios/Momentum.xcodeproj \
  -scheme MomentumPerformanceTests -configuration Release \
  -destination 'platform=iOS Simulator,id=YOUR_TEST_SIMULATOR' \
  -parallel-testing-enabled NO test
```

Use a simulator dedicated to isolated test data. A simulator result is a repeatable regression
boundary; it is not a physical-device launch or energy measurement.
