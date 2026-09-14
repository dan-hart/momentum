// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! The primary modifier for every app shortcut: Ctrl by default, switchable to Alt or
//! Super on Linux and to Option or Command on macOS. Accelerators are written once with
//! `<Control>` and rewritten through [`accel`]; hints shown to the user go through
//! [`label`].
use gettextrs::gettext;
use gtk::gio;
use gtk::prelude::*;

/// `(setting value, human label)` in the order shown in Preferences, for this platform.
pub fn choices() -> Vec<(&'static str, String)> {
    if cfg!(target_os = "macos") {
        vec![
            ("command", gettext("Command")),
            ("control", gettext("Control")),
            ("option", gettext("Option")),
        ]
    } else {
        vec![
            ("control", gettext("Ctrl")),
            ("alt", gettext("Alt")),
            ("super", gettext("Super")),
        ]
    }
}
pub fn current() -> String {
    gio::Settings::new(*crate::config::APP_ID)
        .string("modifier-key")
        .to_string()
}
/// The GTK accelerator token for a setting value.
pub fn token(value: &str) -> &'static str {
    match value {
        "alt" | "option" => "<Alt>",
        "super" => "<Super>",
        // GTK names the Command key Meta on macOS; on Linux the closest key is Super.
        "command" if cfg!(target_os = "macos") => "<Meta>",
        "command" => "<Super>",
        _ => "<Control>",
    }
}
/// Rewrites an accelerator written with `<Control>` for the configured modifier.
pub fn accel(s: &str) -> String {
    s.replace("<Control>", token(&current()))
}
/// The modifier's name as shown in hints, e.g. "Ctrl" or "Super".
pub fn label() -> String {
    let v = current();
    choices()
        .into_iter()
        .find(|(k, _)| *k == v)
        .map(|(_, l)| l)
        .unwrap_or_else(|| gettext("Ctrl"))
}
/// "{mod}+N" with the configured modifier's name.
pub fn hint(keys: &str) -> String {
    format!("{}+{keys}", label())
}
