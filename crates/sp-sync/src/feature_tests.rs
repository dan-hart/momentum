// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! The sync cycle against the in-process WebDAV mock: first upload, download-only,
//! convergence of two clients, conflict retry, relayed-op dedup, archives, errors.
use super::mock_dav::MockDav;
use super::*;
use sp_model::{Task, INBOX_PROJECT_ID};
use sp_oplog::Action;
use std::sync::atomic::Ordering;

fn cfg(server: &MockDav, encrypt: bool) -> NextcloudCfg {
    NextcloudCfg {
        server_url: server.url.clone(),
        user_name: "u".into(),
        password: "p".into(),
        folder: "sp".into(),
        compress: true,
        encrypt_key: encrypt.then(|| "k".to_string()),
    }
}
fn store(with_config: bool) -> (Store, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::load(dir.path().to_path_buf());
    if with_config {
        s.state.rest.insert("globalConfig".into(), serde_json::json!({}));
    }
    (s, dir)
}
fn add(s: &mut Store, title: &str) -> Task {
    let t = Task::new(title, INBOX_PROJECT_ID);
    s.dispatch(Action::AddTask {
        task: t.clone(),
        bottom: true,
    });
    t
}
fn titles(s: &Store) -> Vec<String> {
    let mut v: Vec<String> = s.state.task.iter().map(|t| t.title.clone()).collect();
    v.sort();
    v
}

#[test]
fn fresh_device_without_data_refuses_to_create_the_file() {
    let dav = MockDav::start();
    let (mut s, _d) = store(false);
    assert!(matches!(sync(&cfg(&dav, false), &mut s), Err(SyncError::FreshState)));
    assert_eq!(dav.file_count(), 0);
}
#[test]
fn first_upload_then_download_only() {
    let dav = MockDav::start();
    let c = cfg(&dav, true);
    let (mut a, _da) = store(true);
    add(&mut a, "from A");
    let r = sync(&c, &mut a).unwrap();
    assert!(r.uploaded && r.ops_uploaded == 1 && !r.downloaded);
    assert!(a.pending.is_empty() && a.meta.last_etag.is_some() && a.meta.last_sync_version == 1);
    assert!(dav
        .raw("/remote.php/dav/files/u/sp/sync-data.json")
        .unwrap()
        .starts_with(b"pf_CE2__"));
    // Nothing changed: the next cycle only downloads headers and uploads nothing.
    let r = sync(&c, &mut a).unwrap();
    assert!(!r.uploaded && !r.downloaded);
    let (mut b, _db) = store(false);
    let r = sync(&c, &mut b).unwrap();
    assert!(r.downloaded && !r.uploaded);
    assert_eq!(titles(&b), ["from A"]);
    assert_eq!(b.meta.last_sync_version, 1);
}
#[test]
fn two_clients_converge_by_rebasing() {
    let dav = MockDav::start();
    let c = cfg(&dav, false);
    let (mut a, _da) = store(true);
    let (mut b, _db) = store(false);
    add(&mut a, "from A");
    sync(&c, &mut a).unwrap();
    sync(&c, &mut b).unwrap();
    add(&mut b, "from B");
    add(&mut a, "from A2");
    sync(&c, &mut b).unwrap();
    sync(&c, &mut a).unwrap(); // rebases A2 onto B's upload
    sync(&c, &mut b).unwrap();
    assert_eq!(titles(&a), ["from A", "from A2", "from B"]);
    assert_eq!(titles(&a), titles(&b));
    assert_eq!(a.meta.last_sync_version, 3);
    assert!(a.pending.is_empty() && b.pending.is_empty());
    let (f, _) = download(&c).unwrap().unwrap();
    assert_eq!(f.recent_ops.len(), 3);
    assert_eq!(f.recent_ops[0].a, "HA");
    assert_eq!(f.recent_ops[2].sv, Some(3));
    assert_eq!(f.vector_clock.len(), 2);
}
#[test]
fn conflict_on_upload_is_retried() {
    let dav = MockDav::start();
    let c = cfg(&dav, false);
    let (mut a, _da) = store(true);
    add(&mut a, "x");
    dav.fail_next_put.store(true, Ordering::SeqCst);
    let r = sync(&c, &mut a).unwrap();
    assert!(r.uploaded);
    assert_eq!(dav.puts.load(Ordering::SeqCst), 2, "one 412, one success");
}
#[test]
fn relayed_ops_the_server_already_has_are_not_uploaded_twice() {
    let dav = MockDav::start();
    let c = cfg(&dav, false);
    let (mut a, _da) = store(true);
    let t = add(&mut a, "x");
    sync(&c, &mut a).unwrap();
    // The same op arrives again through a peer (same id, foreign client): queued locally,
    // but the server's op list must not grow.
    let mut b_clock = Default::default();
    let action = Action::UpdateTask {
        id: t.id.clone(),
        changes: [("title".to_string(), serde_json::json!("y"))].into_iter().collect(),
    };
    let mut op = action.to_op("peer", &mut b_clock);
    let (f, _) = download(&c).unwrap().unwrap();
    op.id = f.recent_ops[0].id.clone();
    assert!(a.apply_remote(op, action));
    let r = sync(&c, &mut a).unwrap();
    assert!(r.uploaded);
    let (f, _) = download(&c).unwrap().unwrap();
    assert_eq!(f.recent_ops.len(), 1, "no duplicate op id on the server");
    assert_eq!(
        f.state.task.entities[&t.id].title, "y",
        "the state still carries the change"
    );
}
#[test]
fn archives_ride_beside_the_state_and_come_back() {
    let dav = MockDav::start();
    let c = cfg(&dav, false);
    let (mut a, _da) = store(true);
    let t = add(&mut a, "done");
    a.dispatch(Action::UpdateTask {
        id: t.id.clone(),
        changes: [("isDone".to_string(), serde_json::json!(true))].into_iter().collect(),
    });
    let done = a.state.task.entities[&t.id].clone();
    a.dispatch(Action::MoveToArchive {
        tasks: vec![done],
        sub_tasks: vec![],
    });
    sync(&c, &mut a).unwrap();
    let (f, _) = download(&c).unwrap().unwrap();
    assert!(f.archive_young.is_some(), "archive is a top-level envelope field");
    assert!(
        f.state.rest.get("archiveYoung").is_none(),
        "and not duplicated inside state"
    );
    let (mut b, _db) = store(false);
    sync(&c, &mut b).unwrap();
    assert_eq!(b.state.rest["archiveYoung"]["task"]["ids"].as_array().unwrap().len(), 1);
}
#[test]
fn wrong_password_and_unknown_versions_are_reported() {
    let dav = MockDav::start();
    let (mut a, _da) = store(true);
    add(&mut a, "x");
    sync(&cfg(&dav, true), &mut a).unwrap();
    let mut wrong = cfg(&dav, true);
    wrong.encrypt_key = Some("nope".into());
    let (mut b, _db) = store(false);
    assert!(matches!(sync(&wrong, &mut b), Err(SyncError::Decrypt(_))));
    let mut none = cfg(&dav, false);
    none.encrypt_key = None;
    assert!(matches!(sync(&none, &mut b), Err(SyncError::Encrypted)));
    assert!(matches!(
        decode("pf_2__{\"version\":3}", None),
        Err(SyncError::Version(3))
    ));
    assert!(matches!(decode("pf_2__{}", None), Err(SyncError::Parse(_))));
    let f = SyncFile {
        schema_version: SCHEMA_VERSION + 1,
        ..decode(
            &encode(
                &SyncFile {
                    version: 2,
                    sync_version: 0,
                    schema_version: SCHEMA_VERSION,
                    vector_clock: VectorClock::new(),
                    last_modified: 0,
                    client_id: "c".into(),
                    state: AppData::fresh(),
                    archive_young: None,
                    archive_old: None,
                    recent_ops: vec![],
                    oldest_op_sync_version: None,
                },
                false,
                None,
            )
            .unwrap(),
            None,
        )
        .unwrap()
    };
    assert!(matches!(
        decode(&encode(&f, false, None).unwrap(), None),
        Err(SyncError::Schema(_))
    ));
}
#[test]
fn recent_ops_are_capped() {
    let dav = MockDav::start();
    let c = cfg(&dav, false);
    let (mut a, _da) = store(true);
    // Queue the ops directly: dispatching two thousand tasks would rewrite the store two
    // thousand times, which is not what this test is about.
    for i in 0..(MAX_RECENT_OPS + 5) {
        let action = Action::AddTask {
            task: Task::new(&format!("t{i}"), INBOX_PROJECT_ID),
            bottom: true,
        };
        sp_oplog::apply(&mut a.state, &action);
        let op = action.to_op(&a.meta.client_id.clone(), &mut a.meta.vector_clock);
        a.pending.push(sp_store::Pending { op, action });
    }
    sync(&c, &mut a).unwrap();
    let (f, _) = download(&c).unwrap().unwrap();
    assert_eq!(f.recent_ops.len(), MAX_RECENT_OPS);
    assert_eq!(f.oldest_op_sync_version, Some(1));
}
