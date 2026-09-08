// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Sample data and an in-process screenshot, used to produce the metainfo screenshots
//! reproducibly: `MOMENTUM_DEMO=1 MOMENTUM_SCREENSHOT=/path.png momentum`.
use gtk::prelude::*;
use gtk::{gdk, glib};
use sp_model::*;
use sp_oplog::Action;
use sp_store::Store;

pub fn store(dir: std::path::PathBuf) -> Store {
    let _ = std::fs::remove_dir_all(&dir);
    let mut s = Store::load(dir);
    let (work, home) = (Project::new("Momentum"), Project::new("Home"));
    let (urgent, gnome) = (Tag::new("urgent"), Tag::new("gnome"));
    let (wid, hid, uid, gid) = (work.id.clone(), home.id.clone(), urgent.id.clone(), gnome.id.clone());
    for a in [
        Action::AddProject { project: work },
        Action::AddProject { project: home },
        Action::AddTag { tag: urgent },
        Action::AddTag { tag: gnome },
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
        mk("Book dentist appointment", &hid, 600_000.0, &[], true),
        mk("Plan weekend hike", &hid, 0.0, &[], false),
        mk("Submit Flathub verification", &wid, 900_000.0, &[&gid], false),
    ] {
        s.dispatch(a);
    }
    s
}

/// Render the window to a PNG after it has had a moment to lay out, then quit.
pub fn screenshot(win: &gtk::Window, path: std::path::PathBuf) {
    let win = win.clone();
    glib::timeout_add_seconds_local_once(2, move || {
        let paintable = gtk::WidgetPaintable::new(Some(&win));
        let snapshot = gtk::Snapshot::new();
        let (w, h) = (win.width() as f64, win.height() as f64);
        paintable.snapshot(&snapshot, w, h);
        if let (Some(node), Some(renderer)) = (snapshot.to_node(), win.native().and_then(|n| n.renderer())) {
            let tex = renderer.render_texture(&node, Some(&gtk::graphene::Rect::new(0.0, 0.0, w as f32, h as f32)));
            match tex.save_to_png(&path) {
                Ok(()) => tracing::info!("screenshot saved to {}", path.display()),
                Err(e) => tracing::error!("screenshot failed: {e}"),
            }
        }
        let _ = gdk::Display::default();
        win.application().map(|a| a.quit());
    });
}
