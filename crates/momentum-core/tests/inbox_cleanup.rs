// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
use momentum_core::momentum_model::{Task, INBOX_PROJECT_ID};
use momentum_core::momentum_oplog::Action;
use momentum_core::{Engine, Message};

#[test]
fn cleanup_moves_links_and_adds_existing_tags_once_with_one_undo() {
    let directory = tempfile::tempdir().unwrap();
    let engine = Engine::open(directory.path().to_string_lossy().into_owned());
    let mut task = Task::new(
        "  read  Rust guide (https://example.org/rust-guide), again!!!  ",
        INBOX_PROJECT_ID,
    );
    task.notes = Some("Notes to keep".into());
    task.due_day = Some("2030-01-02".into());
    let id = task.id.clone();
    engine.dispatch(Action::AddTask { task, bottom: true });
    engine.add_tag("Rust".into());
    let rust = engine.tags().into_iter().find(|tag| tag.title == "Rust").unwrap().id;
    let changes = engine.preview_inbox_cleanup();
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].title, "Read Rust guide, again!");
    assert_eq!(changes[0].links, ["https://example.org/rust-guide"]);
    assert_eq!(changes[0].tag_ids, [rust.clone()]);
    let out = engine.apply_inbox_cleanup(changes);
    assert_eq!(out.message, Some(Message::InboxCleaned));
    let detail = engine.task_detail(id.clone()).unwrap();
    assert_eq!(detail.notes, "Notes to keep\n\nhttps://example.org/rust-guide");
    assert_eq!(detail.tag_ids, [rust]);
    assert!(!engine.apply_inbox_cleanup(engine.preview_inbox_cleanup()).changed);
    assert!(engine.undo_batch(out.undo.unwrap()).changed);
    let detail = engine.task_detail(id).unwrap();
    assert_eq!(detail.notes, "Notes to keep");
    assert_eq!(detail.due_day.as_deref(), Some("2030-01-02"));
    assert!(detail.tag_ids.is_empty());
}

#[test]
fn stale_inbox_review_leaves_every_selected_task_untouched() {
    let directory = tempfile::tempdir().unwrap();
    let engine = Engine::open(directory.path().to_string_lossy().into_owned());
    let one = Task::new("first task", INBOX_PROJECT_ID);
    let two = Task::new("second task", INBOX_PROJECT_ID);
    let id_one = one.id.clone();
    let id_two = two.id.clone();
    for task in [one, two] {
        engine.dispatch(Action::AddTask { task, bottom: true });
    }
    let preview = engine.preview_inbox_cleanup();
    engine.dispatch(Action::UpdateTask {
        id: id_two.clone(),
        changes: serde_json::Map::from_iter([("title".into(), "Changed after review".into())]),
    });
    let result = engine.apply_inbox_cleanup(preview);
    assert_eq!(result.message, Some(Message::InboxCleanupStale));
    assert!(!result.changed);
    assert_eq!(engine.task_title(id_one).as_deref(), Some("first task"));
    assert_eq!(engine.task_title(id_two).as_deref(), Some("Changed after review"));
}

#[test]
fn cleanup_keeps_existing_links_and_never_adds_scheduling_tags() {
    let directory = tempfile::tempdir().unwrap();
    let engine = Engine::open(directory.path().to_string_lossy().into_owned());
    let morning = engine.add_tag("Morning".into()).unwrap();
    let mut task = Task::new(
        "https://example.org/morning-plans,https://example.org/another-page",
        INBOX_PROJECT_ID,
    );
    task.notes = Some("Already saved: <https://example.org/morning-plans>".into());
    let id = task.id.clone();
    engine.dispatch(Action::AddTask { task, bottom: true });
    let changes = engine.preview_inbox_cleanup();
    assert_eq!(changes[0].title, "Morning plans – example.org");
    assert_eq!(changes[0].links.len(), 2);
    assert!(changes[0].tag_ids.is_empty());
    let mut invalid = changes.clone();
    invalid[0].tag_ids.push(morning);
    assert_eq!(
        engine.apply_inbox_cleanup(invalid).message,
        Some(Message::InboxCleanupStale)
    );
    assert!(engine.apply_inbox_cleanup(changes).changed);
    let notes = engine.task_detail(id).unwrap().notes;
    assert_eq!(
        notes,
        "Already saved: <https://example.org/morning-plans>\n\nhttps://example.org/another-page"
    );
}
