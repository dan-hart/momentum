// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Shared form for creating and editing a task: title, project, due day, estimate, tags, notes.
use adw::prelude::*;
use gettextrs::gettext;
use gtk::{gio, glib};
use sp_model::*;
use sp_store::Store;
use std::cell::RefCell;
use std::rc::Rc;

use crate::window::{tag_color, MomentumWindow};

pub struct TaskForm {
    pub page: adw::PreferencesPage,
    pub group: adw::PreferencesGroup,
    pub title: adw::EntryRow,
    project: adw::ComboRow,
    project_ids: Vec<String>,
    due: Rc<RefCell<Option<String>>>,
    due_row: adw::ActionRow,
    estimate: adw::EntryRow,
    tag_buttons: Vec<(String, gtk::ToggleButton)>,
    new_tags: adw::EntryRow,
    notes: gtk::TextView,
}

fn tomorrow() -> String {
    glib::DateTime::now_local()
        .unwrap()
        .add_days(1)
        .unwrap()
        .format("%Y-%m-%d")
        .unwrap()
        .to_string()
}

impl TaskForm {
    pub fn new(
        win: &MomentumWindow,
        store: &Store,
        task: Option<&Task>,
        default_project: &str,
        default_due: Option<String>,
    ) -> Self {
        let colorful = gio::Settings::new(*crate::config::APP_ID).boolean("colorful-labels");
        let page = adw::PreferencesPage::new();
        let group = adw::PreferencesGroup::new();
        let title = adw::EntryRow::builder()
            .title(gettext("Title"))
            .text(task.map(|t| t.title.as_str()).unwrap_or(""))
            .build();
        group.add(&title);

        // Project
        let projects: Vec<&Project> = store.state.project.iter().filter(|p| !p.is_archived).collect();
        let project_ids: Vec<String> = projects.iter().map(|p| p.id.clone()).collect();
        let names = gtk::StringList::new(&projects.iter().map(|p| p.title.as_str()).collect::<Vec<_>>());
        let project = adw::ComboRow::builder().title(gettext("Project")).model(&names).build();
        // Colour-coded rows: folder icon in the project's colour, both in the button and the list.
        let classes: Rc<Vec<Option<String>>> = Rc::new(
            projects
                .iter()
                .map(|p| p.color().filter(|_| colorful).and_then(|c| win.color_class(c)))
                .collect(),
        );
        let titles: Rc<Vec<String>> = Rc::new(projects.iter().map(|p| p.title.clone()).collect());
        let make_factory = || {
            let f = gtk::SignalListItemFactory::new();
            f.connect_setup(|_, item| {
                let item = item.downcast_ref::<gtk::ListItem>().unwrap();
                let b = gtk::Box::builder().spacing(8).build();
                b.append(&gtk::Image::from_icon_name("folder-symbolic"));
                b.append(&gtk::Label::builder().xalign(0.0).build());
                item.set_child(Some(&b));
            });
            f.connect_bind(glib::clone!(
                #[strong]
                classes,
                #[strong]
                titles,
                move |_, item| {
                    let item = item.downcast_ref::<gtk::ListItem>().unwrap();
                    let b = item.child().and_downcast::<gtk::Box>().unwrap();
                    let (icon, label) = (
                        b.first_child().and_downcast::<gtk::Image>().unwrap(),
                        b.last_child().and_downcast::<gtk::Label>().unwrap(),
                    );
                    let i = item.position() as usize;
                    label.set_label(titles.get(i).map(String::as_str).unwrap_or(""));
                    icon.set_css_classes(&[]);
                    if let Some(Some(c)) = classes.get(i) {
                        icon.add_css_class(c);
                    }
                }
            ));
            f
        };
        project.set_factory(Some(&make_factory()));
        project.set_list_factory(Some(&make_factory()));
        let want = task.map(|t| t.project_id.as_str()).unwrap_or(default_project);
        project.set_selected(project_ids.iter().position(|i| i == want).unwrap_or(0) as u32);
        if task.is_some_and(|t| t.parent_id.is_some()) {
            project.set_visible(false);
        }
        group.add(&project);

        // Due day: calendar popover with quick choices
        let due = Rc::new(RefCell::new(task.and_then(|t| t.due_day.clone()).or(default_due)));
        let due_row = adw::ActionRow::builder().title(gettext("Due")).build();
        let pick = gtk::MenuButton::builder()
            .icon_name("x-office-calendar-symbolic")
            .valign(gtk::Align::Center)
            .css_classes(["flat"])
            .tooltip_text(gettext("Pick a day"))
            .build();
        let cal = gtk::Calendar::new();
        let quick = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .homogeneous(true)
            .margin_top(6)
            .build();
        let pop_box = gtk::Box::builder().orientation(gtk::Orientation::Vertical).build();
        pop_box.append(&cal);
        pop_box.append(&quick);
        let popover = gtk::Popover::builder().child(&pop_box).build();
        pick.set_popover(Some(&popover));
        let set_due = {
            let due = due.clone();
            let row = due_row.clone();
            move |v: Option<String>| {
                row.set_subtitle(
                    &v.as_deref()
                        .map(crate::window::fmt_day)
                        .unwrap_or_else(|| gettext("Not scheduled")),
                );
                *due.borrow_mut() = v;
            }
        };
        for (label, value) in [
            (gettext("Today"), Some(today_str())),
            (gettext("Tomorrow"), Some(tomorrow())),
            (gettext("None"), None),
        ] {
            let b = gtk::Button::builder().label(label).css_classes(["flat"]).build();
            b.connect_clicked(glib::clone!(
                #[strong]
                set_due,
                #[weak]
                popover,
                move |_| {
                    set_due(value.clone());
                    popover.popdown();
                }
            ));
            quick.append(&b);
        }
        cal.connect_day_selected(glib::clone!(
            #[strong]
            set_due,
            move |c| set_due(Some(c.date().format("%Y-%m-%d").unwrap().to_string()))
        ));
        let initial = due.borrow().clone();
        set_due(initial);
        due_row.add_suffix(&pick);
        group.add(&due_row);

        let estimate = adw::EntryRow::builder()
            .title(gettext("Estimate, e.g. 1h 30m"))
            .text(
                task.filter(|t| t.time_estimate > 0.0)
                    .map(|t| crate::window::fmt_ms(t.time_estimate))
                    .unwrap_or_default(),
            )
            .build();
        group.add(&estimate);

        // Tags: toggle chips for existing tags, entry for new ones
        let tags_group = adw::PreferencesGroup::builder().title(gettext("Tags")).build();
        let flow = gtk::FlowBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .column_spacing(6)
            .row_spacing(6)
            .margin_bottom(6)
            .build();
        let mut tag_buttons = vec![];
        for g in store.state.tag.iter().filter(|g| g.id != TODAY_TAG_ID) {
            let content = gtk::Box::builder().spacing(6).build();
            let icon = gtk::Image::from_icon_name("tag-symbolic");
            if let Some(c) = tag_color(g).filter(|_| colorful).and_then(|c| win.color_class(c)) {
                icon.add_css_class(&c);
            }
            content.append(&icon);
            content.append(&gtk::Label::new(Some(&g.title)));
            let b = gtk::ToggleButton::builder()
                .child(&content)
                .active(task.is_some_and(|t| t.tag_ids.contains(&g.id)))
                .css_classes(["pill"])
                .tooltip_text(&g.title)
                .build();
            b.update_property(&[gtk::accessible::Property::Label(&g.title)]);
            flow.insert(&b, -1);
            tag_buttons.push((g.id.clone(), b));
        }
        if !tag_buttons.is_empty() {
            tags_group.add(&flow);
        }
        let new_tags = adw::EntryRow::builder()
            .title(gettext("New tags, comma separated"))
            .build();
        tags_group.add(&new_tags);

        let notes_group = adw::PreferencesGroup::builder().title(gettext("Notes")).build();
        // Header-suffix copy button, the AdwPreferencesGroup pattern for group-level actions.
        let copy = gtk::Button::builder()
            .icon_name("edit-copy-symbolic")
            .tooltip_text(gettext("Copy Notes"))
            .valign(gtk::Align::Center)
            .css_classes(["flat"])
            .build();
        notes_group.set_header_suffix(Some(&copy));
        let notes = gtk::TextView::builder()
            .wrap_mode(gtk::WrapMode::WordChar)
            .top_margin(12)
            .bottom_margin(12)
            .left_margin(12)
            .right_margin(12)
            .height_request(240)
            .build();
        copy.connect_clicked(glib::clone!(
            #[weak]
            notes,
            move |b| {
                let buf = notes.buffer();
                let text = buf.text(&buf.start_iter(), &buf.end_iter(), false);
                b.clipboard().set_text(&text);
                if let Some(root) = b.root().and_downcast::<crate::window::MomentumWindow>() {
                    root.toast(&gettext("Notes copied"));
                }
            }
        ));
        notes
            .buffer()
            .set_text(task.and_then(|t| t.notes.as_deref()).unwrap_or(""));
        notes_group.add(
            &gtk::Frame::builder()
                .child(&gtk::ScrolledWindow::builder().child(&notes).build())
                .build(),
        );

        page.add(&group);
        page.add(&tags_group);
        page.add(&notes_group);
        Self {
            page,
            group,
            title,
            project,
            project_ids,
            due,
            due_row,
            estimate,
            tag_buttons,
            new_tags,
            notes,
        }
    }

    pub fn title_text(&self) -> String {
        self.title.text().trim().to_string()
    }
    pub fn project_id(&self) -> String {
        self.project_ids
            .get(self.project.selected() as usize)
            .cloned()
            .unwrap_or_default()
    }
    pub fn due_day(&self) -> Option<String> {
        self.due.borrow().clone()
    }
    pub fn estimate_ms(&self) -> f64 {
        crate::window::parse_ms(&self.estimate.text()).unwrap_or(0.0)
    }
    pub fn notes_text(&self) -> String {
        let b = self.notes.buffer();
        b.text(&b.start_iter(), &b.end_iter(), false).to_string()
    }
    /// Selected existing tags plus any typed new ones (created on the fly).
    pub fn tag_ids(&self, store: &mut Store) -> Vec<String> {
        let mut ids: Vec<String> = self
            .tag_buttons
            .iter()
            .filter(|(_, b)| b.is_active())
            .map(|(id, _)| id.clone())
            .collect();
        for name in self.new_tags.text().split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let existing = store
                .state
                .tag
                .iter()
                .find(|g| g.title.eq_ignore_ascii_case(name))
                .map(|g| g.id.clone());
            let id = existing.unwrap_or_else(|| {
                let tag = Tag::new(name);
                let id = tag.id.clone();
                store.dispatch(sp_oplog::Action::AddTag { tag });
                id
            });
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        ids
    }
    /// Build a new task from the form.
    pub fn into_task(&self, store: &mut Store) -> Task {
        let mut t = Task::new(&self.title_text(), &self.project_id());
        t.due_day = self.due_day();
        t.time_estimate = self.estimate_ms();
        let n = self.notes_text();
        if !n.is_empty() {
            t.notes = Some(n);
        }
        t.tag_ids = self.tag_ids(store);
        t
    }
    #[allow(dead_code)]
    pub fn due_row(&self) -> &adw::ActionRow {
        &self.due_row
    }
}
