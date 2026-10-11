# Android implementation plan

**Goal:** Build the native Kotlin/Compose client through iOS parity and record evidence.

**Architecture:** Generated UniFFI uses the existing Rust engine. Android repository,
ViewModel, Compose screens and native services own only platform presentation/lifecycle.

**Tech stack:** Kotlin, Material 3 Compose, AndroidX lifecycle/adaptive/WorkManager,
UniFFI/JNA, Rust, Android Keystore and Storage Access Framework. Pin stable versions.

## Tasks

- [ ] Foundation: `android/settings.gradle.kts`, `build.gradle.kts`, version catalog,
  wrapper, `app/build.gradle.kts`, manifest, `scripts/build-core.py`, bindgen config,
  source-build documentation. Verify generated bindings compile and native ELF load.
- [ ] Repository and state: `data/MomentumRepository.kt`, `MomentumViewModel.kt`,
  pure draft policy tests first and Android real-core persistence tests. Serialize
  local state access off-main; retain failures and exact undo identity. Cancellable
  network exchanges use the same engine outside the edit queue.
- [ ] UI: localized strings/theme, adaptive root, task list/editor, lists management,
  repeat editor, options and selection. Test actual native engine boundaries and
  Compose actions with isolated data; preserve archived read-only policy.
- [ ] Native integrations: SAF backup/restore, shares/deep links, notifications,
  Keystore connection store, Nextcloud and LibreSync lifecycle/background adapters.
  Test provider exclusion, failed writes, permission and cancellation boundaries.
- [ ] Verification: debug and release APKs, JVM tests, lint, emulator instrumentation,
  English/German light/dark/large-text/expanded layouts. Core/CLI compatibility if
  shared Rust changes are required. Review scope and code independently.
- [ ] Records: Android README/checklist/F-Droid build recipe, FEATURES/BUGS/PROGRESS,
  actual checks with date/base and dirty-state evidence, explicit unverified gaps.

Run `cd android && ./gradlew testDebugUnitTest assembleDebug lintDebug assembleRelease`
and `./gradlew connectedDebugAndroidTest` on a disposable emulator. Shared Rust checks
use `cargo test --workspace --exclude momentum`. No commits or external publication.
