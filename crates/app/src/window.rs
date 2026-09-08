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

use crate::application::MomentumApplication;
use crate::config::{APP_ID, PROFILE};

#[derive(Clone, PartialEq)]
pub enum View { Today, Project(String), Tag(String) }

mod imp {
    use super::*;

    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/github/danhart/Momentum/ui/window.ui")]
    pub struct MomentumWindow {
        #[template_child] pub split_view: TemplateChild<adw::NavigationSplitView>,
        #[template_child] pub sidebar_list: TemplateChild<gtk::ListBox>,
        #[template_child] pub content_page: TemplateChild<adw::NavigationPage>,
        #[template_child] pub task_list: TemplateChild<gtk::ListBox>,
        #[template_child] pub add_entry: TemplateChild<gtk::Entry>,
        #[template_child] pub create_button: TemplateChild<gtk::Button>,
        #[template_child] pub sync_button: TemplateChild<gtk::Button>,
        #[template_child] pub search_bar: TemplateChild<gtk::SearchBar>,
        #[template_child] pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child] pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child] pub empty: TemplateChild<adw::StatusPage>,
        pub settings: gio::Settings,
        pub store: RefCell<Store>,
        pub view: RefCell<View>,
        pub views: RefCell<Vec<Option<View>>>,
        pub rows: RefCell<Vec<String>>,
        pub filter: RefCell<String>,
        pub syncing: Cell<bool>,
        pub notified: RefCell<HashSet<String>>,
    }

    impl Default for MomentumWindow {
        fn default() -> Self {
            let dir = glib::user_data_dir().join("momentum");
            Self {
                split_view: Default::default(), sidebar_list: Default::default(), content_page: Default::default(),
                task_list: Default::default(), add_entry: Default::default(), create_button: Default::default(), sync_button: Default::default(),
                search_bar: Default::default(), search_entry: Default::default(), toast_overlay: Default::default(), empty: Default::default(),
                settings: gio::Settings::new(*APP_ID), store: RefCell::new(Store::load(dir)), view: RefCell::new(View::Today),
                views: Default::default(), rows: Default::default(), filter: Default::default(), syncing: Cell::new(false), notified: Default::default(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MomentumWindow {
        const NAME: &'static str = "MomentumWindow";
        type Type = super::MomentumWindow;
        type ParentType = adw::ApplicationWindow;
        fn class_init(klass: &mut Self::Class) { klass.bind_template(); }
        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) { obj.init_template(); }
    }

    impl ObjectImpl for MomentumWindow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            if *PROFILE == "Devel" { obj.add_css_class("devel"); }
            obj.load_window_size();
            // libadwaita ≥ 1.6 follows the system accent colour through the settings portal;
            // nothing here hard-codes colours, so the whole UI inherits it.
            let sm = adw::StyleManager::default();
            tracing::info!("System accent colour: supported={} {:?}", sm.is_system_supports_accent_colors(), sm.accent_color());
            obj.setup();
        }
    }
    impl WidgetImpl for MomentumWindow {}
    impl WindowImpl for MomentumWindow {
        fn close_request(&self) -> glib::Propagation {
            if let Err(err) = self.obj().save_window_size() { tracing::warn!("Failed to save window state, {}", &err); }
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
    if m >= 60 { format!("{}h {:02}m", m / 60, m % 60) } else { format!("{m}m") }
}
/// "1h 30m", "45m", "2h" → ms
pub fn parse_ms(s: &str) -> Option<f64> {
    let mut total = 0.0; let mut num = String::new(); let mut any = false;
    for c in s.chars() {
        if c.is_ascii_digit() || c == '.' { num.push(c) } else if c == 'h' || c == 'm' {
            total += num.parse::<f64>().ok()? * if c == 'h' { 3_600_000.0 } else { 60_000.0 }; num.clear(); any = true;
        }
    }
    any.then_some(total)
}

impl MomentumWindow {
    pub fn new(app: &MomentumApplication) -> Self { glib::Object::builder().property("application", app).build() }

    fn setup(&self) {
        let imp = self.imp();
        imp.sidebar_list.connect_row_selected(glib::clone!(#[weak(rename_to = w)] self, move |_, row| {
            let Some(row) = row else { return };
            if let Some(Some(v)) = w.imp().views.borrow().get(row.index() as usize) { *w.imp().view.borrow_mut() = v.clone(); }
            w.imp().split_view.set_show_content(true);
            w.refresh_tasks();
        }));
        imp.add_entry.connect_activate(glib::clone!(#[weak(rename_to = w)] self, move |e| { w.add_task(&e.text()); e.set_text(""); }));
        imp.create_button.connect_clicked(glib::clone!(#[weak(rename_to = w)] self, move |_| { let e = &w.imp().add_entry; w.add_task(&e.text()); e.set_text(""); e.grab_focus(); }));
        imp.task_list.connect_row_activated(glib::clone!(#[weak(rename_to = w)] self, move |_, row| {
            if let Some(id) = w.imp().rows.borrow().get(row.index() as usize).cloned() { w.open_task(&id); }
        }));
        imp.search_bar.set_key_capture_widget(Some(self));
        imp.search_entry.connect_search_changed(glib::clone!(#[weak(rename_to = w)] self, move |e| { *w.imp().filter.borrow_mut() = e.text().to_lowercase(); w.refresh_tasks(); }));
        let key = gtk::EventControllerKey::new();
        key.connect_key_pressed(glib::clone!(#[weak(rename_to = w)] self, #[upgrade_or] glib::Propagation::Proceed, move |_, k, _, m| {
            // Alt+1…9 jumps to the n-th sidebar entry, like tabs in Files and Terminal.
            let n = k.to_unicode().and_then(|c| c.to_digit(10)).filter(|&d| d > 0 && m.contains(gtk::gdk::ModifierType::ALT_MASK));
            match n.and_then(|d| w.imp().views.borrow().iter().enumerate().filter(|(_, v)| v.is_some()).nth(d as usize - 1).map(|(i, _)| i)) {
                Some(i) => { w.imp().sidebar_list.select_row(w.imp().sidebar_list.row_at_index(i as i32).as_ref()); glib::Propagation::Stop }
                None => glib::Propagation::Proceed,
            }
        }));
        self.add_controller(key);
        let act = |name: &str, f: fn(&MomentumWindow)| gio::ActionEntry::builder(name).activate(move |w: &MomentumWindow, _, _| f(w)).build();
        self.add_action_entries([
            act("search", |w| { let b = &w.imp().search_bar; b.set_search_mode(!b.is_search_mode()); if b.is_search_mode() { w.imp().search_entry.grab_focus(); } }),
            act("toggle-done", |w| if let Some(id) = w.focused_task() { let done = w.imp().store.borrow().state.task.entities[&id].is_done; w.set_done(&id, !done); }),
            act("delete-task", |w| if let Some(id) = w.focused_task() { w.delete_task(&id); }),
        ]);
        imp.settings.connect_changed(None, glib::clone!(#[weak(rename_to = w)] self, move |_, _| w.update_sync_button()));
        self.update_sync_button();
        glib::timeout_add_seconds_local(30, glib::clone!(#[weak(rename_to = w)] self, #[upgrade_or] glib::ControlFlow::Break, move || { w.check_reminders(); glib::ControlFlow::Continue }));
        glib::timeout_add_seconds_local(300, glib::clone!(#[weak(rename_to = w)] self, #[upgrade_or] glib::ControlFlow::Break, move || {
            if w.imp().settings.boolean("auto-sync") && w.sync_configured() { w.sync(); } glib::ControlFlow::Continue
        }));
        self.refresh();
        if imp.settings.boolean("auto-sync") && self.sync_configured() { self.sync(); }
    }

    fn sync_configured(&self) -> bool { let s = &self.imp().settings; ["nextcloud-server", "nextcloud-user", "nextcloud-folder"].iter().all(|k| !s.string(k).trim().is_empty()) }
    fn update_sync_button(&self) { self.imp().sync_button.set_visible(self.sync_configured()); }
    fn focused_task(&self) -> Option<String> {
        let row = self.imp().task_list.focus_child()?.downcast::<gtk::ListBoxRow>().ok()?;
        self.imp().rows.borrow().get(row.index() as usize).cloned()
    }

    pub fn toast(&self, msg: &str) { self.imp().toast_overlay.add_toast(adw::Toast::new(msg)); }
    pub fn dispatch(&self, a: Action) { self.imp().store.borrow_mut().dispatch(a); self.refresh(); }
    pub fn focus_add(&self) { self.imp().add_entry.grab_focus(); }

    // ---- rendering -------------------------------------------------------

    pub fn refresh(&self) { self.refresh_sidebar(); self.refresh_tasks(); }

    fn sidebar_row(&self, title: &str, icon: &str, view: Option<View>, color: Option<&str>) {
        let imp = self.imp();
        let row: gtk::ListBoxRow = if view.is_some() {
            let r = adw::ActionRow::builder().title(title).build();
            let img = gtk::Image::from_icon_name(icon);
            if color.is_none() && icon == "starred-symbolic" { img.add_css_class("accent"); } // follows the system accent colour
            r.add_prefix(&img); r.upcast()
        } else {
            let l = gtk::Label::builder().label(title).xalign(0.0).margin_top(12).margin_start(6).css_classes(["heading", "dim-label"]).build();
            gtk::ListBoxRow::builder().child(&l).selectable(false).activatable(false).build()
        };
        imp.sidebar_list.append(&row); imp.views.borrow_mut().push(view);
    }

    fn refresh_sidebar(&self) {
        let imp = self.imp();
        let current = imp.view.borrow().clone();
        imp.sidebar_list.remove_all(); imp.views.borrow_mut().clear();
        let store = imp.store.borrow();
        self.sidebar_row(&gettext("Today"), "starred-symbolic", Some(View::Today), None);
        self.sidebar_row(&gettext("Projects"), "", None, None);
        for p in store.state.project.iter().filter(|p| !p.is_archived && !p.is_hidden_from_menu) {
            self.sidebar_row(&p.title, "folder-symbolic", Some(View::Project(p.id.clone())), p.color());
        }
        self.sidebar_row(&gettext("Tags"), "", None, None);
        for t in store.state.tag.iter().filter(|t| t.id != TODAY_TAG_ID) {
            self.sidebar_row(&t.title, "tag-symbolic", Some(View::Tag(t.id.clone())), None);
        }
        drop(store);
        let idx = imp.views.borrow().iter().position(|v| v.as_ref() == Some(&current)).unwrap_or(0);
        imp.sidebar_list.select_row(imp.sidebar_list.row_at_index(idx as i32).as_ref());
    }

    fn view_task_ids(&self, store: &Store) -> Vec<String> {
        match &*self.imp().view.borrow() {
            View::Today => store.state.today_ids(),
            View::Project(id) => store.state.project.entities.get(id).map(|p| p.task_ids.clone()).unwrap_or_default(),
            View::Tag(id) => store.state.tag.entities.get(id).map(|t| t.task_ids.clone()).unwrap_or_default(),
        }
    }

    fn task_row(&self, t: &Task, store: &Store, indent: bool) {
        let imp = self.imp();
        let mut sub = vec![];
        if t.time_estimate > 0.0 { sub.push(format!("~{}", fmt_ms(t.time_estimate))); }
        if let Some(d) = &t.due_day { if *imp.view.borrow() != View::Today { sub.push(d.clone()); } }
        for tag in &t.tag_ids { if let Some(g) = store.state.tag.entities.get(tag) { sub.push(format!("#{}", g.title)); } }
        let row = adw::ActionRow::builder().title(glib::markup_escape_text(&t.title)).subtitle(sub.join("  ·  ")).activatable(true).build();
        if indent { row.set_margin_start(32); }
        if t.is_done { row.add_css_class("dim-label"); }
        let check = gtk::CheckButton::builder().active(t.is_done).valign(gtk::Align::Center).build();
        let id = t.id.clone();
        check.connect_toggled(glib::clone!(#[weak(rename_to = w)] self, move |c| w.set_done(&id, c.is_active())));
        row.add_prefix(&check);
        imp.task_list.append(&row); imp.rows.borrow_mut().push(t.id.clone());
    }

    fn refresh_tasks(&self) {
        let imp = self.imp();
        imp.task_list.remove_all(); imp.rows.borrow_mut().clear();
        let store = imp.store.borrow();
        let title = match &*imp.view.borrow() {
            View::Today => gettext("Today"),
            View::Project(id) => store.state.project.entities.get(id).map(|p| p.title.clone()).unwrap_or_default(),
            View::Tag(id) => store.state.tag.entities.get(id).map(|t| t.title.clone()).unwrap_or_default(),
        };
        imp.content_page.set_title(&title);
        let ids = self.view_task_ids(&store);
        let (mut open, mut done) = (vec![], vec![]);
        let filter = imp.filter.borrow();
        for id in ids { if let Some(t) = store.state.task.entities.get(&id) { if !t.title.to_lowercase().contains(&*filter) { continue; } if t.is_done { done.push(t) } else { open.push(t) } } }
        for t in open.into_iter().chain(done) {
            self.task_row(t, &store, false);
            for s in t.sub_task_ids.iter().filter_map(|i| store.state.task.entities.get(i)) { self.task_row(s, &store, true); }
        }
        imp.empty.set_visible(imp.rows.borrow().is_empty());
        imp.task_list.set_visible(!imp.rows.borrow().is_empty());
    }

    // ---- actions ---------------------------------------------------------

    fn current_project(&self, store: &Store) -> String {
        match &*self.imp().view.borrow() { View::Project(id) => id.clone(), _ => store.state.project.ids.first().cloned().unwrap_or(INBOX_PROJECT_ID.into()) }
    }

    /// Short syntax: `#tag` adds/creates tags, a trailing `1h 30m` sets the estimate.
    pub fn add_task(&self, text: &str) {
        if text.trim().is_empty() { return; }
        let imp = self.imp();
        let (mut words, mut tags, mut est) = (vec![], vec![], 0.0);
        for w in text.split_whitespace() {
            if let Some(tag) = w.strip_prefix('#') { tags.push(tag.to_string()); } else if let Some(ms) = parse_ms(w) { est = ms; } else { words.push(w); }
        }
        let project = self.current_project(&imp.store.borrow());
        let mut task = Task::new(&words.join(" "), &project);
        task.time_estimate = est;
        let view = imp.view.borrow().clone();
        if view == View::Today { task.due_day = Some(today_str()); }
        if let View::Tag(id) = &view { tags.push(imp.store.borrow().state.tag.entities[id].title.clone()); }
        for name in tags {
            let existing = imp.store.borrow().state.tag.iter().find(|t| t.title.eq_ignore_ascii_case(&name)).map(|t| t.id.clone());
            let id = existing.unwrap_or_else(|| { let tag = Tag::new(&name); let id = tag.id.clone(); imp.store.borrow_mut().dispatch(Action::AddTag { tag }); id });
            if !task.tag_ids.contains(&id) { task.tag_ids.push(id); }
        }
        self.dispatch(Action::AddTask { task, bottom: true });
    }

    pub fn add_project(&self, title: &str) { if !title.trim().is_empty() { self.dispatch(Action::AddProject { project: Project::new(title) }); } }

    fn set_done(&self, id: &str, done: bool) {
        self.update_task(id, [("isDone".to_string(), json!(done))].into_iter().collect());
    }
    fn update_task(&self, id: &str, changes: Map<String, Value>) { self.dispatch(Action::UpdateTask { id: id.into(), changes }); }

    fn delete_task(&self, id: &str) {
        let store = self.imp().store.borrow();
        let Some(task) = store.state.task.entities.get(id).cloned() else { return };
        let sub_tasks = task.sub_task_ids.iter().filter_map(|i| store.state.task.entities.get(i).cloned()).collect();
        drop(store);
        self.dispatch(Action::DeleteTask { task, sub_tasks });
        self.toast(&gettext("Task deleted"));
    }

    fn check_reminders(&self) {
        let imp = self.imp();
        let now = now_ms();
        let due: Vec<Task> = imp.store.borrow().state.task.iter().filter(|t| !t.is_done && t.remind_at.is_some_and(|r| r <= now) && !imp.notified.borrow().contains(&t.id)).cloned().collect();
        for t in due {
            imp.notified.borrow_mut().insert(t.id.clone());
            let n = gio::Notification::new(&t.title);
            n.set_body(Some(&gettext("Reminder")));
            if let Some(app) = self.application() { app.send_notification(Some(&t.id), &n); }
        }
    }

    // ---- task detail dialog ---------------------------------------------

    pub fn open_task(&self, id: &str) {
        let store = self.imp().store.borrow();
        let Some(t) = store.state.task.entities.get(id).cloned() else { return };
        let tag_names: Vec<String> = t.tag_ids.iter().filter_map(|i| store.state.tag.entities.get(i)).map(|g| g.title.clone()).collect();
        drop(store);
        let page = adw::PreferencesPage::new();
        let g = adw::PreferencesGroup::new();
        let title = adw::EntryRow::builder().title(gettext("Title")).text(&t.title).build();
        let est = adw::EntryRow::builder().title(gettext("Estimate (e.g. 1h 30m)")).text(if t.time_estimate > 0.0 { fmt_ms(t.time_estimate) } else { String::new() }).build();
        let due = adw::EntryRow::builder().title(gettext("Due day (YYYY-MM-DD)")).text(t.due_day.clone().unwrap_or_default()).build();
        let today_btn = gtk::Button::builder().label(gettext("Today")).valign(gtk::Align::Center).css_classes(["flat"]).build();
        today_btn.connect_clicked(glib::clone!(#[weak] due, move |_| due.set_text(&today_str())));
        due.add_suffix(&today_btn);
        let tags = adw::EntryRow::builder().title(gettext("Tags (comma separated)")).text(tag_names.join(", ")).build();
        let sub = adw::EntryRow::builder().title(gettext("Add subtask")).build();
        let notes = gtk::TextView::builder().wrap_mode(gtk::WrapMode::WordChar).top_margin(8).bottom_margin(8).left_margin(8).right_margin(8).height_request(140).build();
        notes.buffer().set_text(t.notes.as_deref().unwrap_or(""));
        let notes_frame = gtk::Frame::builder().child(&gtk::ScrolledWindow::builder().child(&notes).build()).margin_top(12).build();
        for r in [&title, &est, &due, &tags, &sub] { g.add(r); }
        g.add(&notes_frame);
        let del = gtk::Button::builder().label(gettext("Delete Task")).css_classes(["destructive-action"]).margin_top(12).halign(gtk::Align::End).build();
        g.add(&del);
        page.add(&g);
        let tv = adw::ToolbarView::new(); tv.add_top_bar(&adw::HeaderBar::new()); tv.set_content(Some(&page));
        let dialog = adw::Dialog::builder().title(gettext("Task")).content_width(520).content_height(600).child(&tv).build();
        let id = t.id.clone();
        del.connect_clicked(glib::clone!(#[weak(rename_to = w)] self, #[weak] dialog, #[strong] id, move |_| { dialog.close(); w.delete_task(&id); }));
        sub.connect_entry_activated(glib::clone!(#[weak(rename_to = w)] self, #[strong] id, move |e| {
            if !e.text().trim().is_empty() {
                let mut task = Task::new(&e.text(), ""); task.parent_id = Some(id.clone());
                w.dispatch(Action::AddSubTask { task, parent_id: id.clone() }); e.set_text("");
            }
        }));
        dialog.connect_closed(glib::clone!(#[weak(rename_to = w)] self, move |_| {
            let imp = w.imp();
            if !imp.store.borrow().state.task.entities.contains_key(&id) { return; }
            let mut ch = Map::new();
            let nt = title.text().trim().to_string(); if !nt.is_empty() && nt != t.title { ch.insert("title".into(), json!(nt)); }
            let ne = parse_ms(&est.text()).unwrap_or(0.0); if ne != t.time_estimate { ch.insert("timeEstimate".into(), json!(ne)); }
            let nd = due.text().trim().to_string(); let nd = (nd.len() == 10).then_some(nd);
            if nd != t.due_day { ch.insert("dueDay".into(), json!(nd)); if nd.is_some() { ch.insert("dueWithTime".into(), Value::Null); } }
            let b = notes.buffer(); let nn = b.text(&b.start_iter(), &b.end_iter(), false).to_string();
            if nn != t.notes.clone().unwrap_or_default() { ch.insert("notes".into(), json!(nn)); }
            let mut ids = vec![];
            for name in tags.text().split(',').map(str::trim).filter(|s| !s.is_empty()) {
                let found = imp.store.borrow().state.tag.iter().find(|g| g.title.eq_ignore_ascii_case(name)).map(|g| g.id.clone());
                ids.push(found.unwrap_or_else(|| { let tag = Tag::new(name); let tid = tag.id.clone(); imp.store.borrow_mut().dispatch(Action::AddTag { tag }); tid }));
            }
            if ids != t.tag_ids { ch.insert("tagIds".into(), json!(ids)); }
            if ch.is_empty() { w.refresh(); } else { w.update_task(&id, ch); }
        }));
        dialog.present(Some(self));
    }

    // ---- sync / backup ---------------------------------------------------

    pub fn nextcloud_cfg(&self) -> sp_sync::NextcloudCfg {
        let s = &self.imp().settings;
        sp_sync::NextcloudCfg { server_url: s.string("nextcloud-server").into(), user_name: s.string("nextcloud-user").into(),
            folder: s.string("nextcloud-folder").into(), compress: s.boolean("compress"), password: String::new() }
    }

    pub fn sync(&self) {
        let imp = self.imp();
        if imp.syncing.replace(true) { return; }
        let mut cfg = self.nextcloud_cfg();
        let snapshot = imp.store.borrow().clone();
        let n_pending = snapshot.pending.len();
        glib::spawn_future_local(glib::clone!(#[weak(rename_to = w)] self, async move {
            let result = gio::spawn_blocking(move || {
                cfg.password = crate::keyring::get().unwrap_or_default();
                if !cfg.is_complete() { return Err("Nextcloud sync is not configured".to_string()); }
                let mut s = snapshot; sp_sync::sync(&cfg, &mut s).map(|r| (s, r)).map_err(|e| e.to_string())
            }).await.unwrap();
            let imp = w.imp();
            imp.syncing.set(false);
            match result {
                Ok((mut synced, r)) => {
                    // Re-apply anything dispatched while the sync ran, keeping it pending.
                    let live = imp.store.borrow().pending[n_pending..].to_vec();
                    for p in &live { sp_oplog::apply(&mut synced.state, &p.action); }
                    synced.pending.extend(live); synced.save().ok();
                    *imp.store.borrow_mut() = synced;
                    w.refresh();
                    if r.downloaded || r.uploaded { w.toast(&format!("{} ({}↑)", gettext("Synced"), r.ops_uploaded)); }
                }
                Err(e) => w.toast(&format!("{}: {e}", gettext("Sync failed"))),
            }
        }));
    }

    pub fn import_backup(&self) {
        glib::spawn_future_local(glib::clone!(#[weak(rename_to = w)] self, async move {
            let Ok(file) = gtk::FileDialog::builder().title(gettext("Import Super Productivity backup")).build().open_future(Some(&w)).await else { return };
            let res = std::fs::read(file.path().unwrap()).map_err(|e| e.to_string())
                .and_then(|b| serde_json::from_slice::<Value>(&b).map_err(|e| e.to_string()))
                .and_then(|v| AppData::from_backup(v).map_err(|e| e.to_string()));
            match res { Ok(d) => { w.imp().store.borrow_mut().replace_state(d); w.refresh(); w.toast(&gettext("Backup imported")); } Err(e) => w.toast(&e) }
        }));
    }
    pub fn export_backup(&self) {
        glib::spawn_future_local(glib::clone!(#[weak(rename_to = w)] self, async move {
            let dialog = gtk::FileDialog::builder().title(gettext("Export backup")).initial_name(format!("{}.json", today_str())).build();
            let Ok(file) = dialog.save_future(Some(&w)).await else { return };
            let body = json!({"data": w.imp().store.borrow().state, "timestamp": now_ms(), "crossModelVersion": 4.5});
            match std::fs::write(file.path().unwrap(), serde_json::to_vec_pretty(&body).unwrap()) { Ok(_) => w.toast(&gettext("Backup exported")), Err(e) => w.toast(&e.to_string()) }
        }));
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
        if imp.settings.boolean("is-maximized") { self.maximize(); }
    }
}
