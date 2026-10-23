// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Stable native wording for shared completion-message IDs.
use gettextrs::gettext;
use momentum_core::{DayCompletion, View};

#[derive(Default)]
pub struct Selection {
    key: String,
    pub id: Option<u32>,
    previous: std::collections::HashMap<View, u32>,
}
impl Selection {
    pub fn update(&mut self, view: &View, completion: DayCompletion, day: &str, hour: u8) {
        let key = format!(
            "{view:?}:{completion:?}:{day}:{}",
            if hour < 12 {
                0
            } else if hour < 18 {
                1
            } else {
                2
            }
        );
        if key == self.key {
            return;
        }
        self.key = key;
        let pool = momentum_core::completion_message_ids(view.clone(), completion, u32::from(hour));
        let previous = self.previous.get(view).copied();
        let choices: Vec<_> = pool.iter().copied().filter(|id| Some(*id) != previous).collect();
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0) as usize;
        self.id = if choices.is_empty() {
            pool.first().copied()
        } else {
            Some(choices[seed % choices.len()])
        };
        if let Some(id) = self.id {
            self.previous.insert(view.clone(), id);
        }
    }
}
pub fn local_phase() -> u8 {
    let hour = gtk::glib::DateTime::now_local().map(|d| d.hour()).unwrap_or(12);
    if hour < 12 {
        0
    } else if hour < 18 {
        1
    } else {
        2
    }
}
pub fn message(id: u32) -> String {
    match id {
        1 => gettext("Morning list, cleared. Take a breath."),
        2 => gettext("A little more room in your morning."),
        3 => gettext("Your morning tasks are tucked away."),
        4 => gettext("A calm pause between plans."),
        5 => gettext("Morning essentials, taken care of."),
        6 => gettext("One less thing on your mind."),
        7 => gettext("Your morning list can rest now."),
        8 => gettext("There's space for a slower moment."),
        9 => gettext("Morning plans, all checked off."),
        10 => gettext("That part of the day is settled."),
        11 => gettext("A clear morning list. A little breathing room."),
        12 => gettext("Take a moment before the next thing."),
        13 => gettext("Your morning list has nothing left to ask."),
        14 => gettext("A small finish. A welcome pause."),
        15 => gettext("Morning tasks, wrapped up neatly."),
        16 => gettext("Let the next moment be unhurried."),
        17 => gettext("Your morning checklist is at ease."),
        18 => gettext("You can put this list down for a while."),
        19 => gettext("A little calm after the morning tasks."),
        20 => gettext("Morning list complete. Settle into your day."),
        21 => gettext("All clear for today. Take a breath."),
        22 => gettext("Nothing left on today's list."),
        23 => gettext("You can put the checklist down."),
        24 => gettext("The list is quiet. Enjoy the space."),
        25 => gettext("Today's loose ends are tied up."),
        26 => gettext("A clear list and a little peace of mind."),
        27 => gettext("Everything planned for today is taken care of."),
        28 => gettext("Let the next thing be something you enjoy."),
        29 => gettext("Today's checklist can rest."),
        30 => gettext("There is room to just be for a moment."),
        31 => gettext("All checked off. No need to rush onward."),
        32 => gettext("Your plans for today are settled."),
        33 => gettext("A little less to carry in your head."),
        34 => gettext("Today's list has nothing more to ask."),
        35 => gettext("Take the win. Then take a break."),
        36 => gettext("A quiet finish feels good, too."),
        37 => gettext("The tasks are done. The pause is yours."),
        38 => gettext("Let yourself enjoy an empty checklist."),
        39 => gettext("Today's list is complete. Breathe out."),
        40 => gettext("Rest belongs in the day, too."),
        41 => gettext("The list is clear. The afternoon is yours."),
        42 => gettext("A little afternoon breathing room."),
        43 => gettext("Today's tasks are done. Find a comfortable pace."),
        44 => gettext("You can leave the checklist here for now."),
        45 => gettext("An open afternoon feels rather nice."),
        46 => gettext("Everything on today's list is settled."),
        47 => gettext("A pause in the afternoon, without a task attached."),
        48 => gettext("Make a little room for something enjoyable."),
        49 => gettext("Today's checklist is taking the afternoon off."),
        50 => gettext("A clear list. Maybe a little fresh air?"),
        51 => gettext("The afternoon can move a little more slowly now."),
        52 => gettext("Today's loose ends are behind you."),
        53 => gettext("Nothing left to check off this afternoon."),
        54 => gettext("There's room for a proper pause."),
        55 => gettext("The checklist is done. Let the afternoon unfold."),
        56 => gettext("Today's plans are taken care of. Get comfortable."),
        57 => gettext("A calmer afternoon starts with a clear list."),
        58 => gettext("Leave a little space for yourself today."),
        59 => gettext("The afternoon has a little less on its mind."),
        60 => gettext("All done for today. Enjoy this quieter stretch."),
        61 => gettext("Today's list is done. Let the evening be gentle."),
        62 => gettext("Nothing left on today's checklist. Settle in."),
        63 => gettext("The day can end on a quieter note."),
        64 => gettext("Today's loose ends can stay tied up."),
        65 => gettext("The checklist is resting. You can, too."),
        66 => gettext("All done for today. Make yourself comfortable."),
        67 => gettext("Leave today's list here for the night."),
        68 => gettext("Let the rest of the evening feel a little lighter."),
        69 => gettext("Today's plans are complete. Breathe out slowly."),
        70 => gettext("A clear list for a softer landing."),
        71 => gettext("There's nothing more on today's checklist."),
        72 => gettext("The evening doesn't need another checkmark."),
        73 => gettext("Today's tasks are tucked away."),
        74 => gettext("You can close the list and enjoy the quiet."),
        75 => gettext("The day is complete. A little calm comes next."),
        76 => gettext("Leave a little room for winding down."),
        77 => gettext("Today's checklist has said its goodnight."),
        78 => gettext("A peaceful finish to today's plans."),
        79 => gettext("Nothing left here to hold your attention tonight."),
        80 => gettext("All checked off. Let the day settle."),
        81 => gettext("Evening list, cleared. Take a breath."),
        82 => gettext("Your evening tasks are taken care of."),
        83 => gettext("Nothing left on this evening's checklist."),
        84 => gettext("A little breathing room around your evening plans."),
        85 => gettext("This part of the day is settled."),
        86 => gettext("Your evening list can rest now."),
        87 => gettext("Evening plans, all checked off."),
        88 => gettext("One quieter corner of the day."),
        89 => gettext("The evening checklist has nothing more to ask."),
        90 => gettext("A clear evening list feels good."),
        91 => gettext("Those evening loose ends are tied up."),
        92 => gettext("Take a small pause. This list is complete."),
        93 => gettext("Your evening essentials are tucked away."),
        94 => gettext("A little less to think about later."),
        95 => gettext("The evening list is ready to be put down."),
        96 => gettext("Everything on this list is settled."),
        97 => gettext("Evening checklist complete. Let yourself pause."),
        98 => gettext("This list has reached a comfortable stopping point."),
        99 => gettext("A welcome bit of calm around the evening tasks."),
        100 => gettext("Your evening list is clear. Enjoy the space."),
        _ => gettext("All done for today"),
    }
}

/// Explicit, bounded native celebration; animations are never needed to understand completion.
pub fn celebrate_button() -> gtk::Box {
    use gtk::glib;
    use gtk::prelude::*;
    use std::cell::Cell;
    use std::rc::Rc;
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .halign(gtk::Align::Center)
        .build();
    // The overlay does not participate in measurement: replaying the burst never
    // shifts the completion copy or neighboring task rows.
    let overlay = gtk::Overlay::new();
    let drawing = gtk::DrawingArea::builder()
        .content_width(440)
        .content_height(280)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .can_target(false)
        .visible(false)
        .build();
    drawing.set_accessible_role(gtk::AccessibleRole::Presentation);
    let progress = Rc::new(Cell::new(0.0_f64));
    let frame = progress.clone();
    drawing.set_draw_func(move |_, cr, width, height| {
        let p = frame.get();
        let spread = 1.0 - (1.0 - p).powi(3);
        let fade = (1.0 - p).min(0.65) / 0.65;
        let center = (width as f64 / 2.0, height as f64 / 2.0);
        let colors = [
            (1.0, 0.64, 0.12),
            (0.20, 0.68, 0.85),
            (0.70, 0.46, 0.88),
            (0.95, 0.40, 0.57),
            (0.28, 0.74, 0.55),
        ];
        // A soft expanding halo accompanies the outward confetti shower.
        cr.set_source_rgba(1.0, 0.64, 0.12, fade * 0.35);
        cr.set_line_width(2.0);
        cr.arc(center.0, center.1, 24.0 + spread * 100.0, 0.0, std::f64::consts::TAU);
        cr.stroke().ok();
        for i in 0..52 {
            let angle = i as f64 * 2.39996;
            let radius = spread * (70.0 + (i % 7) as f64 * 17.0);
            let x = center.0 + angle.cos() * radius;
            let y = center.1 + angle.sin() * radius * 0.65 + p * p * 45.0;
            let (r, g, b) = colors[i % colors.len()];
            cr.set_source_rgba(r, g, b, fade);
            cr.save().ok();
            cr.translate(x, y);
            cr.rotate(angle + p * 3.0);
            if i % 5 == 0 {
                for point in 0..10 {
                    let theta = point as f64 * std::f64::consts::PI / 5.0;
                    let radius = if point % 2 == 0 { 7.0 } else { 3.0 };
                    let (x, y) = (theta.cos() * radius, theta.sin() * radius);
                    if point == 0 {
                        cr.move_to(x, y);
                    } else {
                        cr.line_to(x, y);
                    }
                }
                cr.close_path();
            } else {
                cr.rectangle(-3.0, -5.0, 6.0, 10.0);
            }
            cr.fill().ok();
            cr.restore().ok();
        }
    });
    let label = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let icon = gtk::Image::from_icon_name("starred-symbolic");
    icon.set_accessible_role(gtk::AccessibleRole::Presentation);
    label.append(&icon);
    label.append(&gtk::Label::new(Some(&gettext("Celebrate"))));
    let button = gtk::Button::builder()
        .child(&label)
        .css_classes(["suggested-action", "pill"])
        .halign(gtk::Align::Center)
        .height_request(44)
        .build();
    crate::celebration_colors::apply(&button);
    button.connect_clicked(glib::clone!(
        #[weak]
        drawing,
        #[weak]
        button,
        move |_| {
            drawing.set_visible(true);
            let animate = gtk::Settings::default().is_some_and(|s| s.is_gtk_enable_animations());
            if !animate {
                progress.set(0.32);
                drawing.queue_draw();
                return;
            }
            button.set_sensitive(false);
            progress.set(0.0);
            drawing.queue_draw();
            let started = std::time::Instant::now();
            let progress = progress.clone();
            glib::timeout_add_local(
                std::time::Duration::from_millis(16),
                glib::clone!(
                    #[weak]
                    drawing,
                    #[weak]
                    button,
                    #[upgrade_or]
                    glib::ControlFlow::Break,
                    move || {
                        if !gtk::Settings::default().is_some_and(|s| s.is_gtk_enable_animations()) {
                            progress.set(0.32);
                            drawing.queue_draw();
                            button.set_sensitive(true);
                            return glib::ControlFlow::Break;
                        }
                        let p = started.elapsed().as_secs_f64() / 1.2;
                        if p >= 1.0 || drawing.root().is_none() {
                            drawing.set_visible(false);
                            button.set_sensitive(true);
                            return glib::ControlFlow::Break;
                        }
                        progress.set(p);
                        drawing.queue_draw();
                        glib::ControlFlow::Continue
                    }
                ),
            );
        }
    ));
    overlay.set_child(Some(&button));
    overlay.add_overlay(&drawing);
    overlay.set_measure_overlay(&drawing, false);
    overlay.set_clip_overlay(&drawing, false);
    content.append(&overlay);
    content
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn messages_stay_stable_and_follow_completion_scope() {
        let mut selection = Selection::default();
        selection.update(&View::Today, DayCompletion::DayComplete, "2026-10-22", 10);
        let before = selection.id.unwrap();
        assert!((21..=40).contains(&before));
        selection.update(&View::Today, DayCompletion::DayComplete, "2026-10-22", 11);
        assert_eq!(selection.id, Some(before));
        selection.update(&View::Morning, DayCompletion::PeriodComplete, "2026-10-22", 11);
        assert!((1..=20).contains(&selection.id.unwrap()));
        selection.update(&View::Today, DayCompletion::Pending, "2026-10-22", 11);
        assert_eq!(selection.id, None);
        selection.update(&View::Today, DayCompletion::Unplanned, "2026-10-23", 11);
        assert_eq!(selection.id, None);
    }
}
