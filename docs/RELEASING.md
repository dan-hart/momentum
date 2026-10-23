<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!-- Copyright (C) 2026 Dan Hart -->
# Releasing

One version, one tag, one workflow. Every app and the command line carry the same
version, a release is a `vX.Y.Z` tag, and the Release workflow does the rest: it runs the
whole test suite against the tag, builds every binary, attaches them to a GitHub Release,
updates the Homebrew tap, and publishes and verifies the signed self-hosted Flatpak
repository. Both install channels are required. Flathub submission is excluded.

Android is a required build in the same tag-driven workflow. It attaches APK/AAB,
matching source, notices, R8 mapping and provenance to the GitHub Release. Optional
distribution secrets sign the binaries; without them the files are explicitly
labelled `-unsigned` and the APK requires signing before installation. There is no
Google Play/F-Droid upload, and local validation does not establish store admission.

## One version

`Cargo.toml`'s workspace version is the source of truth. Platform metadata and the
changelog carry copies, and `build-aux/release.py` keeps them equal. Android reads the
workspace version directly, so a bump also changes its APK/AAB version automatically:

| File | Copy |
|---|---|
| `meson.build` | `project(version:)`, which becomes the GTK app's `--version` and metainfo |
| `macos/project.yml` | `MARKETING_VERSION`, plus `CURRENT_PROJECT_VERSION` derived as `major*10000 + minor*100 + patch` so `CFBundleVersion` only ever goes up |
| `ios/project.yml` | The same marketing version and increasing build number for the app and share extension |
| `android/app/build.gradle.kts` | Reads Cargo directly for `versionName`; derives `versionCode` as `major*1_000_000 + minor*1000 + patch` (minor/patch at most 999, major at most 2099, positive code) |
| `data/…metainfo.xml.in.in` | the newest `<release>` and the screenshot URLs, which point at the tag |
| `CHANGELOG.md` | the newest `## [X.Y.Z]` section |

```sh
build-aux/release.py version    # print it
build-aux/release.py check      # every copy agrees; CI runs this on every push
```

CI's *Versions aligned* job fails a pull request that changes one copy without the
others, and the Release workflow refuses a tag that does not equal `v<version>`.

## Cut a release

1. Make sure `CHANGELOG.md`'s `[Unreleased]` section says what changed. Its bullets
   become the metainfo `<release>` entry and the GitHub Release notes.
2. Bump, review, commit and tag:

   ```sh
   build-aux/release.py bump 0.4.0 --summary "One sentence for the app stores."
   git diff                         # Cargo.toml, Cargo.lock, meson.build, macos/project.yml, CHANGELOG.md, metainfo
   git commit -am "Momentum 0.4.0"
   git tag -a v0.4.0 -m "Momentum 0.4.0"
   git push origin main v0.4.0
   ```

   `--summary` is optional. `--date` overrides today's date. The command refuses a
   version that is not newer, and an empty `[Unreleased]` section unless you pass
   `--allow-empty`.
3. Watch the **Release** workflow. It can also be started by hand from the Actions tab
   with an existing tag, which reruns everything for that tag.

Before tagging, run on a real Mac what CI cannot: the opt-in nearby-sync test in
[TESTING.md](TESTING.md).

## What the workflow does

| Job | What | Runs when |
|---|---|---|
| Verify | required publishing credentials and public signing fingerprint; tag equals every version copy; release notes from the changelog | always; missing configuration fails before builds |
| CI, macOS | the complete `ci.yml` and `macos.yml` suites against the tag: rustfmt, core tests, translations, the Flatpak build with the headless GTK UI tests on both architectures, the macOS unit tests, app build and localization checks | always |
| Flatpak bundle | `momentum-vX.Y.Z-{x86_64,aarch64}.flatpak` from the release manifest, tests included | tests passed |
| macOS app and mo | `Momentum-vX.Y.Z-macos.zip` (universal, `mo` inside `Contents/MacOS`) and `mo-vX.Y.Z-macos-universal.tar.gz`, via `macos/scripts/package-app.sh` | tests passed |
| mo for Linux | `mo-vX.Y.Z-linux-{x86_64,aarch64}.tar.gz`, built on Ubuntu 22.04 so the binary loads on Homebrew's glibc and on any desktop distribution of the last few years | tests passed |
| Android APK, bundle and source | Reuses `android.yml` at the exact tag: pinned public native source, Python/JVM tests, release lint, APK and bundle-produced APK 16 KiB alignment, matching APK/AAB metadata/notices/signatures, and committed source with pinned LibreSync and original locked Rust crates. Attaches `Momentum-vX.Y.Z-android[-unsigned].apk`, `.aab`, and `-android-{source.tar.gz,notices.txt,mapping.txt,distribution.txt,provenance.json}` | CI/macOS tests passed; any failed Android check blocks the GitHub Release |
| Publish Flatpak repository | signed OSTree commits for both architectures on `gh-pages`; checks the signing identity, rejects downgrades, verifies each export and preserves site files | tests passed; required before the GitHub Release |
| GitHub Release | the release named *Momentum X.Y.Z* with every build artifact above and `SHA256SUMS.txt` | every build and both hosted exports succeeded |
| Update Homebrew tap | validates final public asset digests and writes `Formula/momentum-cli.rb` and `Casks/momentum.rb` in `dan-hart/homebrew-tap` with a dedicated SSH deploy key | required after the GitHub Release |
| Deploy and verify hosted Flatpak | explicitly requests a GitHub Pages build, then verifies both public stable refs have the requested version and trusted GPG signature | required after the GitHub Release and both exports |

The Release workflow succeeds only after both install-channel jobs pass. A GitHub
Release can exist while the final channel jobs are still running or have failed;
check the workflow result before announcing complete delivery. Missing required
credentials fail preflight. Optional Mac notarization and Android signing retain
their existing fallback behavior.

Release runs share one repository-wide concurrency group. Both Flatpak architecture
exports run serially, and manual/automatic Homebrew publishers share a separate
lock. Publication rejects older versions and dirty tap checkouts, preserves unrelated
packages and propagates commit/push failures. Rerunning an unchanged version checks
write authentication and pushes any existing local commit before reporting success.
The hosted verifier pins the full public signing fingerprint, requires exact versions
on both architectures and retries boundedly for Pages propagation.

The shared `Install channels` workflow can repair or verify an existing release
without rebuilding native apps or changing its immutable tag/assets:

```sh
gh workflow run install-channels.yml --repo dan-hart/momentum --ref main \
  -f version=0.4.11 -f notarized=false
```

Use the released Mac app's actual notarization state. Blank manual version/tooling
inputs resolve from the current workflow source. The production caller supplies
the exact release version, tooling revision, notarization result and trusted public
fingerprint. An older tag still contains its original workflow; use this shared
workflow for existing assets instead of expecting a tag rerun to rewrite history.
Run the [Homebrew package checks](TESTING.md#homebrew-release-packages-f-068b-161)
when changing package definitions or templates.

The workflow publishes desktop apps, CLI archives and verified Android APK/AAB/source
artifacts. The iOS source version also advances, but this workflow does not submit an
iOS build to App Store Connect. Android store submissions are separate from these
GitHub release assets.

## Install channels after a release

```sh
brew install --cask dan-hart/tap/momentum     # macOS app, mo on PATH
brew install dan-hart/tap/momentum-cli        # mo alone, macOS or Linux
flatpak install --user https://dan-hart.github.io/momentum/momentum.flatpakref
```

The cask and the formula both provide `mo`, so install one or the other; the cask is the
one for people who want the app.

## Secrets

| Secret | Used by | Without it |
|---|---|---|
| `FLATPAK_GPG_PRIVATE_KEY`, `FLATPAK_GPG_KEY_ID` | Publish Flatpak repository | required; release preflight fails. Preserve the existing repository signing identity |
| `HOMEBREW_TAP_SSH_KEY` | Update Homebrew tap | required; release preflight fails. Private half of the dedicated write-enabled deploy key on `dan-hart/homebrew-tap`; stored only as an encrypted Momentum Actions secret |
| `MACOS_CERTIFICATE_P12`, `MACOS_CERTIFICATE_PASSWORD` | macOS app and mo | the app and `mo` are signed ad hoc, and the cask tells users to install with `--no-quarantine`. The P12 is the *Developer ID Application* certificate with its private key, base64-encoded |
| `APPLE_ID`, `APPLE_TEAM_ID`, `APPLE_APP_PASSWORD` | macOS app and mo | the app is not notarized. The password is an app-specific password for the Apple ID; the team ID is also needed for Developer ID signing |
| `LIBRESYNC_TOKEN` | every LibreSync checkout | falls back to the workflow token, which is enough while LibreSync is public |
| `ANDROID_KEYSTORE_BASE64`, `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`, `ANDROID_KEY_PASSWORD` | Android release build only | all absent: unsigned APK/AAB remain attached with explicit filenames/instructions; partially configured: build fails. The keystore is base64-encoded and temporary on the runner; no development key is used for distribution |

`FLATPAK_GPG_FINGERPRINT` is a required public repository variable, not a secret.
It pins the existing full primary fingerprint
`07BA47734E3A0B72D20267940D8E046EFEEF00F0`. Preflight validates its format;
exports and public readback require that exact identity. The hosted job uses the
built-in workflow token with Contents and Pages write permissions to publish the
repository and explicitly request its legacy Pages build. It needs no Flathub token.

Keep an offline backup of the Flatpak signing key: a lost key means a new repository for
every user.

Choose and back up the Android app signing identity before the first signed release.
The workflow does not generate one or enroll an app in Play App Signing. It removes
its temporary decoded key after the job and does not pass signing secrets to ordinary
push/PR builds. The bundle alignment fixture uses a separate disposable validation key;
its APK is never included in the release. Actual store submission needs separate approval.

## Self-hosted Linux distribution

The user-confirmed Linux shipping channel is the signed Flatpak repository on
[GitHub Pages](https://dan-hart.github.io/momentum/). The release workflow publishes
both stable architectures there; it does not require Flathub app admission.
As verified 2026-10-22 (public readback 2026-10-23 UTC), both refs are **0.4.12**
with valid signatures from the existing trusted primary fingerprint. The full
[0.4.12 workflow](https://github.com/dan-hart/momentum/actions/runs/39881411888)
and independent disposable Linux-client readback pass; exact OSTree commits and
native verification limits are recorded in [PROGRESS.md](PROGRESS.md).
New installs use momentum.flatpakref; installed copies update through the existing remote:

```sh
flatpak update --user io.github.dan_hart.Momentum
```

Static index.html and momentum.flatpakrepo live on gh-pages and must be inspected
alongside source branding changes; preserve their URLs, public key and signed
OSTree files when changing descriptive copy. Future Flathub admission is deferred
and does not block this self-hosted channel.

## Flathub

Momentum is not on Flathub yet. As checked on 2026-10-13, initial
[PR #10133](https://github.com/flathub/flathub/pull/10133) is closed for a missing
submission checklist and `flathub/io.github.dan_hart.Momentum` does not exist.
Future releases generate no Flathub submission archive and open no Flathub PR.
Historical archives, including 0.4.11, remain immutable reference artifacts; they
do not prove admission or submission eligibility. The build still obtains GNOME
runtime/SDK dependencies from Flathub.

Follow the [current submission guide](https://docs.flathub.org/docs/for-app-authors/submission)
and [AI-content policy](https://docs.flathub.org/docs/for-app-authors/requirements#generative-ai-policy).
The policy requires disclosure of known AI-generated app/packaging material,
prohibits AI-assisted manifests, and requires humans to handle submission PRs,
commit messages, descriptions and review interactions. A human must review
provenance, author compliant packaging and handle the existing closed PR under
its review guidance if admission is requested later. The former token-gated
submission job has been removed. Lint checks remain useful diagnostics:

```sh
flatpak run --command=flatpak-builder-lint org.flatpak.Builder manifest io.github.dan_hart.Momentum.json
flatpak run --command=flatpak-builder-lint org.flatpak.Builder appstream data/io.github.dan_hart.Momentum.metainfo.xml.in.in
```

## Building a release locally

The same script CI uses, producing the same files in `dist/`:

```sh
macos/scripts/package-app.sh dist            # ad hoc signed unless MACOS_SIGN_IDENTITY is set
REQUIRE_UNIVERSAL=1 macos/scripts/package-app.sh dist    # fail unless both architectures are in
```

For a universal binary, `rustup target add x86_64-apple-darwin` first. Flatpak bundles
come from the release manifest, as in the README's *Build from source*.

## Android artifacts and future store distribution

The native Android app targets API 37 with an API 26 minimum, arm64-v8a/x86_64,
Kotlin/Compose and generated UniFFI from the source-built Rust core. The same source
build produces APK and Android App Bundle artifacts; ordinary functionality has no
Google Play services, store SDK, proprietary account, advertising or analytics
dependency. F-Droid compatibility and future alternative-store/Google Play delivery
are approved implementation requirements. Uploading, signing for distribution,
submitting to a repository/store, tagging and releasing require an explicit request.

Follow [Android source setup](../android/README.md) and
[isolated testing](TESTING.md#android-source-and-native-acceptance) first. From
`android/`, with the pinned public source and SDK/toolchain installed:

```sh
./gradlew :app:testDebugUnitTest :app:assembleRelease :app:bundleRelease :app:lintRelease
cd ..
python3 android/scripts/build_core.py --verify-source
python3 android/scripts/build_core.py --check-apk android/app/build/outputs/apk/release/app-release-unsigned.apk
```

Omit the development-only `momentumRustProfile=debug` override for distribution.
Unsigned APK: `android/app/build/outputs/apk/release/app-release-unsigned.apk`.
AAB: `android/app/build/outputs/bundle/release/app-release.aab`. The app's
`versionName` reads Cargo's workspace version; `versionCode` derives as
`major*1_000_000 + minor*1000 + patch`, with bounded fields so stable version increases
produce increasing Android codes within the store limit. Before release, inspect the
actual APK/AAB version/application ID/SDK metadata against the approved version and
previous published code; source formulas alone are not artifact evidence.

Release signing is optional and outside source. Supply all four environment variables
from a private local secret manager/shell environment: `MOMENTUM_KEYSTORE` (absolute
path), `MOMENTUM_KEYSTORE_PASSWORD`, `MOMENTUM_KEY_ALIAS`, `MOMENTUM_KEY_PASSWORD`.
Partial configuration fails early; no default release key is provided. When configured,
the APK becomes `app-release.apk`; run the same alignment check against that actual
signed artifact. Do not put private keys/passwords in Gradle arguments, logs, repository
files or bundled assets. Local build commands do not upload artifacts.

Choose certificate/update continuity before the first distribution. Direct APKs and
store-generated APKs can update each other only with compatible app signing identity.
Play upload and app signing keys can differ; independently distributed APKs need an
app signing key available to their signer. Conventional F-Droid repository signing
uses a different certificate from an upstream-signed APK, so the same application ID
does not make those installations interchangeable. Retaining an upstream signature
requires F-Droid's separately verified reproducible-build process. See
[Android signing](https://developer.android.com/studio/publish/app-signing) and
[F-Droid reproducible builds](https://f-droid.org/en/docs/Reproducible_Builds/).

Before marking a release ready, record current source/artifact hashes and fresh tests,
lint, metadata, packaged assets/notices and signatures. The native checker inspects
every bundled `.so`, including Rust and JNA, for ELF LOAD/RELRO alignment and APK ZIP
16 KiB alignment. Use checksum-verified bundletool from
[toolchain.json](../android/toolchain.json) to validate/dump the AAB configuration;
`PAGE_ALIGNMENT_16K` must be present. Generate actual APKs from the bundle, then check
their native/ZIP alignment and run them on a confirmed 16 KiB runtime. A local APK
alignment pass or earlier debug component tests do not verify a later release/bundle.
See [Android page-size guidance](https://developer.android.com/guide/practices/page-sizes).

The AndroidX Graphics Path 1.1.0 JNI replacement must come from the
[pinned unchanged Apache-licensed release sources](../android/vendor/GRAPHICS-PATH.md),
built with both NDK r27 page-size flags. Retain its source/build provenance and
licenses in distribution archives and packaged notices. Verify final APK/AAB JNI
bytes and native rendering on API26 and a confirmed 16 KiB device; a standalone
library alignment/reproducibility pass does not establish those later artifacts.

Preserve Momentum GPL-3.0-or-later, LibreSync's AGPL-3.0-only source/license and the
complete dependency notices/corresponding source. The build generates
`assets/THIRD_PARTY_NOTICES.txt` in APK/AAB from resolved public Maven POM/artifact
license files, Rust source package notices and retained native JNA/libffi notices.
Check the packaged output, not only the source Markdown. Source archives must identify
the exact pinned inputs; [distribution audit](../android/DISTRIBUTION_AUDIT.md) and
[third-party notices](../android/THIRD_PARTY_NOTICES.md) record provenance and limits.

The [F-Droid recipe](../android/fdroid/README.md) and metadata are drafts. They build
both native ABIs from public pinned Momentum/LibreSync/Rust sources, use offline Cargo
vendoring and do not bypass scanners. Before submission, select an approved source
revision, run the actual F-Droid builder/scanner, verify license/source availability,
and compare independently prepared builds for reproducibility. Admission, successful
F-Droid builds and bit-for-bit reproducibility have not been established locally.
The [Android workflow](../.github/workflows/android.yml) checks source pins, Python/JVM
tests, APK/AAB, lint, notices and alignment on ordinary pushes/PRs without signing or
uploads. The Release caller selects the exact tag, optionally supplies signing secrets,
packages verified artifacts and uploads only `android-release` for the GitHub Release.
The packager builds and runs JVM tests/release lint in a fresh `git archive` snapshot
of the selected commit. Existing outputs and untracked/ignored local source are never
release inputs. It rejects dirty tracked source, nonempty output directories, wrong
versions/SDK/application IDs, debuggable artifacts and debug signing keys. Both APK
and every signed AAB payload must match the configured distribution certificate;
unsigned appended entries, altered signed contents and mismatched signers fail.
It checks alignment on the actual AAB's generated APK with a disposable validation
key, which is deleted with the snapshot and never attached to the release.
Its source archive includes committed Momentum, verified LibreSync, original locked
Rust crates, native sources/licenses and a relocatable Cargo vendor configuration;
SDK/NDK/Rust and Maven/Gradle inputs remain prerequisites in the build guide.
`build-aux/release.py checksums --assets dist` includes every top-level distribution
file and excludes `SHA256SUMS.txt` itself. The first remote Android/tagged release
execution remains unverified until an explicitly requested push/tag/run occurs.
