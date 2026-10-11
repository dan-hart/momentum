<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Momentum for Android

A Kotlin/Jetpack Compose client of the existing Rust task engine. Source builds
work without an account, payment, proprietary Google services or a Momentum server.
The same app source builds an APK for direct installation and alternative stores,
and an Android App Bundle for eventual Google Play distribution.

## Build from source

Use Linux x86-64 or macOS with Python 3.9+, a FLOSS OpenJDK 17 distribution and rustup. Install the pinned
toolchain and Android SDK packages; accept their licenses using SDK Manager:

```sh
rustup toolchain install 1.99.0 --profile minimal
rustup target add --toolchain 1.99.0 aarch64-linux-android x86_64-linux-android
sdkmanager 'platforms;android-37.0' 'build-tools;37.0.0' 'ndk;27.2.12479018' 'platform-tools'
export ANDROID_HOME=/path/to/android/sdk
python3 android/scripts/build_core.py --prepare-source
cd android
./gradlew :app:assembleDebug
```

`--prepare-source` downloads public LibreSync v0.6.0 only when `libresync-src` is
absent. It verifies the archive SHA-256 and extracted tree against
[toolchain.json](toolchain.json), and refuses to overwrite an existing mismatched
checkout. You can instead supply a clean checkout of commit
`839c98c0386ee6a5b8ba2f298de48fa0bf2dacaa`; `--verify-source` checks the same content.
Keep local SDK paths in ignored `local.properties` or `ANDROID_HOME`.

Gradle runs `buildRustCore` before compilation: locked Cargo dependencies, both
`arm64-v8a` and `x86_64` libraries, then UniFFI Kotlin bindings from a host library.
It uses `android/rust-target` rather than the Apple/desktop target directory. Native
outputs are `app/build/generated/jniLibs/<abi>/libmomentum_ffi.so`; bindings are
`app/build/generated/uniffi/com/codedbydan/momentum/core/momentum.kt`. Both are
generated, ignored and never edited by hand. Builds verify 16 KiB ELF LOAD and
RELRO alignment. AGP packages uncompressed native libraries with modern alignment.

The matching AndroidX Graphics Path 1.1.0 JNI is also source-built with the pinned
NDK and both 16 KiB linker flags. Its unchanged Apache-licensed inputs and exact
Maven/source correspondence are [documented here](vendor/GRAPHICS-PATH.md).
Gradle replaces only the reviewed AAR's JNI entries; public classes, resources,
metadata and licenses stay byte-identical. No prebuilt ELF is patched.

The wrapper pins Gradle 9.8.1 and its distribution checksum. AGP 9.4.1 uses
[built-in Kotlin](https://developer.android.com/build/migrate-to-built-in-kotlin),
with Kotlin/Compose compiler 2.4.21 (stable October 8, 2026; see
[Kotlin releases](https://kotlinlang.org/docs/releases.html)). Dependency versions are in
[libs.versions.toml](gradle/libs.versions.toml). Minimum SDK is 26, target/compile
SDK is stable API 37. The app intentionally has no store-specific flavors.

## Tests and local iteration

```sh
# From the repository root:
python3 -m unittest discover -s android/scripts/tests -v
python3 android/scripts/build_core.py --verify-source
python3 android/scripts/build_graphics_path.py --verify-source
cd android
./gradlew :app:testDebugUnitTest :app:assembleDebug -PmomentumRustProfile=debug
./gradlew :app:connectedDebugAndroidTest -PmomentumRustProfile=debug
./gradlew :app:lintRelease
```

The debug Rust override shortens local iteration; omit it for distribution builds.
JVM integration tests load the host Rust library with `jna.library.path` pointing
at `android/rust-target/debug`. A matching JNA JVM jar supplies host dispatch code;
the Android product uses its AAR. Every test must use disposable stores, not an
installed application's tasks or real sync credentials. Instrumentation requires
a running emulator/device and remains separate from JVM tests and compilation.

`TaskScaleTest.thousandTasksLoadScrollAndEditThroughTheNativeCore` remains in the
ordinary native suite and checks a complete 1,000-task projection, lazy-list
scrolling, editing and persistence after reopening its isolated store. A separate
timing observation requires both explicit disposable-emulator and performance
flags. With current debug app/test APKs already installed on an owned emulator:

```sh
adb -s emulator-5554 shell am instrument -w -r \
  -e momentum.disposableOsTests true -e momentum.performanceTests true \
  -e class 'com.codedbydan.momentum.TaskScaleTest#observeThousandTaskCompleteProjectionAndNativeFixtureTimings' \
  com.codedbydan.momentum.test/androidx.test.runner.AndroidJUnitRunner
adb -s emulator-5554 logcat -d -s MomentumScale:I '*:S'
```

The log records the complete first snapshot after repository reopen, first list
paint in the production Compose/ViewModel fixture, and scrolling/opening the last
task. It has no timing pass threshold and does not measure shipping Activity cold
launch. Only synthetic UUID-scoped cache stores are created and removed.

### Isolated two-emulator LibreSync acceptance

`LibreSyncTwoEmulatorTest` skips unless the host explicitly opts into disposable
emulators. After installing the current debug application and test APK on two
owned emulators, run from the repository root:

```sh
python3 android/scripts/test_libresync_two_emulators.py \
  --alpha emulator-5554 --beta emulator-5556 --disposable-emulators
```

The runner performs no build/install and requires a debuggable app for `run-as`.
It validates both emulator hardware identities before testing, uses UUID cache
stores and their own Keystore aliases, and selects the production LibreSync
coordinator/FFI path. Android 17 LAN permission is granted only for that explicitly
owned fixture and restored if initially denied. Private command/status files and
captured subprocess output keep pairing codes, device identities and credentials
out of logs; only a sanitized result is printed.

The bridge binds host TCP to loopback and forwards opaque encrypted bytes through
a private Unix socket and one scoped ADB forward. One client exchange transfers
operations in both directions. This checks pairing, offline edits/additions,
completion/deletion convergence, persisted tasks and Keystore identity reuse; it
does not prove multicast discovery, symmetric routability through emulator NAT or
physical-device behavior. Normal/failure cleanup stops and drains fixture engines,
removes only their files/aliases/forward and restores the initial permission state.
If drain cannot finish within its bound, the runner stops the owned test app and
invokes its UUID-scoped cleanup mode. It never clears app data or contacts accounts.
Actual native results belong to the separate opt-in run; compiling this fixture
does not verify an exchange.

The 2026-10-10 opt-in run passed on API26 and API37 using the same gate8b APKs:
actual pairing, both devices' offline edits, completion/deletion convergence and
durable Keystore reuse after reopening. Exact artifact hashes and remaining
discovery/physical-device limits are recorded in [PROGRESS.md](../docs/PROGRESS.md).

## Unsigned and signed distribution artifacts

```sh
./gradlew :app:assembleRelease :app:bundleRelease :app:lintRelease
cd ..
python3 android/scripts/build_core.py --check-apk android/app/build/outputs/apk/release/app-release-unsigned.apk
```

Unsigned APK: `android/app/build/outputs/apk/release/app-release-unsigned.apk`.
Bundle: `android/app/build/outputs/bundle/release/app-release.aab`. F-Droid signs its
own build. Other stores or direct distribution can use a locally owned key by
providing all four environment variables before the same Gradle invocation:
`MOMENTUM_KEYSTORE` (absolute path), `MOMENTUM_KEYSTORE_PASSWORD`,
`MOMENTUM_KEY_ALIAS`, and `MOMENTUM_KEY_PASSWORD`. Supply these through a local
secret manager or private shell environment; do not pass secrets on Gradle's
command line or commit keys/passwords. Partial signing configuration fails early.
With signing configured, the APK is `app-release.apk`. There is no default release
key. Build commands do not upload or publish either artifact.

Android updates require compatible signing certificates. If direct/alternative-store
APKs and Play APKs should update each other, supply the independently owned app
signing key when enrolling in Play App Signing; Google's generated key would not
be available to sign independent APKs. The AAB upload key can be separate from the
app signing key. A conventional F-Droid-signed build has its own certificate and
cannot replace an upstream-signed installation under the same application ID.
F-Droid's verified reproducible-build route can retain an upstream signature, but
that process has not been established for Momentum. Choose the intended update
channels before the first release. See [Android signing guidance](https://developer.android.com/studio/publish/app-signing)
and [F-Droid reproducible builds](https://f-droid.org/en/docs/Reproducible_Builds/).

`--check-apk` checks every bundled `.so`, including JNA, for ELF LOAD/RELRO and APK
ZIP alignment using the pinned SDK/NDK tools. Bundle-produced APKs and a 16 KiB
emulator/device must also be tested before a store release. See
[Android's page-size guidance](https://developer.android.com/guide/practices/page-sizes).

## F-Droid and licensing

[fdroid/README.md](fdroid/README.md) documents the draft source recipe and remaining
verification. A local APK demonstrates neither F-Droid admission nor bit-for-bit
reproducibility. No F-Droid or Google Play release has been requested.

Momentum sources retain GPL-3.0-or-later. The linked LibreSync source is
AGPL-3.0-only; preserve its [license](../libresync-src/LICENSE) and corresponding
source when distributing. AndroidX is Apache-2.0, Kotlin/coroutines are Apache-2.0,
UniFFI is MPL-2.0, and JNA is dual LGPL-2.1-or-later/Apache-2.0 (Apache option).
[Third-party notices](THIRD_PARTY_NOTICES.md) link the upstream sources/licenses.
The build generates `assets/THIRD_PARTY_NOTICES.txt` inside each APK/AAB: exact
release-runtime Maven coordinates/POM license declarations, artifact license files
(including nested AAR `classes.jar` notices), full Rust source-package notices,
Momentum/LibreSync licenses, and retained JNA/libffi/Kotlin/coroutines notices.
The pinned Rust standard-library copyright inventory and referenced license texts
are also retained; Cargo metadata alone does not list that library.
JNA's reviewed AAR SHA-256 and upstream notice-file hashes are checked during this
generation. Original packaged Apache/LGPL resources are merged, rather than removed.
Source archives and a release license review still accompany distribution.

[Distribution audit](DISTRIBUTION_AUDIT.md) records current provenance, permission
and store gates. [Android CI](../.github/workflows/android.yml) prepares public pinned
source and checks JVM tests, unsigned release APK/AAB, lint, native/ZIP alignment,
bundle configuration and packaged notices. It requires no signing key and has no
artifact upload or publishing step. Its first Linux runner execution is outstanding.

Task/credential files are excluded from Android cloud backup and device transfer;
use the explicit native export/restore flow. Sync is opt-in. Target SDK 37 requires
[local network permission](https://developer.android.com/privacy-and-security/local-network-permission)
before LAN discovery or private-network sync, independently of `INTERNET`.

Explicit Copy Notes/Title marks clipboard data as sensitive. Small text remains
plain text; larger full UTF-8 text uses a read-only, nonexported AndroidX FileProvider
with a narrow private `clipboard-payloads/` directory. This avoids the system's
[Binder transaction limit](https://developer.android.com/reference/android/os/TransactionTooLargeException).
On API 28 and later, the clipboard service grants recipients read access and revokes
it on replacement. Android 8 (API 26–27) has a confirmed platform URI-grant bug:
large Copy Notes and Copy Title attempts are rejected before changing the previous
clipboard or creating a payload, with localized feedback. Full notes have the
separate sharing action below; full-title sharing is not implemented. The provider
is never made public to bypass the Android 8 limitation.
Explicit Import Paste streams only this app's text provider and enforces the
existing 1,000,000-character source limit; it does not fetch arbitrary content URIs.
Successful verified copies remove prior owned payloads/grants. Active payloads and
process-death orphans remain until the next verified explicit replacement or app
removal, bounded to 32 files of at most 4,000,000 UTF-8 bytes each. Capacity/storage
failure preserves the earlier files and surfaces a localized error. No background
clipboard read or age-based deletion can prematurely break the current clip.

The editor's separate **Share full notes** action opens the standard
[Android Sharesheet](https://developer.android.com/develop/ui/compose/sharing/send),
including on Android 8. Full text travels as a `text/plain` stream with a temporary
read-only grant to the user's selected recipient; the Intent contains only its URI.
The private `shared-notes/` cache has its own 32-file/4,000,000-byte bounds. Clipboard
replacement does not delete shared files. Existing streams and process-restart
recovery retain their files; Android can evict cache, and users can explicitly clear
Momentum's cache in system Settings when its sharing capacity is full. App storage
must not be cleared for that purpose. Chooser cancellation has no dependable
grant-lifetime callback, so it does not trigger deletion of a possibly active share.
