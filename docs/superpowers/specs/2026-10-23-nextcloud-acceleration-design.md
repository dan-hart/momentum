# Nextcloud acceleration across native clients

User approved the client improvements on 2026-10-23, then explicitly deferred server-side changes, on
`task/native-experience-overhaul`, with functional regression testing. Only commit
and push are authorized GitHub mutations; no PR, merge, tag, release or store upload.
The user will verify the numerical 10× claim. No speed multiplier is claimed here.

## Compatibility and ownership

Existing v2 files remain readable/writable by default. Transport pooling, validated
conditional reads, scheduling and unchanged-engine improvements apply to v2.
Delta/checkpoint/archive/key-envelope work uses an explicit device-local optimized
mode and a separate `momentum-v3` collection below the selected legacy folder.
Enabling it explains that every participating device needs the upgraded app/mode;
it copies the legacy state into the new collection without rewriting/removing the
legacy file. Never silently dual-write conflicting versions or interpret a failed
optimized read as an empty account. Upgrading is one-way for that connection: a durable account/collection marker is written before the first optimized exchange, including failed or uncertain publication. Downgrading is rejected, while provider Off and LibreSync remain available. Markers live outside restorable task metadata and survive preference resets. Reconnecting must not discard queued edits. Store ownership, cancellation, provider exclusion,
restore generations, opaque values, vector clocks and commit durability remain.

Shared rules and transport live in Rust. Native clients own lifecycle, credentials,
localized controls and navigation. No mandatory proprietary service is introduced.
Push is an optional Nextcloud capability, an invalidation hint only; authoritative
WebDAV reads and fallback reconciliation remain required. Background wake timing
continues to obey each OS. No private accounts or task stores are used for tests.

## Nine client implementation slices

1. Shared adaptive scheduling: ~750 ms bounded edit coalescing; active foreground
   reconciliation ~5 s, backing off to 150 s while idle. Changes arriving during
   exchange schedule a prompt follow-up. Existing offline, saving-power and selected
   provider guards remain. Native adapters consume shared policy exports.
2. Shared reusable HTTP pool with per-request remaining deadlines; bounded pool
   isolation/lifetime, failed-connection recovery, cancellation and safe retries.
3. Validated remote baselines for conditional reads with pending edits and restart;
   exact configuration/content binding, cold invalidation and safe conflict rebasing.
4. Independent conditional settings exchange; keep consent/review/error semantics,
   avoid decrypt/parse/persist work when unchanged. Overlap independent network work
   only where the shared admission/deadline/commit contract remains sound.
5. Lightweight idle engine validation before cloning the entire store, rechecking
   generation/config/pending state before taking any unchanged fast path.
6. Opt-in v3 immutable operation batches, CAS-published manifest, replay/dedup,
   conflict retry, verified uploads and restart-safe pending acknowledgment.
7. Periodic immutable checkpoints and separately referenced archives; manifests
   reference verified objects. Interrupted publication leaves harmless unreferenced
   objects, never a partial committed snapshot. No destructive GC in this change.
8. Optional discovered Nextcloud Client Push integration; authenticated TLS,
   reconnect/backoff/cancellation, bounded hints, fallback polling, native lifecycle
   integration and no credential/content logging.
9. Versioned encrypted key envelope for optimized objects; retain strong Argon2
   password protection, fresh authenticated nonces, authenticated object context,
   bounded secret lifetime and legacy encryption compatibility. Independent review.

Server-side tuning and deployment inspection are deferred by the user's explicit request. No server access is needed.

## Protocol review requirements

Legacy and optimized cursors remain independent. Envelope and bootstrap manifests use create-if-absent CAS; competing clients join the winning collection. Manifest append order defines replay order and checkpoints record exactly covered operation IDs. All encrypted objects bind type/version/collection/object identity as authenticated context; malformed or unknown formats fail closed. Key caches are bounded and wiped, never reuse the legacy global password cache. Discovered push endpoints require validated same-origin TLS and must never receive credentials through redirects.

## Verification

Failing-first behavioral tests cover unchanged/pending/restart paths, stale caches,
configuration change, failed/uncertain PUT, truncated/corrupt objects, unknown schema,
wrong passwords, offline/concurrent clients, deletion/archive/order/undo preservation,
restore/cancellation, provider switching, settings consent and disabled push fallback.
Fixtures use temporary stores and loopback HTTP/TLS servers. Native tests verify
scheduler policy/control persistence and calls over regenerated bindings. Existing
Rust/CLI/FFI, Apple package/simulator/app and Android JVM/build/lint gates run.
Linux full native suite runs in the existing isolated Linux environment, not inferred
from Mac compilation. Device/OS scheduling/real-server checks remain explicitly scoped.
