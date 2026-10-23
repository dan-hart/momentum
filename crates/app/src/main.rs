// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart

mod application;
mod background_status;
mod celebration_colors;
mod config;
mod demo;
mod empty_messages;
mod keyring;
mod messages;
mod modifier;
mod p2p;
mod portable_settings;
mod prefs;
mod quick_add;
mod repeat_dialog;
mod search_provider;
mod settings_ui;
mod shortcuts;
mod sound;
mod task_form;
mod typography;
mod update_policy;
mod updates;
mod window;

#[cfg(test)]
mod tests;

use gettextrs::{gettext, LocaleCategory};
use gtk::{gio, glib};

use self::application::MomentumApplication;
use self::config::{GETTEXT_PACKAGE, LOCALEDIR, RESOURCES_FILE};

fn main() -> glib::ExitCode {
    // fmt::init also bridges the `log` facade used by LibreSync and mdns-sd.
    tracing_subscriber::fmt::init();

    gettextrs::setlocale(LocaleCategory::LcAll, "");
    gettextrs::bindtextdomain(*GETTEXT_PACKAGE, *LOCALEDIR).expect("Unable to bind the text domain");
    gettextrs::textdomain(*GETTEXT_PACKAGE).expect("Unable to switch to the text domain");

    glib::set_application_name(&gettext("Momentum"));

    let res = gio::Resource::load(*RESOURCES_FILE).expect("Could not load gresource file");
    gio::resources_register(&res);

    let app = MomentumApplication::default();
    app.run()
}
