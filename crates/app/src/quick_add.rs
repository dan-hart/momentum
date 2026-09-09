// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! A small floating window with just the add box, for the system-wide shortcut and `--quick-add`.
use adw::prelude::*;
use gettextrs::gettext;
use gtk::glib;

use crate::application::MomentumApplication;

pub fn open(app: &MomentumApplication) {
    let main = app.ensure_window();
    let entry = gtk::Entry::builder()
        .placeholder_text(gettext("Add a task…  #tag  1h 30m  (Enter to add, Esc to close)"))
        .primary_icon_name("list-add-symbolic")
        .width_chars(48)
        .build();
    let hint = gtk::Label::builder()
        .label(gettext("Added to Today"))
        .css_classes(["dim-label", "caption"])
        .xalign(0.0)
        .build();
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    content.append(&entry);
    content.append(&hint);
    let tv = adw::ToolbarView::new();
    tv.add_top_bar(&adw::HeaderBar::builder().show_title(false).build());
    tv.set_content(Some(&content));
    let win = adw::Window::builder()
        .application(app)
        .title(gettext("Add Task"))
        .default_width(520)
        .resizable(false)
        .content(&tv)
        .build();
    entry.connect_activate(glib::clone!(
        #[weak]
        win,
        #[weak]
        main,
        move |e| {
            let text = e.text().to_string();
            if !text.trim().is_empty() {
                main.add_task_for_today(&text);
            }
            win.close();
        }
    ));
    let keys = gtk::EventControllerKey::new();
    keys.connect_key_pressed(glib::clone!(
        #[weak]
        win,
        #[upgrade_or]
        glib::Propagation::Proceed,
        move |_, k, _, _| {
            if k == gtk::gdk::Key::Escape {
                win.close();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        }
    ));
    win.add_controller(keys);
    win.present();
    entry.grab_focus();
}
