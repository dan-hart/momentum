// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::{gio, glib};

mod imp {
    use super::*;
    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/dan_hart/Momentum/ui/prefs.ui")]
    pub struct MomentumPrefs {
        #[template_child]
        pub method_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub nextcloud_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub nearby_group: TemplateChild<adw::PreferencesGroup>,
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
        #[template_child]
        pub background_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub modifier_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub auto_archive_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub morning_summary_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub morning_hour_row: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub morning_minute_row: TemplateChild<adw::SpinRow>,
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
            let labels = [gettext("Off"), gettext("Nextcloud"), gettext("LibreSync")];
            self.method_row.set_model(Some(&gtk::StringList::new(
                &labels.iter().map(String::as_str).collect::<Vec<_>>(),
            )));
            let methods = ["off", "nextcloud", "libresync"];
            self.method_row
                .set_selected(methods.iter().position(|m| *m == sync_method(&s)).unwrap_or(0) as u32);
            let settings = s.clone();
            self.method_row.connect_selected_notify(move |row| {
                if let Some(method) = methods.get(row.selected() as usize) {
                    settings.set_string("sync-method", method).ok();
                }
            });
            // Native groups expose only the selected service; saved settings are retained.
            let update = glib::clone!(
                #[weak(rename_to = prefs)]
                self.obj(),
                move |s: &gio::Settings| {
                    let method = sync_method(s);
                    let imp = prefs.imp();
                    imp.nextcloud_group.set_visible(method == "nextcloud");
                    imp.nearby_group.set_visible(method == "libresync");
                    for row in [
                        imp.server_row.upcast_ref::<gtk::Widget>(),
                        imp.user_row.upcast_ref(),
                        imp.password_row.upcast_ref(),
                        imp.folder_row.upcast_ref(),
                        imp.encrypt_row.upcast_ref(),
                        imp.auto_row.upcast_ref(),
                        imp.compress_row.upcast_ref(),
                    ] {
                        row.set_sensitive(method == "nextcloud");
                    }
                    imp.method_row
                        .set_selected(methods.iter().position(|m| *m == method).unwrap_or(0) as u32);
                }
            );
            update(&s);
            s.connect_changed(Some("sync-method"), move |s, _| update(s));
            s.bind("auto-sync", &*self.auto_row, "active").build();
            s.bind("compress", &*self.compress_row, "active").build();
            s.bind("colorful-labels", &*self.colorful_row, "active").build();
            s.bind("run-in-background", &*self.background_row, "active").build();
            s.bind("auto-archive", &*self.auto_archive_row, "active").build();
            s.bind("morning-summary-enabled", &*self.morning_summary_row, "active")
                .build();
            for (key, row) in [
                ("morning-summary-hour", &*self.morning_hour_row),
                ("morning-summary-minute", &*self.morning_minute_row),
            ] {
                s.bind(key, row, "value").build();
                s.bind("morning-summary-enabled", row, "sensitive").get_only().build();
            }
            // Modifier key: platform-specific choices mapped to the setting's string value.
            let choices = crate::modifier::choices();
            let labels: Vec<&str> = choices.iter().map(|(_, l)| l.as_str()).collect();
            self.modifier_row.set_model(Some(&gtk::StringList::new(&labels)));
            let current = s.string("modifier-key");
            let idx = choices.iter().position(|(k, _)| *k == current.as_str()).unwrap_or(0);
            self.modifier_row.set_selected(idx as u32);
            let values: Vec<&'static str> = choices.iter().map(|(k, _)| *k).collect();
            let settings = s.clone();
            self.modifier_row.connect_selected_notify(move |row| {
                if let Some(v) = values.get(row.selected() as usize) {
                    settings.set_string("modifier-key", v).ok();
                }
            });
            // Ask the Background portal for permission (and autostart) when switched on.
            self.background_row.connect_active_notify(|row| {
                let on = row.is_active();
                glib::spawn_future_local(async move {
                    use ashpd::desktop::background::Background;
                    let req = Background::request()
                        .reason("Momentum keeps reminders and sync running")
                        .auto_start(on)
                        .command(["momentum", "--background"])
                        .send()
                        .await;
                    match req.and_then(|r| r.response()) {
                        Ok(r) => {
                            tracing::info!("background: run={} autostart={}", r.run_in_background(), r.auto_start())
                        }
                        Err(e) => tracing::warn!("background portal: {e}"),
                    }
                });
            });
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

/// Match the Mac's legacy preference resolution without destroying connections or
/// writing user data. Once selected, the explicit provider overrides both old switches.
pub fn sync_method(s: &gio::Settings) -> String {
    match s.string("sync-method").as_str() {
        "nextcloud" => "nextcloud",
        "libresync" => "libresync",
        "" if s.boolean("p2p-enabled") => "libresync",
        "" if s.boolean("sync-enabled") => "nextcloud",
        _ => "off",
    }
    .into()
}

#[cfg(test)]
mod sync_tests {
    use super::*;

    #[test]
    fn morning_summary_settings_default_off_and_retain_selected_time() {
        let schema = gio::SettingsSchemaSource::default()
            .unwrap()
            .lookup(*crate::config::APP_ID, true)
            .unwrap();
        let backend = gio::memory_settings_backend_new();
        let s = gio::Settings::new_full(&schema, Some(&backend), None);
        assert!(!s.boolean("morning-summary-enabled"));
        assert_eq!(s.int("morning-summary-hour"), 8);
        assert_eq!(s.int("morning-summary-minute"), 0);
        s.set_boolean("morning-summary-enabled", true).unwrap();
        s.set_int("morning-summary-hour", 9).unwrap();
        s.set_int("morning-summary-minute", 45).unwrap();
        let reopened = gio::Settings::new_full(&schema, Some(&backend), None);
        assert!(reopened.boolean("morning-summary-enabled"));
        reopened.set_boolean("morning-summary-enabled", false).unwrap();
        assert_eq!(reopened.int("morning-summary-hour"), 9);
        assert_eq!(reopened.int("morning-summary-minute"), 45);
    }

    #[test]
    fn sync_provider_selection_is_exclusive_and_preserves_legacy_choice() {
        let schema = gio::SettingsSchemaSource::default()
            .unwrap()
            .lookup(*crate::config::APP_ID, true)
            .unwrap();
        let s = gio::Settings::new_full(&schema, Some(&gio::memory_settings_backend_new()), None);
        s.reset("sync-method");
        s.set_boolean("sync-enabled", true).unwrap();
        s.set_boolean("p2p-enabled", true).unwrap();
        assert_eq!(sync_method(&s), "libresync");
        s.set_string("sync-method", "off").unwrap();
        assert_eq!(sync_method(&s), "off", "Off overrides both legacy switches");
        s.set_string("sync-method", "nextcloud").unwrap();
        assert_eq!(sync_method(&s), "nextcloud");
        assert!(
            s.boolean("p2p-enabled"),
            "saved legacy pairing preference is not destroyed"
        );
        s.reset("sync-method");
        s.set_boolean("p2p-enabled", false).unwrap();
        assert_eq!(sync_method(&s), "nextcloud");
        s.set_boolean("sync-enabled", false).unwrap();
        assert_eq!(sync_method(&s), "off");
    }
}
