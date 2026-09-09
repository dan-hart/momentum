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

    #[derive(Debug, Default)]
    pub struct MomentumApplication {
        pub window: std::cell::RefCell<Option<WeakRef<MomentumWindow>>>,
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

            app.ensure_window().present();
        }

        fn dbus_register(&self, connection: &gio::DBusConnection, object_path: &str) -> Result<(), glib::Error> {
            self.parent_dbus_register(connection, object_path)?;
            crate::search_provider::register(&self.obj(), connection, object_path)
        }

        /// `momentum --add "title"`, `--quick-add`, `--today`, `--search q`, `--background`.
        fn command_line(&self, cmd: &gio::ApplicationCommandLine) -> glib::ExitCode {
            let app = self.obj();
            let opts = cmd.options_dict();
            let win = app.ensure_window();
            let mut show = true;
            if let Some(title) = opts.lookup::<String>("add").ok().flatten() {
                win.add_task_for_today(&title);
                tracing::info!("added from command line: {title}");
                show = false;
            }
            if let Some(q) = opts.lookup::<String>("search").ok().flatten() {
                win.go_to(crate::window::View::Search);
                win.set_search_query(&q);
            }
            if opts.contains("today") {
                win.go_to(crate::window::View::Today);
            }
            if opts.contains("quick-add") {
                crate::quick_add::open(&app);
                return glib::ExitCode::SUCCESS;
            }
            if opts.contains("background") {
                show = false;
            }
            if show {
                app.activate();
            } else if !cmd.is_remote() && !win.imp().settings.boolean("run-in-background") {
                // Launched only to run a command: do not linger with a hidden window.
                glib::idle_add_local_once(glib::clone!(
                    #[weak]
                    app,
                    move || app.quit()
                ));
            }
            glib::ExitCode::SUCCESS
        }

        /// `momentum://add?title=…` and Super Productivity's `superproductivity://create-task?title=…`.
        fn open(&self, files: &[gio::File], _hint: &str) {
            let app = self.obj();
            let win = app.ensure_window();
            for f in files {
                let uri = f.uri().to_string();
                let Ok(parsed) = glib::Uri::parse(&uri, glib::UriFlags::NONE) else {
                    continue;
                };
                let host = parsed.host().map(|h| h.to_string()).unwrap_or_default();
                let query = parsed.query().map(|q| q.to_string()).unwrap_or_default();
                let param = |k: &str| -> Option<String> {
                    query.split('&').find_map(|kv| {
                        let (key, v) = kv.split_once('=')?;
                        (key == k).then(|| {
                            glib::Uri::unescape_string(&v.replace('+', " "), None)
                                .map(|g| g.to_string())
                                .unwrap_or_default()
                        })
                    })
                };
                match host.as_str() {
                    "add" | "create-task" => {
                        if let Some(title) = param("title").filter(|t| !t.trim().is_empty()) {
                            let mut text = title;
                            if let Some(tags) = param("tags") {
                                for t in tags.split(',').map(str::trim).filter(|t| !t.is_empty()) {
                                    text.push_str(&format!(" #{t}"));
                                }
                            }
                            win.add_task_with_notes(&text, param("notes").as_deref(), param("due").as_deref());
                        }
                    }
                    "complete-task" => {
                        if let Some(title) = param("title") {
                            win.complete_by_title(&title);
                        }
                    }
                    _ => tracing::warn!("unknown URL: {uri}"),
                }
            }
            win.present();
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
            // The window exists from startup (hidden) so reminders, sync, the search provider
            // and the CLI have a store to talk to, even when launched as a service.
            app.ensure_window();
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
        self.ensure_window()
    }

    /// The single main window, created hidden on first use.
    pub fn ensure_window(&self) -> MomentumWindow {
        if let Some(w) = self.imp().window.borrow().as_ref().and_then(|w| w.upgrade()) {
            return w;
        }
        let window = MomentumWindow::new(self);
        *self.imp().window.borrow_mut() = Some(window.downgrade());
        window
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
        // `mo` (the CLI) hands actions to a running app over D-Bus: org.gtk.Actions.Activate("cli", [json]).
        let cli = gio::ActionEntry::builder("cli")
            .parameter_type(Some(&String::static_variant_type()))
            .activate(|app: &Self, _, p: Option<&glib::Variant>| {
                let Some(payload) = p.and_then(|v| v.get::<String>()) else {
                    return;
                };
                let w = app.main_window();
                if payload == "\"sync\"" {
                    w.sync();
                } else if let Ok(action) = serde_json::from_str::<sp_oplog::Action>(&payload) {
                    w.dispatch(action);
                } else {
                    tracing::warn!("cli: unrecognised payload");
                }
            })
            .build();
        // Notification buttons (Done / Snooze) carry the task id.
        let notify_done = gio::ActionEntry::builder("notify-done")
            .parameter_type(Some(&String::static_variant_type()))
            .activate(|app: &Self, _, p: Option<&glib::Variant>| {
                if let Some(id) = p.and_then(|v| v.get::<String>()) {
                    app.main_window().complete_task(&id);
                }
            })
            .build();
        let notify_snooze = gio::ActionEntry::builder("notify-snooze")
            .parameter_type(Some(&String::static_variant_type()))
            .activate(|app: &Self, _, p: Option<&glib::Variant>| {
                if let Some(id) = p.and_then(|v| v.get::<String>()) {
                    app.main_window().snooze_task(&id, 60);
                }
            })
            .build();
        self.add_action_entries([cli, notify_done, notify_snooze]);
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
            gio::ActionEntry::builder("quick-add")
                .activate(|app: &Self, _, _| crate::quick_add::open(app))
                .build(),
            gio::ActionEntry::builder("today")
                .activate(win(|w| {
                    w.go_to(crate::window::View::Today);
                    w.present();
                }))
                .build(),
            gio::ActionEntry::builder("search")
                .activate(win(|w| {
                    w.go_to(crate::window::View::Search);
                    w.present();
                }))
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
        self.set_accels_for_action("win.repeat", &["<Control><Shift>r"]);
        self.set_accels_for_action("win.undo", &["<Control>z"]);
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
        let app: Self = glib::Object::builder()
            .property("application-id", *APP_ID)
            .property("resource-base-path", "/io/github/dan_hart/Momentum/")
            .property(
                "flags",
                gio::ApplicationFlags::HANDLES_COMMAND_LINE | gio::ApplicationFlags::HANDLES_OPEN,
            )
            .build();
        app.add_main_option(
            "add",
            b'a'.into(),
            glib::OptionFlags::NONE,
            glib::OptionArg::String,
            &gettext("Add a task and exit"),
            Some("TITLE"),
        );
        app.add_main_option(
            "quick-add",
            b'q'.into(),
            glib::OptionFlags::NONE,
            glib::OptionArg::None,
            &gettext("Open the quick-add window"),
            None,
        );
        app.add_main_option(
            "today",
            b't'.into(),
            glib::OptionFlags::NONE,
            glib::OptionArg::None,
            &gettext("Open the Today view"),
            None,
        );
        app.add_main_option(
            "search",
            b's'.into(),
            glib::OptionFlags::NONE,
            glib::OptionArg::String,
            &gettext("Open Search with a query"),
            Some("QUERY"),
        );
        app.add_main_option(
            "background",
            b'b'.into(),
            glib::OptionFlags::NONE,
            glib::OptionArg::None,
            &gettext("Start without showing a window"),
            None,
        );
        app
    }
}
