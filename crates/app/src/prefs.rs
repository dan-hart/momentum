// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};

mod imp {
    use super::*;
    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/dan_hart/Momentum/ui/prefs.ui")]
    pub struct MomentumPrefs {
        #[template_child]
        pub enabled_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub server_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub user_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub password_row: TemplateChild<adw::PasswordEntryRow>,
        #[template_child]
        pub encrypt_row: TemplateChild<adw::PasswordEntryRow>,
        #[template_child]
        pub folder_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub auto_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub compress_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub colorful_row: TemplateChild<adw::SwitchRow>,
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
            s.bind("sync-enabled", &*self.enabled_row, "active").build();
            // The connection rows only matter while sync is on.
            for row in [
                self.server_row.upcast_ref::<gtk::Widget>(),
                self.user_row.upcast_ref(),
                self.password_row.upcast_ref(),
                self.folder_row.upcast_ref(),
                self.encrypt_row.upcast_ref(),
                self.auto_row.upcast_ref(),
                self.compress_row.upcast_ref(),
            ] {
                self.enabled_row
                    .bind_property("active", row, "sensitive")
                    .sync_create()
                    .build();
            }
            s.bind("auto-sync", &*self.auto_row, "active").build();
            s.bind("compress", &*self.compress_row, "active").build();
            s.bind("colorful-labels", &*self.colorful_row, "active").build();
            for (purpose, row) in [
                ("nextcloud", self.password_row.clone()),
                ("encryption", self.encrypt_row.clone()),
            ] {
                glib::spawn_future_local(async move {
                    if let Ok(Some(p)) = gio::spawn_blocking(move || crate::keyring::get(purpose)).await {
                        row.set_text(&p);
                    }
                    row.connect_changed(move |row| {
                        let t = row.text().to_string();
                        gio::spawn_blocking(move || {
                            crate::keyring::set(purpose, &t).map_err(|e| tracing::warn!("keyring: {e}"))
                        });
                    });
                });
            }
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
