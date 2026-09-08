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

#[derive(Clone, Copy)]
pub enum MenuKind {
    Task,
    Project,
    Tag,
}

#[derive(Clone, PartialEq)]
pub enum View {
    Today,
    Tonight,
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
        pub task_box: TemplateChild<gtk::Box>,
        pub current_list: RefCell<Option<gtk::ListBox>>,
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
        pub selecting: Cell<bool>,
        pub selected: RefCell<HashSet<String>>,
        #[template_child]
        pub select_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub select_cancel: TemplateChild<gtk::Button>,
        #[template_child]
        pub select_bar: TemplateChild<gtk::ActionBar>,
        #[template_child]
        pub select_count: TemplateChild<gtk::Label>,
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
                task_box: Default::default(),
                current_list: Default::default(),
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
                selecting: Cell::new(false),
                selected: Default::default(),
                select_button: Default::default(),
                select_cancel: Default::default(),
                select_bar: Default::default(),
                select_count: Default::default(),
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
        imp.select_button.connect_toggled(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |b| w.set_selecting(b.is_active())
        ));
        self.update_selection_ui();
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
                if k == gtk::gdk::Key::Escape && w.imp().selecting.get() {
                    w.set_selecting(false);
                    return glib::Propagation::Stop;
                }
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
        let targeted = |name: &str, f: fn(&MomentumWindow, &str)| {
            gio::ActionEntry::builder(name)
                .parameter_type(Some(&String::static_variant_type()))
                .activate(move |w: &MomentumWindow, _, p: Option<&glib::Variant>| {
                    if let Some(id) = p.and_then(|v| v.get::<String>()) {
                        f(w, &id);
                    }
                })
                .build()
        };
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
            // Context-menu actions carry the target id as a string parameter.
            targeted("ctx-open", |w, id| w.open_task(id)),
            targeted("ctx-done", |w, id| {
                let done = w
                    .imp()
                    .store
                    .borrow()
                    .state
                    .task
                    .entities
                    .get(id)
                    .map(|t| t.is_done)
                    .unwrap_or(false);
                w.set_done(id, !done);
            }),
            targeted("ctx-today", |w, id| {
                let planned = w
                    .imp()
                    .store
                    .borrow()
                    .state
                    .task
                    .entities
                    .get(id)
                    .and_then(|t| t.due_day.clone())
                    == Some(today_str());
                if planned {
                    w.dispatch(Action::RemoveFromToday {
                        task_ids: vec![id.to_string()],
                    });
                    w.toast_undo(
                        &gettext("Removed from today"),
                        vec![Action::PlanForToday {
                            task_ids: vec![id.to_string()],
                            today: today_str(),
                        }],
                    );
                } else {
                    w.drop_task(id, &View::Today);
                }
            }),
            targeted("ctx-tonight", |w, id| {
                let ids = w.selection_or(id);
                w.toggle_tonight(&ids);
            }),
            targeted("ctx-move", |w, id| w.move_to_dialog(id)),
            targeted("ctx-delete", |w, id| w.delete_task(id)),
            targeted("ctx-open-project", |w, id| w.go_to(View::Project(id.into()))),
            targeted("ctx-new-task", |w, id| {
                w.go_to(View::Project(id.into()));
                w.new_task_dialog();
            }),
            targeted("ctx-edit-project", |w, id| {
                w.edit_context_dialog_for(View::Project(id.into()))
            }),
            targeted("ctx-delete-project", |w, id| {
                w.delete_context_dialog_for(View::Project(id.into()))
            }),
            targeted("ctx-open-tag", |w, id| w.go_to(View::Tag(id.into()))),
            targeted("ctx-edit-tag", |w, id| w.edit_context_dialog_for(View::Tag(id.into()))),
            targeted("ctx-delete-tag", |w, id| {
                w.delete_context_dialog_for(View::Tag(id.into()))
            }),
            act("plan-today", |w| {
                if let Some(id) = w.focused_task() {
                    w.drop_task(&id, &View::Today);
                }
            }),
            act("toggle-tonight", |w| {
                let ids: Vec<String> = if w.imp().selecting.get() {
                    w.selected_tasks().iter().map(|t| t.id.clone()).collect()
                } else {
                    w.focused_task().into_iter().collect()
                };
                w.toggle_tonight(&ids);
            }),
            act("move-to", |w| {
                if let Some(id) = w.focused_task() {
                    w.move_to_dialog(&id);
                }
            }),
            act("select-mode", |w| w.set_selecting(!w.imp().selecting.get())),
            act("select-cancel", |w| w.set_selecting(false)),
            act("select-all", |w| w.select_all()),
            act("sel-done", |w| w.bulk_done()),
            act("sel-today", |w| w.bulk_today()),
            act("sel-move", |w| {
                let ids: Vec<String> = w
                    .selected_tasks()
                    .iter()
                    .filter(|t| t.parent_id.is_none())
                    .map(|t| t.id.clone())
                    .collect();
                w.move_many_dialog(ids);
            }),
            act("sel-tag", |w| w.bulk_tag_dialog()),
            act("sel-delete", |w| w.bulk_delete()),
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
            if std::env::var_os("MOMENTUM_SCREENSHOT_SELECT").is_some() {
                let ids: Vec<String> = imp
                    .rows
                    .borrow()
                    .iter()
                    .filter(|r| !r.is_empty())
                    .take(2)
                    .cloned()
                    .collect();
                imp.selected.borrow_mut().extend(ids);
                self.set_selecting(true);
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
        let group = self.imp().task_box.focus_child()?;
        let list = group.focus_child()?.downcast::<gtk::ListBox>().ok()?;
        let row = list.focus_child()?.downcast::<gtk::ListBoxRow>().ok()?;
        let id = row.widget_name().to_string();
        (!id.is_empty() && !id.contains(':')).then_some(id)
    }

    // ---- selection mode (HIG: header toggle, per-row checks, action bar) ----

    pub fn set_selecting(&self, on: bool) {
        let imp = self.imp();
        if imp.selecting.get() == on {
            return;
        }
        imp.selecting.set(on);
        if !on {
            imp.selected.borrow_mut().clear();
        }
        imp.select_button.set_active(on);
        imp.select_cancel.set_visible(on);
        imp.select_bar.set_revealed(on);
        imp.add_clamp.set_visible(!on && *imp.view.borrow() != View::Search);
        self.refresh_tasks();
    }

    fn toggle_selected(&self, id: &str) {
        let imp = self.imp();
        {
            let mut sel = imp.selected.borrow_mut();
            if !sel.remove(id) {
                sel.insert(id.to_string());
            }
        }
        self.update_selection_ui();
    }

    fn update_selection_ui(&self) {
        let imp = self.imp();
        let n = imp.selected.borrow().len();
        imp.select_count.set_text(&format!(
            "{n} {}",
            if n == 1 {
                gettext("selected")
            } else {
                gettext("selected")
            }
        ));
        for name in ["sel-done", "sel-today", "sel-move", "sel-tag", "sel-delete"] {
            if let Some(a) = self.lookup_action(name).and_downcast::<gio::SimpleAction>() {
                a.set_enabled(n > 0);
            }
        }
        if imp.selecting.get() {
            imp.content_page.set_title(&format!("{n} {}", gettext("selected")));
        }
    }

    /// Ids to act on: the selection when in selection mode, else the given id.
    fn selection_or(&self, id: &str) -> Vec<String> {
        let imp = self.imp();
        let sel = imp.selected.borrow();
        if imp.selecting.get() && sel.contains(id) {
            // Keep the visual order of the current list.
            imp.rows.borrow().iter().filter(|r| sel.contains(*r)).cloned().collect()
        } else {
            vec![id.to_string()]
        }
    }

    fn selected_tasks(&self) -> Vec<Task> {
        let imp = self.imp();
        let store = imp.store.borrow();
        let sel = imp.selected.borrow();
        imp.rows
            .borrow()
            .iter()
            .filter(|r| sel.contains(*r))
            .filter_map(|id| store.state.task.entities.get(id).cloned())
            .collect()
    }

    fn select_all(&self) {
        let imp = self.imp();
        let ids: Vec<String> = imp
            .rows
            .borrow()
            .iter()
            .filter(|r| !r.is_empty() && !r.contains(':'))
            .cloned()
            .collect();
        let all = ids.iter().all(|id| imp.selected.borrow().contains(id));
        let mut sel = imp.selected.borrow_mut();
        if all {
            sel.clear();
        } else {
            sel.extend(ids);
        }
        drop(sel);
        self.refresh_tasks();
    }

    /// Move a task between the day and the evening: add or remove the Evening tag,
    /// planning it for today if it was not.
    fn toggle_tonight(&self, ids: &[String]) {
        let evening = self.ensure_evening_tag();
        let today = today_str();
        let tasks: Vec<Task> = {
            let store = self.imp().store.borrow();
            ids.iter()
                .filter_map(|i| store.state.task.entities.get(i).cloned())
                .collect()
        };
        if tasks.is_empty() {
            return;
        }
        // Whole batch goes the same direction as the first task.
        let to_tonight = !tasks[0].tag_ids.contains(&evening);
        let mut undo = vec![];
        for t in &tasks {
            let mut ids = t.tag_ids.clone();
            if to_tonight {
                if !ids.contains(&evening) {
                    ids.push(evening.clone());
                }
            } else {
                ids.retain(|i| *i != evening);
            }
            let mut ch = Map::new();
            ch.insert("tagIds".into(), json!(ids));
            let mut back = Map::new();
            back.insert("tagIds".into(), json!(t.tag_ids));
            if t.due_day.as_deref() != Some(&today) {
                ch.insert("dueDay".into(), json!(today));
                ch.insert("dueWithTime".into(), Value::Null);
                back.insert("dueDay".into(), json!(t.due_day));
            }
            undo.push(Action::UpdateTask {
                id: t.id.clone(),
                changes: back,
            });
            self.imp().store.borrow_mut().dispatch(Action::UpdateTask {
                id: t.id.clone(),
                changes: ch,
            });
        }
        if self.imp().selecting.get() {
            self.set_selecting(false);
        }
        self.refresh();
        let msg = match (to_tonight, tasks.len()) {
            (true, 1) => gettext("Moved to tonight"),
            (false, 1) => gettext("Moved to today"),
            (true, n) => format!("{n} {}", gettext("tasks moved to tonight")),
            (false, n) => format!("{n} {}", gettext("tasks moved to today")),
        };
        self.toast_undo(&msg, undo);
    }

    fn bulk_done(&self) {
        let tasks = self.selected_tasks();
        let n = tasks.len();
        let undo: Vec<Action> = tasks
            .iter()
            .map(|t| Action::UpdateTask {
                id: t.id.clone(),
                changes: [("isDone".to_string(), json!(t.is_done))].into_iter().collect(),
            })
            .collect();
        for t in &tasks {
            self.imp().store.borrow_mut().dispatch(Action::UpdateTask {
                id: t.id.clone(),
                changes: [("isDone".to_string(), json!(true))].into_iter().collect(),
            });
        }
        self.set_selecting(false);
        self.refresh();
        self.toast_undo(&format!("{n} {}", gettext("tasks completed")), undo);
    }

    fn bulk_today(&self) {
        let tasks = self.selected_tasks();
        let today = today_str();
        let ids: Vec<String> = tasks
            .iter()
            .filter(|t| t.due_day.as_deref() != Some(&today))
            .map(|t| t.id.clone())
            .collect();
        if ids.is_empty() {
            return;
        }
        let undo: Vec<Action> = tasks
            .iter()
            .filter(|t| ids.contains(&t.id))
            .map(|t| match &t.due_day {
                Some(d) => Action::PlanForToday {
                    task_ids: vec![t.id.clone()],
                    today: d.clone(),
                },
                None => Action::RemoveFromToday {
                    task_ids: vec![t.id.clone()],
                },
            })
            .collect();
        let n = ids.len();
        self.imp()
            .store
            .borrow_mut()
            .dispatch(Action::PlanForToday { task_ids: ids, today });
        self.set_selecting(false);
        self.refresh();
        self.toast_undo(&format!("{n} {}", gettext("tasks planned for today")), undo);
    }

    fn bulk_delete(&self) {
        let tasks = self.selected_tasks();
        let n = tasks.len();
        let mut undo = vec![];
        for t in &tasks {
            let subs: Vec<Task> = {
                let store = self.imp().store.borrow();
                t.sub_task_ids
                    .iter()
                    .filter_map(|i| store.state.task.entities.get(i).cloned())
                    .collect()
            };
            undo.push(Action::AddTask {
                task: t.clone(),
                bottom: true,
            });
            for st in &subs {
                undo.push(Action::AddSubTask {
                    task: st.clone(),
                    parent_id: t.id.clone(),
                });
            }
            self.imp().store.borrow_mut().dispatch(Action::DeleteTask {
                task: t.clone(),
                sub_tasks: subs,
            });
        }
        self.set_selecting(false);
        self.refresh();
        self.toast_undo(&format!("{n} {}", gettext("tasks deleted")), undo);
    }

    /// Bulk add a tag: dropdown of tags, or type a new one.
    fn bulk_tag_dialog(&self) {
        let tasks = self.selected_tasks();
        if tasks.is_empty() {
            return;
        }
        let tags: Vec<(String, String)> = self
            .imp()
            .store
            .borrow()
            .state
            .tag
            .iter()
            .filter(|t| t.id != TODAY_TAG_ID)
            .map(|t| (t.id.clone(), t.title.clone()))
            .collect();
        let names: Vec<&str> = tags.iter().map(|(_, t)| t.as_str()).collect();
        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .build();
        let drop_down = gtk::DropDown::from_strings(&names);
        let entry = gtk::Entry::builder()
            .placeholder_text(gettext("Or a new tag name"))
            .build();
        content.append(&drop_down);
        content.append(&entry);
        let d = adw::AlertDialog::builder()
            .heading(format!(
                "{} {} {}",
                gettext("Add Tag to"),
                tasks.len(),
                gettext("Tasks")
            ))
            .extra_child(&content)
            .default_response("add")
            .build();
        d.add_responses(&[("cancel", &gettext("Cancel")), ("add", &gettext("Add"))]);
        d.set_response_appearance("add", adw::ResponseAppearance::Suggested);
        d.connect_response(
            None,
            glib::clone!(
                #[weak(rename_to = w)]
                self,
                #[weak]
                drop_down,
                #[weak]
                entry,
                move |_, r| {
                    if r != "add" {
                        return;
                    }
                    let typed = entry.text().trim().to_string();
                    let tag_id = if !typed.is_empty() {
                        let existing = w
                            .imp()
                            .store
                            .borrow()
                            .state
                            .tag
                            .iter()
                            .find(|g| g.title.eq_ignore_ascii_case(&typed))
                            .map(|g| g.id.clone());
                        existing.unwrap_or_else(|| {
                            let tag = Tag::new(&typed);
                            let id = tag.id.clone();
                            w.imp().store.borrow_mut().dispatch(Action::AddTag { tag });
                            id
                        })
                    } else {
                        match tags.get(drop_down.selected() as usize) {
                            Some((id, _)) => id.clone(),
                            None => return,
                        }
                    };
                    let mut undo = vec![];
                    for t in &tasks {
                        if t.tag_ids.contains(&tag_id) {
                            continue;
                        }
                        let mut ids = t.tag_ids.clone();
                        ids.push(tag_id.clone());
                        undo.push(Action::UpdateTask {
                            id: t.id.clone(),
                            changes: [("tagIds".to_string(), json!(t.tag_ids))].into_iter().collect(),
                        });
                        w.imp().store.borrow_mut().dispatch(Action::UpdateTask {
                            id: t.id.clone(),
                            changes: [("tagIds".to_string(), json!(ids))].into_iter().collect(),
                        });
                    }
                    let n = undo.len();
                    w.set_selecting(false);
                    w.refresh();
                    w.toast_undo(&format!("{n} {}", gettext("tasks tagged")), undo);
                }
            ),
        );
        d.present(Some(self));
    }

    fn activate_row(&self, id: &str) {
        if self.imp().selecting.get() && !id.is_empty() && !id.contains(':') {
            self.toggle_selected(id);
            self.refresh_tasks();
            return;
        }
        if let Some(pid) = id.strip_prefix("project:") {
            self.go_to(View::Project(pid.into()));
        } else if let Some(tid) = id.strip_prefix("tag:") {
            self.go_to(View::Tag(tid.into()));
        } else if !id.is_empty() {
            self.open_task(id);
        }
    }

    /// Right-click, long-press, or Menu/Shift+F10 on a row opens a context menu built for it.
    fn attach_context_menu(&self, widget: &impl IsA<gtk::Widget>, kind: MenuKind, id: String) {
        let widget: gtk::Widget = widget.clone().upcast();
        let show = glib::clone!(
            #[weak(rename_to = w)]
            self,
            #[weak]
            widget,
            move |x: f64, y: f64| {
                let menu = w.context_menu_model(&kind, &id);
                let popover = gtk::PopoverMenu::from_model(Some(&menu));
                popover.set_parent(&widget);
                popover.set_has_arrow(false);
                popover.set_halign(gtk::Align::Start);
                popover.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
                popover.connect_closed(|p| {
                    let p = p.clone();
                    glib::idle_add_local_once(move || p.unparent());
                });
                popover.popup();
            }
        );
        let right = gtk::GestureClick::builder().button(3).build();
        right.connect_pressed(glib::clone!(
            #[strong]
            show,
            move |g, _, x, y| {
                g.set_state(gtk::EventSequenceState::Claimed);
                show(x, y);
            }
        ));
        widget.add_controller(right);
        let long = gtk::GestureLongPress::builder().touch_only(true).build();
        long.connect_pressed(glib::clone!(
            #[strong]
            show,
            move |_, x, y| show(x, y)
        ));
        widget.add_controller(long);
        let key = gtk::EventControllerKey::new();
        key.connect_key_pressed(glib::clone!(
            #[strong]
            show,
            #[weak]
            widget,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_, k, _, m| {
                use gtk::gdk::Key;
                if k == Key::Menu || (k == Key::F10 && m.contains(gtk::gdk::ModifierType::SHIFT_MASK)) {
                    show(widget.width() as f64 / 2.0, widget.height() as f64 / 2.0);
                    glib::Propagation::Stop
                } else {
                    glib::Propagation::Proceed
                }
            }
        ));
        widget.add_controller(key);
    }

    fn context_menu_model(&self, kind: &MenuKind, id: &str) -> gio::Menu {
        let store = self.imp().store.borrow();
        let today = today_str();
        let menu = gio::Menu::new();
        let item = |label: String, action: &str| {
            let it = gio::MenuItem::new(Some(&label), None);
            it.set_action_and_target_value(Some(action), Some(&id.to_variant()));
            it
        };
        match kind {
            MenuKind::Task => {
                let Some(t) = store.state.task.entities.get(id) else {
                    return menu;
                };
                let a = gio::Menu::new();
                a.append_item(&item(gettext("Open"), "win.ctx-open"));
                a.append_item(&item(
                    if t.is_done {
                        gettext("Mark as Not Done")
                    } else {
                        gettext("Mark as Done")
                    },
                    "win.ctx-done",
                ));
                let planned = t.due_day.as_deref() == Some(&today);
                a.append_item(&item(
                    if planned {
                        gettext("Remove from Today")
                    } else {
                        gettext("Plan for Today")
                    },
                    "win.ctx-today",
                ));
                let evening = Self::evening_tag_id(&store);
                let tonight = evening.as_ref().is_some_and(|e| t.tag_ids.contains(e));
                a.append_item(&item(
                    if tonight {
                        gettext("Move to Today")
                    } else {
                        gettext("Move to Tonight")
                    },
                    "win.ctx-tonight",
                ));
                if t.parent_id.is_none() {
                    a.append_item(&item(gettext("Move to Project…"), "win.ctx-move"));
                }
                menu.append_section(None, &a);
                let b = gio::Menu::new();
                b.append_item(&item(gettext("Delete"), "win.ctx-delete"));
                menu.append_section(None, &b);
            }
            MenuKind::Project => {
                let a = gio::Menu::new();
                a.append_item(&item(gettext("Open"), "win.ctx-open-project"));
                a.append_item(&item(gettext("New Task Here…"), "win.ctx-new-task"));
                a.append_item(&item(gettext("Edit…"), "win.ctx-edit-project"));
                menu.append_section(None, &a);
                if id != INBOX_PROJECT_ID {
                    let b = gio::Menu::new();
                    b.append_item(&item(gettext("Delete Project…"), "win.ctx-delete-project"));
                    menu.append_section(None, &b);
                }
            }
            MenuKind::Tag => {
                let a = gio::Menu::new();
                a.append_item(&item(gettext("Open"), "win.ctx-open-tag"));
                a.append_item(&item(gettext("Edit…"), "win.ctx-edit-tag"));
                menu.append_section(None, &a);
                let b = gio::Menu::new();
                b.append_item(&item(gettext("Delete Tag…"), "win.ctx-delete-tag"));
                menu.append_section(None, &b);
            }
        }
        menu
    }

    /// Start a new boxed-list section (optionally titled), like an AdwPreferencesGroup.
    fn new_section(&self, title: Option<&str>) -> gtk::ListBox {
        let imp = self.imp();
        let group = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(6)
            .build();
        if let Some(t) = title {
            group.append(
                &gtk::Label::builder()
                    .label(t)
                    .xalign(0.0)
                    .margin_start(6)
                    .css_classes(["heading"])
                    .build(),
            );
        }
        let list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(["boxed-list"])
            .build();
        list.connect_row_activated(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |_, row| w.activate_row(&row.widget_name())
        ));
        group.append(&list);
        imp.task_box.append(&group);
        *imp.current_list.borrow_mut() = Some(list.clone());
        list
    }

    /// Append a row to the current section, creating an untitled one if needed.
    fn append_row(&self, row: &impl IsA<gtk::Widget>, id: &str) {
        // Take the clone out before the borrow guard is dropped: new_section() borrows mutably.
        let existing = self.imp().current_list.borrow().clone();
        let list = match existing {
            Some(l) => l,
            None => self.new_section(None),
        };
        row.set_widget_name(id);
        list.append(row);
        self.imp().rows.borrow_mut().push(id.to_string());
    }

    fn clear_sections(&self) {
        let imp = self.imp();
        while let Some(c) = imp.task_box.first_child() {
            imp.task_box.remove(&c);
        }
        *imp.current_list.borrow_mut() = None;
        imp.rows.borrow_mut().clear();
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
            match view.clone().unwrap() {
                View::Project(id) => self.attach_context_menu(&r, MenuKind::Project, id),
                View::Tag(id) => self.attach_context_menu(&r, MenuKind::Tag, id),
                _ => {}
            }
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
                    let mut any = false;
                    for id in task_id.split('\n') {
                        any |= w.drop_task(id, &dest);
                    }
                    any
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
            &gettext("Tonight"),
            "weather-clear-night-symbolic",
            Some(View::Tonight),
            None,
            None,
        );
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
        if imp.selecting.get() {
            imp.selecting.set(false);
            imp.selected.borrow_mut().clear();
            imp.select_button.set_active(false);
            imp.select_cancel.set_visible(false);
            imp.select_bar.set_revealed(false);
        }
        imp.archive_shown.set(100);
        *imp.view.borrow_mut() = view;
        self.refresh();
    }

    fn section_header(&self, title: &str, _first: bool) {
        self.new_section(Some(title));
    }

    fn section_note(&self, text: &str) {
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
        self.append_row(
            &gtk::ListBoxRow::builder()
                .child(&label)
                .selectable(false)
                .activatable(false)
                .build(),
            "",
        );
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
                self.append_row(&row, &format!("project:{}", p.id));
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
                self.append_row(&row, &format!("tag:{}", g.id));
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
        imp.task_box.set_visible(!none);
        self.update_sync_button();
    }

    /// The "Evening" tag (case-insensitive) marks Tonight tasks.
    fn evening_tag_id(store: &Store) -> Option<String> {
        store
            .state
            .tag
            .iter()
            .find(|t| t.title.eq_ignore_ascii_case("evening"))
            .map(|t| t.id.clone())
    }
    fn ensure_evening_tag(&self) -> String {
        if let Some(id) = Self::evening_tag_id(&self.imp().store.borrow()) {
            return id;
        }
        let tag = Tag::new("Evening");
        let id = tag.id.clone();
        self.imp().store.borrow_mut().dispatch(Action::AddTag { tag });
        id
    }
    fn is_tonight(store: &Store, t: &Task) -> bool {
        Self::evening_tag_id(store).is_some_and(|e| t.tag_ids.contains(&e))
    }

    fn view_task_ids(&self, store: &Store) -> Vec<String> {
        match &*self.imp().view.borrow() {
            View::Today => store.state.today_ids(),
            View::Tonight => store
                .state
                .today_ids()
                .into_iter()
                .filter(|id| {
                    store
                        .state
                        .task
                        .entities
                        .get(id)
                        .is_some_and(|t| Self::is_tonight(store, t))
                })
                .collect(),
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
            if !matches!(*imp.view.borrow(), View::Today | View::Tonight | View::Upcoming) {
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
        let selecting = imp.selecting.get() && !archived;
        let check = gtk::CheckButton::builder()
            .active(if selecting {
                imp.selected.borrow().contains(&t.id)
            } else {
                t.is_done
            })
            .valign(gtk::Align::Center)
            .sensitive(!archived)
            .build();
        let id = t.id.clone();
        if selecting {
            check.add_css_class("selection-mode");
            check.connect_toggled(glib::clone!(
                #[weak(rename_to = w)]
                self,
                move |_| w.toggle_selected(&id)
            ));
        } else {
            check.connect_toggled(glib::clone!(
                #[weak(rename_to = w)]
                self,
                move |c| w.set_done(&id, c.is_active())
            ));
        }
        row.add_prefix(&check);
        if !archived {
            // Ctrl+click enters selection mode with this task, like Files.
            let click = gtk::GestureClick::builder().button(1).build();
            let cid = t.id.clone();
            click.connect_pressed(glib::clone!(
                #[weak(rename_to = w)]
                self,
                move |g, _, _, _| {
                    if g.current_event_state().contains(gtk::gdk::ModifierType::CONTROL_MASK) {
                        g.set_state(gtk::EventSequenceState::Claimed);
                        w.imp().selected.borrow_mut().insert(cid.clone());
                        if w.imp().selecting.get() {
                            w.refresh_tasks();
                        } else {
                            w.set_selecting(true);
                        }
                    }
                }
            ));
            row.add_controller(click);
        }
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
            drag.connect_prepare(glib::clone!(
                #[weak(rename_to = w)]
                self,
                #[upgrade_or]
                None,
                move |_, _, _| {
                    // A selected row drags the whole selection, newline separated.
                    let ids = w.selection_or(&tid).join("\n");
                    Some(gtk::gdk::ContentProvider::for_value(&ids.to_value()))
                }
            ));
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
                    let mut any = false;
                    for id in moved.split('\n') {
                        any |= w.reorder(id, &before);
                    }
                    any
                }
            ));
            row.add_controller(target);
        }
        if !archived {
            self.attach_context_menu(&row, MenuKind::Task, t.id.clone());
        }
        self.append_row(&row, &t.id);
    }

    fn refresh_tasks(&self) {
        let imp = self.imp();
        self.update_context_actions();
        self.clear_sections();
        let store = imp.store.borrow();
        let title = match &*imp.view.borrow() {
            View::Today => gettext("Today"),
            View::Tonight => gettext("Tonight"),
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
        imp.add_clamp.set_visible(!searching && !imp.selecting.get());
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
                imp.task_box.set_visible(false);
                self.update_sync_button();
            } else {
                self.render_search(&store, query.trim());
            }
            return;
        }
        imp.content_page.set_title(&title);
        if imp.selecting.get() {
            self.update_selection_ui();
        }
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
                    current_day = day.clone();
                    self.new_section(Some(&fmt_day(&day)));
                }
                self.task_row(t, &store, false, false);
            }
            imp.empty.set_visible(imp.rows.borrow().is_empty());
            imp.task_box.set_visible(!imp.rows.borrow().is_empty());
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
                self.append_row(
                    &gtk::ListBoxRow::builder()
                        .child(&more)
                        .selectable(false)
                        .activatable(false)
                        .build(),
                    "",
                );
            }
            imp.empty.set_visible(imp.rows.borrow().is_empty());
            imp.task_box.set_visible(!imp.rows.borrow().is_empty());
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
        if *imp.view.borrow() == View::Today {
            for list in [&mut open, &mut done] {
                list.sort_by_key(|t| Self::is_tonight(&store, t)); // stable: keeps order within each group
            }
        }
        let split_tonight =
            *imp.view.borrow() == View::Today && open.iter().chain(done.iter()).any(|t| Self::is_tonight(&store, t));
        let mut rendered_section: Option<bool> = None; // Some(is_tonight) of the current section
        for t in open.into_iter().chain(done) {
            if split_tonight {
                let tonight = Self::is_tonight(&store, t);
                if rendered_section != Some(tonight) {
                    rendered_section = Some(tonight);
                    self.new_section(Some(&if tonight { gettext("Tonight") } else { gettext("Today") }));
                }
            }
            self.task_row(t, &store, false, false);
            for s in t.sub_task_ids.iter().filter_map(|i| store.state.task.entities.get(i)) {
                self.task_row(s, &store, true, false);
            }
        }
        imp.empty.set_visible(imp.rows.borrow().is_empty());
        imp.task_box.set_visible(!imp.rows.borrow().is_empty());
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
        if matches!(view, View::Today | View::Tonight) {
            task.due_day = Some(today_str());
        }
        if view == View::Tonight {
            task.tag_ids.push(self.ensure_evening_tag());
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
            View::Tonight => {
                let evening = self.ensure_evening_tag();
                if task.due_day.as_deref() == Some(&today) && task.tag_ids.contains(&evening) {
                    return false;
                }
                let mut ids = task.tag_ids.clone();
                if !ids.contains(&evening) {
                    ids.push(evening);
                }
                let mut ch = Map::new();
                ch.insert("tagIds".into(), json!(ids));
                if task.due_day.as_deref() != Some(&today) {
                    ch.insert("dueDay".into(), json!(today));
                    ch.insert("dueWithTime".into(), Value::Null);
                }
                let mut undo = Map::new();
                undo.insert("tagIds".into(), json!(task.tag_ids));
                undo.insert("dueDay".into(), json!(task.due_day));
                self.update_task(&task.id, ch);
                self.toast_undo(
                    &gettext("Planned for tonight"),
                    vec![Action::UpdateTask {
                        id: task.id.clone(),
                        changes: undo,
                    }],
                );
            }
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
            View::Today | View::Tonight => ("TAG", TODAY_TAG_ID.to_string()),
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
        self.move_many_dialog(vec![task_id.to_string()]);
    }

    fn move_many_dialog(&self, ids: Vec<String>) {
        let store = self.imp().store.borrow();
        let tasks: Vec<Task> = ids
            .iter()
            .filter_map(|i| store.state.task.entities.get(i).cloned())
            .collect();
        let Some(task) = tasks.first().cloned() else {
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
            .body(if tasks.len() == 1 {
                task.title.clone()
            } else {
                format!("{} {}", tasks.len(), gettext("tasks"))
            })
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
                            for t in &tasks {
                                w.drop_task(&t.id, &View::Project(pid.clone()));
                            }
                            w.set_selecting(false);
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
        self.edit_context_dialog_for(view);
    }
    fn edit_context_dialog_for(&self, view: View) {
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
        let color_row = adw::ActionRow::builder().title(gettext("Color")).build();
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
        self.delete_context_dialog_for(view);
    }
    fn delete_context_dialog_for(&self, view: View) {
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
                        if *w.imp().view.borrow() == view {
                            *w.imp().view.borrow_mut() = View::Today;
                        }
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
                matches!(*imp.view.borrow(), View::Today | View::Tonight).then(today_str),
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
                } else if view == View::Tonight {
                    let id = w.ensure_evening_tag();
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
