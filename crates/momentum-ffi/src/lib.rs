// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Nothing but a link target. Every exported symbol comes from `momentum-core` built with
//! its `ffi` feature; this crate exists so that feature is only ever compiled into the
//! library the Swift package links, never into the GTK app.
pub use momentum_core::*;
