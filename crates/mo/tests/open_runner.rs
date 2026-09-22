// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart

use mo::{run_with, OpenActivator};
use sp_model::{Task, INBOX_PROJECT_ID};
use sp_oplog::Action;
use std::path::Path;
use std::sync::Mutex;

#[derive(Default)]
struct RecordingActivator {
    result: Option<String>,
    ids: Mutex<Vec<String>>,
    data_dirs: Mutex<Vec<std::path::PathBuf>>,
}

impl RecordingActivator {
    fn failing(message: &str) -> Self {
        Self {
            result: Some(message.into()),
            ids: Mutex::new(vec![]),
            data_dirs: Mutex::new(vec![]),
        }
    }
}

impl OpenActivator for RecordingActivator {
    fn open_task(&self, data_dir: &Path, task_id: &str) -> Result<(), String> {
        self.data_dirs.lock().unwrap().push(data_dir.into());
        self.ids.lock().unwrap().push(task_id.into());
        self.result.clone().map_or(Ok(()), Err)
    }
}

fn run(dir: &Path, needle: &str, activator: &dyn OpenActivator) -> (i32, String, String) {
    let mut stdout = vec![];
    let mut stderr = vec![];
    let status = run_with(
        ["mo", "--json", "--data-dir", dir.to_str().unwrap(), "open", needle],
        activator,
        &mut stdout,
        &mut stderr,
    );
    (
        status,
        String::from_utf8(stdout).unwrap(),
        String::from_utf8(stderr).unwrap(),
    )
}

fn task(dir: &Path, id: &str, title: &str) -> Task {
    let mut store = sp_store::Store::load(dir.into());
    let mut task = Task::new(title, INBOX_PROJECT_ID);
    task.id = id.into();
    store.dispatch(Action::AddTask {
        task: task.clone(),
        bottom: true,
    });
    task
}

#[test]
fn accepted_open_has_exact_json_and_does_not_mutate_the_store() {
    let dir = tempfile::tempdir().unwrap();
    task(dir.path(), "stable-task-id", "Reveal Me");
    let task = sp_store::Store::load(dir.path().into()).state.task.entities["stable-task-id"].clone();
    let state_before = std::fs::read(dir.path().join("state.json")).unwrap();
    let pending_before = std::fs::read(dir.path().join("pending.json")).unwrap();
    let activator = RecordingActivator::default();

    let (status, stdout, stderr) = run(dir.path(), "stable-task", &activator);

    assert_eq!(status, 0);
    assert_eq!(stderr, "");
    assert_eq!(
        stdout,
        format!("{}\n", serde_json::json!({"status":"requested", "task":task}))
    );
    assert_eq!(*activator.ids.lock().unwrap(), ["stable-task-id"]);
    assert_eq!(*activator.data_dirs.lock().unwrap(), [dir.path()]);
    assert_eq!(std::fs::read(dir.path().join("state.json")).unwrap(), state_before);
    assert_eq!(std::fs::read(dir.path().join("pending.json")).unwrap(), pending_before);
}

#[test]
fn title_lookup_is_case_insensitive_and_rejection_is_not_reported_as_success() {
    let dir = tempfile::tempdir().unwrap();
    task(dir.path(), "stable-task-id", "Reveal Me");
    let activator = RecordingActivator::failing("desktop rejected the open request");

    let (status, stdout, stderr) = run(dir.path(), "rEvEaL mE", &activator);

    assert_eq!(status, 1);
    assert_eq!(stdout, "");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&stderr).unwrap(),
        serde_json::json!({"error":"desktop rejected the open request"})
    );
    assert_eq!(*activator.ids.lock().unwrap(), ["stable-task-id"]);
}

#[test]
fn activation_timeout_disappearance_and_malformed_ack_are_errors() {
    for message in [
        "desktop activation timed out",
        "Momentum disappeared during activation",
        "desktop returned a malformed acknowledgement",
    ] {
        let dir = tempfile::tempdir().unwrap();
        task(dir.path(), "stable-task-id", "Reveal Me");
        let (status, stdout, stderr) = run(dir.path(), "stable-task-id", &RecordingActivator::failing(message));
        assert_eq!(status, 1);
        assert_eq!(stdout, "");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&stderr).unwrap()["error"],
            message
        );
        assert!(!stderr.contains("requested"));
    }
}

#[test]
fn missing_ambiguous_archived_and_deleted_tasks_never_reach_activation() {
    let dir = tempfile::tempdir().unwrap();
    let first = task(dir.path(), "first-id", "Same title first");
    task(dir.path(), "second-id", "Same title second");
    let archived = task(dir.path(), "archived-id", "Archived target");
    let deleted = task(dir.path(), "deleted-id", "Deleted target");
    let mut store = sp_store::Store::load(dir.path().into());
    store.dispatch(Action::MoveToArchive {
        tasks: vec![archived],
        sub_tasks: vec![],
    });
    store.dispatch(Action::DeleteTask {
        task: deleted,
        sub_tasks: vec![],
    });
    let activator = RecordingActivator::default();

    for (needle, expected) in [
        ("missing-id", "no task matches"),
        ("Same title", "2 tasks match"),
        ("archived-id", "no task matches"),
        ("deleted-id", "no task matches"),
        ("stale-id", "no task matches"),
    ] {
        let (status, stdout, stderr) = run(dir.path(), needle, &activator);
        assert_eq!(status, 1, "{needle}: {stdout}{stderr}");
        assert_eq!(stdout, "");
        assert!(stderr.contains(expected), "{needle}: {stderr}");
    }
    assert!(activator.ids.lock().unwrap().is_empty());
    assert!(first.id.starts_with("first"));
}
