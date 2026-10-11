<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Local F-Droid validation

2026-10-10–11 local validation used **fdroidserver 2.4.5**, installed from PyPI into
an isolated Python 3.12.15 virtual environment under ignored
`android/rust-target/fdroid-audit`. No global installation or external repository
write occurred. The official [installation instructions](https://f-droid.org/en/docs/Installing_the_Server_and_Repo_Tools/)
describe this tooling; [PyPI](https://pypi.org/project/fdroidserver/2.4.5/) identifies
the selected stable release. F-Droid production tooling may use a newer revision.

The fresh offline native-build snapshot copied **654** tracked/nonignored source and pinned
LibreSync files from dirty `task/android-native`, base
`200f686c5e5c9ca9e35eb97cf434d8e54cee38a6`, at
`2026-10-11T02:37:18Z` (2026-10-10 local time), excluding ignored outputs and the
worktree Git link. This source includes B-114's atomic notification action and
restore-generation source identity; the earlier offline proof predates that change
and is superseded here. The snapshot manifest SHA-256 is
`075625bf213a82a448a0f9cae03632dc56f610e20a846eec12434437e8160990`.
It added the verified pinned LibreSync source and **395** locked registry packages
using offline `cargo vendor --locked --versioned-dirs`. Cargo.lock SHA-256 remains
`27728681cc789e7a28b037ffb4fb84aa50664aed08362e14fce50040ee4e3f2d`.
Fresh bindings were generated with the Kotlin 2.4.21 build configuration and
`android_cleaner=true` configuration, including its guarded Android SystemCleaner
and fallback. These checks do not replace the subsequent full Android build.

The final `gate11h-ledger-final` frozen-source scan copied **735** source/LibreSync
files at `2026-10-11T05:26:37Z`, including the B-121 clipboard/explicit notes-sharing
provider, keyboard test guards, B-122 unchanged Graphics Path C++ sources, bundled
source notices, the frozen B-127 imported-name/completion fixes and fixtures,
B-128 bounded lossless source-field presentation and B-129 composition/Undo
history corrections, exact notification-count readback fixtures, the separate
opt-in 1,000-task timing case, all six pinned Rust standard-library notice files,
B-130–126 native fixture corrections, and B-133–128 exact private-draft admission
and format-preserving streaming storage with their JVM/native regressions.
Its manifest SHA-256 is
`33e23c055c43f02b91d6c7a9623204eb251da2e7cf14fe96fb70f20e9e34c151`.
This is a documentation-only successor to the retained `gate11h-final` snapshot
`ce0cc9cc41046735961197e01ad318c88905ee18e1e28a4c791578b1c80c572b`:
only the FEATURES/BUGS `Updated` dates changed to 2026-10-11, together with the
preceding evidence update in this file. Source code and build configuration match.
The Rust workspace/Cargo input digest is unchanged:
`3a5c22a3162b487bf87d4414af1f65096ce7168dcad55c6d6f36f0069ad13370`.
The pinned LibreSync source, Rust build/source-preparation scripts, UniFFI config,
and relevant native toolchain values also match the fresh offline native proof.
The source scan is refreshed; those unchanged Rust builds were not repeated.
The copied inventory matched its manifest and current source before this evidence
record was updated. After scanning, only the three wrapper files removed by the
scanner differed in the prepared inventory. This `VALIDATION.md` evidence update
is the sole subsequent working-source difference from the recorded snapshot;
it does not extend its digest. A source scan does not verify native UI/tests;
the final build and device checks remain separate from this result.

The restore generation invalidates old notification callbacks, including for an
identical restored future reminder. Existing at-most-once policy still suppresses
already-consumed logical occurrences across restore; restoring tasks does not reset
reminder history. This is preserved behavior, not a new replay guarantee.

## Metadata and source scanner

- `fdroid lint --force-yamllint com.codedbydan.momentum` exits **0** with current
  official fdroiddata categories/icons. This corrected the obsolete `Time` category
  to `Task` and changed the Changelog reference to `HEAD`.
- The draft remains disabled at versionCode **4005**. Lint parses the draft schema,
  but its disabled state means enabled-build semantics and provisioning still need
  the actual builder. No approved release commit was invented.
- The installed `fdroidserver.scanner.scan_source` function ran against the entire
  prepared source using the parsed build entry, with no `scanignore`/`scandelete`.
  This direct call permits inspection of the local dirty snapshot; it is not a
  successful CLI checkout/build of an approved commit.
- Initial source scan: **2,837 fatal findings**. After the exact source preparation:
  **0 fatal findings, 5 warnings**, including the final 735-file source snapshot.
  See [SOURCE_PREPARATION.md](SOURCE_PREPARATION.md).
- The five warnings are untouched upstream test fixtures:
  `system-deps-7.0.8/src/tests/lib/libteststatic.a`;
  `system-deps-9.0.0/src/tests/{test.zip,test.tar.gz,uninstalled/lib/libtest.a,lib/libteststatic.a}`.
  Their warning classification comes from the installed scanner, not an app exclusion.
- The scanner's own standard behavior removed `android/gradlew`, `gradlew.bat`, and
  `gradle/wrapper/gradle-wrapper.jar` only from the copied snapshot. The working tree
  wrapper was untouched. F-Droid supplies its managed Gradle toolchain for a real build.
- Fresh official SUSS signature data contained **104 signatures**, timestamp
  `1785586835.090244`; the final fetched cache SHA-256 was
  `7a09288709a6395f8ce14d046b46cc6b493f3bc3f18e46d48ac25cdefdf2c740`.
  The preceding `gate11h-final` runner freshly fetched that official inventory;
  fdroidserver records its local `last_updated` value in the cache. The ledger-date
  successor reused and verified these exact bytes without another fetch. This
  source result is separate from the APK/Dex scans below.

## Offline native build proof

Windows-only preparation passed a fresh offline host build, Kotlin generation,
and both Android release ABI builds with unchanged checksum inventories. The
additional test-fixture removal initially failed Cargo's inventory verification;
the final tool removes only verified deleted-file entries while preserving the
original package checksum and all remaining hashes. Eight behavioral regression
tests cover exact removal, checksum preservation, changed source/license/guards,
unknown inputs, symlink safety, and idempotency.

The final source preparation validates all **395** vendored package inventories
before pruning **2,843** reviewed unused files. Full original registry archives,
code, licenses, and Cargo.lock are retained independently of the prepared copy.
Final builds use a new `CARGO_TARGET_DIR`, isolated Cargo home, `--locked --offline`,
Rust 1.99.0, NDK 27.2.12479018, and four Cargo jobs. No prior compiled target or
parent Gradle output is used. Final results:

| Check | Result |
| --- | --- |
| Fresh host `cargo build --locked --offline -p momentum-ffi -p uniffi-bindgen` | Passed, 34.3 seconds |
| Kotlin bindings generated against the fresh host dylib | Passed, 1.1 seconds |
| Fresh `aarch64-linux-android` release build | Passed, 66.2 seconds; 16 KiB ELF LOAD/RELRO checks passed |
| Fresh `x86_64-linux-android` release build | Passed, 42.1 seconds; 16 KiB ELF LOAD/RELRO checks passed |
| Android build-tool Python suite | 33 tests passed in 1.553 seconds in the final source check, including 8 source-preparation regressions and complete standard-library notice retention |

Both release builds emitted the existing Rust deprecated `fetch_update` warnings;
no warning was promoted to a build error. This checks ELF files rather than APK ZIP
alignment or 16 KiB runtime behavior. The final library SHA-256 values were
`82d214211f2c708ad3b9b6cb9e0df6d5c4f9ca9ef834aac43774598ea1aba129`
(arm64) and `19d949be0079e0c305027a41a3dce50b382eaa6becb3f2fe9ff0073339d565bf`
(x86_64). Local ignored evidence includes `gate5-source-snapshot.json`, `gate5-source-preparation.json`,
`gate5-source-scan.json`, `gate5-lint.log`, `gate5-pruned-build.json`, and per-command logs.
JNA 5.19.1 was separately downloaded and both app ABI dispatch libraries passed
16 KiB ELF LOAD/RELRO checks; [DISTRIBUTION_AUDIT.md](../DISTRIBUTION_AUDIT.md)
records its Maven hash and source provenance.
The later unchanged Graphics Path source rebuild is separately documented in
[GRAPHICS-PATH.md](../vendor/GRAPHICS-PATH.md); it is included in the final source
scan, not in the Rust library build result. Ignored refreshed evidence includes
`gate11h-ledger-final-source-snapshot.json`, `gate11h-ledger-final-source-preparation.json`,
`gate11h-ledger-final-inventory-validation.json`, `gate11h-ledger-final-source-scan.json`,
`gate11h-ledger-final-source-suss.json`, `gate11h-final-lint.log`, and
`gate11h-final-python-tests.log` under `android/rust-target/fdroid-audit`.
The successor scan reused the freshly fetched official SUSS inventory, whose exact
cache hash is recorded above. Metadata lint and the Python 33-test result are
retained from the preceding check: their recipe and executable inputs are unchanged.
No native, Gradle, APK scan, or signature-fetch check was repeated for two ledger
dates. The final evidence update in this file does not extend the snapshot.

## Unverified gates

This macOS-host validation does not prove F-Droid's Linux build VM, Rust/SDK
provisioning, network-disabled Gradle dependency resolution, signing/update
compatibility, or reproducible outputs. The draft remains
disabled until an approved source revision passes the real builder and review.
No F-Droid admission, Play upload, publication, or reproducibility result is claimed.


## 2026-10-11 — current packaged APK scanner scope

The gate11h unsigned minified release APK and bundle-produced universal APK pass
fdroidserver 2.4.5's binary scanner: known nonfree DEX classes via build-tools 37.0.0
`dexdump`, and extra APK signing blocks. Both verbose scans exit zero with zero
warnings/errors. SUSS signatures were freshly fetched once; inventory SHA-256
`0d2f2c91bfe18b7bf797fb8621a9c5b74896a3c7bd0fcd409b61aa3864ccb7dd`.
Exact APK hashes/results and separate verbose logs are in ignored
`android/rust-target/gate11h-fdroid-apk-scan.json`.

This is a local binary scanner result, separate from the prepared-source scan,
metadata lint, dependency/source/license review and independent Linux build/store
admission. It does not prove native binary source equivalence or reproducibility.
The installed Docker client has no running daemon on this host; no VM/service was
started or installed. Independent Linux builder provisioning remains outstanding.
