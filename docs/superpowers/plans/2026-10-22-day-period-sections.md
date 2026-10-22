# Day-period sections implementation plan

> For agentic workers: use superpowers:subagent-driven-development with spec and code-quality review. The user subsequently authorized commits/pushes to this dedicated branch, adversarial fixes, and unit coverage without platform UI tests.

**Goal:** Implement the approved day-period sections design on all four native platforms.

**Architecture:** Rust owns Today membership and a shared section-boundary query. Native applications persist the default-on visibility preference and render native separators. Generated FFI bindings expose the new preference and boundary query without hand edits.

**Tech stack:** Rust/UniFFI, GTK/libadwaita, SwiftUI/UserDefaults, Kotlin/Compose/DataStore.

## Tasks

- [x] 1. Core and Linux: add failing behavioral tests in `crates/momentum-core/src/tests.rs` for filtered Today across grouping modes, completed families, overdue retention, default behavior and other destinations. Add shared section-boundary tests for all group combinations. Implement in `types.rs`, `listing.rs` and existing engine API. Wire the preference into `crates/app/src/prefs.rs`, `window.rs`, native resources/settings schema and English/German translations; render separators using the shared boundary rule. Run focused core tests and GTK checks. Fix CLI/full preference constructors as necessary.
- [x] 2. Apple: regenerate bindings through `macos/scripts/build-core.sh --debug`. Add persistence and refresh regressions. Wire `Preferences.swift`, native Mac/iOS settings and Today list headers to the shared preference and boundary query. Update explicit test constructors; keep portable-settings fields unchanged. Run MomentumKit, portable MomentumMobile and both production builds; native isolated view checks where available.
- [x] 3. Android: regenerate through `android/scripts/build_core.py`, add preference persistence/refresh coverage, wire `MomentumRepository.kt`/ViewOptions/settings and `TaskList.kt` to the shared contract. Add EN/DE resources. Run JVM/build/native checks where available.
- [x] 4. Review spec compliance, then code quality; repair concrete findings. Run Rust workspace/FFI/CLI checks and applicable native checks. Update `docs/FEATURES.md` with F-071 and F-037 scope, `BUGS.md` assessment, dated `PROGRESS.md`, `GROUPING.md` and `MVP.md`. Check formatting, localization, links and diff whitespace. Record actual results and unavailable native environments explicitly. No commit, push or release.

Core contract chosen before native tasks: `Preferences.show_day_period_tasks_in_today: bool` (default true); `Engine.section_has_day_period_separator(listing, index)` or equivalent pure exported helper returns true only for Today/MorningNight boundaries between populated open day-period groups. Native renderers consume this query rather than duplicating the grouping policy.

Core/Linux review: spec compliance and code quality approved after correcting hidden selection retention. Focused core and FFI checks pass; GTK tests type-check. Native Linux runtime remains unavailable on this Mac. Full socket-dependent tests rerun outside the sandbox.

Final verification: 399 Rust workspace, 207 MomentumKit, 170 MomentumMobile and 101 Android JVM tests pass; GTK tests compile; final shipping Apple builds, Android debug/test APKs and lint pass. Scoped iOS native persistence and rendering/selection tests pass. Linux/Mac live interaction, Android instrumentation and physical/spoken limits remain in PROGRESS. No commit or publication.
