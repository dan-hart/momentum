// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Opt-in process-cold GTK timings in the isolated Broadway harness.
use crate::application::MomentumApplication;
use crate::tests::support::pump;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::gio;
use momentum_core::{Engine, View};
use std::{
    env,
    process::Command,
    time::{Duration, Instant},
};

const FIXTURE: &str = "MOMENTUM_GTK_LAUNCH_FIXTURE";
const TITLE: &str = "Momentum launch fixture ";

#[test]
fn launch_fixture_child() {
    let Some(dir) = env::var_os(FIXTURE) else { return };
    assert_eq!(env::var("GSETTINGS_BACKEND").as_deref(), Ok("memory"));
    assert_eq!(env::var("GDK_BACKEND").as_deref(), Ok("broadway"));
    assert_eq!(
        env::var("DBUS_SESSION_BUS_ADDRESS").as_deref(),
        Ok("unix:path=/nonexistent/momentum-tests")
    );
    gtk::init().unwrap();
    adw::init().unwrap();
    let resources = gio::Resource::load(env::var_os("MOMENTUM_TEST_RESOURCES").unwrap()).unwrap();
    gio::resources_register(&resources);
    crate::window::set_test_data_dir(Some(dir.into()));

    let app = MomentumApplication::default();
    app.set_flags(app.flags() | gio::ApplicationFlags::NON_UNIQUE);
    app.register(None::<&gio::Cancellable>).unwrap();
    app.activate();
    let window = app.ensure_window();
    let deadline = Instant::now() + Duration::from_secs(20);
    let ready = loop {
        pump();
        let capture = &window.imp().add_entry;
        // A live row and an editable capture control make the list actionable.
        // Broadway can map GTK widgets without a connected browser frame.
        if capture.is_mapped()
            && capture.is_sensitive()
            && capture.is_editable()
            && has_ready_task(window.imp().task_box.upcast_ref())
        {
            break true;
        }
        if Instant::now() >= deadline {
            break false;
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    assert!(ready, "the GTK Today list never became actionable");
    println!("momentum-launch-ready");
}

fn has_ready_task(widget: &gtk::Widget) -> bool {
    if !widget.is_mapped() {
        return false;
    }
    if let Some(label) = widget.downcast_ref::<gtk::Label>() {
        return label.is_sensitive() && label.text().starts_with(TITLE);
    }
    let mut child = widget.first_child();
    while let Some(current) = child {
        if has_ready_task(&current) {
            return true;
        }
        child = current.next_sibling();
    }
    false
}

#[test]
#[ignore = "opt-in process cold timing; run in the headless Broadway test harness"]
fn measure_actionable_launch() {
    for count in [10, 1000] {
        let root = tempfile::tempdir().unwrap();
        let mut samples = Vec::new();
        for index in 0..5 {
            let fixture = root.path().join(index.to_string());
            let seed = Engine::open_checked(fixture.to_str().unwrap().into()).unwrap();
            let text = (1..=count)
                .map(|number| format!("{TITLE}{number:04}"))
                .collect::<Vec<_>>()
                .join("\n");
            assert!(seed.add_from_text(text, View::Today).changed);
            assert_eq!(seed.all_tasks().len(), count);
            drop(seed);
            let started = Instant::now();
            let child = Command::new(env::current_exe().unwrap())
                .args(["--exact", "tests::launch::launch_fixture_child", "--nocapture"])
                .env(FIXTURE, &fixture)
                .output()
                .unwrap();
            assert!(child.status.success(), "{}", String::from_utf8_lossy(&child.stderr));
            assert!(String::from_utf8_lossy(&child.stdout).contains("momentum-launch-ready"));
            samples.push(started.elapsed().as_secs_f64() * 1000.0);
        }
        println!("GTK/Broadway process-to-actionable {count} tasks, samples (ms) {samples:.1?}");
        samples.sort_by(f64::total_cmp);
        println!(
            "GTK/Broadway process-to-actionable {count} tasks, median {:.1} ms",
            samples[2]
        );
    }
}
