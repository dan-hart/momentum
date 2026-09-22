// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Create or edit a task's repeat schedule: cycle, interval, weekdays, start day, paused.
//! Edits a [`momentum_core::RepeatDraft`]; the engine builds it, previews it and saves it.
use adw::prelude::*;
use gettextrs::gettext;
use gtk::glib;
use momentum_core::{MonthlyRule, RepeatCycle};
use std::cell::RefCell;
use std::rc::Rc;

use crate::window::{repeat_text, MomentumWindow};

const CYCLES: [RepeatCycle; 4] = [
    RepeatCycle::Daily,
    RepeatCycle::Weekly,
    RepeatCycle::Monthly,
    RepeatCycle::Yearly,
];

pub fn open(win: &MomentumWindow, task_id: &str) {
    let engine = win.engine();
    let Some(initial) = engine.repeat_draft(task_id.to_string()) else {
        return;
    };
    let existing = initial.existing;
    let draft = Rc::new(RefCell::new(initial));

    let page = adw::PreferencesPage::new();
    let group = adw::PreferencesGroup::builder()
        .description(repeat_text(&engine.describe_repeat_draft(draft.borrow().clone())))
        .build();

    let cycle = adw::ComboRow::builder()
        .title(gettext("Repeats"))
        .model(&gtk::StringList::new(&[
            &gettext("Daily"),
            &gettext("Weekly"),
            &gettext("Monthly"),
            &gettext("Yearly"),
        ]))
        .build();
    cycle.set_selected(CYCLES.iter().position(|c| *c == draft.borrow().cycle).unwrap_or(1) as u32);
    let every = adw::SpinRow::with_range(1.0, 99.0, 1.0);
    every.set_title(&gettext("Every"));
    every.set_value(draft.borrow().every.max(1) as f64);
    group.add(&cycle);
    group.add(&every);

    // Weekday chips (weekly only)
    let days_row = adw::ActionRow::builder().title(gettext("On")).build();
    let chips = gtk::Box::builder().spacing(4).valign(gtk::Align::Center).build();
    let names = [
        gettext("Mon"),
        gettext("Tue"),
        gettext("Wed"),
        gettext("Thu"),
        gettext("Fri"),
        gettext("Sat"),
        gettext("Sun"),
    ];
    let flags = draft.borrow().weekdays.clone(); // Sunday first
    let order = [1usize, 2, 3, 4, 5, 6, 0]; // Monday-first display
    let mut chip_widgets = vec![];
    for (i, &wd) in order.iter().enumerate() {
        let b = gtk::ToggleButton::builder()
            .label(&names[i])
            .active(flags.get(wd).copied().unwrap_or(false))
            .css_classes(["pill", "small"])
            .build();
        chips.append(&b);
        chip_widgets.push((wd, b));
    }
    days_row.add_suffix(&chips);
    group.add(&days_row);

    // Monthly anchor: same date, last day, or the nth weekday (first Monday, last Friday, …)
    let monthly = adw::ComboRow::builder()
        .title(gettext("Monthly on"))
        .model(&gtk::StringList::new(&[
            &gettext("The same date"),
            &gettext("The last day"),
            &gettext("A weekday of the month"),
        ]))
        .build();
    monthly.set_selected(match draft.borrow().monthly {
        MonthlyRule::NthWeekday { .. } => 2,
        MonthlyRule::LastDay => 1,
        _ => 0,
    });
    let nth_week = adw::ComboRow::builder()
        .title(gettext("Which"))
        .model(&gtk::StringList::new(&[
            &gettext("First"),
            &gettext("Second"),
            &gettext("Third"),
            &gettext("Fourth"),
            &gettext("Last"),
        ]))
        .build();
    let long_names = [
        gettext("Sunday"),
        gettext("Monday"),
        gettext("Tuesday"),
        gettext("Wednesday"),
        gettext("Thursday"),
        gettext("Friday"),
        gettext("Saturday"),
    ];
    let nth_day = adw::ComboRow::builder()
        .title(gettext("Weekday"))
        .model(&gtk::StringList::new(
            &long_names.iter().map(String::as_str).collect::<Vec<_>>(),
        ))
        .build();
    {
        let (w, d) = match draft.borrow().monthly {
            MonthlyRule::NthWeekday { week, weekday } => (week, weekday),
            _ => (1, 1),
        };
        nth_week.set_selected(if w == -1 { 4 } else { (w - 1) as u32 });
        nth_day.set_selected(d);
    }
    group.add(&monthly);
    group.add(&nth_week);
    group.add(&nth_day);

    // Start day with calendar popover
    let start_row = adw::ActionRow::builder()
        .title(gettext("Starts"))
        .subtitle(
            draft
                .borrow()
                .start_date
                .as_deref()
                .map(momentum_core::text::day_label)
                .map(|l| crate::window::fmt_day(&l))
                .unwrap_or_default(),
        )
        .build();
    let pick = gtk::MenuButton::builder()
        .icon_name("x-office-calendar-symbolic")
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .tooltip_text(gettext("Pick a start day"))
        .build();
    pick.update_property(&[gtk::accessible::Property::Label(&gettext("Pick a start day"))]);
    let cal = gtk::Calendar::new();
    if let Some((y, m, d)) = draft.borrow().start_date.as_deref().and_then(sp_model::parse_day) {
        if let Ok(dt) = glib::DateTime::from_local(y as i32, m as i32, d as i32, 0, 0, 0.0) {
            cal.select_day(&dt);
        }
    }
    pick.set_popover(Some(&gtk::Popover::builder().child(&cal).build()));
    start_row.add_suffix(&pick);
    group.add(&start_row);

    let paused = adw::SwitchRow::builder()
        .title(gettext("Paused"))
        .subtitle(gettext("Keep the schedule but stop creating tasks"))
        .active(draft.borrow().paused)
        .build();
    group.add(&paused);
    page.add(&group);

    if existing {
        let remove_group = adw::PreferencesGroup::new();
        let remove = gtk::Button::builder()
            .label(gettext("Stop Repeating"))
            .css_classes(["destructive-action"])
            .halign(gtk::Align::Center)
            .build();
        remove_group.add(&remove);
        page.add(&remove_group);
        let task_id = task_id.to_string();
        remove.connect_clicked(glib::clone!(
            #[weak]
            win,
            #[strong]
            task_id,
            move |b| {
                let dialog = b
                    .root()
                    .and_downcast::<gtk::Window>()
                    .and_then(|_| b.ancestor(adw::Dialog::static_type()))
                    .and_downcast::<adw::Dialog>();
                let out = win.engine().stop_repeat(task_id.clone());
                win.apply(out);
                if let Some(d) = dialog {
                    d.close();
                }
            }
        ));
    }

    // Live preview + show/hide cycle-specific rows
    let refresh = {
        let draft = draft.clone();
        let group = group.clone();
        let engine = engine.clone();
        let (cycle, every, days_row, monthly, nth_week, nth_day, start_row, paused, cal) = (
            cycle.clone(),
            every.clone(),
            days_row.clone(),
            monthly.clone(),
            nth_week.clone(),
            nth_day.clone(),
            start_row.clone(),
            paused.clone(),
            cal.clone(),
        );
        let chips = chip_widgets.clone();
        move || {
            let mut d = draft.borrow_mut();
            d.cycle = CYCLES[cycle.selected() as usize];
            d.every = every.value() as u32;
            for (wd, b) in &chips {
                if let Some(slot) = d.weekdays.get_mut(*wd) {
                    *slot = b.is_active();
                }
            }
            d.monthly = if monthly.selected() == 2 {
                MonthlyRule::NthWeekday {
                    week: if nth_week.selected() == 4 {
                        -1
                    } else {
                        nth_week.selected() as i32 + 1
                    },
                    weekday: nth_day.selected(),
                }
            } else if monthly.selected() == 1 {
                MonthlyRule::LastDay
            } else {
                MonthlyRule::SameDay
            };
            d.paused = paused.is_active();
            d.start_date = cal.date().format("%Y-%m-%d").ok().map(|g| g.to_string());
            start_row.set_subtitle(
                &d.start_date
                    .as_deref()
                    .map(momentum_core::text::day_label)
                    .map(|l| crate::window::fmt_day(&l))
                    .unwrap_or_default(),
            );
            days_row.set_visible(d.cycle == RepeatCycle::Weekly);
            let is_monthly = d.cycle == RepeatCycle::Monthly;
            monthly.set_visible(is_monthly);
            nth_week.set_visible(is_monthly && monthly.selected() == 2);
            nth_day.set_visible(is_monthly && monthly.selected() == 2);
            every.set_subtitle(&match d.cycle {
                RepeatCycle::Daily => gettext("days"),
                RepeatCycle::Weekly => gettext("weeks"),
                RepeatCycle::Monthly => gettext("months"),
                RepeatCycle::Yearly => gettext("years"),
            });
            group.set_description(Some(&repeat_text(&engine.describe_repeat_draft(d.clone()))));
        }
    };
    let r = Rc::new(refresh);
    cycle.connect_selected_notify(glib::clone!(
        #[strong]
        r,
        move |_| r()
    ));
    every.connect_value_notify(glib::clone!(
        #[strong]
        r,
        move |_| r()
    ));
    for row in [&monthly, &nth_week, &nth_day] {
        row.connect_selected_notify(glib::clone!(
            #[strong]
            r,
            move |_| r()
        ));
    }
    paused.connect_active_notify(glib::clone!(
        #[strong]
        r,
        move |_| r()
    ));
    cal.connect_day_selected(glib::clone!(
        #[strong]
        r,
        move |_| r()
    ));
    for (_, b) in &chip_widgets {
        b.connect_toggled(glib::clone!(
            #[strong]
            r,
            move |_| r()
        ));
    }
    r();

    let header = adw::HeaderBar::builder()
        .show_start_title_buttons(false)
        .show_end_title_buttons(false)
        .build();
    let cancel = gtk::Button::with_label(&gettext("Cancel"));
    let save = gtk::Button::builder()
        .label(if existing { gettext("Save") } else { gettext("Repeat") })
        .css_classes(["suggested-action"])
        .build();
    header.pack_start(&cancel);
    header.pack_end(&save);
    let tv = adw::ToolbarView::new();
    tv.add_top_bar(&header);
    tv.set_content(Some(&page));
    let dialog = adw::Dialog::builder()
        .title(gettext("Repeat"))
        .content_width(480)
        .content_height(560)
        .child(&tv)
        .build();
    crate::typography::register_interface_root(&dialog);
    dialog.set_default_widget(Some(&save));
    cancel.connect_clicked(glib::clone!(
        #[weak]
        dialog,
        move |_| {
            dialog.close();
        }
    ));
    let task_id = task_id.to_string();
    save.connect_clicked(glib::clone!(
        #[weak]
        win,
        #[weak]
        dialog,
        #[strong]
        draft,
        #[strong]
        task_id,
        move |_| {
            let d = draft.borrow().clone();
            let out = win.engine().save_repeat(task_id.clone(), d);
            let ok = out.changed;
            win.apply(out);
            if ok {
                dialog.close();
            }
        }
    ));
    dialog.present(Some(win));
}
