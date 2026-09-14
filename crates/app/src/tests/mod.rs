// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! App-level tests. `support` owns the single GTK thread every test runs on; `logic`
//! covers pure helpers; the window tests live in `ui.rs` as a child of `window` so they
//! can drive private methods. Run with `build-aux/run-tests.sh` (see docs/TESTING.md).
pub mod support;

mod logic;
