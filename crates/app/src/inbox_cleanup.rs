// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart

use adw::prelude::*;
use gettextrs::gettext;
use gtk::glib;
use momentum_core::InboxCleanupChange;
use std::collections::HashMap;
use std::rc::Rc;

use super::MomentumWindow;

struct PreviewRow {
    change: InboxCleanupChange,
    included: gtk::CheckButton,
    tags: Vec<(String, gtk::CheckButton)>,
}

impl PreviewRow {
    fn new(change: InboxCleanupChange, tag_titles: &HashMap<String, String>, list: &gtk::Box) -> Self {
        let included = checkbox(&change.original_title);
        included.set_widget_name(&change.task_id);
        included.add_css_class("heading");
        list.append(&included);

        let details = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(6)
            .margin_start(28)
            .build();
        if change.title != change.original_title {
            details.append(&heading(&gettext("New title")));
            details.append(&wrapped_label(&change.title));
        }
        if !change.links.is_empty() {
            details.append(&heading(&gettext("Links moved to notes")));
            for link in &change.links {
                details.append(&wrapped_label(link));
            }
        }

        let tags: Vec<_> = change
            .tag_ids
            .iter()
            .filter_map(|id| tag_titles.get(id).map(|title| (id.clone(), checkbox(title))))
            .collect();
        if !tags.is_empty() {
            details.append(&heading(&gettext("Suggested tags")));
            for (id, choice) in &tags {
                choice.set_widget_name(&format!("inbox-cleanup-tag-{id}"));
                details.append(choice);
            }
        }
        included
            .bind_property("active", &details, "sensitive")
            .sync_create()
            .build();
        list.append(&details);

        Self { change, included, tags }
    }

    fn will_apply(&self) -> bool {
        self.included.is_active()
            && (self.change.title != self.change.original_title
                || !self.change.links.is_empty()
                || self.tags.iter().any(|(_, choice)| choice.is_active()))
    }

    fn selected_change(&self) -> InboxCleanupChange {
        let mut change = self.change.clone();
        change.tag_ids = self
            .tags
            .iter()
            .filter(|(_, choice)| choice.is_active())
            .map(|(id, _)| id.clone())
            .collect();
        change
    }
}

fn wrapped_label(text: &str) -> gtk::Label {
    gtk::Label::builder()
        .label(text)
        .xalign(0.0)
        .wrap(true)
        .wrap_mode(gtk::pango::WrapMode::WordChar)
        .build()
}

fn checkbox(text: &str) -> gtk::CheckButton {
    let choice = gtk::CheckButton::builder()
        .child(&wrapped_label(text))
        .active(true)
        .build();
    choice.update_property(&[gtk::accessible::Property::Label(text)]);
    choice
}

fn heading(text: &str) -> gtk::Label {
    let label = wrapped_label(text);
    label.add_css_class("dim-label");
    label
}

impl MomentumWindow {
    pub(super) fn show_inbox_cleanup(&self) {
        let engine = self.engine();
        let changes = engine
            .preview_inbox_cleanup()
            .into_iter()
            .filter(|change| {
                change.title != change.original_title || !change.links.is_empty() || !change.tag_ids.is_empty()
            })
            .collect::<Vec<_>>();
        if changes.is_empty() {
            self.toast(&gettext("No Inbox changes to review"));
            return;
        }

        let cancel = gtk::Button::with_label(&gettext("Cancel"));
        let apply = gtk::Button::with_label(&gettext("Apply Changes"));
        apply.add_css_class("suggested-action");
        let header = adw::HeaderBar::builder()
            .show_start_title_buttons(false)
            .show_end_title_buttons(false)
            .build();
        header.pack_start(&cancel);
        header.pack_end(&apply);

        let list = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .margin_start(16)
            .margin_end(16)
            .margin_top(16)
            .margin_bottom(16)
            .build();
        let tag_titles = engine.tags().into_iter().map(|tag| (tag.id, tag.title)).collect();
        let rows = Rc::new(
            changes
                .into_iter()
                .enumerate()
                .map(|(index, change)| {
                    if index > 0 {
                        list.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
                    }
                    PreviewRow::new(change, &tag_titles, &list)
                })
                .collect::<Vec<_>>(),
        );
        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&list)
            .build();
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(&scroll));
        let dialog = adw::Dialog::builder()
            .title(gettext("Clean Up Inbox"))
            .content_width(520)
            .content_height(460)
            .child(&toolbar)
            .build();
        crate::typography::register_interface_root(&dialog);

        for row in rows.iter() {
            for choice in std::iter::once(&row.included).chain(row.tags.iter().map(|(_, choice)| choice)) {
                let rows = Rc::downgrade(&rows);
                let apply = apply.downgrade();
                choice.connect_toggled(move |_| {
                    if let (Some(rows), Some(apply)) = (rows.upgrade(), apply.upgrade()) {
                        apply.set_sensitive(rows.iter().any(PreviewRow::will_apply));
                    }
                });
            }
        }
        apply.set_sensitive(rows.iter().any(PreviewRow::will_apply));
        cancel.connect_clicked(glib::clone!(
            #[weak]
            dialog,
            move |_| {
                dialog.close();
            }
        ));
        apply.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            dialog,
            move |_| {
                let changes = rows
                    .iter()
                    .filter(|row| row.will_apply())
                    .map(PreviewRow::selected_change)
                    .collect::<Vec<_>>();
                if !changes.is_empty() {
                    dialog.close();
                    window.apply(window.engine().apply_inbox_cleanup(changes));
                }
            }
        ));
        dialog.present(Some(self));
    }
}
