# Native Experience Overhaul Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans and scoped parallel native work; retain spec and adversarial review gates.

**Goal:** Deliver the approved day-period, encouraging empty-state, searchable Settings/About and quiet sound designs on four native apps.

**Architecture:** Shared Rust owns day completion eligibility, hidden family counts and setting destination/search contracts. Native layers own localized resources, controls, routes, empty-state selection, restrained celebration and audio. Preserve preference keys/unknown values and existing formats.

**Tech Stack:** Rust/UniFFI; GTK/libadwaita; SwiftUI/AppKit/UIKit; Kotlin/Compose; bundled PCM audio.

## Approved specifications

- `docs/superpowers/specs/2026-10-22-day-period-layout-design.md`
- `docs/superpowers/specs/2026-10-22-empty-state-message-catalog.md`
- `docs/superpowers/specs/2026-10-22-settings-discovery-about-design.md`
- `docs/superpowers/specs/2026-10-22-completion-sound-refresh-design.md`

## Stages

- [x] Shared state/search contracts: add pure directory/search and day-status queries in `crates/momentum-core/src`, exported by existing FFI. Cover hidden unfinished root counts, overdue, archived completion/date rollover, period versus whole-day eligibility; directory IDs/capabilities/ranking. Run core tests before native integration. Avoid changing Listing records to preserve existing fixtures.
- [x] Sound assets: reproducible original ten-sound generator, provenance and exact PCM copies in existing Linux/Apple/Android resources. Retain legacy assets/IDs. Extend integrity checks and licensing notices. Run asset checks.
- [x] Apple native implementation: `macos/Momentum`, `macos/Packages/MomentumKit`, `ios/Momentum`, `ios/Packages/MomentumMobile`: spacing/icons, hidden-count navigation, 100 EN/DE messages with stability and accurate completion eligibility, local preference and reduced-motion manual celebration; category Settings/search/global matches/exact routes/About; independent candidate preview, bounded local volume, single-clip native playback and legacy preservation. Add hostless tests and build shipping apps.
- [x] Android native implementation: analogous changes in `android/app/src/main`, native Compose navigation/search and resources; preserve current status/recovery controls. Add JVM tests, run JVM tests/lint/build with regenerated native libraries. No new rendered UI suite.
- [x] GTK native implementation: `crates/app/src`, `data/resources/ui`, schema and `po`: native named Settings pages/search and exact row routes/global settings results; AdwAboutDialog details; day-period UI/footer/100 EN/DE messages/local setting/manual reduced-motion-aware celebration; audio candidate preview and bounded local volume. Compile GTK consumer and resources. Linux runtime remains unavailable on Mac.
- [x] Integration verification: regenerate bindings through build scripts; run Rust workspace excluding GTK, MomentumKit and portable MomentumMobile packages, Android JVM/build/lint, GTK compilation and Mac/iOS builds. Fix regressions with root-cause analysis; no platform UI test additions. Inspect Mac with isolated data when feasible.
- [x] Spec review then independent adversarial code review and unit-coverage audit. Fix findings and rerun affected checks. Update `docs/FEATURES.md`, `docs/BUGS.md`, `docs/PROGRESS.md` with dated dirty/revision evidence and explicit platform gaps.
- [x] Commit focused changes on `task/native-experience-overhaul`, push origin, read back exact remote SHA; no PR, merge, tag or release.

## Ownership and review

Core contracts first; native work owns disjoint platform paths. Root owns shared sound masters, GTK, docs, final integration/publication. Agents do not commit or overwrite unrelated changes. Cross-platform consumers use generated bindings, never hand-edited generated code. Verify units at each stage; native UI claims require actual inspection and are not inferred from tests.

Final verification and scoped runtime limits are recorded in `docs/PROGRESS.md`.
Publication uses the explicitly requested branch; the exact remote head is read back
in the chat handoff after push.
