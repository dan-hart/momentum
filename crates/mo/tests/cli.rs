// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! The `mo` command line as a black box against its own data directory. The session bus
//! is pointed at nowhere so a running desktop app is never touched.
use std::path::Path;
use std::process::Command;

struct Mo {
    dir: tempfile::TempDir,
}
impl Mo {
    fn new() -> Self {
        Self {
            dir: tempfile::tempdir().unwrap(),
        }
    }
    fn run(&self, args: &[&str]) -> (bool, String) {
        let out = self.output(args);
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        (out.status.success(), text)
    }
    fn output(&self, args: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_mo"))
            .args(args)
            .env("MO_DATA_DIR", self.dir.path())
            .env("DBUS_SESSION_BUS_ADDRESS", "unix:path=/nonexistent/momentum-test-bus")
            .env("NO_COLOR", "1")
            .output()
            .unwrap()
    }
    fn ok(&self, args: &[&str]) -> String {
        let (ok, text) = self.run(args);
        assert!(ok, "mo {args:?} failed:\n{text}");
        text
    }
    fn json(&self, args: &[&str]) -> serde_json::Value {
        let mut a = vec!["--json"];
        a.extend_from_slice(args);
        serde_json::from_str(&self.ok(&a)).expect("json output")
    }
    fn state(&self) -> serde_json::Value {
        serde_json::from_slice(&std::fs::read(self.dir.path().join("state.json")).unwrap()).unwrap()
    }
}
fn task_titles(v: &serde_json::Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|t| t["title"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn add_parses_tags_estimate_and_scheduling_flags() {
    let mo = Mo::new();
    mo.ok(&["add", "Write", "docs", "#work", "1h30m"]);
    let tasks = mo.json(&["list"]);
    assert_eq!(task_titles(&tasks), ["Write docs"]);
    assert_eq!(tasks[0]["timeEstimate"], 5_400_000.0);
    let tags = mo.json(&["tags"]);
    assert!(tags.as_array().unwrap().iter().any(|t| t["title"] == "work"));
    mo.ok(&["add", "Today thing", "--today"]);
    mo.ok(&["add", "Evening thing", "--tonight"]);
    mo.ok(&["add", "Morning thing", "--morning"]);
    mo.ok(&["add", "Tomorrow thing", "--tomorrow"]);
    mo.ok(&["add", "Dated", "--due", "2030-01-02", "--notes", "some notes"]);
    let today_list = task_titles(&mo.json(&["today"]));
    assert!(today_list.contains(&"Today thing".to_string()) && today_list.contains(&"Evening thing".to_string()));
    assert_eq!(task_titles(&mo.json(&["tonight"])), ["Evening thing"]);
    assert_eq!(task_titles(&mo.json(&["morning"])), ["Morning thing"]);
    assert!(task_titles(&mo.json(&["today"])).contains(&"Morning thing".to_string()));
    let up = task_titles(&mo.json(&["upcoming", "--days", "3"]));
    assert_eq!(
        up,
        ["Tomorrow thing"],
        "upcoming is strictly after today and within the window"
    );
    let all = mo.json(&["list"]);
    let dated = all.as_array().unwrap().iter().find(|t| t["title"] == "Dated").unwrap();
    assert_eq!(dated["dueDay"], "2030-01-02");
    assert_eq!(dated["notes"], "some notes");
}
#[test]
fn done_undone_plan_and_rm_by_prefix_or_title() {
    let mo = Mo::new();
    mo.ok(&["add", "Alpha"]);
    mo.ok(&["add", "Beta"]);
    mo.ok(&["done", "alp"]);
    let is_done = |mo: &Mo, title: &str| {
        let st = mo.state();
        st["task"]["entities"]
            .as_object()
            .unwrap()
            .values()
            .find(|t| t["title"] == title)
            .unwrap()["isDone"]
            == true
    };
    assert!(is_done(&mo, "Alpha"));
    assert_eq!(task_titles(&mo.json(&["list"])), ["Beta"], "list shows open tasks only");
    mo.ok(&["undone", "Alpha"]);
    assert!(!is_done(&mo, "Alpha"));
    mo.ok(&["plan", "Beta"]);
    assert_eq!(task_titles(&mo.json(&["today"])), ["Beta"]);
    let (ok, text) = mo.run(&["done", "a"]);
    assert!(
        !ok || text.to_lowercase().contains("ambiguous") || text.contains("Alpha"),
        "ambiguous fragments must not silently pick one: {text}"
    );
    mo.ok(&["rm", "Beta"]);
    let mut left = task_titles(&mo.json(&["list"]));
    left.sort();
    assert_eq!(left, ["Alpha"]);
    let (ok, _) = mo.run(&["rm", "does-not-exist"]);
    assert!(!ok);
}
#[test]
fn projects_tags_search_and_filters() {
    let mo = Mo::new();
    mo.ok(&["add", "In inbox"]);
    mo.ok(&["add", "Tagged", "#home"]);
    let projects = mo.json(&["projects"]);
    assert!(projects.as_array().unwrap().iter().any(|p| p["title"] == "Inbox"));
    assert_eq!(task_titles(&mo.json(&["list", "#home"])), ["Tagged"]);
    assert_eq!(task_titles(&mo.json(&["list", "Inbox"])).len(), 2);
    assert_eq!(task_titles(&mo.json(&["search", "tagg"])), ["Tagged"]);
    let (ok, text) = mo.run(&["search", "zzz-nothing"]);
    assert!(ok && (text.contains("Nothing") || text.trim().is_empty() || text.trim() == "[]"));
}
#[test]
fn every_change_is_queued_as_a_sync_op_with_this_clients_id() {
    let mo = Mo::new();
    mo.ok(&["add", "One"]);
    mo.ok(&["done", "One"]);
    let pending: serde_json::Value =
        serde_json::from_slice(&std::fs::read(mo.dir.path().join("pending.json")).unwrap()).unwrap();
    let meta: serde_json::Value =
        serde_json::from_slice(&std::fs::read(mo.dir.path().join("meta.json")).unwrap()).unwrap();
    assert_eq!(pending.as_array().unwrap().len(), 2);
    assert_eq!(pending[0]["op"]["a"], "HA");
    assert_eq!(pending[0]["op"]["c"], meta["client_id"]);
    assert_eq!(mo.state()["task"]["ids"].as_array().unwrap().len(), 1);
}
#[test]
fn config_show_reads_the_local_file_and_set_writes_it() {
    let mo = Mo::new();
    mo.ok(&[
        "config",
        "--server",
        "https://cloud.example",
        "--user",
        "me",
        "--folder",
        "sp",
    ]);
    let cfg: serde_json::Value =
        serde_json::from_slice(&std::fs::read(mo.dir.path().join("cli-config.json")).unwrap()).unwrap();
    assert_eq!(cfg["server"], "https://cloud.example");
    assert_eq!(cfg["user"], "me");
    let text = mo.ok(&["config"]);
    assert!(text.contains("cloud.example"));
    let (ok, text) = Mo::new().run(&["sync"]);
    assert!(
        !ok,
        "sync without configuration must fail before accessing credentials: {text}"
    );
}
#[test]
fn help_lists_every_command() {
    let mo = Mo::new();
    let text = mo.ok(&["--help"]);
    for cmd in [
        "add", "today", "morning", "tonight", "upcoming", "list", "search", "done", "undone", "plan", "rm", "projects",
        "tags", "sync", "config", "open",
    ] {
        assert!(text.contains(cmd), "help lacks {cmd}");
    }
    assert!(!mo.dir.path().join("state.json").exists());
}

#[test]
fn help_and_version_use_stdout_without_stderr() {
    let mo = Mo::new();
    for args in [["--help"], ["--version"]] {
        let output = mo.output(&args);
        assert!(output.status.success(), "mo {args:?} failed");
        assert!(!output.stdout.is_empty(), "mo {args:?} produced no stdout");
        assert!(
            output.stderr.is_empty(),
            "mo {args:?} unexpectedly wrote stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

// `mo open` hands the task to the desktop app over D-Bus, which only exists on Linux.
#[cfg(target_os = "linux")]
#[test]
fn open_resolves_current_tasks_but_requires_desktop_acceptance_without_mutating() {
    let mo = Mo::new();
    let added = mo.json(&["add", "Exact reveal target"]);
    let id = added[0]["id"].as_str().unwrap().to_string();
    let state_before = std::fs::read(mo.dir.path().join("state.json")).unwrap();
    let pending_before = std::fs::read(mo.dir.path().join("pending.json")).unwrap();

    for needle in [&id[..8], "REVEAL TARGET"] {
        let (ok, text) = mo.run(&["--json", "open", needle]);
        assert!(!ok, "a missing desktop app must not be reported as accepted: {text}");
        let error: serde_json::Value = serde_json::from_str(&text).expect("JSON error envelope");
        assert!(
            error["error"]
                .as_str()
                .is_some_and(|message| message.contains("Momentum") && message.contains("open")),
            "unexpected open error: {error}"
        );
    }

    assert_eq!(std::fs::read(mo.dir.path().join("state.json")).unwrap(), state_before);
    assert_eq!(
        std::fs::read(mo.dir.path().join("pending.json")).unwrap(),
        pending_before
    );
}

// `mo open` hands the task to the desktop app over D-Bus, which only exists on Linux.
#[cfg(target_os = "linux")]
#[test]
fn open_refuses_custom_data_dirs_without_writing() {
    let mo = Mo::new();
    let added = mo.json(&["add", "Custom store target"]);
    let id = added[0]["id"].as_str().unwrap().to_string();
    let state_before = std::fs::read(mo.dir.path().join("state.json")).unwrap();
    let pending_before = std::fs::read(mo.dir.path().join("pending.json")).unwrap();

    for args in [
        vec!["--json", "open", id.as_str()],
        vec![
            "--json",
            "--data-dir",
            mo.dir.path().to_str().unwrap(),
            "open",
            id.as_str(),
        ],
    ] {
        let (ok, text) = mo.run(&args);
        assert!(!ok, "custom store unexpectedly opened: {text}");
        let error: serde_json::Value = serde_json::from_str(&text).expect("JSON error envelope");
        assert!(
            error["error"]
                .as_str()
                .is_some_and(|message| message.contains("registered Momentum Flatpak profile")),
            "unexpected custom-store error: {error}"
        );
    }

    assert_eq!(std::fs::read(mo.dir.path().join("state.json")).unwrap(), state_before);
    assert_eq!(
        std::fs::read(mo.dir.path().join("pending.json")).unwrap(),
        pending_before
    );
}

// Off Linux there is no desktop app to hand the task to. `open` must still resolve the
// task, then refuse through the same JSON envelope and leave the store untouched.
#[cfg(not(target_os = "linux"))]
#[test]
fn open_is_unavailable_off_linux_and_never_writes() {
    let mo = Mo::new();
    let added = mo.json(&["add", "Exact reveal target"]);
    let id = added[0]["id"].as_str().unwrap().to_string();
    let state_before = std::fs::read(mo.dir.path().join("state.json")).unwrap();
    let pending_before = std::fs::read(mo.dir.path().join("pending.json")).unwrap();

    for args in [
        vec!["--json", "open", &id[..8]],
        vec!["--json", "open", "REVEAL TARGET"],
        vec![
            "--json",
            "--data-dir",
            mo.dir.path().to_str().unwrap(),
            "open",
            id.as_str(),
        ],
    ] {
        let (ok, text) = mo.run(&args);
        assert!(!ok, "open unexpectedly succeeded without a desktop app: {text}");
        let error: serde_json::Value = serde_json::from_str(&text).expect("JSON error envelope");
        assert!(
            error["error"].as_str().is_some_and(|message| message.contains("Linux")),
            "unexpected off-Linux open error: {error}"
        );
    }

    assert_eq!(std::fs::read(mo.dir.path().join("state.json")).unwrap(), state_before);
    assert_eq!(
        std::fs::read(mo.dir.path().join("pending.json")).unwrap(),
        pending_before
    );
}

#[test]
fn open_lookup_errors_use_the_existing_json_error_envelope_without_mutating() {
    let mo = Mo::new();
    mo.ok(&["add", "Ambiguous alpha"]);
    mo.ok(&["add", "Ambiguous beta"]);
    let state_before = std::fs::read(mo.dir.path().join("state.json")).unwrap();
    let pending_before = std::fs::read(mo.dir.path().join("pending.json")).unwrap();

    for (needle, expected) in [("missing", "no task matches"), ("Ambiguous", "2 tasks match")] {
        let (ok, text) = mo.run(&["--json", "open", needle]);
        assert!(!ok, "invalid open request succeeded: {text}");
        let error: serde_json::Value = serde_json::from_str(&text).expect("JSON error envelope");
        assert!(error["error"].as_str().unwrap().contains(expected), "{error}");
    }

    assert_eq!(std::fs::read(mo.dir.path().join("state.json")).unwrap(), state_before);
    assert_eq!(
        std::fs::read(mo.dir.path().join("pending.json")).unwrap(),
        pending_before
    );
}

#[test]
fn open_rejects_archived_deleted_and_stale_ids_without_mutating() {
    let mo = Mo::new();
    let mut store = sp_store::Store::load(mo.dir.path().into());
    let mut archived = sp_model::Task::new("Archived target", sp_model::INBOX_PROJECT_ID);
    archived.id = "archived-target-id".into();
    let mut deleted = sp_model::Task::new("Deleted target", sp_model::INBOX_PROJECT_ID);
    deleted.id = "deleted-target-id".into();
    store.dispatch(sp_oplog::Action::AddTask {
        task: archived.clone(),
        bottom: true,
    });
    store.dispatch(sp_oplog::Action::AddTask {
        task: deleted.clone(),
        bottom: true,
    });
    store.dispatch(sp_oplog::Action::MoveToArchive {
        tasks: vec![archived],
        sub_tasks: vec![],
    });
    store.dispatch(sp_oplog::Action::DeleteTask {
        task: deleted,
        sub_tasks: vec![],
    });
    let state_before = std::fs::read(mo.dir.path().join("state.json")).unwrap();
    let pending_before = std::fs::read(mo.dir.path().join("pending.json")).unwrap();

    for id in ["archived-target-id", "deleted-target-id", "stale-target-id"] {
        let (ok, text) = mo.run(&["--json", "open", id]);
        assert!(!ok, "{id} unexpectedly opened: {text}");
        let error: serde_json::Value = serde_json::from_str(&text).expect("JSON error envelope");
        assert!(error["error"].as_str().unwrap().contains("no task matches"), "{error}");
    }
    assert_eq!(std::fs::read(mo.dir.path().join("state.json")).unwrap(), state_before);
    assert_eq!(
        std::fs::read(mo.dir.path().join("pending.json")).unwrap(),
        pending_before
    );
}

#[test]
fn quoted_and_split_short_syntax_are_equivalent() {
    for args in [
        vec!["add", "Write docs #work 1h 30m"],
        vec!["add", "Write", "docs", "#work", "1h", "30m"],
    ] {
        let mo = Mo::new();
        let tasks = mo.json(&args);
        assert_eq!(tasks[0]["title"], "Write docs");
        assert_eq!(tasks[0]["timeEstimate"], 5_400_000.0);
        assert_eq!(mo.json(&["tags"])[0]["title"], "work");
    }
}

#[test]
fn invalid_dates_and_conflicting_schedule_flags_do_not_write() {
    for args in [
        vec!["add", "Invalid #new", "--due", "2026-02-30"],
        vec!["add", "Invalid", "--due", "2026-13-01"],
        vec!["add", "Invalid", "--due", "2026-1-1"],
        vec!["add", "Invalid", "--due", "2026-01-01-extra"],
        vec!["add", "Invalid", "--today", "--tomorrow"],
        vec!["add", "Invalid", "--morning", "--tonight"],
        vec!["add", "Invalid", "--due", "2026-01-01", "--today"],
        vec!["add", "   "],
        vec!["upcoming", "--days", "0"],
        vec!["upcoming", "--days", "9223372036854775807"],
    ] {
        let mo = Mo::new();
        assert!(!mo.run(&args).0, "accepted {args:?}");
        assert!(!mo.dir.path().join("state.json").exists(), "wrote invalid {args:?}");
    }
}

#[test]
fn mutations_and_config_emit_json() {
    let mo = Mo::new();
    let added = mo.json(&["add", "JSON task"]);
    let id = added[0]["id"].as_str().unwrap();
    for (command, done) in [("done", true), ("undone", false)] {
        let result = mo.json(&[command, id]);
        assert_eq!(result["task"]["isDone"], done);
        assert_eq!(result["task"]["id"], id);
    }
    assert_eq!(mo.json(&["plan", id])["task"]["dueDay"], sp_model::today_str());
    assert_eq!(mo.json(&["rm", id])["task"]["id"], id);
    let config = mo.json(&["config", "--server", "https://example.invalid", "--user", "test"]);
    assert_eq!(config["server"], "https://example.invalid");
    assert_eq!(mo.json(&["config"])["user"], "test");
}

#[test]
fn upcoming_includes_scheduled_tasks_using_their_local_day() {
    use sp_model::*;
    let mo = Mo::new();
    let mut store = sp_store::Store::load(mo.dir.path().into());
    let mut timed = Task::new("Timed tomorrow", INBOX_PROJECT_ID);
    timed.due_with_time = local_ms(&day_str(day_number(&today_str()).unwrap() + 1), 12, 0);
    store.dispatch(sp_oplog::Action::AddTask {
        task: timed,
        bottom: true,
    });
    assert_eq!(task_titles(&mo.json(&["upcoming"])), ["Timed tomorrow"]);
}

#[test]
fn imported_short_and_unicode_ids_print_and_resolve_safely() {
    let mo = Mo::new();
    let mut store = sp_store::Store::load(mo.dir.path().into());
    for (id, title) in [("a", "Same one"), ("ééééé", "Same two"), ("AbC", "Mixed case")] {
        let mut task = sp_model::Task::new(title, sp_model::INBOX_PROJECT_ID);
        task.id = id.into();
        store.dispatch(sp_oplog::Action::AddTask { task, bottom: true });
    }
    assert!(mo.ok(&["list"]).contains("Same one"));
    let (ok, error) = mo.run(&["done", "Same"]);
    assert!(!ok && error.contains("2 tasks match"), "{error}");
    mo.ok(&["done", "AbC"]);
    assert!(!mo.run(&["done", ""]).0);
}

#[test]
fn ambiguous_id_prefix_does_not_fall_back_to_a_matching_title() {
    let mo = Mo::new();
    let mut store = sp_store::Store::load(mo.dir.path().into());
    for (id, title) in [("ab1", "ab"), ("ab2", "Other")] {
        let mut task = sp_model::Task::new(title, sp_model::INBOX_PROJECT_ID);
        task.id = id.into();
        store.dispatch(sp_oplog::Action::AddTask { task, bottom: true });
    }
    assert!(!mo.run(&["done", "ab"]).0);
}

#[test]
fn config_show_does_not_create_or_rewrite_config() {
    let mo = Mo::new();
    mo.ok(&["config"]);
    assert!(!mo.dir.path().join("cli-config.json").exists());
}

#[test]
fn data_directory_environment_and_flag_precedence() {
    let shared = tempfile::tempdir().unwrap();
    let cli = tempfile::tempdir().unwrap();
    let explicit = tempfile::tempdir().unwrap();
    let run = |args: &[&str], mo_dir: Option<&Path>| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_mo"));
        command
            .args(args)
            .env("HOME", shared.path())
            .env_remove("MO_DATA_DIR")
            .env("MOMENTUM_DATA_DIR", shared.path());
        if let Some(dir) = mo_dir {
            command.env("MO_DATA_DIR", dir);
        }
        let out = command.output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    };
    run(&["add", "Shared override"], None);
    assert!(shared.path().join("state.json").exists());
    run(&["add", "CLI override"], Some(cli.path()));
    assert!(cli.path().join("state.json").exists());
    run(
        &["--data-dir", explicit.path().to_str().unwrap(), "add", "Flag override"],
        Some(cli.path()),
    );
    assert!(explicit.path().join("state.json").exists());
}

#[cfg(unix)]
#[test]
fn live_engine_receives_one_tag_and_json_sync_acknowledgment() {
    use momentum_core::{ipc::CliDelegate, Engine};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    struct Delegate(AtomicUsize);
    impl CliDelegate for Delegate {
        fn store_changed(&self) {}
        fn sync_requested(&self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    let mo = Mo::new();
    let engine = Engine::open(mo.dir.path().to_string_lossy().into());
    let delegate = Arc::new(Delegate(AtomicUsize::new(0)));
    engine.clone().serve_cli(delegate.clone()).unwrap();
    let added = mo.json(&["add", "Live", "#fresh", "#fresh"]);
    assert_eq!(added[0]["tagIds"].as_array().unwrap().len(), 1);
    assert_eq!(mo.json(&["tags"]).as_array().unwrap().len(), 1);
    let output = mo.ok(&["add", "Also live", "#another"]);
    assert!(
        output.contains("#another"),
        "forwarded add must print the new tag: {output}"
    );
    assert_eq!(mo.json(&["sync"])["status"], "requested");
    assert_eq!(delegate.0.load(Ordering::SeqCst), 1);
    engine.stop_cli_server();
}

#[cfg(unix)]
#[test]
fn live_rejection_and_timeout_never_fall_back_to_direct_write() {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;
    for (command, reply) in [
        ("add", Some("error: rejected\n")),
        ("sync", Some("error: rejected\n")),
        ("add", None),
    ] {
        let mo = Mo::new();
        let listener = UnixListener::bind(momentum_core::ipc::socket_path(mo.dir.path())).unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut line = String::new();
            BufReader::new(&stream).read_line(&mut line).unwrap();
            if let Some(reply) = reply {
                stream.write_all(reply.as_bytes()).unwrap();
            } else {
                std::thread::sleep(std::time::Duration::from_secs(6));
            }
        });
        let args = if command == "add" {
            vec!["add", "Do not write"]
        } else {
            vec!["sync"]
        };
        let (ok, output) = mo.run(&args);
        server.join().unwrap();
        assert!(!ok, "live app failure must fail: {output}");
        if reply.is_some() {
            assert!(output.contains("rejected"), "{output}");
        }
        assert!(!mo.dir.path().join("state.json").exists());
    }
}

#[test]
fn failed_persistence_exits_unsuccessfully() {
    let mo = Mo::new();
    // Force atomic state write to fail, even when tests run with elevated permissions.
    std::fs::create_dir(mo.dir.path().join("state.json.tmp")).unwrap();
    let (ok, output) = mo.run(&["add", "Cannot save"]);
    assert!(!ok, "save failure was reported as success: {output}");
    assert!(!mo.dir.path().join("pending.json").exists());
}

#[cfg(unix)]
#[test]
fn config_changes_notify_running_app_after_atomic_write() {
    use momentum_core::{ipc::CliDelegate, Engine};
    use std::sync::{Arc, Mutex};
    struct Delegate {
        dir: std::path::PathBuf,
        observed: Mutex<Vec<serde_json::Value>>,
    }
    impl CliDelegate for Delegate {
        fn store_changed(&self) {
            let config = serde_json::from_slice(&std::fs::read(self.dir.join("cli-config.json")).unwrap()).unwrap();
            self.observed.lock().unwrap().push(config);
        }
        fn sync_requested(&self) {}
    }
    let mo = Mo::new();
    let engine = Engine::open(mo.dir.path().to_string_lossy().into());
    let delegate = Arc::new(Delegate {
        dir: mo.dir.path().into(),
        observed: Mutex::new(vec![]),
    });
    engine.clone().serve_cli(delegate.clone()).unwrap();
    mo.json(&["config", "--server", "https://example.invalid"]);
    assert_eq!(
        delegate.observed.lock().unwrap()[0]["server"],
        "https://example.invalid"
    );
    mo.ok(&["config"]);
    assert_eq!(
        delegate.observed.lock().unwrap().len(),
        1,
        "read-only config must not notify"
    );
    engine.stop_cli_server();
}

#[test]
fn printed_ids_are_unique_prefixes_and_exact_ids_win() {
    let mo = Mo::new();
    let mut store = sp_store::Store::load(mo.dir.path().into());
    for (id, title) in [("sameprefix-1", "First"), ("sameprefix-2", "Second"), ("same", "Exact")] {
        let mut task = sp_model::Task::new(title, sp_model::INBOX_PROJECT_ID);
        task.id = id.into();
        store.dispatch(sp_oplog::Action::AddTask { task, bottom: true });
    }
    let printed = mo.ok(&["list"]);
    let ids: Vec<&str> = printed
        .lines()
        .map(|line| line.split_whitespace().nth(2).unwrap())
        .collect();
    assert_ne!(ids[0], ids[1], "printed IDs must distinguish tasks: {printed}");
    for id in ids {
        mo.ok(&["done", id]);
    }
    assert_eq!(mo.json(&["list"]), serde_json::json!([]));
}

#[cfg(unix)]
#[test]
fn live_engine_persistence_failure_is_not_acknowledged_or_retried() {
    use momentum_core::{ipc::CliDelegate, Engine};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    struct Delegate(AtomicUsize);
    impl CliDelegate for Delegate {
        fn store_changed(&self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
        fn sync_requested(&self) {}
    }
    let mo = Mo::new();
    let engine = Engine::open(mo.dir.path().to_string_lossy().into());
    let delegate = Arc::new(Delegate(AtomicUsize::new(0)));
    engine.clone().serve_cli(delegate.clone()).unwrap();
    mo.ok(&["add", "Saved first"]);
    assert_eq!(delegate.0.load(Ordering::SeqCst), 1);
    std::fs::create_dir(mo.dir.path().join("state.json.tmp")).unwrap();
    let (ok, output) = mo.run(&["--json", "add", "Failed save"]);
    assert!(!ok, "running engine must not acknowledge a failed save: {output}");
    let error: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(error["error"]
        .as_str()
        .is_some_and(|s| s.contains("could not persist CLI change")));
    assert_eq!(
        delegate.0.load(Ordering::SeqCst),
        1,
        "no success callback for failed persistence"
    );
    let store = sp_store::Store::load(mo.dir.path().into());
    assert_eq!(store.state.task.ids.len(), 1, "failed task must not appear on disk");
    assert_eq!(store.pending.len(), 1);
    engine.with_store(|store| {
        assert_eq!(
            store.state.task.ids.len(),
            1,
            "failed operation must not be published in memory"
        );
        assert_eq!(
            store.pending.len(),
            1,
            "failure must not dispatch or retain an uncommitted operation"
        );
    });
    engine.stop_cli_server();
}

#[cfg(unix)]
#[test]
fn live_done_honors_auto_archive_and_undo_for_parent_and_children() {
    use momentum_core::{ipc::CliDelegate, Engine, Preferences};
    use sp_model::{Task, INBOX_PROJECT_ID};
    use sp_oplog::Action;
    use std::sync::Arc;
    struct Delegate;
    impl CliDelegate for Delegate {
        fn store_changed(&self) {}
        fn sync_requested(&self) {}
    }
    let mo = Mo::new();
    let engine = Engine::open(mo.dir.path().to_string_lossy().into());
    let parent = Task::new("Parent from CLI", INBOX_PROJECT_ID);
    let child = Task::new("Child", INBOX_PROJECT_ID);
    let parent_id = parent.id.clone();
    let child_id = child.id.clone();
    engine.dispatch(Action::AddTask {
        task: parent,
        bottom: true,
    });
    engine.dispatch(Action::AddSubTask {
        task: child,
        parent_id: parent_id.clone(),
    });
    engine.set_preferences(Preferences {
        auto_archive: true,
        ..Preferences::default()
    });
    engine.clone().serve_cli(Arc::new(Delegate)).unwrap();
    mo.json(&["done", &parent_id]);
    engine.with_store(|store| {
        assert!(!store.state.task.entities.contains_key(&parent_id));
        assert!(!store.state.task.entities.contains_key(&child_id));
    });
    assert!(engine.can_undo());
    assert!(engine.undo().changed);
    engine.with_store(|store| {
        assert!(!store.state.task.entities[&parent_id].is_done);
        assert!(!store.state.task.entities[&child_id].is_done);
        assert_eq!(
            store.state.task.entities[&parent_id].sub_task_ids,
            vec![child_id.clone()]
        );
    });
    // Reopening an existing completed task uses the shared undoable reopen path and
    // must keep the task live even with auto-archive enabled.
    engine.dispatch(Action::UpdateTask {
        id: parent_id.clone(),
        changes: [("isDone".into(), serde_json::json!(true))].into_iter().collect(),
    });
    mo.json(&["undone", &parent_id]);
    engine.with_store(|store| assert!(!store.state.task.entities[&parent_id].is_done));
    assert!(engine.undo().changed);
    engine.with_store(|store| assert!(store.state.task.entities[&parent_id].is_done));
    engine.stop_cli_server();
}

#[test]
fn sync_respects_selected_provider_without_falling_back_to_nextcloud() {
    let mo = Mo::new();
    for (method, expected) in [
        ("off", "turned off"),
        ("libresync", "Open Momentum"),
        ("unknown", "Unknown sync method"),
    ] {
        std::fs::write(
            mo.dir.path().join("cli-config.json"),
            format!(r#"{{"method":"{method}"}}"#),
        )
        .unwrap();
        let (ok, text) = mo.run(&["sync"]);
        assert!(!ok);
        assert!(text.contains(expected), "{text}");
    }
}

#[test]
fn unreadable_recovery_record_returns_a_normal_json_error_and_preserves_data() {
    let mo = Mo::new();
    mo.ok(&["add", "Keep me"]);
    let before = std::fs::read(mo.dir.path().join("state.json")).unwrap();
    std::fs::create_dir(mo.dir.path().join(".momentum-transaction")).unwrap();
    std::fs::write(mo.dir.path().join(".momentum-transaction/manifest.json"), b"{}").unwrap();
    let (ok, output) = mo.run(&["--json", "list"]);
    assert!(!ok);
    let error: serde_json::Value = serde_json::from_str(&output).expect("structured CLI error, not a panic");
    assert!(error["error"].is_string());
    assert_eq!(std::fs::read(mo.dir.path().join("state.json")).unwrap(), before);
}
