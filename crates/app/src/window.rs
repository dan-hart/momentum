// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::{gio, glib};
use serde_json::{json, Map, Value};
use sp_model::*;
use sp_oplog::Action;
use sp_store::Store;
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;

use crate::application::MomentumApplication;
use crate::config::{APP_ID, PROFILE};

#[derive(Clone, PartialEq)]
pub enum View {
    Today,
    Project(String),
    Tag(String),
}

mod imp {
    use super::*;

    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/github/dan_hart/Momentum/ui/window.ui")]
    pub struct MomentumWindow {
        #[template_child]
        pub split_view: TemplateChild<adw::NavigationSplitView>,
        #[template_child]
        pub sidebar_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub content_page: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub task_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub add_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub sync_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub sync_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub search_bar: TemplateChild<gtk::SearchBar>,
        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub empty: TemplateChild<adw::StatusPage>,
        pub settings: gio::Settings,
        pub store: RefCell<Store>,
        pub view: RefCell<View>,
        pub views: RefCell<Vec<Option<View>>>,
        pub rows: RefCell<Vec<String>>,
        pub filter: RefCell<String>,
        pub color_css: RefCell<String>,
        pub tag_popover: gtk::Popover,
        pub tag_list: gtk::ListBox,
        pub tag_matches: RefCell<Vec<String>>,
        pub color_provider: gtk::CssProvider,
        pub syncing: Cell<bool>,
        pub notified: RefCell<HashSet<String>>,
    }

    impl Default for MomentumWindow {
        fn default() -> Self {
            let demo = std::env::var_os("MOMENTUM_DEMO").is_some();
            let dir = if demo {
                glib::tmp_dir().join("momentum-demo")
            } else {
                glib::user_data_dir().join("momentum")
            };
            Self {
                split_view: Default::default(),
                sidebar_list: Default::default(),
                content_page: Default::default(),
                task_list: Default::default(),
                add_entry: Default::default(),
                sync_button: Default::default(),
                sync_label: Default::default(),
                search_bar: Default::default(),
                search_entry: Default::default(),
                toast_overlay: Default::default(),
                empty: Default::default(),
                settings: gio::Settings::new(*APP_ID),
                store: RefCell::new(if demo {
                    crate::demo::store(dir)
                } else {
                    Store::load(dir)
                }),
                view: RefCell::new(View::Today),
                views: Default::default(),
                rows: Default::default(),
                filter: Default::default(),
                color_css: Default::default(),
                tag_popover: gtk::Popover::builder().autohide(false).has_arrow(false).build(),
                tag_list: gtk::ListBox::builder()
                    .selection_mode(gtk::SelectionMode::Single)
                    .css_classes(["navigation-sidebar"])
                    .build(),
                tag_matches: Default::default(),
                color_provider: gtk::CssProvider::new(),
                syncing: Cell::new(false),
                notified: Default::default(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MomentumWindow {
        const NAME: &'static str = "MomentumWindow";
        type Type = super::MomentumWindow;
        type ParentType = adw::ApplicationWindow;
        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }
        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MomentumWindow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            let demo = std::env::var_os("MOMENTUM_DEMO").is_some();
            if *PROFILE == "Devel" && !demo {
                obj.add_css_class("devel");
            }
            if demo {
                obj.set_default_size(800, 560); // small enough that mutter does not auto-maximise it
            } else {
                obj.load_window_size();
            }
            // libadwaita ≥ 1.6 follows the system accent colour through the settings portal;
            // nothing here hard-codes colours, so the whole UI inherits it.
            let sm = adw::StyleManager::default();
            tracing::info!(
                "System accent colour: supported={} {:?}",
                sm.is_system_supports_accent_colors(),
                sm.accent_color()
            );
            obj.setup();
        }
    }
    impl WidgetImpl for MomentumWindow {}
    impl Drop for MomentumWindow {
        fn drop(&mut self) {
            self.tag_popover.unparent();
        }
    }
    impl WindowImpl for MomentumWindow {
        fn close_request(&self) -> glib::Propagation {
            if let Err(err) = self.obj().save_window_size() {
                tracing::warn!("Failed to save window state, {}", &err);
            }
            self.parent_close_request()
        }
    }
    impl ApplicationWindowImpl for MomentumWindow {}
    impl AdwApplicationWindowImpl for MomentumWindow {}
}

glib::wrapper! {
    pub struct MomentumWindow(ObjectSubclass<imp::MomentumWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Root, gtk::Native, gtk::ShortcutManager,
                    gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

pub fn fmt_ms(ms: f64) -> String {
    let m = (ms / 60000.0).round() as u64;
    if m >= 60 {
        format!("{}h {:02}m", m / 60, m % 60)
    } else {
        format!("{m}m")
    }
}
/// Relative day label: Today, Tomorrow, Yesterday, a weekday within the week, else a locale date.
pub fn fmt_day(day: &str) -> String {
    let parse = |d: &str| -> Option<glib::DateTime> {
        let mut it = d.split('-').map(|x| x.parse::<i32>().ok());
        glib::DateTime::from_local(it.next()??, it.next()??, it.next()??, 0, 0, 0.0).ok()
    };
    let (Some(target), Some(today)) = (parse(day), parse(&today_str())) else {
        return day.to_string();
    };
    let diff = (target.to_unix() - today.to_unix()) / 86_400;
    let fmt = |f: &str| {
        target
            .format(f)
            .map(|g| g.to_string())
            .unwrap_or_else(|_| day.to_string())
    };
    match diff {
        0 => gettext("Today"),
        1 => gettext("Tomorrow"),
        -1 => gettext("Yesterday"),
        2..=6 => fmt("%A"),
        _ if target.year() == today.year() => fmt("%-d %B"),
        _ => fmt("%-d %B %Y"),
    }
}

/// CSS colour string from the sync data → `#rrggbb` for Pango markup.
fn hex_color(color: &str) -> Option<String> {
    let c = gtk::gdk::RGBA::parse(color).ok()?;
    Some(format!(
        "#{:02x}{:02x}{:02x}",
        (c.red() * 255.0) as u8,
        (c.green() * 255.0) as u8,
        (c.blue() * 255.0) as u8
    ))
}
fn tag_color(g: &Tag) -> Option<&str> {
    g.color
        .as_deref()
        .or_else(|| g.theme.get("primary").and_then(Value::as_str))
}

/// "1h 30m", "45m", "2h" → ms
pub fn parse_ms(s: &str) -> Option<f64> {
    let mut total = 0.0;
    let mut num = String::new();
    let mut any = false;
    for c in s.chars() {
        if c.is_ascii_digit() || c == '.' {
            num.push(c)
        } else if c == 'h' || c == 'm' {
            total += num.parse::<f64>().ok()? * if c == 'h' { 3_600_000.0 } else { 60_000.0 };
            num.clear();
            any = true;
        }
    }
    any.then_some(total)
}

impl MomentumWindow {
    pub fn new(app: &MomentumApplication) -> Self {
        glib::Object::builder().property("application", app).build()
    }

    fn setup(&self) {
        let imp = self.imp();
        imp.sidebar_list.connect_row_selected(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |_, row| {
                let Some(row) = row else { return };
                if let Some(Some(v)) = w.imp().views.borrow().get(row.index() as usize) {
                    *w.imp().view.borrow_mut() = v.clone();
                }
                w.imp().split_view.set_show_content(true);
                w.refresh_tasks();
            }
        ));
        imp.add_entry.connect_activate(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |e| {
                w.add_task(&e.text());
                e.set_text("");
            }
        ));
        imp.task_list.connect_row_activated(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |_, row| {
                let id = w.imp().rows.borrow().get(row.index() as usize).cloned();
                if let Some(id) = id {
                    w.open_task(&id);
                }
            }
        ));
        if let Some(d) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &d,
                &imp.color_provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
        // Stateful sort action backed by GSettings; the header menu's radio items target it.
        self.add_action(&imp.settings.create_action("task-sort"));
        imp.settings.connect_changed(
            Some("task-sort"),
            glib::clone!(
                #[weak(rename_to = w)]
                self,
                move |_, _| w.refresh_tasks()
            ),
        );
        imp.settings.connect_changed(
            Some("colorful-labels"),
            glib::clone!(
                #[weak(rename_to = w)]
                self,
                move |_, _| w.refresh()
            ),
        );
        imp.search_bar.set_key_capture_widget(Some(self));
        self.setup_tag_completion();
        imp.search_entry.connect_search_changed(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |e| {
                *w.imp().filter.borrow_mut() = e.text().to_lowercase();
                w.refresh_tasks();
            }
        ));
        let key = gtk::EventControllerKey::new();
        key.connect_key_pressed(glib::clone!(
            #[weak(rename_to = w)]
            self,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_, k, _, m| {
                // Alt+1…9 jumps to the n-th sidebar entry, like tabs in Files and Terminal.
                let n = k
                    .to_unicode()
                    .and_then(|c| c.to_digit(10))
                    .filter(|&d| d > 0 && m.contains(gtk::gdk::ModifierType::ALT_MASK));
                match n.and_then(|d| {
                    w.imp()
                        .views
                        .borrow()
                        .iter()
                        .enumerate()
                        .filter(|(_, v)| v.is_some())
                        .nth(d as usize - 1)
                        .map(|(i, _)| i)
                }) {
                    Some(i) => {
                        w.imp()
                            .sidebar_list
                            .select_row(w.imp().sidebar_list.row_at_index(i as i32).as_ref());
                        glib::Propagation::Stop
                    }
                    None => glib::Propagation::Proceed,
                }
            }
        ));
        self.add_controller(key);
        let act = |name: &str, f: fn(&MomentumWindow)| {
            gio::ActionEntry::builder(name)
                .activate(move |w: &MomentumWindow, _, _| f(w))
                .build()
        };
        self.add_action_entries([
            act("search", |w| {
                let b = &w.imp().search_bar;
                b.set_search_mode(!b.is_search_mode());
                if b.is_search_mode() {
                    w.imp().search_entry.grab_focus();
                }
            }),
            act("toggle-done", |w| {
                if let Some(id) = w.focused_task() {
                    let done = w.imp().store.borrow().state.task.entities[&id].is_done;
                    w.set_done(&id, !done);
                }
            }),
            act("delete-task", |w| {
                if let Some(id) = w.focused_task() {
                    w.delete_task(&id);
                }
            }),
        ]);
        imp.settings.connect_changed(
            None,
            glib::clone!(
                #[weak(rename_to = w)]
                self,
                move |_, _| w.update_sync_button()
            ),
        );
        self.update_sync_button();
        glib::timeout_add_seconds_local(
            30,
            glib::clone!(
                #[weak(rename_to = w)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    w.check_reminders();
                    w.update_sync_button();
                    glib::ControlFlow::Continue
                }
            ),
        );
        glib::timeout_add_seconds_local(
            300,
            glib::clone!(
                #[weak(rename_to = w)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    if w.imp().settings.boolean("auto-sync") && w.sync_configured() {
                        w.sync();
                    }
                    glib::ControlFlow::Continue
                }
            ),
        );
        self.refresh();
        if imp.settings.boolean("auto-sync") && self.sync_configured() {
            self.sync();
        }
        if let Some(path) = std::env::var_os("MOMENTUM_SCREENSHOT") {
            if std::env::var_os("MOMENTUM_SCREENSHOT_DIALOG").is_some() {
                self.new_task_dialog();
            }
            crate::demo::screenshot(self.upcast_ref(), path.into());
        }
    }

    fn sync_configured(&self) -> bool {
        let s = &self.imp().settings;
        ["nextcloud-server", "nextcloud-user", "nextcloud-folder"]
            .iter()
            .all(|k| !s.string(k).trim().is_empty())
    }
    fn update_sync_button(&self) {
        let imp = self.imp();
        let on = self.sync_configured();
        imp.sync_button.set_visible(on);
        imp.sync_label.set_visible(on);
        let last = imp.settings.int64("last-sync-ms") as u64;
        imp.sync_label.set_text(&if last == 0 {
            gettext("Not synced yet")
        } else {
            let secs = now_ms().saturating_sub(last) / 1000;
            let ago = match secs {
                0..=59 => format!("{secs} {}", gettext("seconds ago")),
                60..=3599 => format!("{} {}", secs / 60, gettext("minutes ago")),
                3600..=86399 => format!("{} {}", secs / 3600, gettext("hours ago")),
                _ => format!("{} {}", secs / 86400, gettext("days ago")),
            };
            format!("{} {ago}", gettext("Last synced"))
        });
    }
    fn focused_task(&self) -> Option<String> {
        let row = self.imp().task_list.focus_child()?.downcast::<gtk::ListBoxRow>().ok()?;
        self.imp().rows.borrow().get(row.index() as usize).cloned()
    }

    /// `#` autocomplete in the add-task entry: a popover of matching tags, driven by the keyboard.
    fn setup_tag_completion(&self) {
        let imp = self.imp();
        imp.tag_popover.set_parent(&*imp.add_entry);
        imp.tag_popover.set_position(gtk::PositionType::Bottom);
        imp.tag_popover.set_child(Some(
            &gtk::ScrolledWindow::builder()
                .propagate_natural_height(true)
                .max_content_height(240)
                .child(&imp.tag_list)
                .build(),
        ));
        imp.add_entry.connect_changed(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |_| w.update_tag_completion()
        ));
        imp.tag_list.connect_row_activated(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |_, row| w.accept_tag_completion(row.index())
        ));
        let keys = gtk::EventControllerKey::builder()
            .propagation_phase(gtk::PropagationPhase::Capture)
            .build();
        keys.connect_key_pressed(glib::clone!(
            #[weak(rename_to = w)]
            self,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_, key, _, _| {
                let imp = w.imp();
                if !imp.tag_popover.is_visible() {
                    return glib::Propagation::Proceed;
                }
                use gtk::gdk::Key;
                let selected = imp.tag_list.selected_row().map(|r| r.index()).unwrap_or(0);
                let count = imp.tag_matches.borrow().len() as i32;
                match key {
                    Key::Down | Key::Up => {
                        let next = if key == Key::Down {
                            (selected + 1) % count
                        } else {
                            (selected + count - 1) % count
                        };
                        imp.tag_list.select_row(imp.tag_list.row_at_index(next).as_ref());
                        glib::Propagation::Stop
                    }
                    Key::Return | Key::KP_Enter | Key::Tab => {
                        w.accept_tag_completion(selected);
                        glib::Propagation::Stop
                    }
                    Key::Escape => {
                        imp.tag_popover.popdown();
                        glib::Propagation::Stop
                    }
                    _ => glib::Propagation::Proceed,
                }
            }
        ));
        imp.add_entry.add_controller(keys);
    }

    /// The `#word` under the cursor, as (start byte offset, text without `#`).
    fn hash_word_at_cursor(&self) -> Option<(usize, String)> {
        let entry = &self.imp().add_entry;
        let text = entry.text();
        let cursor: usize = text
            .char_indices()
            .nth(entry.position() as usize)
            .map(|(i, _)| i)
            .unwrap_or(text.len());
        let start = text[..cursor].rfind(char::is_whitespace).map(|i| i + 1).unwrap_or(0);
        let word = &text[start..cursor];
        word.strip_prefix('#').map(|w| (start, w.to_string()))
    }

    fn update_tag_completion(&self) {
        let imp = self.imp();
        let Some((_, prefix)) = self.hash_word_at_cursor() else {
            imp.tag_popover.popdown();
            return;
        };
        let store = imp.store.borrow();
        let lower = prefix.to_lowercase();
        let matches: Vec<String> = store
            .state
            .tag
            .iter()
            .filter(|t| t.id != TODAY_TAG_ID && t.title.to_lowercase().starts_with(&lower))
            .map(|t| t.title.clone())
            .take(8)
            .collect();
        drop(store);
        imp.tag_list.remove_all();
        for m in &matches {
            imp.tag_list.append(
                &gtk::Label::builder()
                    .label(format!("#{m}"))
                    .xalign(0.0)
                    .margin_start(6)
                    .margin_end(6)
                    .build(),
            );
        }
        *imp.tag_matches.borrow_mut() = matches;
        if imp.tag_matches.borrow().is_empty() {
            imp.tag_popover.popdown();
        } else {
            imp.tag_list.select_row(imp.tag_list.row_at_index(0).as_ref());
            imp.tag_popover.popup();
        }
    }

    fn accept_tag_completion(&self, index: i32) {
        let imp = self.imp();
        let Some(name) = imp.tag_matches.borrow().get(index.max(0) as usize).cloned() else {
            return;
        };
        let Some((start, prefix)) = self.hash_word_at_cursor() else {
            return;
        };
        let entry = &imp.add_entry;
        let text = entry.text().to_string();
        let end = start + 1 + prefix.len();
        let new = format!("{}#{name} {}", &text[..start], text[end..].trim_start());
        let cursor = text[..start].chars().count() + name.chars().count() + 2;
        imp.tag_popover.popdown();
        entry.set_text(&new);
        entry.set_position(cursor as i32);
    }

    pub fn toast(&self, msg: &str) {
        self.imp().toast_overlay.add_toast(adw::Toast::new(msg));
    }
    pub fn dispatch(&self, a: Action) {
        self.imp().store.borrow_mut().dispatch(a);
        self.refresh();
    }
    pub fn focus_add(&self) {
        self.imp().split_view.set_show_content(true);
        self.new_task_dialog();
    }

    // ---- rendering -------------------------------------------------------

    pub fn refresh(&self) {
        self.refresh_sidebar();
        self.refresh_tasks();
    }

    /// CSS class that colours symbolic icons with a project/tag colour from the sync data.
    fn color_class(&self, color: &str) -> Option<String> {
        let safe: String = color
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || "#(),. %".contains(*c))
            .collect();
        if safe.is_empty() {
            return None;
        }
        let class = format!(
            "c{:x}",
            safe.bytes()
                .fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3))
        );
        let mut css = self.imp().color_css.borrow_mut();
        if !css.contains(&class) {
            css.push_str(&format!(".{class} {{ color: {safe}; }}\n"));
            self.imp().color_provider.load_from_string(&css);
        }
        Some(class)
    }

    fn sidebar_row(
        &self,
        title: &str,
        icon: &str,
        view: Option<View>,
        color: Option<&str>,
        section: Option<&'static str>,
    ) {
        let imp = self.imp();
        let row: gtk::ListBoxRow = if view.is_some() {
            let r = adw::ActionRow::builder().title(title).build();
            let img = gtk::Image::from_icon_name(icon);
            match color.and_then(|c| self.color_class(c)) {
                Some(class) => img.add_css_class(&class),
                None if icon == "starred-symbolic" => img.add_css_class("accent"),
                None => {}
            }
            r.add_prefix(&img);
            let target = gtk::DropTarget::new(String::static_type(), gtk::gdk::DragAction::MOVE);
            let dest = view.clone().unwrap();
            target.connect_drop(glib::clone!(
                #[weak(rename_to = w)]
                self,
                #[upgrade_or]
                false,
                move |_, value, _, _| {
                    let Ok(task_id) = value.get::<String>() else {
                        return false;
                    };
                    w.drop_task(&task_id, &dest)
                }
            ));
            r.add_controller(target);
            r.upcast()
        } else {
            // Section header: click (or activate) to collapse/expand; state persists in GSettings.
            let key = section.unwrap_or_default();
            let collapsed = imp.settings.boolean(key);
            let b = gtk::Box::builder().spacing(6).margin_top(12).margin_start(6).build();
            b.append(
                &gtk::Label::builder()
                    .label(title)
                    .xalign(0.0)
                    .hexpand(true)
                    .css_classes(["heading", "dim-label"])
                    .build(),
            );
            b.append(
                &gtk::Image::builder()
                    .icon_name(if collapsed {
                        "pan-end-symbolic"
                    } else {
                        "pan-down-symbolic"
                    })
                    .css_classes(["dim-label"])
                    .build(),
            );
            let r = gtk::ListBoxRow::builder()
                .child(&b)
                .selectable(false)
                .activatable(true)
                .build();
            r.update_property(&[gtk::accessible::Property::Label(&format!(
                "{title}, {}",
                if collapsed {
                    gettext("collapsed")
                } else {
                    gettext("expanded")
                }
            ))]);
            let toggle = gtk::GestureClick::new();
            toggle.connect_released(glib::clone!(
                #[weak(rename_to = w)]
                self,
                move |_, _, _, _| {
                    let s = &w.imp().settings;
                    s.set_boolean(key, !s.boolean(key)).ok();
                    w.refresh_sidebar();
                }
            ));
            r.add_controller(toggle);
            r
        };
        imp.sidebar_list.append(&row);
        imp.views.borrow_mut().push(view);
    }

    fn refresh_sidebar(&self) {
        let imp = self.imp();
        let current = imp.view.borrow().clone();
        imp.sidebar_list.remove_all();
        imp.views.borrow_mut().clear();
        let store = imp.store.borrow();
        self.sidebar_row(&gettext("Today"), "starred-symbolic", Some(View::Today), None, None);
        self.sidebar_row(&gettext("Projects"), "", None, None, Some("projects-collapsed"));
        let colorful = imp.settings.boolean("colorful-labels");
        if !imp.settings.boolean("projects-collapsed") {
            for p in store
                .state
                .project
                .iter()
                .filter(|p| !p.is_archived && !p.is_hidden_from_menu)
            {
                self.sidebar_row(
                    &p.title,
                    "folder-symbolic",
                    Some(View::Project(p.id.clone())),
                    p.color().filter(|_| colorful),
                    None,
                );
            }
        }
        self.sidebar_row(&gettext("Tags"), "", None, None, Some("tags-collapsed"));
        if !imp.settings.boolean("tags-collapsed") {
            for t in store.state.tag.iter().filter(|t| t.id != TODAY_TAG_ID) {
                let color = tag_color(t).filter(|_| colorful);
                self.sidebar_row(&t.title, "tag-symbolic", Some(View::Tag(t.id.clone())), color, None);
            }
        }
        drop(store);
        let idx = imp
            .views
            .borrow()
            .iter()
            .position(|v| v.as_ref() == Some(&current))
            .or_else(|| {
                // Current view is inside a collapsed section: keep it, select nothing.
                imp.sidebar_list.unselect_all();
                None
            })
            .unwrap_or(0);
        imp.sidebar_list
            .select_row(imp.sidebar_list.row_at_index(idx as i32).as_ref());
    }

    fn view_task_ids(&self, store: &Store) -> Vec<String> {
        match &*self.imp().view.borrow() {
            View::Today => store.state.today_ids(),
            View::Project(id) => store
                .state
                .project
                .entities
                .get(id)
                .map(|p| p.task_ids.clone())
                .unwrap_or_default(),
            View::Tag(id) => store
                .state
                .tag
                .entities
                .get(id)
                .map(|t| t.task_ids.clone())
                .unwrap_or_default(),
        }
    }

    fn task_row(&self, t: &Task, store: &Store, indent: bool) {
        let imp = self.imp();
        let colorful = imp.settings.boolean("colorful-labels");
        let mut sub = vec![];
        // Outside a project view, lead with the project name in the project's own colour.
        if !matches!(*imp.view.borrow(), View::Project(_)) {
            if let Some(p) = store.state.project.entities.get(&t.project_id) {
                let hex = p.color().filter(|_| colorful).and_then(hex_color);
                let title = glib::markup_escape_text(&p.title);
                sub.push(match hex {
                    Some(h) => format!("<span foreground=\"{h}\">●</span> {title}"),
                    None => format!("● {title}"),
                });
            }
        }
        if t.time_estimate > 0.0 {
            sub.push(format!("~{}", fmt_ms(t.time_estimate)));
        }
        if let Some(d) = &t.due_day {
            if *imp.view.borrow() != View::Today {
                sub.push(glib::markup_escape_text(&fmt_day(d)).to_string());
            }
        }
        for tag in &t.tag_ids {
            if let Some(g) = store.state.tag.entities.get(tag) {
                let name = glib::markup_escape_text(&g.title);
                sub.push(match tag_color(g).filter(|_| colorful).and_then(hex_color) {
                    Some(h) => format!("<span foreground=\"{h}\">#{name}</span>"),
                    None => format!("#{name}"),
                });
            }
        }
        let row = adw::ActionRow::builder()
            .title(glib::markup_escape_text(&t.title))
            .subtitle(sub.join("  ·  "))
            .activatable(true)
            .build();
        if indent {
            row.set_margin_start(32);
        }
        if t.is_done {
            row.add_css_class("dim-label");
        }
        let check = gtk::CheckButton::builder()
            .active(t.is_done)
            .valign(gtk::Align::Center)
            .build();
        let id = t.id.clone();
        check.connect_toggled(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |c| w.set_done(&id, c.is_active())
        ));
        row.add_prefix(&check);
        let drag = gtk::DragSource::builder().actions(gtk::gdk::DragAction::MOVE).build();
        let tid = t.id.clone();
        drag.connect_prepare(move |_, _, _| Some(gtk::gdk::ContentProvider::for_value(&tid.to_value())));
        drag.connect_drag_begin(glib::clone!(
            #[weak]
            row,
            move |src, _| src.set_icon(Some(&gtk::WidgetPaintable::new(Some(&row))), 0, 0)
        ));
        row.add_controller(drag);
        imp.task_list.append(&row);
        imp.rows.borrow_mut().push(t.id.clone());
    }

    fn refresh_tasks(&self) {
        let imp = self.imp();
        imp.task_list.remove_all();
        imp.rows.borrow_mut().clear();
        let store = imp.store.borrow();
        let title = match &*imp.view.borrow() {
            View::Today => gettext("Today"),
            View::Project(id) => store
                .state
                .project
                .entities
                .get(id)
                .map(|p| p.title.clone())
                .unwrap_or_default(),
            View::Tag(id) => store
                .state
                .tag
                .entities
                .get(id)
                .map(|t| t.title.clone())
                .unwrap_or_default(),
        };
        imp.content_page.set_title(&title);
        let ids = self.view_task_ids(&store);
        let (mut open, mut done) = (vec![], vec![]);
        let filter = imp.filter.borrow();
        for id in ids {
            if let Some(t) = store.state.task.entities.get(&id) {
                if !t.title.to_lowercase().contains(&*filter) {
                    continue;
                }
                if t.is_done {
                    done.push(t)
                } else {
                    open.push(t)
                }
            }
        }
        let sort = imp.settings.string("task-sort");
        for list in [&mut open, &mut done] {
            match sort.as_str() {
                "title" => list.sort_by_key(|t| t.title.to_lowercase()),
                "due" => list.sort_by(|a, b| {
                    a.due_day
                        .is_none()
                        .cmp(&b.due_day.is_none())
                        .then_with(|| a.due_day.cmp(&b.due_day))
                }),
                "estimate" => list.sort_by(|a, b| b.time_estimate.total_cmp(&a.time_estimate)),
                "created" => list.sort_by_key(|t| std::cmp::Reverse(t.created)),
                _ => {}
            }
        }
        for t in open.into_iter().chain(done) {
            self.task_row(t, &store, false);
            for s in t.sub_task_ids.iter().filter_map(|i| store.state.task.entities.get(i)) {
                self.task_row(s, &store, true);
            }
        }
        imp.empty.set_visible(imp.rows.borrow().is_empty());
        imp.task_list.set_visible(!imp.rows.borrow().is_empty());
    }

    // ---- actions ---------------------------------------------------------

    fn current_project(&self, store: &Store) -> String {
        match &*self.imp().view.borrow() {
            View::Project(id) => id.clone(),
            _ => store
                .state
                .project
                .ids
                .first()
                .cloned()
                .unwrap_or(INBOX_PROJECT_ID.into()),
        }
    }

    /// Short syntax: `#tag` adds/creates tags, a trailing `1h 30m` sets the estimate.
    pub fn add_task(&self, text: &str) {
        if text.trim().is_empty() {
            return;
        }
        let imp = self.imp();
        let (mut words, mut tags, mut est) = (vec![], vec![], 0.0);
        for w in text.split_whitespace() {
            if let Some(tag) = w.strip_prefix('#') {
                tags.push(tag.to_string());
            } else if let Some(ms) = parse_ms(w) {
                est = ms;
            } else {
                words.push(w);
            }
        }
        let project = self.current_project(&imp.store.borrow());
        let mut task = Task::new(&words.join(" "), &project);
        task.time_estimate = est;
        let view = imp.view.borrow().clone();
        if view == View::Today {
            task.due_day = Some(today_str());
        }
        if let View::Tag(id) = &view {
            tags.push(imp.store.borrow().state.tag.entities[id].title.clone());
        }
        for name in tags {
            let existing = imp
                .store
                .borrow()
                .state
                .tag
                .iter()
                .find(|t| t.title.eq_ignore_ascii_case(&name))
                .map(|t| t.id.clone());
            let id = existing.unwrap_or_else(|| {
                let tag = Tag::new(&name);
                let id = tag.id.clone();
                imp.store.borrow_mut().dispatch(Action::AddTag { tag });
                id
            });
            if !task.tag_ids.contains(&id) {
                task.tag_ids.push(id);
            }
        }
        self.dispatch(Action::AddTask { task, bottom: true });
    }

    pub fn add_project(&self, title: &str) {
        if !title.trim().is_empty() {
            self.dispatch(Action::AddProject {
                project: Project::new(title),
            });
        }
    }

    /// A task dropped on a sidebar row: move to project, add tag, or plan for today.
    fn drop_task(&self, task_id: &str, dest: &View) -> bool {
        let store = self.imp().store.borrow();
        let Some(task) = store.state.task.entities.get(task_id).cloned() else {
            return false;
        };
        let sub_tasks: Vec<Task> = task
            .sub_task_ids
            .iter()
            .filter_map(|i| store.state.task.entities.get(i).cloned())
            .collect();
        let today = today_str();
        drop(store);
        match dest {
            View::Today if task.due_day.as_deref() != Some(&today) => {
                self.dispatch(Action::PlanForToday {
                    task_ids: vec![task.id.clone()],
                    today,
                });
            }
            View::Project(pid) if task.parent_id.is_none() && *pid != task.project_id => {
                self.dispatch(Action::MoveToProject {
                    task: task.clone(),
                    sub_tasks,
                    target_project_id: pid.clone(),
                });
            }
            View::Tag(tid) if !task.tag_ids.contains(tid) => {
                let mut ids = task.tag_ids.clone();
                ids.push(tid.clone());
                self.update_task(&task.id, [("tagIds".to_string(), json!(ids))].into_iter().collect());
            }
            _ => return false,
        }
        true
    }

    fn set_done(&self, id: &str, done: bool) {
        self.update_task(id, [("isDone".to_string(), json!(done))].into_iter().collect());
    }
    fn update_task(&self, id: &str, changes: Map<String, Value>) {
        self.dispatch(Action::UpdateTask { id: id.into(), changes });
    }

    fn delete_task(&self, id: &str) {
        let store = self.imp().store.borrow();
        let Some(task) = store.state.task.entities.get(id).cloned() else {
            return;
        };
        let sub_tasks: Vec<Task> = task
            .sub_task_ids
            .iter()
            .filter_map(|i| store.state.task.entities.get(i).cloned())
            .collect();
        drop(store);
        self.dispatch(Action::DeleteTask {
            task: task.clone(),
            sub_tasks: sub_tasks.clone(),
        });
        // HIG: destructive actions get an undo toast rather than a confirmation dialog.
        let toast = adw::Toast::builder()
            .title(gettext("Task deleted"))
            .button_label(gettext("Undo"))
            .build();
        toast.connect_button_clicked(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |_| {
                w.dispatch(Action::AddTask {
                    task: task.clone(),
                    bottom: true,
                });
                for st in &sub_tasks {
                    w.dispatch(Action::AddSubTask {
                        task: st.clone(),
                        parent_id: task.id.clone(),
                    });
                }
            }
        ));
        self.imp().toast_overlay.add_toast(toast);
    }

    fn check_reminders(&self) {
        let imp = self.imp();
        let now = now_ms();
        let due: Vec<Task> = imp
            .store
            .borrow()
            .state
            .task
            .iter()
            .filter(|t| !t.is_done && t.remind_at.is_some_and(|r| r <= now) && !imp.notified.borrow().contains(&t.id))
            .cloned()
            .collect();
        for t in due {
            imp.notified.borrow_mut().insert(t.id.clone());
            let n = gio::Notification::new(&t.title);
            n.set_body(Some(&gettext("Reminder")));
            if let Some(app) = self.application() {
                app.send_notification(Some(&t.id), &n);
            }
        }
    }

    // ---- task dialogs ----------------------------------------------------

    fn dialog(&self, title: &str, form: &crate::task_form::TaskForm, header: &adw::HeaderBar) -> adw::Dialog {
        let tv = adw::ToolbarView::new();
        tv.add_top_bar(header);
        tv.set_content(Some(&form.page));
        adw::Dialog::builder()
            .title(title)
            .content_width(520)
            .content_height(640)
            .child(&tv)
            .build()
    }

    /// HIG "new item" dialog: Cancel / Create in the header, Create enabled once there is a title.
    pub fn new_task_dialog(&self) {
        let imp = self.imp();
        let (project, due) = {
            let s = imp.store.borrow();
            (
                self.current_project(&s),
                matches!(*imp.view.borrow(), View::Today).then(today_str),
            )
        };
        let form = Rc::new(crate::task_form::TaskForm::new(
            &imp.store.borrow(),
            None,
            &project,
            due,
        ));
        let header = adw::HeaderBar::builder()
            .show_start_title_buttons(false)
            .show_end_title_buttons(false)
            .build();
        let cancel = gtk::Button::with_label(&gettext("Cancel"));
        let create = gtk::Button::builder()
            .label(gettext("Create"))
            .css_classes(["suggested-action"])
            .sensitive(false)
            .build();
        header.pack_start(&cancel);
        header.pack_end(&create);
        let dialog = self.dialog(&gettext("New Task"), &form, &header);
        dialog.set_default_widget(Some(&create));
        form.title.connect_changed(glib::clone!(
            #[weak]
            create,
            move |e| create.set_sensitive(!e.text().trim().is_empty())
        ));
        form.title.connect_entry_activated(glib::clone!(
            #[weak]
            create,
            move |_| create.emit_clicked()
        ));
        cancel.connect_clicked(glib::clone!(
            #[weak]
            dialog,
            move |_| {
                dialog.close();
            }
        ));
        create.connect_clicked(glib::clone!(
            #[weak(rename_to = w)]
            self,
            #[weak]
            dialog,
            #[strong]
            form,
            move |_| {
                let mut task = form.into_task(&mut w.imp().store.borrow_mut());
                // Clone the view first: dispatch() refreshes the sidebar, which re-borrows it mutably.
                let view = w.imp().view.borrow().clone();
                if let View::Tag(id) = view {
                    if !task.tag_ids.contains(&id) {
                        task.tag_ids.push(id);
                    }
                }
                w.dispatch(Action::AddTask { task, bottom: true });
                dialog.close();
            }
        ));
        dialog.present(Some(self));
        form.title.grab_focus();
    }

    /// Edit dialog: changes are applied when the dialog closes.
    pub fn open_task(&self, id: &str) {
        let imp = self.imp();
        let Some(t) = imp.store.borrow().state.task.entities.get(id).cloned() else {
            return;
        };
        let form = Rc::new(crate::task_form::TaskForm::new(
            &imp.store.borrow(),
            Some(&t),
            &t.project_id,
            None,
        ));
        let sub = adw::EntryRow::builder().title(gettext("Add subtask")).build();
        let del = gtk::Button::builder()
            .label(gettext("Delete Task"))
            .css_classes(["destructive-action"])
            .margin_top(12)
            .halign(gtk::Align::End)
            .build();
        if t.parent_id.is_none() {
            form.group.add(&sub);
        }
        form.group.add(&del);
        let dialog = self.dialog(&gettext("Task"), &form, &adw::HeaderBar::new());
        let id = t.id.clone();
        del.connect_clicked(glib::clone!(
            #[weak(rename_to = w)]
            self,
            #[weak]
            dialog,
            #[strong]
            id,
            move |_| {
                dialog.close();
                w.delete_task(&id);
            }
        ));
        sub.connect_entry_activated(glib::clone!(
            #[weak(rename_to = w)]
            self,
            #[strong]
            id,
            move |e| {
                if !e.text().trim().is_empty() {
                    let mut task = Task::new(&e.text(), "");
                    task.parent_id = Some(id.clone());
                    w.dispatch(Action::AddSubTask {
                        task,
                        parent_id: id.clone(),
                    });
                    e.set_text("");
                }
            }
        ));
        dialog.connect_closed(glib::clone!(
            #[weak(rename_to = w)]
            self,
            #[strong]
            form,
            move |_| {
                let imp = w.imp();
                if !imp.store.borrow().state.task.entities.contains_key(&id) {
                    return;
                }
                let mut ch = Map::new();
                let nt = form.title_text();
                if !nt.is_empty() && nt != t.title {
                    ch.insert("title".into(), json!(nt));
                }
                let ne = form.estimate_ms();
                if ne != t.time_estimate {
                    ch.insert("timeEstimate".into(), json!(ne));
                }
                let nd = form.due_day();
                if nd != t.due_day {
                    ch.insert("dueDay".into(), json!(nd));
                    if nd.is_some() {
                        ch.insert("dueWithTime".into(), Value::Null);
                    }
                }
                let np = form.project_id();
                if t.parent_id.is_none() && !np.is_empty() && np != t.project_id {
                    ch.insert("projectId".into(), json!(np));
                }
                let nn = form.notes_text();
                if nn != t.notes.clone().unwrap_or_default() {
                    ch.insert("notes".into(), json!(nn));
                }
                let ids = form.tag_ids(&mut imp.store.borrow_mut());
                if ids != t.tag_ids {
                    ch.insert("tagIds".into(), json!(ids));
                }
                if ch.is_empty() {
                    w.refresh();
                } else {
                    w.update_task(&id, ch);
                }
            }
        ));
        dialog.present(Some(self));
    }

    // ---- sync / backup ---------------------------------------------------

    pub fn nextcloud_cfg(&self) -> sp_sync::NextcloudCfg {
        let s = &self.imp().settings;
        sp_sync::NextcloudCfg {
            server_url: s.string("nextcloud-server").into(),
            user_name: s.string("nextcloud-user").into(),
            folder: s.string("nextcloud-folder").into(),
            compress: s.boolean("compress"),
            encrypt_key: None,
            password: String::new(),
        }
    }

    pub fn sync(&self) {
        let imp = self.imp();
        if imp.syncing.replace(true) {
            return;
        }
        // HIG: say what is happening right away, but only animate if it takes a while,
        // so a one-second sync does not flash a spinner.
        imp.sync_label.set_text(&gettext("Syncing…"));
        imp.sync_button.set_sensitive(false);
        glib::timeout_add_seconds_local_once(
            1,
            glib::clone!(
                #[weak(rename_to = w)]
                self,
                move || {
                    if w.imp().syncing.get() {
                        w.imp().sync_button.set_child(Some(&adw::Spinner::new()));
                        w.imp().sync_button.set_tooltip_text(Some(&gettext("Syncing…")));
                    }
                }
            ),
        );
        let mut cfg = self.nextcloud_cfg();
        let snapshot = imp.store.borrow().clone();
        let n_pending = snapshot.pending.len();
        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to = w)]
            self,
            async move {
                let result = gio::spawn_blocking(move || {
                    cfg.password = crate::keyring::get("nextcloud").unwrap_or_default();
                    cfg.encrypt_key = crate::keyring::get("encryption");
                    if !cfg.is_complete() {
                        return Err("Nextcloud sync is not configured".to_string());
                    }
                    let mut s = snapshot;
                    sp_sync::sync(&cfg, &mut s).map(|r| (s, r)).map_err(|e| e.to_string())
                })
                .await
                .unwrap();
                let imp = w.imp();
                imp.syncing.set(false);
                imp.sync_button.set_icon_name("view-refresh-symbolic");
                imp.sync_button.set_tooltip_text(Some(&gettext("Sync Now")));
                imp.sync_button.set_sensitive(true);
                w.update_sync_button();
                match result {
                    Ok((mut synced, r)) => {
                        // Re-apply anything dispatched while the sync ran, keeping it pending.
                        let live = imp.store.borrow().pending[n_pending..].to_vec();
                        for p in &live {
                            sp_oplog::apply(&mut synced.state, &p.action);
                        }
                        synced.pending.extend(live);
                        synced.save().ok();
                        *imp.store.borrow_mut() = synced;
                        imp.settings.set_int64("last-sync-ms", now_ms() as i64).ok();
                        w.refresh();
                        w.update_sync_button();
                        tracing::info!(
                            "sync ok: downloaded={} uploaded={} ops_uploaded={} sync_version={}",
                            r.downloaded,
                            r.uploaded,
                            r.ops_uploaded,
                            w.imp().store.borrow().meta.last_sync_version
                        );
                        if r.downloaded || r.uploaded {
                            w.toast(&format!("{} ({}↑)", gettext("Synced"), r.ops_uploaded));
                        }
                    }
                    Err(e) => {
                        tracing::warn!("sync failed: {e}");
                        w.toast(&format!("{}: {e}", gettext("Sync failed")));
                    }
                }
            }
        ));
    }

    pub fn import_backup(&self) {
        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to = w)]
            self,
            async move {
                let Ok(file) = gtk::FileDialog::builder()
                    .title(gettext("Import Super Productivity backup"))
                    .build()
                    .open_future(Some(&w))
                    .await
                else {
                    return;
                };
                let res = std::fs::read(file.path().unwrap())
                    .map_err(|e| e.to_string())
                    .and_then(|b| serde_json::from_slice::<Value>(&b).map_err(|e| e.to_string()))
                    .and_then(|v| AppData::from_backup(v).map_err(|e| e.to_string()));
                match res {
                    Ok(d) => {
                        w.imp().store.borrow_mut().replace_state(d);
                        w.refresh();
                        w.toast(&gettext("Backup imported"));
                    }
                    Err(e) => w.toast(&e),
                }
            }
        ));
    }
    pub fn export_backup(&self) {
        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to = w)]
            self,
            async move {
                let dialog = gtk::FileDialog::builder()
                    .title(gettext("Export backup"))
                    .initial_name(format!("{}.json", today_str()))
                    .build();
                let Ok(file) = dialog.save_future(Some(&w)).await else {
                    return;
                };
                let body =
                    json!({"data": w.imp().store.borrow().state, "timestamp": now_ms(), "crossModelVersion": 4.5});
                match std::fs::write(file.path().unwrap(), serde_json::to_vec_pretty(&body).unwrap()) {
                    Ok(_) => w.toast(&gettext("Backup exported")),
                    Err(e) => w.toast(&e.to_string()),
                }
            }
        ));
    }

    fn save_window_size(&self) -> Result<(), glib::BoolError> {
        let imp = self.imp();
        let (width, height) = self.default_size();
        imp.settings.set_int("window-width", width)?;
        imp.settings.set_int("window-height", height)?;
        imp.settings.set_boolean("is-maximized", self.is_maximized())
    }
    fn load_window_size(&self) {
        let imp = self.imp();
        self.set_default_size(imp.settings.int("window-width"), imp.settings.int("window-height"));
        if imp.settings.boolean("is-maximized") {
            self.maximize();
        }
    }
}
