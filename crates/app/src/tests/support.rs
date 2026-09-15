// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! One GTK thread for all tests: GTK is single-threaded, cargo's harness is not. Each test
//! sends a closure to that thread and waits for its result (panics are forwarded).
//!
//! Environment (set by `build-aux/run-tests.sh`): `MOMENTUM_TEST_RESOURCES` points at a
//! compiled `resources.gresource`, `GSETTINGS_SCHEMA_DIR` at compiled schemas, and
//! `GSETTINGS_BACKEND=memory` keeps every setting change inside the test process.
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Mutex, OnceLock};

use crate::application::MomentumApplication;
use crate::window::MomentumWindow;

type Job = Box<dyn FnOnce() + Send>;
static GTK: OnceLock<Mutex<Sender<Job>>> = OnceLock::new();
thread_local! {
    static APP: RefCell<Option<MomentumApplication>> = const { RefCell::new(None) };
}

fn gtk_thread() -> &'static Mutex<Sender<Job>> {
    GTK.get_or_init(|| {
        let (tx, rx) = channel::<Job>();
        std::thread::Builder::new()
            .name("gtk".into())
            .spawn(move || {
                if std::env::var_os("GSETTINGS_BACKEND").is_none() {
                    std::env::set_var("GSETTINGS_BACKEND", "memory");
                }
                gtk::init().expect("GTK needs a display: run tests through build-aux/run-tests.sh");
                adw::init().expect("libadwaita init");
                let path = std::env::var("MOMENTUM_TEST_RESOURCES")
                    .expect("MOMENTUM_TEST_RESOURCES unset: run tests through build-aux/run-tests.sh");
                let res = gio::Resource::load(&path).expect("compiled resources");
                gio::resources_register(&res);
                // The application owns actions and accelerators; NON_UNIQUE keeps it off the
                // session bus so a running Momentum is never contacted.
                let dir = tempfile::tempdir().expect("tempdir").keep();
                crate::demo::store(dir.clone());
                crate::window::set_test_data_dir(Some(dir));
                let app = MomentumApplication::default();
                app.set_flags(app.flags() | gio::ApplicationFlags::NON_UNIQUE);
                app.register(None::<&gio::Cancellable>).expect("register app");
                APP.with(|a| *a.borrow_mut() = Some(app));
                for job in rx {
                    job();
                }
            })
            .expect("spawn gtk thread");
        Mutex::new(tx)
    })
}

/// Runs `f` on the GTK thread and returns its result; a panic inside is re-raised here.
pub fn on_gtk<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    let (tx, rx) = channel();
    let job: Job = Box::new(move || {
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        let _ = tx.send(r);
    });
    gtk_thread()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .send(job)
        .expect("gtk thread alive");
    match rx.recv().expect("gtk thread answered") {
        Ok(v) => v,
        Err(p) => std::panic::resume_unwind(p),
    }
}

/// The shared application (GTK thread only).
pub fn app() -> MomentumApplication {
    APP.with(|a| a.borrow().clone().expect("app on gtk thread"))
}

/// Drains every pending main-loop source (idle callbacks, queued refreshes).
pub fn pump() {
    let ctx = glib::MainContext::default();
    for _ in 0..1000 {
        if !ctx.pending() {
            break;
        }
        ctx.iteration(false);
    }
}
/// Keeps the loop turning for about `ms` milliseconds (debounces, timeouts).
pub fn pump_ms(ms: u64) {
    let ctx = glib::MainContext::default();
    let end = std::time::Instant::now() + std::time::Duration::from_millis(ms);
    while std::time::Instant::now() < end {
        ctx.iteration(false);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}

/// A window over a fresh copy of the demo data in its own directory (GTK thread only).
pub fn demo_window() -> (MomentumWindow, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir").keep();
    crate::demo::store(dir.clone());
    window_in(dir.clone())
}
/// A window over an empty store in its own directory (GTK thread only).
pub fn empty_window() -> (MomentumWindow, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir").keep();
    window_in(dir.clone())
}
fn window_in(dir: PathBuf) -> (MomentumWindow, PathBuf) {
    crate::window::set_test_data_dir(Some(dir.clone()));
    let win = MomentumWindow::new(&app());
    pump();
    (win, dir)
}
/// Settings as the app sees them (memory backend, shared within the process).
pub fn settings() -> gio::Settings {
    gio::Settings::new(*crate::config::APP_ID)
}
/// Resets every key a test may have touched.
pub fn reset_settings() {
    let s = settings();
    for k in [
        "task-sort",
        "sort-direction",
        "upcoming-range",
        "colorful-labels",
        "modifier-key",
        "p2p-enabled",
        "sync-enabled",
        "auto-sync",
        "auto-archive",
    ] {
        s.reset(k);
    }
}
/// The widget's own `visible` flag. Test windows are never presented, so the effective
/// `is_visible()` is always false; this reads what the code set.
pub fn shown(w: &impl IsA<gtk::Widget>) -> bool {
    w.property::<bool>("visible")
}
/// All descendants of a widget, depth first.
pub fn descendants(w: &gtk::Widget) -> Vec<gtk::Widget> {
    let mut out = vec![];
    let mut child = w.first_child();
    while let Some(c) = child {
        out.push(c.clone());
        out.extend(descendants(&c));
        child = c.next_sibling();
    }
    out
}
/// Section headings currently shown in the task list, top to bottom.
pub fn headings(win: &MomentumWindow) -> Vec<String> {
    descendants(win.imp().task_box.upcast_ref())
        .into_iter()
        .filter_map(|w| w.downcast::<gtk::Label>().ok())
        .filter(|l| l.has_css_class("heading"))
        .map(|l| l.label().to_string())
        .collect()
}
/// Task ids rendered as rows, top to bottom (section markers excluded).
pub fn row_ids(win: &MomentumWindow) -> Vec<String> {
    win.imp()
        .rows
        .borrow()
        .iter()
        .filter(|r| !r.is_empty() && !r.contains(':'))
        .cloned()
        .collect()
}
/// The id of the task with this title in the window's store.
pub fn id_of(win: &MomentumWindow, title: &str) -> String {
    win.imp()
        .store
        .borrow()
        .state
        .task
        .iter()
        .find(|t| t.title == title)
        .map(|t| t.id.clone())
        .unwrap_or_else(|| panic!("no task titled {title:?}"))
}
