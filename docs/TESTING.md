<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!-- Copyright (C) 2026 Dan Hart -->
# Testing

Momentum's tests are arranged so that most of them run anywhere Rust runs, and the rest
run on a headless display. Nothing in the suite touches your real data, settings, session
bus or network: every test gets its own directory, settings live in a memory backend,
the session bus address points nowhere, and the Nextcloud tests talk to an in-process
WebDAV stand-in.

## Run it

```sh
build-aux/test.sh                 # everything, inside the GNOME SDK sandbox (recommended)
build-aux/test.sh -p sp-oplog     # arguments go to cargo test
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

## Layers

| Layer | Where | Needs | Covers |
|---|---|---|---|
| Model | `crates/sp-model/src/feature_tests.rs` | nothing | dates, repeat rules and catch-up, Today and Overdue membership, unknown-field round trips |
| App core | `crates/momentum-core/src/tests.rs` | tempdir | every behaviour of section 2 through the engine both apps drive: view listings and sections, quick add, day moves, selection and bulk actions, drops and reordering, repeats, reminders, undo, auto-archive, backup, the `mo` socket |
| Op log | `crates/sp-oplog/src/feature_tests.rs` | nothing | every `Action`: upstream payload, op envelope, vector clocks, `apply` on state |
| Store | `crates/sp-store/src/feature_tests.rs` | tempdir | persistence, pending queue, peer ops, snapshot adoption |
| Sync | `crates/sp-sync/src/feature_tests.rs` + `mock_dav.rs` | tempdir, loopback | first upload, download-only, two-client convergence, conflict retry, relayed-op dedup, archives, encryption errors, op cap |
| P2P | `crates/sp-p2p/src/feature_tests.rs`, `tests/two_devices.rs` | loopback | journal, record adapter, tombstones, snapshots, pairing handler; then two real nodes linking and exchanging |
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

- Put behaviour in the crate that owns it: a reducer rule belongs in `sp-oplog`, a
  membership rule in `sp-model`, and only what needs widgets in `ui.rs`.
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
lands on a hosted runner, though `sp-p2p`'s own two-device test passes there in under a
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

`cargo test -p sp-store` runs a child process that exits without destructors at every
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
corrupt-journal preservation. `cargo test -p sp-p2p --lib` covers non-destructive inbox
reads, exact acknowledgement with newer arrivals and acknowledgement write failure.
It also stalls a local TLS peer to verify that Sync All shutdown cancels active I/O
instead of waiting for its five-second timeout. Discovery retains a two-second bound.

Run `cargo test -p sp-p2p --test two_devices` for actual local pairing/exchange/bootstrap
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
