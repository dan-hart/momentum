<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Android third-party notices

Momentum source: **GPL-3.0-or-later**, [LICENSE](../LICENSE).
Keep existing copyright/license headers and supply corresponding source with builds.

| Component | License | Source / license |
| --- | --- | --- |
| LibreSync v0.6.0 (Rust library linked in momentum_ffi) | AGPL-3.0-only | [Pinned source](https://github.com/dan-hart/LibreSync/tree/839c98c0386ee6a5b8ba2f298de48fa0bf2dacaa), [license](https://github.com/dan-hart/LibreSync/blob/839c98c0386ee6a5b8ba2f298de48fa0bf2dacaa/LICENSE) |
| UniFFI 0.32.1 | MPL-2.0 | [Source and license](https://github.com/mozilla/uniffi-rs) |
| Rust 1.99.0 standard library | Apache-2.0 or MIT, with upstream listed exceptions | [Pinned source](https://github.com/rust-lang/rust/tree/b940084d7eb6a299eb4bfeb8e34901bc051e7ac4/library), [complete upstream inventory](licenses/Rust-1.99.0-COPYRIGHT-library.html) |
| AndroidX / Jetpack Compose / WorkManager / DataStore | Apache-2.0 | [Source](https://android.googlesource.com/platform/frameworks/support/), [license](https://android.googlesource.com/platform/frameworks/support/+/androidx-main/LICENSE.txt) |
| Kotlin standard library / Compose compiler | Apache-2.0 | [Source and license](https://github.com/JetBrains/kotlin) |
| kotlinx.coroutines 1.11.0 | Apache-2.0 | [Source and license](https://github.com/Kotlin/kotlinx.coroutines) |
| JNA 5.19.1 | Apache-2.0 or LGPL-2.1-or-later; Apache option used | [Source](https://github.com/java-native-access/jna/tree/5.19.1), [license](https://github.com/java-native-access/jna/blob/5.19.1/LICENSE) |
| Desugared JDK libraries | GPL-2.0 with Classpath exception and Apache-2.0 portions | [Source and notices](https://github.com/google/desugar_jdk_libs) |
| JUnit 4 (tests only) | EPL-1.0 | [Source and license](https://github.com/junit-team/junit4) |

Rust transitive dependencies and checksums are recorded by the workspace
[Cargo.lock](../Cargo.lock); Gradle dependencies are declared in the
[version catalog](gradle/libs.versions.toml). The release process must collect
transitive copyright/license notices alongside the complete matching source tree.
`package_notices.py` generates a full license/notice asset from resolved runtime
JAR/AARs and the Android Rust dependency graph. Checked-in upstream notices have
their original bytes and provenance hashes in [licenses/provenance.json](licenses/provenance.json).
The APK/AAB includes `assets/THIRD_PARTY_NOTICES.txt`; source documents are not the
only delivered notices. This inventory is not a completed distribution license audit.

The Rust standard-library inventory and Apache/MIT/BSD/Unicode/LLVM-exception
texts preserve the exact installed official `rust-docs` component bytes for
Rust 1.99.0, commit `b940084d7eb6a299eb4bfeb8e34901bc051e7ac4`, matching
`rustc --version --verbose` and the installed channel manifest. Provenance records
the public distribution URL, its manifest archive checksum, component-relative
paths and individual file hashes. No private installation paths are included.
The complete upstream HTML also retains the embedded licenses/copyrights of its
out-of-tree dependencies and includes other targets/build inputs; it is not an
assertion that every listed item is linked into Android. These files enter the
same supplemental notice pipeline and the APK/AAB asset, beyond Cargo metadata.
