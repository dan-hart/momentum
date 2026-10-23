// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
use crate::settings_ui::N_;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::{gio, glib};

mod imp {
    use super::*;
    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/dan_hart/Momentum/ui/prefs.ui")]
    pub struct MomentumPrefs {
        pub configured: std::cell::Cell<bool>,
        pub updating_settings_sync: std::cell::Cell<bool>,
        #[template_child]
        pub settings_sync_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub settings_status_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub settings_review_row: TemplateChild<adw::ButtonRow>,
        #[template_child]
        pub settings_retry_row: TemplateChild<adw::ButtonRow>,
        #[template_child]
        pub settings_export_row: TemplateChild<adw::ButtonRow>,
        #[template_child]
        pub settings_import_row: TemplateChild<adw::ButtonRow>,
        #[template_child]
        pub migrate_folder_row: TemplateChild<adw::ButtonRow>,
        #[template_child]
        pub optimized_row: TemplateChild<adw::ButtonRow>,
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
        pub background_sync_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub background_count_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub modifier_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub auto_archive_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub day_period_today_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub dynamic_messages_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub grouping_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub sorting_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub direction_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub upcoming_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub sound_volume_row: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub sound_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub about_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub sync_status_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub completion_sound_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub completion_sound_choice_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub completion_sound_preview_row: TemplateChild<adw::ButtonRow>,
        #[template_child]
        pub morning_summary_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub morning_hour_row: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub morning_minute_row: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub font_button: TemplateChild<gtk::FontDialogButton>,
        #[template_child]
        pub content_scale_row: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub interface_scale_row: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub reset_typography_row: TemplateChild<adw::ButtonRow>,
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
            crate::updates::add_page(self.obj().upcast_ref());
            // PreferencesGroup titles accept markup, unlike page titles.
            self.about_group
                .set_title(&glib::markup_escape_text(&gettext("About & Help")));
            let s = gio::Settings::new(*crate::config::APP_ID);
            crate::typography::register_interface_root(&*self.obj());
            for (key, row) in [
                ("nextcloud-server", &*self.server_row),
                ("nextcloud-user", &*self.user_row),
                ("nextcloud-folder", &*self.folder_row),
            ] {
                s.bind(key, row, "text").build();
            }
            self.folder_row.set_text(&effective_nextcloud_folder(&s));
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
            s.bind("background-sync", &*self.background_sync_row, "active").build();
            let count_labels = [
                gettext("Due or scheduled today"),
                gettext("Today including overdue"),
                gettext("Off"),
            ];
            self.background_count_row.set_model(Some(&gtk::StringList::new(
                &count_labels.iter().map(String::as_str).collect::<Vec<_>>(),
            )));
            let count_mode =
                crate::background_status::BackgroundCountMode::from_setting(&s.string("background-count-mode"));
            let count_index = match count_mode {
                crate::background_status::BackgroundCountMode::DueToday => 0,
                crate::background_status::BackgroundCountMode::TodayIncludingOverdue => 1,
                crate::background_status::BackgroundCountMode::Off => 2,
            };
            self.background_count_row.set_selected(count_index);
            self.background_count_row.upcast_ref::<gtk::Widget>().update_property(&[
                gtk::accessible::Property::Label(&gettext("Background task count")),
                gtk::accessible::Property::Description(&gettext(
                    "Choose which tasks appear in the Background Apps status",
                )),
            ]);
            let settings = s.clone();
            self.background_count_row.connect_selected_notify(move |row| {
                if let Some(value) = crate::background_status::BackgroundCountMode::VALUES.get(row.selected() as usize)
                {
                    settings.set_string("background-count-mode", value).ok();
                }
            });
            s.connect_changed(
                Some("background-count-mode"),
                glib::clone!(
                    #[weak(rename_to = row)]
                    self.background_count_row,
                    move |settings, _| {
                        let selected = match crate::background_status::BackgroundCountMode::from_setting(
                            &settings.string("background-count-mode"),
                        ) {
                            crate::background_status::BackgroundCountMode::DueToday => 0,
                            crate::background_status::BackgroundCountMode::TodayIncludingOverdue => 1,
                            crate::background_status::BackgroundCountMode::Off => 2,
                        };
                        row.set_selected(selected);
                    }
                ),
            );
            s.bind("auto-archive", &*self.auto_archive_row, "active").build();
            s.bind("show-day-period-tasks-in-today", &*self.day_period_today_row, "active")
                .build();
            s.bind("dynamic-empty-state-messages", &*self.dynamic_messages_row, "active")
                .build();
            bind_choice(
                &s,
                "group-by",
                &self.grouping_row,
                &["morning-night", "none", "project", "tag", "estimate"],
                &[
                    N_("Morning & Night"),
                    N_("None"),
                    N_("Project"),
                    N_("Tag"),
                    N_("Time Estimate"),
                ],
            );
            bind_choice(
                &s,
                "task-sort",
                &self.sorting_row,
                &["manual", "title", "due", "estimate", "created"],
                &[
                    N_("Manual Order"),
                    N_("Title"),
                    N_("Due Day"),
                    N_("Estimate"),
                    N_("Created"),
                ],
            );
            bind_choice(
                &s,
                "sort-direction",
                &self.direction_row,
                &["ascending", "descending"],
                &[N_("Ascending"), N_("Descending")],
            );
            bind_choice(
                &s,
                "upcoming-range",
                &self.upcoming_row,
                &["7", "30"],
                &[N_("Next 7 Days"), N_("Next 30 Days")],
            );
            self.sound_volume_row
                .set_value(s.double("completion-sound-volume") * 100.0);
            self.sound_volume_row.connect_value_notify(glib::clone!(
                #[strong]
                s,
                move |row| {
                    s.set_double("completion-sound-volume", row.value() / 100.0).ok();
                }
            ));
            s.connect_changed(
                Some("completion-sound-volume"),
                glib::clone!(
                    #[weak(rename_to = row)]
                    self.sound_volume_row,
                    move |s, _| {
                        row.set_value(s.double("completion-sound-volume") * 100.0);
                    }
                ),
            );
            for choice in crate::sound::CompletionSoundChoice::NEW {
                let row = adw::ActionRow::builder().title(choice.label()).build();
                let preview = gtk::Button::builder()
                    .icon_name("media-playback-start-symbolic")
                    .tooltip_text(gettext("Preview sound"))
                    .valign(gtk::Align::Center)
                    .build();
                preview.update_property(&[gtk::accessible::Property::Label(&format!(
                    "{}: {}",
                    gettext("Preview sound"),
                    choice.label()
                ))]);
                preview.connect_clicked(move |_| crate::sound::play_completion(choice));
                let select = gtk::Button::builder()
                    .label(gettext("Use this sound"))
                    .valign(gtk::Align::Center)
                    .build();
                select.update_property(&[gtk::accessible::Property::Label(&format!(
                    "{}: {}",
                    gettext("Use this sound"),
                    choice.label()
                ))]);
                select.connect_clicked(glib::clone!(
                    #[strong]
                    s,
                    move |_| {
                        s.set_string("completion-sound-choice", choice.slug()).ok();
                    }
                ));
                row.add_suffix(&preview);
                row.add_suffix(&select);
                self.sound_group.add(&row);
            }
            for (id, title, uri) in [
                ("about.source", "Source Code", "https://github.com/dan-hart/momentum"),
                (
                    "about.issues",
                    "Report an Issue",
                    "https://github.com/dan-hart/momentum/issues",
                ),
            ] {
                let row = adw::ActionRow::builder().title(gettext(title)).build();
                row.set_widget_name(id);
                let link = gtk::LinkButton::with_label(uri, &gettext("Open"));
                link.set_valign(gtk::Align::Center);
                row.add_suffix(&link);
                self.about_group.add(&row);
            }
            for (id, title) in [
                ("about.license", "License"),
                ("about.acknowledgments", "Acknowledgments"),
            ] {
                let row = adw::ButtonRow::builder()
                    .title(gettext(title))
                    .action_name("app.about")
                    .build();
                row.set_widget_name(id);
                self.about_group.add(&row);
            }
            let details = adw::ButtonRow::builder().title(gettext("Copy app details")).build();
            details.connect_activated(|row| {
                row.clipboard().set_text(&format!(
                    "Momentum {}\nBuild: {}\nPlatform: Linux",
                    *crate::config::VERSION,
                    *crate::config::PROFILE
                ));
            });
            self.about_group.add(&details);
            self.obj().connect_closed(|_| crate::sound::stop());
            self.obj().connect_visible_page_notify(|_| crate::sound::stop());
            s.bind("completion-sound", &*self.completion_sound_row, "active")
                .build();
            for (id, row) in [
                ("tasks.grouping", self.grouping_row.upcast_ref::<gtk::Widget>()),
                ("tasks.sort", self.sorting_row.upcast_ref()),
                ("tasks.sort-direction", self.direction_row.upcast_ref()),
                ("tasks.upcoming", self.upcoming_row.upcast_ref()),
                ("tasks.day-period-visibility", self.day_period_today_row.upcast_ref()),
                ("tasks.archive", self.auto_archive_row.upcast_ref()),
                ("tasks.dynamic-messages", self.dynamic_messages_row.upcast_ref()),
                ("sounds.enabled", self.completion_sound_row.upcast_ref()),
                ("sounds.selection", self.completion_sound_choice_row.upcast_ref()),
                ("sounds.volume", self.sound_volume_row.upcast_ref()),
                ("sync.provider", self.method_row.upcast_ref()),
                ("sync.nextcloud", self.server_row.upcast_ref()),
                ("sync.automatic", self.auto_row.upcast_ref()),
                ("sync.compression", self.compress_row.upcast_ref()),
                ("sync.folder-copy", self.migrate_folder_row.upcast_ref()),
                ("sync.settings", self.settings_sync_row.upcast_ref()),
                ("sync.background", self.background_sync_row.upcast_ref()),
                ("data.settings-import", self.settings_import_row.upcast_ref()),
                ("data.settings-export", self.settings_export_row.upcast_ref()),
                ("appearance.colors", self.colorful_row.upcast_ref()),
                ("appearance.font", self.font_button.upcast_ref()),
                ("appearance.content-size", self.content_scale_row.upcast_ref()),
                ("appearance.interface-size", self.interface_scale_row.upcast_ref()),
                ("appearance.reset", self.reset_typography_row.upcast_ref()),
                ("notifications.morning-summary", self.morning_summary_row.upcast_ref()),
                ("integrations.shortcuts", self.modifier_row.upcast_ref()),
                ("integrations.startup", self.background_row.upcast_ref()),
                ("integrations.count", self.background_count_row.upcast_ref()),
            ] {
                row.set_widget_name(id);
            }
            update_sound_choices(&self.completion_sound_choice_row, &s);
            self.completion_sound_preview_row.set_sensitive(
                crate::sound::CompletionSoundChoice::ALL
                    .iter()
                    .any(|c| c.slug() == s.string("completion-sound-choice")),
            );
            self.completion_sound_choice_row
                .upcast_ref::<gtk::Widget>()
                .update_property(&[
                    gtk::accessible::Property::Label(&gettext("Completion sound")),
                    gtk::accessible::Property::Description(&gettext("Choose a short sound for completed tasks")),
                ]);
            self.completion_sound_preview_row
                .upcast_ref::<gtk::Widget>()
                .update_property(&[
                    gtk::accessible::Property::Label(&gettext("Preview sound")),
                    gtk::accessible::Property::Description(&gettext("Play the selected completion sound")),
                ]);
            // A settings-to-row update must not save its sanitized fallback over an
            // unknown value written by a newer app. Only the user's selection writes.
            let updating_sound = std::rc::Rc::new(std::cell::Cell::new(false));
            self.completion_sound_choice_row.connect_selected_notify(glib::clone!(
                #[strong]
                s,
                #[strong]
                updating_sound,
                move |row| {
                    if !updating_sound.get() {
                        if let Some(choice) = crate::sound::CompletionSoundChoice::ALL.get(row.selected() as usize) {
                            s.set_string("completion-sound-choice", choice.slug()).ok();
                        }
                    }
                }
            ));
            s.connect_changed(
                Some("completion-sound-choice"),
                glib::clone!(
                    #[weak(rename_to = row)]
                    self.completion_sound_choice_row,
                    #[strong]
                    updating_sound,
                    move |settings, _| {
                        // A ComboRow selection writes this setting synchronously.
                        // Replacing its model inside that notification destroys
                        // native selection state that set_selected still uses.
                        glib::idle_add_local_once(glib::clone!(
                            #[weak]
                            row,
                            #[strong]
                            settings,
                            #[strong]
                            updating_sound,
                            move || {
                                updating_sound.set(true);
                                update_sound_choices(&row, &settings);
                                updating_sound.set(false);
                            }
                        ));
                    }
                ),
            );
            s.connect_changed(
                Some("completion-sound-choice"),
                glib::clone!(
                    #[weak(rename_to = row)]
                    self.completion_sound_preview_row,
                    move |s, _| {
                        row.set_sensitive(
                            crate::sound::CompletionSoundChoice::ALL
                                .iter()
                                .any(|c| c.slug() == s.string("completion-sound-choice")),
                        );
                    }
                ),
            );
            self.completion_sound_preview_row.connect_activated(glib::clone!(
                #[strong]
                s,
                move |_| {
                    if let Some(choice) = crate::sound::CompletionSoundChoice::ALL
                        .into_iter()
                        .find(|c| c.slug() == s.string("completion-sound-choice"))
                    {
                        crate::sound::play_completion(choice);
                    }
                }
            ));
            for (key, row) in [
                ("typography-content-scale", &*self.content_scale_row),
                ("typography-interface-scale", &*self.interface_scale_row),
            ] {
                s.bind(key, row, "value").build();
            }
            let updating_font = std::rc::Rc::new(std::cell::Cell::new(false));
            if let Some(font) = crate::typography::strip_font_size(&s.string("typography-font")) {
                self.font_button
                    .set_font_desc(&gtk::pango::FontDescription::from_string(&font));
            }
            self.font_button.update_property(&[
                gtk::accessible::Property::Label(&gettext("Font")),
                gtk::accessible::Property::Description(&gettext(
                    "Choose a font for Momentum; its size is controlled separately",
                )),
            ]);
            self.content_scale_row.upcast_ref::<gtk::Widget>().update_property(&[
                gtk::accessible::Property::Label(&gettext("Content size")),
                gtk::accessible::Property::Description(&gettext("Task titles, notes, and task entry, in percent")),
            ]);
            self.interface_scale_row.upcast_ref::<gtk::Widget>().update_property(&[
                gtk::accessible::Property::Label(&gettext("Interface size")),
                gtk::accessible::Property::Description(&gettext(
                    "Navigation, settings, headings, and controls, in percent",
                )),
            ]);
            self.reset_typography_row.upcast_ref::<gtk::Widget>().update_property(&[
                gtk::accessible::Property::Label(&gettext("Reset Typography")),
                gtk::accessible::Property::Description(&gettext(
                    "Restore the system font and both sizes to 100 percent",
                )),
            ]);
            self.font_button.connect_font_desc_notify(glib::clone!(
                #[strong]
                s,
                #[strong]
                updating_font,
                move |button| {
                    if updating_font.get() {
                        return;
                    }
                    let font = button
                        .font_desc()
                        .and_then(|font| crate::typography::strip_font_size(&font.to_str()))
                        .unwrap_or_default();
                    s.set_string("typography-font", &font).ok();
                }
            ));
            s.connect_changed(
                Some("typography-font"),
                glib::clone!(
                    #[weak(rename_to = button)]
                    self.font_button,
                    #[strong]
                    updating_font,
                    move |settings, _| {
                        let saved = crate::typography::strip_font_size(&settings.string("typography-font"));
                        let shown = button
                            .font_desc()
                            .and_then(|font| crate::typography::strip_font_size(&font.to_str()));
                        if saved == shown {
                            return;
                        }
                        updating_font.set(true);
                        button.set_font_desc(&gtk::pango::FontDescription::from_string(
                            saved.as_deref().unwrap_or("Sans"),
                        ));
                        updating_font.set(false);
                    }
                ),
            );
            self.reset_typography_row.connect_activated(glib::clone!(
                #[strong]
                s,
                #[weak(rename_to = button)]
                self.font_button,
                #[strong]
                updating_font,
                move |_| {
                    updating_font.set(true);
                    s.reset("typography-font");
                    s.reset("typography-content-scale");
                    s.reset("typography-interface-scale");
                    button.set_font_desc(&gtk::pango::FontDescription::from_string("Sans"));
                    updating_font.set(false);
                }
            ));
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

impl MomentumPrefs {
    pub fn present(&self, parent: Option<&impl IsA<gtk::Widget>>) {
        if let Some(owner) = parent
            .and_then(|parent| parent.root())
            .and_downcast::<crate::window::MomentumWindow>()
        {
            self.connect_app_settings(&owner);
            let status = owner.engine().sync_status();
            let text = owner.imp().sync_error.borrow().clone().unwrap_or_else(|| {
                if status.last_nextcloud_ms > 0 {
                    gettext("Previous sync completed")
                } else {
                    gettext("No completed sync yet")
                }
            });
            self.imp().sync_status_row.set_subtitle(&text);
        }
        // A short desktop window must not squeeze a full preferences page.
        // Keep libadwaita's compact-width boundary (450px) rather than comparing
        // the content allocation with a 600px toplevel request: native solid
        // decorations subtract their padding from that allocation.
        // https://gnome.pages.gitlab.gnome.org/libadwaita/doc/1.9/property.Dialog.presentation-mode.html
        if let Some(parent) =
            parent.filter(|parent| parent.width() > 450 && (parent.width() < 720 || parent.height() < 700))
        {
            adw::prelude::AdwDialogExt::present(self, None::<&gtk::Widget>);
            if let (Some(window), Some(owner)) = (
                self.root().and_downcast::<gtk::Window>(),
                parent.root().and_downcast::<gtk::Window>(),
            ) {
                window.set_transient_for(Some(&owner));
                window.set_modal(true);
                window.set_resizable(true);
            }
            return;
        }
        if let Some(parent) = parent {
            crate::typography::fit_dialog_to_parent(self, parent);
        }
        adw::prelude::AdwDialogExt::present(self, parent);
        if let Some(parent) = parent {
            crate::typography::fit_dialog_to_parent(self, parent);
        }
    }

    fn refresh_app_settings(&self, owner: &crate::window::MomentumWindow) {
        let imp = self.imp();
        let status = owner.engine().settings_sync_status();
        imp.updating_settings_sync.set(true);
        imp.settings_sync_row.set_active(status.enabled);
        imp.updating_settings_sync.set(false);
        let local_error = owner.app_settings_error();
        imp.settings_status_row
            .set_visible(status.enabled || local_error.is_some());
        let mut message = if let Some(error) = local_error.as_ref().or(status.error.as_ref()) {
            error.clone()
        } else if status.bootstrap_required {
            gettext("Review the saved settings before syncing them.")
        } else if sync_method(&gio::Settings::new(*crate::config::APP_ID)) == "off" {
            gettext("Choose a sync provider to share app settings.")
        } else if status.pending_keys > 0 {
            gettext("App settings are waiting to sync.")
        } else if status.last_success_ms > 0 {
            gettext("App settings are up to date.")
        } else {
            gettext("App settings have not synced yet.")
        };
        if status.last_success_ms > 0 {
            message.push_str(&format!(
                "\n{} {}",
                gettext("Last synced"),
                crate::window::ago_text(status.last_success_ms)
            ));
        }
        imp.settings_status_row
            .set_subtitle(&glib::markup_escape_text(&message));
        imp.settings_review_row.set_visible(status.bootstrap_required);
        imp.settings_review_row.set_sensitive(owner.can_review_app_settings());
        imp.settings_retry_row
            .set_visible(owner.can_retry_app_settings() || (status.enabled && status.error.is_some()));
        let settings = gio::Settings::new(*crate::config::APP_ID);
        let optimized = nextcloud_optimized(&settings);
        imp.migrate_folder_row
            .set_visible(effective_nextcloud_folder(&settings) != "momentum");
        imp.optimized_row.set_title(&gettext(if optimized {
            "Optimized sync enabled"
        } else {
            "Upgrade to optimized sync…"
        }));
        imp.optimized_row
            .set_sensitive(!optimized && !owner.imp().syncing.get() && owner.sync_configured());
        let busy = owner.imp().syncing.get();
        imp.method_row.set_sensitive(!busy);
        for row in [
            imp.server_row.upcast_ref::<gtk::Widget>(),
            imp.user_row.upcast_ref(),
            imp.password_row.upcast_ref(),
            imp.folder_row.upcast_ref(),
            imp.encrypt_row.upcast_ref(),
            imp.auto_row.upcast_ref(),
            imp.compress_row.upcast_ref(),
            imp.migrate_folder_row.upcast_ref(),
        ] {
            row.set_sensitive(!busy && sync_method(&settings) == "nextcloud");
        }
    }

    fn connect_app_settings(&self, owner: &crate::window::MomentumWindow) {
        if self.imp().configured.replace(true) {
            return;
        }
        self.refresh_app_settings(owner);
        self.imp()
            .settings_sync_row
            .set_sensitive(std::env::var_os("MOMENTUM_DEMO").is_none());
        self.imp().settings_sync_row.connect_active_notify(glib::clone!(
            #[weak(rename_to = prefs)]
            self,
            #[weak]
            owner,
            move |row| {
                if prefs.imp().updating_settings_sync.get() {
                    return;
                }
                let result = owner.stage_app_settings(None).and_then(|()| {
                    owner.refresh_app_settings();
                    owner
                        .engine()
                        .set_settings_sync_enabled(row.is_active())
                        .map_err(|error| error.to_string())
                });
                if let Err(error) = result {
                    prefs.add_toast(adw::Toast::new(&error));
                }
                prefs.refresh_app_settings(&owner);
                if row.is_active() && sync_method(&owner.imp().settings) != "off" {
                    owner.sync();
                }
            }
        ));
        for (row, export) in [
            (&*self.imp().settings_export_row, true),
            (&*self.imp().settings_import_row, false),
        ] {
            row.connect_activated(glib::clone!(
                #[weak(rename_to = prefs)]
                self,
                #[weak]
                owner,
                move |_| prefs.transfer_settings(&owner, export)
            ));
        }
        self.imp().settings_retry_row.connect_activated(glib::clone!(
            #[weak(rename_to = prefs)]
            self,
            #[weak]
            owner,
            move |_| {
                if owner.can_retry_app_settings() {
                    if let Err(error) = owner.retry_app_settings() {
                        prefs.add_toast(adw::Toast::new(&error));
                    }
                    prefs.refresh_app_settings(&owner);
                } else {
                    owner.sync();
                }
            }
        ));
        self.imp().settings_review_row.connect_activated(glib::clone!(
            #[weak(rename_to = prefs)]
            self,
            #[weak]
            owner,
            move |_| {
                prefs.close();
                owner.review_app_settings();
            }
        ));
        self.imp().optimized_row.connect_activated(glib::clone!(
            #[weak(rename_to = prefs)]
            self,
            #[weak]
            owner,
            move |_| {
                let settings = gio::Settings::new(*crate::config::APP_ID);
                let identity = nextcloud_identity(&settings);
                let dialog = adw::AlertDialog::builder()
                    .heading(gettext("Upgrade to optimized sync?"))
                    .body(gettext("Upgrade every participating device and enable optimized sync on each one. This connection cannot return to legacy sync. Existing legacy files stay intact. Turning sync off remains available."))
                    .build();
                dialog.add_response("cancel", &gettext("Cancel"));
                dialog.add_response("upgrade", &gettext("Upgrade"));
                dialog.set_close_response("cancel");
                dialog.connect_response(None, glib::clone!(
                    #[weak]
                    owner,
                    move |_, response| {
                        if response == "upgrade" && !owner.imp().syncing.get() && nextcloud_identity(&settings) == identity {
                            if let Err(error) = remember_optimized_connection(&settings, &identity) {
                                owner.toast(&error.to_string());
                            } else { owner.sync(); }
                        }
                    }
                ));
                dialog.present(Some(&prefs));
            }
        ));
        self.imp().migrate_folder_row.connect_activated(glib::clone!(
            #[weak(rename_to = prefs)]
            self,
            #[weak]
            owner,
            move |_| {
                prefs.close();
                owner.migrate_nextcloud_folder();
            }
        ));
        glib::timeout_add_local(
            std::time::Duration::from_secs(1),
            glib::clone!(
                #[weak(rename_to = prefs)]
                self,
                #[weak]
                owner,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    prefs.refresh_app_settings(&owner);
                    glib::ControlFlow::Continue
                }
            ),
        );
    }

    fn transfer_settings(&self, owner: &crate::window::MomentumWindow, export: bool) {
        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to = prefs)]
            self,
            #[weak]
            owner,
            async move {
                let parent = prefs.root().and_downcast::<gtk::Window>();
                let dialog = gtk::FileDialog::builder()
                    .title(if export {
                        gettext("Export app settings")
                    } else {
                        gettext("Import app settings")
                    })
                    .initial_name("momentum-settings.json")
                    .build();
                let result = if export {
                    dialog.save_future(parent.as_ref()).await
                } else {
                    dialog.open_future(parent.as_ref()).await
                };
                let Ok(file) = result else {
                    return;
                };
                let Some(path) = file.path() else {
                    prefs.add_toast(adw::Toast::new(&gettext("Choose a local file")));
                    return;
                };
                let result = if export {
                    owner.export_app_settings_to(path.to_string_lossy().into_owned())
                } else {
                    owner.import_app_settings_from(path.to_string_lossy().into_owned())
                };
                match result {
                    Ok(_) => {
                        owner.refresh();
                        prefs.refresh_app_settings(&owner);
                        prefs.add_toast(adw::Toast::new(&if export {
                            gettext("App settings saved")
                        } else {
                            gettext("App settings imported")
                        }));
                    }
                    Err(error) => {
                        prefs.refresh_app_settings(&owner);
                        prefs.add_toast(adw::Toast::new(&error));
                    }
                }
            }
        ));
    }
}

const LEGACY_NEXTCLOUD_FOLDER: &str = "super-productivity";

/// A failed preference write must never redirect an existing connection. This is also
/// used when displaying or exporting the effective connection while pinning retries.
pub fn nextcloud_identity(settings: &gio::Settings) -> String {
    momentum_core::nextcloud_connection_identity(
        settings.string("nextcloud-server").into(),
        settings.string("nextcloud-user").into(),
        effective_nextcloud_folder(settings),
    )
}

pub fn nextcloud_optimized(settings: &gio::Settings) -> bool {
    let identity = nextcloud_identity(settings);
    settings
        .strv("nextcloud-optimized-connections")
        .iter()
        .any(|value| value.as_str() == identity)
}

pub fn remember_optimized_connection(settings: &gio::Settings, identity: &str) -> Result<(), glib::BoolError> {
    let mut identities: Vec<String> = settings
        .strv("nextcloud-optimized-connections")
        .iter()
        .map(|value| value.to_string())
        .collect();
    if !identities.iter().any(|value| value == identity) {
        identities.push(identity.to_owned());
    }
    settings.set_strv("nextcloud-optimized-connections", identities.as_slice())
}

pub fn effective_nextcloud_folder(settings: &gio::Settings) -> String {
    if !settings.boolean("nextcloud-folder-default-pinned")
        && settings.user_value("nextcloud-folder").is_none()
        && !settings.string("nextcloud-server").is_empty()
        && !settings.string("nextcloud-user").is_empty()
    {
        LEGACY_NEXTCLOUD_FOLDER.into()
    } else {
        settings.string("nextcloud-folder").into()
    }
}

/// Preserve configured connections whose old effective folder was never stored.
pub fn pin_existing_nextcloud_folder(settings: &gio::Settings) -> Result<(), glib::BoolError> {
    pin_existing_nextcloud_folder_with(settings, |key, value| settings.set_value(key, value))
}

fn pin_existing_nextcloud_folder_with(
    settings: &gio::Settings,
    write: impl Fn(&str, &glib::Variant) -> Result<(), glib::BoolError>,
) -> Result<(), glib::BoolError> {
    if settings.boolean("nextcloud-folder-default-pinned") {
        return Ok(());
    }
    if settings.user_value("nextcloud-folder").is_none()
        && !settings.string("nextcloud-server").is_empty()
        && !settings.string("nextcloud-user").is_empty()
    {
        write("nextcloud-folder", &LEGACY_NEXTCLOUD_FOLDER.to_variant())?;
    }
    write("nextcloud-folder-default-pinned", &true.to_variant())
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
    fn legacy_folder_pin_failure_leaves_the_migration_retryable() {
        let schema = gio::SettingsSchemaSource::default()
            .unwrap()
            .lookup(*crate::config::APP_ID, true)
            .unwrap();
        let settings = gio::Settings::new_full(&schema, Some(&gio::memory_settings_backend_new()), None);
        settings
            .set_string("nextcloud-server", "https://example.test/dav")
            .unwrap();
        settings.set_string("nextcloud-user", "fixture").unwrap();
        let result = pin_existing_nextcloud_folder_with(&settings, |key, value| {
            if key == "nextcloud-folder" {
                Err(glib::bool_error!("fixture denies folder write"))
            } else {
                settings.set_value(key, value)
            }
        });
        assert!(result.is_err());
        assert_eq!(effective_nextcloud_folder(&settings), LEGACY_NEXTCLOUD_FOLDER);
        assert!(
            !settings.boolean("nextcloud-folder-default-pinned"),
            "failed writes must not mark folder migration complete"
        );
        assert!(settings.user_value("nextcloud-folder").is_none());
        pin_existing_nextcloud_folder(&settings).unwrap();
        assert!(settings.boolean("nextcloud-folder-default-pinned"));
        assert_eq!(settings.string("nextcloud-folder"), LEGACY_NEXTCLOUD_FOLDER);
    }

    #[test]
    fn optimized_consent_defaults_off_and_remains_bound_to_the_connection() {
        let schema = gio::SettingsSchemaSource::default()
            .unwrap()
            .lookup(*crate::config::APP_ID, true)
            .unwrap();
        let backend = gio::memory_settings_backend_new();
        let settings = gio::Settings::new_full(&schema, Some(&backend), None);
        settings
            .set_string("nextcloud-server", "https://fixture.invalid")
            .unwrap();
        settings.set_string("nextcloud-user", "fixture").unwrap();
        assert!(!nextcloud_optimized(&settings));
        let original_folder = effective_nextcloud_folder(&settings);
        remember_optimized_connection(&settings, &nextcloud_identity(&settings)).unwrap();
        assert!(nextcloud_optimized(&settings));
        settings.set_string("nextcloud-folder", "other").unwrap();
        assert!(!nextcloud_optimized(&settings));
        settings.set_string("nextcloud-folder", &original_folder).unwrap();
        let reopened = gio::Settings::new_full(&schema, Some(&backend), None);
        assert!(nextcloud_optimized(&reopened));
    }

    #[test]
    fn typography_settings_default_to_system_font_and_full_scale() {
        let schema = gio::SettingsSchemaSource::default()
            .unwrap()
            .lookup(*crate::config::APP_ID, true)
            .unwrap();
        let s = gio::Settings::new_full(&schema, Some(&gio::memory_settings_backend_new()), None);

        assert!(s.string("typography-font").is_empty());
        assert_eq!(s.int("typography-content-scale"), 100);
        assert_eq!(s.int("typography-interface-scale"), 100);
        assert!(s.set_int("typography-content-scale", 250).is_ok());
        let key = schema.key("typography-content-scale");
        assert!(key.range_check(&250.to_variant()));
        assert!(!key.range_check(&251.to_variant()));
    }

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

/// Bind a presentation picker without overwriting unknown raw values during refresh.
fn bind_choice(
    settings: &gio::Settings,
    key: &'static str,
    row: &adw::ComboRow,
    values: &'static [&'static str],
    titles: &[&str],
) {
    let titles: Vec<_> = titles.iter().map(|title| gettext(*title)).collect();
    row.set_model(Some(&gtk::StringList::new(
        &titles.iter().map(String::as_str).collect::<Vec<_>>(),
    )));
    row.set_selected(values.iter().position(|v| *v == settings.string(key)).unwrap_or(0) as u32);
    let updating = std::rc::Rc::new(std::cell::Cell::new(false));
    row.connect_selected_notify(glib::clone!(
        #[strong]
        settings,
        #[strong]
        updating,
        move |row| {
            if !updating.get() {
                if let Some(value) = values.get(row.selected() as usize) {
                    settings.set_string(key, value).ok();
                }
            }
        }
    ));
    settings.connect_changed(
        Some(key),
        glib::clone!(
            #[weak]
            row,
            move |s, _| {
                updating.set(true);
                row.set_selected(values.iter().position(|v| *v == s.string(key)).unwrap_or(0) as u32);
                updating.set(false);
            }
        ),
    );
}

impl MomentumPrefs {
    /// Reveal a read-only search destination without activating its control.
    pub fn reveal(&self, id: &str) {
        let category = id.split('.').next().unwrap_or("tasks");
        self.set_visible_page_name(category);
        let settings = gio::Settings::new(*crate::config::APP_ID);
        let method = sync_method(&settings);
        let requires_nextcloud = [
            "sync.nextcloud",
            "sync.compression",
            "sync.folder-copy",
            "sync.automatic",
        ]
        .contains(&id);
        let requires_libresync = ["sync.libresync", "sync.devices"].contains(&id);
        if (requires_nextcloud && method != "nextcloud") || (requires_libresync && method != "libresync") {
            self.add_toast(adw::Toast::new(&gettext(
                "Choose the matching sync provider to use this setting. Your saved connections are preserved.",
            )));
            self.imp().method_row.set_focusable(true);
            self.imp().method_row.grab_focus();
            return;
        }
        let id = id.to_string();
        let prefs = self.clone();
        glib::idle_add_local_once(move || {
            let entry = crate::settings_ui::entries().into_iter().find(|e| e.id == id);
            fn find(widget: &gtk::Widget, id: &str, title: Option<&str>) -> Option<gtk::Widget> {
                if widget.widget_name() == id
                    || widget
                        .downcast_ref::<adw::PreferencesRow>()
                        .is_some_and(|row| Some(row.title().as_str()) == title)
                {
                    return Some(widget.clone());
                }
                let mut child = widget.first_child();
                while let Some(c) = child {
                    if let Some(found) = find(&c, id, title) {
                        return Some(found);
                    }
                    child = c.next_sibling();
                }
                None
            }
            if let Some(row) = find(prefs.upcast_ref(), &id, entry.as_ref().map(|e| e.title.as_str())) {
                row.set_focusable(true);
                if row.is_visible() && row.is_sensitive() {
                    row.grab_focus();
                } else if let Some(parent) = row.parent() {
                    parent.grab_focus();
                }
                row.add_css_class("settings-target");
                glib::timeout_add_local_once(std::time::Duration::from_secs(2), move || {
                    row.remove_css_class("settings-target")
                });
            } else {
                prefs.add_toast(adw::Toast::new(&gettext(
                    "This setting is available in the selected section.",
                )));
            }
        });
    }
    pub fn search_for(&self, query: &str) {
        self.set_search_enabled(true);
        fn apply(widget: &gtk::Widget, query: &str) -> bool {
            if let Some(entry) = widget.downcast_ref::<gtk::SearchEntry>() {
                entry.set_text(query);
                entry.grab_focus();
                return true;
            }
            let mut child = widget.first_child();
            while let Some(c) = child {
                if apply(&c, query) {
                    return true;
                }
                child = c.next_sibling();
            }
            false
        }
        fn reveal_search(widget: &gtk::Widget) {
            if let Some(bar) = widget.downcast_ref::<gtk::SearchBar>() {
                bar.set_search_mode(true);
            }
            if let Some(toggle) = widget.downcast_ref::<gtk::ToggleButton>() {
                if toggle.icon_name().as_deref() == Some("edit-find-symbolic") {
                    toggle.set_active(true);
                }
            }
            let mut child = widget.first_child();
            while let Some(c) = child {
                reveal_search(&c);
                child = c.next_sibling();
            }
        }
        reveal_search(self.upcast_ref());
        apply(self.upcast_ref(), query);
    }
}

fn update_sound_choices(row: &adw::ComboRow, settings: &gio::Settings) {
    let raw = settings.string("completion-sound-choice");
    let mut labels: Vec<_> = crate::sound::CompletionSoundChoice::ALL
        .into_iter()
        .map(|c| c.label())
        .collect();
    let selected = crate::sound::CompletionSoundChoice::ALL
        .iter()
        .position(|c| c.slug() == raw)
        .unwrap_or_else(|| {
            labels.push(gettext("Unavailable sound: {}").replace("{}", &raw));
            labels.len() - 1
        });
    row.set_model(Some(&gtk::StringList::new(
        &labels.iter().map(String::as_str).collect::<Vec<_>>(),
    )));
    row.set_selected(selected as u32);
}
