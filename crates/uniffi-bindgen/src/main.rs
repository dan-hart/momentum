// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! `cargo run -p uniffi-bindgen -- generate --library <libmomentum_ffi.dylib> --language swift --out-dir <dir>`
fn main() {
    uniffi::uniffi_bindgen_main()
}
