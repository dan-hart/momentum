// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart

use std::sync::LazyLock;

/// Declares a lazily-initialized static variable that reads its value from a
/// compile-time environment variable with a `MESON_` prefix. Meson sets them for every
/// real build; the fallback lets `cargo test -p momentum` compile and run outside Meson
/// (see `build-aux/run-tests.sh`), always as a Devel profile.
macro_rules! config_var {
    ($name:ident, $fallback:expr) => {
        pub static $name: LazyLock<&'static str> =
            LazyLock::new(|| option_env!(concat!("MESON_", stringify!($name))).unwrap_or($fallback));
    };
}

config_var!(APP_ID, "io.github.dan_hart.Momentum.Devel");
config_var!(GETTEXT_PACKAGE, "momentum");
config_var!(LOCALEDIR, "/usr/share/locale");
config_var!(PKGDATADIR, "/app/share/momentum");
config_var!(PROFILE, "Devel");
config_var!(VERSION, concat!(env!("CARGO_PKG_VERSION"), "-test"));
config_var!(RESOURCES_FILE, "");
