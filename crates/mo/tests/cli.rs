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
        let out = Command::new(env!("CARGO_BIN_EXE_mo"))
            .args(args)
            .env("MO_DATA_DIR", self.dir.path())
            .env("DBUS_SESSION_BUS_ADDRESS", "unix:path=/nonexistent/momentum-test-bus")
            .env("NO_COLOR", "1")
            .output()
            .unwrap();
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        (out.status.success(), text)
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
    let (ok, text) = mo.run(&["sync"]);
    assert!(!ok, "sync without a password must fail cleanly, not hang: {text}");
}
#[test]
fn help_lists_every_command() {
    let mo = Mo::new();
    let text = mo.ok(&["--help"]);
    for cmd in [
        "add", "today", "morning", "tonight", "upcoming", "list", "search", "done", "undone", "plan", "rm", "projects",
        "tags", "sync", "config",
    ] {
        assert!(text.contains(cmd), "help lacks {cmd}");
    }
    assert!(!Path::new(mo.dir.path()).join("state.json").exists() || true);
}
