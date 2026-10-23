# Desktop Updates Implementation Plan

> For agentic workers: use the subagent-driven-development and test-driven-development skills. No commits or publication without a new explicit request.

**Goal:** Channel-aware native desktop update checks and explicit installation/restart.

**Architecture:** Platform-specific update drivers retain native package ownership. Shared Rust only extends offline settings discovery. Signed feed generation is separate release tooling.

**Tech Stack:** SwiftUI/AppKit, Sparkle 2.10.0, Homebrew, GTK/libadwaita/GIO Flatpak portal, Python release tooling.

- [x] Add test-first macOS policy/process driver and native Settings/menu integration; pin app-only Sparkle; preserve iOS category filtering.
- [x] Add test-first Linux portal state/driver and native preferences, GSettings default-off consent, EN/DE wording.
- [x] Add desktop-only updates.check/automatic/restart discovery IDs with regression coverage and regenerate bindings through canonical tooling.
- [x] Add test-first signed appcast generation/publication validation, workflow dependencies/required key gates, and signing/acknowledgment documentation.
- [x] Run scoped units, full shared consumers, Apple app build/localization, Linux compile and isolated supported VM checks.
- [x] Independent spec/adversarial review; fix findings and rerun relevant tests.
- [x] Update FEATURES, BUGS, PROGRESS, TESTING, RELEASING with exact evidence and remaining external configuration/native limits. Leave local changes on task/desktop-updates.

Target files: macos/Momentum/Services and Views, macos/Packages/MomentumKit update policy/tests/resources, macos/project.yml; crates/app/src/updates.rs and prefs/settings_ui/application, data schema and po; crates/momentum-core/src/settings.rs/tests.rs; build-aux/sparkle-release.py/tests/test_sparkle_release.py, macos/scripts/package-app.sh, .github/workflows/release.yml; affected ledgers and NOTICE.

Commands: cargo test -p momentum-core settings_; swift test --package-path macos/Packages/MomentumKit --build-system swiftbuild; cargo check -p momentum --tests; python3 -m unittest discover -s build-aux/tests -p test_sparkle_release.py -v; python3 -m unittest discover -s build-aux/tests -p test_release_channels.py -v; canonical macos/scripts/build-core.sh --debug and Xcode Debug build; Linux build-aux/test.sh in isolated GNOME environment.


Validation record: see dated F-076/B-177 handoff in docs/PROGRESS.md. Native install,
consent and relaunch acceptance remains explicitly outstanding rather than checked
off as verified; the plan's native build/test/review/documentation tasks are done.
No commit, push, release, production key creation or remote configuration performed.
