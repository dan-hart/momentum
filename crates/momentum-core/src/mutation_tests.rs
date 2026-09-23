// SPDX-License-Identifier: GPL-3.0-or-later
//! Durable logical edits: observers see one complete action, never its intermediate operations.
use crate::*;
use sp_model::{Task, INBOX_PROJECT_ID};
use sp_oplog::Action;
use std::sync::{Arc, Mutex};

fn seeded(count: usize) -> (Arc<Engine>, tempfile::TempDir, Vec<String>) {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open(dir.path().to_string_lossy().into_owned());
    let tasks: Vec<_> = (0..count)
        .map(|i| Task::new(&format!("Fixture {i}"), INBOX_PROJECT_ID))
        .collect();
    let ids = tasks.iter().map(|t| t.id.clone()).collect();
    engine.with_store_mut(|store| {
        store.dispatch_batch(tasks.into_iter().map(|task| Action::AddTask { task, bottom: true }))
    });
    (engine, dir, ids)
}
fn snapshot(engine: &Engine) -> serde_json::Value {
    engine.with_store(|s| serde_json::json!({"state": s.state, "pending": s.pending, "meta": s.meta}))
}

#[test]
fn bulk_completion_and_undo_each_publish_one_complete_durable_change() {
    let (engine, dir, ids) = seeded(20);
    let observed = Arc::new(Mutex::new(Vec::new()));
    let records = observed.clone();
    let path = dir.path().to_path_buf();
    engine.lock().change_signal = Some(Arc::new(move || {
        let store = sp_store::Store::try_load(path.clone()).unwrap();
        records
            .lock()
            .unwrap()
            .push(store.state.task.iter().filter(|t| t.is_done).count());
    }));
    assert!(engine.bulk_done(ids).changed);
    assert_eq!(
        *observed.lock().unwrap(),
        [20],
        "nearby sync sees only the committed complete batch"
    );
    assert!(engine.undo().changed);
    assert_eq!(*observed.lock().unwrap(), [20, 0]);
    let reopened = Engine::open(dir.path().to_string_lossy().into_owned());
    assert_eq!(snapshot(&engine), snapshot(&reopened));
    assert_eq!(engine.pending_count(), 60);
    engine.with_store(|s| assert_eq!(s.meta.vector_clock[&s.meta.client_id], 60));
}

#[test]
fn failed_bulk_commit_preserves_live_state_undo_and_disk_without_notifying() {
    let (engine, dir, ids) = seeded(3);
    engine.move_to_tomorrow(ids.clone());
    let before = snapshot(&engine);
    let undo = engine.undo_top();
    let changes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed = changes.clone();
    engine.lock().change_signal = Some(Arc::new(move || {
        observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }));
    std::fs::create_dir(dir.path().join("pending.json.tmp")).unwrap();
    let result = engine.bulk_done(ids);
    assert!(!result.changed, "unsaved changes must not be reported as successful");
    assert!(result.message.is_some(), "explain the failed save");
    assert!(result.undo.is_none());
    assert!(!result.sync_now);
    assert_eq!(snapshot(&engine), before);
    assert_eq!(engine.undo_top(), undo);
    assert_eq!(changes.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(
        snapshot(&Engine::open(dir.path().to_string_lossy().into_owned())),
        before
    );
}

#[test]
fn failed_creation_does_not_publish_an_id_or_orphan_tags() {
    let (engine, dir, _) = seeded(1);
    let before = snapshot(&engine);
    std::fs::create_dir(dir.path().join("pending.json.tmp")).unwrap();
    let result = engine.create_task_with_id(
        TaskDraft {
            title: "Not saved".into(),
            project_id: INBOX_PROJECT_ID.into(),
            due_day: None,
            time: None,
            reminder_minutes_before: None,
            estimate_ms: 0.0,
            notes: String::new(),
            tag_ids: vec![],
            new_tags: vec!["new tag".into()],
        },
        View::Morning,
    );
    assert!(!result.outcome.changed);
    assert!(result.outcome.message.is_some());
    assert!(result.id.is_none());
    assert_eq!(snapshot(&engine), before);
}

#[test]
fn failed_undo_keeps_the_batch_available_for_retry() {
    let (engine, dir, ids) = seeded(3);
    engine.bulk_delete(ids);
    let before = snapshot(&engine);
    let undo = engine.undo_top();
    std::fs::create_dir(dir.path().join("pending.json.tmp")).unwrap();
    assert!(!engine.undo().changed);
    assert_eq!(snapshot(&engine), before);
    assert_eq!(engine.undo_top(), undo);
    std::fs::remove_dir(dir.path().join("pending.json.tmp")).unwrap();
    assert!(engine.undo().changed);
    assert_eq!(engine.with_store(|s| s.state.task.ids.len()), 3);
}

#[test]
fn compound_capture_paste_and_new_tag_each_publish_once() {
    let (engine, dir, ids) = seeded(3);
    let published = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observer = published.clone();
    engine.lock().change_signal = Some(Arc::new(move || {
        observer.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }));
    assert!(
        engine
            .add_task_with_notes("With #tag".into(), Some("Notes".into()), Some("2030-01-01".into()))
            .changed
    );
    assert_eq!(published.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert!(engine.add_from_text("First\nSecond\nThird".into(), View::Today).changed);
    assert_eq!(published.load(std::sync::atomic::Ordering::SeqCst), 2);
    assert!(engine.add_tag_by_name(ids, "another tag".into()).changed);
    assert_eq!(published.load(std::sync::atomic::Ordering::SeqCst), 3);
    assert_eq!(
        snapshot(&engine),
        snapshot(&Engine::open(dir.path().to_string_lossy().into_owned()))
    );
}

#[test]
#[ignore = "manual local latency sample; no timing threshold or real user data"]
fn measure_durable_bulk_edits() {
    let (engine, dir, ids) = seeded(100);
    let start = std::time::Instant::now();
    assert!(engine.bulk_done(ids).changed);
    let completed = start.elapsed();
    let start = std::time::Instant::now();
    assert!(engine.undo().changed);
    let undone = start.elapsed();
    let lines = (0..100).map(|i| format!("Pasted {i}")).collect::<Vec<_>>().join("\n");
    let start = std::time::Instant::now();
    assert!(engine.add_from_text(lines, View::Today).changed);
    println!(
        "100-task completion: {completed:?}; undo: {undone:?}; 100-task paste: {:?}",
        start.elapsed()
    );
    assert_eq!(
        snapshot(&engine),
        snapshot(&Engine::open(dir.path().to_string_lossy().into_owned()))
    );
}

#[test]
fn failed_organization_creation_returns_no_identity_or_live_change() {
    let (engine, dir, _) = seeded(1);
    let before = snapshot(&engine);
    std::fs::create_dir(dir.path().join("pending.json.tmp")).unwrap();
    assert!(engine.add_project("Unsaved project".into()).is_none());
    assert!(engine.add_tag("Unsaved tag".into()).is_none());
    assert_eq!(snapshot(&engine), before);
}

#[test]
fn unchanged_edit_does_not_need_a_writable_store() {
    let (engine, dir, ids) = seeded(1);
    let before = snapshot(&engine);
    std::fs::create_dir(dir.path().join("pending.json.tmp")).unwrap();
    let outcome = engine.save_task(
        ids[0].clone(),
        TaskDraft {
            title: "Fixture 0".into(),
            project_id: INBOX_PROJECT_ID.into(),
            due_day: None,
            time: None,
            reminder_minutes_before: None,
            estimate_ms: 0.0,
            notes: String::new(),
            tag_ids: vec![],
            new_tags: vec![],
        },
    );
    assert!(!outcome.changed);
    assert!(outcome.message.is_none());
    assert_eq!(snapshot(&engine), before);
}

#[test]
fn failed_repeat_creation_preserves_the_cursor_for_retry() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::demo(dir.path().join("demo").to_string_lossy().into_owned());
    let before = snapshot(&engine);
    let blocked = dir.path().join("demo/pending.json.tmp");
    std::fs::create_dir(&blocked).unwrap();
    assert_eq!(engine.spawn_repeats(), 0);
    assert_eq!(snapshot(&engine), before);
    std::fs::remove_dir(&blocked).unwrap();
    assert_eq!(engine.spawn_repeats(), 1);
    assert_eq!(engine.spawn_repeats(), 0);
}

#[test]
fn rejected_capture_and_empty_tag_selection_do_not_create_orphan_tags() {
    let (engine, _directory, _) = seeded(1);
    let before = snapshot(&engine);
    assert!(!engine.add_task("#orphan 30m".into(), View::Today).changed);
    assert_eq!(snapshot(&engine), before, "rejected capture must not create a tag");
    assert!(!engine.add_tag_by_name(vec!["missing".into()], "unused".into()).changed);
    assert_eq!(
        snapshot(&engine),
        before,
        "an empty target selection must not create a tag"
    );
}
