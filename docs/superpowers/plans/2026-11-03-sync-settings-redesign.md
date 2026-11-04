# Sync Settings Redesign Implementation Plan

**Goal:** Simplify setup and recovery on all four native platforms (F-077/B-186).
**Architecture:** Preserve the shared Rust operations and platform state owners. Reorder and group native controls; disclose advanced controls without changing storage or transport semantics.
**Tech stack:** SwiftUI, GTK/libadwaita, Kotlin/Compose; existing EN/DE localization.

- [x] Review the design and existing sync contracts. Preserve inherited edits in FEATURES, PROGRESS and ios/README.
- [x] macOS: SettingsView SyncSettings; status/recovery first, guided basic connection, advanced disclosure, timing group and optional app settings last. Update native catalog and verify build.
- [x] Linux: prefs.rs; equivalent native preference groups/expander, status/actions first, guided setup, advanced controls, settings consent last. Update gettext and run available checks.
- [x] Android: SyncScreen.kt; equivalent overview/setup/advanced sections, saved-edit safety and native EN/DE resources. Run JVM/build checks.
- [x] iOS: SyncSettings.swift; equivalent form order, guided setup, advanced disclosure and search expansion, persist bootstrap/error surfaces. Update catalog and run targeted simulator tests/build.
- [x] Review each implementation for acceptance and code quality. Test recovery/search boundaries; keep real accounts untouched.
- [x] Update FEATURES, BUGS and PROGRESS with actual platform evidence and remaining manual acceptance. Review diff and formatting. Commit/push only with the user’s follow-up authorization; no release.

## Handoff

Implemented locally on all four platforms. Available checks and independent reviews
completed; platform statuses remain Implemented pending the native acceptance gaps
recorded in the [F-077 progress handoff](../../PROGRESS.md). The user subsequently
authorized committing and pushing the redesign to main; no release.
