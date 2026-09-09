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
            self.window.set(window.downgrade()).expect("Window already set.");

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
            crate::shortcuts::register_global(&app);
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
    pub fn main_window(&self) -> MomentumWindow {
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
        let win = |f: fn(&MomentumWindow)| {
            move |app: &Self, _: &gio::SimpleAction, _: Option<&glib::Variant>| f(&app.main_window())
        };
        let simple = [
            gio::ActionEntry::builder("preferences")
                .activate(win(|w| crate::prefs::MomentumPrefs::default().present(Some(w))))
                .build(),
            gio::ActionEntry::builder("sync").activate(win(|w| w.sync())).build(),
            gio::ActionEntry::builder("import")
                .activate(win(|w| w.import_backup()))
                .build(),
            gio::ActionEntry::builder("export")
                .activate(win(|w| w.export_backup()))
                .build(),
            gio::ActionEntry::builder("add-task")
                .activate(win(|w| w.focus_add()))
                .build(),
            gio::ActionEntry::builder("new-project")
                .activate(win(|w| {
                    let entry = gtk::Entry::builder()
                        .placeholder_text(gettext("Project name"))
                        .activates_default(true)
                        .build();
                    let d = adw::AlertDialog::builder()
                        .heading(gettext("New Project"))
                        .extra_child(&entry)
                        .default_response("add")
                        .build();
                    d.add_responses(&[("cancel", &gettext("Cancel")), ("add", &gettext("Add"))]);
                    d.set_response_appearance("add", adw::ResponseAppearance::Suggested);
                    d.connect_response(
                        None,
                        glib::clone!(
                            #[weak]
                            w,
                            #[weak]
                            entry,
                            move |_, r| if r == "add" {
                                w.add_project(&entry.text());
                            }
                        ),
                    );
                    d.present(Some(w));
                }))
                .build(),
        ];
        self.add_action_entries(simple);
        self.add_action_entries([action_quit, action_about]);
    }

    fn setup_accels(&self) {
        self.set_accels_for_action("app.quit", &["<Control>q"]);
        self.set_accels_for_action("window.close", &["<Control>w"]);
        self.set_accels_for_action("app.preferences", &["<Control>comma"]);
        self.set_accels_for_action("app.add-task", &["<Control>n"]);
        self.set_accels_for_action("app.sync", &["<Control>r", "F5"]);
        self.set_accels_for_action("app.new-project", &["<Control><Shift>n"]);
        self.set_accels_for_action("win.search", &["<Control>f"]);
        self.set_accels_for_action("win.toggle-done", &["<Control>d"]);
        self.set_accels_for_action("win.delete-task", &["Delete"]);
        self.set_accels_for_action("win.plan-today", &["<Control>t"]);
        self.set_accels_for_action("win.move-to", &["<Control>m"]);
        self.set_accels_for_action("win.toggle-tonight", &["<Control><Shift>t"]);
        self.set_accels_for_action("win.move-tomorrow", &["<Control><Shift>Right"]);
        self.set_accels_for_action("win.move-next-week", &["<Control><Shift>Down"]);
        self.set_accels_for_action("win.select-all", &["<Control>a"]);
        self.set_accels_for_action("win.select-none", &["<Control><Shift>a"]);
        self.set_accels_for_action("win.archive-done", &["<Control>e"]);
        self.set_accels_for_action("win.focus-add", &["<Control>l"]);
        self.set_accels_for_action("win.toggle-sidebar", &["F9"]);
        self.set_accels_for_action("win.next-view", &["<Control>Page_Down"]);
        self.set_accels_for_action("win.prev-view", &["<Control>Page_Up"]);
        self.set_accels_for_action("win.move-up", &["<Control>Up"]);
        self.set_accels_for_action("win.move-down", &["<Control>Down"]);
        self.set_accels_for_action("win.duplicate", &["<Control><Shift>d"]);
        self.set_accels_for_action("win.copy-title", &["<Control><Shift>c"]);
        self.set_accels_for_action("win.open-focused", &["<Control>o"]);
    }

    fn setup_css(&self) {
        let provider = gtk::CssProvider::new();
        provider.load_from_resource("/io/github/dan_hart/Momentum/style.css");
        if let Some(display) = gdk::Display::default() {
            gtk::style_context_add_provider_for_display(&display, &provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
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
            .website("https://github.com/dan-hart/momentum")
            .issue_url("https://github.com/dan-hart/momentum/issues")
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
            .property("resource-base-path", "/io/github/dan_hart/Momentum/")
            .build()
    }
}
