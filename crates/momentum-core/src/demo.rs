// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Sample data for screenshots, previews and tests: two projects, four tags, a day's worth
//! of tasks in every slot, an overdue one, a timed one with a reminder, upcoming days and
//! a weekly repeat.
use sp_model::*;
use sp_oplog::Action;
use sp_store::Store;

pub fn store(dir: std::path::PathBuf) -> Store {
    let _ = std::fs::remove_dir_all(&dir);
    let mut s = Store::load(dir);
    let (work, home) = (Project::new("Momentum"), Project::new("Home"));
    let (urgent, gnome, evening, morning) = (
        Tag::new("urgent"),
        Tag::new("gnome"),
        Tag::new("Evening"),
        Tag::new("Morning"),
    );
    let (wid, hid, uid, gid, eid, mid) = (
        work.id.clone(),
        home.id.clone(),
        urgent.id.clone(),
        gnome.id.clone(),
        evening.id.clone(),
        morning.id.clone(),
    );
    for a in [
        Action::AddProject { project: work },
        Action::AddProject { project: home },
        Action::AddTag { tag: urgent },
        Action::AddTag { tag: gnome },
        Action::AddTag { tag: evening },
        Action::AddTag { tag: morning },
    ] {
        s.dispatch(a);
    }
    let today = today_str();
    let mk = |title: &str, project: &str, est: f64, tags: &[&str], due: bool| {
        let mut t = Task::new(title, project);
        t.time_estimate = est;
        t.tag_ids = tags.iter().map(|x| x.to_string()).collect();
        if due {
            t.due_day = Some(today.clone());
        }
        Action::AddTask { task: t, bottom: true }
    };
    for a in [
        mk("Review sync conflict handling", &wid, 3_600_000.0, &[&uid], true),
        mk("Write release notes for 0.1", &wid, 1_800_000.0, &[], true),
        mk("Test with Orca and high contrast", &wid, 2_700_000.0, &[&gid], true),
        {
            let mut t = Task::new("Dentist appointment", &hid);
            t.time_estimate = 3_600_000.0;
            t.due_with_time = local_ms(&today, 15, 30);
            t.remind_at = t.due_with_time.map(|ms| ms - 30 * 60_000);
            Action::AddTask { task: t, bottom: true }
        },
        {
            let mut t = Task::new("Renew library books", &hid);
            t.due_day = Some(day_str(day_number(&today).unwrap_or(0) - 2));
            Action::AddTask { task: t, bottom: true }
        },
        mk("Stretch and plan the day", &hid, 600_000.0, &[&mid], true),
        mk("Read two chapters", &hid, 1_800_000.0, &[&eid], true),
        mk("Prep tomorrow's lunch", &hid, 900_000.0, &[&eid], true),
        mk("Plan weekend hike", &hid, 0.0, &[], false),
        mk("Submit Flathub verification", &wid, 900_000.0, &[&gid], false),
    ] {
        s.dispatch(a);
    }
    // Upcoming tasks and a weekly repeat, so Coming Up and the repeat badge have something to show.
    let plus = |n: i64| day_str(day_number(&today).unwrap() + n);
    for (title, days) in [
        ("Renew passport", 1),
        ("Team retrospective", 1),
        ("Water the plants", 3),
        ("Pay rent", 6),
    ] {
        let mut t = Task::new(title, &hid);
        t.due_day = Some(plus(days));
        s.dispatch(Action::AddTask { task: t, bottom: true });
    }
    let mut cfg = RepeatCfg {
        id: "demo-weekly".into(),
        project_id: Some(wid.clone()),
        title: Some("Weekly planning".into()),
        repeat_cycle: "WEEKLY".into(),
        repeat_every: 1,
        start_date: Some("2026-01-05".into()),
        default_estimate: Some(1_800_000.0),
        ..Default::default()
    };
    let wd = weekday(day_number(&today).unwrap()) as usize;
    let flags = [
        &mut cfg.sunday,
        &mut cfg.monday,
        &mut cfg.tuesday,
        &mut cfg.wednesday,
        &mut cfg.thursday,
        &mut cfg.friday,
        &mut cfg.saturday,
    ];
    *flags[wd] = true;
    s.state.task_repeat_cfg.insert("demo-weekly", cfg);
    s.save().ok();
    s
}
