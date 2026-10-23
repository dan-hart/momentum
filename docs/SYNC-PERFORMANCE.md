<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!-- Copyright (C) 2026 Dan Hart -->
# Sync performance and recovery

## 2026-10-23 client acceleration (F-075)

F-075 adds nine client improvements across Linux, macOS, iOS and Android. Server
administration/tuning is deferred at the user's request. No measured speed multiplier
is claimed; real-server latency and bandwidth depend on the installation and workload.

### Existing connections

Legacy v2 remains the default. Native clients share a 750 ms first-edit coalescing
window, followed by another exchange if edits arrive while one is running. Foreground
reconciliation uses 5 seconds during recent activity, then 15 seconds, then 150 seconds
while idle. Native power, foreground/background and selected-provider rules still apply.
OS background scheduling is not a guarantee of a particular wake interval.

The shared transport reuses a bounded HTTP connection pool. Conditional downloads
use an exact validated baseline even with pending edits; an encrypted baseline sidecar
preserves encryption at rest and never stores credentials. Changed configuration,
corrupt cache, conflicts and failed validation invalidate cached knowledge. Task and
optional settings exchanges run independently within one provider lease and deadline;
settings consent and review remain separate. Unchanged legacy polls validate before
cloning the full task store. Concurrent local edits remain pending for the next exchange.

If the server already advertises Nextcloud Client Push, foreground clients discover
its endpoint and listen for best-effort invalidation hints. Only same-origin validated
HTTPS/WSS is accepted; credentials never follow discovery or websocket redirects.
Notifications coalesce and trigger ordinary authoritative synchronization. Missing
capabilities, disconnects and unsupported hosting retain periodic reconciliation.
Changing providers, backgrounding and power restrictions stop native push work.
No server installation, APNs, FCM or proprietary Momentum service is required.
See [Nextcloud's protocol](https://github.com/nextcloud/notify_push/blob/main/DEVELOPING.md).

### Optional optimized collection

Sync settings offer an explicit **Upgrade** action with an explanation on each native
platform. Consent is specific to the server, account and folder. Upgrade every device
participating in that collection; older apps continue seeing the retained legacy file,
which is no longer the authoritative optimized dataset. Do not mix legacy and optimized
writers expecting them to synchronize with each other.

Upgrade creates a separate `momentum-v3` collection, retaining the original legacy
file. It is one-way for that connection: a durable local marker is recorded before
attempting optimized sync, including uncertain failures. Turning sync Off remains
available. A downgrade requires a future verified migration flow; clearing preferences
or restoring a task backup does not authorize a silent downgrade.

Optimized sync publishes immutable operation batches and periodically makes checkpoints,
with historical archives referenced separately. Manifest append order defines replay;
stable operation IDs prevent duplicate actions. Objects are verified before a
strong-ETag compare-and-swap manifest publication. Interrupted writes leave unreferenced
objects and retain local pending edits. No destructive garbage collection is added.
An initialized collection that loses its manifest fails closed instead of importing a
stale legacy copy. Independent acknowledged manifest receipts preserve unchanged-poll,
restore and undo behavior without changing legacy cursors.

Encrypted optimized objects use a versioned Argon2-protected vault-key envelope and
fresh AES-GCM nonces with collection/type/object context authentication. A one-entry,
expiring in-memory key cache amortizes password derivation; invalidation and lifecycle
changes release idle keys. Keys are never persisted in plaintext. Existing legacy
ciphertexts and password handling remain compatible.

Verified folder copying reconstructs committed optimized data, encrypts it for the
new collection identity, checks source stability and refuses conflicting destinations.
Optional settings are copied too. Source data and local pending edits remain intact;
local preferences change only after verified success. A failed partial copy may need
a different empty destination; it never authorizes overwriting an existing collection.

### Verification boundaries

Regression tests use disposable stores and mock HTTP/TLS/WebSocket servers. They cover
pool reuse, conditional/restart caching, pending edits, CAS conflicts, uncertain uploads,
ordered replay, checkpoints/archives, wrong keys, malformed objects, restore, undo,
folder copying, native scheduling/consent and push authentication/cancellation/fallback.
The dated F-075 entry in [PROGRESS.md](PROGRESS.md) records actual platform checks.
Real Nextcloud installations, suspended-device timing, physical-device acceptance and
the proposed 10× target require separate measurement.

## Historical 2026-10-10 evidence (F-058)

The following records its original checkout and measured scope. Its cache/scheduling
and Android availability descriptions are historical; F-075 above describes the current design.

F-058, B-100–B-103. Evidence recorded 2026-10-10 on the dirty
`task/sync-performance-recovery` checkout based on `200f686`. Local changes only.
All fixtures use temporary task stores and loopback servers, with synthetic contents.

## Scope and measured targets

The request is 10× faster sync, 2× reliability and 2× recovery across platforms.
These are acceptance targets for named workloads, not universal production claims.
The same Rust transport and orchestration feed Linux, macOS and iOS; Android has no
native client in this checkout. Its implementation and device verification remain Planned.

| Workload | Reference | Revised | Result |
|---|---|---|---|
| Ten complete shared-engine idle polls, 10,000 tasks, compression and encryption enabled | Uncached body polls, with the new metadata commit: 1.582 s | Validated polls, including store cloning and durable metadata commit: 136.146 ms | 11.6× faster complete engine path |
| Ten idle Nextcloud exchanges, 10,000 tasks, compression and encryption enabled | Full GET/decrypt/decompress/parse each time: 2.505 s | Validated conditional GET: 28.729 ms | 87.2× faster transport; body bytes 6,852,680 → 0 |
| Publish 1,000 offline LibreSync operations | Original per-operation journal serialization/rename: 13.235 s | One staged, durable batch: 54.271 ms | 243.9× faster publication |
| Twenty exchanges: ten healthy, ten with one injected HTTP 503 | Original single-attempt GET succeeds 10/20 | Bounded retry succeeds 20/20 | 2× successful exchanges in this specified fault matrix |
| Recovery from that single transient 503 | Failure exits the exchange; idle native polling is every 150 s | Worst same-exchange recovery: 262.698 ms | Under 0.3 s in the fixture; avoids waiting for another poll |

Timings are one local debug-profile sample on an Apple Silicon Mac. The engine reference
alternates otherwise equivalent output-compression preferences to invalidate its session;
both sides use the optimized metadata commit, making this a conservative comparison to
the old full-store save. The engine test includes admission, snapshot cloning, encrypted
HTTP exchange and durable timestamp persistence. It excludes Swift/GTK presentation,
whole-app startup and real network latency. The separate transport sample covers `momentum-sync`
without the engine clone or commit. LibreSync timing covers backlog publication, not discovery,
pairing, snapshot transfer or whole-engine incoming reconciliation. The reference
publication loop retains its original cheaper unsynced write; the new path also fsyncs
the file and parent directory. There is no production reliability-rate or physical-device
energy claim. Manual retry and the 10 s edit debounce can recover earlier than idle polling.
The 150 s comparison is existing scheduling policy, not a timed real-server outage.

## Behavior

- An engine retains a validated Nextcloud configuration/version/ETag in memory. Only
  unchanged stores with no pending edits use conditional GET. Credentials, encryption
  options, destination, version or validator changes require the body again. Restarting
  the process starts without a validation cache.
- A no-change exchange persists only the successful-sync timestamp. It preserves task
  files, pending edits, vector clocks, undo and cached indexes, including edits made while
  the request was in flight. The existing owner, generation and cancellation barriers stay.
- GET and upload-verification PROPFIND retry eligible transient errors up to three attempts,
  with 250/500 ms backoff inside the original total deadline. Cancellation interrupts the
  backoff. Authentication, TLS and damaged data fail immediately. Uncertain PUTs are not
  automatically replayed.
- Existing remote files require a strong ETag for conditional overwrite. A successful PUT
  must have a matching stored size and matching PUT/PROPFIND validators before local edits
  are acknowledged. Missing or changed validators retain pending edits and report recovery.
  Same-version remote replacements are adopted when their ETag changes.
- LibreSync publishes pending operations in one journal transaction. Inbound receipts,
  snapshot publication and pruning publish their memory changes only after persistence.
  Failed saves stay retryable; unchanged adapter loads skip journal writes. Error callbacks
  run after releasing the engine/provider locks.
- Invalid gzip is classified as a damaged remote copy, enabling existing replacement
  recovery. English/German upload errors accurately say that local edits remain queued.

No persisted format, provider policy, dependency, account requirement or sync interval
changed. The validation cache contains no new logged or persisted credentials.

## Repeat the evidence

```sh
cargo test --workspace --exclude momentum --all-features --all-targets
cargo test -p momentum-sync benchmark_ -- --ignored --nocapture --test-threads=1
cargo test -p momentum-p2p benchmark_offline_publish -- --ignored --nocapture
cargo test -p momentum-core --all-features benchmark_idle_engine_exchange -- --ignored --nocapture
cargo check -p momentum --tests
python3 ios/scripts/test.py prepare
swift test --package-path macos/Packages/MomentumKit
python3 ios/scripts/test.py fast
python3 ios/scripts/test.py transport \
  --destination 'platform=iOS Simulator,id=YOUR_TEST_SIMULATOR'
```

The ignored benchmarks stay out of ordinary test runs. Regression tests cover failed
verification, mismatched validators, same-version replacements, transient reads,
cancellation/deadlines, concurrent edits, metadata persistence failures, failed journal
receipts, snapshot retry and callback reentrancy. The iOS transport lane exercises real
Swift → UniFFI → Rust → HTTP/disk and two real LibreSync peers in isolated stores.
See [PROGRESS.md](PROGRESS.md) for completed platform checks and remaining boundaries.

## Remaining platform acceptance

Linux requires GNOME runtime checks on a supported Linux environment. Apple platforms
require disposable hosted Nextcloud/TLS and separate-device LibreSync acceptance for
network behavior outside loopback, plus physical-device/background and battery evidence.
The changed recovery text still needs spoken accessibility acceptance. Existing unrelated
deferred checks are not resumed by this change.

Android must use the same core through native bindings, platform credential storage,
one selected provider, offline persistence, actionable status/retry, and lifecycle-aware
background limits. Repeat the fault matrix and native two-device convergence there when
its approved frontend exists; Rust source portability alone is not Android app verification.
The shared Rust/UniFFI code checks for `aarch64-linux-android` with installed Rust 1.91.1
and NDK 27.2/API-26 compiler tooling; that test-tool choice does not establish the eventual
Android app's minimum SDK, library linkage or runtime behavior.

Protocol references: [HTTP conditional requests](https://www.rfc-editor.org/rfc/rfc9110.html)
and [Nextcloud WebDAV properties and response validators](https://docs.nextcloud.com/server/stable/developer_manual/client_apis/WebDAV/basic.html).
