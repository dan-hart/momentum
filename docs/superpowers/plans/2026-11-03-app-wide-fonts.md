# App-wide Fonts Implementation Plan

> **For agentic workers:** Use superpowers:subagent-driven-development or superpowers:executing-plans to implement and review bounded platform tasks.

**Goal:** Offer the three approved font choices on every native platform without losing typography customization.

**Architecture:** Bundle licensed font binaries; use each existing native typography adapter and preference key. Apple clients share registration/resources through MomentumKit; GTK and Compose retain platform-specific adapters. No core task/FFI changes.

**Tech Stack:** Rust/GTK/Pango, SwiftUI/CoreText, Kotlin/Compose.

- [x] Obtain original regular/bold font assets, licenses and pinned provenance; copy identical bytes into native resource bundles and verify hashes/metadata.
- [x] Apple: add bundled font catalog/registration and tests that resolve both families at independent sizes; retain font panel/native picker and add curated choices. Extend iOS semantic text styles without double scaling or replacing symbols.
- [x] Linux: load bundled fonts into the native font map before preference resolution; add curated System/Mono/OpenDyslexic choices alongside installed-font selection. Test family CSS, unchanged scales, persistence/reset and fallbacks.
- [x] Android: extend the persisted enum and resource FontFamily mappings; retain built-ins; test all Material styles and independent scales/reset, package regular/bold assets.
- [x] Run Swift package tests, relevant Rust/GTK checks, Android JVM/build checks and Apple app builds using guarded tooling. Add isolated native checks when supported.
- [x] Review complete diff and font packaging/licensing, update F-022 and dated platform evidence in all three ledgers, commit and push the requested feature branch.

## Verification limits

See the dated F-022/B-182 handoff in PROGRESS.md. All implementation/review and
available local checks completed. Linux fresh Blueprint/native interaction and
Android instrumentation require a supported runtime; physical/accessibility checks
remain open. The broader iOS suite has a reproducible pre-existing empty-state test
failure on original sources; focused font rendering tests pass on final source.
