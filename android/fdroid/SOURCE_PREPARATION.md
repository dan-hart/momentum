<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Android-only vendored source preparation

F-Droid scans the whole vendored tree, including dependencies for other targets
and upstream test inputs. `cargo vendor --locked --versioned-dirs` originally
produced 2,837 fatal binary findings in the local source scan. The preparation
script removes only reviewed inputs unused by the Android application and host
binding generator. It does not suppress scanner findings or alter Rust code.

Run the script only on the isolated vendor copy produced by the recipe:

```sh
python3 android/scripts/prepare_fdroid_source.py \
  --vendor vendor --lock Cargo.lock \
  --report android/rust-target/fdroid-source-preparation.json
```

The tool checks package identities and original package checksums against
Cargo.lock, verifies every vendor file against its existing file checksum, and
rejects unknown files, altered code/licenses, changed reviewed guards, and
symlinks before deleting anything. It records every removed path, original hash,
reason, and original/prepared checksum-metadata hash. Unknown versions and other
binary inputs remain visible to the scanner.

Cargo verifies every file listed in a compiled directory-source package, even an
unused test fixture. A fresh build with deleted fixtures and unchanged inventories
failed while opening `flate2-1.1.10/tests/multi.gz`. The script consequently removes
only each verified deleted path's entry from `.cargo-checksum.json`'s `files` map.
The original `package` checksum, Cargo.lock, and every remaining file checksum
stay unchanged. This uses Cargo's existing
[directory-source checksum format and verifier](https://raw.githubusercontent.com/rust-lang/cargo/master/src/sources/directory.rs),
not a new checksum for modified Rust source. See also the
[Cargo source-replacement documentation](https://doc.rust-lang.org/cargo/reference/source-replacement.html).

## Windows import libraries

The two `winapi-{i686,x86_64}-pc-windows-gnu` 0.4.0 crates contain 2,803
`lib/libwinapi_*.a` import archives. Their build scripts emit a library search path
only for the corresponding Windows GNU target. `winapi` 0.3.9 selects these crates
under its exact `i686-pc-windows-gnu` and `x86_64-pc-windows-gnu` dependency tables.
No Linux, macOS, or Android target matches those tables.

The 0.48.5 and 0.52.6 Windows target crates contain another 15 import archives:

| Version | Exact crate-name suffixes after `windows_` | Removed path |
| --- | --- | --- |
| 0.48.5 | `aarch64_gnullvm`, `i686_gnu`, `x86_64_gnu`, `x86_64_gnullvm` | `lib/libwindows.0.48.5.a` |
| 0.48.5 | `aarch64_msvc`, `i686_msvc`, `x86_64_msvc` | `lib/windows.0.48.5.lib` |
| 0.52.6 | `aarch64_gnullvm`, `i686_gnu`, `i686_gnullvm`, `x86_64_gnu`, `x86_64_gnullvm` | `lib/libwindows.0.52.0.a` |
| 0.52.6 | `aarch64_msvc`, `i686_msvc`, `x86_64_msvc` | `lib/windows.0.52.0.lib` |

Version 0.48.5 build scripts return unless their Windows target matches. Version
0.52.6 build scripts add `lib` unconditionally, but their parent `windows-targets`
dependency tables select them by Windows target, GNU/MSVC environment, and LLVM
ABI. Rust Android has `target_env=""`, rather than `gnu`/`msvc`. Linux users of
`windows-sys` in the resolved graph are gated by `cfg(windows)`; `windows-targets`
also gates the actual `#[link(name="windows...")]` macro with `cfg(windows)`.
Both parent manifests, their link macros, and all affected build scripts were
inspected in the locked registry sources. The tool pins each reviewed build-script
hash. This preparation is unsuitable for a Windows build.

## Exact test, example, documentation, and wasm inputs

[source-pruning.json](source-pruning.json) lists all 25 additional paths and original
hashes, plus the inspected source guards and their hashes. Only those paths are
removed; runtime cryptographic tables, algorithms, and host bindgen source remain.

| Crate | Count | Source evidence and why unused |
| --- | --- | --- |
| `poly1305` 0.8.0 | 9 | `src/lib.rs` gates `mod fuzz` with AVX2 plus `any(fuzzing, test)`; `src/fuzz.rs` includes these regression inputs only in its tests. |
| `askama_parser` 0.16.1 | 1 | `src/lib.rs` declares `mod tests` under `cfg(test)`; `src/tests.rs` includes the parser regression fixture. |
| `digest` 0.11.3 | 1 | `tests/dummy_fixed.rs` consumes the hash serialization fixture in an upstream integration test. |
| `sha2` 0.11.0 | 6 | `tests/mod.rs` consumes the six serialization fixtures through upstream test macros. |
| `ring` 0.17.14 | 3 | `tests/ed25519_tests.rs` and `tests/rsa_tests.rs` include public test keys and a modulus. These files are test inputs, not runtime crypto material. |
| `flate2` 1.1.10 | 3 | `tests/gunzip.rs` reads two compressed fixtures; `examples/decompress_file.rs` documents the compressed example input. |
| `wit-bindgen` 0.57.1 | 2 | `build.rs` returns before reading `src/rt/libwit_bindgen_cabi.a` for `target_family != "wasm"`. The `wasi-cli@0.2.0.wasm` component is referenced by two examples exposed only under `cfg(docsrs)` in `src/lib.rs`. |

These removals are specific to the app's native build. The prepared vendor copy
cannot run those upstream fixture tests/examples or build WIT wasm targets; use
the complete original crates for those purposes. Ordinary source builds do not
perform this preparation. All licenses and source code remain in the prepared copy.

`cargo vendor` obtains the original registry archives verified against Cargo.lock.
Preparation never modifies the registry cache or original `.crate` archives. Keep
those archives alongside distribution source records; they remain obtainable from
the named/versioned crates.io releases with the checksums in Cargo.lock. Retain the
full LibreSync source and licenses as specified by the recipe. A final corresponding
source review remains part of distribution approval.

If preparation stops or its inputs differ, recreate an isolated vendor copy from
the locked original crates rather than repairing source or weakening checksums.
New dependency versions require new source inspection, policy review, fresh offline
builds, and a new scanner run.
