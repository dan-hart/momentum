# iOS Sync, Feedback, and List Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add truthful connection tests, compact sync metadata, a space-efficient floating Add Task control, transient Liquid Glass operation feedback, and colorful counted project/tag rows to Momentum on iOS 26 and 27.

**Architecture:** Rust owns read-only WebDAV probing, attributable LibreSync cycle identity, and canonical task-family counts. MomentumMobile owns cancellable presentation state and off-main transport bridges. SwiftUI owns native layout, Liquid Glass, timing, Dynamic Type, accessibility, and provider-specific copy. Generated UniFFI code is refreshed only through the existing tooling.

**Tech Stack:** Rust (`sp-sync`, `momentum-core`, UniFFI), Swift 6, SwiftUI/UIKit public hosting tests, Swift Testing/XCTest, iOS 26/27 Liquid Glass APIs, existing Python iOS runner.

**Working constraints:** Preserve the inherited dirty worktree. Do not add XCUITest/ViewInspector. Do not use personal data or credentials. Do not commit, push, clean, reset, or release; repository instructions require local changes unless explicitly authorized.

**Design:** `docs/superpowers/specs/2026-09-19-ios-sync-feedback-list-polish-design.md`

---

## File map

- `crates/sp-sync/src/lib.rs`: read-only guarded WebDAV connection probe.
- `crates/sp-sync/src/mock_dav.rs`, `crates/sp-sync/src/feature_tests.rs`: probe transport fixtures and behavior.
- `crates/momentum-core/src/engine.rs`, `types.rs`, `listing.rs`, `p2p.rs`: UniFFI probe, sidebar family counts, attributable LibreSync cycle IDs.
- `crates/momentum-core/src/tests.rs`, `transport_tests.rs`: shared rules and transport regressions.
- `crates/app/src/p2p.rs`, `crates/app/src/application.rs`: exhaustive Linux event/admission consumption.
- `macos/Packages/MomentumKit/Sources/MomentumKit/AppState.swift`, `Services.swift`, and `Tests/MomentumKitTests/NearbySyncTests.swift`: exhaustive macOS event/admission consumption.
- `ios/Packages/MomentumMobile/Sources/MomentumMobile/NextcloudOperation.swift`: off-main probe operation.
- `ios/Packages/MomentumMobile/Sources/MomentumMobile/CoreNearbyRuntime.swift`: typed core P2P admission result.
- `ios/Packages/MomentumMobile/Sources/MomentumMobile/NextcloudSyncState.swift`: provider connection-test state and cancellation.
- `ios/Packages/MomentumMobile/Sources/MomentumMobile/NearbyLifecycle.swift`: exact-cycle LibreSync wait/join logic.
- `ios/Packages/MomentumMobile/Sources/MomentumMobile/EngineWorker.swift`: exact-batch Undo bridge if needed by root feedback.
- `ios/Packages/MomentumMobile/Tests/MomentumMobileTests/NextcloudSyncStateTests.swift`, `NearbyLifecycleTests.swift`, and organization tests: fast portable state/count coverage.
- `ios/Momentum/Settings/SyncSettings.swift`: Test Connection UI and compact reusable sync status view.
- `ios/Momentum/Tasks/TaskScreen.swift`: bottom footer and floating Add Task overlay.
- `ios/Momentum/Tasks/ListPicker.swift`: sidebar entry rows with filled symbols and counts.
- `ios/Momentum/App/MobileAppModel.swift`: typed, uniquely identified feedback and exact-batch Undo.
- Create `ios/Momentum/Components/FeedbackToast.swift`: root-owned transient/persistent Liquid Glass presentation.
- `ios/Momentum/App/RootView.swift`: one feedback overlay and timeout owner.
- `ios/MomentumViewTests/*.swift`: public `UIHostingController` integration coverage.
- `ios/MomentumTransportTests/LoopbackDAV.swift`, `NextcloudTransportTests.swift`: real iOS/UniFFI/HTTP probe coverage.
- Create `ios/scripts/check-count-localization.swift` and `ios/scripts/tests/test_count_catalog.py`: compiled iOS English/German count-plural regression.
- `ios/Momentum/Resources/Localizable.xcstrings`: new English/German strings and plurals.
- `docs/FEATURES.md`, `docs/BUGS.md`, `docs/PROGRESS.md`, `docs/IOS-PARITY-CHECKLIST.md`, `ios/README.md`: F-043/B-084–B-088 implementation, superseded B-016/F-017 behavior, user guidance, and evidence.

### Task 1: Read-only Nextcloud probe

**Files:**
- Modify: `crates/sp-sync/src/lib.rs`
- Modify: `crates/sp-sync/src/mock_dav.rs`
- Modify: `crates/sp-sync/src/feature_tests.rs`
- Modify: `crates/momentum-core/src/engine.rs`
- Modify: `crates/momentum-core/src/transport_tests.rs`

- [ ] **Step 1: Write failing transport tests**

Add tests that call the wished-for `Engine::test_nextcloud_connection_cancellable` and assert: authenticated existing collection succeeds; an authenticated missing collection succeeds after probing the account root; bad credentials fail; cancellation/deadline fail through an injected short deadline; store bytes, pending operations, and sync timestamps are unchanged.

- [ ] **Step 2: Run the focused Rust tests and verify RED**

Run: `cargo test -p momentum-core transport_tests::nextcloud_connection`

Expected: compile failure because the probe API does not exist.

- [ ] **Step 3: Implement the minimal transport probe**

Add `sp_sync::probe_guarded` using the existing `NextcloudCfg`, authorization header, agent deadline, cancellation guard, and an injected timeout so tests never wait on a production deadline. Export `Engine.test_nextcloud_connection_cancellable(settings, cancellation, timeout_ms)` through UniFFI; production Swift passes `30_000`, while the real native transport deadline test passes a small value through the same Swift → UniFFI → Rust → HTTP path. Probe the configured collection with read-only WebDAV `PROPFIND`; on collection 404, probe the authenticated account root and accept success because normal sync may create the collection. Map other statuses through existing `SyncError`/`CoreError` conversion. The method does not acquire or mutate the task-store sync lease.

- [ ] **Step 4: Run focused tests and verify GREEN**

Run the focused command from Step 2 and `cargo test -p sp-sync`.

Expected: all probe tests pass and the no-mutation assertions hold.

- [ ] **Step 5: Review the scoped diff**

Confirm no write request is issued and no existing sync behavior changed.

### Task 2: Canonical sidebar family counts

**Files:**
- Modify: `crates/momentum-core/src/types.rs`
- Modify: `crates/momentum-core/src/listing.rs`
- Modify: `crates/momentum-core/src/tests.rs`

- [ ] **Step 1: Write failing sidebar count tests**

Add assertions for project zero/one counts, completed-root exclusion, archived-root exclusion, child-only tag membership, parent-plus-child de-duplication, and completed-root exclusion with inconsistent child completion.

- [ ] **Step 2: Run focused tests and verify RED**

Run: `cargo test -p momentum-core sidebar_`

Expected: failure because `SidebarEntry.task_count` is absent.

- [ ] **Step 3: Implement one family-count helper**

Map each direct context member to its root parent ID, de-duplicate roots, and count only live unfinished roots. Add `task_count: u32` to `SidebarEntry` and populate fixed entries with zero and project/tag entries with the helper.

- [ ] **Step 4: Run focused tests and verify GREEN**

Run the focused command and `cargo test -p momentum-core`.

- [ ] **Step 5: Review the scoped diff**

Confirm counts are derived once from the store and no Swift-side rule is introduced.

### Task 3: Attributable LibreSync cycles and desktop consumers

**Files:**
- Modify: `crates/momentum-core/src/p2p.rs`
- Modify: `crates/momentum-core/src/transport_tests.rs`
- Modify: `crates/sp-p2p/src/feature_tests.rs` if the ticket seam needs lower-level coverage
- Modify: `crates/app/src/p2p.rs`
- Modify: `crates/app/src/application.rs`
- Modify: `macos/Packages/MomentumKit/Sources/MomentumKit/AppState.swift`
- Modify: `macos/Packages/MomentumKit/Sources/MomentumKit/Services.swift`
- Modify: `macos/Packages/MomentumKit/Tests/MomentumKitTests/NearbySyncTests.swift`

- [ ] **Step 1: Write failing cycle-attribution tests**

Cover a newly requested outbound cycle, joining the active outbound cycle, no running provider, no linked peer, immediate `sync_all` failure, and unrelated inbound/state-save events that must not complete the requested test.

- [ ] **Step 2: Run focused tests and verify RED**

Run: `cargo test -p momentum-core p2p_ --features p2p`

Expected: failure because sync-now returns no cycle ID and events carry none.

- [ ] **Step 3: Add explicit ticket identity and join semantics**

Make `p2p_sync_now` return `Result<u64, CoreError>`: a new or joined outbound ticket on admission, and a typed error for no runtime, no linked peers, or immediate `sync_all` failure. Include `cycle_id` on outbound started/completed events; unattributed inbound/state-save failures keep no ID. Preserve existing automatic sync behavior. Update Linux and macOS consumers exhaustively; their manual Sync actions may ignore a successful ticket but must continue surfacing immediate errors.

- [ ] **Step 4: Run focused tests and verify GREEN**

Run the focused command, the existing two-peer P2P tests, `cargo check -p momentum --all-features`, and focused MomentumKit nearby tests.

- [ ] **Step 5: Review the scoped diff**

Confirm cycle IDs cannot be satisfied by inbound events and existing consumers remain exhaustive.

### Task 4: Regenerate UniFFI and add mobile connection-test state

**Files:**
- Generated via tooling: `macos/Packages/MomentumCore/Sources/MomentumCore/momentum.swift`
- Modify: `ios/Packages/MomentumMobile/Sources/MomentumMobile/NextcloudOperation.swift`
- Modify: `ios/Packages/MomentumMobile/Sources/MomentumMobile/NextcloudSyncState.swift`
- Modify: `ios/Packages/MomentumMobile/Sources/MomentumMobile/NearbyLifecycle.swift`
- Modify: `ios/Packages/MomentumMobile/Sources/MomentumMobile/CoreNearbyRuntime.swift`
- Modify: `ios/Packages/MomentumMobile/Tests/MomentumMobileTests/NextcloudSyncStateTests.swift`
- Modify: `ios/Packages/MomentumMobile/Tests/MomentumMobileTests/NearbyLifecycleTests.swift`
- Modify: `ios/MomentumTransportTests/LoopbackDAV.swift`
- Modify: `ios/MomentumTransportTests/NextcloudTransportTests.swift`

- [ ] **Step 1: Regenerate bindings through the normal preparation tool**

Run: `python3 ios/scripts/test.py prepare`

Expected: generated Swift reflects the Rust records/methods/events; no generated source is hand-edited.

- [ ] **Step 2: Write failing Swift state and native transport tests**

Specify `ConnectionTestState` idle/testing/success/failure, double-tap exclusion, Nextcloud cancellation, saved-only eligibility, provider-change reset, LibreSync exact-cycle success/failure/timeout, join, no runtime/no peer/immediate failure, and ignored unrelated events. Add the race where a keyed completion arrives before the caller begins awaiting it. Extend `LoopbackDAV` with authenticated `PROPFIND`, collection/root routing, held responses, and request counters. Add native tests for success, missing collection fallback, auth failure, injected short deadline, cancellation, and unchanged store/status.

- [ ] **Step 3: Run focused package tests and verify RED**

Run:

```bash
swift test --package-path ios/Packages/MomentumMobile --filter Connection
python3 ios/scripts/test.py transport \
  --destination 'platform=iOS Simulator,id=681FF376-73A1-4EFC-807E-E858C7B5B7C2' \
  --filter NextcloudTransportTests
```

Expected: compile/test failure because the state API is absent.

- [ ] **Step 4: Implement minimal state and operation bridges**

Run the core probe in a detached utility task with cooperative cancellation. Add one state enum to `NextcloudSyncState`; delegate LibreSync waiting to `NearbyLifecycle` with a bounded cancellable continuation keyed by cycle ID. Cache a bounded terminal result if the event wins the call-return race, then consume it exactly once. Inject the timeout/sleep seam for deterministic tests. Clear results on provider change and cancellation/backgrounding.

- [ ] **Step 5: Run focused tests and verify GREEN**

Run focused Nextcloud and Nearby lifecycle suites, the focused native probe tests, then the full MomentumMobile package suite. Repeat the focused native probe tests on iOS 26.5.

- [ ] **Step 6: Review the scoped diff**

Confirm test results do not overwrite persistent automatic-sync failures and no polling was added.

### Task 5: Typed feedback model and exact-batch Undo

**Files:**
- Modify: `ios/Momentum/App/MobileAppModel.swift`
- Modify: `ios/Packages/MomentumMobile/Sources/MomentumMobile/EngineWorker.swift`
- Modify: `ios/MomentumViewTests/TaskMutationIntegrationTests.swift`

- [ ] **Step 1: Write failing model tests**

Assert unique IDs for identical repeated messages, 3.5-second informational classification, 5-second undoable classification with the exact batch ID, persistent `SaveFailed`, replacement semantics, announcement behavior, dismiss, and exact-batch Undo after a later mutation. Inject the feedback sleep/clock so expiration advances through controlled continuations without wall-clock waiting.

- [ ] **Step 2: Run the focused native tests and verify RED**

Run: `python3 ios/scripts/test.py views --destination 'platform=iOS Simulator,id=681FF376-73A1-4EFC-807E-E858C7B5B7C2' --filter TaskMutationIntegrationTests`

Expected: failure because feedback is currently a `String` and the undo ID is discarded.

- [ ] **Step 3: Implement the typed presentation**

Replace `String?` with a small identifiable feedback value carrying message, symbol/severity, timeout policy, and optional undo ID. Map `SaveFailed` to persistent error. Add worker/model exact-batch Undo using the existing core method. Keep one low-priority announcement per presentation.

- [ ] **Step 4: Run focused tests and verify GREEN**

Run the focused native test on iOS 27 and iOS 26.5.

- [ ] **Step 5: Review the scoped diff**

Confirm errors cannot auto-dismiss and repeated text still restarts presentation.

### Task 6: Root Liquid Glass toast

**Files:**
- Create: `ios/Momentum/Components/FeedbackToast.swift`
- Modify: `ios/Momentum/App/RootView.swift`
- Modify: `ios/Momentum/Tasks/TaskScreen.swift`
- Create: `ios/MomentumViewTests/FeedbackToastTests.swift`
- Modify: `ios/MomentumViewTests/TaskMutationIntegrationTests.swift`

- [ ] **Step 1: Write failing hosted-view tests**

Mount the production root/toast and assert one toast across retained tabs, deterministic automatic timeout, replacement deadline, separate Undo accessibility action, persistent/focused save error with Dismiss, and readable Reduce Transparency/Increase Contrast variants. Cover the race where Undo of presentation A finishes after presentation B appears: only A may be dismissed, and B plus the Undo result remain intact. Assert the toast frame clears the tab bar and floating Add control.

- [ ] **Step 2: Run focused native tests and verify RED**

Run: `python3 ios/scripts/test.py views --destination 'platform=iOS Simulator,id=681FF376-73A1-4EFC-807E-E858C7B5B7C2' --filter FeedbackToastTests`

Expected: tests fail because feedback is still rendered per `TaskScreen` and no glass toast exists.

- [ ] **Step 3: Implement the root overlay**

Create a compact rounded toast using the iOS 26 Liquid Glass API with semantic solid fallback. RootView owns `.task(id:)` timeout cancellation through the injected feedback sleeper and reduced-motion transitions. Dismiss by matching presentation ID so an old timeout/action cannot clear a replacement. Remove TaskScreen's feedback safe-area inset.

- [ ] **Step 4: Run focused tests and verify GREEN**

Run:

```bash
python3 ios/scripts/test.py views --destination 'platform=iOS Simulator,id=681FF376-73A1-4EFC-807E-E858C7B5B7C2' --filter FeedbackToastTests
python3 ios/scripts/test.py views --destination 'platform=iOS Simulator,id=6C0B4007-CD8B-49DD-BF15-9A887BC80D9D' --filter FeedbackToastTests
```

- [ ] **Step 5: Review the scoped diff**

Confirm no duplicate timer or toast exists per tab and Undo remains independently focusable.

### Task 7: Compact sync footer and floating Add Task overlay

**Files:**
- Modify: `ios/Momentum/Settings/SyncSettings.swift`
- Modify: `ios/Momentum/Tasks/TaskScreen.swift`
- Modify: `ios/MomentumViewTests/SyncSettingsIntegrationTests.swift`
- Create: `ios/MomentumViewTests/TaskScreenChromeTests.swift`

- [ ] **Step 1: Write failing view tests**

Assert Test Connection states/actions, bottom footer copy for never/current/error/syncing, provider-general visibility, a 44-point footer target, absence of the old top status card, intrinsic floating-button overlay bounds, last-row scroll clearance, and stable button geometry before/after the first snapshot.

- [ ] **Step 2: Run focused native tests and verify RED**

Run:

```bash
python3 ios/scripts/test.py views --destination 'platform=iOS Simulator,id=681FF376-73A1-4EFC-807E-E858C7B5B7C2' --filter SyncSettingsIntegrationTests
python3 ios/scripts/test.py views --destination 'platform=iOS Simulator,id=681FF376-73A1-4EFC-807E-E858C7B5B7C2' --filter TaskScreenChromeTests
```

Expected: old large status row and full-width add inset violate assertions.

- [ ] **Step 3: Implement minimal native presentation**

Add provider-specific Test Connection UI, replace `SyncStatusLink` with a caption-style footer, and move ordinary Add Task to a bottom-trailing overlay. Reserve only scroll-content margin; keep selection controls in their full-width safe-area inset.

- [ ] **Step 4: Run focused tests and verify GREEN**

Repeat both focused commands on iOS 26.5 by replacing the destination ID with `6C0B4007-CD8B-49DD-BF15-9A887BC80D9D`; all four invocations must pass.

- [ ] **Step 5: Review the scoped diff**

Confirm empty/populated lists share the continuous grouped canvas and the button does not intercept surrounding space.

### Task 8: Filled colorful list symbols and counts

**Files:**
- Modify: `ios/Momentum/Tasks/ListPicker.swift`
- Create: `ios/MomentumViewTests/ListPickerRenderingTests.swift`
- Modify: `ios/Packages/MomentumMobile/Tests/MomentumMobileTests/OrganizationTests.swift`

- [ ] **Step 1: Write failing rendering tests**

Assert `folder.fill`/`tag.fill`, saved color on the symbol only, zero/nonzero monospaced-digit count text, plural accessibility label, accessibility-size reflow, and secondary-tint fallback for Colorful Labels off, Increase Contrast, and Differentiate Without Color.

- [ ] **Step 2: Run focused tests and verify RED**

Run:

```bash
swift test --package-path ios/Packages/MomentumMobile --filter OrganizationTests
python3 ios/scripts/test.py views --destination 'platform=iOS Simulator,id=681FF376-73A1-4EFC-807E-E858C7B5B7C2' --filter ListPickerRenderingTests
```

Expected: rows still use outline symbols and expose no counts.

- [ ] **Step 3: Render sidebar entries directly**

Iterate `snapshot.sidebar.projects/tags`, derive IDs from typed views, use always-filled symbols, add adaptive count labels, and retain existing edit/drop behavior.

- [ ] **Step 4: Run focused tests and verify GREEN**

Run the package filter plus the ListPicker view filter on iOS 27 and iOS 26.5.

- [ ] **Step 5: Review the scoped diff**

Confirm row text/outline is not colored and every action still targets the correct typed context ID.

### Task 9: Localization and ledgers

**Files:**
- Modify: `ios/Momentum/Resources/Localizable.xcstrings`
- Modify: `docs/FEATURES.md`
- Modify: `docs/BUGS.md`
- Modify: `docs/PROGRESS.md`
- Modify: `docs/IOS-PARITY-CHECKLIST.md`
- Modify: `ios/README.md`
- Create: `ios/scripts/check-count-localization.swift`
- Create: `ios/scripts/tests/test_count_catalog.py`

- [ ] **Step 1: Add English/German strings and plural variants**

Cover Test Connection states, relative/never/error footer text, task-count pluralization, Undo, Dismiss, and persistent save guidance. Add a compiled-resource verifier that compiles the actual iOS catalog and resolves the task-count key through Foundation in English and German for 0, 1, 2, and 99. After catalog edits validate directly; use compiler extraction only if new source keys need catalog synchronization.

- [ ] **Step 2: Run localization checks**

Run: `python3 ios/scripts/localize.py && python3 ios/scripts/tests/test_count_catalog.py && python3 macos/scripts/tests/test_shared_plurals.py`

- [ ] **Step 3: Update F-043 and B-084–B-088**

Record exact behavior, shared/native ownership, current Implemented/Verified evidence, and remaining hosted/device/system limitations without copying the entire spec. Update the existing F-004/F-017/F-019/F-020/F-025/F-026/F-028/F-039 rows and B-016 history where behavior changed, then add F-043 and B-084–B-088. Update `ios/README.md` setup/status guidance for Test Connection and the compact footer.

- [ ] **Step 4: Run Markdown link and diff checks**

Run this read-only changed-link check, then `git diff --check`:

```bash
python3 - <<'PY'
import pathlib, re, sys
root = pathlib.Path.cwd()
paths = [
    "README.md",
    "ios/README.md",
    "docs/FEATURES.md",
    "docs/BUGS.md",
    "docs/PROGRESS.md",
    "docs/IOS-PARITY-CHECKLIST.md",
    "docs/superpowers/specs/2026-09-19-ios-sync-feedback-list-polish-design.md",
    "docs/superpowers/plans/2026-09-19-ios-sync-feedback-list-polish.md",
]
missing = []
for name in paths:
    source = root / name
    if not source.exists():
        continue
    for target in re.findall(r"\[[^]]*\]\(([^)]+)\)", source.read_text()):
        target = target.split("#", 1)[0]
        if not target or "://" in target or target.startswith("mailto:"):
            continue
        if not (source.parent / target).resolve().exists():
            missing.append(f"{name}: {target}")
print("\n".join(missing))
sys.exit(bool(missing))
PY
git diff --check
```

### Task 10: Complete verification and final installation

**Files:**
- Review all files above; no new production scope.

- [ ] **Step 1: Run shared consumers**

Run:

```bash
cargo test --workspace --exclude momentum
cargo test --workspace --exclude momentum --all-features --all-targets
cargo check -p momentum --all-features
swift test --package-path macos/Packages/MomentumKit
```

- [ ] **Step 2: Run both iOS lanes on both runtimes**

Run the repository runner's complete `all` lane on simulator IDs `6C0B4007-CD8B-49DD-BF15-9A887BC80D9D` (iOS 26.5) and `681FF376-73A1-4EFC-807E-E858C7B5B7C2` (iOS 27):

```bash
python3 ios/scripts/test.py all --destination 'platform=iOS Simulator,id=6C0B4007-CD8B-49DD-BF15-9A887BC80D9D'
python3 ios/scripts/test.py all --destination 'platform=iOS Simulator,id=681FF376-73A1-4EFC-807E-E858C7B5B7C2'
```

- [ ] **Step 3: Build the production simulator app**

Run: `xcodebuild -project ios/Momentum.xcodeproj -scheme Momentum -configuration Debug -destination 'generic/platform=iOS Simulator' CODE_SIGNING_ALLOWED=NO build`

- [ ] **Step 4: Inspect the visual matrix**

Use isolated preview/demo data to inspect empty/populated Today, footer, toast, lists, all four tabs, light/dark, Increase Contrast, Reduce Transparency, and AX5 on phone and iPad. Record screenshots/results without claiming spoken assistive acceptance.

- [ ] **Step 5: Run final hygiene and independent code review**

Run `python3 ios/scripts/localize.py`, `python3 ios/scripts/tests/test_count_catalog.py`, `python3 macos/scripts/tests/test_shared_plurals.py`, `python3 -m unittest ios/scripts/tests/test_runner.py`, the changed-Markdown-link helper from Task 9, `cargo fmt --all -- --check`, and `git diff --check`, followed by focused spec/code review. Fix findings and rerun affected gates. Attempt `build-aux/test.sh`; if Flatpak remains unavailable on this Mac, record Linux runtime as unverified rather than a pass.

- [ ] **Step 6: Install the latest signed production build on dip17pm**

With `DEVELOPER_DIR=/Applications/Xcode-27.0.0.app/Contents/Developer`, run:

```bash
xcodebuild -project ios/Momentum.xcodeproj -scheme Momentum -configuration Debug \
  -destination 'generic/platform=iOS' -derivedDataPath ios/DerivedData/Device \
  DEVELOPMENT_TEAM=WRJ8J85K4Q CODE_SIGN_STYLE=Automatic build
xcrun devicectl device install app --device 00008150-00064D313A20401C \
  ios/DerivedData/Device/Build/Products/Debug-iphoneos/Momentum.app
xcrun devicectl device process launch --device 00008150-00064D313A20401C \
  --terminate-existing com.codedbydan.Momentum.ios
xcrun devicectl device info apps --device 00008150-00064D313A20401C \
  --bundle-id com.codedbydan.Momentum.ios
```

Verify process/version readback only. Do not open personal sync settings or inspect task data.

- [ ] **Step 7: Update evidence and hand off**

Record exact test counts, result bundle paths, dirty base revision, device readback, skipped hosted credentials/system checks, and current parity status in the ledgers. Summarize what changed, why, actual verification, and remaining limits.
