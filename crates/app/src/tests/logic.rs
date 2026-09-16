// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Pure helpers behind the UI: formatting, parsing, colours, repeat descriptions, the
//! modifier key. They touch glib and gettext, so they run on the GTK thread too.
use super::support::{on_gtk, reset_settings, settings};
use crate::modifier;
use crate::window::{ago_text, fmt_clock, fmt_day, fmt_ms, hex_color, parse_ms, repeat_text};
use gtk::prelude::*;
use momentum_core::text::{day_label, describe_repeat, parse_time, tag_color};
use momentum_core::ClockTime;
use sp_model::*;

#[test]
fn estimates_parse_and_format_both_ways() {
    on_gtk(|| {
        assert_eq!(parse_ms("1h"), Some(3_600_000.0));
        assert_eq!(parse_ms("1h30m"), Some(5_400_000.0));
        assert_eq!(parse_ms("45m"), Some(2_700_000.0));
        assert_eq!(parse_ms("2h 15m"), Some(8_100_000.0));
        assert!(parse_ms("soon").is_none() && parse_ms("").is_none() && parse_ms("h").is_none());
        assert_eq!(fmt_ms(5_400_000.0), "1h 30m");
        assert_eq!(fmt_ms(3_600_000.0), "1h 00m");
        assert_eq!(fmt_ms(600_000.0), "10m");
        assert_eq!(parse_ms(&fmt_ms(8_100_000.0)), Some(8_100_000.0));
    });
}
#[test]
fn relative_days_read_naturally() {
    on_gtk(|| {
        let today = day_number(&today_str()).unwrap();
        let fmt = |d: &str| fmt_day(&day_label(d));
        assert_eq!(fmt(&today_str()), "Today");
        assert_eq!(fmt(&day_str(today + 1)), "Tomorrow");
        assert_eq!(fmt(&day_str(today - 1)), "Yesterday");
        let in_three = fmt(&day_str(today + 3));
        assert!(
            !in_three.contains(' ') && in_three.len() >= 6,
            "a weekday name: {in_three}"
        );
        let far = fmt(&day_str(today + 400));
        assert!(
            far.chars().any(|c| c.is_ascii_digit()) && far.len() > 8,
            "a full date: {far}"
        );
        assert_eq!(fmt("not a day"), "not a day");
    });
}
#[test]
fn clock_times_and_ago_text() {
    on_gtk(|| {
        assert_eq!(fmt_clock(&ClockTime { hour: 15, minute: 30 }), "15:30");
        let now = now_ms();
        assert_eq!(ago_text(now), "just now");
        assert_eq!(ago_text(now - 30_000), "30 seconds ago");
        assert_eq!(ago_text(now - 5 * 60_000), "5 minutes ago");
        assert_eq!(ago_text(now - 2 * 3_600_000), "2 hours ago");
        assert_eq!(ago_text(now - 3 * 86_400_000), "3 days ago");
    });
}
#[test]
fn colours_from_sync_data_become_hex() {
    on_gtk(|| {
        assert_eq!(hex_color("#ff8800").as_deref(), Some("#ff8800"));
        assert_eq!(hex_color("rgb(255, 0, 0)").as_deref(), Some("#ff0000"));
        assert!(hex_color("not-a-colour").is_none());
        let mut g = Tag::new("g");
        assert!(tag_color(&g).is_some(), "new tags carry a theme colour");
        g.color = Some("#123456".into());
        assert_eq!(
            tag_color(&g).as_deref(),
            Some("#123456"),
            "an explicit colour wins over the theme"
        );
    });
}
#[test]
fn repeat_descriptions_are_plain_language() {
    on_gtk(|| {
        let mut c = RepeatCfg {
            repeat_cycle: "DAILY".into(),
            repeat_every: 1,
            ..Default::default()
        };
        assert_eq!(repeat_text(&describe_repeat(&c)), "Repeats daily");
        c.repeat_every = 3;
        assert_eq!(repeat_text(&describe_repeat(&c)), "Repeats every 3 days");
        c = RepeatCfg {
            repeat_cycle: "WEEKLY".into(),
            repeat_every: 1,
            monday: true,
            ..Default::default()
        };
        assert_eq!(repeat_text(&describe_repeat(&c)), "Repeats every Monday");
        c.wednesday = true;
        assert_eq!(repeat_text(&describe_repeat(&c)), "Repeats weekly on Mon, Wed");
        c.repeat_every = 2;
        assert_eq!(repeat_text(&describe_repeat(&c)), "Repeats every 2 weeks on Mon, Wed");
        c = RepeatCfg {
            repeat_cycle: "MONTHLY".into(),
            repeat_every: 1,
            start_date: Some("2026-01-03".into()),
            ..Default::default()
        };
        assert_eq!(repeat_text(&describe_repeat(&c)), "Repeats monthly on the 3rd");
        c.monthly_last_day = true;
        assert_eq!(repeat_text(&describe_repeat(&c)), "Repeats monthly on the last day");
        c.monthly_last_day = false;
        c.monthly_week_of_month = Some(2);
        c.monthly_weekday = Some(2);
        assert_eq!(
            repeat_text(&describe_repeat(&c)),
            "Repeats monthly on the second Tuesday"
        );
        c.monthly_week_of_month = Some(-1);
        assert_eq!(repeat_text(&describe_repeat(&c)), "Repeats monthly on the last Tuesday");
        c = RepeatCfg {
            repeat_cycle: "YEARLY".into(),
            repeat_every: 2,
            start_date: Some("2026-03-01".into()),
            ..Default::default()
        };
        assert!(
            repeat_text(&describe_repeat(&c)).starts_with("Repeats every 2 years on"),
            "{}",
            repeat_text(&describe_repeat(&c))
        );
        c.repeat_every = 1;
        assert!(
            repeat_text(&describe_repeat(&c)).starts_with("Repeats yearly on"),
            "{}",
            repeat_text(&describe_repeat(&c))
        );
    });
}
#[test]
fn times_typed_by_hand_are_forgiving() {
    let t = |hour, minute| Some(ClockTime { hour, minute });
    assert_eq!(parse_time("14:30"), t(14, 30));
    assert_eq!(parse_time("2:30pm"), t(14, 30));
    assert_eq!(parse_time("12am"), t(0, 0));
    assert_eq!(parse_time("0930"), t(9, 30));
    assert_eq!(parse_time("7"), t(7, 0));
    assert_eq!(parse_time("24:00"), None);
    assert_eq!(parse_time("tea time"), None);
}
#[test]
fn modifier_key_maps_to_gtk_tokens_and_labels() {
    on_gtk(|| {
        reset_settings();
        assert_eq!(modifier::token("control"), "<Control>");
        assert_eq!(modifier::token("alt"), "<Alt>");
        assert_eq!(modifier::token("option"), "<Alt>");
        assert_eq!(modifier::token("super"), "<Super>");
        assert_eq!(
            modifier::token("command"),
            if cfg!(target_os = "macos") { "<Meta>" } else { "<Super>" }
        );
        assert_eq!(modifier::token("garbage"), "<Control>");
        assert_eq!(modifier::accel("<Control><Shift>t"), "<Control><Shift>t");
        assert_eq!(modifier::hint("E"), "Ctrl+E");
        settings().set_string("modifier-key", "super").unwrap();
        assert_eq!(modifier::accel("<Control><Shift>t"), "<Super><Shift>t");
        assert_eq!(modifier::accel("F9"), "F9", "keys without the modifier are untouched");
        assert_eq!(modifier::hint("E"), "Super+E");
        assert_eq!(modifier::label(), "Super");
        let choices = modifier::choices();
        assert_eq!(choices.len(), 3);
        assert!(choices.iter().any(|(k, _)| *k == "control"));
        reset_settings();
        assert_eq!(modifier::label(), "Ctrl");
    });
}
#[test]
fn demo_data_is_what_the_screenshots_and_tests_rely_on() {
    let dir = tempfile::tempdir().unwrap();
    let s = momentum_core::demo::store(dir.path().to_path_buf());
    let titles: Vec<&str> = s.state.task.iter().map(|t| t.title.as_str()).collect();
    for want in [
        "Write release notes for 0.1",
        "Dentist appointment",
        "Renew library books",
        "Read two chapters",
    ] {
        assert!(titles.contains(&want), "missing {want}");
    }
    assert_eq!(s.state.task_repeat_cfg.ids, vec!["demo-weekly"]);
    assert!(s.state.tag.iter().any(|g| g.title == "Evening") && s.state.tag.iter().any(|g| g.title == "Morning"));
    assert!(dir.path().join("state.json").exists(), "the demo store is persisted");
    let overdue = s.state.overdue_ids();
    assert_eq!(overdue.len(), 1);
    assert!(s
        .state
        .task
        .iter()
        .any(|t| t.due_with_time.is_some() && t.remind_at.is_some()));
}
