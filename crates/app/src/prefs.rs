// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};

mod imp {
    use super::*;
    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/danhart/Momentum/ui/prefs.ui")]
    pub struct MomentumPrefs {
        #[template_child]
        pub server_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub user_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub password_row: TemplateChild<adw::PasswordEntryRow>,
        #[template_child]
        pub folder_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub auto_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub compress_row: TemplateChild<adw::SwitchRow>,
    }
    #[glib::object_subclass]
    impl ObjectSubclass for MomentumPrefs {
        const NAME: &'static str = "MomentumPrefs";
        type Type = super::MomentumPrefs;
        type ParentType = adw::PreferencesDialog;
        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }
        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }
    impl ObjectImpl for MomentumPrefs {
        fn constructed(&self) {
            self.parent_constructed();
            let s = gio::Settings::new(*crate::config::APP_ID);
            for (key, row) in [
                ("nextcloud-server", &*self.server_row),
                ("nextcloud-user", &*self.user_row),
                ("nextcloud-folder", &*self.folder_row),
            ] {
                s.bind(key, row, "text").build();
            }
            s.bind("auto-sync", &*self.auto_row, "active").build();
            s.bind("compress", &*self.compress_row, "active").build();
            let pw = self.password_row.clone();
            glib::spawn_future_local(async move {
                if let Ok(Some(p)) = gio::spawn_blocking(crate::keyring::get).await {
                    pw.set_text(&p);
                }
                pw.connect_changed(|row| {
                    let t = row.text().to_string();
                    gio::spawn_blocking(move || crate::keyring::set(&t).map_err(|e| tracing::warn!("keyring: {e}")));
                });
            });
        }
    }
    impl WidgetImpl for MomentumPrefs {}
    impl AdwDialogImpl for MomentumPrefs {}
    impl PreferencesDialogImpl for MomentumPrefs {}
}

glib::wrapper! {
    pub struct MomentumPrefs(ObjectSubclass<imp::MomentumPrefs>)
        @extends gtk::Widget, adw::Dialog, adw::PreferencesDialog,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
impl Default for MomentumPrefs {
    fn default() -> Self {
        glib::Object::new()
    }
}
