# Nextcloud Acceleration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox syntax for tracking.

**Goal:** Implement the nine client sync improvements (server work explicitly deferred) on all four native platforms without functional regressions; commit/push only.

**Architecture:** Shared Rust transport, optimized opt-in protocol, caching, notification and scheduling policy; native lifecycle and settings adapters. Existing v2 remains default. Optimized immutable batches/checkpoints/archives and encryption envelope live in a separate collection.

**Tech Stack:** Rust/ureq/serde/AES-GCM/Argon2, native GTK, SwiftUI and Compose, generated UniFFI.

## Task 1 — Shared transport and portable-settings efficiency
Files: `crates/momentum-sync/src/lib.rs`, `portable.rs`, new transport tests/modules.
- [x] Write failing loopback tests for connection reuse, conditional reads with pending edits, unchanged settings, deadlines, conflicts and cache invalidation.
- [x] Run targeted `cargo test -p momentum-sync` and record RED.
- [x] Implement pooled request execution and exact validated baseline caching. Keep verified PUT/ETag and uncertain-write protections.
- [x] Test GREEN and run full sync crate tests; review spec and code independently.

## Task 2 — Optimized immutable protocol and encrypted objects
Files: new `crates/momentum-sync/src/optimized.rs`, optimized tests, `crypto.rs` additions.
- [x] Write failing tests for bootstrap, two-client convergence, delta upload, checkpoint/archive separation, interruption/restart, conflict/dedup, unknown formats and encrypted wrong-key/corruption rejection.
- [x] Implement protocol behind explicit optimized config; isolate all files beneath `momentum-v3`; never mutate legacy source during bootstrap.
- [x] Implement versioned key envelope, bounded cached secret lifecycle and fresh authenticated object nonces/context.
- [x] Verify all objects before CAS manifest publication; preserve pending edits on every failure.
- [x] Run full crate regressions, spec compliance and independent security/code review.

## Task 3 — Shared core integration, fast idle validation and push
Files: `crates/momentum-core/src/engine.rs`, `types.rs`, `portable.rs`, new scheduling/push modules, sync notification module, generated bindings via tooling.
- [x] Add failing tests for clone-free idle validation, pending/restore races, independent settings completion, hint loss/duplication, disconnect and provider cancellation.
- [x] Add `optimized: bool` to core settings record (default false in device preferences); integrate protocol under same guarded store commit.
- [x] Export adaptive scheduling policy once from Rust and consume it in native clients.
- [x] Integrate optional capabilities-discovered push listener with TLS and cancellation; hints trigger normal safe exchange.
- [x] Run Rust workspace tests/default and all features, including CLI, then regenerate bindings.

## Task 4 — Native Linux/macOS/iOS/Android adapters
Files: GTK window/preferences/schema/Blueprint/po; Apple state/preferences/mobile sync/localization; Android coordinator/settings/resources/tests.
- [x] Write failing native policy/persistence tests for coalescing/backoff/lifecycle and opt-in mode controls.
- [x] Wire shared policy, bounded follow-up, selected-provider guards and push wake hints.
- [x] Expose optimized-mode explanation/consent and preserve queued edits on settings changes; keep EN/DE resources complete.
- [x] Test packages, build apps, run Android JVM/lint and existing native Linux suite using isolated data; record physical-device/hosting limits.

## Task 5 — Server work deferred
- [x] Respect user instruction to skip server-side changes; do not inspect or tune real Nextcloud hosting.

## Task 6 — Integration, review and delivery
- [x] Update FEATURES/BUGS/PROGRESS with independent platform scopes and acceptance limits.
- [x] Run all relevant regression/build/resource/format/link gates and adversarial code/security review; fix real findings and recheck.
- [x] Review complete diff against inherited native-fix source and verify no unrelated changes.
- [x] Prepare reviewed changes for authorized commit and push only `task/native-experience-overhaul`; verify remote head. No GitHub API mutations, PR, tag, merge, release or store distribution.
