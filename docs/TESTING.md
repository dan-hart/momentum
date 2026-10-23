<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!-- Copyright (C) 2026 Dan Hart -->
# Testing

Momentum's tests are arranged so that most of them run anywhere Rust runs, and the rest
run on a headless display. Nothing in the suite touches your real data, settings, session
bus or network: every test gets its own directory, settings live in a memory backend,
the session bus address points nowhere, and the Nextcloud tests talk to an in-process
WebDAV stand-in.

## Nextcloud acceleration (F-075/B-176)

Use disposable loopback fixtures; no real account or task store is needed:

```sh
cargo test -p momentum-sync transport_tests
cargo test -p momentum-sync optimized::tests
cargo test -p momentum-sync push::tests
cargo test -p momentum-core optimized_
swift test --package-path macos/Packages/MomentumKit
swift test --package-path ios/Packages/MomentumMobile
```

The sync fixtures include an ephemeral trusted HTTPS/WSS server and negative TLS,
origin, credential logging, restart-cache and slow-frame cancellation checks. The
optimized tests exercise verified immutable publication, replay, archived data,
wrong keys, missing manifests, restore receipts and checked folder copying. Native
suites cover cadence, bounded edit bursts, follow-ups and identity-bound consent.
Run the existing Linux headless harness, Android JVM/build/lint gates and canonical
iOS simulator transport lane as well; compiled Android instrumentation cases need
a separate isolated emulator/device run. See [SYNC-PERFORMANCE.md](SYNC-PERFORMANCE.md)
and [SYNC-PUSH.md](SYNC-PUSH.md) for recovery, privacy and acceptance boundaries.

## Completion-sound assets

`python3 build-aux/check-completion-sounds.py` verifies the ten approved WAV hashes,
short PCM format, distinct clips, matching Linux/Apple/Android copies and selected
resource catalogs. The source revision, samples and arrangements are recorded in
`data/resources/sounds/provenance.json`; [NOTICE.md](../NOTICE.md) records CC0 rights.
Native preference/playback tests cover default Bright Finish, persistence, unknown
choice fallback, off-by-default behavior and silence for undo/reopening. Audio device
output remains a separate native acceptance check.

## Homebrew release packages (F-068/B-161)

Use a clean task branch in the existing trusted tap; preserve inherited changes.
Download final release files and SHA256SUMS into `dist`, then render and validate:

```sh
build-aux/release.py homebrew --assets dist --out "$(brew --repository dan-hart/tap)"
brew style dan-hart/tap/momentum-cli dan-hart/tap/momentum
brew audit --formula --strict --os all --arch all dan-hart/tap/momentum-cli
brew audit --cask --strict dan-hart/tap/momentum
brew ruby build-aux/tests/test_homebrew_packages.rb "$(build-aux/release.py version)" dist
brew fetch --formula dan-hart/tap/momentum-cli
brew fetch --cask dan-hart/tap/momentum
brew test dan-hart/tap/momentum-cli # requires the current formula installed
```

The consumer check uses Homebrew's actual loader with simulated Mac/Linux ARM/Intel CLI and
both Mac cask selections, verifying inferred versions, release URLs and digests
against the asset bytes. These checks do not launch the app or inspect task data.
Native Linux Homebrew installation and Mac app launch need separate evidence.

## Release-channel automation (F-070/B-164)

```sh
python3 -m pip install PyYAML==6.0.2
python3 -m unittest discover -s build-aux/tests -p 'test_release_channels.py' -v
actionlint -shellcheck='' .github/workflows/release.yml .github/workflows/install-channels.yml .github/workflows/ci.yml
```

The parser is a test-only dependency. These regressions use fake assets and real
isolated temporary Git repositories: missing credentials without disclosure,
checksum corruption, dirty/older guards, commit/push failures, failed-push retry,
idempotency and workflow dependencies/permissions. They run in the small CI lane
and again in the shared publisher job before the scoped credential checkout.

A manual `Install channels` run against an existing release checks actual SSH
write authentication, final asset bytes, built-in repository/Pages permissions
and both public signed architecture versions without rebuilding apps. The exact-source
102a978 run [38488293659](https://github.com/dan-hart/momentum/actions/runs/38488293659)
passes all 23 regressions and both jobs against 0.4.11; Pages subsequently reports built. Use the
command in [RELEASING.md](RELEASING.md#what-the-workflow-does). Native Linux
OSTree positive/negative controls separately exercise stale versions, unexpected
keys and downgrade protection. Record the actual Actions revision/run and public
channel refs in PROGRESS; this scope does not establish a new native build.

## Run it

```sh
build-aux/test.sh                 # everything, inside the GNOME SDK sandbox (recommended)
build-aux/test.sh -p momentum-oplog     # arguments go to cargo test
cargo test --workspace --exclude momentum   # core crates only, on any OS, no GTK needed
swift test --package-path macos/Packages/MomentumKit   # the macOS app's own tests
python3 macos/scripts/tests/test_repeat_catalogs.py    # compiled Apple plural resources (Xcode required)
python3 macos/scripts/tests/test_shared_plurals.py     # shared message/time/count plurals (Xcode required)
```

Focused Linux-parity checks use the same isolated harness:

```sh
cargo test -p momentum-core task_count -- --nocapture
cargo test -p mo open -- --nocapture
build-aux/test.sh -p momentum typography -- --nocapture
build-aux/test.sh -p momentum background_status -- --nocapture
```

The `mo open` integration tests start a private `dbus-daemon`, create temporary stable
and Devel profile paths under a temporary home, and never address the normal session bus
or a personal task store.

When `lychee` is unavailable, audit relative links and Markdown fragments in every
changed or untracked Markdown file with this read-only fallback:

```sh
python3 - <<'PY'
import re
from pathlib import Path
import subprocess
root = Path.cwd()
tracked = subprocess.run(['git','diff','--name-only','--','*.md'], check=True, text=True, capture_output=True).stdout.splitlines()
untracked = subprocess.run(['git','ls-files','--others','--exclude-standard','--','*.md'], check=True, text=True, capture_output=True).stdout.splitlines()
files = sorted(set(tracked + untracked))
link_re = re.compile(r'(?<!!)\[[^\]]*\]\(([^)]+)\)')
def slug(text):
    text = re.sub(r'<[^>]+>', '', text).strip().lower()
    text = re.sub(r'[^\w\- ]', '', text)
    return re.sub(r'[ ]+', '-', text)
def anchors(path):
    found=set(); seen={}
    for line in path.read_text(encoding='utf-8').splitlines():
        m=re.match(r'^#{1,6}\s+(.+?)\s*#*$', line)
        if not m: continue
        base=slug(m.group(1)); n=seen.get(base,0); seen[base]=n+1
        found.add(base if n==0 else f'{base}-{n}')
    return found
errors=[]; checked=0
for rel in files:
    src=root/rel; in_fence=False
    for lineno,line in enumerate(src.read_text(encoding='utf-8').splitlines(),1):
        if line.lstrip().startswith('```'): in_fence=not in_fence; continue
        if in_fence: continue
        for raw in link_re.findall(line):
            target=raw.strip().split()[0].strip('<>')
            if target.startswith(('http://','https://','mailto:','#')): continue
            pathpart,_,frag=target.partition('#'); dest=(src.parent/pathpart).resolve() if pathpart else src.resolve(); checked+=1
            if not dest.exists(): errors.append(f'{rel}:{lineno}: missing {target}')
            elif frag and dest.suffix.lower()=='.md' and frag not in anchors(dest): errors.append(f'{rel}:{lineno}: missing fragment #{frag} in {dest.relative_to(root)}')
print(f'checked {checked} relative Markdown links across {len(files)} changed files')
if errors: print('\n'.join(errors)); raise SystemExit(1)
print('all relative targets and Markdown fragments resolve')
PY
```

`build-aux/test.sh` enters `org.gnome.Sdk//50` and runs `build-aux/run-tests.sh`, which
compiles the Blueprint UI, the resource bundle and the GSettings schema into
`target/test-assets`, starts `gtk4-broadwayd` when no display is available, and runs
`cargo test --workspace`. On a desktop with a display the app tests use it; set
`MOMENTUM_TEST_BROADWAY=1` to force the headless path. The Flatpak build runs the same
script as its Meson test, so CI executes the UI tests on every push. The whole suite
finishes in a few seconds once compiled.

The repeat-catalog regression compiles the actual iOS and macOS string catalogs with
`xcstringstool`, then resolves their English/German resources through Foundation.
It checks day/week/month/year wording at 0, 1, 2 and 99 without launching an app
or simulator. This catches missing source-language plural forms that catalog
completeness checks alone cannot detect. The test skips on non-macOS hosts.

## Android source and native acceptance

AndroidX Graphics Path JNI is built from the exact pinned 1.1.0 release sources;
run `python3 android/scripts/build_graphics_path.py --verify-source` with the other
source checks. The Python suite covers tampering/unknown inputs and deterministic
JNI-only AAR replacement. See [source provenance and build](../android/vendor/GRAPHICS-PATH.md).
Run `NativeGraphicsPathTest` on API26 as well as the current 16 KiB runtime: it
checks actual native iterator control points and conic-to-quadratic Canvas rendering,
not only library loading. Final APK checks still cover every `.so` LOAD and RELRO
segment plus ZIP alignment. Independent source/build reproducibility remains required.

Use an isolated checkout/worktree, private test stores and disposable credentials.
The [Android guide](../android/README.md) and [toolchain pins](../android/toolchain.json)
specify JDK 17, Rust 1.99.0, stable SDK 37, Build Tools 37.0.0, NDK 27.2.12479018
and the two Rust Android targets. Set `ANDROID_HOME` to the intended SDK; do not
repoint an unrelated checkout's configuration or share its generated bindings/output.
Android builds isolate Cargo output in `android/rust-target` and generated source/native
libraries in `android/app/build/generated`. Never edit generated UniFFI source.

From the repository root, prepare only the pinned public LibreSync input:

```sh
python3 android/scripts/build_core.py --prepare-source
python3 android/scripts/build_core.py --verify-source
python3 android/scripts/build_core.py --bindings-only
python3 -m unittest discover -s android/scripts/tests -v
cd android
./gradlew :app:testDebugUnitTest :app:assembleDebug :app:assembleDebugAndroidTest -PmomentumRustProfile=debug
```

Preparation refuses to replace a mismatched existing `libresync-src`; investigate
that checkout rather than deleting or silently overwriting it. Locked Rust inputs,
archive/tree hashes and host bindings must match. The debug override shortens local
iteration; distribution uses the default release Rust profile. JVM tests use the host
library from this Android output directory, not another frontend's FFI build.

Run native tests only on an explicitly selected owned disposable emulator. Inventory
devices with `adb devices -l`, verify the chosen AVD name, and reserve its port before
launching. Do not send device mutations through an unqualified `adb`, run a connected-device Gradle task against
all attached devices, stop the shared adb server, clear personal stores, or alter
another task's emulator. For example, after creating a uniquely named AVD from the
stable `system-images;android-37.0;google_apis_ps16k;arm64-v8a` image on an arm64 host:

```sh
emulator -avd Momentum_Test_API37_16K -port 5554 -no-snapshot -gpu swiftshader
```

Use `-no-window` only for component instrumentation; native visual/touch/accessibility
acceptance needs an inspectable UI. Use the host-compatible ABI on other machines.
From `android/`, after confirming this is the owned disposable AVD:

```sh
export MOMENTUM_TEST_SERIAL=emulator-5554
adb -s "$MOMENTUM_TEST_SERIAL" emu avd name
adb -s "$MOMENTUM_TEST_SERIAL" shell getprop ro.build.version.sdk
adb -s "$MOMENTUM_TEST_SERIAL" shell getprop ro.product.cpu.abi
adb -s "$MOMENTUM_TEST_SERIAL" shell getconf PAGE_SIZE
adb -s "$MOMENTUM_TEST_SERIAL" shell getprop sys.boot_completed
shasum -a 256 app/build/outputs/apk/debug/app-debug.apk app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk
adb -s "$MOMENTUM_TEST_SERIAL" install -r app/build/outputs/apk/debug/app-debug.apk
adb -s "$MOMENTUM_TEST_SERIAL" install -r app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk
adb -s "$MOMENTUM_TEST_SERIAL" shell am instrument -w com.codedbydan.momentum.test/androidx.test.runner.AndroidJUnitRunner
```

`PAGE_SIZE` must actually be `16384` for 16 KiB runtime evidence. A 4 KiB API 36
emulator or APK alignment pass is a separate check. API 26 minimum-runtime coverage,
both supported ABIs, final release/bundle-produced APKs and a real device remain
separate gates. Tests instantiate disposable stores and credentials; platform
fixtures must not affect the ordinary app's OS notifications or real sync peers.

### Android release tooling

The Python suite also exercises version/SDK/debug metadata refusal, optional/partial
signing setup, private temporary-key permissions, all-asset checksum inventory,
committed-source selection and actual JDK signature verification. The signed-JAR
fixtures use disposable keys and reject a different signer, appended unsigned
payload and altered signed payload. Run with JDK 17 available; a missing JDK is a
skipped signing regression, not verified signing acceptance.

On a clean committed checkout with the pinned toolchain and checksum-verified
bundletool, the release packager builds its own fresh source snapshot:

```sh
python3 android/scripts/package_release.py --out dist/android --bundletool android/rust-target/bundletool.jar
python3 build-aux/release.py checksums --assets dist/android
cd dist/android
shasum -a 256 -c SHA256SUMS.txt
```

The output directory must be empty. Existing APKs and untracked/ignored local source
are never build inputs. This gate runs JVM tests/release lint, checks APK/AAB metadata,
notices, configured signing identity and every AAB payload signature, and checks the
actual bundle-produced APK's 16 KiB native/ZIP alignment. Its temporary validation
key/APK are excluded from distribution. Official-key update continuity, hosted CI,
physical acceptance, F-Droid builds/admission and store uploads remain separate gates.

SDK 37 requires [local network permission](https://developer.android.com/privacy-and-security/local-network-permission)
even for loopback transport fixtures. Use the API-conditional test permission fixture.
For an older already-installed test artifact lacking it, grant only on the selected
owned disposable API 37 emulator before the fixture run:

```sh
adb -s "$MOMENTUM_TEST_SERIAL" shell pm grant com.codedbydan.momentum android.permission.ACCESS_LOCAL_NETWORK
```

Record that grant with the test result. Production permission-denial/recovery behavior
must still be exercised separately; do not loosen production transport validation
to make fixtures pass. Test restricted classes with `am instrument -e class` when
appropriate, but do not present a filtered run as the full suite. App/test artifacts
must come from the same generated core/build; record their hashes, dirty source scope,
date, API/ABI/page size, test filter and result. Do not reinstall while another build
is writing the same output without coordinating artifact ownership.

On macOS 27, the owned API 37 16 KiB AVD initially had `target=android-0` in its
`.ini` metadata. This disabled HVF and produced `qemu_mprotect: Permission denied`
despite a successful hypervisor check. Correcting only that disposable AVD's target
to android-37 enabled boot with the same emulator/image. If this symptom recurs,
inspect image/API metadata, emulator logs and `emulator -accel-check` first; back up
and correct only a demonstrably malformed owned AVD. The suspected SDK parser origin
has not been established, so do not rewrite global SDK package metadata.

The [Android checklist](ANDROID-PARITY-CHECKLIST.md) maps every feature to native gates:
offline restart/process death, full/long editor drafts, failures/stale task recovery,
selection/archive/Undo, actual drag and user-initiated paste/IME, all grouping/sort
choices, English/German 12/24-hour and large-text compact/tablet/landscape rendering,
keyboard/TalkBack/focus/contrast, native document providers, permission denial/regrant,
real notification delivery/cold actions/reboot/time changes, background scheduler and
power-saver behavior, hosted TLS and separate-device LibreSync. Core tests and Compose
semantics do not replace those platform interactions. Preserve earlier user-deferred
desktop/Apple checks instead of rerunning them incidentally.

Release/native alignment checks and F-Droid builder gates are in
[RELEASING.md](RELEASING.md#android-artifacts-and-future-store-distribution).
The [Android workflow](../.github/workflows/android.yml) checks source/JVM/release
APK/AAB/lint/notices/alignment without uploading or publishing; a remote workflow
result is separate from a local pass.

F-022 native typography has separate JVM `NativeTypographyTest` and instrumentation
`TypographySettingsTest`/`NativeToolbarTest` coverage. Verify built-in font selection, independent content
and interface sizes, reset retaining unrelated settings, invalid-scale normalization,
private preference reopen, native slider/radio actions and 48dp minimum controls.
The theme scales `sp` styles before Android conversion, preserves relative `em` values,
and leaves the supplied Density/fontScale unchanged; see
[Android scalable-content guidance](https://developer.android.com/develop/ui/compose/accessibility/scalable-content).
Inspect English/German phone/tablet/landscape with OS and app scales combined above
200%, including editor/import notes, task metadata, settings and selection controls.
`NativeToolbarTest` uses a German configuration with Android context-backed Density at
OS scale 2 and saved app scales 2. It checks actual root/editor/import title layout
bounds, full title semantics, visible 48dp actions and content scrolling. Toolbar height
comes from the actual converted title line rather than multiplying system fontScale.
Its test-only 800dp public WindowInfo fixture selects the actual native rail while
retaining Activity window services and Android's font converter. Check full labels
inside each native target and the real host window, vertical rail placement and
48dp targets; healthy rail expansion is allowed. It does not replace a real tablet
or resize the device under other tests.
These scoped tests do not replace the wider layout, screen-reader and keyboard matrix.

F-019 adds JVM `AccentSchemeTest` coverage for all ten seeds in both native
appearances, contrasting primary/container/fixed roles and swatch selection ink.
`NativeAppearanceTest` checks private preference reopen, wallpaper/accent independence,
labeled palette selection, and F-042's controlled initial-loading→ready transition
with one status and stable navigation/Add bounds. `LocaleResourcesTest` covers
observable English→German date/feedback recomposition and singular/plural/zero grammar.
Run these against fresh artifacts; tests added after gate3 are not covered by that gate.

B-110 records the old Espresso input-injection failure on API 37; current UI tests use
explicit Espresso 3.7.0 and the Compose `junit4.v2` rule. B-111's controlled suspended
task-open/restore and rejected oversized-input cases are separate from actual external
share/deep-link/force-stop inspection. Keep the tested app/test hashes and source scope
when reporting either gate; do not infer a pass for later source changes.

## Layers

| Layer | Where | Needs | Covers |
|---|---|---|---|
| Model | `crates/momentum-model/src/feature_tests.rs` | nothing | dates, repeat rules and catch-up, Today and Overdue membership, unknown-field round trips |
| App core | `crates/momentum-core/src/tests.rs` | tempdir | every behaviour of section 2 through the engine both apps drive: view listings and sections, quick add, day moves, selection and bulk actions, drops and reordering, repeats, reminders, undo, auto-archive, backup, the `mo` socket |
| Op log | `crates/momentum-oplog/src/feature_tests.rs` | nothing | every `Action`: upstream payload, op envelope, vector clocks, `apply` on state |
| Store | `crates/momentum-store/src/feature_tests.rs` | tempdir | persistence, pending queue, peer ops, snapshot adoption |
| Sync | `crates/momentum-sync/src/feature_tests.rs` + `mock_dav.rs` | tempdir, loopback | first upload, download-only, two-client convergence, conflict retry, relayed-op dedup, archives, encryption errors, op cap |
| P2P | `crates/momentum-p2p/src/feature_tests.rs`, `tests/two_devices.rs` | loopback | journal, record adapter, tombstones, snapshots, pairing handler; then two real nodes linking and exchanging |
| CLI | `crates/mo/tests/cli.rs` | the built binary | every `mo` command as a black box, JSON output, the queued ops |
| App logic | `crates/app/src/tests/logic.rs` | GTK thread | formatting, parsing, colours, repeat text, modifier key, demo data |
| macOS logic | `macos/Packages/MomentumKit/Tests` | nothing | the Swift half: navigation and selection order, outcomes becoming toasts and undo, every `Message`, `EmptyState` and `RepeatDescription` turned into words, preferences and the Keychain, URL and command-line handling |
| App UI | `crates/app/src/tests/ui.rs` | display (Broadway) | windows over demo data: views and sections, quick add, completion and undo, archive paging, selection and bulk actions, day moves, drop targets, reorder, sorting, Coming Up, search, context menus, the task form, repeats and catch-up, preferences, modifier accelerators and the shortcuts overlay, quick-add window, search provider, paste/drop, reminders, projects |

The UI tests are a child module of `window` (declared with `#[path]`), so they can call
the same private methods the widgets call. `crates/app/src/tests/support.rs` runs every
app test on one GTK thread (`on_gtk`), gives each test a window over a fresh copy of the
demo store (`demo_window`) or an empty one (`empty_window`), and offers `pump`,
`pump_ms`, `headings`, `row_ids`, `id_of` and `shown` for assertions.

## Writing a test

- Put behaviour in the crate that owns it: a reducer rule belongs in `momentum-oplog`, a
  membership rule in `momentum-model`, and only what needs widgets in `ui.rs`.
- UI tests: create the window, act through the same methods the UI uses (`add_task`,
  `set_done`, `go_to`, actions via `activate_action`), call `pump()` after anything that
  queues work, then assert on the store, `row_ids` and `headings`. Windows are never
  presented, so check visibility with `shown(widget)`.
- Reset any setting you change (`reset_settings()`), even on the failure path if the
  test's later assertions depend on it.
- Keep tests independent of the wall clock beyond "today": derive days from `today_str()`
  and `day_str(day_number(..) + n)`.
- Anything slow (network timeouts, sleeps) is a smell; the sync mock answers instantly
  and the p2p test uses loopback with a 15 s ceiling it never approaches.

## Two platforms, one contract

`crates/momentum-core` is what both apps run on, so its tests are the specification: if
they pass, both platforms agree about what a view contains and what a change does. Each
app then has a thin layer of its own tests for the part only it can get wrong — the GTK
window tests for widgets, the MomentumKit tests for `AppState` and the wording.

Two habits keep the platforms from drifting:

- **Put a behaviour's test in the core.** A rule about which tasks appear, what a change
  writes, or what can be undone belongs in `crates/momentum-core/src/tests.rs`, where it
  covers Linux and macOS at once. Only what needs a widget belongs in a UI test.
- **Keep the wording switches exhaustive.** The core hands over a `Message`, an
  `EmptyState`, a `RepeatDescription`; `crates/app/src/messages.rs` and
  `macos/Packages/MomentumKit/Sources/MomentumKit/Strings.swift` turn them into text with
  no `default` arm. Adding a case to the core therefore stops the other platform
  compiling, instead of quietly showing a blank label.

The macOS tests deliberately use no UI automation. The app's logic lives in the
`MomentumKit` package rather than the app target, so the tests build the real state object
over a throwaway copy of the demo data and drive it directly, with no window, no app
launch and nothing touching the real store, preferences or Keychain. Dated suite
counts and measured timings are recorded in [PROGRESS](PROGRESS.md).

An opt-in socket integration test verifies the real Swift → UniFFI → LibreSync path:

```sh
MOMENTUM_TEST_NEARBY=1 swift test --package-path macos/Packages/MomentumKit \
  --build-system swiftbuild --filter NearbySyncTests
```

It starts two temporary nodes with file-backed keys, pairs over loopback, sends a task,
receives completion in the opposite direction, and unlinks. Assertions inspect the
Swift `AppState` and its recording search index after the native `P2pBridge` callbacks.
The normal test run skips this test, and **CI does not run it at all**: it uses local
sockets *and the transport's discovery service*, and the exchange after pairing never
lands on a hosted runner, though `momentum-p2p`'s own two-device test passes there in under a
second. Run it yourself before a release. It needs no app host and does not touch personal
tasks or Keychain entries. This does not substitute for physical-device discovery or
macOS-to-Linux acceptance.

The mobile adapter has a separate opt-in socket test, using the same generated core:

```sh
python3 ios/scripts/test.py prepare  # after any Rust/FFI source change
MOMENTUM_TEST_NEARBY=1 swift test --package-path ios/Packages/MomentumMobile --filter CoreNearbyRuntimeTests
```

It pairs two disposable stores, exchanges a task and completion in opposite
directions, verifies callback delivery, stops on background, restarts with the saved
peer and unlinks. The ordinary fast suite skips sockets and tests lifecycle races
with controlled async gates. This host test does not prove native Bonjour, local
network permission, physical iOS background behavior or mobile Settings integration.
`SyncBoundaryIntegrationTests` in the native view lane separately exercises real
Security.framework reads, device-only key protection and preservation of malformed
stored credentials on both iOS versions.

`NearbyTransportTests` in the native `transport` lane runs the two-peer lifecycle
on iOS with unique Keychain services. It also checks certificate preservation and
another exchange after restart without pairing again; teardown stops both nodes
before removing only those fixture services and stores. Use `--filter
NearbyTransportTests` with a simulator destination to run this acceptance case.


## macOS MVP acceptance

The handoff's remaining-work checklist and named test counterparts are tracked in
[MACOS-MVP-CHECKLIST.md](MACOS-MVP-CHECKLIST.md). Fast Swift tests now also cover task-form
mapping, subtitle ordering, menu shortcut definitions, drag payloads and batch undo,
localization resources, color accessibility policy, and the remaining named parity gaps.
The shared Rust tests cover selection reorder (including descending and filtered slots),
schedule/reminder restoration, and invalid drag targets on both platforms.

Localization validation is documented in [macos/LOCALIZATION.md](../macos/LOCALIZATION.md).
The read-only macOS accessibility audit is `swift macos/scripts/a11y-audit.swift`; pass
`--self-test` to verify the checker without an app or accessibility permission. A passing
unit suite does not substitute for the live drag/drop and accessibility acceptance checks.

## iOS first-project touch regression

`ios/scripts/test_project_navigation.py` is an opt-in native touch check for B-142.
Hosted view tests exercise accessibility activation; they do not reproduce the
compact navigation transition that cancelled the first project snapshot.
The script requires an explicit, booted disposable iPhone 17 Pro simulator, a
simulator app bundle, the existing AXe CLI and `--confirm-disposable`. It launches
only demo data in English; it never erases a store or contacts a sync account.
It checks first Home selection, selected-project reentry, another project, Today
return and further switches, using one touch per selection and collecting evidence.
It settles native slides before the next touch; this is a correctness regression,
not a timing benchmark. The compact toolbar fallback is scoped to the documented
402 × 874 point viewport; all project rows use their current accessibility labels.

```sh
DEVELOPER_DIR=/Applications/Xcode-26.5.0.app/Contents/Developer \
  python3 ios/scripts/test_project_navigation.py \
  --udid YOUR_DISPOSABLE_SIMULATOR --app /path/to/Simulator/Momentum.app \
  --output /tmp/momentum-project-navigation --confirm-disposable
```

On this host AXe needs Xcode26.5's SimulatorKit, including when targeting the
installed iOS27 simulator. Set `DEVELOPER_DIR` only for that command. This changes
no global Xcode selection. Do not use a personal simulator or physical device.

## iOS accessibility inventory

For the ordinary iOS edit/test loop, see [Fast iOS regression tests](../ios/TESTING.md).
The default portable package suite is independent of simulator startup. The
`MomentumFastTests` unit scheme hosts production SwiftUI views in-process through
`UIHostingController`, without ViewInspector or app automation. Its fixture sets
explicit light/dark, normal/increased contrast, Dynamic Type and locale traits so
results do not inherit mutable simulator settings. Tests assert layout, rendered
pixels, semantic color resolution and stable accessibility metadata.

The project has no XCUITest target. Unit tests do not establish spoken VoiceOver,
Voice Control, Switch Control, Full Keyboard Access, gestures, permission prompts,
system notification delivery or cross-app flows. Check those boundaries manually on
a disposable simulator when relevant code changes, and record the exact scope and
runtime. Do not treat a unit-suite pass as complete assistive-technology acceptance.
See the [dated audit](audits/2026-09-16-ios-accessibility.md) for findings and
verification limits.


## Interrupted store writes

`cargo test -p momentum-store` runs a child process that exits without destructors at every
journal publication/file-replacement/commit boundary. Reopening must return either
all three old files or all three committed files. Additional fixtures cover a failed
later write, interrupted first save, invalid/incomplete recovery records and safe
cleanup. `momentum-core::transport_tests` checks that failed import preserves live
state/undo and `mo` checks its normal JSON error boundary.

The private `.momentum-transaction` directory is an undo record, not a user backup.
Its versioned manifest lists only the existing store filenames. It is published and
synced before any live file is replaced; all replacement files and their parent are
synced before the journal is atomically retired. Opening retries recovery before any
client can write. Only reserved UUID-named preparation/retirement directories are
cleaned up; unknown, malformed or incomplete active journals are preserved. One owner
must serialize writes; this does not introduce cross-process locking or alter the
backup/sync JSON schemas. Hardware power-loss and disk-controller guarantees are not
established by process-exit tests.

`LifecycleTests.failedBootstrapPreservesDataAndAllowsRetry` and the native
`StoreRecoveryIntegrationTests` use isolated data to verify checked iOS startup and
retry. Production never deletes a bad journal on Retry; tests remove only their own
injected corrupt record to model an externally resolved fault. The screen uses Apple's
[ContentUnavailableView](https://developer.apple.com/documentation/swiftui/contentunavailableview)
with a descriptive message and retry action. Component rendering is not VoiceOver or
full native recovery acceptance. Track write-performance regressions separately in
B-023 rather than weakening persistence for fast tests.


## Durable edit transactions

`cargo test -p momentum-core mutation_tests` covers bulk completion/undo, failed save
rollback, tagged creation, organization identities, compound capture/paste, no-op form
saves and repeat-cursor retry. Its observer reads the actual committed store, proving
that sync is awakened only after the complete batch is durable. The manual isolated
sample runs with `cargo test -p momentum-core measure_durable_bulk_edits -- --ignored --nocapture`;
there is no flaky timing threshold, and normal tests never disable disk durability.

Mobile Swift tests cover real failed-save draft retention/retry, notification-action
retry and localized automation errors. `TaskMutationIntegrationTests` exercises the
same app-model/Quick Add path against the real iOS core, checking that a failed write
does not trigger success invalidation or lose the draft. The Mac form adapter returns
its outcome so explicit save failure can retain the sheet and use a native alert,
following Apple's [alert guidance](https://developer.apple.com/design/human-interface-guidelines/alerts).
Native keyboard/dismissal and physical-device performance remain separate acceptance.

## Nearby receive, stop and restore

`cargo test -p momentum-core p2p::tests` uses disposable keys/stores and isolated
runtimes to cover durable received batches, store-write failure with retry, provider
exclusion, concurrent startup, stopped late application, restore/restart epochs and
corrupt-journal preservation. `cargo test -p momentum-p2p --lib` covers non-destructive inbox
reads, exact acknowledgement with newer arrivals and acknowledgement write failure.
It also stalls a local TLS peer to verify that Sync All shutdown cancels active I/O
instead of waiting for its five-second timeout. Discovery retains a two-second bound.

Run `cargo test -p momentum-p2p --test two_devices` for actual local pairing/exchange/bootstrap
compatibility. These checks do not prove native permission handling, physical-device
reachability, background expiration or battery behavior. A restored store is locally
authoritative at replacement; future new remote operations still follow normal sync
rules. The default-zero restore epoch is internal metadata, not a backup migration.

Peer retry receipts are tested across a failed transport acknowledgement, a real
Nextcloud upload that clears pending operations, process reopen and nearby restart.
The same suite covers snapshot-covered operations and receipt pruning when the next
inbox is empty. Receipts are committed with task state; they are not inferred from
vector-clock maxima or the current upload queue.

`transport_tests::explicit_cancellation_preserves_edits_and_allows_a_fresh_exchange`
pauses a real GET, requests cancellation, edits locally and releases the response.
It verifies zero subsequent PUTs, no stale commit/success, durable pending edits and
successful retry. `cancel_nextcloud()` signals the current exchange; callers must
still await it because in-flight HTTP I/O retains the 30-second exchange deadline.
This is not evidence of immediate socket cancellation or iOS background acceptance.

## iOS native backups — F-029

`BackupTests` runs in the fast MomentumMobile package lane. It exercises real Rust
export/restore identity, unrelated JSON rejection, failed-write retry, immutable
coordinated file reads, cancellation, overlapping actions and the distinction between
prepared export bytes and a successful system save. `BackupIntegrationTests` runs
in the native view lane against isolated stores/preferences; it checks task refresh,
selected-tab preservation, draft protection and failure-state cleanup, and attaches
English/German AX5 light/dark renders. System document-picker interaction is a manual
simulator boundary after the removal of XCUITest; unit coverage does not establish
picker cancellation, spoken VoiceOver or third-party file-provider behavior.

The implementation follows Apple's [FileDocument](https://developer.apple.com/documentation/swiftui/filedocument),
[file exporter](https://developer.apple.com/documentation/swiftui/view/fileexporter(ispresented:document:contenttype:defaultfilename:oncompletion:))
and [file importer](https://developer.apple.com/documentation/swiftui/view/fileimporter(ispresented:allowedcontenttypes:allowsmultipleselection:oncompletion:))
interfaces. Import copies coordinated, security-scoped bytes off the main actor,
then requires confirmation before invoking the shared core. Cancellation is not a
reported save/restore success. File-provider interaction, notification delivery and
provider lifecycle checks remain distinct from these fast model tests.

## Queued sync cancellation and checked credentials — F-025/B-031/B-032

`SyncCancellation` is a single-use core object shared across executor admission,
HTTP checkpoints and the final commit boundary. `cancel()` accepts a request only
while queued or running; it cannot retroactively cancel a committing/completed
operation. An accepted cancellation still awaits the in-flight request's bounded
deadline. The real HTTP tests `cancellation_before_executor_admission_never_starts_http`
and `operation_cancellation_drains_the_admitted_exchange_without_upload_or_commit`
verify zero stale upload/commit, preserved pending edits, lease retention and a fresh
subsequent operation. Existing desktop `sync_nextcloud` remains compatible.

`CheckedKeychainTests` injects Security statuses and uses isolated actual items.
`SyncConnectionTests` tests the atomic mobile record, corrupt-record preservation,
exact password handling, already-cancelled Swift tasks and the generated operation
binding. `SyncBoundaryIntegrationTests` reads back real iOS Keychain protection and
checks queued cancellation against the app's isolated engine. These are adapter
checks; the native provider screen, lifecycle and physical locked-device behavior
are not inferred.

The mobile record follows Apple's [Keychain accessibility guidance](https://developer.apple.com/documentation/security/restricting-keychain-item-accessibility)
and [checked update/delete handling](https://developer.apple.com/documentation/security/updating-and-deleting-keychain-items).
It uses After First Unlock This Device Only for the planned bounded background-sync
path, without migrating secrets to another device. Tests never erase a simulator
or access production Keychain accounts.

## Running the Linux app natively on macOS

Homebrew's GTK 4 and libadwaita link the GTK app on macOS, so the Linux UI can be
tried here without a VM: `brew install gtk4 libadwaita adwaita-icon-theme
gobject-introspection` (the icon theme supplies every standard symbolic icon; the app
bundles only its own two), `pip install blueprint-compiler` in a virtualenv, then:

```sh
export DYLD_LIBRARY_PATH=/opt/homebrew/lib GI_TYPELIB_PATH=/opt/homebrew/lib/girepository-1.0
OUT=/tmp/gtk-assets; mkdir -p "$OUT/ui" "$OUT/schemas"
blueprint-compiler batch-compile "$OUT" data/resources data/resources/ui/*.blp
glib-compile-resources --sourcedir="$OUT" --sourcedir=data/resources \
  --target="$OUT/resources.gresource" data/resources/resources.gresource.xml
sed 's/@app-id@/io.github.dan_hart.Momentum.Devel/g; s/@gettext-package@/momentum/g' \
  data/io.github.dan_hart.Momentum.gschema.xml.in > "$OUT/schemas/io.github.dan_hart.Momentum.Devel.gschema.xml"
glib-compile-schemas "$OUT/schemas"
touch crates/app/src/config.rs
MESON_RESOURCES_FILE="$OUT/resources.gresource" cargo build -p momentum
GSETTINGS_SCHEMA_DIR="$OUT/schemas" GSETTINGS_BACKEND=memory MOMENTUM_DEMO=1 target/debug/momentum
```

`cargo check -p momentum --tests` works too, but the GTK UI tests do not run natively:
macOS requires GTK on the process main thread and the harness runs it on a worker
thread. They stay on `build-aux/run-tests.sh` under Broadway (CI's Flatpak jobs).


## Sync performance and failure recovery

The F-058 isolated benchmarks and regression matrix are documented in
[SYNC-PERFORMANCE.md](SYNC-PERFORMANCE.md). Ordinary Rust tests include transport
failures, concurrent edits and failed journal persistence. The expensive 10,000-task
idle-exchange and 1,000-operation publication samples are ignored by default:

```sh
cargo test -p momentum-sync benchmark_ -- --ignored --nocapture --test-threads=1
cargo test -p momentum-p2p benchmark_offline_publish -- --ignored --nocapture
```

Run the [iOS transport lane](../ios/TESTING.md) after regenerating the core to prove
conditional HTTP polling, same-cycle transient recovery and real peer convergence
through Swift/UniFFI. Local fixtures do not replace disposable hosted TLS, Linux runtime,
separate-device, physical/background, or spoken accessibility acceptance.
