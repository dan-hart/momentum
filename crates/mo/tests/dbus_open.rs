// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart

#![cfg(target_os = "linux")]

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use zbus::blocking::connection;
use zbus::zvariant::{OwnedValue, Value};

const APP_ID: &str = "io.github.dan_hart.Momentum";
const DEV_ID: &str = "io.github.dan_hart.Momentum.Devel";
const APP_PATH: &str = "/io/github/dan_hart/Momentum";
const DEV_PATH: &str = "/io/github/dan_hart/Momentum/Devel";

struct PrivateBus {
    process: Child,
    address: String,
}

impl Drop for PrivateBus {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

fn start_bus(dir: &Path, record: &Path) -> PrivateBus {
    let socket = dir.join("bus.sock");
    let service_dir = dir.join("services");
    std::fs::create_dir(&service_dir).unwrap();
    let helper = std::env::current_exe().unwrap();
    std::fs::write(
        service_dir.join(format!("{APP_ID}.service")),
        format!(
            "[D-BUS Service]\nName={APP_ID}\nExec={} --exact private_dbus_activation_helper --ignored --nocapture\n",
            helper.display()
        ),
    )
    .unwrap();
    let config = dir.join("bus.conf");
    std::fs::write(
        &config,
        format!(
            "<busconfig>\n  <type>session</type>\n  <auth>EXTERNAL</auth>\n  <listen>unix:path={}</listen>\n  <servicedir>{}</servicedir>\n  <policy context=\"default\">\n    <allow own=\"*\"/>\n    <allow send_destination=\"*\"/>\n    <allow receive_sender=\"*\"/>\n  </policy>\n</busconfig>\n",
            socket.display(),
            service_dir.display()
        ),
    )
    .unwrap();
    let address = format!("unix:path={}", socket.display());
    let process = Command::new("dbus-daemon")
        .args(["--nofork", "--nopidfile", "--config-file", config.to_str().unwrap()])
        .env("DBUS_SESSION_BUS_ADDRESS", &address)
        .env("MOMENTUM_DBUS_RECORD", record)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("private dbus-daemon");
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Ok(connection) = connection::Builder::address(address.as_str()).and_then(|builder| builder.build()) {
            drop(connection);
            break;
        }
        assert!(
            Instant::now() < deadline,
            "private dbus-daemon did not accept connections"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    PrivateBus { process, address }
}

#[derive(Clone)]
struct Actions(Arc<Mutex<Vec<String>>>);

#[zbus::interface(name = "org.gtk.Actions")]
impl Actions {
    #[zbus(name = "Activate")]
    fn activate(
        &self,
        action_name: &str,
        parameters: Vec<OwnedValue>,
        _platform_data: std::collections::HashMap<String, OwnedValue>,
    ) -> zbus::fdo::Result<()> {
        if action_name != "open-task" {
            return Err(zbus::fdo::Error::InvalidArgs("unexpected action".into()));
        }
        let id: String = parameters
            .into_iter()
            .next()
            .ok_or_else(|| zbus::fdo::Error::InvalidArgs("missing id".into()))?
            .try_into()
            .map_err(|_| zbus::fdo::Error::InvalidArgs("id is not a string".into()))?;
        self.0.lock().unwrap().push(id);
        Ok(())
    }
}

struct Application;

#[zbus::interface(name = "org.freedesktop.Application")]
impl Application {
    #[zbus(name = "Activate")]
    fn activate(&self, _platform_data: std::collections::HashMap<String, Value<'_>>) {}
}

struct HangingActions;

#[zbus::interface(name = "org.gtk.Actions")]
impl HangingActions {
    #[zbus(name = "Activate")]
    fn activate(
        &self,
        _action_name: &str,
        _parameters: Vec<OwnedValue>,
        _platform_data: std::collections::HashMap<String, OwnedValue>,
    ) -> zbus::fdo::Result<()> {
        std::thread::sleep(Duration::from_secs(4));
        Ok(())
    }
}

fn wait_for(path: &Path) -> String {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Ok(value) = std::fs::read_to_string(path) {
            return value;
        }
        assert!(
            Instant::now() < deadline,
            "cold-activated helper did not record the action"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn profile_data(home: &Path, app_id: &str) -> PathBuf {
    home.join(".var/app").join(app_id).join("data/momentum")
}

fn run_mo_output(address: &str, home: &Path, dir: &Path, id: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_mo"))
        .args(["--json", "--data-dir", dir.to_str().unwrap(), "open", id])
        .env("HOME", home)
        .env("DBUS_SESSION_BUS_ADDRESS", address)
        .output()
        .unwrap()
}

fn run_mo_output_with_test_timeout(address: &str, home: &Path, dir: &Path, id: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_mo"))
        .args(["--json", "--data-dir", dir.to_str().unwrap(), "open", id])
        .env("HOME", home)
        .env("DBUS_SESSION_BUS_ADDRESS", address)
        .env("MO_TEST_DBUS_METHOD_TIMEOUT_MS", "250")
        .output()
        .unwrap()
}

fn run_mo(address: &str, home: &Path, dir: &Path, id: &str) -> serde_json::Value {
    let output = run_mo_output(address, home, dir, id);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn private_bus_serializes_warm_then_cold_gapplication_open() {
    let dir = tempfile::tempdir().unwrap();
    let record = dir.path().join("cold-record");
    let bus = start_bus(dir.path(), &record);
    let home = dir.path().join("home");
    let data = profile_data(&home, APP_ID);
    let mut store = sp_store::Store::load(data.clone());
    for (id, title) in [("warm-task", "Warm target"), ("cold-task", "Cold target")] {
        let mut task = sp_model::Task::new(title, sp_model::INBOX_PROJECT_ID);
        task.id = id.into();
        store.dispatch(sp_oplog::Action::AddTask { task, bottom: true });
    }
    let state_before = std::fs::read(data.join("state.json")).unwrap();
    let pending_before = std::fs::read(data.join("pending.json")).unwrap();
    let received = Arc::new(Mutex::new(vec![]));
    let warm = connection::Builder::address(bus.address.as_str())
        .unwrap()
        .name(APP_ID)
        .unwrap()
        .serve_at(APP_PATH, Actions(received.clone()))
        .unwrap()
        .serve_at(APP_PATH, Application)
        .unwrap()
        .build()
        .unwrap();
    let warm_json = run_mo(&bus.address, &home, &data, "warm-task");
    assert_eq!(warm_json["status"], "requested");
    assert_eq!(warm_json["task"]["id"], "warm-task");
    assert_eq!(*received.lock().unwrap(), ["warm-task"]);

    drop(warm);
    std::thread::sleep(Duration::from_millis(50));
    let cold_json = run_mo(&bus.address, &home, &data, "cold-task");
    assert_eq!(cold_json["status"], "requested");
    assert_eq!(cold_json["task"]["id"], "cold-task");
    assert_eq!(wait_for(&record), "cold-task");
    assert_eq!(std::fs::read(data.join("state.json")).unwrap(), state_before);
    assert_eq!(std::fs::read(data.join("pending.json")).unwrap(), pending_before);
}

#[test]
fn coexisting_profiles_receive_only_their_own_store_request() {
    let dir = tempfile::tempdir().unwrap();
    let record = dir.path().join("unused-cold-record");
    let bus = start_bus(dir.path(), &record);
    let home = dir.path().join("home");
    let stable_data = profile_data(&home, APP_ID);
    let devel_data = profile_data(&home, DEV_ID);
    for (data, id) in [(&stable_data, "stable-task"), (&devel_data, "devel-task")] {
        let mut store = sp_store::Store::load(data.clone());
        let mut task = sp_model::Task::new(id, sp_model::INBOX_PROJECT_ID);
        task.id = id.into();
        store.dispatch(sp_oplog::Action::AddTask { task, bottom: true });
    }
    let stable_received = Arc::new(Mutex::new(vec![]));
    let devel_received = Arc::new(Mutex::new(vec![]));
    let _stable = connection::Builder::address(bus.address.as_str())
        .unwrap()
        .name(APP_ID)
        .unwrap()
        .serve_at(APP_PATH, Actions(stable_received.clone()))
        .unwrap()
        .serve_at(APP_PATH, Application)
        .unwrap()
        .build()
        .unwrap();
    let _devel = connection::Builder::address(bus.address.as_str())
        .unwrap()
        .name(DEV_ID)
        .unwrap()
        .serve_at(DEV_PATH, Actions(devel_received.clone()))
        .unwrap()
        .serve_at(DEV_PATH, Application)
        .unwrap()
        .build()
        .unwrap();

    assert_eq!(
        run_mo(&bus.address, &home, &devel_data, "devel-task")["status"],
        "requested"
    );
    assert_eq!(*stable_received.lock().unwrap(), Vec::<String>::new());
    assert_eq!(*devel_received.lock().unwrap(), ["devel-task"]);

    assert_eq!(
        run_mo(&bus.address, &home, &stable_data, "stable-task")["status"],
        "requested"
    );
    assert_eq!(*stable_received.lock().unwrap(), ["stable-task"]);
    assert_eq!(*devel_received.lock().unwrap(), ["devel-task"]);
}

#[test]
fn private_bus_nonreply_times_out_without_writing() {
    // MO_TEST_DBUS_METHOD_TIMEOUT_MS is a debug-only hook (crates/mo/src/lib.rs). A release
    // build keeps the real 5 s timeout, so the hanging service below answers first and the
    // test would be checking nothing; the release Flatpak build runs the suite in release.
    if !cfg!(debug_assertions) {
        eprintln!("skipped: the test timeout hook is compiled out of release builds");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let record = dir.path().join("unused-cold-record");
    let bus = start_bus(dir.path(), &record);
    let home = dir.path().join("home");
    let data = profile_data(&home, APP_ID);
    let mut store = sp_store::Store::load(data.clone());
    let mut task = sp_model::Task::new("Hanging target", sp_model::INBOX_PROJECT_ID);
    task.id = "hanging-task".into();
    store.dispatch(sp_oplog::Action::AddTask { task, bottom: true });
    let state_before = std::fs::read(data.join("state.json")).unwrap();
    let pending_before = std::fs::read(data.join("pending.json")).unwrap();
    let _service = connection::Builder::address(bus.address.as_str())
        .unwrap()
        .name(APP_ID)
        .unwrap()
        .serve_at(APP_PATH, HangingActions)
        .unwrap()
        .serve_at(APP_PATH, Application)
        .unwrap()
        .build()
        .unwrap();

    let started = Instant::now();
    let output = run_mo_output_with_test_timeout(&bus.address, &home, &data, "hanging-task");

    assert!(!output.status.success(), "nonreply was reported as success");
    assert!(
        started.elapsed() < Duration::from_millis(750),
        "test timeout injection was not honored"
    );
    assert!(output.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).expect("JSON error envelope");
    assert!(error["error"].as_str().unwrap().contains("open request"), "{error}");
    assert_eq!(std::fs::read(data.join("state.json")).unwrap(), state_before);
    assert_eq!(std::fs::read(data.join("pending.json")).unwrap(), pending_before);
}

#[test]
#[ignore = "launched only by the private dbus-daemon activation test"]
fn private_dbus_activation_helper() {
    let record = PathBuf::from(std::env::var_os("MOMENTUM_DBUS_RECORD").expect("record path"));
    let address = std::env::var("DBUS_SESSION_BUS_ADDRESS").expect("private bus address");
    let received = Arc::new(Mutex::new(vec![]));
    let _service = connection::Builder::address(address.as_str())
        .unwrap()
        .name(APP_ID)
        .unwrap()
        .serve_at(APP_PATH, Actions(received.clone()))
        .unwrap()
        .serve_at(APP_PATH, Application)
        .unwrap()
        .build()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(id) = received.lock().unwrap().first().cloned() {
            std::fs::write(&record, id).unwrap();
            return;
        }
        assert!(Instant::now() < deadline, "helper timed out waiting for open-task");
        std::thread::sleep(Duration::from_millis(10));
    }
}
