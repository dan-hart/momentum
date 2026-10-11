<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Android distribution audit

2026-10-11, dirty `task/android-native` worktree based on `200f686`. This records
source/build findings; no store listing, upload, key registration or release occurred.

## Native dependency provenance and source inputs

JNA `net.java.dev.jna:jna:5.19.1@aar` is from
[Maven Central](https://repo.maven.apache.org/maven2/net/java/dev/jna/jna/5.19.1/).
A fresh upstream download has SHA-256
`b57125cb7d16253f0d65a80f7d3a4c3664effa711b8bdbb7f87fb572ce1624ed`.
Its [POM](https://repo.maven.apache.org/maven2/net/java/dev/jna/jna/5.19.1/jna-5.19.1.pom)
declares the Apache-2.0/LGPL-2.1-or-later choice; Momentum uses Apache-2.0.
Tag `5.19.1` resolves to commit `1a91122853f6ab6f1fb2a4a284a6cf2ed8af0a4d`.
The [native Makefile](https://github.com/java-native-access/jna/blob/1a91122853f6ab6f1fb2a4a284a6cf2ed8af0a4d/native/Makefile)
contains Android dispatch/libffi build inputs and both 16 KiB linker flags.
The [Ant AAR target](https://github.com/java-native-access/jna/blob/1a91122853f6ab6f1fb2a4a284a6cf2ed8af0a4d/build.xml#L586)
packages native dispatch libraries from upstream native JARs. Public source/build
instructions and a matching Maven artifact are established; a local rebuild or
byte-for-byte source-to-AAR equivalence has not been established.
Version 5.19.1 restores older Android compatibility by replacing MethodHandle use
with reflection, according to its [upstream changes](https://github.com/java-native-access/jna/blob/1a91122853f6ab6f1fb2a4a284a6cf2ed8af0a4d/CHANGES.md).
The freshly downloaded AAR's arm64-v8a and x86_64 dispatch libraries both pass the
16 KiB ELF LOAD and RELRO checks. Other AAR architectures are excluded by the app's
ABI filters. JVM/Android integration, R8 and APK ZIP packaging are checked by the
subsequent application build; this artifact inspection alone does not establish them.

Current [F-Droid policy](https://f-droid.org/en/docs/Inclusion_Policy/) permits
freely licensed binaries from trusted Maven repositories, including Maven Central,
with review conditions. JNA is therefore not automatically disqualified merely
because it contains upstream native binaries. A local F-Droid source scanner run
passes on the prepared source. The current minified release and bundle-produced
APKs also pass the refreshed known-nonfree-class/extra-signing-block scan. Separate
Maven dependency review and F-Droid's actual builder/admission remain outstanding;
local scanner results do not establish approval.

Momentum's native library is built from Rust source. LibreSync v0.6.0 is pinned by
commit, archive checksum and a full source-tree digest in [toolchain.json](toolchain.json).
The source verifier rejects modified/extra files and unexpected symlinks, excludes
Git/local build metadata, and refuses to overwrite mismatched source. Cargo.lock
has 395 registry checksum entries and no Git-source dependencies. Each Cargo build
uses `--locked`. The draft F-Droid recipe uses
[cargo vendor](https://doc.rust-lang.org/cargo/commands/cargo-vendor.html) with
versioned directories and subsequently disables Cargo network access. Its narrowly
reviewed [Android-only preparation](fdroid/SOURCE_PREPARATION.md) removes unused
Windows/wasm target inputs and upstream fixture data, without rewriting Rust code.
The original package checksums and remaining file hashes are preserved; only
deleted-file checksum-inventory entries are removed as required by Cargo. The
[local validation record](fdroid/VALIDATION.md) reports metadata/source-scan findings
and fresh offline build evidence separately from actual builder/reproducibility gates.

These checks fix reviewed inputs, not output reproducibility. Independent builders
must compare unsigned APKs, including Rust paths/toolchain, native stripping and
ZIP metadata. [F-Droid's reproducibility guidance](https://f-droid.org/en/docs/Reproducible_Builds/)
describes Rust build-path and compiler-version concerns. No reproducibility result
or F-Droid admission is claimed.

## Runtime SDKs, permissions and notices

The resolved debug runtime graph and Android sources were inspected. They contain
AndroidX, Kotlin/coroutines and JNA, without GMS, Firebase, advertising or proprietary
analytics SDKs. AGP's `com.android.tools.analytics-library` entries occur in the
buildscript graph and are not packaged runtime SDKs. This is a source/dependency
inventory, not a network-capture audit of optional synchronization.

The manifest requests internet/network state for optional sync, local network and
multicast access for discovery, notifications/reminder alarms, boot reconciliation,
and WorkManager wake locks. The unused WorkManager foreground-service permission
and service are removed from the merged manifest; workers use ordinary scheduled
jobs. There are no broad storage, location, contacts, camera, microphone, advertising
ID or package-install permissions. The system file picker supports import/export.

Target API 37 requires the native
[local network permission](https://developer.android.com/privacy-and-security/local-network-permission)
flow before LAN access. Precise reminders use user-granted
[SCHEDULE_EXACT_ALARM](https://developer.android.com/develop/background-work/services/alarms),
with permission checks and an inexact fallback, rather than the more restricted
`USE_EXACT_ALARM` permission. Runtime permission denial and revocation remain native
acceptance checks. Cloud backup and device-transfer data extraction are disabled.

Explicit clipboard copies are sensitive and large full text uses a nonexported,
read-only [FileProvider](https://developer.android.com/reference/androidx/core/content/FileProvider)
limited to private `clipboard-payloads/` and explicit sharing's `shared-notes/` cache.
No new storage permission or dependency
was introduced. A separate-UID API 37 disposable recipient pasted a complete
1,105,920-byte Unicode clip through `ClipData.Item.coerceToText`, including after
the sender process was killed and its provider restarted. The recipient received
read access, no write access, and lost read access after clipboard replacement.
This live system-boundary proof does not replace the subsequent app-native suite
or establish behavior on every supported API. B-115's native regressions cover the
app helper, bounded explicit own-provider Paste, and capacity failure preserving
earlier payloads. [README.md](README.md) records the bounded payload lifecycle.

An independent API 26 probe reproduced the Android 8 URI-clipboard grant failure:
the separate UID had no read/write grant and received no source text. Official
[Android 8 ClipboardService](https://android.googlesource.com/platform/frameworks/base/+/android-8.0.0_r1/services/core/java/com/android/server/clipboard/ClipboardService.java)
requests the grant as system UID, which its activity manager refuses; Android 8.1
retains that path. [Android 9's implementation](https://android.googlesource.com/platform/frameworks/base/+/android-9.0.0_r1/services/core/java/com/android/server/clipboard/ClipboardService.java)
uses the original source UID. The app now truthfully rejects large URI copies on
API 26–27 and provides a separate user-invoked full-notes Sharesheet with a private
read-only stream. The independent API 26 ACTION_SEND probe received all 1,105,920
UTF-8 bytes exactly, with read permission and no write permission. App helper and
native chooser regressions passed on API 26 and 36; the revised chooser touch
fixture passed on API 36. Direct full-text read-only sharing also passed on API 37.
On API 36, the actual shipping TaskEditor Sharesheet sent its private notes URI to
a separate disposable recipient UID. It read all 1,105,920 UTF-8 bytes exactly
before and after the sender was killed with SIGKILL; the subsequent read restarted
the shipping FileProvider process and retained read-only access. This proves the
file-backed grant path for that device/revision, not indefinite cache retention.
Android 8 also rejects oversized Copy Title without changing its clipboard; there
is no full-title sharing action. Notes have the explicit alternative above.

License generation packages `assets/THIRD_PARTY_NOTICES.txt`, with the exact Maven
runtime inventory/POM declarations, available JAR/AAR notices (including nested
`classes.jar`), Rust normal/build dependency source notices for both ABIs, full
Momentum/LibreSync license text, and original JNA/libffi/Kotlin/coroutines notices.
Checked-in upstream notice bytes are hashed in [licenses/provenance.json](licenses/provenance.json).
Artifacts that inherit POM licenses are resolved through their cached parent POM.
Apache/LGPL artifact resources are merged instead of excluded. Distribution still
requires matching corresponding source and a final transitive license review.

The pinned Rust 1.99 standard-library inventory is separate from Cargo metadata.
Its upstream copyright HTML and five referenced full license texts are retained
without abridgement, with exact distribution/source provenance and SHA-256 hashes.
Gate11h debug APK, minified unsigned release APK and AAB contain all six files'
bytes in the 2,170,926-byte notice asset. Independent store review remains open.

Gate11h release APK and bundle-generated universal APK both pass strict ELF
LOAD/RELRO and ZIP alignment for all eight native libraries. The AAB validates
with bundletool and declares `PAGE_ALIGNMENT_16K`; its universal APK uses only the
local disposable debug key for runtime checks. No official signing key or public
release was created. Exact hashes and native checks are in
[PROGRESS.md](../docs/PROGRESS.md).

## Current APK scanner checkpoint

On 2026-10-11, fdroidserver 2.4.5 scanned both gate11h APKs using Android build-tools
37.0.0 `dexdump` and freshly fetched SUSS signatures. Each completed with exit zero,
zero warning/error findings for known nonfree classes and extra signing blocks.
The signature inventory SHA-256 is
`0d2f2c91bfe18b7bf797fb8621a9c5b74896a3c7bd0fcd409b61aa3864ccb7dd`.
Exact artifact identities, tool scope and logs are retained in
`android/rust-target/gate11h-fdroid-apk-scan.json`. This checks packaged DEX/signing
blocks; it does not independently rebuild native libraries, audit all network
behavior or replace corresponding-source/license, Maven or store review.

## Store and signing gates

The same source produces an unsigned release APK and AAB, with optional signing
from four local environment variables documented in [README.md](README.md).
No signing key, password or store credential is in source or CI.
Updates require compatible signing certificates. For independent APK and Play
update continuity, retain an owned app-signing key and supply it when configuring
Play App Signing. The AAB upload key may differ; it does not sign Play-delivered
APKs. See [Android's signing guidance](https://developer.android.com/studio/publish/app-signing).
Ordinary F-Droid repository signing creates a different update channel. Its
verified reproducible-build route is needed to retain an upstream signature.

Target API 37 exceeds the current Play requirement of API 36 for new phone apps
and updates from August 31, 2026. [Official target API policy](https://support.google.com/googleplay/android-developer/answer/11926878?hl=en).
AGP 9.4.1 and the alignment verifier address native ELF and APK ZIP packaging;
[Android's page-size guidance](https://developer.android.com/guide/practices/page-sizes)
also requires checking bundle-generated APKs and testing on a 16 KiB device.

Before a public store release, required outstanding work includes an approved
release revision and increasing version code, signed upgrade validation, independent
F-Droid builder validation, final Maven/source review, independently reproduced outputs if pursuing
upstream-signature distribution, native/accessibility/offline/sync acceptance,
and 16 KiB runtime testing. A Play privacy policy and Data safety declaration must
reflect optional Nextcloud/LibreSync data flows; absence of analytics alone does
not determine that declaration. [Official Data safety requirements](https://support.google.com/googleplay/android-developer/answer/10787469?hl=en).
Developer registration/verification for intended stores and regions is a later
distribution decision; current [Android developer verification guidance](https://developer.android.com/developer-verification/guides)
does not require adding a proprietary SDK or changing free source builds.

The [Android CI workflow](../.github/workflows/android.yml) is source-build verification
only: pinned public sources, host/generated-binding regression, JVM tests, unsigned
APK/AAB, release lint, bundled notice checks, ELF/ZIP alignment and AAB alignment
configuration. It has read-only repository permissions and no publishing/artifact
upload step. Its first hosted Linux run is still outstanding.
