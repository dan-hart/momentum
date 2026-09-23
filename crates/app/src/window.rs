// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! The main window: widgets, rendering and input. Everything the window shows comes from
//! `momentum_core::Engine` (the same core the macOS app drives); everything the user does
//! goes back to it and comes out as an [`Outcome`] that this file turns into a refresh, a
//! toast and an Undo button.

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::{gio, glib};
use momentum_core::{
    AllDone, ClockTime, DayLabel, DayRelation, EmptyState, Engine, EstimateRange, GroupBy, MonthlyRule, Outcome,
    Preferences, RepeatDescription, Row, SectionKind, SectionNote, Slot, SortDirection, SortKey, TaskGroup, TaskRow,
    ViewTitle,
};
use sp_model::{now_ms, INBOX_PROJECT_ID};
use sp_oplog::Action;
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use crate::application::MomentumApplication;
use crate::config::{APP_ID, PROFILE};

pub use momentum_core::View;

#[derive(Clone, Copy)]
pub enum MenuKind {
    Task,
    Project,
    Tag,
}

/// Data directory for windows created from now on (tests give every window its own).
static TEST_DATA_DIR: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);
#[allow(dead_code)] // tests only
pub fn set_test_data_dir(dir: Option<PathBuf>) {
    *TEST_DATA_DIR.lock().unwrap_or_else(|e| e.into_inner()) = dir;
}
fn test_data_dir() -> Option<PathBuf> {
    TEST_DATA_DIR.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

#[cfg(test)]
#[path = "tests/ui.rs"]
mod ui_tests;

mod imp {
    use super::*;

    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/github/dan_hart/Momentum/ui/window.ui")]
    pub struct MomentumWindow {
        #[template_child]
        pub split_view: TemplateChild<adw::OverlaySplitView>,
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
        pub engine: Arc<Engine>,
        pub view: RefCell<View>,
        pub views: RefCell<Vec<Option<View>>>,
        pub rows: RefCell<Vec<String>>,
        pub filter: RefCell<String>,
        pub color_css: RefCell<String>,
        pub tag_popover: gtk::Popover,
        pub tag_list: gtk::ListBox,
        pub tag_matches: RefCell<Vec<String>>,
        pub color_provider: gtk::CssProvider,
        /// A Nextcloud cycle is running (drives the button and label; the engine refuses a second one anyway).
        pub syncing: Cell<bool>,
        pub nearby_syncing: Cell<bool>,
        pub sync_error: RefCell<Option<String>>,
        #[template_child]
        pub sync_retry: TemplateChild<gtk::Button>,
        pub archive_shown: Cell<u32>,
        pub selecting: Cell<bool>,
        pub selected: RefCell<HashSet<String>>,
        #[template_child]
        pub select_cancel: TemplateChild<gtk::Button>,
        #[template_child]
        pub select_bar: TemplateChild<gtk::ActionBar>,
        #[template_child]
        pub select_count: TemplateChild<gtk::Label>,
        pub search_debounce: RefCell<Option<glib::SourceId>>,
        pub sync_debounce: RefCell<Option<glib::SourceId>>,
        pub store_monitor: RefCell<Option<gio::FileMonitor>>,
        pub background_status: Rc<RefCell<crate::background_status::BackgroundStatusController>>,
        pub p2p_dialog: RefCell<Option<Rc<dyn Fn()>>>,
    }

    impl Default for MomentumWindow {
        fn default() -> Self {
            let demo = std::env::var_os("MOMENTUM_DEMO").is_some();
            let dir = if let Some(d) = super::test_data_dir() {
                d
            } else if let Some(d) = std::env::var_os("MOMENTUM_DATA_DIR") {
                PathBuf::from(d)
            } else if demo {
                glib::tmp_dir().join("momentum-demo")
            } else {
                glib::user_data_dir().join("momentum")
            };
            let dir = dir.to_string_lossy().into_owned();
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
                engine: if demo { Engine::demo(dir) } else { Engine::open(dir) },
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
                nearby_syncing: Cell::new(false),
                sync_error: Default::default(),
                sync_retry: Default::default(),
                archive_shown: Cell::new(100),
                selecting: Cell::new(false),
                selected: Default::default(),
                select_cancel: Default::default(),
                select_bar: Default::default(),
                select_count: Default::default(),
                search_debounce: Default::default(),
                sync_debounce: Default::default(),
                store_monitor: Default::default(),
                background_status: Default::default(),
                p2p_dialog: Default::default(),
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
            crate::typography::register_interface_root(&*obj);
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
            self.engine.p2p_stop();
            self.engine.stop_cli_server();
            if self.tag_popover.parent().is_some() {
                self.tag_popover.unparent();
            }
        }
    }
    impl WindowImpl for MomentumWindow {
        fn close_request(&self) -> glib::Propagation {
            if self.settings.boolean("run-in-background") {
                // Keep reminders and sync alive; the window comes back from the launcher,
                // the search provider, the shortcut or `momentum`.
                if let Err(err) = self.obj().save_window_size() {
                    tracing::warn!("Failed to save window state, {}", &err);
                }
                self.obj().set_visible(false);
                return glib::Propagation::Stop;
            }
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

// ---- formatting of core types (the wording the po files carry) ---------------------

/// Milliseconds → "1h 30m" or "45m".
pub fn fmt_ms(ms: f64) -> String {
    momentum_core::text::format_estimate(ms)
}
/// "1h 30m", "45m", "2h" → ms
pub fn parse_ms(s: &str) -> Option<f64> {
    momentum_core::text::parse_estimate(s)
}

fn glib_day(d: &str) -> Option<glib::DateTime> {
    let mut it = d.split('-').map(|x| x.parse::<i32>().ok());
    glib::DateTime::from_local(it.next()??, it.next()??, it.next()??, 0, 0, 0.0).ok()
}
fn fmt_day_as(day: &str, f: &str) -> String {
    glib_day(day)
        .and_then(|d| d.format(f).ok())
        .map(|g| g.to_string())
        .unwrap_or_else(|| day.to_string())
}
/// Relative day label: Today, Tomorrow, Yesterday, a weekday within the week, else a locale date.
pub fn fmt_day(label: &DayLabel) -> String {
    match label.relation {
        DayRelation::Today => gettext("Today"),
        DayRelation::Tomorrow => gettext("Tomorrow"),
        DayRelation::Yesterday => gettext("Yesterday"),
        DayRelation::ThisWeek => fmt_day_as(&label.day, "%A"),
        DayRelation::ThisYear => fmt_day_as(&label.day, "%-d %B"),
        DayRelation::Other => fmt_day_as(&label.day, "%-d %B %Y"),
    }
}

/// A clock time as `14:30`.
pub fn fmt_clock(t: &ClockTime) -> String {
    format!("{:02}:{:02}", t.hour, t.minute)
}
/// "just now", "5 minutes ago", … for a past timestamp in ms.
pub fn ago_text(ms: u64) -> String {
    let secs = now_ms().saturating_sub(ms) / 1000;
    match secs {
        0..=9 => gettext("just now"),
        10..=59 => format!("{secs} {}", gettext("seconds ago")),
        60..=3599 => format!("{} {}", secs / 60, gettext("minutes ago")),
        3600..=86399 => format!("{} {}", secs / 3600, gettext("hours ago")),
        _ => format!("{} {}", secs / 86400, gettext("days ago")),
    }
}

/// CSS colour string → `#rrggbb` for Pango markup.
pub fn hex_color(color: &str) -> Option<String> {
    let c = gtk::gdk::RGBA::parse(color).ok()?;
    Some(format!(
        "#{:02x}{:02x}{:02x}",
        (c.red() * 255.0) as u8,
        (c.green() * 255.0) as u8,
        (c.blue() * 255.0) as u8
    ))
}

fn weekday_names() -> [String; 7] {
    [
        gettext("Sunday"),
        gettext("Monday"),
        gettext("Tuesday"),
        gettext("Wednesday"),
        gettext("Thursday"),
        gettext("Friday"),
        gettext("Saturday"),
    ]
}

/// Short, human description of a repeat schedule: "Repeats every Monday", "Repeats daily", …
pub fn repeat_text(d: &RepeatDescription) -> String {
    let names = weekday_names();
    let name = |wd: u32| names.get(wd as usize).cloned().unwrap_or_default();
    match d {
        RepeatDescription::Daily => gettext("Repeats daily"),
        RepeatDescription::EveryNDays { n } => format!("{} {n} {}", gettext("Repeats every"), gettext("days")),
        RepeatDescription::EveryWeekday { weekday } => format!("{} {}", gettext("Repeats every"), name(*weekday)),
        RepeatDescription::Weekly { n, weekdays } => {
            let list = weekdays
                .iter()
                .map(|d| name(*d).chars().take(3).collect::<String>())
                .collect::<Vec<_>>()
                .join(", ");
            if *n == 1 {
                format!("{} {list}", gettext("Repeats weekly on"))
            } else {
                format!("{} {n} {} {list}", gettext("Repeats every"), gettext("weeks on"))
            }
        }
        RepeatDescription::Monthly { n, rule } => {
            let day = match rule {
                MonthlyRule::NthWeekday { week, weekday } => {
                    let which = match week {
                        1 => gettext("the first"),
                        2 => gettext("the second"),
                        3 => gettext("the third"),
                        4 => gettext("the fourth"),
                        _ => gettext("the last"),
                    };
                    format!("{which} {}", name(*weekday))
                }
                MonthlyRule::LastDay => gettext("the last day"),
                MonthlyRule::DayOfMonth { day } => format!("{} {}", gettext("the"), ordinal(*day)),
                MonthlyRule::SameDay => gettext("the same day"),
            };
            if *n == 1 {
                format!("{} {day}", gettext("Repeats monthly on"))
            } else {
                format!("{} {n} {} {day}", gettext("Repeats every"), gettext("months on"))
            }
        }
        RepeatDescription::Yearly { n, month, day } => {
            // The year does not show; 2000 is a leap year so 29 February stays valid.
            let date = glib::DateTime::from_local(2000, *month as i32, *day as i32, 0, 0, 0.0)
                .ok()
                .and_then(|d| d.format("%-d %B").ok())
                .map(|g| g.to_string())
                .unwrap_or_default();
            if *n == 1 {
                format!("{} {date}", gettext("Repeats yearly on"))
            } else {
                format!("{} {n} {} {date}", gettext("Repeats every"), gettext("years on"))
            }
        }
        RepeatDescription::Repeats => gettext("Repeats"),
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

/// `mo` over the data directory's socket: changes and sync requests arrive on a
/// background thread and are handed to the main loop.
#[cfg(target_os = "linux")]
struct CliBridge(glib::SendWeakRef<MomentumWindow>);
#[cfg(target_os = "linux")]
impl momentum_core::ipc::CliDelegate for CliBridge {
    fn store_changed(&self) {
        let w = self.0.clone();
        glib::idle_add_once(move || {
            if let Some(w) = w.upgrade() {
                w.import_cli_config();
                w.refresh();
            }
        });
    }
    fn sync_requested(&self) {
        let w = self.0.clone();
        glib::idle_add_once(move || {
            if let Some(w) = w.upgrade() {
                w.import_cli_config();
                w.sync();
            }
        });
    }
}

#[cfg(test)]
fn send_background_status_request(
    _controller: Rc<RefCell<crate::background_status::BackgroundStatusController>>,
    _request: crate::background_status::BackgroundStatusRequest,
) {
    // Unit/UI tests exercise the controller through an injected sender and never contact
    // a real desktop portal. Leaving this request in flight also lets refresh-burst tests
    // inspect the newest desired value while preserving the one-request invariant.
}

#[cfg(not(test))]
fn send_background_status_request(
    controller: Rc<RefCell<crate::background_status::BackgroundStatusController>>,
    request: crate::background_status::BackgroundStatusRequest,
) {
    glib::spawn_future_local(async move {
        let options = ashpd::desktop::background::SetStatusOptions::default().set_message(&request.message);
        let result = match ashpd::desktop::background::BackgroundProxy::new().await {
            Ok(proxy) => proxy.set_status(options).await,
            Err(error) => Err(error),
        };
        match result {
            Ok(()) => {
                let retry_controller = controller.clone();
                controller.borrow_mut().succeeded(request.id, &mut |next| {
                    send_background_status_request(retry_controller.clone(), next)
                });
            }
            Err(error) => {
                controller.borrow_mut().failed(request.id);
                tracing::debug!("background status: {error}");
            }
        }
    });
}

impl MomentumWindow {
    pub fn new(app: &MomentumApplication) -> Self {
        glib::Object::builder().property("application", app).build()
    }

    /// The application core this window renders and drives.
    pub fn engine(&self) -> Arc<Engine> {
        self.imp().engine.clone()
    }

    fn setup(&self) {
        let imp = self.imp();
        crate::typography::register_content_root(&*imp.add_entry);
        self.apply_preferences();
        self.import_cli_config();
        self.export_cli_config();
        imp.settings.connect_changed(
            Some("sync-method"),
            glib::clone!(
                #[weak(rename_to = w)]
                self,
                move |_, _| {
                    if let Some(id) = w.imp().sync_debounce.borrow_mut().take() {
                        id.remove();
                    }
                    w.imp().nearby_syncing.set(false);
                    w.set_sync_error(None);
                    w.p2p_apply_setting();
                    w.export_cli_config();
                }
            ),
        );
        glib::idle_add_local_once(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move || {
                w.p2p_apply_setting();
                w.apply_modifier(); // the help overlay exists once the app has adopted the window
            }
        ));
        adw::StyleManager::default().connect_high_contrast_notify(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |_| w.refresh()
        ));
        imp.sidebar_list.connect_row_selected(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |_, row| {
                let Some(row) = row else { return };
                if let Some(Some(v)) = w.imp().views.borrow().get(row.index() as usize) {
                    *w.imp().view.borrow_mut() = v.clone();
                }
                w.imp().archive_shown.set(100);
                if w.imp().split_view.is_collapsed() {
                    w.imp().split_view.set_show_sidebar(false);
                }
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
        self.add_action(&imp.settings.create_action("group-by"));
        self.add_action(&imp.settings.create_action("task-sort"));
        self.add_action(&imp.settings.create_action("sort-direction"));
        self.add_action(&imp.settings.create_action("upcoming-range"));
        // The preferences the core computes with: handed over whenever they change.
        for key in ["group-by", "task-sort", "sort-direction", "upcoming-range"] {
            imp.settings.connect_changed(
                Some(key),
                glib::clone!(
                    #[weak(rename_to = w)]
                    self,
                    move |_, _| {
                        w.apply_preferences();
                        w.refresh_tasks();
                    }
                ),
            );
        }
        for key in [
            "auto-archive",
            "morning-summary-enabled",
            "morning-summary-hour",
            "morning-summary-minute",
        ] {
            imp.settings.connect_changed(
                Some(key),
                glib::clone!(
                    #[weak(rename_to = w)]
                    self,
                    move |_, _| w.apply_preferences()
                ),
            );
        }
        for key in ["background-count-mode", "run-in-background"] {
            imp.settings.connect_changed(
                Some(key),
                glib::clone!(
                    #[weak(rename_to = w)]
                    self,
                    move |_, _| w.update_background_status()
                ),
            );
        }
        self.update_selection_ui();
        imp.banner.connect_button_clicked(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |_| w.show_sync_error()
        ));
        imp.settings.connect_changed(
            Some("colorful-labels"),
            glib::clone!(
                #[weak(rename_to = w)]
                self,
                move |_, _| w.refresh()
            ),
        );
        self.setup_tag_completion();
        // Drop text or a link from another app onto the list to make a task of it.
        let text_drop = gtk::DropTarget::new(String::static_type(), gtk::gdk::DragAction::COPY);
        text_drop.connect_drop(glib::clone!(
            #[weak(rename_to = w)]
            self,
            #[upgrade_or]
            false,
            move |_, value, _, _| {
                let Ok(text) = value.get::<String>() else { return false };
                if w.engine().is_task_id_list(text.clone()) {
                    return false; // our own row drags are handled by the row targets
                }
                w.add_from_text(&text);
                true
            }
        ));
        imp.task_box.add_controller(text_drop);
        imp.add_entry.connect_insert_text(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |e, text, _| {
                if text.contains('\n') {
                    e.stop_signal_emission_by_name("insert-text");
                    let pasted = text.to_string();
                    glib::idle_add_local_once(glib::clone!(
                        #[weak]
                        w,
                        move || w.add_from_text(&pasted)
                    ));
                }
            }
        ));
        // If another process (mo without D-Bus, or a second instance) writes the store, reload it.
        let pending = PathBuf::from(imp.engine.data_dir()).join("pending.json");
        let monitor = gio::File::for_path(pending)
            .monitor_file(gio::FileMonitorFlags::NONE, gio::Cancellable::NONE)
            .ok();
        if let Some(m) = monitor {
            m.connect_changed(glib::clone!(
                #[weak(rename_to = w)]
                self,
                move |_, _, _, event| {
                    if event != gio::FileMonitorEvent::ChangesDoneHint {
                        return;
                    }
                    if w.engine().reload_from_disk() {
                        w.refresh();
                    }
                }
            ));
            *imp.store_monitor.borrow_mut() = Some(m);
        }
        // `mo` on the same machine can also hand actions over the data directory's socket.
        #[cfg(target_os = "linux")]
        {
            let bridge = Arc::new(CliBridge(glib::SendWeakRef::from(self.downgrade())));
            if let Err(e) = imp.engine.clone().serve_cli(bridge) {
                tracing::warn!("cli socket: {e}");
            }
        }
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
                    let out = w.engine().toggle_done(id);
                    w.apply(out);
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
                let out = w.engine().toggle_done(id.into());
                w.apply(out);
            }),
            targeted("ctx-today", |w, id| {
                let out = w.engine().toggle_today(id.into());
                w.apply(out);
            }),
            targeted("ctx-tonight", |w, id| {
                let ids = w.selection_or(id);
                w.toggle_tonight(&ids);
            }),
            targeted("ctx-morning", |w, id| {
                let ids = w.selection_or(id);
                w.toggle_morning(&ids);
            }),
            targeted("ctx-tomorrow", |w, id| {
                let ids = w.selection_or(id);
                w.move_to_tomorrow(&ids);
            }),
            targeted("ctx-next-week", |w, id| {
                let ids = w.selection_or(id);
                w.move_to_next_week(&ids);
            }),
            targeted("ctx-move", |w, id| w.move_to_dialog(id)),
            targeted("ctx-repeat", |w, id| crate::repeat_dialog::open(w, id)),
            targeted("ctx-delete", |w, id| w.delete_task(id)),
            targeted("ctx-open-project", |w, id| w.go_to(View::project(id))),
            targeted("ctx-new-task", |w, id| {
                w.go_to(View::project(id));
                w.new_task_dialog();
            }),
            targeted("ctx-edit-project", |w, id| w.edit_context_dialog_for(View::project(id))),
            targeted("ctx-delete-project", |w, id| {
                w.delete_context_dialog_for(View::project(id))
            }),
            targeted("ctx-open-tag", |w, id| w.go_to(View::tag(id))),
            targeted("ctx-edit-tag", |w, id| w.edit_context_dialog_for(View::tag(id))),
            targeted("ctx-delete-tag", |w, id| w.delete_context_dialog_for(View::tag(id))),
            act("plan-today", |w| {
                if let Some(id) = w.focused_task() {
                    w.drop_task(&id, &View::Today);
                }
            }),
            act("focus-add", |w| {
                w.imp().add_entry.grab_focus();
            }),
            act("toggle-sidebar", |w| {
                let sv = &w.imp().split_view;
                sv.set_show_sidebar(!sv.shows_sidebar());
            }),
            act("select-none", |w| w.set_selecting(false)),
            act("next-view", |w| w.step_view(1)),
            act("prev-view", |w| w.step_view(-1)),
            act("move-up", |w| w.nudge(-1)),
            act("move-down", |w| w.nudge(1)),
            act("duplicate", |w| {
                if let Some(id) = w.focused_task() {
                    w.duplicate_task(&id);
                }
            }),
            act("copy-title", |w| {
                if let Some(id) = w.focused_task() {
                    let title = w.engine().task_title(id).unwrap_or_default();
                    w.clipboard().set_text(&title);
                    w.toast(&gettext("Title copied"));
                }
            }),
            act("open-focused", |w| {
                if let Some(id) = w.focused_task() {
                    w.open_task(&id);
                }
            }),
            act("move-next-week", |w| {
                let ids = w.selected_or_focused();
                w.move_to_next_week(&ids);
            }),
            act("move-tomorrow", |w| {
                let ids = w.selected_or_focused();
                w.move_to_tomorrow(&ids);
            }),
            act("toggle-tonight", |w| {
                let ids = w.selected_or_focused();
                w.toggle_tonight(&ids);
            }),
            act("toggle-morning", |w| {
                let ids = w.selected_or_focused();
                w.toggle_morning(&ids);
            }),
            act("repeat", |w| {
                if let Some(id) = w.focused_task() {
                    crate::repeat_dialog::open(w, &id);
                }
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
            act("undo", |w| w.undo_last()),
            act("edit-context", |w| w.edit_context_dialog()),
            act("delete-context", |w| w.delete_context_dialog()),
        ]);
        imp.settings.connect_changed(
            None,
            glib::clone!(
                #[weak(rename_to = w)]
                self,
                move |_, key| {
                    w.update_sync_button();
                    if ["nextcloud-server", "nextcloud-user", "nextcloud-folder", "compress"].contains(&key) {
                        w.export_cli_config();
                    }
                }
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
                    if w.engine().day_changed() {
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
            if std::env::var_os("MOMENTUM_SCREENSHOT_DONE").is_some() {
                let ids: Vec<String> = imp.engine.with_store(|s| s.state.today_ids());
                for id in ids {
                    imp.engine.dispatch(Action::UpdateTask {
                        id,
                        changes: [("isDone".to_string(), serde_json::json!(true))].into_iter().collect(),
                    });
                }
                self.refresh();
            }
            if std::env::var_os("MOMENTUM_SCREENSHOT_NARROW").is_some() {
                self.set_default_size(360, 720);
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
            if std::env::var_os("MOMENTUM_SCREENSHOT_PREFS").is_some() {
                crate::prefs::MomentumPrefs::default().present(Some(self));
            }
            if std::env::var_os("MOMENTUM_SCREENSHOT_DEVICES").is_some() {
                // The node starts from an idle callback; open the dialog once it is up.
                glib::timeout_add_seconds_local_once(
                    1,
                    glib::clone!(
                        #[weak(rename_to = w)]
                        self,
                        move || w.show_devices_dialog()
                    ),
                );
            }
            let delay = std::env::var("MOMENTUM_SCREENSHOT_DELAY")
                .ok()
                .and_then(|d| d.parse().ok())
                .unwrap_or(2);
            crate::demo::screenshot(self.upcast_ref(), path.into(), delay);
        }
    }

    /// The sort, direction, Coming Up range and auto-archive preferences, as the core needs them.
    fn apply_preferences(&self) {
        let s = &self.imp().settings;
        let sort = match s.string("task-sort").as_str() {
            "title" => SortKey::Title,
            "due" => SortKey::Due,
            "estimate" => SortKey::Estimate,
            "created" => SortKey::Created,
            _ => SortKey::Manual,
        };
        let direction = if s.string("sort-direction") == "descending" {
            SortDirection::Descending
        } else {
            SortDirection::Ascending
        };
        self.engine().set_preferences(Preferences {
            group_by: match s.string("group-by").as_str() {
                "none" => GroupBy::None,
                "project" => GroupBy::Project,
                "tag" => GroupBy::Tag,
                "estimate" => GroupBy::Estimate,
                _ => GroupBy::MorningNight,
            },
            sort,
            direction,
            upcoming_days: s.string("upcoming-range").parse().unwrap_or(7),
            auto_archive: s.boolean("auto-archive"),
            morning_summary_enabled: s.boolean("morning-summary-enabled"),
            morning_summary_time: ClockTime {
                hour: s.int("morning-summary-hour") as u32,
                minute: s.int("morning-summary-minute") as u32,
            },
        });
    }

    /// What a change did: rebuild the views, say so in a toast (with Undo when the change
    /// pushed a batch), and upload right away when the change asks for it.
    pub(crate) fn apply(&self, out: Outcome) {
        if out.changed {
            self.refresh();
        }
        if let Some(m) = &out.message {
            let text = crate::messages::text(m);
            match out.undo {
                Some(batch) => self.toast_undo(&text, batch),
                None => self.toast(&text),
            }
        }
        if out.sync_now && self.sync_configured() {
            self.sync();
        }
    }

    fn sync_configured(&self) -> bool {
        // Demo/screenshot runs must never touch a real server.
        if std::env::var_os("MOMENTUM_DEMO").is_some() {
            return false;
        }
        let s = &self.imp().settings;
        crate::prefs::sync_method(s) == "nextcloud"
            && ["nextcloud-server", "nextcloud-user", "nextcloud-folder"]
                .iter()
                .all(|k| !s.string(k).trim().is_empty())
    }
    pub fn update_sync_button(&self) {
        let imp = self.imp();
        let status = imp.engine.sync_status();
        let method = crate::prefs::sync_method(&imp.settings);
        let demo = std::env::var_os("MOMENTUM_DEMO").is_some();
        let active = !demo && method != "off";
        let busy = imp.syncing.get() || (method == "libresync" && imp.nearby_syncing.get());
        let service = match method.as_str() {
            "nextcloud" => gettext("Nextcloud"),
            "libresync" => gettext("LibreSync"),
            _ => gettext("Sync"),
        };
        let error = imp.sync_error.borrow();
        let text = if demo {
            gettext("Preview mode · Sync is off")
        } else if busy {
            format!(
                "{} · {}",
                if imp.syncing.get() {
                    gettext("Nextcloud")
                } else {
                    service.clone()
                },
                gettext("Syncing…")
            )
        } else if !active {
            gettext("Sync is off")
        } else if error.is_some() {
            format!("{service} · {}", gettext("Sync needs attention"))
        } else if method == "nextcloud" && !self.sync_configured() {
            format!("{service} · {}", gettext("Finish setup in Preferences"))
        } else if method == "libresync" && status.linked_devices == 0 {
            format!("{service} · {}", gettext("No linked devices"))
        } else {
            let last = if method == "nextcloud" {
                status.last_nextcloud_ms
            } else {
                status.last_nearby_ms
            };
            format!(
                "{service} · {}",
                if last == 0 {
                    gettext("Not synced yet")
                } else {
                    format!("{} {}", gettext("Last synced"), ago_text(last))
                }
            )
        };
        imp.sync_label.set_visible(true);
        imp.sync_label.set_text(&text);
        imp.sync_button.set_visible(active || busy);
        imp.sync_button.set_sensitive(active && !busy);
        if busy {
            imp.sync_button.set_child(Some(&adw::Spinner::new()));
        } else {
            imp.sync_button.set_icon_name("view-refresh-symbolic");
        }
        imp.sync_button.set_tooltip_text(Some(&text));
        imp.sync_retry.set_visible(active);
        imp.sync_retry.set_sensitive(!busy);
        imp.sync_retry.set_label(&if error.is_some() {
            gettext("Retry")
        } else {
            gettext("Sync Now")
        });
        imp.banner.set_revealed(active && error.is_some());
        imp.banner
            .set_title(&format!("{service} · {}", gettext("Sync needs attention")));
    }

    pub fn set_sync_error(&self, error: Option<String>) {
        *self.imp().sync_error.borrow_mut() = error;
        self.update_sync_button();
    }

    fn show_sync_error(&self) {
        let Some(error) = self.imp().sync_error.borrow().clone() else {
            return;
        };
        let dialog = adw::AlertDialog::builder()
            .heading(gettext("Sync needs attention"))
            .body(error)
            .build();
        crate::typography::register_interface_root(&dialog);
        dialog.add_responses(&[
            ("close", &gettext("Close")),
            ("preferences", &gettext("Preferences")),
            ("retry", &gettext("Retry")),
        ]);
        dialog.set_close_response("close");
        dialog.connect_response(
            None,
            glib::clone!(
                #[weak(rename_to = w)]
                self,
                move |_, response| {
                    match response {
                        "retry" => w.sync(),
                        "preferences" => crate::prefs::MomentumPrefs::default().present(Some(&w)),
                        _ => {}
                    }
                }
            ),
        );
        dialog.present(Some(self));
    }

    fn import_cli_config(&self) {
        // Import only connection fields, never the UI's selected provider.
        let path = PathBuf::from(self.engine().data_dir()).join("cli-config.json");
        let Ok(bytes) = std::fs::read(path) else { return };
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            return;
        };
        for (field, key) in [
            ("server", "nextcloud-server"),
            ("user", "nextcloud-user"),
            ("folder", "nextcloud-folder"),
        ] {
            if let Some(text) = value[field].as_str() {
                self.imp().settings.set_string(key, text).ok();
            }
        }
        if let Some(value) = value["compress"].as_bool() {
            self.imp().settings.set_boolean("compress", value).ok();
        }
    }

    fn export_cli_config(&self) {
        let settings = &self.imp().settings;
        let value = serde_json::json!({
            "method": if std::env::var_os("MOMENTUM_DEMO").is_some() { "off".into() } else { crate::prefs::sync_method(settings) },
            "server": settings.string("nextcloud-server").as_str(),
            "user": settings.string("nextcloud-user").as_str(),
            "folder": settings.string("nextcloud-folder").as_str(),
            "compress": settings.boolean("compress"),
        });
        let path = PathBuf::from(self.engine().data_dir()).join("cli-config.json");
        let bytes = serde_json::to_vec_pretty(&value).expect("serializable settings");
        if std::fs::read(&path).ok().as_ref() == Some(&bytes) {
            return;
        }
        if let Err(error) = gio::File::for_path(path).replace_contents(
            &bytes,
            None,
            false,
            gio::FileCreateFlags::PRIVATE,
            gio::Cancellable::NONE,
        ) {
            tracing::warn!("Could not save CLI sync preferences: {error}");
            self.toast(&gettext("Could not save sync preferences for the command line."));
        }
    }
    fn focused_task(&self) -> Option<String> {
        let group = self.imp().task_box.focus_child()?;
        let list = group.focus_child()?.downcast::<gtk::ListBox>().ok()?;
        let row = list.focus_child()?.downcast::<gtk::ListBoxRow>().ok()?;
        let id = row.widget_name().to_string();
        (!id.is_empty() && !id.contains(':')).then_some(id)
    }
    /// The selection in selection mode, else the focused task.
    fn selected_or_focused(&self) -> Vec<String> {
        if self.imp().selecting.get() {
            self.selected_ids()
        } else {
            self.focused_task().into_iter().collect()
        }
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

    /// Selected task ids in the visual order of the current list.
    fn selected_ids(&self) -> Vec<String> {
        let imp = self.imp();
        let sel = imp.selected.borrow();
        imp.rows.borrow().iter().filter(|r| sel.contains(*r)).cloned().collect()
    }

    fn selected_tasks(&self) -> Vec<TaskRow> {
        let engine = self.engine();
        self.selected_ids()
            .into_iter()
            .filter_map(|id| engine.task_row(id))
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

    fn toggle_tonight(&self, ids: &[String]) {
        self.toggle_slot(ids, Slot::Tonight);
    }
    fn toggle_morning(&self, ids: &[String]) {
        self.toggle_slot(ids, Slot::Morning);
    }
    /// Move tasks between the plain day and a slot (Morning or Tonight).
    fn toggle_slot(&self, ids: &[String], slot: Slot) {
        let out = self.engine().toggle_slot(ids.to_vec(), slot);
        if out.changed && self.imp().selecting.get() {
            self.set_selecting(false);
        }
        self.apply(out);
    }

    /// Push tasks to tomorrow (keeps tags; clears any time of day).
    fn move_to_tomorrow(&self, ids: &[String]) {
        let out = self.engine().move_to_tomorrow(ids.to_vec());
        self.leave_selection_and_apply(out);
    }

    fn move_to_next_week(&self, ids: &[String]) {
        let out = self.engine().move_to_next_week(ids.to_vec());
        self.leave_selection_and_apply(out);
    }

    /// A change made from selection mode ends it when something happened.
    fn leave_selection_and_apply(&self, out: Outcome) {
        if out.changed && self.imp().selecting.get() {
            self.set_selecting(false);
        }
        self.apply(out);
    }

    fn bulk_done(&self) {
        let out = self.engine().bulk_done(self.selected_ids());
        self.set_selecting(false);
        self.apply(out);
    }

    fn bulk_today(&self) {
        let out = self.engine().plan_for_today(self.selected_ids());
        if !out.changed {
            return;
        }
        self.set_selecting(false);
        self.apply(out);
    }

    fn bulk_delete(&self) {
        let out = self.engine().bulk_delete(self.selected_ids());
        self.set_selecting(false);
        self.apply(out);
    }

    /// Bulk add a tag: dropdown of tags, or type a new one.
    fn bulk_tag_dialog(&self) {
        let ids = self.selected_ids();
        if ids.is_empty() {
            return;
        }
        let tags = self.engine().tags();
        let names: Vec<&str> = tags.iter().map(|t| t.title.as_str()).collect();
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
            .heading(format!("{} {} {}", gettext("Add Tag to"), ids.len(), gettext("Tasks")))
            .extra_child(&content)
            .default_response("add")
            .build();
        crate::typography::register_interface_root(&d);
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
                    let out = if !typed.is_empty() {
                        w.engine().add_tag_by_name(ids.clone(), typed)
                    } else {
                        match tags.get(drop_down.selected() as usize) {
                            Some(t) => w.engine().add_tag_to(ids.clone(), t.id.clone()),
                            None => return,
                        }
                    };
                    w.set_selecting(false);
                    w.apply(out);
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
            self.go_to(View::project(pid));
        } else if let Some(tid) = id.strip_prefix("tag:") {
            self.go_to(View::tag(tid));
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
        let menu = gio::Menu::new();
        let item = |label: String, action: &str| {
            let it = gio::MenuItem::new(Some(&label), None);
            it.set_action_and_target_value(Some(action), Some(&id.to_variant()));
            it
        };
        match kind {
            MenuKind::Task => {
                let Some(t) = self.engine().task_menu(id.to_string()) else {
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
                menu.append_section(None, &a);
                // Scheduling moves in their own section, labelled so the group reads as one idea.
                let m = gio::Menu::new();
                m.append_item(&item(
                    if t.planned_today {
                        gettext("Remove from Today")
                    } else {
                        gettext("Plan for Today")
                    },
                    "win.ctx-today",
                ));
                m.append_item(&item(
                    if t.slot == Some(Slot::Morning) {
                        gettext("Move to Today")
                    } else {
                        gettext("Move to Morning")
                    },
                    "win.ctx-morning",
                ));
                m.append_item(&item(
                    if t.slot == Some(Slot::Tonight) {
                        gettext("Move to Today")
                    } else {
                        gettext("Move to Tonight")
                    },
                    "win.ctx-tonight",
                ));
                m.append_item(&item(gettext("Move to Tomorrow"), "win.ctx-tomorrow"));
                m.append_item(&item(gettext("Move to Next Week"), "win.ctx-next-week"));
                if t.top_level {
                    m.append_item(&item(gettext("Move to Project…"), "win.ctx-move"));
                }
                menu.append_section(None, &m);
                if t.top_level {
                    let r = gio::Menu::new();
                    r.append_item(&item(
                        if t.repeats {
                            gettext("Edit Repeat…")
                        } else {
                            gettext("Repeat…")
                        },
                        "win.ctx-repeat",
                    ));
                    menu.append_section(None, &r);
                }
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
    fn new_section(&self, title: Option<&str>, task_group: Option<&TaskGroup>) -> gtk::ListBox {
        let imp = self.imp();
        let group = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(6)
            .build();
        if let Some(t) = title {
            let label = gtk::Label::builder()
                .label(t)
                .xalign(0.0)
                .wrap(true)
                .margin_start(6)
                .css_classes(["heading"])
                .build();
            let colored_heading = match task_group {
                Some(TaskGroup::Tag { title, color, .. }) => Some((format!("#{title}"), color)),
                Some(TaskGroup::Project { title, color, .. }) => Some((title.clone(), color)),
                _ => None,
            };
            if let Some((name, color)) = colored_heading {
                if let Some(hex) = color.as_deref().filter(|_| self.colorful()).and_then(hex_color) {
                    let context = t.strip_suffix(&name).unwrap_or("");
                    label.set_markup(&format!(
                        "{}<span foreground=\"{hex}\">{}</span>",
                        glib::markup_escape_text(context),
                        glib::markup_escape_text(&name)
                    ));
                }
            }
            group.append(&label);
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
            None => self.new_section(None, None),
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

    fn update_tag_completion(&self) {
        let imp = self.imp();
        let entry: &gtk::Entry = &imp.add_entry;
        let Some(word) = momentum_core::text::hash_word_at(&entry.text(), entry.position().max(0) as u32) else {
            imp.tag_popover.popdown();
            return;
        };
        let colorful = self.colorful();
        let matches = self.engine().tag_completions(word.prefix);
        imp.tag_list.remove_all();
        for m in &matches {
            // Same look as the sidebar: coloured tag icon, name, and a dim task count.
            let icon = gtk::Image::from_icon_name("tag-symbolic");
            if let Some(class) = m
                .color
                .as_deref()
                .filter(|_| colorful)
                .and_then(|c| self.color_class(c))
            {
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
            row.append(&gtk::Label::builder().label(&m.title).xalign(0.0).hexpand(true).build());
            row.append(
                &gtk::Label::builder()
                    .label(m.task_count.to_string())
                    .css_classes(["dim-label", "caption"])
                    .build(),
            );
            imp.tag_list.append(&row);
        }
        *imp.tag_matches.borrow_mut() = matches.into_iter().map(|m| m.title).collect();
        if imp.tag_matches.borrow().is_empty() {
            imp.tag_popover.popdown();
        } else {
            imp.tag_list.select_row(imp.tag_list.row_at_index(0).as_ref());
            // Anchor under the text cursor rather than centred below the whole entry.
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
        let entry: &gtk::Entry = &imp.add_entry;
        let Some(done) = momentum_core::text::complete_hash_word(&entry.text(), entry.position().max(0) as u32, &name)
        else {
            return;
        };
        imp.tag_popover.popdown();
        entry.set_text(&done.text);
        entry.set_position(done.cursor as i32);
    }

    /// Toast with an Undo button that reverts the engine's batch.
    fn toast_undo(&self, msg: &str, batch: u64) {
        let toast = adw::Toast::builder().title(msg).button_label(gettext("Undo")).build();
        toast.connect_button_clicked(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |_| {
                if w.engine().undo_batch(batch).changed {
                    w.refresh();
                }
            }
        ));
        self.imp().toast_overlay.add_toast(toast);
    }

    /// Add via short syntax and plan for today (quick-add, `--add`, search provider).
    pub fn add_task_for_today(&self, text: &str) {
        let out = self.engine().add_task_for_today(text.into());
        self.apply(out);
    }

    /// URL scheme: title with short syntax, optional notes and due day.
    pub fn add_task_with_notes(&self, text: &str, notes: Option<&str>, due: Option<&str>) {
        let out = self
            .engine()
            .add_task_with_notes(text.into(), notes.map(str::to_string), due.map(str::to_string));
        self.apply(out);
    }

    pub fn complete_by_title(&self, title: &str) {
        let out = self.engine().complete_by_title(title.into());
        self.apply(out);
    }

    pub fn complete_task(&self, id: &str) {
        if let Some(app) = self.application() {
            app.withdraw_notification(id);
        }
        self.set_done(id, true);
    }

    /// Push a reminder forward by `minutes` and let it fire again.
    pub fn snooze_task(&self, id: &str, minutes: u32) {
        if let Some(app) = self.application() {
            app.withdraw_notification(id);
        }
        let out = self.engine().snooze(id.into(), minutes);
        self.apply(out);
    }

    pub fn set_search_query(&self, q: &str) {
        self.imp().search_entry.set_text(q);
        self.imp().search_entry.grab_focus();
    }

    /// Ctrl+Z: undo the most recent undoable change, even after its toast is gone.
    fn undo_last(&self) {
        let out = self.engine().undo();
        self.apply(out);
    }

    /// Text from a drop or a multi-line paste: a URL or a paragraph becomes one task with the
    /// text in its notes; several short lines become several tasks.
    pub fn add_from_text(&self, text: &str) {
        let view = self.imp().view.borrow().clone();
        let out = self.engine().add_from_text(text.into(), view);
        self.apply(out);
    }

    /// An action handed over by `mo` (the D-Bus "cli" action): `"sync"` or an action as JSON.
    pub fn dispatch_json(&self, payload: &str) {
        if payload == "\"sync\"" {
            self.import_cli_config();
            self.sync();
            return;
        }
        match self.engine().dispatch_json(payload.into()) {
            Ok(out) => self.apply(out),
            Err(e) => tracing::warn!("cli: unrecognised payload: {e}"),
        }
    }

    pub fn toast(&self, msg: &str) {
        self.imp().toast_overlay.add_toast(adw::Toast::new(msg));
    }
    /// A raw action (tests, screenshot setups): no undo, no toast.
    pub fn dispatch(&self, a: Action) {
        self.engine().dispatch(a);
        self.refresh();
    }
    pub fn focus_add(&self) {
        if self.imp().split_view.is_collapsed() {
            self.imp().split_view.set_show_sidebar(false);
        }
        self.new_task_dialog();
    }

    /// Rewrites the Keyboard Shortcuts overlay for the configured modifier and refreshes
    /// hints that name it. The overlay keeps its original accelerator in the widget name.
    #[allow(deprecated)] // GtkShortcutsWindow is what gtk/help-overlay.ui still builds
    pub fn apply_modifier(&self) {
        #[allow(deprecated)]
        fn walk(w: &gtk::Widget, token: &str) {
            if let Some(sc) = w.downcast_ref::<gtk::ShortcutsShortcut>() {
                // GTK reports the type name when no widget name is set, so tag ours.
                let original = match sc.widget_name().strip_prefix("accel:") {
                    Some(o) => o.to_string(),
                    None => {
                        let current = sc.accelerator().map(|a| a.to_string()).unwrap_or_default();
                        sc.set_widget_name(&format!("accel:{current}"));
                        current
                    }
                };
                // Global (portal) shortcuts stay Ctrl+Alt: they are not GTK accelerators.
                if !original.contains("<Alt>") {
                    sc.set_accelerator(Some(&original.replace("<Control>", token)));
                }
            }
            let mut child = w.first_child();
            while let Some(c) = child {
                walk(&c, token);
                child = c.next_sibling();
            }
        }
        if let Some(overlay) = self.help_overlay() {
            walk(
                overlay.upcast_ref(),
                crate::modifier::token(&crate::modifier::current()),
            );
        }
        self.refresh_tasks();
    }

    // ---- rendering -------------------------------------------------------

    fn archive_done(&self) {
        let out = self.engine().archive_done();
        self.apply(out);
    }

    fn update_context_actions(&self) {
        let view = self.imp().view.borrow().clone();
        let (editable, deletable) = match &view {
            View::Project { id } => (true, id != INBOX_PROJECT_ID),
            View::Tag { .. } => (true, true),
            _ => (false, false),
        };
        for (name, on) in [("edit-context", editable), ("delete-context", deletable)] {
            if let Some(a) = self.lookup_action(name).and_downcast::<gio::SimpleAction>() {
                a.set_enabled(on);
            }
        }
    }

    /// Sync shortly after local changes settle: 20 s after the last change, so a burst of
    /// check-offs becomes one upload, while other devices still see it within half a minute.
    fn schedule_sync(&self) {
        let imp = self.imp();
        if !imp.settings.boolean("auto-sync") || !self.sync_configured() {
            return;
        }
        if let Some(id) = imp.sync_debounce.borrow_mut().take() {
            id.remove();
        }
        let id = glib::timeout_add_seconds_local_once(
            20,
            glib::clone!(
                #[weak(rename_to = w)]
                self,
                move || {
                    w.imp().sync_debounce.borrow_mut().take();
                    if !w.imp().settings.boolean("auto-sync") || !w.sync_configured() {
                        return;
                    }
                    if w.imp().syncing.get() {
                        w.schedule_sync(); // a sync is running; try again after it
                    } else if w.engine().pending_count() > 0 {
                        w.sync();
                    }
                }
            ),
        );
        *imp.sync_debounce.borrow_mut() = Some(id);
    }

    pub fn refresh(&self) {
        self.update_background_status();
        if self.engine().pending_count() > 0 {
            self.schedule_sync();
        }
        // Menu item stays visible but disabled when there is nothing to archive (HIG).
        if let Some(a) = self.lookup_action("archive-done").and_downcast::<gio::SimpleAction>() {
            a.set_enabled(self.engine().can_archive());
        }
        self.refresh_sidebar();
        self.refresh_tasks();
    }

    /// CSS class that colours symbolic icons with a project/tag colour from the sync data.
    /// Colour-coded labels are a preference, and never shown in high contrast: the tag and
    /// project colours come from the sync data, so their contrast cannot be guaranteed.
    pub fn colorful(&self) -> bool {
        self.imp().settings.boolean("colorful-labels") && !adw::StyleManager::default().is_high_contrast()
    }
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
                View::Project { id } => self.attach_context_menu(&r, MenuKind::Project, id),
                View::Tag { id } => self.attach_context_menu(&r, MenuKind::Tag, id),
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
                    // A selected row drags the whole selection, one id per line.
                    let ids: Vec<String> = task_id.split('\n').map(str::to_string).collect();
                    let out = w.engine().drop_tasks(ids, dest.clone());
                    let changed = out.changed;
                    w.apply(out);
                    changed
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
        let sidebar = imp.engine.sidebar();
        // Built-in views are named here; Morning and Tonight are listed only while today uses them.
        for entry in &sidebar.fixed {
            let (name, icon) = match entry.view {
                View::Today => (gettext("Today"), "starred-symbolic"),
                View::Morning => (gettext("Morning"), "weather-clear-symbolic"),
                View::Tonight => (gettext("Tonight"), "weather-clear-night-symbolic"),
                View::Upcoming => (gettext("Coming Up"), "x-office-calendar-symbolic"),
                View::Archive => (gettext("Archive"), "archive-symbolic"),
                View::Search => (gettext("Search"), "edit-find-symbolic"),
                View::Project { .. } | View::Tag { .. } => continue,
            };
            self.sidebar_row(&name, icon, Some(entry.view.clone()), None, None);
        }
        self.sidebar_row(&gettext("Projects"), "", None, None, Some("projects-collapsed"));
        let colorful = self.colorful();
        if !imp.settings.boolean("projects-collapsed") {
            for p in &sidebar.projects {
                self.sidebar_row(
                    &p.title,
                    "folder-symbolic",
                    Some(p.view.clone()),
                    p.color.as_deref().filter(|_| colorful),
                    None,
                );
            }
        }
        self.sidebar_row(&gettext("Tags"), "", None, None, Some("tags-collapsed"));
        if !imp.settings.boolean("tags-collapsed") {
            for t in &sidebar.tags {
                self.sidebar_row(
                    &t.title,
                    "tag-symbolic",
                    Some(t.view.clone()),
                    t.color.as_deref().filter(|_| colorful),
                    None,
                );
            }
        }
        let idx = imp
            .views
            .borrow()
            .iter()
            .position(|v| v.as_ref() == Some(&current))
            .or_else(|| {
                if matches!(current, View::Morning | View::Tonight) {
                    // The slot emptied and its entry is gone: fall back to Today (the task
                    // list is rebuilt right after the sidebar).
                    *imp.view.borrow_mut() = View::Today;
                    return Some(0);
                }
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
            imp.select_cancel.set_visible(false);
            imp.select_bar.set_revealed(false);
        }
        imp.archive_shown.set(100);
        *imp.view.borrow_mut() = view;
        self.refresh();
    }

    fn section_note(&self, note: &SectionNote) {
        let text = if note.suggest_narrowing {
            format!(
                "{} {} {} {}. {}",
                gettext("Showing"),
                note.shown,
                gettext("of"),
                note.total,
                gettext("Add another word to narrow it down.")
            )
        } else {
            format!(
                "{} {} {} {}.",
                gettext("Showing"),
                note.shown,
                gettext("of"),
                note.total
            )
        };
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

    /// Heading of a section, or none for the single untitled list of a plain view.
    fn section_title(&self, kind: &SectionKind, count: u32) -> Option<String> {
        Some(match kind {
            SectionKind::Plain => return None,
            SectionKind::Overdue => format!("{} ({count})", gettext("Overdue")),
            SectionKind::Morning => gettext("Morning"),
            SectionKind::Today => gettext("Today"),
            SectionKind::Tonight => gettext("Tonight"),
            SectionKind::Day { label } => fmt_day(label),
            SectionKind::Completed => format!("{} ({count})", gettext("Completed")),
            SectionKind::SearchTasks => format!("{} ({count})", gettext("Tasks")),
            SectionKind::SearchProjects => gettext("Projects"),
            SectionKind::SearchTags => gettext("Tags"),
            SectionKind::SearchArchived => format!("{} ({count})", gettext("Archived")),
        })
    }

    fn group_title(group: &TaskGroup) -> String {
        match group {
            TaskGroup::Today => gettext("Today"),
            TaskGroup::Morning => gettext("Morning"),
            TaskGroup::Evening => gettext("Evening"),
            TaskGroup::Project { title, .. } => title.clone(),
            TaskGroup::Tag { title, .. } => format!("#{title}"),
            TaskGroup::NoProject => gettext("No Project"),
            TaskGroup::Untagged => gettext("Untagged"),
            TaskGroup::Estimate { range } => match range {
                EstimateRange::UpTo15Minutes => gettext("Up to 15 min"),
                EstimateRange::UpTo30Minutes => gettext("16–30 min"),
                EstimateRange::UpTo60Minutes => gettext("31–60 min"),
                EstimateRange::UpTo2Hours => gettext("1–2 hours"),
                EstimateRange::Over2Hours => gettext("Over 2 hours"),
                EstimateRange::NoEstimate => gettext("No estimate"),
            },
        }
    }

    /// A project or tag hit in search results: a row that opens that view.
    fn link_row(&self, title: &str, icon: &str, color: Option<&str>, id: &str) {
        let row = adw::ActionRow::builder()
            .title(glib::markup_escape_text(title))
            .activatable(true)
            .build();
        let image = gtk::Image::from_icon_name(icon);
        if let Some(c) = color.filter(|_| self.colorful()).and_then(|c| self.color_class(c)) {
            image.add_css_class(&c);
        }
        row.add_prefix(&image);
        row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
        self.append_row(&row, id);
    }

    /// What the current view is when it has nothing to show.
    fn empty_state_kind(&self) -> EmptyState {
        let engine = self.engine();
        match &*self.imp().view.borrow() {
            View::Today => EmptyState::Today,
            View::Morning => EmptyState::Morning,
            View::Tonight => EmptyState::Tonight,
            View::Upcoming => EmptyState::Upcoming {
                days: engine.preferences().upcoming_days,
            },
            View::Archive => EmptyState::Archive,
            View::Search => EmptyState::Search,
            View::Project { id } => EmptyState::Project {
                name: engine.project(id.clone()).map(|p| p.title).unwrap_or_default(),
            },
            View::Tag { id } => EmptyState::Tag {
                name: engine.tag(id.clone()).map(|t| t.title).unwrap_or_default(),
            },
        }
    }

    /// Icon, title and description for the current view's empty state.
    pub fn empty_state(&self) -> (String, String, String) {
        self.empty_state_text(&self.empty_state_kind())
    }

    /// Icon, title and description for an empty view, so the empty state says what to do next.
    fn empty_state_text(&self, state: &EmptyState) -> (String, String, String) {
        let synced = self.sync_configured();
        let add = if synced {
            gettext("Add a task above, or press {}.").replace("{}", &crate::modifier::hint("N"))
        } else {
            gettext("Add a task above, press {}, or turn on sync in Preferences to bring in your tasks.")
                .replace("{}", &crate::modifier::hint("N"))
        };
        match state {
            EmptyState::Today => (
                "starred-symbolic".into(),
                gettext("Nothing planned for today"),
                format!(
                    "{add} {}",
                    gettext("Drag tasks here from Coming Up, or press {} on any task.")
                        .replace("{}", &crate::modifier::hint("T"))
                ),
            ),
            EmptyState::Morning => (
                "weather-clear-symbolic".into(),
                gettext("Nothing planned for the morning"),
                gettext("Tag a task “Morning”, or press {} on a task to move it here.")
                    .replace("{}", &crate::modifier::hint("Shift+M")),
            ),
            EmptyState::Tonight => (
                "weather-clear-night-symbolic".into(),
                gettext("Nothing planned for tonight"),
                gettext("Tag a task “Evening”, or press {} on a task to move it here.")
                    .replace("{}", &crate::modifier::hint("Shift+T")),
            ),
            EmptyState::Upcoming { days } => (
                "x-office-calendar-symbolic".into(),
                gettext("Nothing coming up"),
                format!(
                    "{} {days} {}",
                    gettext("Tasks due in the next"),
                    gettext("days appear here. Set a due day in a task's details.")
                ),
            ),
            EmptyState::Archive => (
                "archive-symbolic".into(),
                gettext("No archived tasks"),
                gettext("Completed tasks land here when you archive them with {}.")
                    .replace("{}", &crate::modifier::hint("E")),
            ),
            EmptyState::Search => (
                "edit-find-symbolic".into(),
                gettext("Search Everything"),
                gettext("Tasks, notes, subtasks, projects, tags and the archive"),
            ),
            EmptyState::NoResults => (
                "edit-find-symbolic".into(),
                gettext("No Results Found"),
                gettext("Try a different search"),
            ),
            EmptyState::Project { name } => (
                "folder-symbolic".into(),
                format!("{} {name}", gettext("No tasks in")),
                format!("{add} {}", gettext("Drag tasks here from any other view.")),
            ),
            EmptyState::Tag { name } => (
                "tag-symbolic".into(),
                format!("{} #{name}", gettext("No tasks tagged")),
                format!("{add} {}", gettext("Drag tasks here to tag them.")),
            ),
        }
    }

    fn task_row(&self, t: &TaskRow) {
        let imp = self.imp();
        let colorful = self.colorful();
        let indent = t.is_subtask && !t.archived;
        let mut sub = vec![];
        // Outside a project view, lead with the project name in the project's own colour.
        if let Some(p) = &t.project {
            let hex = p.color.as_deref().filter(|_| colorful).and_then(hex_color);
            let title = glib::markup_escape_text(&p.title);
            sub.push(match hex {
                Some(h) => format!("<span foreground=\"{h}\">●</span> {title}"),
                None => format!("● {title}"),
            });
        }
        if t.estimate_ms > 0.0 {
            sub.push(format!("~{}", fmt_ms(t.estimate_ms)));
        }
        let repeat = t.repeat.as_ref().map(repeat_text);
        if let Some(text) = &repeat {
            sub.push(glib::markup_escape_text(text).to_string());
        }
        if let Some(done) = &t.done_day {
            sub.push(glib::markup_escape_text(&format!("{} {}", gettext("Done"), fmt_day(done))).to_string());
        }
        // Day views already say which day it is, except for overdue tasks, whose day is the point.
        let when = [t.day.as_ref().map(fmt_day), t.time.as_ref().map(fmt_clock)]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" ");
        if !when.is_empty() {
            sub.push(glib::markup_escape_text(&when).to_string());
        }
        for g in &t.tags {
            let name = glib::markup_escape_text(&g.title);
            sub.push(match g.color.as_deref().filter(|_| colorful).and_then(hex_color) {
                Some(h) => format!("<span foreground=\"{h}\">#{name}</span>"),
                None => format!("#{name}"),
            });
        }
        let row = adw::ActionRow::builder()
            .title(glib::markup_escape_text(&t.title))
            .subtitle(sub.join("  ·  "))
            .activatable(!t.archived)
            .build();
        crate::typography::register_content_root(&row);
        if indent {
            row.set_margin_start(32);
        }
        if t.is_done {
            row.add_css_class("dim-label");
        }
        let archived = t.archived;
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
        check.update_property(&[gtk::accessible::Property::Label(&format!(
            "{}: {}",
            if selecting { gettext("Select") } else { gettext("Done") },
            t.title
        ))]);
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
        if let Some(preview) = &t.notes_preview {
            // Notes badge: the first line as tooltip so a hover shows what is there.
            let icon = gtk::Image::builder()
                .icon_name("text-x-generic-symbolic")
                .tooltip_text(preview)
                .css_classes(["dim-label"])
                .build();
            icon.update_property(&[gtk::accessible::Property::Label(&gettext("Has notes"))]);
            row.add_suffix(&icon);
        }
        if let Some(at) = &t.reminder {
            let text = format!("{} {}", gettext("Reminder at"), fmt_clock(at));
            let icon = gtk::Image::builder()
                .icon_name("alarm-symbolic")
                .tooltip_text(&text)
                .css_classes(["dim-label"])
                .build();
            icon.update_property(&[gtk::accessible::Property::Label(&text)]);
            row.add_suffix(&icon);
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
                    w.reorder(&moved, &before)
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
        let view = imp.view.borrow().clone();
        let searching = view == View::Search;
        imp.add_clamp.set_visible(!searching && !imp.selecting.get());
        imp.search_clamp.set_visible(searching);
        let listing = if searching {
            imp.engine.search(imp.filter.borrow().clone())
        } else {
            imp.engine.listing(view.clone(), imp.archive_shown.get())
        };
        let title = match &listing.title {
            ViewTitle::Today => gettext("Today"),
            ViewTitle::Morning => gettext("Morning"),
            ViewTitle::Tonight => gettext("Tonight"),
            ViewTitle::ComingUp => gettext("Coming Up"),
            ViewTitle::Archive => gettext("Archive"),
            ViewTitle::Search => gettext("Search"),
            ViewTitle::Named { name } => name.clone(),
        };
        imp.content_page.set_title(&title);
        if imp.selecting.get() && !searching {
            self.update_selection_ui();
        }
        let empty = listing.empty.clone().unwrap_or_else(|| self.empty_state_kind());
        let (icon, title, desc) = self.empty_state_text(&empty);
        imp.empty.set_icon_name(Some(&icon));
        imp.empty.set_title(&title);
        imp.empty.set_description(Some(&desc));
        // Everything is done: celebrate above the Completed section and offer to archive.
        if let Some(all_done) = &listing.all_done {
            let (title, desc) = match all_done {
                AllDone::Morning => (gettext("Morning done"), gettext("The rest of the day is yours.")),
                AllDone::Tonight => (
                    gettext("All done for tonight"),
                    gettext("Enjoy the rest of your evening."),
                ),
                AllDone::Today { completed } => (
                    gettext("All done for today"),
                    format!(
                        "{} {} {}",
                        gettext("You completed"),
                        completed,
                        gettext("tasks. Time to switch off.")
                    ),
                ),
                AllDone::Context => (gettext("All caught up"), gettext("Every task here is complete.")),
            };
            // A hand-built panel: AdwStatusPage collapses inside a vertical box.
            let page = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .spacing(6)
                .margin_top(24)
                .margin_bottom(12)
                .build();
            page.append(
                &gtk::Image::builder()
                    .icon_name("object-select-symbolic")
                    .pixel_size(64)
                    .css_classes(["dim-label"])
                    .margin_bottom(6)
                    .build(),
            );
            page.append(&gtk::Label::builder().label(title).css_classes(["title-2"]).build());
            page.append(
                &gtk::Label::builder()
                    .label(desc)
                    .wrap(true)
                    .justify(gtk::Justification::Center)
                    .css_classes(["dim-label"])
                    .build(),
            );
            let archive = gtk::Button::builder()
                .label(gettext("Archive Completed"))
                .action_name("win.archive-done")
                .halign(gtk::Align::Center)
                .margin_top(12)
                .css_classes(["suggested-action", "pill"])
                .build();
            page.append(&archive);
            imp.task_box.append(&page);
            imp.rows.borrow_mut().push(String::new());
        }
        for section in &listing.sections {
            let base = self.section_title(&section.kind, section.count);
            let heading = match &section.group {
                Some(group) => {
                    let title = Self::group_title(group);
                    Some(base.map(|base| format!("{base} · {title}")).unwrap_or(title))
                }
                None => base,
            };
            self.new_section(heading.as_deref(), section.group.as_ref());
            for row in &section.rows {
                match row {
                    Row::Task { row } => self.task_row(row),
                    Row::Project { item } => self.link_row(
                        &item.title,
                        "folder-symbolic",
                        item.color.as_deref(),
                        &format!("project:{}", item.id),
                    ),
                    Row::Tag { item } => self.link_row(
                        &item.title,
                        "tag-symbolic",
                        item.color.as_deref(),
                        &format!("tag:{}", item.id),
                    ),
                }
            }
            if let Some(note) = &section.note {
                self.section_note(note);
            }
        }
        if listing.more_available > 0 {
            // The archive renders in pages: it can hold thousands of tasks and every row is a widget.
            let more = gtk::Button::builder()
                .label(format!(
                    "{} ({} {})",
                    gettext("Show More"),
                    listing.more_available,
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
        let none = imp.rows.borrow().is_empty();
        imp.empty.set_visible(none);
        imp.task_box.set_visible(!none);
        self.update_sync_button();
    }

    // ---- actions ---------------------------------------------------------

    /// Short syntax: `#tag` adds/creates tags, a trailing `1h 30m` sets the estimate.
    pub fn add_task(&self, text: &str) {
        let view = self.imp().view.borrow().clone();
        let out = self.engine().add_task(text.into(), view);
        self.apply(out);
    }

    pub fn add_project(&self, title: &str) {
        if self.engine().add_project(title.into()).is_some() {
            self.refresh();
        }
    }

    /// A task dropped on a sidebar row: move to project, add tag, or plan for today.
    fn drop_task(&self, task_id: &str, dest: &View) -> bool {
        let out = self.engine().drop_tasks(vec![task_id.to_string()], dest.clone());
        let changed = out.changed;
        self.apply(out);
        changed
    }

    fn set_done(&self, id: &str, done: bool) {
        let out = self.engine().set_done(id.into(), done);
        self.apply(out);
    }

    /// HIG: destructive actions get an undo toast rather than a confirmation dialog.
    fn delete_task(&self, id: &str) {
        let out = self.engine().delete_task(id.into());
        self.apply(out);
    }

    /// Ctrl+PageDown / Ctrl+PageUp: step through the sidebar entries.
    fn step_view(&self, delta: i32) {
        let imp = self.imp();
        let views: Vec<usize> = imp
            .views
            .borrow()
            .iter()
            .enumerate()
            .filter(|(_, v)| v.is_some())
            .map(|(i, _)| i)
            .collect();
        if views.is_empty() {
            return;
        }
        let current = imp
            .views
            .borrow()
            .iter()
            .position(|v| v.as_ref() == Some(&*imp.view.borrow()));
        let pos = current.and_then(|c| views.iter().position(|&i| i == c)).unwrap_or(0) as i32;
        let next = (pos + delta).rem_euclid(views.len() as i32) as usize;
        imp.sidebar_list
            .select_row(imp.sidebar_list.row_at_index(views[next] as i32).as_ref());
    }

    /// Ctrl+Up / Ctrl+Down: move the focused task one place in Manual Order.
    fn nudge(&self, delta: i32) {
        let Some(id) = self.focused_task() else { return };
        let view = self.imp().view.borrow().clone();
        let out = self.engine().nudge(id.clone(), delta, view);
        let changed = out.changed;
        self.apply(out);
        if changed {
            self.focus_task(&id);
        }
    }

    /// Put keyboard focus back on a task row after the list was rebuilt.
    fn focus_task(&self, id: &str) {
        let mut group = self.imp().task_box.first_child();
        while let Some(g) = group {
            let mut list_child = g.first_child();
            while let Some(c) = list_child {
                if let Some(list) = c.downcast_ref::<gtk::ListBox>() {
                    let mut row = list.first_child();
                    while let Some(r) = row {
                        if r.widget_name() == id {
                            r.grab_focus();
                            return;
                        }
                        row = r.next_sibling();
                    }
                }
                list_child = c.next_sibling();
            }
            group = g.next_sibling();
        }
    }

    /// Ctrl+Shift+D: a fresh copy of the task (same project, tags, estimate, notes, due day).
    fn duplicate_task(&self, id: &str) {
        let out = self.engine().duplicate_task(id.into());
        let changed = out.changed;
        self.apply(out);
        if changed {
            if let Some(new_id) = self.engine().last_added_id() {
                self.focus_task(&new_id);
            }
        }
    }

    /// Drag reorder: put `moved` before `before` in the current context list.
    fn reorder(&self, moved: &str, before: &str) -> bool {
        let view = self.imp().view.borrow().clone();
        let out = self
            .engine()
            .reorder_tasks(moved.split('\n').map(str::to_string).collect(), before.into(), view);
        let changed = out.changed;
        self.apply(out);
        changed
    }

    /// Ctrl+M: pick a project for the focused task.
    fn move_to_dialog(&self, task_id: &str) {
        self.move_many_dialog(vec![task_id.to_string()]);
    }

    fn move_many_dialog(&self, ids: Vec<String>) {
        let engine = self.engine();
        let tasks: Vec<TaskRow> = ids.iter().filter_map(|i| engine.task_row(i.clone())).collect();
        let Some(task) = tasks.first().cloned() else {
            return;
        };
        if task.parent_id.is_some() {
            self.toast(&gettext("Subtasks move with their parent task"));
            return;
        }
        let projects = engine.projects();
        let names: Vec<&str> = projects.iter().map(|p| p.title.as_str()).collect();
        let drop_down = gtk::DropDown::from_strings(&names);
        let current = task.project.as_ref().map(|p| p.id.as_str()).unwrap_or("");
        drop_down.set_selected(projects.iter().position(|p| p.id == current).unwrap_or(0) as u32);
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
        crate::typography::register_interface_root(&d);
        d.add_responses(&[("cancel", &gettext("Cancel")), ("move", &gettext("Move"))]);
        d.set_response_appearance("move", adw::ResponseAppearance::Suggested);
        let ids: Vec<String> = tasks.iter().map(|t| t.id.clone()).collect();
        d.connect_response(
            None,
            glib::clone!(
                #[weak(rename_to = w)]
                self,
                #[weak]
                drop_down,
                move |_, r| {
                    if r == "move" {
                        if let Some(p) = projects.get(drop_down.selected() as usize) {
                            let out = w.engine().move_to_project(ids.clone(), p.id.clone());
                            w.set_selecting(false);
                            w.apply(out);
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
        let engine = self.engine();
        let (heading, title, color) = match &view {
            View::Project { id } => {
                let Some(p) = engine.project(id.clone()) else {
                    return;
                };
                (gettext("Edit Project"), p.title, p.color)
            }
            View::Tag { id } => {
                let Some(t) = engine.tag(id.clone()) else {
                    return;
                };
                (gettext("Edit Tag"), t.title, t.color)
            }
            _ => return,
        };
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
        crate::typography::register_interface_root(&d);
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
                    let out = match &view {
                        View::Project { id } => w.engine().update_project(id.clone(), new_title, Some(hex)),
                        View::Tag { id } => w.engine().update_tag(id.clone(), new_title, Some(hex)),
                        _ => return,
                    };
                    w.apply(out);
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
        let engine = self.engine();
        let (heading, body) = match &view {
            View::Project { id } if id != INBOX_PROJECT_ID => {
                let Some(p) = engine.project(id.clone()) else {
                    return;
                };
                (
                    format!("{} “{}”?", gettext("Delete"), p.title),
                    format!(
                        "{} {}",
                        engine.project_task_count(id.clone()),
                        gettext("tasks in this project will be deleted. This cannot be undone.")
                    ),
                )
            }
            View::Tag { id } => {
                let Some(t) = engine.tag(id.clone()) else {
                    return;
                };
                (
                    format!("{} “{}”?", gettext("Delete"), t.title),
                    gettext("Tasks keep their other tags."),
                )
            }
            _ => return,
        };
        let d = adw::AlertDialog::builder()
            .heading(heading)
            .body(body)
            .default_response("cancel")
            .build();
        crate::typography::register_interface_root(&d);
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
                        let out = match &view {
                            View::Project { id } => w.engine().delete_project(id.clone()),
                            View::Tag { id } => w.engine().delete_tag(id.clone()),
                            _ => return,
                        };
                        w.apply(out);
                    }
                }
            ),
        );
        d.present(Some(self));
    }

    /// Create today's instances of repeating tasks, like upstream's TaskRepeatCfgService.
    pub fn spawn_repeats(&self) {
        self.engine().spawn_repeats();
    }

    fn check_reminders(&self) {
        for r in self.engine().due_reminders() {
            let n = gio::Notification::new(&r.title);
            n.set_body(Some(&match &r.time {
                Some(t) => format!("{} {}", gettext("Due at"), fmt_clock(t)),
                None => gettext("Reminder"),
            }));
            n.set_default_action_and_target_value("app.search", Some(&r.title.to_variant()));
            n.add_button_with_target_value(&gettext("Done"), "app.notify-done", Some(&r.task_id.to_variant()));
            n.add_button_with_target_value(
                &gettext("Snooze 1 hour"),
                "app.notify-snooze",
                Some(&r.task_id.to_variant()),
            );
            if let Some(app) = self.application() {
                app.send_notification(Some(&r.task_id), &n);
            }
        }
        self.morning_summary();
    }

    /// Once per day at the configured local time: "Today: 5 tasks, 2 tonight". Only when there is something to do.
    fn morning_summary(&self) {
        let Some(s) = self.engine().morning_summary() else {
            return;
        };
        let mut body = format!("{} {}", s.total, gettext("tasks today"));
        if s.morning > 0 {
            body.push_str(&format!(", {} {}", s.morning, gettext("this morning")));
        }
        if s.tonight > 0 {
            body.push_str(&format!(", {} {}", s.tonight, gettext("tonight")));
        }
        let note = gio::Notification::new(&gettext("Good morning"));
        note.set_body(Some(&body));
        note.set_default_action("app.today");
        if let Some(app) = self.application() {
            app.send_notification(Some("morning"), &note);
        }
    }

    /// Background Apps status line when running in the background.
    pub(crate) fn update_background_status(&self) {
        let imp = self.imp();
        if !imp.settings.boolean("run-in-background") {
            return;
        }
        let mode =
            crate::background_status::BackgroundCountMode::from_setting(&imp.settings.string("background-count-mode"));
        let count = imp.engine.task_count(mode.task_count_mode());
        let status = crate::background_status::truncate_status(&crate::background_status::format_status(mode, count));
        let controller = imp.background_status.clone();
        controller.borrow_mut().refresh(status, &mut |request| {
            send_background_status_request(controller.clone(), request)
        });
    }

    // ---- task dialogs ----------------------------------------------------

    fn dialog(&self, title: &str, form: &crate::task_form::TaskForm, header: &adw::HeaderBar) -> adw::Dialog {
        let tv = adw::ToolbarView::new();
        tv.add_top_bar(header);
        tv.set_content(Some(&form.page));
        let dialog = adw::Dialog::builder()
            .title(title)
            .content_width(520)
            .content_height(860)
            .child(&tv)
            .build();
        crate::typography::register_interface_root(&dialog);
        dialog
    }

    /// HIG "new item" dialog: Cancel / Create in the header, Create enabled once there is a title.
    pub fn new_task_dialog(&self) {
        let imp = self.imp();
        let view = imp.view.borrow().clone();
        let project = imp.engine.project_for(&view);
        let due = view.is_day().then(sp_model::today_str);
        let form = Rc::new(crate::task_form::TaskForm::new(self, None, &project, due));
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
                // The view adds its slot or tag, like quick-add does.
                let view = w.imp().view.borrow().clone();
                let out = w.engine().create_task(form.into_draft(), view);
                w.apply(out);
                dialog.close();
            }
        ));
        dialog.present(Some(self));
        form.title.grab_focus();
    }

    /// Edit dialog: changes are applied when the dialog closes.
    pub fn open_task(&self, id: &str) {
        let Some(t) = self.engine().task_detail(id.to_string()) else {
            return;
        };
        let form = Rc::new(crate::task_form::TaskForm::new(self, Some(&t), &t.project_id, None));
        let repeat_row = adw::ActionRow::builder()
            .title(gettext("Repeat"))
            .subtitle(
                t.repeat
                    .as_ref()
                    .map(repeat_text)
                    .unwrap_or_else(|| gettext("Does not repeat")),
            )
            .activatable(true)
            .build();
        repeat_row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
        {
            let id = t.id.clone();
            repeat_row.connect_activated(glib::clone!(
                #[weak(rename_to = w)]
                self,
                move |_| crate::repeat_dialog::open(&w, &id)
            ));
        }
        form.group.add(&repeat_row);
        let sub = adw::EntryRow::builder().title(gettext("Add subtask")).build();
        crate::typography::register_content_root(&sub);
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
                    let out = w.engine().add_subtask(id.clone(), e.text().to_string());
                    w.apply(out);
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
                let engine = w.engine();
                if engine.task_detail(id.clone()).is_none() {
                    return;
                }
                // Only the fields that differ become one update; subtasks added meanwhile
                // still need the list rebuilt.
                let out = engine.save_task(id.clone(), form.into_draft());
                if out.changed || out.message.is_some() {
                    w.apply(out);
                } else {
                    w.refresh();
                }
            }
        ));
        dialog.present(Some(self));
    }

    /// Reveal one exact current task before opening its editor. Prefer its owning
    /// project; imported tasks whose project is unavailable fall back to a title
    /// search, where the stable id still chooses the row.
    pub fn reveal_and_open_task(&self, id: &str) -> bool {
        let Some(task) = self.engine().task_detail(id.to_string()) else {
            return false;
        };
        let project = View::project(&task.project_id);
        let project_available = self
            .engine()
            .sidebar()
            .projects
            .iter()
            .any(|entry| entry.view == project);
        if project_available {
            self.go_to(project);
        } else {
            self.imp().search_entry.set_text(&task.title);
            *self.imp().filter.borrow_mut() = task.title.clone();
            self.go_to(View::Search);
            // Normal Search deliberately caps task matches. Exact external reveal is
            // window-only behavior: if the requested stable id fell beyond that cap,
            // render its real row explicitly without changing core search semantics.
            if !self.imp().rows.borrow().iter().any(|row_id| row_id == id) {
                let Some(row) = self.engine().task_row(id.to_string()) else {
                    return false;
                };
                self.new_section(None, None);
                self.task_row(&row);
            }
        }

        fn named_descendant(root: &gtk::Widget, id: &str) -> Option<gtk::Widget> {
            let mut child = root.first_child();
            while let Some(widget) = child {
                if widget.widget_name() == id {
                    return Some(widget);
                }
                if let Some(found) = named_descendant(&widget, id) {
                    return Some(found);
                }
                child = widget.next_sibling();
            }
            None
        }
        let Some(row) = named_descendant(self.imp().task_box.upcast_ref(), id) else {
            tracing::warn!("task {id} was not present after reveal navigation");
            return false;
        };
        row.set_state_flags(gtk::StateFlags::SELECTED, false);
        row.grab_focus();
        self.open_task(id);
        true
    }

    // ---- sync / backup ---------------------------------------------------

    /// Nextcloud settings without the secrets (those are read on the sync thread).
    fn nextcloud_settings(&self) -> momentum_core::NextcloudSettings {
        let s = &self.imp().settings;
        momentum_core::NextcloudSettings {
            server_url: s.string("nextcloud-server").into(),
            user_name: s.string("nextcloud-user").into(),
            folder: s.string("nextcloud-folder").into(),
            compress: s.boolean("compress"),
            password: String::new(),
            encryption_password: None,
        }
    }

    pub fn sync(&self) {
        let imp = self.imp();
        if std::env::var_os("MOMENTUM_DEMO").is_some() {
            return;
        }
        match crate::prefs::sync_method(&imp.settings).as_str() {
            "libresync" => {
                self.p2p_apply_setting();
                if !imp.syncing.get() {
                    self.p2p_sync_now();
                }
                return;
            }
            "nextcloud" => self.p2p_apply_setting(),
            _ => {
                self.toast(&gettext("Sync is turned off. Enable it in Preferences."));
                return;
            }
        }
        if imp.syncing.replace(true) {
            return;
        }
        self.update_sync_button();
        let mut settings = self.nextcloud_settings();
        let engine = self.engine();
        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to = w)]
            self,
            async move {
                let result = gio::spawn_blocking(move || {
                    settings.password = crate::keyring::get("nextcloud").unwrap_or_default();
                    settings.encryption_password = crate::keyring::get("encryption");
                    engine.sync_nextcloud(settings)
                })
                .await
                .unwrap();
                let imp = w.imp();
                imp.syncing.set(false);
                // Apply a provider switch only after the in-flight Nextcloud cycle has ended.
                w.p2p_apply_setting();
                if crate::prefs::sync_method(&imp.settings) != "nextcloud" {
                    w.refresh();
                    return;
                }
                use momentum_core::CoreError;
                match result {
                    Ok(r) => {
                        w.set_sync_error(None);
                        w.refresh();
                        w.update_sync_button();
                        tracing::info!(
                            "sync ok: downloaded={} uploaded={} ops_uploaded={}",
                            r.downloaded,
                            r.uploaded,
                            r.ops_uploaded
                        );
                        if r.downloaded || r.uploaded {
                            w.toast(&crate::messages::text(&momentum_core::Message::Synced {
                                ops_uploaded: r.ops_uploaded,
                            }));
                        }
                    }
                    Err(CoreError::Busy) => w.update_sync_button(),
                    Err(e) => {
                        tracing::warn!("sync failed: {e}");
                        w.set_sync_error(Some(e.to_string()));
                    }
                }
                w.update_background_status();
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
                let path = file.path().unwrap().to_string_lossy().into_owned();
                match w.engine().import_backup(path) {
                    Ok(out) => w.apply(out),
                    Err(e) => w.toast(&e.to_string()),
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
                    .initial_name(format!("{}.json", sp_model::today_str()))
                    .build();
                let Ok(file) = dialog.save_future(Some(&w)).await else {
                    return;
                };
                let path = file.path().unwrap().to_string_lossy().into_owned();
                match w.engine().export_backup(path) {
                    Ok(out) => w.apply(out),
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
