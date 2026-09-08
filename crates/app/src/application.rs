// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::{gdk, gio, glib};
use tracing::{debug, info};

use crate::config::{APP_ID, PKGDATADIR, PROFILE, VERSION};
use crate::window::MomentumWindow;

mod imp {
    use super::*;
    use glib::WeakRef;
    use std::cell::OnceCell;

    #[derive(Debug, Default)]
    pub struct MomentumApplication {
        pub window: OnceCell<WeakRef<MomentumWindow>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MomentumApplication {
        const NAME: &'static str = "MomentumApplication";
        type Type = super::MomentumApplication;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for MomentumApplication {}

    impl ApplicationImpl for MomentumApplication {
        fn activate(&self) {
            debug!("AdwApplication<MomentumApplication>::activate");
            self.parent_activate();
            let app = self.obj();

            if let Some(window) = self.window.get() {
                let window = window.upgrade().unwrap();
                window.present();
                return;
            }

            let window = MomentumWindow::new(&app);
            self.window
                .set(window.downgrade())
                .expect("Window already set.");

            app.main_window().present();
        }

        fn startup(&self) {
            debug!("AdwApplication<MomentumApplication>::startup");
            self.parent_startup();
            let app = self.obj();

            gtk::Window::set_default_icon_name(*APP_ID);

            app.setup_css();
            app.setup_gactions();
            app.setup_accels();
        }
    }

    impl GtkApplicationImpl for MomentumApplication {}
    impl AdwApplicationImpl for MomentumApplication {}
}

glib::wrapper! {
    pub struct MomentumApplication(ObjectSubclass<imp::MomentumApplication>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionMap, gio::ActionGroup;
}

impl MomentumApplication {
    fn main_window(&self) -> MomentumWindow {
        self.imp().window.get().unwrap().upgrade().unwrap()
    }

    fn setup_gactions(&self) {
        let action_quit = gio::ActionEntry::builder("quit")
            .activate(move |app: &Self, _, _| {
                // Triggers close_request so the window state is saved.
                app.main_window().close();
                app.quit();
            })
            .build();

        let action_about = gio::ActionEntry::builder("about")
            .activate(|app: &Self, _, _| {
                app.show_about_dialog();
            })
            .build();
        self.add_action_entries([action_quit, action_about]);
    }

    fn setup_accels(&self) {
        self.set_accels_for_action("app.quit", &["<Control>q"]);
        self.set_accels_for_action("window.close", &["<Control>w"]);
    }

    fn setup_css(&self) {
        let provider = gtk::CssProvider::new();
        provider.load_from_resource("/io/github/danhart/Momentum/style.css");
        if let Some(display) = gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }

    fn authors() -> Vec<&'static str> {
        env!("CARGO_PKG_AUTHORS").split(':').collect()
    }

    fn show_about_dialog(&self) {
        let dialog = adw::AboutDialog::builder()
            .application_name(gettext("Momentum"))
            .application_icon(*APP_ID)
            .developer_name("Dan Hart")
            .license_type(gtk::License::Gpl30)
            .website("https://github.com/danhart/momentum")
            .issue_url("https://github.com/danhart/momentum/issues")
            .version(*VERSION)
            .translator_credits(gettext("translator-credits"))
            .developers(Self::authors())
            .build();

        dialog.present(Some(&self.main_window()));
    }

    pub fn run(&self) -> glib::ExitCode {
        info!("Momentum ({})", *APP_ID);
        info!("Version: {} ({})", *VERSION, *PROFILE);
        info!("Datadir: {}", *PKGDATADIR);
        info!("Super Productivity schema version: {}", sp_model::SCHEMA_VERSION);

        ApplicationExtManual::run(self)
    }
}

impl Default for MomentumApplication {
    fn default() -> Self {
        glib::Object::builder()
            .property("application-id", *APP_ID)
            .property("resource-base-path", "/io/github/danhart/Momentum/")
            .build()
    }
}
