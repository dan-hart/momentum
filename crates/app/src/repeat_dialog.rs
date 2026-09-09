// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Create or edit a task's repeat schedule: cycle, interval, weekdays, start day, paused.
use adw::prelude::*;
use adw::subclass::prelude::ObjectSubclassIsExt;
use gettextrs::gettext;
use gtk::glib;
use sp_model::*;
use sp_oplog::Action;
use std::cell::RefCell;
use std::rc::Rc;

use crate::window::{repeat_text, MomentumWindow};

const CYCLES: [&str; 4] = ["DAILY", "WEEKLY", "MONTHLY", "YEARLY"];

pub fn open(win: &MomentumWindow, task_id: &str) {
    let (task, existing) = {
        let store = win.imp().store.borrow();
        let Some(t) = store.state.task.entities.get(task_id).cloned() else {
            return;
        };
        let cfg = t
            .repeat_cfg_id
            .as_ref()
            .and_then(|id| store.state.task_repeat_cfg.entities.get(id).cloned());
        (t, cfg)
    };
    let cfg = Rc::new(RefCell::new(
        existing.clone().unwrap_or_else(|| RepeatCfg::for_task(&task)),
    ));

    let page = adw::PreferencesPage::new();
    let group = adw::PreferencesGroup::builder()
        .description(repeat_text(&cfg.borrow()))
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
    cycle.set_selected(CYCLES.iter().position(|c| *c == cfg.borrow().repeat_cycle).unwrap_or(1) as u32);
    let every = adw::SpinRow::with_range(1.0, 99.0, 1.0);
    every.set_title(&gettext("Every"));
    every.set_value(cfg.borrow().repeat_every.max(1) as f64);
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
    let flags = cfg.borrow().weekdays(); // Sunday first
    let order = [1usize, 2, 3, 4, 5, 6, 0]; // Monday-first display
    let mut chip_widgets = vec![];
    for (i, &wd) in order.iter().enumerate() {
        let b = gtk::ToggleButton::builder()
            .label(&names[i])
            .active(flags[wd])
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
    monthly.set_selected(if cfg.borrow().nth_weekday_anchor().is_some() {
        2
    } else if cfg.borrow().monthly_last_day {
        1
    } else {
        0
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
        let (w, d) = cfg.borrow().nth_weekday_anchor().unwrap_or((1, 1));
        nth_week.set_selected(if w == -1 { 4 } else { (w - 1) as u32 });
        nth_day.set_selected(d);
    }
    group.add(&monthly);
    group.add(&nth_week);
    group.add(&nth_day);

    // Start day with calendar popover
    let start_row = adw::ActionRow::builder()
        .title(gettext("Starts"))
        .subtitle(cfg.borrow().start_date.clone().unwrap_or_default())
        .build();
    let pick = gtk::MenuButton::builder()
        .icon_name("x-office-calendar-symbolic")
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build();
    let cal = gtk::Calendar::new();
    if let Some((y, m, d)) = cfg.borrow().start_date.as_deref().and_then(parse_day) {
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
        .active(cfg.borrow().is_paused)
        .build();
    group.add(&paused);
    page.add(&group);

    if existing.is_some() {
        let remove_group = adw::PreferencesGroup::new();
        let remove = gtk::Button::builder()
            .label(gettext("Stop Repeating"))
            .css_classes(["destructive-action"])
            .halign(gtk::Align::Center)
            .build();
        remove_group.add(&remove);
        page.add(&remove_group);
        let id = cfg.borrow().id.clone();
        let task_title = task.title.clone();
        remove.connect_clicked(glib::clone!(
            #[weak]
            win,
            move |b| {
                let dialog = b
                    .root()
                    .and_downcast::<gtk::Window>()
                    .and_then(|_| b.ancestor(adw::Dialog::static_type()))
                    .and_downcast::<adw::Dialog>();
                win.dispatch(Action::DeleteRepeatCfg { id: id.clone() });
                win.toast(&format!("“{task_title}” {}", gettext("no longer repeats")));
                if let Some(d) = dialog {
                    d.close();
                }
            }
        ));
    }

    // Live preview + show/hide cycle-specific rows
    let refresh = {
        let cfg = cfg.clone();
        let group = group.clone();
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
            let mut c = cfg.borrow_mut();
            c.repeat_cycle = CYCLES[cycle.selected() as usize].into();
            c.repeat_every = every.value() as u32;
            for (wd, b) in &chips {
                let on = b.is_active();
                match wd {
                    0 => c.sunday = on,
                    1 => c.monday = on,
                    2 => c.tuesday = on,
                    3 => c.wednesday = on,
                    4 => c.thursday = on,
                    5 => c.friday = on,
                    _ => c.saturday = on,
                }
            }
            c.monthly_last_day = monthly.selected() == 1;
            if monthly.selected() == 2 {
                c.monthly_week_of_month = Some(if nth_week.selected() == 4 {
                    -1
                } else {
                    nth_week.selected() as i32 + 1
                });
                c.monthly_weekday = Some(nth_day.selected());
            } else {
                c.monthly_week_of_month = None;
                c.monthly_weekday = None;
            }
            c.is_paused = paused.is_active();
            c.start_date = cal.date().format("%Y-%m-%d").ok().map(|g| g.to_string());
            start_row.set_subtitle(&c.start_date.as_deref().map(crate::window::fmt_day).unwrap_or_default());
            days_row.set_visible(c.repeat_cycle == "WEEKLY");
            let is_monthly = c.repeat_cycle == "MONTHLY";
            monthly.set_visible(is_monthly);
            nth_week.set_visible(is_monthly && monthly.selected() == 2);
            nth_day.set_visible(is_monthly && monthly.selected() == 2);
            every.set_subtitle(&match c.repeat_cycle.as_str() {
                "DAILY" => gettext("days"),
                "WEEKLY" => gettext("weeks"),
                "MONTHLY" => gettext("months"),
                _ => gettext("years"),
            });
            group.set_description(Some(&repeat_text(&c)));
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
        .label(if existing.is_some() {
            gettext("Save")
        } else {
            gettext("Repeat")
        })
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
    dialog.set_default_widget(Some(&save));
    cancel.connect_clicked(glib::clone!(
        #[weak]
        dialog,
        move |_| {
            dialog.close();
        }
    ));
    let is_new = existing.is_none();
    let task_id = task.id.clone();
    save.connect_clicked(glib::clone!(
        #[weak]
        win,
        #[weak]
        dialog,
        #[strong]
        cfg,
        move |_| {
            let c = cfg.borrow().clone();
            if c.repeat_cycle == "WEEKLY" && !c.weekdays().iter().any(|d| *d) {
                win.toast(&gettext("Pick at least one weekday"));
                return;
            }
            if is_new {
                win.dispatch(Action::AddRepeatCfg {
                    task_id: task_id.clone(),
                    cfg: c.clone(),
                });
            } else {
                let mut changes = serde_json::to_value(&c)
                    .ok()
                    .and_then(|v| v.as_object().cloned())
                    .unwrap_or_default();
                // Fields serde skips when None must still be cleared on the other side.
                for key in ["monthlyWeekOfMonth", "monthlyWeekday"] {
                    changes.entry(key).or_insert(serde_json::Value::Null);
                }
                win.dispatch(Action::UpdateRepeatCfg {
                    id: c.id.clone(),
                    changes,
                });
            }
            win.toast(&repeat_text(&c));
            dialog.close();
        }
    ));
    dialog.present(Some(win));
}
