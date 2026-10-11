<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# AndroidX Graphics Path JNI source

`graphics-path/` contains fourteen unchanged source/build/license files (68,547
bytes) from the AndroidX Graphics Path 1.1.0 release snapshot
[`1c44f0be720ff16cd30357ad2375239bf97a1da8`](https://android.googlesource.com/platform/frameworks/support/+/1c44f0be720ff16cd30357ad2375239bf97a1da8/graphics/graphics-path/).
The [official release notes](https://developer.android.com/jetpack/androidx/releases/graphics#graphics-path-1.1.0)
identify this snapshot as the May 6, 2026 release. All five Kotlin files in the
published sources JAR were compared byte-for-byte with the same snapshot. The
published Kotlin classes stay unchanged; only the two packaged JNI libraries are
compiled locally. Every retained input's upstream path and SHA-256, the Kotlin
source hashes, and the Maven AAR/classes hashes are in
[toolchain.json](../toolchain.json).

All retained files preserve their upstream copyright/license text. The upstream
root [Apache-2.0 license](graphics-path/LICENSE.txt) is retained; this module and
snapshot have no separate upstream NOTICE file. The source remains Apache-2.0;
Momentum's integration scripts remain GPL-3.0-or-later. The corresponding release
artifact license and the source license/provenance are included in the app's
generated notices. No binary is kept in this source directory.

The release's CMake file supplies `max-page-size=16384`, but omits
`common-page-size=16384`. Both the published 1.0.1 and 1.1.0 native libraries fail
the current RELRO endpoint check even though their LOAD alignment is 16 KB.
[Android's NDK r27 guidance](https://developer.android.com/guide/practices/page-sizes)
requires both flags. `build_graphics_path.py` uses pinned NDK 27.2.12479018, API26,
the original three translation units, compiler options and export map. Its local
build adds both page flags, RELRO/NOW protection and deterministic build settings.
It verifies LOAD alignment, RELRO endpoint alignment and the sole `JNI_OnLoad`
export before packaging. It adds no C++ shared-runtime dependency or product SDK.

The Gradle artifact transform builds into its own workspace and replaces the
JNI entries of the checksum-verified 1.1.0 AAR. Every non-JNI entry, including
`classes.jar`, resources, manifest, ProGuard metadata and license bytes, stays
byte-for-byte identical. Unrelated AARs pass through unchanged. Unsupported 32-bit
JNI entries are omitted because Momentum's supported ABIs are arm64-v8a/x86_64.
No ELF is patched, and the APK checker remains strict.

```sh
python3 android/scripts/build_graphics_path.py --verify-source
python3 android/scripts/build_graphics_path.py
python3 -m unittest android/scripts/tests/test_build_graphics_path.py
```

The initial dirty-worktree native builds reproduced byte-for-byte across two
separate output directories on macOS. Independent Linux/F-Droid reproducibility
and final APK/AAB/runtime validation remain separate gates. `NativeGraphicsPathTest`
checks real iterator order/control points and conic conversion followed by native
Canvas raster coverage, including the API26 implementation boundary.
