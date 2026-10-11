<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# F-Droid source recipe

The draft metadata is disabled until an Android release is approved and a real
F-Droid build-server run passes. It targets the same application and source as
the direct APK and Google Play AAB; there is no GMS/Firebase/telemetry dependency,
paywall, nonfree flavor or network requirement for local task use.

Copy `metadata/com.codedbydan.momentum.yml` and `srclibs/LibreSync.yml` into a local
fdroiddata checkout. Prepare JDK 17, the pinned Rust 1.99.0 toolchain and its two
Android targets in the builder image, SDK API 37.0, build-tools 37.0.0 and
NDK 27.2.12479018. Set `commit` to the approved source revision and remove the
per-build `disable` field only for a deliberate validation run. F-Droid obtains
the pinned LibreSync source; prebuild verifies it and vendors Cargo's locked
dependencies. The [source preparation](SOURCE_PREPARATION.md) removes exact reviewed
Windows/wasm target inputs and upstream test/example fixtures from this Android-only
vendor copy, preserving code, licenses, lock/package checksums and remaining file
hashes. Cargo's later build/notice commands are forced offline by the generated
Cargo configuration. Gradle's preBuild task builds both native ABIs and generates bindings.
The Graphics Path AAR retains its published Kotlin/API classes while its JNI is
rebuilt from the checked-in, checksum-pinned unchanged Apache-licensed
[matching source](../vendor/GRAPHICS-PATH.md), with both 16 KiB linker page flags.
The APK bundles that source license and rebuild provenance with the other notices.
The unsigned APK is signed by the repository rather than an upstream private key.

Run `fdroid lint com.codedbydan.momentum`, then
`fdroid build --test com.codedbydan.momentum:4005` in that prepared environment.
The schema and command fields follow the official
[metadata reference](https://f-droid.org/docs/Build_Metadata_Reference/).
No scanner exclusions are included. The wrapper is a standard Gradle wrapper,
with a pinned distribution checksum; generated native libraries are absent from Git.
The [local validation record](VALIDATION.md) distinguishes the passing metadata
lint/source scan and isolated native compilation from a real F-Droid builder run.

Remaining release gates:

- Validate enabled-build semantics and Rust provisioning/offline dependency setup
  in the actual F-Droid Linux builder. Local disabled metadata lint and prepared
  source scanning passed; Linux builder execution remains outstanding.
- Run F-Droid's dependency/APK scanners against the resolved build. Its
  current [inclusion policy](https://f-droid.org/en/docs/Inclusion_Policy/) allows
  freely licensed binaries from trusted Maven repositories. JNA's upstream source,
  native/AAR assembly recipe and reviewed Maven hash are in the
  [distribution audit](../DISTRIBUTION_AUDIT.md); the app does not rebuild JNA locally.
  This policy permits a path for review, not an admission guarantee. Do not add
  scanner bypasses if the actual review identifies a dependency problem.
- Collect transitive dependency notices and matching source archives, preserving
  Momentum's GPL-3.0-or-later sources and LibreSync's AGPL-3.0-only notice.
- Inspect both architectures, APK alignment, AAB-generated APKs and native behavior
  on a 16 KiB emulator/device. Complete accessibility/offline/sync acceptance.
- Compare unsigned outputs from independently prepared builders before claiming
  bit-for-bit reproducibility. Admission requires F-Droid's own review.
- Decide update-signing continuity. Repository signing differs from the upstream
  signing key; sharing an application ID does not make those APKs interchangeable.
  Retaining the upstream signature needs the verified reproducible-build process.

Nothing in this directory uploads an APK, submits a listing, or authorizes release.
