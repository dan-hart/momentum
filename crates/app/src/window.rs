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

/// Precomputed lowercase haystacks so typing never re-lowercases or re-parses the archive.
#[derive(Default)]
pub struct SearchIndex {
    tasks: Vec<(String, String)>,    // (task id, haystack)
    archived: Vec<(Task, String)>,   // parsed once
    projects: Vec<(String, String)>, // (project id, lower title)
    tags: Vec<(String, String)>,     // (tag id, lower title)
}
const SEARCH_LIMIT: usize = 60;

#[derive(Clone, PartialEq)]
pub enum View {
    Today,
    Upcoming,
    Archive,
    Search,
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
        pub banner: TemplateChild<adw::Banner>,
        #[template_child]
        pub add_clamp: TemplateChild<adw::Clamp>,
        #[template_child]
        pub search_clamp: TemplateChild<adw::Clamp>,
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
        pub last_day: RefCell<String>,
        pub index: RefCell<Option<Rc<SearchIndex>>>,
        pub archive_shown: Cell<usize>,
        pub search_debounce: RefCell<Option<glib::SourceId>>,
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
                banner: Default::default(),
                add_clamp: Default::default(),
                search_clamp: Default::default(),
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
                tag_popover: gtk::Popover::builder()
                    .autohide(false)
                    .has_arrow(false)
                    .css_classes(["menu"])
                    .build(),
                tag_list: gtk::ListBox::builder()
                    .selection_mode(gtk::SelectionMode::Single)
                    .css_classes(["navigation-sidebar"])
                    .build(),
                tag_matches: Default::default(),
                color_provider: gtk::CssProvider::new(),
                syncing: Cell::new(false),
                notified: Default::default(),
                last_day: RefCell::new(today_str()),
                index: Default::default(),
                archive_shown: Cell::new(100),
                search_debounce: Default::default(),
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
pub fn hex_color(color: &str) -> Option<String> {
    let c = gtk::gdk::RGBA::parse(color).ok()?;
    Some(format!(
        "#{:02x}{:02x}{:02x}",
        (c.red() * 255.0) as u8,
        (c.green() * 255.0) as u8,
        (c.blue() * 255.0) as u8
    ))
}
pub fn tag_color(g: &Tag) -> Option<&str> {
    g.color
        .as_deref()
        .or_else(|| g.theme.get("primary").and_then(Value::as_str))
}

/// Tasks in `archiveYoung` and `archiveOld` (kept in `state.rest` by sp-sync).
fn archived_tasks(store: &Store) -> Vec<Task> {
    ["archiveYoung", "archiveOld"]
        .iter()
        .filter_map(|k| store.state.rest.get(*k)?.get("task")?.get("entities")?.as_object())
        .flat_map(|e| {
            e.values()
                .filter_map(|v| serde_json::from_value::<Task>(v.clone()).ok())
        })
        .collect()
}

/// Short, human description of a repeat config: "Repeats every Monday", "Repeats daily", …
pub fn repeat_text(c: &RepeatCfg) -> String {
    let every = c.repeat_every.max(1);
    let names = [
        gettext("Sunday"),
        gettext("Monday"),
        gettext("Tuesday"),
        gettext("Wednesday"),
        gettext("Thursday"),
        gettext("Friday"),
        gettext("Saturday"),
    ];
    let days: Vec<&String> = c
        .weekdays()
        .iter()
        .zip(names.iter())
        .filter(|(on, _)| **on)
        .map(|(_, n)| n)
        .collect();
    match c.repeat_cycle.as_str() {
        "DAILY" if every == 1 => gettext("Repeats daily"),
        "DAILY" => format!("{} {every} {}", gettext("Repeats every"), gettext("days")),
        "WEEKLY" if days.len() == 7 && every == 1 => gettext("Repeats daily"),
        "WEEKLY" if days.len() == 1 && every == 1 => format!("{} {}", gettext("Repeats every"), days[0]),
        "WEEKLY" => {
            let list = days
                .iter()
                .map(|d| d.chars().take(3).collect::<String>())
                .collect::<Vec<_>>()
                .join(", ");
            if every == 1 {
                format!("{} {list}", gettext("Repeats weekly on"))
            } else {
                format!("{} {every} {} {list}", gettext("Repeats every"), gettext("weeks on"))
            }
        }
        "MONTHLY" => {
            let day = if c.monthly_last_day {
                gettext("the last day")
            } else {
                match c.start_date.as_deref().and_then(parse_day) {
                    Some((_, _, d)) => format!("{} {}", gettext("the"), ordinal(d)),
                    None => gettext("the same day"),
                }
            };
            if every == 1 {
                format!("{} {day}", gettext("Repeats monthly on"))
            } else {
                format!("{} {every} {} {day}", gettext("Repeats every"), gettext("months on"))
            }
        }
        "YEARLY" => {
            let date = c
                .start_date
                .as_deref()
                .and_then(parse_day)
                .and_then(|(y, m, d)| glib::DateTime::from_local(y as i32, m as i32, d as i32, 0, 0, 0.0).ok())
                .and_then(|d| d.format("%-d %B").ok())
                .map(|g| g.to_string())
                .unwrap_or_default();
            if every == 1 {
                format!("{} {date}", gettext("Repeats yearly on"))
            } else {
                format!("{} {every} {} {date}", gettext("Repeats every"), gettext("years on"))
            }
        }
        _ => gettext("Repeats"),
    }
}
fn ordinal(d: u32) -> String {
    let suffix = match (d % 10, d % 100) {
        (1, 11) | (2, 12) | (3, 13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{d}{suffix}")
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
                w.imp().archive_shown.set(100);
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
                let Some(id) = id else { return };
                if let Some(pid) = id.strip_prefix("project:") {
                    w.go_to(View::Project(pid.into()));
                } else if let Some(tid) = id.strip_prefix("tag:") {
                    w.go_to(View::Tag(tid.into()));
                } else if !id.is_empty() {
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
        self.add_action(&imp.settings.create_action("sort-direction"));
        self.add_action(&imp.settings.create_action("upcoming-range"));
        imp.settings.connect_changed(
            Some("upcoming-range"),
            glib::clone!(
                #[weak(rename_to = w)]
                self,
                move |_, _| w.refresh_tasks()
            ),
        );
        imp.banner.connect_button_clicked(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |_| crate::prefs::MomentumPrefs::default().present(Some(&w))
        ));
        imp.settings.connect_changed(
            Some("task-sort"),
            glib::clone!(
                #[weak(rename_to = w)]
                self,
                move |_, _| w.refresh_tasks()
            ),
        );
        imp.settings.connect_changed(
            Some("sort-direction"),
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
        self.setup_tag_completion();
        imp.search_entry.connect_search_changed(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |e| {
                *w.imp().filter.borrow_mut() = e.text().to_lowercase();
                // Coalesce keystrokes: rebuild the result list at most every 120 ms.
                if let Some(id) = w.imp().search_debounce.borrow_mut().take() {
                    id.remove();
                }
                let id = glib::timeout_add_local_once(
                    std::time::Duration::from_millis(120),
                    glib::clone!(
                        #[weak]
                        w,
                        move || {
                            w.imp().search_debounce.borrow_mut().take();
                            w.refresh_tasks();
                        }
                    ),
                );
                *w.imp().search_debounce.borrow_mut() = Some(id);
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
                w.go_to(View::Search);
                w.imp().search_entry.grab_focus();
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
            act("archive-done", |w| w.archive_done()),
            act("plan-today", |w| {
                if let Some(id) = w.focused_task() {
                    w.drop_task(&id, &View::Today);
                }
            }),
            act("move-to", |w| {
                if let Some(id) = w.focused_task() {
                    w.move_to_dialog(&id);
                }
            }),
            act("edit-context", |w| w.edit_context_dialog()),
            act("delete-context", |w| w.delete_context_dialog()),
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
                    let today = today_str();
                    if *w.imp().last_day.borrow() != today {
                        *w.imp().last_day.borrow_mut() = today;
                        w.spawn_repeats();
                        w.refresh();
                    }
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
        self.spawn_repeats();
        self.refresh();
        if imp.settings.boolean("auto-sync") && self.sync_configured() {
            self.sync();
        }
        if let Some(path) = std::env::var_os("MOMENTUM_SCREENSHOT") {
            if std::env::var_os("MOMENTUM_SCREENSHOT_DIALOG").is_some() {
                self.new_task_dialog();
            }
            if std::env::var_os("MOMENTUM_SCREENSHOT_UPCOMING").is_some() {
                *imp.view.borrow_mut() = View::Upcoming;
                self.refresh();
            }
            if let Some(q) = std::env::var_os("MOMENTUM_SCREENSHOT_SEARCH") {
                *imp.view.borrow_mut() = View::Search;
                imp.search_entry.set_text(&q.to_string_lossy());
                self.refresh();
            }
            if std::env::var_os("MOMENTUM_SCREENSHOT_TAG").is_some() {
                imp.add_entry.grab_focus();
                imp.add_entry.set_text("Write the docs #");
                imp.add_entry.set_position(-1);
            }
            crate::demo::screenshot(self.upcast_ref(), path.into());
        }
    }

    fn sync_configured(&self) -> bool {
        // Demo/screenshot runs must never touch a real server.
        if std::env::var_os("MOMENTUM_DEMO").is_some() {
            return false;
        }
        let s = &self.imp().settings;
        s.boolean("sync-enabled")
            && ["nextcloud-server", "nextcloud-user", "nextcloud-folder"]
                .iter()
                .all(|k| !s.string(k).trim().is_empty())
    }
    fn update_sync_button(&self) {
        let imp = self.imp();
        let on = self.sync_configured();
        imp.sync_button.set_visible(on);
        imp.sync_label.set_visible(on && !imp.rows.borrow().is_empty());
        let last = imp.settings.int64("last-sync-ms") as u64;
        imp.sync_label.set_text(&if last == 0 {
            gettext("Not synced yet")
        } else {
            let secs = now_ms().saturating_sub(last) / 1000;
            let ago = match secs {
                0..=9 => gettext("just now"),
                10..=59 => format!("{secs} {}", gettext("seconds ago")),
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
        imp.tag_popover.set_child(Some(&imp.tag_list));
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
        let focus = gtk::EventControllerFocus::new();
        focus.connect_leave(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |_| w.imp().tag_popover.popdown()
        ));
        imp.add_entry.add_controller(focus);
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
        let colorful = imp.settings.boolean("colorful-labels");
        let matches: Vec<(String, Option<String>, usize)> = store
            .state
            .tag
            .iter()
            .filter(|t| t.id != TODAY_TAG_ID && t.title.to_lowercase().starts_with(&lower))
            .map(|t| {
                (
                    t.title.clone(),
                    tag_color(t).filter(|_| colorful).map(str::to_string),
                    t.task_ids.len(),
                )
            })
            .take(8)
            .collect();
        drop(store);
        imp.tag_list.remove_all();
        for (title, color, count) in &matches {
            // Same look as the sidebar: coloured tag icon, name, and a dim task count.
            let icon = gtk::Image::from_icon_name("tag-symbolic");
            if let Some(class) = color.as_deref().and_then(|c| self.color_class(c)) {
                icon.add_css_class(&class);
            }
            let row = gtk::Box::builder()
                .spacing(8)
                .margin_start(6)
                .margin_end(6)
                .margin_top(4)
                .margin_bottom(4)
                .build();
            row.append(&icon);
            row.append(&gtk::Label::builder().label(title).xalign(0.0).hexpand(true).build());
            row.append(
                &gtk::Label::builder()
                    .label(count.to_string())
                    .css_classes(["dim-label", "caption"])
                    .build(),
            );
            imp.tag_list.append(&row);
        }
        let matches: Vec<String> = matches.into_iter().map(|(t, _, _)| t).collect();
        *imp.tag_matches.borrow_mut() = matches;
        if imp.tag_matches.borrow().is_empty() {
            imp.tag_popover.popdown();
        } else {
            imp.tag_list.select_row(imp.tag_list.row_at_index(0).as_ref());
            // Anchor under the text cursor rather than centred below the whole entry.
            let entry: &gtk::Entry = &imp.add_entry;
            let byte_index = entry
                .text()
                .char_indices()
                .nth(entry.position() as usize)
                .map(|(i, _)| i)
                .unwrap_or(entry.text().len());
            // GTK 4 entries expose no cursor rect: measure the text before the cursor and add
            // the primary icon plus padding.
            let text = entry.text();
            let x = entry.create_pango_layout(Some(&text[..byte_index])).pixel_size().0 + 36;
            imp.tag_popover
                .set_pointing_to(Some(&gtk::gdk::Rectangle::new(x.max(0), 0, 1, entry.height())));
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

    /// Toast with an Undo button that dispatches the given actions.
    pub fn toast_undo(&self, msg: &str, undo: Vec<Action>) {
        let toast = adw::Toast::builder().title(msg).button_label(gettext("Undo")).build();
        toast.connect_button_clicked(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |_| {
                for a in undo.clone() {
                    w.dispatch(a);
                }
            }
        ));
        self.imp().toast_overlay.add_toast(toast);
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

    /// Done top-level tasks with their subtasks, ready to archive.
    fn done_tasks(&self) -> (Vec<Task>, Vec<Task>) {
        let store = self.imp().store.borrow();
        let tasks: Vec<Task> = store
            .state
            .task
            .iter()
            .filter(|t| t.is_done && t.parent_id.is_none())
            .cloned()
            .collect();
        let sub_tasks = tasks
            .iter()
            .flat_map(|t| {
                t.sub_task_ids
                    .iter()
                    .filter_map(|i| store.state.task.entities.get(i).cloned())
            })
            .collect();
        (tasks, sub_tasks)
    }

    fn archive_done(&self) {
        let (tasks, sub_tasks) = self.done_tasks();
        if tasks.is_empty() {
            return;
        }
        let n = tasks.len();
        let undo: Vec<Action> = tasks
            .iter()
            .map(|t| Action::RestoreTask {
                task: t.clone(),
                sub_tasks: sub_tasks
                    .iter()
                    .filter(|s| s.parent_id.as_deref() == Some(&t.id))
                    .cloned()
                    .collect(),
            })
            .collect();
        self.dispatch(Action::MoveToArchive { tasks, sub_tasks });
        self.toast_undo(&format!("{n} {}", gettext("completed tasks archived")), undo);
        if self.sync_configured() {
            self.sync();
        }
    }

    fn update_context_actions(&self) {
        let view = self.imp().view.borrow().clone();
        let (editable, deletable) = match &view {
            View::Project(id) => (true, id != INBOX_PROJECT_ID),
            View::Tag(_) => (true, true),
            _ => (false, false),
        };
        for (name, on) in [("edit-context", editable), ("delete-context", deletable)] {
            if let Some(a) = self.lookup_action(name).and_downcast::<gio::SimpleAction>() {
                a.set_enabled(on);
            }
        }
    }

    pub fn refresh(&self) {
        *self.imp().index.borrow_mut() = None;
        let (done, _) = self.done_tasks();
        // Menu item stays visible but disabled when there is nothing to archive (HIG).
        if let Some(a) = self.lookup_action("archive-done").and_downcast::<gio::SimpleAction>() {
            a.set_enabled(!done.is_empty());
        }
        self.refresh_sidebar();
        self.refresh_tasks();
    }

    /// CSS class that colours symbolic icons with a project/tag colour from the sync data.
    pub fn color_class(&self, color: &str) -> Option<String> {
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
        self.sidebar_row(
            &gettext("Coming Up"),
            "x-office-calendar-symbolic",
            Some(View::Upcoming),
            None,
            None,
        );
        self.sidebar_row(&gettext("Archive"), "archive-symbolic", Some(View::Archive), None, None);
        self.sidebar_row(&gettext("Search"), "edit-find-symbolic", Some(View::Search), None, None);
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

    /// Switch view, leaving search, and select the matching sidebar row.
    pub fn go_to(&self, view: View) {
        let imp = self.imp();
        imp.archive_shown.set(100);
        *imp.view.borrow_mut() = view;
        self.refresh();
    }

    fn section_header(&self, title: &str, first: bool) {
        let imp = self.imp();
        let label = gtk::Label::builder()
            .label(title)
            .xalign(0.0)
            .margin_start(12)
            .margin_end(12)
            .margin_top(if first { 10 } else { 28 })
            .margin_bottom(6)
            .css_classes(["heading"])
            .build();
        imp.task_list.append(
            &gtk::ListBoxRow::builder()
                .child(&label)
                .selectable(false)
                .activatable(false)
                .build(),
        );
        imp.rows.borrow_mut().push(String::new());
    }

    fn section_note(&self, text: &str) {
        let imp = self.imp();
        let label = gtk::Label::builder()
            .label(text)
            .xalign(0.0)
            .wrap(true)
            .margin_start(12)
            .margin_end(12)
            .margin_top(6)
            .margin_bottom(8)
            .css_classes(["dim-label", "caption"])
            .build();
        imp.task_list.append(
            &gtk::ListBoxRow::builder()
                .child(&label)
                .selectable(false)
                .activatable(false)
                .build(),
        );
        imp.rows.borrow_mut().push(String::new());
    }

    /// Global search across tasks (open, done, subtasks, archived), projects and tags.
    fn search_index(&self, store: &Store) -> Rc<SearchIndex> {
        if let Some(i) = self.imp().index.borrow().as_ref() {
            return i.clone();
        }
        let tag_name = |id: &String| {
            store
                .state
                .tag
                .entities
                .get(id)
                .map(|g| g.title.to_lowercase())
                .unwrap_or_default()
        };
        let hay = |t: &Task| {
            let mut h = t.title.to_lowercase();
            if let Some(n) = &t.notes {
                h.push('\n');
                h.push_str(&n.to_lowercase());
            }
            for tag in &t.tag_ids {
                h.push('\n');
                h.push_str(&tag_name(tag));
            }
            if let Some(p) = store.state.project.entities.get(&t.project_id) {
                h.push('\n');
                h.push_str(&p.title.to_lowercase());
            }
            h
        };
        let mut idx = SearchIndex {
            tasks: store.state.task.iter().map(|t| (t.id.clone(), hay(t))).collect(),
            archived: archived_tasks(store)
                .into_iter()
                .map(|t| {
                    let h = hay(&t);
                    (t, h)
                })
                .collect(),
            projects: store
                .state
                .project
                .iter()
                .map(|p| (p.id.clone(), p.title.to_lowercase()))
                .collect(),
            tags: store
                .state
                .tag
                .iter()
                .filter(|t| t.id != TODAY_TAG_ID)
                .map(|t| (t.id.clone(), t.title.to_lowercase()))
                .collect(),
        };
        idx.archived
            .sort_by_key(|(t, _)| std::cmp::Reverse(t.done_on.unwrap_or(t.created)));
        let idx = Rc::new(idx);
        *self.imp().index.borrow_mut() = Some(idx.clone());
        idx
    }

    /// Global search across tasks (open, done, subtasks, archived), projects and tags.
    fn render_search(&self, store: &Store, query: &str) {
        let imp = self.imp();
        let idx = self.search_index(store);
        // Every word must match somewhere in the haystack.
        let words: Vec<String> = query.to_lowercase().split_whitespace().map(str::to_string).collect();
        let matches = |h: &str| words.iter().all(|w| h.contains(w.as_str()));
        let mut first = true;
        let mut tasks: Vec<&Task> = idx
            .tasks
            .iter()
            .filter(|(_, h)| matches(h))
            .filter_map(|(id, _)| store.state.task.entities.get(id))
            .collect();
        tasks.sort_by(|a, b| {
            a.is_done
                .cmp(&b.is_done)
                .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
        });
        let total = tasks.len();
        if total > 0 {
            self.section_header(&format!("{} ({total})", gettext("Tasks")), first);
            first = false;
            for t in tasks.iter().take(SEARCH_LIMIT) {
                self.task_row(t, store, t.parent_id.is_some(), false);
            }
            if total > SEARCH_LIMIT {
                self.section_note(&format!(
                    "{} {SEARCH_LIMIT} {} {total}. {}",
                    gettext("Showing"),
                    gettext("of"),
                    gettext("Add another word to narrow it down.")
                ));
            }
        }
        let colorful = imp.settings.boolean("colorful-labels");
        let projects: Vec<&Project> = idx
            .projects
            .iter()
            .filter(|(_, h)| matches(h))
            .filter_map(|(id, _)| store.state.project.entities.get(id))
            .collect();
        if !projects.is_empty() {
            self.section_header(&gettext("Projects"), first);
            first = false;
            for p in projects {
                let row = adw::ActionRow::builder()
                    .title(glib::markup_escape_text(&p.title))
                    .activatable(true)
                    .build();
                let icon = gtk::Image::from_icon_name("folder-symbolic");
                if let Some(c) = p.color().filter(|_| colorful).and_then(|c| self.color_class(c)) {
                    icon.add_css_class(&c);
                }
                row.add_prefix(&icon);
                row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
                imp.task_list.append(&row);
                imp.rows.borrow_mut().push(format!("project:{}", p.id));
            }
        }
        let tags: Vec<&Tag> = idx
            .tags
            .iter()
            .filter(|(_, h)| matches(h))
            .filter_map(|(id, _)| store.state.tag.entities.get(id))
            .collect();
        if !tags.is_empty() {
            self.section_header(&gettext("Tags"), first);
            first = false;
            for g in tags {
                let row = adw::ActionRow::builder()
                    .title(glib::markup_escape_text(&g.title))
                    .activatable(true)
                    .build();
                let icon = gtk::Image::from_icon_name("tag-symbolic");
                if let Some(c) = tag_color(g).filter(|_| colorful).and_then(|c| self.color_class(c)) {
                    icon.add_css_class(&c);
                }
                row.add_prefix(&icon);
                row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
                imp.task_list.append(&row);
                imp.rows.borrow_mut().push(format!("tag:{}", g.id));
            }
        }
        let archived: Vec<&Task> = idx
            .archived
            .iter()
            .filter(|(_, h)| matches(h))
            .map(|(t, _)| t)
            .collect();
        if !archived.is_empty() {
            self.section_header(&format!("{} ({})", gettext("Archived"), archived.len()), first);
            for t in archived.iter().take(SEARCH_LIMIT / 2) {
                self.task_row(t, store, false, true);
            }
            if archived.len() > SEARCH_LIMIT / 2 {
                self.section_note(&format!(
                    "{} {} {} {}.",
                    gettext("Showing"),
                    SEARCH_LIMIT / 2,
                    gettext("of"),
                    archived.len()
                ));
            }
        }
        let none = imp.rows.borrow().is_empty();
        imp.empty.set_icon_name(Some("edit-find-symbolic"));
        imp.empty.set_title(&gettext("No Results Found"));
        imp.empty.set_description(Some(&gettext("Try a different search")));
        imp.empty.set_visible(none);
        imp.task_list.set_visible(!none);
        self.update_sync_button();
    }

    fn view_task_ids(&self, store: &Store) -> Vec<String> {
        match &*self.imp().view.borrow() {
            View::Today => store.state.today_ids(),
            View::Upcoming | View::Archive | View::Search => vec![],
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

    fn task_row(&self, t: &Task, store: &Store, indent: bool, archived: bool) {
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
        let repeat = t
            .repeat_cfg_id
            .as_ref()
            .and_then(|id| store.state.task_repeat_cfg.entities.get(id))
            .map(repeat_text);
        if let Some(text) = &repeat {
            sub.push(glib::markup_escape_text(text).to_string());
        }
        if archived {
            if let Some(done) = t.done_on {
                let day = glib::DateTime::from_unix_local(done as i64 / 1000)
                    .and_then(|d| d.format("%Y-%m-%d"))
                    .map(|g| g.to_string());
                sub.push(
                    glib::markup_escape_text(&format!(
                        "{} {}",
                        gettext("Done"),
                        day.as_deref().map(fmt_day).unwrap_or_default()
                    ))
                    .to_string(),
                );
            }
        }
        if let Some(d) = &t.due_day {
            if !matches!(*imp.view.borrow(), View::Today | View::Upcoming) {
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
            .activatable(!archived)
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
            .sensitive(!archived)
            .build();
        let id = t.id.clone();
        check.connect_toggled(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |c| w.set_done(&id, c.is_active())
        ));
        row.add_prefix(&check);
        if let Some(text) = &repeat {
            let icon = gtk::Image::builder()
                .icon_name("media-playlist-repeat-symbolic")
                .tooltip_text(text)
                .css_classes(["dim-label"])
                .build();
            icon.update_property(&[gtk::accessible::Property::Label(text)]);
            row.add_suffix(&icon);
        }
        if !archived {
            let drag = gtk::DragSource::builder().actions(gtk::gdk::DragAction::MOVE).build();
            let tid = t.id.clone();
            drag.connect_prepare(move |_, _, _| Some(gtk::gdk::ContentProvider::for_value(&tid.to_value())));
            drag.connect_drag_begin(glib::clone!(
                #[weak]
                row,
                move |src, _| src.set_icon(Some(&gtk::WidgetPaintable::new(Some(&row))), 0, 0)
            ));
            row.add_controller(drag);
        }
        if !archived && !indent {
            // Drop another task here to place it before this one (manual order only).
            let target = gtk::DropTarget::new(String::static_type(), gtk::gdk::DragAction::MOVE);
            let before = t.id.clone();
            target.connect_drop(glib::clone!(
                #[weak(rename_to = w)]
                self,
                #[upgrade_or]
                false,
                move |_, value, _, _| {
                    let Ok(moved) = value.get::<String>() else { return false };
                    w.reorder(&moved, &before)
                }
            ));
            row.add_controller(target);
        }
        imp.task_list.append(&row);
        imp.rows.borrow_mut().push(t.id.clone());
    }

    fn refresh_tasks(&self) {
        let imp = self.imp();
        self.update_context_actions();
        imp.task_list.remove_all();
        imp.rows.borrow_mut().clear();
        let store = imp.store.borrow();
        let title = match &*imp.view.borrow() {
            View::Today => gettext("Today"),
            View::Upcoming => gettext("Coming Up"),
            View::Archive => gettext("Archive"),
            View::Search => gettext("Search"),
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
        let searching = *imp.view.borrow() == View::Search;
        imp.add_clamp.set_visible(!searching);
        imp.search_clamp.set_visible(searching);
        if searching {
            imp.content_page.set_title(&gettext("Search"));
            let query = imp.filter.borrow().clone();
            if query.trim().is_empty() {
                imp.empty.set_icon_name(Some("edit-find-symbolic"));
                imp.empty.set_title(&gettext("Search Everything"));
                imp.empty
                    .set_description(Some(&gettext("Tasks, notes, subtasks, projects, tags and the archive")));
                imp.empty.set_visible(true);
                imp.task_list.set_visible(false);
                self.update_sync_button();
            } else {
                self.render_search(&store, query.trim());
            }
            return;
        }
        imp.content_page.set_title(&title);
        imp.empty.set_icon_name(Some("io.github.dan_hart.Momentum-symbolic"));
        imp.empty.set_title(&gettext("Nothing here yet"));
        imp.empty.set_description(Some(&gettext(
            "Add a task above, or sync with Nextcloud from Preferences.",
        )));
        let filter = String::new();
        if *imp.view.borrow() == View::Upcoming {
            let today_n = day_number(&today_str()).unwrap_or(0);
            let range: i64 = imp.settings.string("upcoming-range").parse().unwrap_or(7);
            let mut tasks: Vec<&Task> = store
                .state
                .task
                .iter()
                .filter(|t| !t.is_done && t.parent_id.is_none() && t.title.to_lowercase().contains(&*filter))
                .filter(|t| {
                    t.due_day
                        .as_deref()
                        .and_then(day_number)
                        .is_some_and(|d| d > today_n && d <= today_n + range)
                })
                .collect();
            tasks.sort_by(|a, b| a.due_day.cmp(&b.due_day).then_with(|| a.title.cmp(&b.title)));
            let mut current_day = String::new();
            for t in tasks {
                let day = t.due_day.clone().unwrap_or_default();
                if day != current_day {
                    let first = current_day.is_empty();
                    current_day = day.clone();
                    let label = gtk::Label::builder()
                        .label(fmt_day(&day))
                        .xalign(0.0)
                        .margin_start(12)
                        .margin_end(12)
                        .margin_top(if first { 10 } else { 28 })
                        .margin_bottom(6)
                        .css_classes(["heading"])
                        .build();
                    imp.task_list.append(
                        &gtk::ListBoxRow::builder()
                            .child(&label)
                            .selectable(false)
                            .activatable(false)
                            .build(),
                    );
                    imp.rows.borrow_mut().push(String::new());
                }
                self.task_row(t, &store, false, false);
            }
            imp.empty.set_visible(imp.rows.borrow().is_empty());
            imp.task_list.set_visible(!imp.rows.borrow().is_empty());
            return;
        }
        if *imp.view.borrow() == View::Archive {
            // Read-only view over archiveYoung + archiveOld, newest completion first.
            // Uses the cached, pre-sorted index and renders in pages: the archive can hold
            // thousands of tasks and every row is a real widget.
            let idx = self.search_index(&store);
            let limit = imp.archive_shown.get();
            let parents: Vec<&Task> = idx
                .archived
                .iter()
                .map(|(t, _)| t)
                .filter(|t| t.parent_id.is_none())
                .collect();
            for t in parents.iter().take(limit) {
                self.task_row(t, &store, false, true);
            }
            if parents.len() > limit {
                let more = gtk::Button::builder()
                    .label(format!(
                        "{} ({} {})",
                        gettext("Show More"),
                        parents.len() - limit,
                        gettext("remaining")
                    ))
                    .css_classes(["flat"])
                    .margin_top(6)
                    .margin_bottom(6)
                    .build();
                more.connect_clicked(glib::clone!(
                    #[weak(rename_to = w)]
                    self,
                    move |_| {
                        w.imp().archive_shown.set(w.imp().archive_shown.get() + 200);
                        w.refresh_tasks();
                    }
                ));
                imp.task_list.append(
                    &gtk::ListBoxRow::builder()
                        .child(&more)
                        .selectable(false)
                        .activatable(false)
                        .build(),
                );
                imp.rows.borrow_mut().push(String::new());
            }
            imp.empty.set_visible(imp.rows.borrow().is_empty());
            imp.task_list.set_visible(!imp.rows.borrow().is_empty());
            return;
        }
        let ids = self.view_task_ids(&store);
        let (mut open, mut done) = (vec![], vec![]);
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
        let descending = imp.settings.string("sort-direction") == "descending";
        for list in [&mut open, &mut done] {
            match sort.as_str() {
                "title" => list.sort_by_key(|t| t.title.to_lowercase()),
                "due" => list.sort_by(|a, b| {
                    a.due_day
                        .is_none()
                        .cmp(&b.due_day.is_none())
                        .then_with(|| a.due_day.cmp(&b.due_day))
                }),
                "estimate" => list.sort_by(|a, b| a.time_estimate.total_cmp(&b.time_estimate)),
                "created" => list.sort_by_key(|t| t.created),
                _ => {}
            }
            if descending {
                list.reverse();
            }
        }
        for t in open.into_iter().chain(done) {
            self.task_row(t, &store, false, false);
            for s in t.sub_task_ids.iter().filter_map(|i| store.state.task.entities.get(i)) {
                self.task_row(s, &store, true, false);
            }
        }
        imp.empty.set_visible(imp.rows.borrow().is_empty());
        imp.task_list.set_visible(!imp.rows.borrow().is_empty());
        self.update_sync_button();
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
                let undo = match task.due_day.clone() {
                    Some(day) => Action::PlanForToday {
                        task_ids: vec![task.id.clone()],
                        today: day,
                    },
                    None => Action::RemoveFromToday {
                        task_ids: vec![task.id.clone()],
                    },
                };
                self.toast_undo(&gettext("Planned for today"), vec![undo]);
            }
            View::Project(pid) if task.parent_id.is_none() && *pid != task.project_id => {
                let moved = Task {
                    project_id: pid.clone(),
                    ..task.clone()
                };
                let undo = Action::MoveToProject {
                    task: moved,
                    sub_tasks: sub_tasks.clone(),
                    target_project_id: task.project_id.clone(),
                };
                self.dispatch(Action::MoveToProject {
                    task: task.clone(),
                    sub_tasks,
                    target_project_id: pid.clone(),
                });
                let name = self
                    .imp()
                    .store
                    .borrow()
                    .state
                    .project
                    .entities
                    .get(pid)
                    .map(|p| p.title.clone())
                    .unwrap_or_default();
                self.toast_undo(&format!("{} {name}", gettext("Moved to")), vec![undo]);
            }
            View::Tag(tid) if !task.tag_ids.contains(tid) => {
                let mut ids = task.tag_ids.clone();
                ids.push(tid.clone());
                let undo = Action::UpdateTask {
                    id: task.id.clone(),
                    changes: [("tagIds".to_string(), json!(task.tag_ids))].into_iter().collect(),
                };
                self.update_task(&task.id, [("tagIds".to_string(), json!(ids))].into_iter().collect());
                let name = self
                    .imp()
                    .store
                    .borrow()
                    .state
                    .tag
                    .entities
                    .get(tid)
                    .map(|t| t.title.clone())
                    .unwrap_or_default();
                self.toast_undo(&format!("{} #{name}", gettext("Tagged")), vec![undo]);
            }
            _ => return false,
        }
        true
    }

    fn set_done(&self, id: &str, done: bool) {
        self.update_task(id, [("isDone".to_string(), json!(done))].into_iter().collect());
        if done {
            let undo = Action::UpdateTask {
                id: id.into(),
                changes: [("isDone".to_string(), json!(false))].into_iter().collect(),
            };
            self.toast_undo(&gettext("Task completed"), vec![undo]);
        }
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

    /// Drag reorder: put `moved` before `before` in the current context list.
    fn reorder(&self, moved: &str, before: &str) -> bool {
        let imp = self.imp();
        if moved == before {
            return false;
        }
        if imp.settings.string("task-sort") != "manual" {
            self.toast(&gettext("Switch to Manual Order to rearrange tasks"));
            return false;
        }
        let (context_type, context_id) = match &*imp.view.borrow() {
            View::Today => ("TAG", TODAY_TAG_ID.to_string()),
            View::Project(id) => ("PROJECT", id.clone()),
            View::Tag(id) => ("TAG", id.clone()),
            _ => return false,
        };
        let list: Vec<String> = self
            .view_task_ids(&imp.store.borrow())
            .into_iter()
            .filter(|i| i != moved)
            .collect();
        let Some(pos) = list.iter().position(|i| i == before) else {
            return false;
        };
        let after_task_id = if pos == 0 { None } else { list.get(pos - 1).cloned() };
        self.dispatch(Action::MoveInList {
            task_id: moved.into(),
            after_task_id,
            context_type: context_type.into(),
            context_id,
        });
        true
    }

    /// Ctrl+M: pick a project for the focused task.
    fn move_to_dialog(&self, task_id: &str) {
        let store = self.imp().store.borrow();
        let Some(task) = store.state.task.entities.get(task_id).cloned() else {
            return;
        };
        let projects: Vec<(String, String)> = store
            .state
            .project
            .iter()
            .filter(|p| !p.is_archived)
            .map(|p| (p.id.clone(), p.title.clone()))
            .collect();
        drop(store);
        if task.parent_id.is_some() {
            self.toast(&gettext("Subtasks move with their parent task"));
            return;
        }
        let names: Vec<&str> = projects.iter().map(|(_, t)| t.as_str()).collect();
        let drop_down = gtk::DropDown::from_strings(&names);
        drop_down.set_selected(projects.iter().position(|(id, _)| *id == task.project_id).unwrap_or(0) as u32);
        let d = adw::AlertDialog::builder()
            .heading(gettext("Move to Project"))
            .body(task.title.clone())
            .extra_child(&drop_down)
            .default_response("move")
            .build();
        d.add_responses(&[("cancel", &gettext("Cancel")), ("move", &gettext("Move"))]);
        d.set_response_appearance("move", adw::ResponseAppearance::Suggested);
        d.connect_response(
            None,
            glib::clone!(
                #[weak(rename_to = w)]
                self,
                #[weak]
                drop_down,
                move |_, r| {
                    if r == "move" {
                        if let Some((pid, _)) = projects.get(drop_down.selected() as usize) {
                            w.drop_task(&task.id, &View::Project(pid.clone()));
                        }
                    }
                }
            ),
        );
        d.present(Some(self));
    }

    /// Rename and recolour the current project or tag.
    fn edit_context_dialog(&self) {
        let view = self.imp().view.borrow().clone();
        let store = self.imp().store.borrow();
        let (heading, title, color) = match &view {
            View::Project(id) => {
                let Some(p) = store.state.project.entities.get(id) else {
                    return;
                };
                (gettext("Edit Project"), p.title.clone(), p.color().map(str::to_string))
            }
            View::Tag(id) => {
                let Some(t) = store.state.tag.entities.get(id) else {
                    return;
                };
                (gettext("Edit Tag"), t.title.clone(), tag_color(t).map(str::to_string))
            }
            _ => return,
        };
        drop(store);
        let content = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(["boxed-list"])
            .build();
        let name = adw::EntryRow::builder().title(gettext("Name")).text(&title).build();
        let color_row = adw::ActionRow::builder().title(gettext("Colour")).build();
        let button = gtk::ColorDialogButton::builder()
            .dialog(&gtk::ColorDialog::new())
            .valign(gtk::Align::Center)
            .build();
        if let Some(c) = color.as_deref().and_then(|c| gtk::gdk::RGBA::parse(c).ok()) {
            button.set_rgba(&c);
        }
        color_row.add_suffix(&button);
        content.append(&name);
        content.append(&color_row);
        let d = adw::AlertDialog::builder()
            .heading(heading)
            .extra_child(&content)
            .default_response("save")
            .build();
        d.add_responses(&[("cancel", &gettext("Cancel")), ("save", &gettext("Save"))]);
        d.set_response_appearance("save", adw::ResponseAppearance::Suggested);
        d.connect_response(
            None,
            glib::clone!(
                #[weak(rename_to = w)]
                self,
                #[weak]
                name,
                #[weak]
                button,
                move |_, r| {
                    if r != "save" {
                        return;
                    }
                    let c = button.rgba();
                    let hex = format!(
                        "#{:02x}{:02x}{:02x}",
                        (c.red() * 255.0) as u8,
                        (c.green() * 255.0) as u8,
                        (c.blue() * 255.0) as u8
                    );
                    let new_title = name.text().trim().to_string();
                    let mut ch = Map::new();
                    if !new_title.is_empty() {
                        ch.insert("title".into(), json!(new_title));
                    }
                    match &view {
                        View::Project(id) => {
                            let mut theme = w
                                .imp()
                                .store
                                .borrow()
                                .state
                                .project
                                .entities
                                .get(id)
                                .map(|p| p.theme.clone())
                                .unwrap_or(json!({}));
                            theme["primary"] = json!(hex);
                            ch.insert("theme".into(), theme);
                            w.dispatch(Action::UpdateProject {
                                id: id.clone(),
                                changes: ch,
                            });
                        }
                        View::Tag(id) => {
                            let mut theme = w
                                .imp()
                                .store
                                .borrow()
                                .state
                                .tag
                                .entities
                                .get(id)
                                .map(|t| t.theme.clone())
                                .unwrap_or(json!({}));
                            theme["primary"] = json!(hex);
                            ch.insert("theme".into(), theme);
                            ch.insert("color".into(), json!(hex));
                            w.dispatch(Action::UpdateTag {
                                id: id.clone(),
                                changes: ch,
                            });
                        }
                        _ => {}
                    }
                }
            ),
        );
        d.present(Some(self));
    }

    /// Delete the current project (with its tasks) or tag, after confirmation.
    fn delete_context_dialog(&self) {
        let view = self.imp().view.borrow().clone();
        let store = self.imp().store.borrow();
        let (heading, body, action) = match &view {
            View::Project(id) if id != INBOX_PROJECT_ID => {
                let Some(p) = store.state.project.entities.get(id) else {
                    return;
                };
                let all: Vec<String> = store
                    .state
                    .task
                    .iter()
                    .filter(|t| t.project_id == *id)
                    .map(|t| t.id.clone())
                    .collect();
                (
                    format!("{} “{}”?", gettext("Delete"), p.title),
                    format!(
                        "{} {}",
                        all.len(),
                        gettext("tasks in this project will be deleted. This cannot be undone.")
                    ),
                    Action::DeleteProject {
                        project_id: id.clone(),
                        note_ids: p.note_ids.clone(),
                        all_task_ids: all,
                    },
                )
            }
            View::Tag(id) => {
                let Some(t) = store.state.tag.entities.get(id) else {
                    return;
                };
                (
                    format!("{} “{}”?", gettext("Delete"), t.title),
                    gettext("Tasks keep their other tags."),
                    Action::DeleteTag { id: id.clone() },
                )
            }
            _ => return,
        };
        drop(store);
        let d = adw::AlertDialog::builder()
            .heading(heading)
            .body(body)
            .default_response("cancel")
            .build();
        d.add_responses(&[("cancel", &gettext("Cancel")), ("delete", &gettext("Delete"))]);
        d.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
        d.connect_response(
            None,
            glib::clone!(
                #[weak(rename_to = w)]
                self,
                move |_, r| {
                    if r == "delete" {
                        *w.imp().view.borrow_mut() = View::Today;
                        w.dispatch(action.clone());
                    }
                }
            ),
        );
        d.present(Some(self));
    }

    /// Create today's instances of repeating tasks, like upstream's TaskRepeatCfgService.
    pub fn spawn_repeats(&self) {
        let today = today_str();
        let due: Vec<RepeatCfg> = {
            let store = self.imp().store.borrow();
            let archived: Vec<Task> = archived_tasks(&store);
            store
                .state
                .task_repeat_cfg
                .iter()
                .filter(|c| c.is_due(&today))
                .filter(|c| {
                    let id = format!("rpt_{}_{}", c.id, today);
                    !store.state.task.entities.contains_key(&id) && !archived.iter().any(|t| t.id == id)
                })
                .cloned()
                .collect()
        };
        for cfg in due {
            let project = cfg
                .project_id
                .clone()
                .filter(|p| !p.is_empty())
                .unwrap_or_else(|| self.current_project(&self.imp().store.borrow()));
            let mut task = Task::new(cfg.title.as_deref().unwrap_or(""), &project);
            task.id = format!("rpt_{}_{}", cfg.id, today);
            task.repeat_cfg_id = Some(cfg.id.clone());
            task.time_estimate = cfg.default_estimate.unwrap_or(0.0);
            task.notes = cfg.notes.clone().filter(|n| !n.is_empty());
            task.due_day = Some(today.clone());
            task.tag_ids = cfg.tag_ids.iter().filter(|t| *t != TODAY_TAG_ID).cloned().collect();
            self.imp()
                .store
                .borrow_mut()
                .dispatch(Action::AddTask { task, bottom: true });
            let changes = [
                ("lastTaskCreationDay".to_string(), json!(today)),
                ("lastTaskCreation".to_string(), json!(now_ms())),
            ]
            .into_iter()
            .collect();
            self.imp().store.borrow_mut().dispatch(Action::UpdateRepeatCfg {
                id: cfg.id.clone(),
                changes,
            });
        }
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
            self,
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
            self,
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
        if std::env::var_os("MOMENTUM_DEMO").is_some() {
            return;
        }
        if !imp.settings.boolean("sync-enabled") {
            self.toast(&gettext("Sync is turned off. Enable it in Preferences."));
            return;
        }
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
                        return Err(("Nextcloud sync is not configured".to_string(), true));
                    }
                    let mut s = snapshot;
                    sp_sync::sync(&cfg, &mut s).map(|r| (s, r)).map_err(|e| {
                        use sp_sync::SyncError::*;
                        (
                            e.to_string(),
                            matches!(e, Encrypted | Decrypt(_) | Schema(_) | Version(_) | FreshState),
                        )
                    })
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
                        imp.banner.set_revealed(false);
                        w.spawn_repeats();
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
                    Err((e, actionable)) => {
                        tracing::warn!("sync failed: {e}");
                        if actionable {
                            // HIG: persistent, fixable problems get a banner with the fix, not a toast.
                            imp.banner
                                .set_title(&format!("{}: {e}", gettext("Sync needs attention")));
                            imp.banner.set_revealed(true);
                        } else {
                            w.toast(&format!("{}: {e}", gettext("Sync failed")));
                        }
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
