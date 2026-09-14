// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Persistence: files on disk, pending queue, peer ops and snapshots.
use super::*;
use sp_model::{Task, INBOX_PROJECT_ID};
use sp_oplog::Action;

fn add(store: &mut Store, title: &str) -> Task {
    let t = Task::new(title, INBOX_PROJECT_ID);
    store.dispatch(Action::AddTask {
        task: t.clone(),
        bottom: true,
    });
    t
}

#[test]
fn fresh_store_creates_files_and_a_stable_client_id() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::load(dir.path().to_path_buf());
    assert!(s.meta.client_id.starts_with("momentum_") && s.meta.client_id.len() > 12);
    assert_eq!(s.state.project.ids, vec![INBOX_PROJECT_ID.to_string()]);
    s.save().unwrap();
    for f in ["state.json", "pending.json", "meta.json"] {
        assert!(dir.path().join(f).exists(), "{f}");
    }
    let again = Store::load(dir.path().to_path_buf());
    assert_eq!(again.meta.client_id, s.meta.client_id, "client id survives restarts");
    assert_eq!(again.dir(), dir.path());
}
#[test]
fn dispatch_applies_persists_and_queues_one_op() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::load(dir.path().to_path_buf());
    let t = add(&mut s, "a");
    assert_eq!(s.pending.len(), 1);
    assert_eq!(s.pending[0].op.c, s.meta.client_id);
    assert_eq!(s.meta.vector_clock[&s.meta.client_id], 1);
    let reloaded = Store::load(dir.path().to_path_buf());
    assert_eq!(reloaded.state.task.entities[&t.id].title, "a");
    assert_eq!(reloaded.pending.len(), 1, "pending survives a restart");
    assert_eq!(reloaded.meta.vector_clock, s.meta.vector_clock);
}
#[test]
fn corrupt_files_fall_back_to_a_fresh_store() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("state.json"), b"{not json").unwrap();
    std::fs::write(dir.path().join("pending.json"), b"[").unwrap();
    let s = Store::load(dir.path().to_path_buf());
    assert!(s.state.task.ids.is_empty() && s.pending.is_empty());
}
#[test]
fn replace_state_drops_pending_ops() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::load(dir.path().to_path_buf());
    add(&mut s, "a");
    let mut fresh = AppData::fresh();
    let t = Task::new("imported", INBOX_PROJECT_ID);
    fresh.task.insert(&t.id.clone(), t);
    s.replace_state(fresh);
    assert!(s.pending.is_empty());
    assert_eq!(s.state.task.ids.len(), 1);
}
#[test]
fn apply_remote_skips_own_and_duplicate_ops_and_queues_foreign_ones() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::load(dir.path().to_path_buf());
    let own = add(&mut s, "mine");
    // Our own op echoed back by a peer: ignored.
    let (op, action) = (s.pending[0].op.clone(), s.pending[0].action.clone());
    assert!(!s.apply_remote(op, action));
    // A foreign op: applied, clock merged, queued for the server.
    let mut clock: VectorClock = Default::default();
    let theirs = Task::new("theirs", INBOX_PROJECT_ID);
    let action = Action::AddTask {
        task: theirs.clone(),
        bottom: true,
    };
    let op = action.to_op("peer", &mut clock);
    assert!(s.apply_remote(op.clone(), action.clone()));
    assert!(s.state.task.entities.contains_key(&theirs.id) && s.state.task.entities.contains_key(&own.id));
    assert_eq!(s.meta.vector_clock["peer"], 1);
    assert_eq!(s.pending.len(), 2);
    assert!(!s.apply_remote(op, action), "the same op id is never applied twice");
    assert_eq!(s.pending.len(), 2);
}
#[test]
fn adopt_snapshot_replaces_a_fresh_store_and_unions_into_a_used_one() {
    let dir = tempfile::tempdir().unwrap();
    let mut fresh = Store::load(dir.path().to_path_buf());
    let mut snap = AppData::fresh();
    let t = Task::new("remote", INBOX_PROJECT_ID);
    snap.task.insert(&t.id.clone(), t.clone());
    snap.rest.insert("globalConfig".into(), serde_json::json!({"x": 1}));
    assert_eq!(fresh.adopt_snapshot(snap.clone()), usize::MAX, "whole snapshot adopted");
    assert!(fresh.meta.p2p_bootstrapped);
    assert_eq!(fresh.state.rest["globalConfig"]["x"], 1);

    let dir2 = tempfile::tempdir().unwrap();
    let mut used = Store::load(dir2.path().to_path_buf());
    let mine = add(&mut used, "mine");
    let pending_before = used.pending.len();
    let mut snap2 = snap.clone();
    let mut sub = Task::new("sub", INBOX_PROJECT_ID);
    sub.parent_id = Some(t.id.clone());
    snap2
        .task
        .entities
        .get_mut(&t.id)
        .unwrap()
        .sub_task_ids
        .push(sub.id.clone());
    snap2.task.insert(&sub.id.clone(), sub.clone());
    let p = sp_model::Project::new("Extra");
    snap2.project.insert(&p.id.clone(), p.clone());
    let added = used.adopt_snapshot(snap2);
    assert_eq!(added, 3, "project, task and subtask were added");
    assert!(used.state.task.entities.contains_key(&mine.id) && used.state.task.entities.contains_key(&t.id));
    assert_eq!(used.state.task.entities[&t.id].sub_task_ids, vec![sub.id.clone()]);
    assert_eq!(used.state.project.entities[&p.id].task_ids, Vec::<String>::new());
    assert_eq!(used.pending.len(), pending_before, "merging a snapshot emits no ops");
    assert!(
        !used.state.rest.contains_key("globalConfig"),
        "unions never overwrite local slices"
    );
}
